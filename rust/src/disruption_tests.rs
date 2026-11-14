use crate::{
    disruptions::{self, Cursor, Leg},
    game::Game,
    operations, save,
};

fn tight_interval() -> u32 {
    let g = game(180);
    let (flight, ground) = operations::timings(&g.routes[0], "atr72_600").unwrap();
    (flight + ground).div_ceil(60) as u32
}
fn game(interval: u32) -> Game {
    let mut g = Game::default();
    g.found("Disruption Audit", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, interval).unwrap();
    g.clock.set_running(true);
    g
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("delay.json");
    save::save(&path, g).unwrap();
    save::load(&path).unwrap()
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn cursor(g: &mut Game) -> &mut Cursor {
    g.routes[0]
        .plan
        .as_mut()
        .unwrap()
        .disruption
        .as_mut()
        .unwrap()
}

/// Select a real seeded event, without relying on a particular airline hash.
fn delayed(interval: u32) -> Game {
    let mut g = game(interval);
    let seed = (0..100_000)
        .find(|seed| {
            disruptions::technical_delay(*seed, &g.fleet[0].wear, "atr72_600", 60, 0).unwrap()
                > 1000
                && (1..5).all(|cycle| {
                    let w = g.fleet[0]
                        .wear
                        .added(
                            cycle,
                            operations::timings(&g.routes[0], "atr72_600").unwrap().0,
                        )
                        .unwrap();
                    disruptions::technical_delay(
                        *seed,
                        &w,
                        "atr72_600",
                        60 + cycle * u64::from(interval) * 60,
                        0,
                    )
                    .unwrap()
                        == 0
                })
        })
        .unwrap();
    cursor(&mut g).seed = seed;
    g
}
#[test]
fn disruptions_absorb_and_propagate_without_shortening_ground_time() {
    for interval in [tight_interval(), 180] {
        let g = delayed(interval);
        let r = &g.routes[0];
        assert_eq!(
            operations::movement(r, "atr72_600", 0)
                .unwrap()
                .delay_seconds,
            0
        );
        assert!(
            operations::movement(r, "atr72_600", 60)
                .unwrap()
                .delay_seconds
                > 1000
        );
        let mut c = r.plan.as_ref().unwrap().disruption.clone().unwrap();
        let (_, ground) = operations::timings(r, "atr72_600").unwrap();
        let mut delays = Vec::new();
        for _ in 0..5 {
            let leg = c.next(r, "atr72_600").unwrap();
            if let Some(last) = &c.previous {
                assert!(leg.departure >= last.landing + ground);
            }
            delays.push(leg.departure - leg.scheduled);
            c.landed(leg).unwrap();
        }
        assert!(delays[0] > 1000);
        if interval == 180 {
            assert_eq!(delays[1], 0);
        } else {
            assert!(delays[1] > 0 && delays[2] > 0);
            assert!(delays[4] < delays[0]);
        }
    }
}
#[test]
fn disruptions_restart_midflight_speeds_steps_pause_and_care_reconcile() {
    let initial = delayed(tight_interval());
    let departure = cursor(&mut initial.clone())
        .next(&initial.routes[0], "atr72_600")
        .unwrap()
        .departure;
    let mut a = initial.clone();
    assert!(a.advance((departure + 10) as f64));
    assert!(a.routes[0].plan.as_ref().unwrap().airborne.is_some());
    let mut b = restart(&a);
    let frozen = b.routes[0].plan.as_ref().unwrap().airborne.clone();
    assert!(b.advance(10.0));
    assert_eq!(frozen, b.routes[0].plan.as_ref().unwrap().airborne);
    let target = 3 * 86400;
    let mut reference = initial.clone();
    assert!(reference.advance(target as f64));
    for speed in [1, 4, 16] {
        for step in [17, 3600, 86400] {
            let mut g = restart(&a);
            g.clock.set_speed(speed);
            g.clock.set_running(false);
            let paused = state(&g);
            assert!(g.advance(500.0));
            assert_eq!(paused, state(&g));
            g.clock.set_running(true);
            while g.clock.elapsed().as_secs() < target {
                let dt = step.min(target - g.clock.elapsed().as_secs());
                assert!(g.advance(dt as f64 / f64::from(speed)));
            }
            g.clock.set_speed(1);
            assert_eq!(state(&g), state(&reference), "speed={speed} step={step}");
            g.validate_history().unwrap();
        }
    }
    let t = &reference.finance.totals;
    assert_eq!(t.flights, t.punctuality_flights);
    assert!(t.late_flights > 0 && t.delay_seconds > 0);
    let mut t = crate::economy::Totals {
        passengers: 10,
        flights: 1,
        ..Default::default()
    };
    disruptions::apply(
        &mut t,
        &Leg {
            index: 0,
            scheduled: 0,
            departure: disruptions::RULES.care_threshold_seconds + 1,
            landing: 9000,
            ready: 10000,
        },
    )
    .unwrap();
    assert_eq!(t.care, 10 * disruptions::RULES.care_eur_per_passenger);
    assert_eq!(t.profit(), -t.care);
}
#[test]
fn disruptions_late_arrival_starts_due_and_requested_checks() {
    for requested in [false, true] {
        let mut g = delayed(tight_interval());
        let flight = operations::timings(&g.routes[0], "atr72_600").unwrap().0;
        let leg = cursor(&mut g.clone())
            .next(&g.routes[0], "atr72_600")
            .unwrap();
        if !requested {
            let rules = crate::maintenance::rules("atr72_600").unwrap();
            g.fleet[0].wear.check_cycles = rules.interval_cycles - 1;
            g.fleet[0].wear.cycles = rules.interval_cycles - 1;
            let wear = g.fleet[0].wear.clone();
            cursor(&mut g).wear = wear;
        }
        let leg = if requested {
            leg
        } else {
            cursor(&mut g.clone())
                .next(&g.routes[0], "atr72_600")
                .unwrap()
        };
        assert!(g.advance((leg.departure + 1) as f64));
        if requested {
            g.request_maintenance(1).unwrap();
        }
        let g = restart(&g);
        let mut g = g;
        assert!(g.advance((leg.landing - g.clock.elapsed().as_secs() - 1) as f64));
        assert_eq!(g.fleet[0].status, crate::aircraft::AircraftStatus::InFlight);
        assert!(g.advance(1.0));
        assert_eq!(
            g.fleet[0].status,
            crate::aircraft::AircraftStatus::Maintenance
        );
        assert_eq!(g.fleet[0].maintenance.as_ref().unwrap().start, leg.landing);
        assert_eq!(g.fleet[0].wear.flight_seconds, flight);
        restart(&g);
    }
}
#[test]
fn disruptions_sale_and_lease_return_clear_pending_delay() {
    for leased in [false, true] {
        let mut g = delayed(tight_interval());
        if leased {
            let mut l = game(tight_interval());
            l.stop_schedule(1).unwrap();
            l.sell_aircraft(1).unwrap();
            l.lease_aircraft("atr72_600").unwrap();
            l.assign_route(1, Some(2)).unwrap();
            l.schedule(1, 1, tight_interval()).unwrap();
            let seed = cursor(&mut g).seed;
            cursor(&mut l).seed = seed;
            g = l;
        }
        let id = g.fleet[0].id;
        let departure = cursor(&mut g.clone())
            .next(&g.routes[0], "atr72_600")
            .unwrap()
            .departure;
        assert!(departure > 60);
        assert!(g.advance(60.0));
        if leased {
            g.return_aircraft(id).unwrap();
        } else {
            g.sell_aircraft(id).unwrap();
        }
        assert!(g.fleet.is_empty() && g.routes[0].plan.is_none());
        assert!(g.advance(86400.0));
        assert_eq!(g.finance.totals.flights, 0);
        restart(&g);
    }
}

#[test]
fn disruptions_v25_migrates_all_owners_empty_and_keeps_airborne_times() {
    let mut g = game(180);
    g.competitors = crate::competition::initial_competitors().unwrap();
    assert!(g.advance(100.0));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v25.json");
    save::save(&path, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                for key in [
                    "reputation",
                    "disruption",
                    "otp",
                    "late",
                    "delay_seconds",
                    "care",
                ] {
                    o.remove(key);
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
            _ => (),
        }
    }
    v["version"] = 25.into();
    strip(&mut v);
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let mut loaded = save::load(&path).unwrap();
    let frozen = loaded.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .clone()
        .unwrap();
    assert!(loaded.routes[0].plan.as_ref().unwrap().disruption.is_none());
    for c in &loaded.competitors {
        assert_eq!(c.game.finance.totals.punctuality_flights, 0);
        assert!(
            c.game
                .routes
                .iter()
                .all(|r| r.plan.as_ref().is_none_or(|p| p.disruption.is_none()))
        );
    }
    assert!(loaded.advance((frozen.landing - 100) as f64));
    assert_eq!(loaded.finance.recent[0], frozen);
    assert_eq!(loaded.finance.totals.punctuality_flights, 0);
    restart(&loaded);
}

#[test]
fn disruptions_wear_age_and_corrupt_cursor_validation() {
    let fresh = crate::maintenance::Wear::default();
    let rules = crate::maintenance::rules("atr72_600").unwrap();
    let worn = crate::maintenance::Wear {
        check_cycles: rules.interval_cycles - 1,
        ..fresh.clone()
    };
    let mut counts = [0, 0, 0];
    let mut seconds = [0, 0, 0];
    for seed in 0..50_000 {
        for (i, (wear, time)) in [
            (&fresh, 0),
            (&worn, 0),
            (&worn, 20 * crate::demand::YEAR_SECONDS),
        ]
        .into_iter()
        .enumerate()
        {
            let d = disruptions::technical_delay(seed, wear, "atr72_600", time, 0).unwrap();
            counts[i] += u64::from(d > 0);
            seconds[i] += d;
        }
    }
    assert!(counts[0] < counts[1] && counts[1] < counts[2]);
    assert!(seconds[0] / counts[0] < seconds[1] / counts[1]);
    assert!(seconds[1] / counts[1] < seconds[2] / counts[2]);
    let mut g = game(180);
    cursor(&mut g).index = 100;
    assert!(g.validate_operations().is_err());
    cursor(&mut g).index = 0;
    cursor(&mut g).wear.cycles = u64::MAX;
    assert!(g.validate_operations().is_err());
}

#[test]
fn disruptions_care_settles_once_reduces_tax_base_and_attention_is_read_only() {
    let mut g = game(tight_interval());
    let rules = crate::maintenance::rules("atr72_600").unwrap();
    g.fleet[0].wear.cycles = rules.interval_cycles - 3;
    g.fleet[0].wear.check_cycles = rules.interval_cycles - 3;
    let wear = g.fleet[0].wear.clone();
    cursor(&mut g).wear = wear.clone();
    let flight = operations::timings(&g.routes[0], "atr72_600").unwrap().0;
    let next_wear = wear.added(1, flight).unwrap();
    let interval = g.routes[0].plan.as_ref().unwrap().interval_seconds;
    let seed = (0..100_000)
        .find(|s| {
            disruptions::technical_delay(*s, &wear, "atr72_600", 60, 0).unwrap() > 2000
                && disruptions::technical_delay(*s, &next_wear, "atr72_600", 60 + interval, 0)
                    .unwrap()
                    > 2000
        })
        .unwrap();
    cursor(&mut g).seed = seed;
    let leg = cursor(&mut g.clone())
        .next(&g.routes[0], "atr72_600")
        .unwrap();
    assert!(g.advance((leg.departure + 1) as f64));
    let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
    assert!(frozen.totals.passengers > 0);
    assert!(
        frozen.totals.costs()
            <= crate::economy::flight_cost_ceiling(
                &g.routes[0],
                "atr72_600",
                g.fuel_market.max_price()
            )
            .unwrap()
    );
    assert_eq!(
        frozen.totals.care,
        frozen.totals.passengers * disruptions::RULES.care_eur_per_passenger
    );
    let before = state(&g);
    let snapshot = g.attention_snapshot();
    assert_eq!(state(&g), before);
    assert_eq!(
        snapshot.iter().any(|e| e.kind == "delay"),
        leg.departure - leg.scheduled > disruptions::RULES.attention_threshold_seconds
    );
    let mut loaded = restart(&g);
    let cash = loaded.airline.as_ref().unwrap().capital_eur;
    assert!(loaded.advance((leg.landing - loaded.clock.elapsed().as_secs()) as f64));
    assert_eq!(
        loaded.airline.as_ref().unwrap().capital_eur,
        cash + frozen.totals.profit()
    );
    assert_eq!(loaded.finance.totals.care, frozen.totals.care);
    let h = loaded.history.as_ref().unwrap();
    assert_eq!(h.totals(1, leg.landing).care, frozen.totals.care);
    assert_eq!(
        h.pretax_profit(leg.landing).unwrap(),
        frozen.totals.profit()
    );
    let mut reloaded = restart(&loaded);
    assert!(reloaded.advance(1.0));
    assert_eq!(reloaded.finance.totals.care, frozen.totals.care);
    let next = cursor(&mut reloaded.clone())
        .next(&reloaded.routes[0], "atr72_600")
        .unwrap();
    assert!(reloaded.advance((next.departure + 1 - reloaded.clock.elapsed().as_secs()) as f64));
    let before = state(&reloaded);
    let snapshot = reloaded.attention_snapshot();
    let entry = snapshot.iter().find(|e| e.kind == "delay").unwrap();
    assert_eq!(
        (entry.aircraft_id, entry.route_id, entry.page),
        (1, 1, "fleet")
    );
    assert_eq!(before, state(&reloaded));
    restart(&reloaded);
}
#[test]
fn disruptions_long_delay_cash_runway_never_reuses_a_past_arrival() {
    let mut g = Game::default();
    g.found("Audit", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 0).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(365.0 * 86400.0));
}

/// Pins the inputs used by tests/disruptions_test.gd to the production config.
#[test]
fn disruptions_godot_fixture_has_care_backlog_and_natural_recovery() {
    let mut g = game(90);
    g.fleet[0].wear.cycles = 570;
    g.fleet[0].wear.check_cycles = 570;
    let wear = g.fleet[0].wear.clone();
    cursor(&mut g).wear = wear;
    cursor(&mut g).seed = 33364;
    g = restart(&g);
    let mut c = cursor(&mut g).clone();
    let mut legs = Vec::new();
    for _ in 0..12 {
        let leg = c.next(&g.routes[0], "atr72_600").unwrap();
        c.landed(leg.clone()).unwrap();
        legs.push(leg);
    }
    assert_eq!(legs[0].landing, 5104);
    assert_eq!(legs[1].departure, 9612);
    assert_eq!(legs[2].departure - legs[2].scheduled, 2664);
    assert_eq!(legs[11].departure, 59460);
    assert_eq!(
        disruptions::technical_delay(0, &g.fleet[0].wear, "atr72_600", 60, 0).unwrap(),
        0
    );
    assert_eq!(legs[0].departure - legs[0].scheduled, 2932);
    assert!(legs[1].departure - legs[1].scheduled > disruptions::RULES.attention_threshold_seconds);
    assert!(legs[2].departure - legs[2].scheduled > 0);
    assert_eq!(legs[11].departure, legs[11].scheduled);
    assert!(g.advance((legs[1].departure + 1) as f64));
    assert!(g.attention_snapshot().iter().any(|e| e.kind == "delay"));
    let saved = restart(&g);
    assert_eq!(state(&g), state(&saved));
    let mut saved = saved;
    assert!(g.advance((legs[11].departure + 1 - g.clock.elapsed().as_secs()) as f64));
    assert!(!g.attention_snapshot().iter().any(|e| e.kind == "delay"));
    assert!(saved.advance((g.clock.elapsed().as_secs() - saved.clock.elapsed().as_secs()) as f64));
    assert_eq!(state(&g), state(&saved));
    g.validate_history().unwrap();
}

#[test]
fn disruptions_godot_restart_families_match_all_owners_after_100_days() {
    for legacy in [true, false] {
        let mut g = game(90);
        g.competitors = crate::competition::initial_competitors().unwrap();
        g.fleet[0].wear.cycles = 570;
        g.fleet[0].wear.check_cycles = 570;
        let wear = g.fleet[0].wear.clone();
        cursor(&mut g).wear = wear;
        cursor(&mut g).seed = if legacy { 0 } else { 33364 };
        assert!(g.advance(if legacy { 120.0 } else { 2993.0 }));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("family.json");
        save::save(&path, &g).unwrap();
        if legacy {
            fn strip(v: &mut serde_json::Value) {
                match v {
                    serde_json::Value::Object(o) => {
                        for key in [
                            "reputation",
                            "disruption",
                            "otp",
                            "late",
                            "delay_seconds",
                            "care",
                        ] {
                            o.remove(key);
                        }
                        for v in o.values_mut() {
                            strip(v);
                        }
                    }
                    serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
                    _ => (),
                }
            }
            let mut v: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            v["version"] = 25.into();
            strip(&mut v);
            std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
            g = save::load(&path).unwrap();
            assert_eq!(g.finance.totals.punctuality_flights, 0);
            for ai in &g.competitors {
                assert_eq!(ai.game.finance.totals.punctuality_flights, 0);
            }
        }
        let mut loaded = save::load(&path).unwrap();
        assert_eq!(state(&g), state(&loaded));
        let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
        if !legacy {
            assert!(frozen.totals.care > 0);
        }
        let now = g.clock.elapsed().as_secs();
        let to_landing = (frozen.landing - now) as f64;
        assert!(g.advance(to_landing));
        assert!(loaded.advance(to_landing));
        assert_eq!(g.finance.recent[0], frozen);
        assert_eq!(state(&g), state(&loaded));
        let rest = (now + 100 * 86400 - g.clock.elapsed().as_secs()) as f64;
        assert!(g.advance(rest));
        assert!(loaded.advance(rest));
        assert_eq!(state(&g), state(&loaded));
        g.validate_history().unwrap();
        for ai in &g.competitors {
            ai.game.validate_history().unwrap();
            assert!(ai.game.finance.totals.punctuality_flights > 0);
        }
    }
}
