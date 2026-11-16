use crate::{
    demand, economy,
    game::Game,
    loyalty::{self, Level, RULES},
    save,
};
fn game() -> Game {
    let mut g = Game::default();
    g.found("Loyalty audit", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("a320_200").unwrap();
    g.set_cabin_layout(1, 12).unwrap();
    g.staff_up();
    g.create_route("EDDF", "LTFM", 1).unwrap();
    g.set_ticket_price(1, 120).unwrap();
    g.set_business_ticket_price(1, 300).unwrap();
    g.schedule(1, 1, 360).unwrap();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("loyalty.json");
    save::save(&p, g).unwrap();
    save::load(&p).unwrap()
}
#[test]
fn loyalty_daily_rounding_lapse_and_cap() {
    let mut g = game();
    // Tiny flights combine before rounding; both fractions survive save/load.
    g.loyalty.pending = 3 * Level::Basic.enrolment(1);
    g.finance.totals.passengers = 3;
    g.clock =
        crate::clock::Clock::from_state(std::time::Duration::from_secs(86400), true, 1).unwrap();
    g.apply_loyalty().unwrap();
    assert_eq!(g.loyalty.members, 0);
    assert_eq!(g.loyalty.enrolment_remainder, 3600);
    g.loyalty.pending = Level::Generous.enrolment(3);
    g.clock =
        crate::clock::Clock::from_state(std::time::Duration::from_secs(172800), true, 1).unwrap();
    g.apply_loyalty().unwrap();
    assert_eq!(g.loyalty.members, 1);
    assert_eq!(g.loyalty.trend, 1);
    assert_eq!(g.loyalty.enrolment_remainder, 2600);
    let before = g.loyalty.clone();
    g.apply_loyalty().unwrap();
    assert_eq!(g.loyalty, before);
    // None retains membership and continues the same daily lapse share.
    g.loyalty.members = 10000;
    g.set_loyalty_level(Level::None).unwrap();
    g.clock =
        crate::clock::Clock::from_state(std::time::Duration::from_secs(259200), true, 1).unwrap();
    g.apply_loyalty().unwrap();
    assert_eq!(g.loyalty.members, 9990);
    assert_eq!(g.loyalty.trend, -10);
    assert_eq!(loyalty::bonus(0), 0.0);
    assert!((loyalty::bonus(RULES.saturation_members / 2) - 0.03).abs() < 1e-10);
    assert!((loyalty::bonus(i64::MAX) - 0.06).abs() < 1e-10);
    assert_eq!(loyalty::weight(0, 0.123), 0.123);
    assert!((loyalty::path_weight([i64::MAX, i64::MAX], 1.0) - 1.06).abs() < 1e-10);
    assert!((loyalty::path_weight([0, i64::MAX], 1.0) - 1.03).abs() < 1e-10);
    // A single member eventually lapses, rather than rounding away daily decay.
    g.loyalty = loyalty::Loyalty {
        members: 1,
        ..Default::default()
    };
    for day in 1..=1000 {
        g.clock =
            crate::clock::Clock::from_state(std::time::Duration::from_secs(day * 86400), true, 1)
                .unwrap();
        g.apply_loyalty().unwrap();
    }
    assert_eq!(g.loyalty.members, 0);
}
#[test]
fn loyalty_cost_formula_shared_market_and_freighter() {
    let mut g = game();
    for (level, y, j) in [
        (Level::None, 0, 0),
        (Level::Basic, 1, 3),
        (Level::Generous, 3, 8),
    ] {
        assert_eq!(level.cost(3, 1), 2 * y + j);
        assert_eq!(level.cost(0, 0), 0);
        g.set_loyalty_level(level).unwrap();
        let q = economy::quote_shared(&g.routes[0], "a320_200", &g.market_routes()).unwrap();
        assert_eq!(
            q.loyalty_cost,
            (q.passengers - q.business_passengers) * y + q.business_passengers * j
        );
        assert_eq!(q.loyalty_enrolment, level.enrolment(q.passengers));
        assert!(q.valid_cabins(120));
    }
    let mut a = g.market_routes()[0].clone();
    a.seats = 1000;
    a.business_seats = 100;
    let mut b = a.clone();
    b.route.id = 99;
    b.owner = "rival".into();
    let equal = demand::estimate(&a.route, a.seats, &[a.clone(), b.clone()]).unwrap();
    a.loyalty_members = RULES.saturation_members;
    let improved = demand::estimate(&a.route, a.seats, &[a.clone(), b.clone()]).unwrap();
    let rival = demand::estimate(&b.route, b.seats, &[a.clone(), b.clone()]).unwrap();
    assert!(improved.per_departure > equal.per_departure);
    assert!(rival.per_departure <= equal.per_departure);
    assert_eq!(
        improved,
        demand::estimate(&a.route, a.seats, &[b.clone(), a.clone()]).unwrap()
    );
    assert!(
        improved.daily_passengers + rival.daily_passengers
            <= demand::market(&a.route, 0).unwrap().total()
    );
    let before = state(&g);
    g.fare_suggestion(1).unwrap();
    assert_eq!(before, state(&g));
    let mut f = Game::default();
    f.found("Cargo", "DE", "EDDF").unwrap();
    f.competitors.clear();
    f.set_loyalty_level(Level::Generous).unwrap();
    f.lease_aircraft("atr72_600f").unwrap();
    f.staff_up();
    f.create_route("EDDF", "EDDM", 1).unwrap();
    f.schedule(1, 1, 120).unwrap();
    f.clock.set_running(true);
    assert!(f.advance(86400.0));
    assert!(f.finance.totals.cargo_kg > 0);
    assert_eq!(f.finance.totals.loyalty_cost, 0);
    assert_eq!(f.loyalty.members, 0);
    assert!(Level::parse("invalid").is_err());
}
#[test]
fn loyalty_frozen_midflight_steps_speeds_pause_and_restart() {
    let mut initial = game();
    initial.set_loyalty_level(Level::Basic).unwrap();
    initial.competitors = crate::competition::initial_competitors().unwrap();
    for c in &mut initial.competitors {
        c.game.set_loyalty_level(Level::Generous).unwrap();
    }
    assert!(initial.advance(60.0));
    let frozen = initial.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .clone()
        .unwrap();
    assert!(frozen.totals.loyalty_cost > 0);
    assert_eq!(initial.finance.totals.loyalty_cost, 0);
    initial.set_loyalty_level(Level::None).unwrap();
    assert_eq!(initial.loyalty.members, 0);
    let mut expected = restart(&initial);
    assert!(expected.advance((frozen.landing - 60) as f64));
    assert_eq!(
        expected.finance.totals.loyalty_cost,
        frozen.totals.loyalty_cost
    );
    assert_eq!(expected.loyalty.pending, frozen.totals.loyalty_enrolment);
    assert!(expected.advance((14 * 86400 - frozen.landing) as f64));
    assert_eq!(
        expected.finance.totals.loyalty_cost,
        frozen.totals.loyalty_cost
    );
    assert!(expected.loyalty.members > 0);
    assert!(
        expected
            .competitors
            .iter()
            .all(|c| c.game.finance.totals.loyalty_cost > 0)
    );
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        let before = state(&g);
        g.clock.set_running(false);
        assert!(g.advance(1000.0));
        g.clock.set_running(true);
        assert_eq!(before, state(&g));
        let mut left = 14 * 86400 - 60;
        let mut i = 0;
        while left > 0 {
            let step = left.min([1, 3599, 100003][i % 3]);
            assert!(g.advance(step as f64 / f64::from(speed)));
            if i == 2 {
                g = restart(&g);
            }
            left -= step;
            i += 1;
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected), "speed {speed}");
        g.validate_history().unwrap();
        g.validate_loyalty().unwrap();
    }
    assert_eq!(state(&restart(&expected)), state(&expected));
}
#[test]
fn loyalty_v41_migration_all_owners_airborne_and_history() {
    let mut g = game();
    g.competitors = crate::competition::initial_competitors().unwrap();
    assert!(g.advance(60.0));
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("v41.json");
    save::save(&p, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    v["version"] = 41.into();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                for k in [
                    "loyalty",
                    "loyalty_level",
                    "loyalty_cost",
                    "loyalty_enrolment",
                ] {
                    o.remove(k);
                }
                for c in o.values_mut() {
                    strip(c);
                }
            }
            serde_json::Value::Array(a) => {
                for c in a {
                    strip(c);
                }
            }
            _ => {}
        }
    }
    strip(&mut v);
    std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
    let loaded = save::load(&p).unwrap();
    assert_eq!(state(&loaded), state(&g));
    for owner in std::iter::once(&loaded).chain(loaded.competitors.iter().map(|c| &*c.game)) {
        assert_eq!(owner.loyalty, loyalty::Loyalty::default());
        assert!(owner.routes.iter().all(|r| r.loyalty_level == Level::None));
        assert_eq!(owner.finance.totals.loyalty_cost, 0);
    }
    let mut bad = state(&g);
    bad["loyalty"]["members"] = (-1).into();
    let corrupt: Game = serde_json::from_value(bad).unwrap();
    assert!(save::save(&p, &corrupt).is_err());
}
#[test]
fn loyalty_ai_rule() {
    let mut g = game();
    g.airline.as_mut().unwrap().capital_eur = RULES.ai_reserve_eur;
    g.review_loyalty();
    assert_eq!(g.loyalty.level, Level::Basic);
    g.airline.as_mut().unwrap().capital_eur = RULES.ai_reserve_eur - 1;
    g.review_loyalty();
    assert_eq!(g.loyalty.level, Level::None);
    g.airline.as_mut().unwrap().capital_eur = RULES.ai_reserve_eur;
    g.resolution.deadline = Some(999999);
    g.review_loyalty();
    assert_eq!(g.loyalty.level, Level::None);
    g.resolution.deadline = None;
    g.airline.as_mut().unwrap().capital_eur = -1;
    g.review_loyalty();
    assert_eq!(g.loyalty.level, Level::None);
}
#[test]
fn loyalty_normal_eddf_rivals_two_and_ten_year_comparison() {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            loyalty::LEGACY_AI.with(|v| v.set(false));
        }
    }
    let _reset = Reset;
    let mut new = Game::default();
    new.found("EDDF baseline", "DE", "EDDF").unwrap();
    new.clock.set_running(true);
    let mut baseline = new.clone();
    for year in 1..=10 {
        assert!(new.advance(365.0 * 86400.0));
        loyalty::LEGACY_AI.with(|v| v.set(true));
        assert!(baseline.advance(365.0 * 86400.0));
        loyalty::LEGACY_AI.with(|v| v.set(false));
        if year == 2 || year == 10 {
            assert_eq!(new.competitors.len(), baseline.competitors.len());
            for (a, b) in new.competitors.iter().zip(&baseline.competitors) {
                println!(
                    "LOYALTY year={year} airline={} fleet={}/{} cash={}/{} insolvent={}/{} distressed={}/{} members={} rewards={}",
                    a.game.airline.as_ref().unwrap().name,
                    a.game.fleet.len(),
                    b.game.fleet.len(),
                    a.game.cash(),
                    b.game.cash(),
                    a.game.insolvent(),
                    b.game.insolvent(),
                    a.game.distressed(),
                    b.game.distressed(),
                    a.game.loyalty.members,
                    a.game.finance.totals.loyalty_cost
                );
                assert!(!a.game.insolvent() && !a.game.distressed());
                assert!(a.game.cash() > b.game.cash() / 2 && a.game.cash() < b.game.cash() * 3 / 2);
                assert!(a.game.fleet.len().abs_diff(b.game.fleet.len()) <= 2);
            }
            assert_eq!(state(&restart(&new)), state(&new));
        }
    }
}

#[test]
fn loyalty_codeshare_operating_legs_tax_and_ferry() {
    let mut g = Game::default();
    g.found("Connecting rewards", "DE", "EDDF").unwrap();
    g.competitors = crate::competition::initial_competitors().unwrap();
    g.competitors[1].game.set_ticket_price(1, 120).unwrap();
    g.competitors[1].game.stop_schedule(1).unwrap();
    g.competitors[1].game.schedule(1, 1, 240).unwrap();
    g.competitors[1]
        .game
        .set_loyalty_level(Level::Basic)
        .unwrap();
    g.lease_aircraft("a320_200").unwrap();
    g.staff_up();
    g.set_loyalty_level(Level::Generous).unwrap();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 90).unwrap();
    g.schedule(1, 1, 240).unwrap();
    let partner = g.competitors[1].game.market_owner_key();
    g.propose_codeshare(&partner).unwrap();
    let supply = g.prioritize_market(&g.market_routes());
    for owner in [&g, &g.competitors[1].game] {
        let r = &owner.routes[0];
        let q = economy::quote_at_time(r, "a320_200", &supply, owner.fuel_market.price_at(60), 60)
            .unwrap();
        assert!(q.connecting_passengers > 0);
        assert!(!q.codeshare.is_empty());
        assert_eq!(
            q.loyalty_cost,
            owner
                .loyalty
                .level
                .cost(q.passengers, q.business_passengers)
        );
        assert_eq!(
            q.loyalty_enrolment,
            owner.loyalty.level.enrolment(q.passengers)
        );
        let mut h = crate::history::History::new(0, 0);
        h.record(crate::history::Batch {
            route_id: 1,
            first: 100,
            interval: 100,
            count: 1,
            origin: "EDDF".into(),
            destination: "EDDM".into(),
            ticket_eur: 90,
            totals: q.clone(),
        });
        assert_eq!(h.pretax_profit(100).unwrap(), q.profit());
        let mut without = q.clone();
        without.loyalty_cost = 0;
        assert_eq!(without.profit() - q.profit(), q.loyalty_cost);
    }
    let mut f = Game::default();
    f.found("Ferry rewards", "DE", "EDDF").unwrap();
    f.competitors.clear();
    f.set_loyalty_level(Level::Generous).unwrap();
    f.buy_aircraft("atr72_600").unwrap();
    f.staff_up();
    f.send_ferry(1, "EDDM").unwrap();
    assert_eq!(f.ferries[0].quote.totals.loyalty_cost, 0);
    assert_eq!(f.ferries[0].quote.totals.loyalty_enrolment, 0);
}
