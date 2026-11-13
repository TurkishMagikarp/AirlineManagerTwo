use crate::{
    game::Game,
    marketing::{DAY, SCALE},
    save,
};
fn game() -> Game {
    let mut g = Game::default();
    g.found("Brand Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("brand.json");
    save::save(&p, g).unwrap();
    save::load(&p).unwrap()
}
#[test]
fn marketing_zero_gradual_diminishing_decay_and_deductible_costs() {
    let mut g = game();
    let initial = g.cash();
    assert!(g.advance((400 * DAY) as f64));
    assert_eq!(g.cash(), initial);
    assert_eq!(g.marketing.awareness, 0);
    let before = state(&g);
    assert!(g.set_marketing_budget(-1).is_err());
    assert!(g.set_marketing_budget(1_000_001).is_err());
    assert_eq!(state(&g), before);
    g.set_marketing_budget(50_000).unwrap();
    assert_eq!(g.marketing.awareness, 0);
    let mut high = g.clone();
    high.set_marketing_budget(100_000).unwrap();
    assert!(g.advance(DAY as f64));
    assert!(high.advance(DAY as f64));
    assert!(g.marketing.awareness > 0 && g.marketing.awareness < SCALE);
    assert!(high.marketing.awareness < 2 * g.marketing.awareness);
    assert!(g.advance((29 * DAY) as f64));
    assert_eq!(initial - g.cash(), 50_000);
    let h = g.history.as_ref().unwrap();
    assert_eq!(h.marketing_through(u64::MAX), 50_000);
    assert_eq!(h.pretax_profit(u64::MAX).unwrap(), -50_000);
    g.set_marketing_budget(0).unwrap();
    let awareness = g.marketing.awareness;
    assert!(g.advance((3650 * DAY) as f64));
    assert!(g.marketing.awareness < awareness);
    assert_eq!(g.marketing.awareness, 0);
    assert_eq!(initial - g.cash(), 50_000);
    g.validate_history().unwrap();
}
#[test]
fn marketing_steps_speeds_pause_midmonth_restart_and_ai_finances() {
    let mut initial = game();
    initial.airline.as_mut().unwrap().name = "Regional Wings".into();
    initial.competitors = crate::competition::initial_competitors().unwrap();
    initial.buy_aircraft("atr72_600").unwrap();
    initial.staff_up();
    initial.create_route("EDDF", "EDDM", 1).unwrap();
    initial.schedule(1, 1, 120).unwrap();
    initial.set_marketing_budget(73_333).unwrap();
    let duration = 120 * DAY;
    let mut expected = initial.clone();
    assert!(expected.advance(duration as f64));
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        g.clock.set_running(false);
        let before = state(&g);
        assert!(g.advance(1234.0));
        assert_eq!(state(&g), before);
        g.clock.set_running(true);
        assert!(g.advance(1_234_567.0 / f64::from(speed)));
        g = restart(&g);
        let mut left = duration - 1_234_567;
        while left > 0 {
            let n = left.min(100_003);
            assert!(g.advance(n as f64 / f64::from(speed)));
            left -= n;
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected));
    }
    expected.validate_competitors().unwrap();
    assert_eq!(expected.competitors.len(), 2);
    assert!(
        expected
            .competitors
            .iter()
            .all(|c| c.game.marketing.awareness > 0 && c.game.cash() > 0)
    );
}
#[test]
fn marketing_rate_changes_carry_fractional_costs_and_ai_cuts() {
    let mut g = game();
    g.set_marketing_budget(1).unwrap();
    for _ in 0..30 {
        assert!(g.advance(DAY as f64 / 2.0));
        g.set_marketing_budget(1).unwrap();
        assert!(g.advance(DAY as f64 / 2.0));
    }
    assert_eq!(g.history.as_ref().unwrap().marketing_through(u64::MAX), 1);
    g = restart(&g);
    g.review_marketing().unwrap();
    assert!(g.marketing.budget_eur > 0);
    let shift = 1_000_000 - g.cash();
    g.history.as_mut().unwrap().opening_balance += shift;
    g.airline.as_mut().unwrap().capital_eur += shift;
    g.review_marketing().unwrap();
    assert_eq!(g.marketing.budget_eur, 0);
    let shift = -1 - g.cash();
    g.history.as_mut().unwrap().opening_balance += shift;
    g.airline.as_mut().unwrap().capital_eur += shift;
    g.review_marketing().unwrap();
    assert_eq!(g.marketing.budget_eur, 0);
}
#[test]
fn marketing_v19_migration_zero_for_all_owners_no_retroactive_cost() {
    let mut g = game();
    g.competitors = crate::competition::initial_competitors().unwrap();
    assert!(g.advance(12_345.0));
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("legacy.json");
    save::save(&p, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    v["version"] = 19.into();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.remove("marketing");
                o.remove("marketing_costs");
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
    let mut loaded = save::load(&p).unwrap();
    for company in std::iter::once(&loaded).chain(loaded.competitors.iter().map(|c| &*c.game)) {
        assert_eq!(company.marketing.budget_eur, 0);
        assert_eq!(company.marketing.awareness, 0);
        assert_eq!(company.marketing.settled_at, 12_345);
        assert!(company.history.as_ref().unwrap().marketing_costs.is_empty());
    }
    let balance = loaded.cash();
    loaded.set_marketing_budget(30_000).unwrap();
    assert_eq!(loaded.cash(), balance);
    assert!(loaded.advance((DAY - 12_345) as f64));
    assert_eq!(
        balance - loaded.cash(),
        (DAY - 12_345) as i64 * 30_000 / (30 * DAY) as i64
    );
    assert_eq!(state(&restart(&loaded)), state(&loaded));
    loaded.marketing.awareness = 101 * SCALE;
    assert!(save::save(&p, &loaded).is_err());
}
#[test]
fn marketing_choice_is_modest_preserves_ceilings_and_owner_identity() {
    let mut g = game();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, 360).unwrap();
    let route = g.routes[0].clone();
    let mut own = crate::demand::Offer::new(&route, 1000);
    own.owner = g.market_owner_key();
    let mut rival = own.clone();
    rival.owner = "company:Other".into();
    // Identical IDs and commercial keys must retain each owner's awareness.
    let base = crate::demand::estimate_at(&route, 1000, &[own.clone(), rival.clone()], 0)
        .unwrap()
        .per_departure;
    own.awareness = 100 * SCALE;
    let supply = g.prioritize_market(&[rival.clone(), own.clone()]);
    let branded = crate::demand::estimate_at(&route, 1000, &supply, 0)
        .unwrap()
        .per_departure;
    assert!(branded >= base && branded <= base * 110 / 100 + 1);
    let mut other = game();
    other.airline.as_mut().unwrap().name = "Other".into();
    let supply = other.prioritize_market(&[own.clone(), rival.clone()]);
    let rival_pax = crate::demand::estimate_at(&route, 1000, &supply, 0)
        .unwrap()
        .per_departure;
    assert!(branded >= rival_pax);
    own.route.ticket_eur = 100_000;
    let supply = g.prioritize_market(&[rival, own.clone()]);
    assert_eq!(
        crate::demand::estimate_at(&own.route, 1000, &supply, 0)
            .unwrap()
            .per_departure,
        0
    );
}
#[test]
fn marketing_changed_rates_and_airborne_restart_are_partition_independent() {
    let mut initial = game();
    initial.buy_aircraft("atr72_600").unwrap();
    initial.staff_up();
    initial.create_route("EDDF", "EDDM", 1).unwrap();
    initial.schedule(1, 1, 120).unwrap();
    initial.set_marketing_budget(50_001).unwrap();
    let mut expected = initial.clone();
    assert!(expected.advance(123_457.0));
    let frozen = expected.routes[0].plan.as_ref().unwrap().airborne.clone();
    assert!(frozen.is_some());
    expected.set_marketing_budget(1).unwrap();
    assert_eq!(expected.routes[0].plan.as_ref().unwrap().airborne, frozen);
    assert!(expected.advance(654_321.0));
    expected.set_marketing_budget(0).unwrap();
    assert!(expected.advance(2_345_678.0));
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        for (duration, budget) in [(123_457, Some(1)), (654_321, Some(0)), (2_345_678, None)] {
            let mut left = duration;
            while left > 0 {
                let step = left.min(7_777);
                assert!(g.advance(f64::from(step) / f64::from(speed)));
                left -= step;
            }
            if let Some(budget) = budget {
                g.set_marketing_budget(budget).unwrap();
            }
            g = restart(&g);
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected));
    }
}
