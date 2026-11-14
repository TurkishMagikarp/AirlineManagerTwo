use crate::{
    attention::Entry,
    game::Game,
    save,
    staff::{self, Role},
};

fn game() -> Game {
    let mut g = Game::default();
    g.found("Attention", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g
}
fn kinds(g: &Game) -> Vec<&'static str> {
    g.attention_snapshot().iter().map(|e| e.kind).collect()
}
fn entry(g: &Game, kind: &str) -> Entry {
    g.attention_snapshot()
        .into_iter()
        .find(|e| e.kind == kind)
        .unwrap()
}

#[test]
fn attention_conditions_toggle_and_targets_reuse_existing_reasons() {
    assert!(Game::default().attention_snapshot().is_empty());
    let mut g = game();
    assert_eq!(kinds(&g), ["crew", "idle"]);
    let crew = entry(&g, "crew");
    assert_eq!(
        (crew.page, crew.route_id, crew.aircraft_id),
        ("routes", 1, 1)
    );
    assert!(
        crew.text
            .contains(&g.require_crew("atr72_600").unwrap_err())
    );
    assert!(
        crew.text
            .contains(&staff::needs("atr72_600").unwrap().rating)
    );
    g.staff_up();
    assert_eq!(kinds(&g), ["idle"]);
    g.schedule(1, 1, 120).unwrap();
    assert!(g.attention_snapshot().is_empty());
    g.clock.set_running(true);
    let (flight, ground) = crate::operations::timings(&g.routes[0], "atr72_600").unwrap();
    assert!(g.advance((60 + flight + ground + 10) as f64));
    g.stop_schedule(1).unwrap();
    let technicians: Vec<_> = g
        .staff
        .employees
        .iter()
        .filter(|e| e.role == Role::Technician)
        .map(|e| e.id)
        .collect();
    for id in technicians {
        g.dismiss_staff(id).unwrap();
    }
    g.request_maintenance(1).unwrap();
    assert_eq!(kinds(&g), ["technicians"]);
    assert_eq!(
        (
            entry(&g, "technicians").page,
            entry(&g, "technicians").aircraft_id
        ),
        ("fleet", 1)
    );
    let cash = g.airline.as_ref().unwrap().capital_eur;
    g.airline.as_mut().unwrap().capital_eur = g.salary_reserve().unwrap() - 1;
    assert_eq!(kinds(&g), ["technicians", "salary_reserve"]);
    g.airline.as_mut().unwrap().capital_eur = cash;
    g.hire_staff(
        Role::Technician,
        staff::needs("atr72_600").unwrap().technicians,
    )
    .unwrap();
    assert!(!kinds(&g).contains(&"technicians"));

    let mut g = game();
    g.hire_staff(Role::Pilot, 1).unwrap();
    let id = g.staff.employees[0].id;
    let old_rating = g.staff.employees[0].rating.clone().unwrap();
    let rating = staff::ratings()
        .into_iter()
        .find(|r| *r != old_rating)
        .unwrap();
    g.train_pilot(id, &rating).unwrap();
    let t = entry(&g, "training");
    assert_eq!((t.page, t.employee_id), ("staff", id));
    assert!(t.text.contains(&crate::operations::display_time(
        g.next_training_event().unwrap()
    )));
    g.clock.set_running(true);
    assert!(g.advance((staff::conversion_terms().1 * 86400) as f64));
    assert!(!kinds(&g).contains(&"training"));
    let reserve = g.salary_reserve().unwrap();
    g.airline.as_mut().unwrap().capital_eur = reserve;
    assert!(!kinds(&g).contains(&"salary_reserve"));
    g.airline.as_mut().unwrap().capital_eur = reserve - 1;
    assert_eq!(entry(&g, "salary_reserve").page, "finances");
    g.airline.as_mut().unwrap().capital_eur = -1;
    assert_eq!(kinds(&g)[0], "negative_cash");
    assert!(!kinds(&g).contains(&"salary_reserve"));
    g.airline.as_mut().unwrap().capital_eur = reserve;
    assert!(!kinds(&g).contains(&"negative_cash"));
}

#[test]
fn attention_order_is_stable_and_snapshot_does_not_touch_any_owner() {
    let mut g = game();
    g.buy_aircraft("atr72_600").unwrap();
    g.hire_staff(Role::Pilot, 2).unwrap();
    let rating = staff::ratings()
        .into_iter()
        .find(|r| Some(r) != g.staff.employees[0].rating.as_ref())
        .unwrap();
    for id in [1, 2] {
        g.train_pilot(id, &rating).unwrap();
    }
    g.airline.as_mut().unwrap().capital_eur = -1;
    let before = serde_json::to_value(&g).unwrap();
    let expected = g.attention_snapshot();
    assert_eq!(
        kinds(&g),
        [
            "negative_cash",
            "crew",
            "idle",
            "idle",
            "training",
            "training"
        ]
    );
    assert_eq!(expected[2].aircraft_id, 1);
    assert_eq!(expected[3].aircraft_id, 2);
    g.fleet.reverse();
    g.staff.employees.reverse();
    assert_eq!(g.attention_snapshot(), expected);
    g.fleet.reverse();
    g.staff.employees.reverse();
    for _ in 0..20 {
        assert_eq!(g.attention_snapshot(), expected);
    }
    assert_eq!(serde_json::to_value(&g).unwrap(), before);
}

#[test]
fn attention_steps_speeds_pause_save_load_and_economy_match() {
    let mut initial = game();
    initial.hire_staff(Role::Pilot, 1).unwrap();
    let rating = staff::ratings()
        .into_iter()
        .find(|r| Some(r) != initial.staff.employees[0].rating.as_ref())
        .unwrap();
    initial.train_pilot(1, &rating).unwrap();
    initial.clock.set_running(true);
    let day = 86400.0;
    let mut reference = initial.clone();
    assert!(reference.advance(8.0 * day));
    for speed in [1, 4, 16] {
        for step in [3600.0, day] {
            let mut g = initial.clone();
            g.clock.set_speed(speed);
            for _ in 0..(8.0 * day / step) as usize {
                let before = serde_json::to_value(&g).unwrap();
                let snapshot = g.attention_snapshot();
                g.clock.set_running(false);
                assert!(g.advance(day));
                assert_eq!(snapshot, g.attention_snapshot());
                g.clock.set_running(true);
                assert_eq!(serde_json::to_value(&g).unwrap(), before);
                assert!(g.advance(step / f64::from(speed)));
            }
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("attention.json");
            save::save(&path, &g).unwrap();
            let mut loaded = save::load(&path).unwrap();
            assert_eq!(loaded.attention_snapshot(), g.attention_snapshot());
            assert!(g.advance(day / f64::from(speed)));
            assert!(loaded.advance(day / f64::from(speed)));
            assert_eq!(
                serde_json::to_value(&g).unwrap(),
                serde_json::to_value(&loaded).unwrap()
            );
            g.clock.set_speed(1);
            let mut control = reference.clone();
            assert!(control.advance(day));
            assert_eq!(g.attention_snapshot(), control.attention_snapshot());
            // Includes complete AI games, ledgers, quotes and economy state.
            assert_eq!(
                serde_json::to_value(&g).unwrap(),
                serde_json::to_value(&control).unwrap()
            );
        }
    }
}
