use crate::{game::Game, slots};

fn game() -> Game {
    let mut g = Game::default();
    g.found("Slots", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g
}

#[test]
fn slots_exact_capacity_transactional_admission_and_release() {
    let mut g = game();
    assert_eq!(slots::departures(7200), 6);
    assert_eq!(slots::departures(5000), 9);
    g.external_slots.insert("EDDF".into(), 114);
    assert_eq!(g.slot_usage("EDDF"), 120);
    g.buy_aircraft("atr72_600").unwrap();
    let before = serde_json::to_value(&g).unwrap();
    assert!(
        g.create_route("EDDF", "EDLP", 2)
            .unwrap_err()
            .contains("FRA is full: 0 of 120")
    );
    assert!(g.add_route_aircraft(1, 2).is_err());
    assert_eq!(before, serde_json::to_value(&g).unwrap());
    g.stop_schedule(1).unwrap();
    assert_eq!(g.slot_usage("EDDF"), 114);
    g.schedule(1, 1, 120).unwrap(); // exactly at capacity
    g.stop_schedule(1).unwrap();
    g.delete_route(1).unwrap();
    assert_eq!(g.slot_usage("EDDF"), 114);
}

#[test]
fn slots_sale_return_maintenance_and_single_attention_entry() {
    let mut g = game();
    let before = g.slot_usage("EDDF");
    g.clock.set_running(true);
    assert!(g.advance(61.0));
    g.request_maintenance(1).unwrap();
    assert!(g.advance(3600.0));
    assert!(g.routes[0].plan.is_none());
    assert_eq!(g.slot_usage("EDDF"), before);
    g.external_slots.insert("EDDF".into(), 110);
    assert_eq!(
        g.attention_snapshot()
            .iter()
            .filter(|e| e.kind == "slots")
            .count(),
        1
    );
    let mut g = game();
    g.sell_aircraft(1).unwrap();
    assert_eq!(g.slot_usage("EDDF"), 0);
    let mut g = game();
    g.stop_schedule(1).unwrap();
    g.delete_route(1).unwrap();
    g.lease_aircraft("atr72_600").unwrap();
    g.create_route("EDDF", "EDDM", 2).unwrap();
    g.schedule(2, 1, 120).unwrap();
    assert_eq!(g.slot_usage("EDDF"), 6);
    g.return_aircraft(2).unwrap();
    assert_eq!(g.slot_usage("EDDF"), 0);
}

#[test]
fn slots_v26_overcapacity_migration_preserves_all_frequencies() {
    let mut g = game();
    // Old saves could own arbitrarily many valid services. Clone untouched,
    // future departures to build one without bypassing any save validation.
    for id in 2..=21 {
        let mut plane = g.fleet[0].clone();
        plane.id = id;
        g.fleet.push(plane);
        let mut r = g.routes[0].clone();
        r.id = id;
        r.aircraft_id = Some(id);
        g.routes.push(r);
    }
    g.staff_up();
    g.next_aircraft_id = 22;
    g.next_route_id = 22;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    crate::save::save(&path, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["version"] = 26.into();
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let mut loaded = crate::save::load(&path).unwrap();
    assert_eq!(loaded.slot_usage("EDDF"), 126);
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        serde_json::to_value(&loaded).unwrap()
    );
    assert!(loaded.check_slots("EDDF", "EDDM", 7200).is_err());
    loaded.stop_schedule(21).unwrap();
    assert!(loaded.check_slots("EDDF", "EDDM", 7200).is_err()); // exactly full
    loaded.stop_schedule(20).unwrap();
    assert!(loaded.check_slots("EDDF", "EDDM", 7200).is_ok());
}

#[test]
fn slots_steps_speeds_pause_and_restart_are_identical() {
    let mut initial = game();
    initial.clock.set_running(true);
    let mut reference = initial.clone();
    assert!(reference.advance(7.0 * 86400.0));
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        for _ in 0..7 {
            assert!(g.advance(86400.0 / f64::from(speed)));
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("slots.json");
            crate::save::save(&path, &g).unwrap();
            g = crate::save::load(&path).unwrap();
            g.clock.set_running(false);
            let before = serde_json::to_value(&g).unwrap();
            assert!(g.advance(1e6));
            assert_eq!(before, serde_json::to_value(&g).unwrap());
            g.clock.set_running(true);
        }
        g.clock.set_speed(1);
        assert_eq!(g.slot_usage("EDDF"), reference.slot_usage("EDDF"));
        assert_eq!(
            serde_json::to_value(&g).unwrap(),
            serde_json::to_value(&reference).unwrap()
        );
    }
}

#[test]
fn slots_other_owners_and_pending_frequency_removal_are_counted_once() {
    let mut g = game();
    g.competitors = crate::competition::initial_competitors().unwrap();
    let expected = 6 + g
        .competitors
        .iter()
        .map(|c| c.game.slot_usage("EDDF"))
        .sum::<u64>();
    assert_eq!(g.slot_usage("EDDF"), expected);
    let snapshot = g.slots_except_competitor(0);
    assert_eq!(
        snapshot["EDDF"],
        expected - g.competitors[0].game.slot_usage("EDDF")
    );
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.add_route_aircraft(1, 2).unwrap();
    assert_eq!(g.slot_usage("EDDF"), expected + 6);
    g.remove_route_aircraft(1, 2).unwrap();
    assert_eq!(g.slot_usage("EDDF"), expected);
    g.clock.set_running(true);
    assert!(g.advance(61.0));
    g.request_maintenance(1).unwrap();
    assert_eq!(g.slot_usage("EDDF"), expected);
    assert!(g.advance(3600.0));
    assert_eq!(g.slot_usage("EDDF"), expected);
}
