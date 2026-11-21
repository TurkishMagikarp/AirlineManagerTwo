use crate::{
    game::Game,
    labour, save,
    staff::{self, Role},
};
const DAY: f64 = 86_400.0;
fn game() -> Game {
    let mut g = Game::default();
    g.found("Labour Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("atr72_600").unwrap();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.staff_up();
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn reload(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("labour.json");
    save::save(&path, g).unwrap();
    save::load(&path).unwrap()
}
fn warning(g: &mut Game, role: Role, at: u64) {
    let group = &mut g.staff.labour[role as usize];
    group.pay_percent = 85;
    group.pending_percent = 85;
    group.morale = 20;
    group.warning_at = Some(at);
    g.refresh_labour_holds().unwrap();
}
#[test]
fn labour_market_pay_is_neutral_and_changes_start_with_next_period() {
    let mut g = game();
    let cash = g.cash();
    let payroll = g.monthly_payroll();
    let count = g.staff.employees.len();
    g.set_pay_level(Role::Pilot, 85).unwrap();
    assert_eq!(g.pay_preview(Role::Pilot, 85).unwrap(), 4 * 5525);
    assert!(g.set_pay_level(Role::Pilot, 84).is_err());
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY));
    assert_eq!(g.cash(), cash - payroll);
    assert_eq!(g.labour_group(Role::Pilot).morale, 75);
    assert_eq!(g.staff.employees.len(), count);
    assert_eq!(g.monthly_payroll(), payroll - 3900);
    let mut loaded = reload(&g);
    assert!(g.advance(30.0 * DAY));
    assert!(loaded.advance(30.0 * DAY));
    assert_eq!(state(&g), state(&loaded));
    assert_eq!(g.cash(), cash - payroll * 2 + 3900);
    assert_eq!(g.labour_group(Role::Pilot).morale, 65);
    g.validate_history().unwrap();
    let mut normal = game();
    normal.clock.set_running(true);
    assert!(normal.advance(120.0 * DAY));
    assert!(
        normal
            .staff
            .labour
            .iter()
            .all(|g| g.morale == 75 && g.resigned == 0 && g.warning_at.is_none())
    );
    assert_eq!(normal.cash(), cash - payroll * 4);
    assert_eq!(normal.staff.employees.len(), count);
}
#[test]
fn labour_resignations_remove_rated_pilots_and_hiring_releases_grounded_plan() {
    let mut g = game();
    g.schedule(1, 1, 120).unwrap();
    g.set_pay_level(Role::Pilot, 85).unwrap();
    g.staff.labour[0].morale = 30;
    g.staff.labour[0].pay_percent = 85;
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY));
    assert_eq!(g.labour_group(Role::Pilot).resigned, 1);
    assert_eq!(g.rated_count("ATR 72"), 3);
    let resigned = g
        .staff
        .employees
        .iter()
        .filter(|e| e.role == Role::Pilot)
        .map(|e| e.id)
        .collect::<Vec<_>>();
    assert_eq!(resigned, vec![1, 2, 3]);
    assert!(
        g.routes[0]
            .plan
            .as_ref()
            .unwrap()
            .disruption
            .as_ref()
            .unwrap()
            .staff_hold
            .is_some()
    );
    assert!(g.advance(DAY));
    let flights = g.finance.totals.flights;
    assert!(g.advance(DAY));
    assert_eq!(g.finance.totals.flights, flights);
    let mut g = reload(&g);
    assert_eq!(g.idle_pilot_shortage("ATR 72"), 1);
    g.hire_pilots("ATR 72", 1).unwrap();
    assert!(g.advance(DAY));
    assert!(g.finance.totals.flights > flights);
    g.validate_history().unwrap();
    g.validate_staff().unwrap();
}
#[test]
fn crew_gaps_name_the_held_aircraft_and_clear_after_hiring() {
    let mut g = game();
    g.schedule(1, 1, 120).unwrap();
    assert!(g.crew_gaps().is_empty());
    g.set_pay_level(Role::Pilot, 85).unwrap();
    g.staff.labour[0].morale = 30;
    g.staff.labour[0].pay_percent = 85;
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY));
    let g = reload(&g);
    let id = g.routes[0].aircraft_id.unwrap();
    assert_eq!(g.crew_gaps(), vec![(id, 1, 0)]);
    let entries = g.attention_snapshot();
    assert!(
        entries
            .iter()
            .any(|e| e.kind == "crew" && e.aircraft_id == id)
    );
    let mut g = g;
    g.hire_pilots("ATR 72", 1).unwrap();
    assert!(g.crew_gaps().is_empty());
    assert!(!g.attention_snapshot().iter().any(|e| e.kind == "crew"));
}
#[test]
fn labour_warning_raise_averts_strike_and_jump_stops_on_strike() {
    let mut base = game();
    base.schedule(1, 1, 120).unwrap();
    warning(&mut base, Role::Technician, 86_400);
    let mut averted = reload(&base);
    averted.set_pay_level(Role::Technician, 100).unwrap();
    assert!(averted.labour_group(Role::Technician).warning_at.is_none());
    averted.clock.set_running(true);
    assert!(averted.advance(2.0 * DAY));
    assert!(averted.finance.totals.flights > 10);
    let summary = base.jump(2.0 * DAY).unwrap();
    assert!(summary.reason.contains("Streik"));
    assert_eq!(summary.advanced_seconds, 86_400);
    assert!(base.labour_group(Role::Technician).strike_until.is_some());
    let mut natural = game();
    natural.set_pay_level(Role::Technician, 85).unwrap();
    natural.staff.labour[2].morale = 20;
    natural.staff.labour[2].pay_percent = 85;
    let summary = natural.jump(31.0 * DAY).unwrap();
    assert!(summary.reason.contains("Streikwarnung"));
    assert_eq!(summary.advanced_seconds, 30 * 86_400);
    assert_eq!(
        natural.labour_group(Role::Technician).warning_at,
        Some(40 * 86_400)
    );
}
#[test]
fn labour_strike_preserves_airborne_and_is_identical_across_speeds_steps_pause_reload() {
    let mut base = game();
    base.schedule(1, 1, 120).unwrap();
    base.clock.set_running(true);
    assert!(base.advance(61.0));
    let airborne = base.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .clone()
        .unwrap();
    warning(&mut base, Role::Technician, 62);
    assert!(base.advance(1.0));
    assert_eq!(
        base.routes[0].plan.as_ref().unwrap().airborne.as_ref(),
        Some(&airborne)
    );
    let mut strike = reload(&base);
    assert!(strike.advance(DAY));
    assert_eq!(strike.finance.totals.flights, 1);
    assert!(strike.routes[0].plan.as_ref().unwrap().airborne.is_none());
    let mut mid = reload(&strike);
    assert!(mid.advance(2.0 * DAY + 3600.0));
    assert!(mid.labour_group(Role::Technician).strike_until.is_none());
    assert!(mid.finance.totals.flights > 1);
    let duration = 3.0 * DAY + 3600.0;
    let mut exact = base.clone();
    assert!(exact.advance(duration));
    for speed in [1, 4, 16] {
        for steps in [1, 73] {
            let mut g = base.clone();
            g.clock.set_speed(speed);
            g.clock.set_running(false);
            assert!(g.advance(1000.0));
            g.clock.set_running(true);
            for i in 0..steps {
                assert!(g.advance(duration / f64::from(speed) / f64::from(steps)));
                if i == steps / 2 {
                    g = reload(&g);
                }
            }
            g.clock.set_speed(1);
            assert_eq!(state(&g), state(&exact));
            g.validate_history().unwrap();
        }
    }
}
#[test]
fn labour_ai_policy_and_financial_factors_use_shared_rules_and_terminal_cleanup() {
    let mut g = game();
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY));
    // A solvent review restores the market rate. Distress cuts are bounded and
    // restored for unhappy employees; the same payroll function applies to AI.
    g.staff.labour_period = 0;
    g.airline.as_mut().unwrap().capital_eur = -1;
    g.ai_labour_policy().unwrap();
    assert_eq!(g.staff.labour[0].pending_percent, 95);
    g.staff.labour[0].morale = 50;
    g.ai_labour_policy().unwrap();
    assert_eq!(g.staff.labour[0].pending_percent, 100);
    g.airline.as_mut().unwrap().capital_eur = 1_000_000;
    g.ai_labour_policy().unwrap();
    assert!(g.staff.labour.iter().all(|g| g.pending_percent == 100));
    g.staff.labour[0].morale = 75;
    g.resolution.deadline = Some(99 * 86_400);
    g.close_labour_period(30 * 86_400, true).unwrap();
    assert_eq!(g.labour_group(Role::Pilot).morale, 65);
    assert!(
        g.labour_group(Role::Pilot).missed_payroll && g.labour_group(Role::Pilot).restructuring
    );
    warning(&mut g, Role::Technician, 31 * 86_400);
    g.resolution.insolvent_at = Some(30 * 86_400);
    g.apply_resolution();
    assert!(g.next_labour_event().is_none());
}
#[test]
fn labour_v39_migration_is_neutral_for_every_owner_and_retains_airborne_money() {
    let mut g = game();
    g.competitors = crate::competition::initial_competitors().unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(61.0));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v39.json");
    save::save(&path, &g).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["version"] = 39.into();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.remove("labour");
                o.remove("labour_period");
                o.remove("salary_since");
                o.remove("salary_remainder");
                for v in o.values_mut() {
                    strip(v);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    strip(v)
                }
            }
            _ => {}
        }
    }
    strip(&mut value);
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let mut loaded = save::load(&path).unwrap();
    g.adopt_events_from(&loaded);
    assert_eq!(g.cash(), loaded.cash());
    assert_eq!(g.finance, loaded.finance);
    assert_eq!(g.routes, loaded.routes);
    for owner in std::iter::once(&loaded).chain(loaded.competitors.iter().map(|c| c.game.as_ref()))
    {
        assert!(
            owner
                .staff
                .labour
                .iter()
                .all(|g| g.pay_percent == 100 && g.morale == labour::rules().neutral_morale)
        );
    }
    assert!(g.advance(DAY));
    assert!(loaded.advance(DAY));
    assert_eq!(state(&g), state(&loaded));
    loaded.staff.labour[0].morale = 101;
    assert!(save::save(&path, &loaded).is_err());
    assert_eq!(staff::period_seconds().unwrap(), 30 * 86_400);
}

#[test]
fn labour_first_hire_after_empty_period_does_not_replay_pay_or_morale() {
    let mut g = Game::default();
    g.found("Late Hire", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.clock.set_running(true);
    assert!(g.advance(40.0 * DAY));
    g.hire_staff(Role::Pilot, 1).unwrap();
    g.set_pay_level(Role::Pilot, 85).unwrap();
    assert!(g.advance(DAY));
    assert_eq!(g.labour_group(Role::Pilot).pay_percent, 100);
    assert_eq!(g.labour_group(Role::Pilot).morale, 75);
    g.validate_staff().unwrap();
    let cash = g.cash();
    assert!(g.advance(19.0 * DAY));
    assert_eq!(cash - g.cash(), 4333);
    assert_eq!(g.labour_group(Role::Pilot).pay_percent, 85);
    let mut g = reload(&g);
    assert!(g.advance(30.0 * DAY));
    g.validate_history().unwrap();
}

#[test]
fn labour_cabin_strike_does_not_block_freighters_and_technician_strike_blocks_schedules() {
    let mut g = game();
    g.sell_aircraft(1).unwrap();
    g.buy_aircraft("atr72_600f").unwrap();
    g.equip_staff_for("atr72_600f").unwrap();
    g.create_route("EDDF", "EDDM", 2).unwrap();
    g.schedule(2, 1, 120).unwrap();
    warning(&mut g, Role::CabinCrew, 86_400);
    g.clock.set_running(true);
    assert!(g.advance(2.0 * DAY));
    assert!(g.finance.totals.flights > 10);
    assert!(g.labour_group(Role::CabinCrew).strike_until.is_some());
    let mut parked = game();
    warning(&mut parked, Role::Technician, 1);
    parked.clock.set_running(true);
    assert!(parked.advance(1.0));
    assert!(parked.schedule(1, 1, 120).unwrap_err().contains("Streik"));
    parked.set_pay_level(Role::Technician, 120).unwrap();
    assert!(parked.labour_group(Role::Technician).strike_until.is_some());
    assert!(parked.advance(3.0 * DAY));
    assert!(parked.labour_group(Role::Technician).strike_until.is_none());
    parked.schedule(1, 1, 120).unwrap();
}

#[test]
fn labour_understaffing_cooldown_and_threshold_boundaries() {
    let mut g = game();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY));
    // Exact resignation threshold is safe, and a future cooldown prevents a warning.
    let r = labour::rules();
    let group = &mut g.staff.labour[0];
    group.morale = 40;
    group.pay_percent = 85;
    group.pending_percent = 85;
    group.cooldown_until = 100 * 86_400;
    g.close_labour_period(30 * 86_400, false).unwrap();
    assert_eq!(g.labour_group(Role::Pilot).resigned, 1);
    assert!(g.labour_group(Role::Pilot).warning_at.is_none());
    assert!(g.advance(30.0 * DAY));
    assert!(g.labour_group(Role::Pilot).understaffed);
    assert!(g.labour_group(Role::Pilot).warning_at.is_none());
    assert!(r.target(100) >= r.calm_threshold);
    assert!(r.target(85) < r.strike_threshold);
    assert!(r.target(120) > r.neutral_morale);
    let mut boundary = game();
    boundary.staff.labour[0].morale = 50;
    boundary.staff.labour[0].pay_percent = 85;
    boundary.staff.labour[0].pending_percent = 85;
    boundary.close_labour_period(0, false).unwrap();
    assert_eq!(boundary.labour_group(Role::Pilot).morale, 40);
    assert_eq!(boundary.labour_group(Role::Pilot).resigned, 0);
}

#[test]
fn labour_ai_strike_reaches_same_event_boundaries_and_survives_restart() {
    let mut g = Game::default();
    g.found("Rival Labour", "DE", "EDDF").unwrap();
    warning(&mut g.competitors[0].game, Role::Technician, 86_400);
    g.clock.set_running(true);
    assert!(g.advance(DAY));
    assert!(
        g.competitors[0]
            .game
            .labour_group(Role::Technician)
            .strike_until
            .is_some()
    );
    let mut loaded = reload(&g);
    assert!(g.advance(DAY));
    assert!(loaded.advance(DAY));
    assert_eq!(state(&g), state(&loaded));
    let flights = g.competitors[0].game.finance.totals.flights;
    assert!(g.advance(DAY));
    assert_eq!(g.competitors[0].game.finance.totals.flights, flights);
    assert!(g.advance(DAY));
    assert!(
        g.competitors[0]
            .game
            .labour_group(Role::Technician)
            .strike_until
            .is_none()
    );
    assert!(!g.competitors[0].game.insolvent());
    g.validate_competitors().unwrap();
}

#[test]
fn labour_market_pay_does_not_immunize_financially_distressed_owners_from_strikes() {
    let mut g = game();
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY));
    g.resolution.deadline = Some(100 * 86_400);
    g.staff.labour[0].morale = 0;
    g.close_labour_period(30 * 86_400, true).unwrap();
    assert!(g.labour_group(Role::Pilot).warning_at.is_some());
    g.set_pay_level(Role::Pilot, 100).unwrap();
    assert!(g.labour_group(Role::Pilot).warning_at.is_some());
    g.set_pay_level(Role::Pilot, 120).unwrap();
    assert!(g.labour_group(Role::Pilot).warning_at.is_none());
}

#[test]
fn labour_ferry_dispatch_is_blocked_but_an_airborne_ferry_finishes() {
    let mut parked = game();
    parked.delete_route(1).unwrap();
    warning(&mut parked, Role::Pilot, 1);
    parked.clock.set_running(true);
    assert!(parked.advance(1.0));
    assert!(
        parked
            .quote_ferry(1, "EDDM")
            .unwrap_err()
            .contains("Streik")
    );
    let mut flying = game();
    flying.delete_route(1).unwrap();
    flying.send_ferry(1, "EDDM").unwrap();
    warning(&mut flying, Role::Pilot, 1);
    flying.clock.set_running(true);
    assert!(flying.advance(DAY));
    assert_eq!(flying.fleet[0].location_icao, "EDDM");
    assert_ne!(
        flying.fleet[0].status,
        crate::aircraft::AircraftStatus::InFlight
    );
    let _ = reload(&flying);
}
