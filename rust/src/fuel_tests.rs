use crate::{economy::FuelMarket, game::Game};
const WEEK: u64 = 7 * 86400;

fn game() -> Game {
    let mut g = Game::default();
    g.found("Fuel", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 90).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}

#[test]
fn fuel_bounds_seed_and_cadence() {
    let a = FuelMarket::default();
    let b = FuelMarket {
        seed: a.seed + 1,
        ..a.clone()
    };
    assert_eq!(a.price_at(0), 80);
    assert_eq!(a.price_at(WEEK - 1), 80);
    assert_ne!(a.price_at(WEEK), 80);
    let mut different = false;
    for week in 0..1040 {
        let t = week * WEEK;
        let price = a.price_at(t);
        assert!((50..=120).contains(&price));
        assert_eq!(price, a.price_at(t + WEEK - 1));
        different |= price != b.price_at(t);
    }
    assert!(different);
}

#[test]
fn fuel_steps_speeds_pause_restart_and_global_price() {
    let mut large = game();
    let mut small = large.clone();
    let mut medium = large.clone();
    assert!(large.advance((WEEK * 12) as f64));
    for _ in 0..84 {
        assert!(small.advance(86400.0));
    }
    assert_eq!(state(&large), state(&small));
    medium.clock.set_speed(4);
    for _ in 0..84 {
        assert!(medium.advance(21600.0));
    }
    medium.clock.set_speed(1);
    assert_eq!(state(&large), state(&medium));
    let mut fast = game();
    fast.clock.set_running(false);
    let paused = state(&fast);
    assert!(fast.advance(1e8));
    assert_eq!(paused, state(&fast));
    fast.clock.set_running(true);
    fast.clock.set_speed(16);
    // Save in flight after a price change, preserving departure result.
    assert!(fast.advance((WEEK + 300) as f64 / 16.0));
    let frozen = fast.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .clone()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fuel.json");
    crate::save::save(&path, &fast).unwrap();
    let mut restored = crate::save::load(&path).unwrap();
    assert_eq!(
        frozen,
        restored.routes[0]
            .plan
            .as_ref()
            .unwrap()
            .airborne
            .clone()
            .unwrap()
    );
    assert!(fast.advance((WEEK * 11 - 300) as f64 / 16.0));
    assert!(restored.advance((WEEK * 11 - 300) as f64 / 16.0));
    assert_eq!(state(&fast), state(&restored));
    fast.clock.set_speed(1);
    assert_eq!(state(&large), state(&fast));
    for company in std::iter::once(&large).chain(large.competitors.iter().map(|c| &*c.game)) {
        assert_eq!(company.fuel_market, large.fuel_market);
        let mut checked = 0;
        for result in &company.finance.recent {
            let Some(route) = company.routes.iter().find(|r| r.id == result.route_id) else {
                continue;
            };
            let Some(plane) = company
                .fleet
                .iter()
                .find(|p| Some(p.id) == route.aircraft_id)
            else {
                continue;
            };
            let flight = crate::operations::timings(route, &plane.model_id)
                .unwrap()
                .0;
            let price = large.fuel_market.price_at(result.landing - flight);
            let expected = crate::economy::quote_at_price(
                route,
                &plane.model_id,
                &company.market_routes(),
                price,
            )
            .unwrap();
            assert_eq!(result.totals.fuel, expected.fuel);
            checked += 1;
        }
        assert!(checked > 0);
    }
}

#[test]
fn fuel_migration_keeps_airborne_costs_and_starts_at_base() {
    let mut g = game();
    // Historical pre-cabin fixture: AI cabin migration is covered in cabin_tests.
    g.competitors.clear();
    assert!(g.advance((WEEK * 3 + 300) as f64));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    crate::save::save(&path, &g).unwrap();
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let valid = old.clone();
    old["version"] = 15.into();
    old["state"].as_object_mut().unwrap().remove("fuel_market");
    for c in old["state"]["competitors"].as_array_mut().unwrap() {
        c["game"].as_object_mut().unwrap().remove("fuel_market");
    }
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let migrated = crate::save::load(&path).unwrap();
    assert_eq!(
        migrated.fuel_market.price_at(g.clock.elapsed().as_secs()),
        80
    );
    assert_eq!(migrated.finance, g.finance);
    assert_eq!(migrated.routes, g.routes);
    crate::save::save(&path, &migrated).unwrap();
    assert_eq!(state(&migrated), state(&crate::save::load(&path).unwrap()));
    for corrupt in [
        serde_json::Value::Null,
        serde_json::json!({"seed":1,"started_at":u64::MAX}),
    ] {
        let mut bad = valid.clone();
        bad["state"]["fuel_market"] = corrupt;
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(crate::save::load(&path).is_err());
    }
}

#[test]
fn fuel_departure_boundary_and_identical_company_charges() {
    let mut g = game();
    g.competitors.clear();
    g.stop_schedule(1).unwrap();
    // Depart just before a weekly update and remain airborne across it.
    g.schedule(1, (WEEK / 60 - 1) as u32, 120).unwrap();
    assert!(g.advance((WEEK - 30) as f64));
    let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
    let base =
        crate::economy::quote_at_price(&g.routes[0], "atr72_600", &g.market_routes(), 80).unwrap();
    assert_eq!(frozen.totals.fuel, base.fuel);
    assert!(g.advance(60.0));
    assert_ne!(g.fuel_market.price_at(WEEK), 80);
    assert_eq!(
        g.routes[0]
            .plan
            .as_ref()
            .unwrap()
            .airborne
            .as_ref()
            .unwrap(),
        &frozen
    );
    assert!(g.advance(86400.0));
    assert_eq!(g.finance.recent[0], frozen);
    let r = &g.routes[0];
    let exact = crate::operations::departure_quote_in_market(
        r,
        "atr72_600",
        1,
        &g.market_routes(),
        &g.fuel_market,
    )
    .unwrap();
    let at_price = crate::economy::quote_at_price(
        r,
        "atr72_600",
        &g.market_routes(),
        g.fuel_market.price_at(WEEK),
    )
    .unwrap();
    assert_eq!(exact.totals.fuel, at_price.fuel);
    // Departure exactly on the boundary uses the new week's price too.
    g.routes[0].plan.as_mut().unwrap().first_departure = WEEK;
    let exact = crate::operations::departure_quote_in_market(
        &g.routes[0],
        "atr72_600",
        0,
        &g.market_routes(),
        &g.fuel_market,
    )
    .unwrap();
    assert_eq!(exact.totals.fuel, at_price.fuel);
}
