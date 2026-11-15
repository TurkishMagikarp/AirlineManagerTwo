use crate::{aircraft, economy, game::Game, operations};

fn game(model: &str, origin: &str, destination: &str) -> Game {
    let mut g = Game::default();
    let country = &crate::airports::catalog()
        .unwrap()
        .airports
        .iter()
        .find(|a| a.icao == origin)
        .unwrap()
        .country_code;
    g.found("Widebody audit", country, origin).unwrap();
    g.competitors.clear();
    // Scenario capital only; production financing and AI budgets stay unchanged.
    g.airline.as_mut().unwrap().capital_eur += 100_000_000;
    g.history.as_mut().unwrap().opening_balance += 100_000_000;
    g.buy_aircraft(model).unwrap();
    g.staff_up();
    g.set_cabin_layout(1, 30).unwrap();
    g.create_route(origin, destination, 1).unwrap();
    let (flight, ground) = operations::timings(&g.routes[0], model).unwrap();
    g.schedule(1, 1, (flight + ground).div_ceil(60) as u32)
        .unwrap();
    g
}

#[test]
fn widebody_longhaul_fare_audit() {
    for (model, origin, destination) in [
        ("a330_300", "EDDF", "KJFK"),
        ("a330_300", "EDDF", "RJTT"),
        ("a330_300", "SBGR", "EDDF"),
        ("b777_300er", "RJTT", "KLAX"),
    ] {
        let mut g = game(model, origin, destination);
        g.routes[0].plan.as_mut().unwrap().interval_seconds = 86400;
        let mut best = (i64::MIN, 0, 0, economy::Totals::default());
        for business in (0..=aircraft::model(model).unwrap().seats / 2).step_by(10) {
            g.fleet[0].business_seats = business;
            for fare in (100..=1000).step_by(10) {
                g.set_ticket_price(1, fare).unwrap();
                g.set_business_ticket_price(1, fare * 2).unwrap();
                let q = economy::quote_at_price(
                    &g.routes[0],
                    model,
                    &g.market_routes(),
                    economy::fuel_mean(),
                )
                .unwrap();
                if q.profit() > best.0 {
                    best = (q.profit(), fare, business, q);
                }
            }
        }
        let (profit, fare, business, q) = best;
        assert!(
            profit > 0,
            "no profitable cabin/fare for {origin}–{destination}"
        );
        println!(
            "LONGHAUL {model} {origin}–{destination} best fare={fare} business_seats={business} pax={} business={} revenue={} costs={} cargo={} contribution={profit}",
            q.passengers,
            q.business_passengers,
            q.revenue,
            q.costs(),
            q.cargo_kg
        );
        assert_eq!(aircraft::model(model).unwrap().class, "Widebody");
    }
}

fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("widebody.json");
    crate::save::save(&path, g).unwrap();
    crate::save::load(&path).unwrap()
}

#[test]
fn widebody_long_legs_maintenance_delays_and_range() {
    for model in ["a330_300", "b777_300er"] {
        let mut g = game(model, "EDDF", "RJTT");
        let (flight, ground) = operations::timings(&g.routes[0], model).unwrap();
        assert!((10 * 3600..14 * 3600).contains(&flight));
        assert_eq!(
            ground,
            u64::from(aircraft::model(model).unwrap().turnaround_minutes) * 60
        );
        g.set_ticket_price(1, 450).unwrap();
        g.set_business_ticket_price(1, 900).unwrap();
        g.clock.set_running(true);
        // Select a real deterministic technical delay on the long leg.
        let seed = (0..100_000)
            .find(|seed| {
                crate::disruptions::technical_delay(*seed, &g.fleet[0].wear, model, 60, 0).unwrap()
                    > 0
            })
            .unwrap();
        g.routes[0]
            .plan
            .as_mut()
            .unwrap()
            .disruption
            .as_mut()
            .unwrap()
            .seed = seed;
        let mut cursor = g.routes[0]
            .plan
            .as_ref()
            .unwrap()
            .disruption
            .clone()
            .unwrap();
        let first = cursor.next(&g.routes[0], model).unwrap();
        assert!(first.departure > 60);
        assert_eq!(first.landing - first.departure, flight);
        cursor.previous = Some(first.clone());
        cursor.index += 1;
        let second = cursor.next(&g.routes[0], model).unwrap();
        assert!(second.departure >= first.landing + ground);
        assert!(g.advance(first.departure as f64));
        let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
        g.request_maintenance(1).unwrap();
        let visit = g.fleet[0].maintenance.clone().unwrap();
        assert_eq!(visit.start, first.landing);
        assert!(
            visit.end
                >= visit.start + crate::maintenance::rules(model).unwrap().duration_hours * 3600
        );
        g = restart(&g);
        assert!(g.advance((first.landing - first.departure) as f64));
        assert_eq!(g.finance.totals, frozen.totals);
        assert!(g.finance.totals.cargo_revenue > 0 && g.finance.totals.fuel > 0);
        assert_eq!(g.fleet[0].wear.flight_seconds, flight);
        assert_eq!(g.fleet[0].status, aircraft::AircraftStatus::Maintenance);
        assert!(g.advance((visit.end - first.landing) as f64));
        assert_eq!(g.fleet[0].wear.check_seconds, 0);
        g.validate_operations().unwrap();
        g.validate_maintenance().unwrap();
        g.validate_history().unwrap();
        assert_eq!(
            serde_json::to_value(restart(&g)).unwrap(),
            serde_json::to_value(&g).unwrap()
        );

        let mut out = Game::default();
        out.found("Range audit", "DE", "EDDF").unwrap();
        out.lease_aircraft(model).unwrap();
        let before = serde_json::to_value(&out).unwrap();
        let err = out.create_route("EDDF", "YSSY", 1).unwrap_err();
        assert!(err.starts_with("Reichweite überschritten:"));
        assert_eq!(before, serde_json::to_value(&out).unwrap());
    }
}

#[test]
fn widebody_buy_lease_used_market_and_ratings() {
    for model in ["a330_300", "b777_300er"] {
        let mut g = game(model, "EDDF", "KJFK");
        g.stop_schedule(1).unwrap();
        let price = aircraft::model(model).unwrap().price_eur;
        assert_eq!(g.fleet[0].purchase_price_eur, price);
        assert_eq!(g.disposal_quote(1).unwrap(), price * 80 / 100);
        g.sell_aircraft(1).unwrap();
        let listing = g.used_market.listings[0].id;
        g.buy_used_aircraft(listing).unwrap();
        assert_eq!(g.fleet[0].model_id, model);
        let before = g.cash();
        let (deposit, monthly) = crate::trading::lease_terms(model).unwrap();
        g.lease_aircraft(model).unwrap();
        assert_eq!(g.cash(), before - deposit);
        assert_eq!(g.fleet[1].lease.as_ref().unwrap().monthly, monthly);
        let rating = &crate::staff::needs(model).unwrap().rating;
        assert!(crate::staff::ratings().contains(rating));
        g.validate_trading().unwrap();
        g.validate_history().unwrap();
        restart(&g);
    }
}

#[test]
fn widebody_world_steps_speeds_pause_and_annual_restart() {
    let mut initial = game("a330_300", "EDDF", "RJTT");
    initial.set_cabin_layout(1, 60).unwrap();
    initial.set_ticket_price(1, 450).unwrap();
    initial.set_business_ticket_price(1, 900).unwrap();
    initial.competitors = crate::competition::initial_competitors().unwrap();
    initial.clock.set_running(true);
    let mut weekly = initial.clone();
    for _ in 0..104 {
        assert!(weekly.advance(7.0 * 86400.0));
    }
    assert!(weekly.advance(2.0 * 86400.0));
    println!(
        "WIDEBODY_TWO_YEAR opening={} final={} flights={} cargo={} revenue={} flight_costs={}",
        initial.cash(),
        weekly.cash(),
        weekly.finance.totals.flights,
        weekly.finance.totals.cargo_kg,
        weekly.finance.totals.revenue,
        weekly.finance.totals.costs()
    );
    assert!(!weekly.distressed());
    let expected = serde_json::to_value(&weekly).unwrap();
    for speed in [1, 4, 16] {
        let mut daily = initial.clone();
        daily.clock.set_running(false);
        let paused = serde_json::to_value(&daily).unwrap();
        assert!(daily.advance(86400.0));
        assert_eq!(serde_json::to_value(&daily).unwrap(), paused);
        daily.clock.set_running(true);
        daily.clock.set_speed(speed);
        for day in 0..730 {
            assert!(daily.advance(86400.0 / f64::from(speed)));
            if day == 364 {
                daily = restart(&daily);
            }
        }
        daily.clock.set_speed(1);
        assert_eq!(
            serde_json::to_value(&daily).unwrap(),
            expected,
            "speed={speed}"
        );
    }
    assert!(weekly.finance.totals.flights > 100);
    assert!(
        weekly
            .history
            .as_ref()
            .unwrap()
            .maintenance_through(u64::MAX)
            > 0
    );
}

#[test]
fn widebody_pacific_map_and_staggered_rotation() {
    let mut g = game("b777_300er", "RJTT", "KLAX");
    g.lease_aircraft("b777_300er").unwrap();
    g.equip_staff_for("b777_300er").unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    let a = g.routes[0].plan.as_ref().unwrap();
    let b = g.routes[1].plan.as_ref().unwrap();
    assert_eq!(
        b.first_departure - a.first_departure,
        a.interval_seconds / 2
    );
    let flight = operations::timings(&g.routes[0], "b777_300er").unwrap().0;
    g.clock.set_running(true);
    assert!(g.advance((60 + flight / 2) as f64));
    let flights = crate::map::flights(&g);
    let first = flights.iter().find(|f| f.aircraft_id == 1).unwrap();
    assert_eq!((&*first.origin, &*first.destination), ("RJTT", "KLAX"));
    assert!(first.position.0.abs() > 160.0);
    assert!(first.position.1 > 40.0);
    assert!((first.progress - 0.5).abs() < 0.01);
    assert_eq!(crate::map::flights(&restart(&g)), flights);
    g.validate_operations().unwrap();
}

#[test]
fn single_cabin_fare_sweep_matches_fresh_quotes() {
    for (model, origin, destination) in [
        ("a330_300", "EDDF", "RJTT"),
        ("b777_300er", "SBGR", "EDDF"),
        ("a320_200", "EDDF", "EDDM"),
    ] {
        let mut g = game(model, origin, destination);
        for percent in [0, 20, 40, 50, 100] {
            let business = crate::aircraft::model(model).unwrap().seats * percent
                / 100
                / crate::cabins::RULES.business_space_ratio;
            g.set_cabin_layout(1, business).unwrap();
            let mut prepared =
                economy::QuoteMarket::new(&g.routes[0], model, &g.market_routes()).unwrap();
            for fare in [0, 100, 400, 800, 100_000, 800, 400, 100, 0] {
                g.set_ticket_price(1, fare).unwrap();
                g.set_business_ticket_price(1, (fare * 2).min(100_000))
                    .unwrap();
                let route = &g.routes[0];
                let supply = g.market_routes();
                prepared.reprice_single(route);
                for fuel in [50, 80, 120] {
                    let fresh = economy::quote_at_price(route, model, &supply, fuel).unwrap();
                    let reused =
                        economy::quote_prepared(route, model, &supply, fuel, None, &mut prepared)
                            .unwrap();
                    assert_eq!(reused, fresh, "{model} fare={fare} fuel={fuel}");
                }
            }
        }
    }
}
