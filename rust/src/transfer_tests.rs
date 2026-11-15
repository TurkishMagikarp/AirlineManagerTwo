use crate::{economy, game::Game, save};

fn game() -> Game {
    let mut g = Game::default();
    g.found("Transfer audit", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.airline.as_mut().unwrap().capital_eur += 100_000_000;
    g.history.as_mut().unwrap().opening_balance += 100_000_000;
    for (id, destination) in [(1, "EDDM"), (2, "LTFM")] {
        g.buy_aircraft("a320_200").unwrap();
        g.staff_up();
        g.create_route("EDDF", destination, id).unwrap();
        g.set_ticket_price(id, if id == 1 { 90 } else { 120 })
            .unwrap();
        g.schedule(id, 1, 240).unwrap();
    }
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("transfers.json");
    save::save(&p, g).unwrap();
    save::load(&p).unwrap()
}

#[test]
fn transfers_frozen_midflight_settle_once_with_cargo_airport_and_finance() {
    let mut g = game();
    assert!(g.advance(60.0));
    let flight = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
    assert!(
        flight.totals.connecting_passengers > 0,
        "{:?}",
        crate::demand::estimate_at(&g.routes[0], 180, &g.market_routes(), 60).unwrap()
    );
    assert!(flight.totals.valid_cabins(flight.ticket_eur));
    assert_eq!(
        flight.totals.airport,
        economy::airport_bill(&g.routes[0], "a320_200", flight.totals.passengers).unwrap()
    );
    g.set_ticket_price(2, 100_000).unwrap();
    let mut restored = restart(&g);
    assert_eq!(state(&g), state(&restored));
    assert_eq!(
        g.routes[0].plan.as_ref().unwrap().airborne,
        Some(flight.clone())
    );
    for seconds in [flight.landing - 60, 1, 2 * 86400] {
        assert!(g.advance(seconds as f64));
        assert!(restored.advance(seconds as f64));
        assert_eq!(state(&g), state(&restored));
    }
    assert!(g.finance.totals.connecting_passengers >= flight.totals.connecting_passengers);
    g.validate_finance().unwrap();
    g.validate_history().unwrap();
    let archived = g.history.as_ref().unwrap().totals(1, u64::MAX);
    assert!(archived.connecting_passengers > 0);
}

#[test]
fn transfers_pause_speeds_steps_and_restart_have_identical_state() {
    let initial = game();
    let mut expected = initial.clone();
    assert!(expected.advance(14.0 * 86400.0));
    let mut jumped = initial.clone();
    let mut day = initial.clone();
    assert!(day.advance(86400.0));
    assert_eq!(jumped.jump(86400.0).unwrap().advanced_seconds, 86400);
    assert_eq!(state(&day), state(&jumped));
    for speed in [1, 4, 16] {
        let mut replay = initial.clone();
        replay.clock.set_running(false);
        let before = state(&replay);
        assert!(replay.advance(86400.0));
        assert_eq!(before, state(&replay));
        replay.clock.set_running(true);
        replay.clock.set_speed(speed);
        for hour in 0..14 * 24 {
            assert!(replay.advance(3600.0 / f64::from(speed)));
            if hour == 42 {
                replay = restart(&replay);
            }
        }
        replay.clock.set_speed(1);
        assert_eq!(state(&expected), state(&replay), "speed={speed}");
    }
}

#[test]
fn transfers_v31_migration_preserves_old_airborne_and_historical_results() {
    let mut g = game();
    // Legacy flight data has no connecting revenue; create a genuine direct-only
    // snapshot, including both archived landings and an airborne departure.
    g.stop_schedule(2).unwrap();
    assert!(g.advance(14460.0));
    assert!(g.routes[0].plan.as_ref().unwrap().airborne.is_some());
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("legacy.json");
    save::save(&p, &g).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    value["version"] = 31.into();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.remove("connecting_passengers");
                m.remove("connecting_revenue");
                for v in m.values_mut() {
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
    std::fs::write(&p, serde_json::to_vec(&value).unwrap()).unwrap();
    let mut loaded = save::load(&p).unwrap();
    g.migrate_world_economy_fixture();
    assert_eq!(state(&g), state(&loaded));
    assert!(g.advance(86400.0));
    assert!(loaded.advance(86400.0));
    assert_eq!(state(&g), state(&loaded));
    // Reject tampered transfer counters/revenue instead of re-quoting them.
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    value["version"] = save::VERSION.into();
    value["state"]["finance"]["totals"]["connecting_passengers"] = (-1).into();
    std::fs::write(&p, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(save::load(&p).is_err());
}

#[test]
fn transfers_stored_quotes_equal_fresh_after_long_run_and_commercial_events() {
    let mut g = game();
    assert!(g.advance(400.0 * 86400.0));
    for event in 0..7 {
        match event {
            1 => g.set_ticket_price(1, 95).unwrap(),
            2 => {
                g.reputation.score = -500_000;
                g.commercial_changed();
            }
            3 => {
                g.marketing.awareness = 10_000_000;
                g.commercial_changed();
            }
            4 => {
                g.route_mut(2)
                    .unwrap()
                    .plan
                    .as_mut()
                    .unwrap()
                    .interval_seconds *= 2
            }
            5 => {
                g.fleet[0].business_seats = 20;
                g.commercial_changed();
            }
            6 => g.route_mut(2).unwrap().plan = None,
            _ => {}
        }
        let supply = g.prioritize_market(&g.market_routes());
        g.departure_markets.refresh(
            g.supply_revision(),
            g.supply_structure_revision(),
            &supply,
            || supply.clone(),
        );
        for route in &g.routes {
            let model = &g
                .fleet
                .iter()
                .find(|p| Some(p.id) == route.aircraft_id)
                .unwrap()
                .model_id;
            // Stored dispatch markets quote actual departures; annual planning
            // builds a fresh snapshot at the review date.
            for time in [Some(60), Some(400 * 86400), Some(401 * 86400 + 123)] {
                let mut fresh = economy::QuoteMarket::new(route, model, &supply).unwrap();
                let expected =
                    economy::quote_prepared(route, model, &supply, 100, time, &mut fresh).unwrap();
                let mut stored = g.departure_markets.market(route, model).unwrap();
                let actual =
                    economy::quote_prepared(route, model, &supply, 100, time, &mut stored).unwrap();
                assert_eq!(actual, expected, "event={event}, time={time:?}");
            }
        }
    }
}

#[test]
fn transfers_dependency_epochs_track_commercial_events_and_diverging_clones() {
    let mut world = game();
    assert!(world.advance(60.0));
    let mut fork = world.clone();
    world.set_ticket_price(1, 95).unwrap();
    fork.set_ticket_price(1, 100).unwrap();
    assert_ne!(world.supply_revision(), fork.supply_revision());
    let revision = world.supply_revision();
    // Settlement progress and frozen airborne results are not commercial inputs.
    assert!(world.advance(1.0));
    assert_eq!(world.supply_revision(), revision);
    let mut stored = std::mem::take(&mut world.departure_markets);
    stored.refresh(
        revision,
        world.supply_structure_revision(),
        &world.market_routes(),
        || world.prioritize_market(&world.market_routes()),
    );
    stored.refresh(
        revision,
        world.supply_structure_revision(),
        &world.market_routes(),
        || panic!("unchanged supply must not be cloned"),
    );
    world.competitors = crate::competition::initial_competitors().unwrap();
    let before = world.supply_revision();
    world.competitors[0].game.set_ticket_price(1, 150).unwrap();
    assert_ne!(before, world.supply_revision());
    stored.refresh(
        world.supply_revision(),
        world.supply_structure_revision(),
        &world.market_routes(),
        || world.prioritize_market(&world.market_routes()),
    );
    let supply = world.prioritize_market(&world.market_routes());
    for route in &world.routes {
        let model = &world
            .fleet
            .iter()
            .find(|p| Some(p.id) == route.aircraft_id)
            .unwrap()
            .model_id;
        let expected = economy::quote_at_price(route, model, &supply, 100).unwrap();
        let actual = economy::quote_prepared(
            route,
            model,
            &supply,
            100,
            None,
            &mut stored.market(route, model).unwrap(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
fn transfers_reputation_reordering_keeps_stored_seat_reservations_fresh() {
    let mut g = game();
    let mut supply = g.market_routes();
    let mut rival = supply.clone();
    for offer in &mut rival {
        offer.owner = "company:Rival".into();
    }
    supply.extend(rival);
    let mut stored = economy::DepartureMarkets::default();
    let structure = g.supply_structure_revision();
    for score in [-900_000, 900_000, -500_000, 0] {
        for offer in &mut supply {
            offer.reputation = if offer.owner == "company:Rival" {
                score
            } else {
                -score
            };
        }
        g.reputation_changed();
        stored.refresh(g.supply_revision(), structure, &supply, || supply.clone());
        for route in &g.routes {
            let mut fresh = economy::QuoteMarket::new(route, "a320_200", &supply).unwrap();
            for time in [None, Some(60), Some(180 * 86400), Some(400 * 86400)] {
                let expected =
                    economy::quote_prepared(route, "a320_200", &supply, 100, time, &mut fresh)
                        .unwrap();
                let actual = economy::quote_prepared(
                    route,
                    "a320_200",
                    &supply,
                    100,
                    time,
                    &mut stored.market(route, "a320_200").unwrap(),
                )
                .unwrap();
                assert_eq!(actual, expected, "score={score}, time={time:?}");
            }
        }
    }
}

#[test]
fn transfers_zero_economy_reactions_preserve_legacy_quotes_exactly() {
    let g = game();
    let active = g.market_routes();
    let mut legacy = active.clone();
    for offer in &mut legacy {
        offer.economy = None;
    }
    for time in [
        60,
        4 * crate::demand::YEAR_SECONDS,
        4 * crate::demand::YEAR_SECONDS + 43200,
    ] {
        for route in &g.routes {
            let expected = economy::quote_at_time(route, "a320_200", &legacy, 80, time).unwrap();
            crate::world_economy::ZERO_REACTIONS.with(|v| v.set(true));
            let actual = economy::quote_at_time(route, "a320_200", &active, 80, time).unwrap();
            crate::world_economy::ZERO_REACTIONS.with(|v| v.set(false));
            assert!(expected.connecting_passengers > 0);
            assert_eq!(actual, expected, "route={} time={time}", route.id);
        }
    }
}
