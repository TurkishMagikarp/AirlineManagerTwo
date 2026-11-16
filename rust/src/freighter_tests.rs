use crate::{aircraft, cargo, economy, game::Game, save};
const DAY: f64 = 86400.0;
fn fresh(ai: bool) -> Game {
    let mut g = Game::default();
    g.found("Freighter", "DE", "EDDF").unwrap();
    if !ai {
        g.competitors.clear();
    }
    g
}
fn service(ai: bool) -> Game {
    let mut g = fresh(ai);
    g.buy_aircraft("atr72_600f").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
fn restart(g: &Game) -> Game {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("freighter.json");
    save::save(&p, g).unwrap();
    save::load(&p).unwrap()
}
#[test]
fn freighter_revenue_no_passenger_costs_frozen_quote_and_v36_identity() {
    let mut g = service(false);
    assert_eq!(crate::staff::needs("atr72_600f").unwrap().cabin_crew, 0);
    assert!(
        g.staff
            .employees
            .iter()
            .all(|p| p.role != crate::staff::Role::CabinCrew)
    );
    g.set_service_level(1, crate::service::Level::Premium)
        .unwrap();
    assert!(g.advance(60.0));
    let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
    let t = &frozen.totals;
    assert!(t.cargo_kg > 0 && t.cargo_revenue > 0);
    assert_eq!(t.revenue, t.cargo_revenue);
    assert_eq!(
        (
            t.passengers,
            t.seats,
            t.business_passengers,
            t.connecting_passengers,
            t.service,
            t.care,
            t.crew
        ),
        (0, 0, 0, 0, 0, 0, 0)
    );
    assert_eq!(
        t.airport,
        economy::airport_bill(&g.routes[0], "atr72_600f", 0).unwrap()
    );
    g.set_cargo_rate(1, 10000).unwrap();
    let mut loaded = restart(&g);
    assert_eq!(
        loaded.routes[0].plan.as_ref().unwrap().airborne.as_ref(),
        Some(&frozen)
    );
    assert!(loaded.advance(4000.0));
    assert_eq!(loaded.finance.totals.cargo_revenue, t.cargo_revenue);
    loaded.validate_history().unwrap();
    // Previous-version worlds contain no freighters; identity migration retains every quote.
    let mut old = fresh(true);
    old.buy_aircraft("atr72_600").unwrap();
    old.staff_up();
    old.create_route("EDDF", "EDDM", 1).unwrap();
    old.schedule(1, 1, 120).unwrap();
    old.clock.set_running(true);
    assert!(old.advance(60.0));
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("v36.json");
    save::save(&p, &old).unwrap();
    let text = std::fs::read_to_string(&p).unwrap().replacen(
        &format!("\"version\":{}", save::VERSION),
        "\"version\":36",
        1,
    );
    assert!(text.contains("\"version\":36"));
    std::fs::write(&p, text).unwrap();
    let loaded = save::load(&p).unwrap();
    old.adopt_events_from(&loaded);
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        serde_json::to_value(loaded).unwrap()
    );
}
#[test]
fn freighter_pools_shared_conserved_and_passenger_belly_unchanged() {
    let mut pax = fresh(false);
    pax.buy_aircraft("atr72_600").unwrap();
    pax.staff_up();
    pax.create_route("EDDF", "EDDM", 1).unwrap();
    pax.schedule(1, 1, 120).unwrap();
    let before = economy::quote_shared(&pax.routes[0], "atr72_600", &pax.market_routes()).unwrap();
    let mut freight = service(false);
    freight.routes[0].id = 9;
    freight.routes[0].aircraft_id = Some(9);
    freight.fleet[0].id = 9;
    let mut supply = pax.market_routes();
    supply.extend(freight.market_routes());
    let after = economy::quote_shared(&pax.routes[0], "atr72_600", &supply).unwrap();
    assert_eq!(before, after);
    let solo = cargo::estimate(&freight.routes[0], "atr72_600f", &supply, None).unwrap();
    let mut rival = freight.clone();
    rival.routes[0].id = 10;
    rival.fleet[0].id = 10;
    rival.routes[0].aircraft_id = Some(10);
    rival.routes[0].cargo_rate_cents = 300;
    rival.routes[0].plan.as_mut().unwrap().interval_seconds = 14400;
    supply.extend(rival.market_routes());
    let a = cargo::estimate(&freight.routes[0], "atr72_600f", &supply, None).unwrap();
    let b = cargo::estimate(&rival.routes[0], "atr72_600f", &supply, None).unwrap();
    assert!(a > 0 && b > 0 && a < solo);
    assert!(
        a as f64 * 6.0 + b as f64 * 3.0
            <= cargo::daily_demand_for(&freight.routes[0], true).unwrap() * 0.75
    );
    assert_eq!(
        before,
        economy::quote_shared(&pax.routes[0], "atr72_600", &supply).unwrap()
    );
    // Passenger and freighter services also share one public route.
    pax.lease_aircraft("atr72_600f").unwrap();
    pax.staff_up();
    pax.add_route_aircraft(1, 2).unwrap();
    assert_eq!(pax.routes.len(), 2);
    pax.clock.set_running(true);
    assert!(pax.advance(DAY));
    let freight_service = pax.history.as_ref().unwrap().settled_service_totals(2);
    assert!(freight_service.cargo_kg > 0);
    assert_eq!(freight_service.passengers, 0);
    assert_eq!(freight_service.revenue, freight_service.cargo_revenue);
    assert!(pax.finance.totals.passengers > 0);
    pax.validate_history().unwrap();
}
#[test]
fn freighter_maintenance_trading_lease_and_ferry() {
    for model in ["atr72_600f", "b737_800bcf", "b777f"] {
        let mut g = fresh(false);
        g.lease_aircraft(model).unwrap();
        g.staff_up();
        assert_eq!(aircraft::model(model).unwrap().seats, 0);
        g.send_ferry(1, "EDDM").unwrap();
        g.clock.set_running(true);
        assert!(g.advance(DAY));
        assert_eq!(g.fleet[0].location_icao, "EDDM");
        assert_eq!(g.finance.totals.revenue, 0);
        g.request_maintenance(1).unwrap();
        assert!(g.advance(5.0 * DAY));
        assert!(g.fleet[0].maintenance.is_none());
        g.return_aircraft(1).unwrap();
        assert!(g.fleet.is_empty());
        g.validate_history().unwrap();
    }
    let mut g = fresh(false);
    g.buy_aircraft("atr72_600f").unwrap();
    g.sell_aircraft(1).unwrap();
    let listing = g.used_market.listings[0].id;
    g.buy_used_aircraft(listing).unwrap();
    assert_eq!(g.fleet[0].model_id, "atr72_600f");
    g.validate_history().unwrap();
}
#[test]
fn freighter_two_year_speeds_steps_pause_restart_with_ai() {
    let mut initial = service(true);
    initial.stop_schedule(1).unwrap();
    initial.schedule(1, 1, 1440).unwrap();
    let mut fast = initial.clone();
    assert!(fast.advance(730.0 * DAY));
    assert!(!fast.insolvent() && !fast.distressed());
    assert!(fast.finance.totals.flights > 600);
    assert!(fast.finance.totals.profit() > 0);
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_running(false);
        assert!(g.advance(DAY));
        assert_eq!(g.clock.elapsed(), initial.clock.elapsed());
        g.clock.set_running(true);
        g.clock.set_speed(speed);
        for day in 0..730 {
            assert!(g.advance(DAY / f64::from(speed)));
            if day == 364 {
                g = restart(&g);
            }
        }
        g.clock.set_speed(1);
        g.validate_history().unwrap();
        g.validate_competitors().unwrap();
        assert_eq!(
            serde_json::to_value(&fast).unwrap(),
            serde_json::to_value(&g).unwrap()
        );
    }
}
#[test]
fn freighter_rival_comparison_two_and_ten_years() {
    let mut g = fresh(true);
    g.clock.set_running(true);
    for day in 1..=3650 {
        assert!(g.advance(DAY));
        assert_eq!(g.competitors.len(), 2);
        for c in &g.competitors {
            assert!(!c.game.insolvent() && !c.game.distressed());
        }
        if day == 730 || day == 3650 {
            for c in &g.competitors {
                println!(
                    "FREIGHT_AUDIT day={day} airline={} passengers={} cash={} profit={} freighters={} cargo={}",
                    c.game.airline.as_ref().unwrap().name,
                    c.game.finance.totals.passengers,
                    c.game.cash(),
                    c.game.finance.totals.profit(),
                    c.game
                        .fleet
                        .iter()
                        .filter(|p| aircraft::model(&p.model_id).unwrap().seats == 0)
                        .count(),
                    c.game.finance.totals.cargo_kg
                );
            }
        }
    }
    g.validate_competitors().unwrap();
}

#[test]
fn freighter_ai_books_the_same_departure_quote() {
    let mut world = fresh(true);
    let mut freight = service(false);
    freight.airline.as_mut().unwrap().name = "Regional Freight".into();
    freight.stop_schedule(1).unwrap();
    freight.schedule(1, 1, 1440).unwrap();
    let expected = economy::quote_at_time(
        &freight.routes[0],
        "atr72_600f",
        &freight.market_routes(),
        freight.fuel_market.price_at(0),
        60,
    )
    .unwrap();
    world.competitors[0].game = Box::new(freight);
    world.clock.set_running(true);
    assert!(world.advance(4000.0));
    let ai = &world.competitors[0].game;
    assert_eq!(ai.finance.totals.cargo_kg, expected.cargo_kg);
    assert_eq!(ai.finance.totals.revenue, expected.cargo_revenue);
    assert_eq!(
        (
            ai.finance.totals.passengers,
            ai.finance.totals.service,
            ai.finance.totals.care
        ),
        (0, 0, 0)
    );
    world.validate_competitors().unwrap();
}
