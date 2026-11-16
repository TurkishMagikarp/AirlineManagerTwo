use crate::{ancillaries::Policy, demand, economy, game::Game, save};

fn game() -> Game {
    let mut g = Game::default();
    g.found("Ancillary audit", "DE", "EDDF").unwrap();
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
    let p = d.path().join("ancillary.json");
    save::save(&p, g).unwrap();
    save::load(&p).unwrap()
}

#[test]
fn ancillary_formula_rounding_market_share_and_fare_advisor() {
    let mut g = game();
    for p in [Policy::Included, Policy::Standard, Policy::Aggressive] {
        g.set_ancillary_policy(p).unwrap();
        let q = economy::quote_shared(&g.routes[0], "a320_200", &g.market_routes()).unwrap();
        let (y, j) = match p {
            Policy::Included => (0, 0),
            Policy::Standard => (8, 4),
            Policy::Aggressive => (20, 10),
        };
        assert!(q.passengers > q.business_passengers && q.business_passengers > 0);
        assert_eq!(
            q.ancillary,
            (q.passengers - q.business_passengers) * y + q.business_passengers * j
        );
        // Fees use the distributor's final integer cabin counts; no separate rounding.
        assert_eq!(
            q.revenue,
            (q.passengers - q.business_passengers) * 120
                + q.business_passengers * 300
                + q.cargo_revenue
                + q.ancillary
        );
        assert!(q.valid_cabins(120));
        assert_eq!(p.revenue(0, 0), 0);
        assert_eq!(p.revenue(1, 0), y);
        assert_eq!(p.revenue(1, 1), j);
        assert_eq!(p.revenue(3, 1), 2 * y + j);
    }
    let mut a = g.market_routes()[0].clone();
    a.seats = 1000;
    a.business_seats = 100;
    a.route.ancillary_policy = Policy::Included;
    let mut b = a.clone();
    b.route.id = 99;
    let equal = demand::estimate(&a.route, a.seats, &[a.clone(), b.clone()]).unwrap();
    let mut previous = equal.per_departure;
    for p in [Policy::Standard, Policy::Aggressive] {
        a.route.ancillary_policy = p;
        let supply = [a.clone(), b.clone()];
        let q = demand::estimate(&a.route, a.seats, &supply).unwrap();
        let rival = demand::estimate(&b.route, b.seats, &supply).unwrap();
        assert!(q.per_departure < previous);
        assert!(rival.per_departure >= equal.per_departure);
        assert_eq!(
            q,
            demand::estimate(&a.route, a.seats, &[b.clone(), a.clone()]).unwrap()
        );
        assert!(
            q.daily_passengers + rival.daily_passengers
                <= demand::market(&a.route, 0).unwrap().total()
        );
        previous = q.per_departure;
    }
    let before = state(&g);
    let advice = g.fare_suggestion(1).unwrap().unwrap();
    assert!(advice.current_profit != 0);
    assert_eq!(before, state(&g));
    assert!(Policy::parse("unknown").is_err());
}

#[test]
fn ancillary_policy_midflight_settlement_steps_speeds_and_restart() {
    let mut initial = game();
    initial.set_ancillary_policy(Policy::Standard).unwrap();
    initial.competitors = crate::competition::initial_competitors().unwrap();
    for rival in &mut initial.competitors {
        rival.game.set_ancillary_policy(Policy::Standard).unwrap();
    }
    assert!(initial.advance(60.0));
    let frozen = initial.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .clone()
        .unwrap();
    assert!(frozen.totals.ancillary > 0);
    assert_eq!(initial.finance.totals.ancillary, 0);
    initial.set_ancillary_policy(Policy::Aggressive).unwrap();
    let mut expected = restart(&initial);
    assert_eq!(
        expected.routes[0].plan.as_ref().unwrap().airborne,
        Some(frozen.clone())
    );
    assert!(expected.advance((frozen.landing - 60) as f64));
    assert_eq!(expected.finance.totals.ancillary, frozen.totals.ancillary);
    let duration = 14 * 86400;
    assert!(expected.advance((duration - frozen.landing) as f64));
    let later: Vec<_> = expected
        .finance
        .recent
        .iter()
        .filter(|f| f.landing > frozen.landing)
        .collect();
    assert!(!later.is_empty());
    for flight in later {
        let t = &flight.totals;
        assert_eq!(
            t.ancillary,
            20 * (t.passengers - t.business_passengers) + 10 * t.business_passengers
        );
    }
    assert!(
        expected
            .competitors
            .iter()
            .all(|c| c.game.finance.totals.ancillary > 0)
    );
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        let before = state(&g);
        g.clock.set_running(false);
        assert!(g.advance(1000.0));
        g.clock.set_running(true);
        assert_eq!(before, state(&g));
        let mut left = duration - 60;
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
    }
    // New routes inherit the airline policy and route-group additions retain it.
    let mut g = expected;
    g.borrow(8_000_000).unwrap();
    g.buy_aircraft("a320_200").unwrap();
    g.staff_up();
    g.add_route_aircraft(1, 2).unwrap();
    assert!(
        g.routes
            .iter()
            .all(|r| r.ancillary_policy == Policy::Aggressive)
    );
    g.routes[1].ancillary_policy = Policy::Included;
    assert!(g.validate_routes().is_err());
}

#[test]
fn ancillary_v40_migration_is_neutral_for_all_owners_and_frozen_results() {
    let mut g = game();
    g.competitors = crate::competition::initial_competitors().unwrap();
    assert!(g.advance(21660.0));
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("v40.json");
    save::save(&p, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.remove("ancillary_policy");
                o.remove("ancillary");
                for v in o.values_mut() {
                    strip(v);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    strip(v);
                }
            }
            _ => {}
        }
    }
    strip(&mut v);
    v["version"] = 40.into();
    std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
    let mut loaded = save::load(&p).unwrap();
    g.adopt_events_from(&loaded);
    assert_eq!(state(&loaded), state(&g));
    assert!(
        loaded
            .competitors
            .iter()
            .all(|c| c.game.ancillary_policy == Policy::Included)
    );
    for step in [3600.0, 86400.0, 700000.0] {
        assert!(g.advance(step));
        assert!(loaded.advance(step));
        assert_eq!(state(&loaded), state(&g));
    }
    let mut bad = loaded;
    bad.finance.totals.ancillary = -1;
    assert!(bad.validate_finance().is_err());
}

#[test]
fn ancillary_ai_rule_and_freighters() {
    let mut g = game();
    g.review_ancillaries();
    assert_eq!(g.ancillary_policy, Policy::Included);
    g.airline.as_mut().unwrap().capital_eur = -1;
    g.review_ancillaries();
    assert_eq!(g.ancillary_policy, Policy::Standard);
    assert!(
        g.routes
            .iter()
            .all(|r| r.ancillary_policy == Policy::Standard)
    );
    g.airline.as_mut().unwrap().capital_eur = 0;
    g.review_ancillaries();
    assert_eq!(g.ancillary_policy, Policy::Included);
    let mut freight = Game::default();
    freight.found("Cargo fees", "DE", "EDDF").unwrap();
    freight.competitors.clear();
    freight.buy_aircraft("atr72_600f").unwrap();
    freight.staff_up();
    freight.set_ancillary_policy(Policy::Aggressive).unwrap();
    freight.create_route("EDDF", "EDDM", 1).unwrap();
    freight.schedule(1, 1, 120).unwrap();
    assert_eq!(freight.routes[0].ancillary_policy, Policy::Aggressive);
    let q =
        economy::quote_shared(&freight.routes[0], "atr72_600f", &freight.market_routes()).unwrap();
    assert_eq!(
        (q.passengers, q.ancillary, q.revenue),
        (0, 0, q.cargo_revenue)
    );
}

#[test]
fn ancillary_codeshare_connections_charge_the_operating_airline() {
    let mut g = Game::default();
    g.found("Connecting fees", "DE", "EDDF").unwrap();
    g.competitors = crate::competition::initial_competitors().unwrap();
    g.competitors[1].game.set_ticket_price(1, 120).unwrap();
    g.competitors[1].game.stop_schedule(1).unwrap();
    g.competitors[1].game.schedule(1, 1, 240).unwrap();
    g.competitors[1]
        .game
        .set_ancillary_policy(Policy::Standard)
        .unwrap();
    g.lease_aircraft("a320_200").unwrap();
    g.staff_up();
    g.set_ancillary_policy(Policy::Aggressive).unwrap();
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
            q.ancillary,
            owner
                .ancillary_policy
                .revenue(q.passengers, q.business_passengers)
        );
        assert!(q.valid_cabins(r.ticket_eur));
        assert_eq!(
            q.codeshare.iter().map(|r| r.revenue).sum::<i64>(),
            q.connecting_revenue
        );
    }
}

#[test]
fn ancillary_normal_eddf_rivals_two_and_ten_year_comparison() {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            crate::ancillaries::LEGACY_AI.with(|v| v.set(false));
        }
    }
    let _reset = Reset;
    let mut new = Game::default();
    new.found("EDDF baseline", "DE", "EDDF").unwrap();
    new.clock.set_running(true);
    let mut baseline = new.clone();
    for year in 1..=10 {
        assert!(new.advance(365.0 * 86400.0));
        crate::ancillaries::LEGACY_AI.with(|v| v.set(true));
        assert!(baseline.advance(365.0 * 86400.0));
        crate::ancillaries::LEGACY_AI.with(|v| v.set(false));
        if year == 2 || year == 10 {
            for (a, b) in new.competitors.iter().zip(&baseline.competitors) {
                println!(
                    "ANCILLARY year={year} airline={} fleet={}/{} cash={}/{} insolvent={}/{} distressed={}/{}",
                    a.game.airline.as_ref().unwrap().name,
                    a.game.fleet.len(),
                    b.game.fleet.len(),
                    a.game.cash(),
                    b.game.cash(),
                    a.game.insolvent(),
                    b.game.insolvent(),
                    a.game.distressed(),
                    b.game.distressed()
                );
                assert!(!a.game.insolvent() && !a.game.distressed());
                assert!(a.game.cash() > b.game.cash() / 2 && a.game.cash() < b.game.cash() * 3 / 2);
                assert!(a.game.fleet.len().abs_diff(b.game.fleet.len()) <= 2);
            }
            assert_eq!(state(&restart(&new)), state(&new));
        }
    }
}
