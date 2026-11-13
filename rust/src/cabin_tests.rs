use crate::{aircraft::AircraftStatus, cabins, economy, game::Game, save};

fn game() -> Game {
    let mut g = Game::default();
    g.found("Cabins", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("a320_200").unwrap();
    g.staff_up();
    g.create_route("EDDF", "LTFM", 1).unwrap();
    g.set_ticket_price(1, 100).unwrap();
    g.set_business_ticket_price(1, 220).unwrap();
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cabins.json");
    save::save(&path, g).unwrap();
    save::load(&path).unwrap()
}

#[test]
fn cabins_layout_floor_space_fee_and_ground_restrictions() {
    let mut g = game();
    let cash = g.cash();
    g.set_cabin_layout(1, 12).unwrap();
    assert_eq!(cabins::layout(&g.fleet[0]).unwrap(), (156, 12));
    assert_eq!(g.cash(), cash - cabins::RULES.layout_fee_eur);
    let before = state(&g);
    assert!(g.set_cabin_layout(1, 91).is_err());
    assert_eq!(before, state(&g));
    g.set_cabin_layout(1, 12).unwrap();
    assert_eq!(before, state(&g));
    g.set_cabin_layout(1, 90).unwrap();
    assert_eq!(cabins::layout(&g.fleet[0]).unwrap(), (0, 90));
    g.set_cabin_layout(1, 0).unwrap();
    g.schedule(1, 1, 360).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(60.0));
    let before = state(&g);
    assert!(g.set_cabin_layout(1, 10).is_err());
    assert_eq!(before, state(&g));
    assert_eq!(g.fleet[0].status, AircraftStatus::InFlight);
    let landing = g.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .as_ref()
        .unwrap()
        .landing;
    assert!(g.advance((landing - 60) as f64));
    assert_eq!(g.fleet[0].status, AircraftStatus::Ground);
    g.set_cabin_layout(1, 10).unwrap();
    g.request_maintenance(1).unwrap();
    let before = state(&g);
    assert!(g.set_cabin_layout(1, 0).is_err());
    assert_eq!(before, state(&g));
    assert_eq!(state(&restart(&g)), state(&g));
}

#[test]
fn cabins_shared_spill_overflow_price_limits_and_frozen_results() {
    let mut g = game();
    g.set_cabin_layout(1, 1).unwrap();
    g.schedule(1, 1, 1440).unwrap();
    let supply = g.market_routes();
    let r = &g.routes[0];
    let quote = economy::quote_shared(r, "a320_200", &supply).unwrap();
    assert_eq!(quote.business_passengers, 1); // full premium; business spills down
    let business_only = {
        let mut r = r.clone();
        r.ticket_eur = 240; // no willing leisure, but business still accepts
        let mut o = supply[0].clone();
        o.route = r.clone();
        let q = economy::quote_shared(&r, "a320_200", &[o]).unwrap();
        assert!(q.passengers > q.business_passengers);
        q
    };
    assert!(business_only.passengers <= 179);
    assert!(quote.passengers > quote.business_passengers);
    assert_eq!(quote.revenue, (quote.passengers - 1) * 100 + 220);
    assert_eq!(quote.seats, 179);
    g.set_business_ticket_price(1, 100_000).unwrap();
    let expensive = economy::quote_shared(&g.routes[0], "a320_200", &g.market_routes()).unwrap();
    assert_eq!(expensive.business_passengers, 0);
    assert!(expensive.passengers > 0);
    g.set_business_ticket_price(1, 220).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(60.0));
    let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
    g.set_business_ticket_price(1, 999).unwrap();
    g.set_ticket_price(1, 999).unwrap();
    let mut g = restart(&g);
    assert_eq!(
        g.routes[0].plan.as_ref().unwrap().airborne.as_ref(),
        Some(&frozen)
    );
    assert!(g.advance((frozen.landing - 60) as f64));
    assert_eq!(g.finance.recent.last().unwrap(), &frozen);
}

#[test]
fn cabins_speeds_steps_restart_and_ai_finances() {
    let mut initial = game();
    initial.set_cabin_layout(1, 12).unwrap();
    initial.schedule(1, 1, 360).unwrap();
    // Include real competitors so their premium heuristics, costs and reviews run.
    initial.competitors = crate::competition::initial_competitors().unwrap();
    let duration = 28 * 86400;
    let mut expected = initial.clone();
    expected.clock.set_running(true);
    assert!(expected.advance(duration as f64));
    expected.validate_competitors().unwrap();
    assert!(
        expected
            .competitors
            .iter()
            .any(|c| c.game.fleet.iter().any(|p| p.business_seats > 0))
    );
    assert!(
        expected
            .competitors
            .iter()
            .all(|c| c.game.cash().abs() < 1_000_000_000)
    );
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_running(true);
        g.clock.set_speed(speed);
        assert!(g.advance(12345.0 / speed as f64));
        g = restart(&g);
        let mut left = duration - 12345;
        while left > 0 {
            let step = left.min(100_003);
            assert!(g.advance(step as f64 / speed as f64));
            left -= step;
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected));
        assert_eq!(state(&restart(&g)), state(&g));
    }
}

#[test]
fn cabins_v16_migration_keeps_all_economy_money_and_airborne_quotes() {
    let mut g = game();
    g.competitors = crate::competition::initial_competitors().unwrap();
    g.schedule(1, 1, 360).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(60.0));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    save::save(&path, &g).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["version"] = 16.into();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                for k in [
                    "business_seats",
                    "business_ticket_eur",
                    "business_passengers",
                    "business_revenue",
                ] {
                    o.remove(k);
                }
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
    strip(&mut value);
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let loaded = save::load(&path).unwrap();
    assert!(
        loaded
            .fleet
            .iter()
            .chain(loaded.competitors.iter().flat_map(|c| c.game.fleet.iter()))
            .all(|p| p.business_seats == 0)
    );
    assert_eq!(loaded.cash(), g.cash());
    assert_eq!(loaded.finance, g.finance);
    assert_eq!(loaded.routes[0].plan, g.routes[0].plan);
    assert_eq!(
        loaded
            .market_routes()
            .iter()
            .map(|o| o.seats)
            .collect::<Vec<_>>(),
        g.market_routes()
            .iter()
            .map(|o| o.seats)
            .collect::<Vec<_>>()
    );
    let mut bad = loaded.clone();
    bad.fleet[0].business_seats = u32::MAX;
    assert!(save::save(&path, &bad).is_err());
    bad = loaded.clone();
    bad.routes[0].business_ticket_eur = -1;
    assert!(save::save(&path, &bad).is_err());
}

#[test]
fn cabins_leisure_cannot_fill_an_all_business_plane_and_market_is_conserved() {
    let mut g = game();
    g.set_cabin_layout(1, 90).unwrap();
    g.schedule(1, 1, 1440).unwrap();
    let mut r = g.routes[0].clone();
    r.origin_icao = "EDLP".into();
    r.destination_icao = "EDFH".into();
    r.ticket_eur = 0;
    r.business_ticket_eur = 0;
    let o = cabins::offer(&r, &g.fleet[0]).unwrap();
    let q = economy::quote_shared(&r, "a320_200", &[o]).unwrap();
    let m = crate::demand::market(&r, 0).unwrap();
    assert!(m.business < 90.0 && m.leisure > 0.0);
    assert_eq!(q.passengers, q.business_passengers);
    assert!(q.passengers as f64 <= m.business);
    // A mixed rival and the premium flight still share one business/leisure pool.
    let mut rival = r.clone();
    rival.id = 9;
    rival.aircraft_id = Some(9);
    let supply = [
        cabins::offer(&r, &g.fleet[0]).unwrap(),
        crate::demand::Offer::new(&rival, 180),
    ];
    let a = economy::quote_shared(&r, "a320_200", &supply).unwrap();
    let b = economy::quote_shared(&rival, "a320_200", &supply).unwrap();
    assert!((a.passengers + b.passengers) as f64 * 0.5 <= m.total());
}

#[test]
fn cabins_layout_limits_budget_and_group_fares_are_transactional() {
    let mut g = game();
    g.lease_aircraft("b737_800").unwrap();
    g.set_cabin_layout(2, 94).unwrap();
    assert_eq!(cabins::layout(&g.fleet[1]).unwrap(), (1, 94));
    let mut poor = g.clone();
    poor.airline.as_mut().unwrap().capital_eur = cabins::RULES.layout_fee_eur - 1;
    let before = state(&poor);
    assert!(poor.set_cabin_layout(1, 10).is_err());
    assert_eq!(state(&poor), before);
    g.add_route_aircraft(1, 2).unwrap();
    g.set_business_ticket_price(1, 456).unwrap();
    assert!(g.routes.iter().all(|r| r.business_ticket_eur == 456));
    let before = state(&g);
    assert!(g.set_business_ticket_price(1, -1).is_err());
    assert!(g.set_business_ticket_price(1, 100_001).is_err());
    assert_eq!(before, state(&g));
    assert_eq!(state(&restart(&g)), before);
}
