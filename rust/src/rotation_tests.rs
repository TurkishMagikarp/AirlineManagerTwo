use crate::{aircraft::AircraftStatus, game::Game, operations, save};

fn game(planes: u32) -> Game {
    let mut g = Game::default();
    g.found("Rotation Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.airline.as_mut().unwrap().capital_eur += 20_000_000;
    g.history.as_mut().unwrap().opening_balance += 20_000_000;
    for _ in 0..planes {
        g.buy_aircraft("atr72_600").unwrap();
    }
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g
}

fn state(g: &Game) -> serde_json::Value {
    let mut value = serde_json::to_value(g).unwrap();
    value.as_object_mut().unwrap().remove("clock");
    value
}

#[test]
fn rotation_staggers_two_and_three_aircraft_and_checks_each_crew() {
    let mut g = game(3);
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    assert_eq!(g.routes[0].plan.as_ref().unwrap().first_departure, 60);
    assert_eq!(g.routes[1].plan.as_ref().unwrap().first_departure, 7260);
    g.add_route_aircraft(1, 3).unwrap();
    let departures: Vec<_> = g
        .routes
        .iter()
        .map(|r| r.plan.as_ref().unwrap().first_departure)
        .collect();
    assert_eq!(departures, vec![60, 4860, 9660]);
    assert!(g.add_route_aircraft(1, 2).is_err());
    let mut unstaffed = game(2);
    unstaffed.staff.employees.truncate(0);
    unstaffed.add_route_aircraft(1, 2).unwrap();
    let before = state(&unstaffed);
    assert!(unstaffed.schedule(1, 1, 240).is_err());
    assert_eq!(state(&unstaffed), before);
}

#[test]
fn rotation_midflight_addition_preserves_quote_then_rephases_after_turnaround() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(100.0));
    let quote = g.routes[0].plan.as_ref().unwrap().airborne.clone();
    g.add_route_aircraft(1, 2).unwrap();
    assert_eq!(g.routes[0].plan.as_ref().unwrap().airborne, quote);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pending-phase.json");
    save::save(&path, &g).unwrap();
    let mut loaded = save::load(&path).unwrap();
    let ready = operations::movement(&g.routes[0], "atr72_600", 100)
        .unwrap()
        .ready;
    assert!(g.advance((ready - 100) as f64));
    assert!(loaded.advance((ready - 100) as f64));
    assert_eq!(state(&loaded), state(&g));
    assert!(g.routes[0].rotation_phase.is_none());
    assert_eq!(g.finance.totals.flights, 1);
    assert_eq!(g.finance.recent[0], quote.unwrap());
    assert_eq!(g.fleet[0].location_icao, "EDDM");
    assert!(g.routes[1].plan.is_some());
    g.validate_operations().unwrap();
}

#[test]
fn rotation_airborne_removal_finishes_once_and_survives_restart() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(100.0));
    let other = g.routes[1].clone();
    g.remove_route_aircraft(1, 1).unwrap();
    let ready = g.routes[0].remove_at.unwrap();
    assert_eq!(g.fleet[0].status, AircraftStatus::InFlight);
    assert_eq!(g.routes[1], other);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rotation.json");
    save::save(&path, &g).unwrap();
    let mut loaded = save::load(&path).unwrap();
    assert_eq!(state(&loaded), state(&g));
    assert!(g.advance((ready - 100) as f64));
    assert!(loaded.advance((ready - 100) as f64));
    assert_eq!(state(&loaded), state(&g));
    assert_eq!(g.routes[0].aircraft_id, None);
    assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
    assert_eq!(g.fleet[0].location_icao, "EDDM");
    assert_eq!(g.finance.totals.flights, 1);
    assert!(g.advance(86400.0));
    assert_eq!(g.fleet[0].wear.cycles, 1);
    assert!(g.fleet[1].wear.cycles > 0);
    assert_eq!(
        g.history
            .as_ref()
            .unwrap()
            .totals(1, g.clock.elapsed().as_secs()),
        g.finance.totals
    );
    save::save(&path, &g).unwrap();
    assert_eq!(state(&save::load(&path).unwrap()), state(&g));
}

#[test]
fn rotation_sale_return_and_maintenance_affect_only_the_selected_aircraft() {
    let mut g = game(2);
    g.lease_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    g.add_route_aircraft(1, 3).unwrap();
    let primary = g.routes[0].clone();
    g.return_aircraft(3).unwrap();
    g.sell_aircraft(2).unwrap();
    assert_eq!(g.routes[0], primary);
    assert_eq!(g.routes[1].aircraft_id, None);
    assert_eq!(g.routes[2].aircraft_id, None);
    g.clock.set_running(true);
    assert!(g.advance(100.0));
    g.request_maintenance(1).unwrap();
    let landing = g.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .as_ref()
        .unwrap()
        .landing;
    assert!(g.advance((landing - 100) as f64));
    assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
    g.validate_operations().unwrap();
    g.validate_maintenance().unwrap();
}

#[test]
fn rotation_v14_migrates_single_assignment_and_v15_rejects_inconsistent_lists() {
    let mut g = game(1);
    g.competitors = crate::competition::initial_competitors().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("migration.json");
    save::save(&path, &g).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["version"], crate::save::VERSION);
    assert_eq!(
        value["state"]["routes"]["routes"][0]["aircraft_ids"],
        serde_json::json!([1])
    );
    let valid = value.clone();
    value["version"] = 14.into();
    let route = value["state"]["routes"]["routes"][0]
        .as_object_mut()
        .unwrap();
    route.remove("aircraft_ids");
    route.remove("rotations");
    route.insert("aircraft_id".into(), 1.into());
    for competitor in value["state"]["competitors"].as_array_mut().unwrap() {
        for route in competitor["game"]["routes"].as_array_mut().unwrap() {
            let object = route.as_object_mut().unwrap();
            let id = object.remove("aircraft_ids").unwrap()[0].clone();
            object.remove("rotations");
            object.insert("aircraft_id".into(), id);
        }
    }
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    g.migrate_insurance_fixture();
    g.migrate_world_economy_fixture();
    assert_eq!(state(&save::load(&path).unwrap()), state(&g));
    for ids in [
        serde_json::json!([1, 1]),
        serde_json::json!([1, 2]),
        serde_json::json!([99]),
    ] {
        let mut bad = valid.clone();
        bad["state"]["routes"]["routes"][0]["aircraft_ids"] = ids;
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(save::load(&path).is_err());
    }
}

#[test]
fn rotation_shared_market_speeds_steps_and_save_load_are_identical() {
    let mut g = game(2);
    g.competitors = crate::competition::initial_competitors().unwrap();
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    let supply = g.market_routes();
    assert_eq!(supply.iter().filter(|o| crate::demand::same_market(&g.routes[0], &o.route) && o.route.plan.is_some()).count(), 3);
    let passengers: f64 = supply
        .iter()
        .filter(|o| crate::demand::same_market(&g.routes[0], &o.route))
        .map(|o| {
            crate::demand::estimate(&o.route, o.seats, &supply)
                .unwrap()
                .daily_passengers
        })
        .sum();
    assert!(passengers <= crate::demand::market(&g.routes[0], 0).unwrap().total());
    g.clock.set_running(true);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("speeds.json");
    save::save(&path, &g).unwrap();
    let mut large = save::load(&path).unwrap();
    assert!(large.advance(3.0 * 86400.0));
    for speed in [1, 4, 16] {
        let mut small = save::load(&path).unwrap();
        small.clock.set_speed(speed);
        for _ in 0..432 {
            assert!(small.advance(600.0 / f64::from(speed)));
        }
        assert_eq!(state(&small), state(&large));
        save::save(&path, &small).unwrap();
        let loaded = save::load(&path).unwrap();
        assert_eq!(state(&loaded), state(&large));
        // Reset fixture for the next speed.
        save::save(&path, &g).unwrap();
    }
    large.validate_competitors().unwrap();
}

#[test]
fn rotation_zero_gap_removal_does_not_dispatch_an_extra_flight() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    let (flight, ground) = operations::timings(&g.routes[0], "atr72_600").unwrap();
    g.routes[0].plan.as_mut().unwrap().interval_seconds = flight + ground;
    g.clock.set_running(true);
    assert!(g.advance(100.0));
    g.remove_route_aircraft(1, 1).unwrap();
    assert!(g.advance((60 + flight + ground - 100) as f64));
    assert_eq!(g.finance.totals.flights, 1);
    assert_eq!(g.fleet[0].wear.cycles, 1);
    assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
    assert!(g.routes[0].plan.is_none());
    assert!(g.routes[1].plan.is_some());
    g.validate_operations().unwrap();
}

#[test]
fn rotation_maintenance_rejoins_its_phase_and_preserves_other_services() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(100.0));
    let other = g.routes[1].plan.clone();
    let phase = g.routes[0].plan.as_ref().unwrap().first_departure % 14400;
    g.request_maintenance(1).unwrap();
    assert_eq!(g.routes[1].plan, other);
    let end = g.fleet[0].maintenance.as_ref().unwrap().end;
    assert!(g.advance((end - 100) as f64));
    assert_eq!(
        g.routes[0].plan.as_ref().unwrap().first_departure % 14400,
        phase
    );
    assert_eq!(
        g.routes[1].plan.as_ref().unwrap().first_departure,
        other.unwrap().first_departure
    );
    assert!(g.routes[1].plan.is_some());
    let dir = tempfile::tempdir().unwrap();
    save::save(&dir.path().join("maint.json"), &g).unwrap();
}

#[test]
fn rotation_addition_checks_range_location_interval_and_available_staff_transactionally() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.fleet[1].location_icao = "EDDM".into();
    let before = state(&g);
    assert!(g.add_route_aircraft(1, 2).is_err());
    assert_eq!(state(&g), before);
    g.fleet[1].location_icao = "EDDF".into();
    let needs = crate::staff::needs("atr72_600").unwrap();
    let mut pilots = 0;
    g.staff.employees.retain(|e| {
        if e.role == crate::staff::Role::Pilot {
            pilots += 1;
            pilots <= needs.pilots
        } else {
            true
        }
    });
    let before = state(&g);
    assert!(g.add_route_aircraft(1, 2).is_err());
    assert_eq!(state(&g), before);
    let mut long = game(1);
    long.buy_aircraft("a320_200").unwrap();
    long.staff_up();
    long.create_route("EDDF", "LTFM", 2).unwrap();
    let before = state(&long);
    assert!(long.add_route_aircraft(2, 1).is_err());
    assert_eq!(state(&long), before);
}

#[test]
fn rotation_distress_stops_parked_service_while_other_aircraft_finishes() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(7300.0));
    assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
    assert_eq!(g.fleet[1].status, AircraftStatus::InFlight);
    let delta = -g.airline.as_ref().unwrap().capital_eur - 1_000_000;
    g.airline.as_mut().unwrap().capital_eur += delta;
    g.history.as_mut().unwrap().opening_balance += delta;
    g.apply_financing().unwrap();
    assert!(g.routes[0].plan.is_none());
    assert!(g.routes[1].plan.is_some());
    let ready = operations::movement(&g.routes[1], "atr72_600", 7300)
        .unwrap()
        .ready;
    assert!(g.advance((ready - 7300) as f64));
    assert!(g.routes.iter().all(|r| r.plan.is_none()));
    assert!(g.fleet.iter().all(|p| p.status == AircraftStatus::Parked));
    g.validate_operations().unwrap();
    g.validate_financing().unwrap();
    g.validate_history().unwrap();
}

#[test]
fn rotation_archive_pages_and_totals_stay_grouped_after_deletion() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(20000.0));
    let h = g.history.as_ref().unwrap();
    let expected: Vec<_> = g.finance.recent.iter().rev().cloned().collect();
    assert_eq!(h.page(1, 0, 50), expected);
    assert_eq!(h.page(1, 1, 1), vec![expected[1].clone()]);
    g.stop_schedule(1).unwrap();
    g.delete_route(1).unwrap();
    assert!(g.routes.is_empty());
    assert_eq!(
        g.history.as_ref().unwrap().totals(1, 20000),
        g.finance.totals
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deleted.json");
    save::save(&path, &g).unwrap();
    assert_eq!(state(&save::load(&path).unwrap()), state(&g));
}

fn board(g: &Game) -> Vec<operations::BoardRow> {
    let now = g.clock.elapsed().as_secs();
    g.routes
        .iter()
        .filter_map(|r| {
            let plane = g.fleet.iter().find(|p| Some(p.id) == r.aircraft_id)?;
            let state = operations::movement(r, &plane.model_id, now).ok();
            Some(operations::board_row(r, plane, state.as_ref()))
        })
        .collect()
}

#[test]
fn board_rows_show_both_directions_airborne_landing_and_pending_release() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    let rows = board(&g);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.state_kind == "scheduled" && r.expected_landing.is_none()));
    assert_eq!((rows[0].origin.as_str(), rows[0].destination.as_str()), ("EDDF", "EDDM"));
    assert_eq!(rows[0].next_departure, Some(60));
    assert_eq!(rows[1].next_departure, Some(7260));
    g.clock.set_running(true);
    assert!(g.advance(100.0));
    let rows = board(&g);
    assert_eq!(rows[0].state_kind, "airborne");
    assert!(rows[0].expected_landing.unwrap() > 100);
    assert_eq!(rows[0].next_departure, Some(60 + 14400));
    assert_eq!(rows[1].state_kind, "scheduled");
    g.remove_route_aircraft(1, 1).unwrap();
    let rows = board(&g);
    assert_eq!(rows[0].state_kind, "release");
    assert!(rows[0].release_pending);
    assert_eq!(rows[0].next_departure, None);
    assert!(rows[0].expected_landing.is_some());
    let ready = g.routes[0].remove_at.unwrap();
    assert!(g.advance((ready - 100) as f64));
    let rows = board(&g);
    assert_eq!(rows.len(), 1);
    // After landing, the second aircraft flies the return leg from EDDM.
    assert!(g.advance(14400.0));
    let rows = board(&g);
    assert_eq!(rows[0].state_kind, "scheduled");
    assert_eq!((rows[0].origin.as_str(), rows[0].destination.as_str()), ("EDDM", "EDDF"));
    assert_eq!(rows[0].next_departure, Some(7260 + 14400));
}

#[test]
fn board_rows_flag_maintenance_and_weather_delay() {
    let mut g = game(1);
    g.schedule(1, 1, 240).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(100.0));
    g.request_maintenance(1).unwrap();
    let end = g.fleet[0].maintenance.as_ref().unwrap().end;
    assert_eq!(board(&g)[0].state_kind, "airborne");
    assert!(g.advance((end - 100 - 1) as f64));
    let rows = board(&g);
    assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
    assert_eq!(rows[0].state_kind, "maintenance");
}

#[test]
fn wind_down_releases_staggered_aircraft_once_and_allows_deletion() {
    let mut g = game(3);
    g.create_route("EDDF", "EDDM", 3).unwrap();
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    g.schedule(2, 1, 240).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(100.0));
    assert_eq!(g.fleet[0].status, AircraftStatus::InFlight);
    assert!(g.wind_down_route(99).is_err());
    let control = g.routes[2].clone();
    g.wind_down_route(1).unwrap();
    let pending = g.pending_releases(1);
    assert_eq!(pending.len(), 1);
    let before = state(&g);
    g.wind_down_route(1).unwrap();
    assert_eq!(state(&g), before);
    assert!(g.delete_route(1).is_err());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wind-down.json");
    save::save(&path, &g).unwrap();
    let mut loaded = save::load(&path).unwrap();
    assert_eq!(loaded.pending_releases(1), pending);
    let last = pending.iter().map(|(_, t)| *t).max().unwrap();
    assert!(g.advance((last - 100) as f64));
    assert!(loaded.advance((last - 100) as f64));
    assert_eq!(state(&loaded), state(&g));
    assert!(g.pending_releases(1).is_empty());
    assert!(
        g.routes
            .iter()
            .filter(|r| r.public_id() == 1)
            .all(|r| r.plan.is_none() && r.aircraft_id.is_none())
    );
    assert!(
        g.fleet[..2]
            .iter()
            .all(|p| p.status == AircraftStatus::Parked)
    );
    let now = g.clock.elapsed().as_secs();
    let closed = g.history.as_ref().unwrap().totals(1, now).flights;
    assert!(closed >= 1);
    let landed = g.finance.totals.flights;
    g.delete_route(1).unwrap();
    assert!(g.routes.iter().all(|r| r.public_id() != 1));
    assert_eq!(g.routes.last().unwrap().plan.is_some(), control.plan.is_some());
    assert!(g.advance(86400.0));
    assert!(g.finance.totals.flights > landed);
    let now = g.clock.elapsed().as_secs();
    assert_eq!(g.history.as_ref().unwrap().totals(1, now).flights, closed);
    assert_eq!(g.history.as_ref().unwrap().totals(0, now), g.finance.totals);
}

#[test]
fn wind_down_at_turnaround_boundary_counts_each_landing_once() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    let (flight, ground) = operations::timings(&g.routes[0], "atr72_600").unwrap();
    g.routes[0].plan.as_mut().unwrap().interval_seconds = flight + ground;
    g.clock.set_running(true);
    assert!(g.advance((60 + flight + ground - 1) as f64));
    g.wind_down_route(1).unwrap();
    assert!(g.advance(1.0));
    assert!(g.advance(2.0 * (flight + ground) as f64));
    assert!(g.pending_releases(1).is_empty());
    assert!(
        g.routes
            .iter()
            .all(|r| r.plan.is_none() && r.aircraft_id.is_none())
    );
    assert_eq!(
        g.finance.totals.flights as u64,
        g.fleet.iter().map(|p| p.wear.cycles).sum::<u64>()
    );
    g.validate_operations().unwrap();
    g.delete_route(1).unwrap();
}

#[test]
fn wind_down_in_maintenance_keeps_check_and_clears_resume() {
    let mut g = game(2);
    g.schedule(1, 1, 240).unwrap();
    g.add_route_aircraft(1, 2).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(100.0));
    g.request_maintenance(1).unwrap();
    g.wind_down_route(1).unwrap();
    let visit = g.fleet[0].maintenance.as_ref().unwrap();
    assert!(visit.resume.is_none());
    let end = visit.end;
    assert!(g.advance(end as f64));
    assert!(
        g.routes
            .iter()
            .filter(|r| r.public_id() == 1)
            .all(|r| r.plan.is_none())
    );
    assert!(g.wind_down_route(1).is_err());
    g.delete_route(1).unwrap();
}

#[test]
fn adding_an_aircraft_parked_away_from_the_route_origin_is_rejected() {
    let mut g = game(2);
    let origin = g.routes[0].origin_icao.clone();
    g.fleet[1].location_icao = "EDDM".into();
    if origin == "EDDM" {
        g.fleet[1].location_icao = "EDDF".into();
    }
    let before = state(&g);
    assert!(g.add_route_aircraft(1, 2).unwrap_err().contains("überführen"));
    assert_eq!(state(&g), before);
    g.schedule(1, 1, 240).unwrap();
    let before = state(&g);
    assert!(g.add_route_aircraft(1, 2).is_err());
    assert_eq!(state(&g), before);
    g.fleet[1].location_icao = origin;
    g.add_route_aircraft(1, 2).unwrap();
}
