use crate::{game::Game, pricing};
const DAY: u64 = 86400;
fn game() -> Game {
    let mut g = Game::default();
    g.found("Fare audit", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 90).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    let mut g = g.clone();
    g.clock.set_speed(1);
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pricing.json");
    crate::save::save(&path, g).unwrap();
    crate::save::load(&path).unwrap()
}
#[test]
fn pricing_suggestion_is_read_only_valid_and_matches_next_quote() {
    for fare in [0, 90, 100_000] {
        let mut g = game();
        g.set_ticket_price(1, fare).unwrap();
        let before = state(&g);
        let suggestion = g.fare_suggestion(1).unwrap().unwrap();
        assert_eq!(before, state(&g));
        assert!((0..=100_000).contains(&suggestion.fare));
        g.apply_fare_suggestion(1).unwrap();
        assert!(g.advance(61.0));
        let quote = g.routes[0]
            .plan
            .as_ref()
            .unwrap()
            .airborne
            .as_ref()
            .unwrap();
        assert_eq!(quote.totals.profit(), suggestion.profit);
        assert!(g.advance(60.0));
        let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone();
        g.apply_fare_suggestion(1).unwrap();
        assert_eq!(g.routes[0].plan.as_ref().unwrap().airborne, frozen);
    }
    let mut g = game();
    g.stop_schedule(1).unwrap();
    g.set_cabin_layout(1, 12).unwrap();
    g.schedule(1, 1, 120).unwrap();
    let s = g.fare_suggestion(1).unwrap().unwrap();
    assert_eq!(s.business, Some(crate::cabins::premium_fare(s.fare)));
    g.apply_fare_suggestion(1).unwrap();
    assert!(g.advance(61.0));
    assert_eq!(
        g.routes[0]
            .plan
            .as_ref()
            .unwrap()
            .airborne
            .as_ref()
            .unwrap()
            .totals
            .profit(),
        s.profit
    );
}
#[test]
fn pricing_empty_routes_cleanup_migration_and_corruption() {
    let mut g = game();
    assert!(g.fare_suggestion(999).unwrap().is_none());
    g.stop_schedule(1).unwrap();
    assert!(g.fare_suggestion(1).unwrap().is_none());
    g.assign_route(1, None).unwrap();
    assert!(g.fare_suggestion(1).unwrap().is_none());
    g.assign_route(1, Some(1)).unwrap();
    g.set_pricing_autopilot(1, true).unwrap();
    assert_eq!(
        g.pricing[0].next_review,
        Some(pricing::RULES.fare_review_seconds)
    );
    g.schedule(1, 20, 240).unwrap();
    assert_eq!(g.pricing.len(), 1);
    g.stop_schedule(1).unwrap();
    g.delete_route(1).unwrap();
    assert!(g.pricing.is_empty());
    assert!(g.next_pricing_event().is_none());
    let mut g = game();
    g.set_pricing_autopilot(1, true).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.json");
    crate::save::save(&path, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["version"] = 34.into();
    v["state"].as_object_mut().unwrap().remove("pricing");
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let migrated = crate::save::load(&path).unwrap();
    g.pricing.clear();
    assert_eq!(state(&migrated), state(&g));
    g.set_pricing_autopilot(1, true).unwrap();
    g.pricing[0].route_id = 999;
    assert!(crate::save::save(&path, &g).is_err());
}
#[test]
fn pricing_autopilot_weekly_manual_override_and_identical_speeds_restart() {
    let mut initial = game();
    initial.set_pricing_autopilot(1, true).unwrap();
    let mut reference = initial.clone();
    assert!(reference.advance(35.0 * DAY as f64));
    for speed in [1, 4, 16] {
        for step in [3600, DAY] {
            let mut g = initial.clone();
            g.clock.set_running(false);
            assert!(g.advance(500.0));
            assert_eq!(state(&g).get("pricing"), state(&initial).get("pricing"));
            g.clock.set_running(true);
            g.clock.set_speed(speed);
            for i in 0..35 * DAY / step {
                assert!(g.advance(step as f64 / f64::from(speed)));
                if i == 3 * DAY / step {
                    g = restart(&g);
                }
            }
            assert_eq!(state(&g), state(&reference), "speed={speed} step={step}");
        }
    }
    let mut g = initial;
    assert!(g.advance(6.0 * DAY as f64));
    g.set_ticket_price(1, 137).unwrap();
    let r = &g.routes[0];
    let plane = &g.fleet[0];
    let expected = pricing::review(&g, r, plane, &g.market_routes(), false).unwrap();
    let cargo = r.cargo_rate_cents;
    let service = r.service_level;
    let interval = r.plan.as_ref().unwrap().interval_seconds;
    assert!(g.advance(DAY as f64));
    assert_eq!(g.routes[0].ticket_eur, expected.fare);
    assert_eq!(g.routes[0].cargo_rate_cents, cargo);
    assert_eq!(g.routes[0].service_level, service);
    assert_eq!(
        g.routes[0].plan.as_ref().unwrap().interval_seconds,
        interval
    );
    assert_eq!(g.pricing[0].last_change.as_ref().unwrap().at, 7 * DAY);
    g.set_pricing_autopilot(1, false).unwrap();
    assert!(g.next_pricing_event().is_none());
}
#[test]
fn pricing_autopilot_jump_does_not_add_attention_or_stop() {
    let mut g = game();
    g.competitors.clear();
    g.set_pricing_autopilot(1, true).unwrap();
    let mut waiting = g.clone();
    let summary = g.jump(7.0 * DAY as f64).unwrap();
    assert!(summary.reason.is_empty(), "{}", summary.reason);
    assert!(waiting.advance(7.0 * DAY as f64));
    assert_eq!(state(&g), state(&waiting));
    assert!(
        !g.attention_snapshot()
            .iter()
            .any(|e| e.text.contains("Preis"))
    );
}
#[test]
fn pricing_two_year_player_comparison() {
    for enabled in [false, true] {
        let mut g = game();
        g.set_ticket_price(1, 150).unwrap();
        g.set_pricing_autopilot(1, enabled).unwrap();
        assert!(g.advance(730.0 * DAY as f64));
        println!(
            "FARE_TWO_YEAR autopilot={enabled} cash={} profit={} flights={} passengers={} final_fare={}",
            g.cash(),
            g.finance.totals.profit(),
            g.finance.totals.flights,
            g.finance.totals.passengers,
            g.routes[0].ticket_eur
        );
        g.validate_routes().unwrap();
    }
}

#[test]
fn pricing_terminal_boundary_cancels_reviews_and_remains_saveable() {
    let mut g = game();
    g.competitors.clear();
    g.set_ticket_price(1, 100_000).unwrap();
    g.set_pricing_autopilot(1, true).unwrap();
    let cash = g.cash();
    g.airline.as_mut().unwrap().capital_eur = -1;
    g.history.as_mut().unwrap().opening_balance -= cash + 1;
    assert!(g.advance(180.0 * DAY as f64));
    assert!(g.insolvent());
    assert!(g.next_pricing_event().is_none());
    assert_eq!(state(&restart(&g)), state(&g));
}
