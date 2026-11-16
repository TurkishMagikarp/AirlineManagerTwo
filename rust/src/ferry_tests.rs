use crate::{aircraft::AircraftStatus, game::Game};
use serde_json::Value;

fn state(g: &Game) -> Value {
    serde_json::to_value(g).unwrap()
}
fn game(model: &str) -> Game {
    let mut g = Game::default();
    g.found("Ferry", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.lease_aircraft(model).unwrap();
    let n = crate::staff::needs(model).unwrap();
    g.hire_pilots(&n.rating, n.pilots).unwrap();
    g.clock.set_running(true);
    g
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ferry.json");
    crate::save::save(&path, g).unwrap();
    crate::save::load(&path).unwrap()
}

#[test]
fn ferry_empty_physics_frozen_cost_once_and_no_route_results() {
    for (model, destination) in [("a330_300", "KJFK"), ("a320_200", "EDDM")] {
        let mut g = game(model);
        let before = state(&g);
        let quote = g.quote_ferry(1, destination).unwrap();
        assert_eq!(state(&g), before);
        let cash = g.cash();
        let finance = g.finance.clone();
        let reputation = g.reputation.clone();
        let route = crate::routes::Route {
            id: 0,
            origin_icao: "EDDF".into(),
            destination_icao: destination.into(),
            aircraft_id: None,
            parent_route_id: None,
            remove_at: None,
            rotation_phase: None,
            ticket_eur: 0,
            business_ticket_eur: 0,
            cargo_rate_cents: 0,
            ancillary_policy: Default::default(),
            service_level: Default::default(),
            plan: None,
        };
        assert_eq!(
            quote.quote.totals.fuel,
            crate::economy::fuel_bill(&route, model, g.fuel_market.price_at(0) * 100).unwrap()
        );
        assert_eq!(
            quote.quote.totals.airport,
            crate::economy::airport_bill(&route, model, 0).unwrap()
        );
        let overhead = if model == "a330_300" {
            1200 + ((quote.quote.landing as i64 * 4200 + 3599) / 3600)
        } else {
            700 + ((quote.quote.landing as i64 * 2400 + 3599) / 3600)
        };
        assert_eq!(quote.quote.totals.overhead, overhead);
        g.send_ferry(1, destination).unwrap();
        assert_eq!(g.cash(), cash);
        assert_eq!(g.slot_usage("EDDF"), 0);
        assert_eq!(crate::map::flights(&g)[0].destination, destination);
        let n = crate::staff::needs(model).unwrap();
        assert_eq!(g.rated_available(&n.rating), 0);
        assert_eq!(g.reserved_crew(), (n.pilots, 0));
        assert!(g.advance(quote.quote.landing as f64 - 1.0));
        assert_eq!(g.cash(), cash);
        assert_eq!(state(&restart(&g)), state(&g));
        assert!(g.advance(1.0));
        assert_eq!(g.cash(), cash - quote.quote.totals.costs());
        assert_eq!(g.fleet[0].status, AircraftStatus::Ground);
        assert_eq!(g.fleet[0].location_icao, destination);
        assert_eq!(g.fleet[0].wear.cycles, 1);
        assert_eq!(g.fleet[0].wear.flight_seconds, quote.quote.landing);
        assert_eq!(g.finance, finance);
        assert_eq!(g.reputation, reputation);
        assert_eq!(g.history.as_ref().unwrap().ferries, vec![quote.clone()]);
        assert!(crate::map::flights(&g).is_empty());
        assert_eq!(state(&restart(&g)), state(&g));
        assert!(g.advance((quote.ready - quote.quote.landing) as f64));
        assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
        assert_eq!(g.cash(), cash - quote.quote.totals.costs());
        assert!(g.ferries.is_empty());
        g.create_route(destination, "EGLL", 1).unwrap();
        g.validate_history().unwrap();
    }
}

fn rejects(g: &mut Game, id: u32, destination: &str) {
    let before = state(g);
    assert!(g.quote_ferry(id, destination).is_err());
    assert!(g.send_ferry(id, destination).is_err());
    assert_eq!(state(g), before);
}

#[test]
fn ferry_rejections_and_airborne_actions_are_transactional() {
    let mut g = game("a320_200");
    rejects(&mut g, 1, "NZAA");
    rejects(&mut g, 1, "EDDF");
    rejects(&mut g, 1, "XXXX");
    rejects(&mut g, 99, "EDDM");
    let mut missing = g.clone();
    missing.staff.employees.clear();
    rejects(&mut missing, 1, "EDDM");
    let mut wrong_rating = g.clone();
    for e in &mut wrong_rating.staff.employees {
        e.rating = Some("737".into());
    }
    rejects(&mut wrong_rating, 1, "EDDM");
    g.create_route("EDDF", "EDDM", 1).unwrap();
    rejects(&mut g, 1, "EDDM");
    g.assign_route(1, None).unwrap();
    let mut distressed = g.clone();
    distressed.airline.as_mut().unwrap().capital_eur = -1;
    distressed.resolution.deadline = Some(86400);
    rejects(&mut distressed, 1, "EDDM");
    let mut insolvent = g.clone();
    insolvent.resolution.insolvent_at = Some(0);
    rejects(&mut insolvent, 1, "EDDM");
    let mut maintenance = g.clone();
    maintenance.fleet[0].wear = maintenance.fleet[0].wear.added(1, 100).unwrap();
    maintenance.request_maintenance(1).unwrap();
    rejects(&mut maintenance, 1, "EDDM");
    g.send_ferry(1, "EDDM").unwrap();
    rejects(&mut g, 1, "EGLL");
    let before = state(&g);
    assert!(g.sell_aircraft(1).is_err());
    assert!(g.return_aircraft(1).is_err());
    assert!(g.set_cabin_layout(1, 4).is_err());
    assert!(g.create_route("EDDF", "EGLL", 1).is_err());
    assert!(g.assign_route(1, Some(1)).is_err());
    assert!(g.train_pilot(g.staff.employees[0].id, "737").is_err());
    assert!(g.dismiss_staff(g.staff.employees[0].id).is_err());
    assert_eq!(state(&g), before);
}

#[test]
fn ferry_steps_speeds_pause_restart_and_previous_save_migration() {
    let initial = game("a330_300");
    let mut baseline = initial.clone();
    baseline.send_ferry(1, "KJFK").unwrap();
    assert!(baseline.advance(7.0 * 86400.0));
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        g.send_ferry(1, "KJFK").unwrap();
        assert!(g.advance(1000.125 / f64::from(speed)));
        g.clock.set_running(false);
        let before = state(&g);
        assert!(g.advance(1e6));
        assert_eq!(state(&g), before);
        g = restart(&g);
        g.clock.set_running(true);
        assert!(g.advance((86400.0 - 1000.125) / f64::from(speed)));
        for _ in 1..7 {
            assert!(g.advance(86400.0 / f64::from(speed)));
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&baseline));
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("previous.json");
    crate::save::save(&path, &initial).unwrap();
    let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["version"] = 30.into();
    v["state"].as_object_mut().unwrap().remove("ferries");
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let mut expected = initial.clone();
    expected.migrate_world_economy_fixture();
    assert_eq!(state(&crate::save::load(&path).unwrap()), state(&expected));
}

#[test]
fn ferry_maintenance_starts_at_destination_with_current_base_pricing() {
    for base in [false, true] {
        let mut g = game("a320_200");
        let n = crate::staff::needs("a320_200").unwrap();
        g.hire_staff(crate::staff::Role::Technician, n.technicians)
            .unwrap();
        g.send_ferry(1, "EDDM").unwrap();
        let ferry = g.ferries[0].clone();
        g.request_maintenance(1).unwrap();
        if base {
            g.open_maintenance_base("EDDM").unwrap();
        }
        let cash = g.cash();
        let mut replay = restart(&g);
        assert!(g.advance(ferry.quote.landing as f64));
        assert!(replay.advance(ferry.quote.landing as f64));
        assert_eq!(state(&g), state(&replay));
        let visit = g.fleet[0].maintenance.as_ref().unwrap();
        let expected = g
            .check_at(
                "EDDM",
                &g.fleet[0].wear,
                crate::maintenance::rules("a320_200").unwrap(),
                false,
            )
            .unwrap();
        assert_eq!(visit.start, ferry.quote.landing);
        assert_eq!((visit.cost_eur, visit.end - visit.start), expected);
        assert_eq!(g.cash(), cash - ferry.quote.totals.costs() - expected.0);
        assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
        assert_eq!(g.fleet[0].location_icao, "EDDM");
        assert!(g.ferries.is_empty());
        assert_eq!(state(&restart(&g)), state(&g));
    }
}

#[test]
fn ferry_forced_check_and_debt_settle_without_losing_dispatch() {
    let mut g = game("a320_200");
    let rules = crate::maintenance::rules("a320_200").unwrap();
    g.fleet[0].wear.cycles = rules.interval_cycles - 1;
    g.fleet[0].wear.check_cycles = rules.interval_cycles - 1;
    g.send_ferry(1, "EDDM").unwrap();
    let f = g.ferries[0].clone();
    // Operating costs, including mandatory checks, may create debt at landing.
    g.airline.as_mut().unwrap().capital_eur = 1;
    let mut h = crate::history::History::new(0, 1);
    h.purchases.clear();
    g.history = Some(h);
    assert!(g.advance(f.quote.landing as f64));
    let visit = g.fleet[0].maintenance.as_ref().unwrap();
    assert!(visit.unscheduled);
    assert_eq!(visit.start, f.quote.landing);
    assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
    assert_eq!(g.cash(), 1 - f.quote.totals.costs() - visit.cost_eur);
    assert!(g.ferries.is_empty());
    assert_eq!(state(&restart(&g)), state(&g));
}

#[test]
fn ferry_save_rejects_corrupt_dispatch_and_archive() {
    let mut g = game("a320_200");
    g.send_ferry(1, "EDDM").unwrap();
    for change in 0..5 {
        let mut bad = g.clone();
        match change {
            0 => bad.ferries[0].quote.totals.revenue = 1,
            1 => bad.ferries[0].quote.landing += 1,
            2 => bad.ferries[0].settled = true,
            3 => bad.fleet[0].status = AircraftStatus::Parked,
            _ => bad.ferries.push(bad.ferries[0].clone()),
        }
        assert!(bad.validate_operations().is_err());
    }
    let landing = g.ferries[0].quote.landing;
    assert!(g.advance(landing as f64));
    g.history.as_mut().unwrap().ferries[0].quote.totals.fuel += 1;
    assert!(g.validate_ferries().is_err());
}
