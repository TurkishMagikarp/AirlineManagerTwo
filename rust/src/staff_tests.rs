use crate::{
    aircraft::AircraftStatus,
    game::Game,
    maintenance::Wear,
    staff::{Activity, Role, max_hire_batch},
};

const DAY: f64 = 86_400.0;

fn fresh() -> Game {
    let mut g = Game::default();
    g.found("Staff Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("atr72_600").unwrap();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g
}

fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}

fn staffed() -> Game {
    let mut g = fresh();
    g.hire_staff(Role::Pilot, 4).unwrap();
    g.hire_staff(Role::CabinCrew, 3).unwrap();
    g.hire_staff(Role::Technician, 3).unwrap();
    g
}

fn worn() -> Wear {
    Wear {
        flight_seconds: 3600,
        cycles: 2,
        check_seconds: 3600,
        check_cycles: 2,
        checks: 0,
    }
}

fn capital(g: &Game) -> i64 {
    g.airline.as_ref().unwrap().capital_eur
}

#[test]
fn flights_need_unreserved_pilots_and_cabin_crew() {
    let mut g = fresh();
    let before = state(&g);
    let error = g.schedule(1, 1, 120).unwrap_err();
    assert!(
        error.contains("4 Piloten") && error.contains("3 Flugbegleiter"),
        "{error}"
    );
    assert_eq!(state(&g), before);
    g.hire_staff(Role::Pilot, 3).unwrap();
    g.hire_staff(Role::CabinCrew, 3).unwrap();
    let error = g.schedule(1, 1, 120).unwrap_err();
    assert!(
        error.contains("1 Piloten") && !error.contains("Flugbegleiter"),
        "{error}"
    );
    g.hire_staff(Role::Pilot, 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    assert_eq!(g.reserved_crew(), (4, 3));
    assert_eq!(g.available_staff(Role::Pilot), 0);
    // A second aircraft cannot take the reserved crew.
    g.buy_aircraft("atr72_600").unwrap();
    g.create_route("EDDF", "EDDM", 2).unwrap();
    assert!(g.schedule(2, 1, 120).is_err());
    g.validate_staff().unwrap();
}

#[test]
fn hiring_is_atomic_bounded_and_needs_cash() {
    let mut g = fresh();
    assert!(g.hire_staff(Role::Pilot, 0).is_err());
    assert!(g.hire_staff(Role::Pilot, max_hire_batch() + 1).is_err());
    g.airline.as_mut().unwrap().capital_eur = 6_499;
    let before = state(&g);
    assert!(g.hire_staff(Role::Pilot, 1).is_err());
    assert_eq!(state(&g), before);
    g.airline.as_mut().unwrap().capital_eur = 6_500;
    g.hire_staff(Role::Pilot, 1).unwrap();
    assert_eq!(g.staff.employees[0].salary_eur, 6_500);
    // Salaries are not paid upfront.
    assert_eq!(capital(&g), 6_500);
    assert!(Game::default().hire_staff(Role::Pilot, 1).is_err());
}

#[test]
fn reserved_staff_cannot_be_dismissed_but_free_staff_is_paid_pro_rata() {
    let mut g = staffed();
    g.hire_staff(Role::Pilot, 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    let activity = g.staff_activity();
    let flying: Vec<_> = g
        .staff
        .employees
        .iter()
        .zip(&activity)
        .filter(|(e, a)| e.role == Role::Pilot && **a == Activity::Flying)
        .map(|(e, _)| e.id)
        .collect();
    assert_eq!(flying.len(), 4);
    let before = state(&g);
    assert!(
        g.dismiss_staff(flying[0])
            .unwrap_err()
            .contains("im Einsatz")
    );
    assert!(g.dismiss_staff(999).is_err());
    assert_eq!(state(&g), before);
    // The fifth pilot is free: 15 of 30 days of salary are paid on dismissal.
    let free = g
        .staff
        .employees
        .iter()
        .rfind(|e| e.role == Role::Pilot)
        .unwrap()
        .id;
    g.clock.set_running(true);
    assert!(g.advance(15.0 * DAY));
    let balance = capital(&g);
    g.dismiss_staff(free).unwrap();
    assert_eq!(capital(&g), balance - 3_250);
    assert_eq!(g.staff_count(Role::Pilot), 4);
    g.validate_staff().unwrap();
    g.validate_history().unwrap();
}

#[test]
fn payroll_is_paid_once_per_period_and_reconciles_with_the_ledger() {
    let mut g = staffed();
    g.clock.set_running(true);
    let monthly = g.monthly_payroll();
    assert_eq!(monthly, 4 * 6_500 + 3 * 3_200 + 3 * 4_200);
    let start = capital(&g);
    assert!(g.advance(29.0 * DAY));
    assert_eq!(capital(&g), start);
    assert!(g.advance(DAY));
    assert_eq!(capital(&g), start - monthly);
    assert!(g.advance(45.0 * DAY));
    // Day 75: two full periods paid, the third accrues.
    assert_eq!(capital(&g), start - 2 * monthly);
    let history = g.history.as_ref().unwrap();
    assert_eq!(history.payroll.len(), 2);
    assert_eq!(history.payroll_through(u64::MAX), i128::from(2 * monthly));
    g.validate_staff().unwrap();
    g.validate_history().unwrap();
    // A late hire is only paid for the time actually employed (day 75 to 90 here).
    g.hire_staff(Role::Pilot, 1).unwrap();
    assert!(g.advance(15.0 * DAY));
    assert_eq!(capital(&g), start - 3 * monthly - 3_250);
    g.validate_history().unwrap();
}

#[test]
fn payroll_and_staffing_are_identical_for_every_step_size_and_restart() {
    let build = || {
        let mut g = Game::default();
        g.found("Step Air", "DE", "EDDF").unwrap();
        g.buy_aircraft("a320_200").unwrap();
        g.equip_staff_for("a320_200").unwrap();
        g.create_route("EDDF", "LTFM", 1).unwrap();
        g.schedule(1, 1, 360).unwrap();
        g.clock.set_running(true);
        g
    };
    let total = 100.0 * DAY;
    let mut reference = build();
    assert!(reference.advance(total));
    for (step, speed_factor) in [(0.25, 1), (997.5, 4), (3600.0, 16), (DAY, 16)] {
        let mut g = build();
        // Sub-second frames first, then large frames across several paydays.
        let mut left = total;
        if step < 1.0 {
            for _ in 0..4000 {
                assert!(g.advance(step));
            }
            left -= step * 4000.0;
        }
        while left > 0.0 {
            let chunk = if step < 1.0 {
                7_777.0
            } else {
                step * f64::from(speed_factor)
            }
            .min(left);
            assert!(g.advance(chunk), "step {step}");
            left -= chunk;
        }
        assert_eq!(state(&g), state(&reference), "step {step}");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.json");
        crate::save::save(&path, &g).unwrap();
        assert_eq!(state(&crate::save::load(&path).unwrap()), state(&g));
    }
    reference.validate_history().unwrap();
    assert_eq!(reference.history.as_ref().unwrap().payroll.len(), 3);
    for c in &reference.competitors {
        c.game.validate_history().unwrap();
    }
}

#[test]
fn hangar_waits_for_technicians_then_starts_when_they_are_hired() {
    let mut g = fresh();
    g.hire_staff(Role::Pilot, 4).unwrap();
    g.hire_staff(Role::CabinCrew, 3).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(1000.0));
    g.fleet[0].wear = worn();
    g.request_maintenance(1).unwrap();
    assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
    assert!(!g.fleet[0].maintenance.as_ref().unwrap().staffed);
    // No timed event exists for the waiting aircraft, and days pass without progress.
    assert_eq!(g.next_maintenance_event(), None);
    assert!(g.advance(5.0 * DAY));
    assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
    assert!(!g.fleet[0].maintenance.as_ref().unwrap().staffed);
    // Two technicians are not enough for three.
    g.hire_staff(Role::Technician, 2).unwrap();
    assert!(!g.fleet[0].maintenance.as_ref().unwrap().staffed);
    let now = g.clock.elapsed().as_secs();
    g.hire_staff(Role::Technician, 1).unwrap();
    let v = g.fleet[0].maintenance.clone().unwrap();
    assert!(v.staffed);
    let hours = crate::maintenance::rules("atr72_600")
        .unwrap()
        .duration_hours;
    assert_eq!(v.end, now + hours * 3600);
    assert_eq!(g.busy_technicians(), 3);
    assert_eq!(g.available_staff(Role::Technician), 0);
    let tech = g
        .staff
        .employees
        .iter()
        .find(|e| e.role == Role::Technician)
        .unwrap()
        .id;
    assert!(g.dismiss_staff(tech).is_err());
    g.validate_maintenance().unwrap();
    g.validate_history().unwrap();
    assert!(g.advance((hours * 3600) as f64 + 1.0));
    assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
    assert_eq!(g.available_staff(Role::Technician), 3);
    g.validate_history().unwrap();
}

#[test]
fn waiting_aircraft_take_over_technicians_freed_by_other_checks() {
    let mut g = fresh();
    g.buy_aircraft("atr72_600").unwrap();
    g.hire_staff(Role::Technician, 3).unwrap();
    g.clock.set_running(true);
    g.fleet[0].wear = worn();
    g.fleet[1].wear = worn();
    g.request_maintenance(1).unwrap();
    g.request_maintenance(2).unwrap();
    assert!(g.fleet[0].maintenance.as_ref().unwrap().staffed);
    assert!(!g.fleet[1].maintenance.as_ref().unwrap().staffed);
    let end = g.fleet[0].maintenance.as_ref().unwrap().end;
    assert!(g.advance((end - g.clock.elapsed().as_secs()) as f64));
    assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
    let second = g.fleet[1].maintenance.as_ref().unwrap();
    assert!(second.staffed);
    assert_eq!(second.end, end + 12 * 3600);
    assert!(g.advance(12.0 * 3600.0));
    assert_eq!(g.fleet[1].status, AircraftStatus::Parked);
    g.validate_maintenance().unwrap();
}

#[test]
fn suspended_plans_keep_their_crew_reserved_through_a_check() {
    let mut g = staffed();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(3.0 * 3600.0));
    g.fleet[0].wear = worn();
    g.request_maintenance(1).unwrap();
    // Aircraft may still be airborne: the check starts after landing.
    let start = g.fleet[0].maintenance.as_ref().unwrap().start;
    assert!(g.advance((start - g.clock.elapsed().as_secs()) as f64));
    assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
    assert!(g.routes[0].plan.is_none());
    assert_eq!(g.reserved_crew(), (4, 3), "the resumed plan needs its crew");
    let pilot = g
        .staff
        .employees
        .iter()
        .find(|e| e.role == Role::Pilot)
        .unwrap()
        .id;
    assert!(g.dismiss_staff(pilot).is_err());
    assert!(g.advance(2.0 * DAY));
    assert!(g.routes[0].plan.is_some(), "resumed after the check");
    g.validate_maintenance().unwrap();
}

#[test]
fn ai_airlines_hire_follow_the_same_rules_and_pay_salaries() {
    let mut g = Game::default();
    g.found("Player", "DE", "EDDF").unwrap();
    for c in &g.competitors {
        let ai = &c.game;
        assert!(ai.staff_count(Role::Pilot) >= 4);
        assert!(ai.staff_count(Role::Technician) >= 3);
        assert!(ai.routes[0].plan.is_some());
    }
    g.clock.set_running(true);
    for _ in 0..120 {
        assert!(g.advance(DAY));
        for c in &g.competitors {
            let ai = &c.game;
            let (pilots, cabin) = ai.reserved_crew();
            assert!(pilots <= ai.staff_count(Role::Pilot));
            assert!(cabin <= ai.staff_count(Role::CabinCrew));
            ai.validate_staff().unwrap();
            ai.validate_history().unwrap();
        }
    }
    for c in &g.competitors {
        let ai = &c.game;
        let paid = ai.history.as_ref().unwrap().payroll_through(u64::MAX);
        // Expansion can increase today's workforce shortly before the last
        // payday; today's payroll is not a lower bound on historical salaries.
        assert!(paid > 0, "paid {paid}");
    }
    g.validate_competitors().unwrap();
}

#[test]
fn old_saves_without_staff_are_grandfathered_with_the_minimum_crew() {
    let mut g = staffed();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(3000.0));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v10.json");
    crate::save::save(&path, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["version"] = 10.into();
    v["state"].as_object_mut().unwrap().remove("staff");
    v["state"]["history"]
        .as_object_mut()
        .unwrap()
        .remove("payroll");
    for c in v["state"]["competitors"].as_array_mut().unwrap() {
        c["game"].as_object_mut().unwrap().remove("staff");
        c["game"]["history"]
            .as_object_mut()
            .unwrap()
            .remove("payroll");
    }
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let mut loaded = crate::save::load(&path).unwrap();
    assert_eq!(loaded.staff_count(Role::Pilot), 4);
    assert_eq!(loaded.staff_count(Role::CabinCrew), 3);
    assert_eq!(loaded.staff_count(Role::Technician), 3);
    assert!(loaded.routes[0].plan.is_some());
    for c in &loaded.competitors {
        let (pilots, _) = c.game.reserved_crew();
        assert!(pilots > 0 && c.game.staff_count(Role::Pilot) >= pilots);
    }
    assert!(loaded.advance(40.0 * DAY));
    assert_eq!(loaded.history.as_ref().unwrap().payroll.len(), 1);
    crate::save::save(&path, &loaded).unwrap();
    assert_eq!(state(&crate::save::load(&path).unwrap()), state(&loaded));
    // Corrupt staff data is rejected.
    let mut bad = loaded.clone();
    bad.staff.employees[1].id = bad.staff.employees[0].id;
    assert!(crate::save::save(&path, &bad).is_err());
}

#[test]
fn repeated_hiring_reserves_existing_payroll_and_accrued_debt() {
    let mut g = fresh();
    g.airline.as_mut().unwrap().capital_eur = 13_000;
    g.hire_staff(Role::Pilot, 1).unwrap();
    g.hire_staff(Role::Pilot, 1).unwrap();
    let before = state(&g);
    assert!(g.hire_staff(Role::Pilot, 1).is_err());
    assert_eq!(state(&g), before);
    g.clock.set_running(true);
    assert!(g.advance(15.0 * DAY));
    assert_eq!(g.accrued_payroll().unwrap(), 6_500);
    g.airline.as_mut().unwrap().capital_eur = 25_999;
    let before = state(&g);
    assert!(g.hire_staff(Role::Pilot, 1).is_err());
    assert_eq!(state(&g), before);
    g.airline.as_mut().unwrap().capital_eur = 26_000;
    g.hire_staff(Role::Pilot, 1).unwrap();
    assert_eq!(capital(&g), 26_000);
}

#[test]
fn short_employment_is_not_free_and_dismissal_settles_debt() {
    let mut g = fresh();
    g.hire_staff(Role::Pilot, 1).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(1.0));
    let start = capital(&g);
    g.dismiss_staff(1).unwrap();
    assert_eq!(capital(&g), start - 1);
    g.validate_history().unwrap();
    g.hire_staff(Role::Pilot, 1).unwrap();
    assert!(g.advance(15.0 * DAY));
    // Spending cash does not erase wages already earned; dismissal may create
    // debt, just like payroll, but never silently forgives the liability.
    g.airline.as_mut().unwrap().capital_eur = 0;
    g.dismiss_staff(2).unwrap();
    assert_eq!(capital(&g), -3_250);
}

#[test]
fn fractional_payroll_carries_across_boundaries_and_restart() {
    let mut g = fresh();
    g.clock.set_running(true);
    assert!(g.advance(1.0));
    g.hire_staff(Role::Pilot, 1).unwrap();
    let start = capital(&g);
    assert!(g.advance(30.0 * DAY - 1.0));
    assert_eq!(capital(&g), start - 6_499);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fraction.json");
    crate::save::save(&path, &g).unwrap();
    g = crate::save::load(&path).unwrap();
    assert!(g.advance(1.0));
    g.dismiss_staff(1).unwrap();
    assert_eq!(capital(&g), start - 6_500);
    g.validate_history().unwrap();
}

#[test]
fn legacy_crew_costs_and_airborne_quotes_survive_without_new_duplicate_charges() {
    let mut g = staffed();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(61.0));
    g.routes[0]
        .plan
        .as_mut()
        .unwrap()
        .airborne
        .as_mut()
        .unwrap()
        .totals
        .crew = 777;
    assert!(g.advance(7200.0));
    assert_eq!(g.finance.totals.crew, 777);
    g.routes[0]
        .plan
        .as_mut()
        .unwrap()
        .airborne
        .as_mut()
        .unwrap()
        .totals
        .crew = 888;
    let history = g.history.clone();
    let quote = g.routes[0].plan.as_ref().unwrap().airborne.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy-crew.json");
    crate::save::save(&path, &g).unwrap();
    let mut loaded = crate::save::load(&path).unwrap();
    assert_eq!(loaded.history, history);
    assert_eq!(loaded.routes[0].plan.as_ref().unwrap().airborne, quote);
    assert!(loaded.advance(7200.0));
    assert_eq!(loaded.finance.totals.crew, 777 + 888);
    assert!(loaded.advance(35.0 * DAY));
    assert_eq!(loaded.finance.totals.crew, 777 + 888);
    assert!(loaded.history.as_ref().unwrap().payroll_through(u64::MAX) > 0);
    loaded.validate_history().unwrap();
    loaded.validate_finance().unwrap();
}

#[test]
fn ratings_block_wrong_pilots_and_reserved_conversion_is_atomic() {
    let mut g = fresh();
    g.hire_pilots("737", 4).unwrap();
    g.hire_staff(Role::CabinCrew, 3).unwrap();
    let before = state(&g);
    assert!(g.schedule(1, 1, 120).unwrap_err().contains("ATR 72"));
    assert_eq!(state(&g), before);
    g.hire_pilots("ATR 72", 4).unwrap();
    g.schedule(1, 1, 120).unwrap();
    let id = g
        .staff
        .employees
        .iter()
        .find(|e| e.rating.as_deref() == Some("ATR 72"))
        .unwrap()
        .id;
    let before = state(&g);
    assert!(g.train_pilot(id, "737").is_err());
    assert!(g.dismiss_staff(id).is_err());
    assert_eq!(state(&g), before);
    g.validate_staff().unwrap();
    // AI housekeeping must remove free pilots even when reserved pilots have higher ids.
    g.dismiss_surplus().unwrap();
    assert_eq!(g.rated_count("ATR 72"), 4);
    assert_eq!(g.rated_count("737"), 0);
    g.validate_staff().unwrap();
}

#[test]
fn conversion_boundary_speeds_pause_steps_and_restart_match() {
    let mut base = fresh();
    base.hire_pilots("737", 1).unwrap();
    let id = base.staff.employees[0].id;
    let money = capital(&base);
    base.train_pilot(id, "ATR 72").unwrap();
    assert_eq!(money - capital(&base), crate::staff::conversion_terms().0);
    assert_eq!(base.rated_available("737"), 0);
    assert!(base.dismiss_staff(id).is_err());
    assert!(base.train_pilot(id, "A320 family").is_err());
    let paused = state(&base);
    assert!(base.advance(100.0 * DAY));
    assert_eq!(state(&base), paused);
    let duration = crate::staff::conversion_terms().1 as f64 * DAY;
    let mut exact = base.clone();
    exact.clock.set_running(true);
    assert!(exact.advance(duration - 1.0));
    assert!(exact.staff.employees[0].training.is_some());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("training.json");
    crate::save::save(&path, &exact).unwrap();
    exact = crate::save::load(&path).unwrap();
    assert!(exact.advance(1.0));
    assert!(exact.staff.employees[0].training.is_none());
    assert_eq!(exact.rated_available("ATR 72"), 1);
    assert_eq!(exact.rated_count("737"), 0);
    assert!(exact.advance(DAY));
    for speed in [1, 4, 16] {
        for steps in [1, 15, 120] {
            let mut g = base.clone();
            g.clock.set_running(true);
            g.clock.set_speed(speed);
            for _ in 0..steps {
                assert!(g.advance((duration + DAY) / f64::from(speed) / f64::from(steps)));
            }
            g.clock.set_speed(1);
            assert_eq!(state(&g), state(&exact));
            g.validate_history().unwrap();
            g.validate_staff().unwrap();
        }
    }
}

#[test]
fn ratings_v24_migration_preserves_mixed_running_fleets_and_ai() {
    let mut g = staffed();
    g.schedule(1, 1, 120).unwrap();
    g.lease_aircraft("a320_200").unwrap();
    g.equip_staff_for("a320_200").unwrap();
    g.create_route("EDDF", "EDDM", 2).unwrap();
    g.schedule(2, 2, 120).unwrap();
    g.competitors = crate::competition::initial_competitors().unwrap();
    g.clock.set_running(true);
    assert!(g.advance(600.0));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v24.json");
    crate::save::save(&path, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["version"] = 24.into();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.remove("rating");
                o.remove("training");
                for value in o.values_mut() {
                    strip(value);
                }
            }
            serde_json::Value::Array(a) => {
                for value in a {
                    strip(value);
                }
            }
            _ => {}
        }
    }
    strip(&mut v["state"]);
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let mut a = crate::save::load(&path).unwrap();
    let b = crate::save::load(&path).unwrap();
    assert_eq!(state(&a), state(&b));
    assert_eq!(capital(&a), capital(&g));
    assert_eq!(a.finance, g.finance);
    assert_eq!(a.routes, g.routes);
    a.validate_staff().unwrap();
    for (c, old) in a.competitors.iter().zip(&g.competitors) {
        c.game.validate_staff().unwrap();
        assert_eq!(c.game.monthly_payroll(), old.game.monthly_payroll());
    }
    assert!(a.advance(DAY));
}

#[test]
fn selling_last_type_keeps_available_rating_and_ai_hiring_costs() {
    let mut g = staffed();
    let payroll = g.monthly_payroll();
    let rating = g.staff.employees[0].rating.clone();
    g.sell_aircraft(1).unwrap();
    assert_eq!(g.rated_available("ATR 72"), 4);
    assert_eq!(g.staff.employees[0].rating, rating);
    assert_eq!(g.monthly_payroll(), payroll);
    g.validate_staff().unwrap();
    let mut leased = fresh();
    leased.sell_aircraft(1).unwrap();
    leased.lease_aircraft("atr72_600").unwrap();
    leased.equip_staff_for("atr72_600").unwrap();
    leased.return_aircraft(2).unwrap();
    assert_eq!(leased.rated_available("ATR 72"), 4);
    leased.validate_staff().unwrap();
    for model in crate::aircraft::models().unwrap() {
        let mut ai = fresh();
        ai.equip_staff_for(&model.id).unwrap();
        let n = crate::staff::needs(&model.id).unwrap();
        assert_eq!(ai.rated_available(&n.rating), n.pilots);
        assert_eq!(
            ai.monthly_payroll(),
            i64::from(n.pilots) * crate::staff::salary(Role::Pilot).unwrap()
                + i64::from(n.cabin_crew) * crate::staff::salary(Role::CabinCrew).unwrap()
                + i64::from(n.technicians) * crate::staff::salary(Role::Technician).unwrap()
        );
    }
}

#[test]
fn rated_shortage_hiring_is_atomic_and_ignores_other_ratings() {
    let mut g = fresh();
    g.hire_pilots("737", 4).unwrap();
    assert_eq!(g.idle_pilot_shortage("ATR 72"), 4);
    assert_eq!(g.idle_pilot_shortage("737"), 0);
    g.hire_missing_rated_staff(3, 3).unwrap();
    assert_eq!(g.rated_count("737"), 4);
    assert_eq!(g.rated_available("ATR 72"), 4);
    g.schedule(1, 1, 120).unwrap();
    g.validate_staff().unwrap();
    let before = state(&g);
    assert!(g.hire_missing_rated_staff(50, 1).is_err());
    assert_eq!(state(&g), before);
}
