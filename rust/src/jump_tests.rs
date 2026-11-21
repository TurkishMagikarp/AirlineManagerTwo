use crate::{clock::Clock, game::Game, save};
use std::time::{Duration, Instant};
const DAY: u64 = 86400;
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn game() -> Game {
    let mut g = Game::default();
    g.found("Jump Air", "DE", "EDDF").unwrap();
    g.lease_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 110).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g
}
fn small_steps(g: &mut Game, seconds: u64) {
    let running = g.clock.is_running();
    g.clock.set_running(true);
    let speed = f64::from(g.clock.speed());
    let mut left = seconds;
    while left > 0 {
        let step = left.min(900);
        assert!(g.advance(step as f64 / speed));
        left -= step;
    }
    g.clock.set_running(running && !g.insolvent());
}
#[test]
fn jumps_match_small_steps_all_speeds_paused_month_lease_and_restart() {
    let initial = game();
    for speed in [1, 4, 16] {
        for running in [false, true] {
            let mut jumped = initial.clone();
            jumped.clock.set_speed(speed);
            jumped.clock.set_running(running);
            let mut reference = jumped.clone();
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("jump.json");
            // Starting at a subsecond also retains the exact phase.
            jumped.clock = Clock::from_state(Duration::from_millis(125), running, speed).unwrap();
            reference.clock = jumped.clock.clone();
            let end = jumped.clock.elapsed() + Duration::from_secs(32 * DAY);
            let mut restarted = false;
            while jumped.clock.elapsed() < end {
                let remaining = (end - jumped.clock.elapsed()).as_secs();
                let result = jumped.jump(remaining.min(7 * DAY) as f64).unwrap();
                assert!(result.advanced_seconds > 0);
                small_steps(&mut reference, result.advanced_seconds);
                assert_eq!(
                    state(&jumped),
                    state(&reference),
                    "speed={speed} running={running}"
                );
                if !restarted && jumped.clock.elapsed().as_secs() >= 15 * DAY {
                    save::save(&path, &jumped).unwrap();
                    jumped = save::load(&path).unwrap();
                    restarted = true;
                }
            }
            assert!(restarted);
            assert!(
                jumped
                    .history
                    .as_ref()
                    .unwrap()
                    .aircraft_cash
                    .iter()
                    .any(|e| e.kind == crate::trading::Kind::Rent && e.time == 30 * DAY)
            );
        }
    }
}
#[test]
fn jumps_tax_boundary_matches_waiting() {
    let mut g = game();
    g.clock.set_running(true);
    assert!(g.advance((crate::demand::YEAR_SECONDS - DAY) as f64));
    let mut reference = g.clone();
    let r = g.jump((2 * DAY) as f64).unwrap();
    small_steps(&mut reference, r.advanced_seconds);
    assert_eq!(state(&g), state(&reference));
    assert_eq!(g.history.as_ref().unwrap().taxes.len(), 1);
}
#[test]
fn jumps_new_attention_distress_and_insolvency_reuse_reasons() {
    // A parked leased aircraft already has an idle entry. Rent creates a NEW
    // cash warning at day 30; the existing idle entry must not stop day one.
    let mut g = Game::default();
    g.found("Rent", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.lease_aircraft("atr72_600").unwrap();
    let shift = 1 - g.cash();
    g.airline.as_mut().unwrap().capital_eur += shift;
    g.history.as_mut().unwrap().opening_balance += shift;
    let mut reference = g.clone();
    let r = g.jump((31 * DAY) as f64).unwrap();
    assert_eq!(r.advanced_seconds, 30 * DAY);
    assert_eq!(r.reason, g.financing_status());
    assert!(g.distressed());
    small_steps(&mut reference, r.advanced_seconds);
    assert_eq!(state(&g), state(&reference));
    // Cash text changing on an existing warning does not create a new entry.
    let mut r = g.jump((30 * DAY) as f64).unwrap();
    while !g.insolvent() {
        r = g.jump((30 * DAY) as f64).unwrap();
    }
    assert!(g.insolvent());
    assert!(r.reason.contains("INSOLVENT"));

    // New salary-reserve warning without distress: zero-time loan repayment
    // is not involved; payroll itself crosses the configured reserve.
    let mut g = Game::default();
    g.found("Payroll", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    let reserve = g.salary_reserve().unwrap();
    let shift = reserve - g.cash();
    g.airline.as_mut().unwrap().capital_eur += shift;
    g.history.as_mut().unwrap().opening_balance += shift;
    let r = g.jump((31 * DAY) as f64).unwrap();
    assert!(!g.distressed());
    assert_eq!(
        r.reason,
        g.attention_snapshot()
            .into_iter()
            .find(|e| e.kind == "salary_reserve")
            .unwrap()
            .text
    );
    assert!(r.advanced_seconds < 31 * DAY);
    let target = r.attention.as_ref().unwrap();
    assert_eq!(target.kind, "salary_reserve");
    assert!(g.attention_snapshot().len() > 1 || g.attention_snapshot()[0].kind == "salary_reserve");
}
#[test]
fn jumps_rejected_inputs_leave_every_owner_untouched() {
    for mut g in [Game::default(), game()] {
        let before = state(&g);
        for seconds in [
            0.0,
            -1.0,
            0.5,
            f64::NAN,
            f64::INFINITY,
            f64::MAX,
            u64::MAX as f64,
        ] {
            assert!(g.jump(seconds).is_err());
            assert_eq!(state(&g), before);
        }
        if g.airline.is_none() {
            assert!(g.jump(3600.0).is_err());
            assert_eq!(state(&g), before);
        }
    }
    let mut overflowing_cash = game();
    overflowing_cash.airline.as_mut().unwrap().capital_eur = i64::MAX;
    let before = state(&overflowing_cash);
    assert!(overflowing_cash.jump((30 * DAY) as f64).is_err());
    assert_eq!(state(&overflowing_cash), before);
    let mut g = game();
    g.clock = Clock::from_state(Duration::MAX, false, 16).unwrap();
    let before = state(&g);
    assert!(g.jump(3600.0).is_err());
    assert_eq!(state(&g), before);
}
#[test]
fn jumps_busy_six_ai_thirty_day_timing() {
    let mut g = game();
    // Six real AI companies from the three catalogue regions.
    g.competitors.clear();
    for home in ["EDDF", "SBGR", "RJTT"] {
        g.competitors
            .extend(crate::competition::new_competitors(home).unwrap());
    }
    for _ in 0..3 {
        g.buy_aircraft("atr72_600").unwrap();
    }
    for id in 2..=4 {
        g.create_route("EDDF", "EDDM", id).unwrap();
        g.staff_up();
        g.schedule(id, 1, 120).unwrap();
    }
    g.airline.as_mut().unwrap().capital_eur += 1_000_000_000;
    g.history.as_mut().unwrap().opening_balance += 1_000_000_000;
    // Warm up real AI growth so this is a busy world, not only starter fleets.
    g.clock.set_running(true);
    assert!(g.advance((180 * DAY) as f64));
    assert_eq!(g.competitors.len(), 6);
    let mut reference = g.clone();
    let start = Instant::now();
    let end = g.clock.elapsed() + Duration::from_secs(30 * DAY);
    let mut stops = 0;
    while g.clock.elapsed() < end {
        let r = g.jump((end - g.clock.elapsed()).as_secs_f64()).unwrap();
        assert!(r.advanced_seconds > 0);
        stops += usize::from(!r.reason.is_empty());
    }
    eprintln!(
        "JUMP_BUSY six_ai fleet={} thirty_days={:?} attention_stops={stops}",
        g.fleet.len(),
        start.elapsed()
    );
    assert!(reference.advance((30 * DAY) as f64));
    assert_eq!(state(&g), state(&reference));
}

fn commitment_game() -> Game {
    let mut g = Game::default();
    g.found("Commit Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.history.as_mut().unwrap().opening_balance += 1_000_000_000 - g.cash();
    g.airline.as_mut().unwrap().capital_eur = 1_000_000_000;
    g.place_order("atr72_600", 1).unwrap();
    g.hire_pilots("737", 1).unwrap();
    let id = g.staff.employees[0].id;
    g.train_pilot(id, "ATR 72").unwrap();
    g
}

#[test]
fn jump_to_commitment_arrives_exactly_from_fractional_clock_and_matches_advance() {
    for start_ms in [0u64, 125, 999] {
        for speed in [1, 4, 16] {
            let mut g = commitment_game();
            g.clock = Clock::from_state(Duration::from_millis(start_ms), false, speed).unwrap();
            let next = g.next_commitment_jump();
            let next = next.expect("commitment within the longest preset");
            let mut reference = g.clone();
            let summary = g.jump_to_commitment(next.at).unwrap();
            assert!(summary.reason.is_empty(), "{}", summary.reason);
            {
                assert_eq!(g.clock.elapsed(), Duration::from_secs(next.at));
                small_steps(&mut reference, next.at - start_ms / 1000);
                // Reference started at the same fraction, so compare after aligning.
                reference.clock =
                    Clock::from_state(Duration::from_secs(next.at), false, speed).unwrap();
                g.clock = reference.clock.clone();
                assert_eq!(state(&g), state(&reference));
            }
        }
    }
}

#[test]
fn jump_to_commitment_stale_or_out_of_range_target_leaves_time_intact() {
    let mut g = commitment_game();
    let before = state(&g);
    assert!(g.jump_to_commitment(1).is_err());
    assert!(g.jump_to_commitment(u64::MAX).is_err());
    assert_eq!(state(&g), before);
    let next = g.next_commitment_jump();
    if let Some(next) = next {
        assert!(g.jump_to_commitment(next.at + 1).is_err());
        assert_eq!(state(&g), before);
    }
    let mut far = commitment_game();
    far.orders[0].delivery_at = 400 * DAY;
    far.staff.employees[0].training = None;
    assert!(far.next_commitment_jump().is_none());
    assert!(far.jump_to_commitment(400 * DAY).is_err());
    let mut held = commitment_game();
    held.staff.employees[0].training = None;
    held.orders[0].held_since = Some(0);
    assert!(held.next_commitment_jump().is_none());
}

#[test]
fn jump_to_commitment_stops_early_on_attention() {
    let mut g = game();
    g.place_order("atr72_600", 1).unwrap();
    let at = g.orders[0].delivery_at;
    g.orders[0].delivery_at = at.min(5 * DAY);
    let at = g.orders[0].delivery_at;
    let mut stopped = g.clone();
    let summary = stopped.jump_to_commitment(at).unwrap();
    assert!(stopped.clock.elapsed() <= Duration::from_secs(at));
    if summary.reason.is_empty() {
        assert_eq!(stopped.clock.elapsed(), Duration::from_secs(at));
    }
}

fn near_limit() -> Game {
    let mut g = game();
    let rules = crate::maintenance::rules("atr72_600").unwrap();
    g.fleet[0].wear.cycles = rules.interval_cycles - 1;
    g.fleet[0].wear.check_cycles = rules.interval_cycles - 1;
    g
}

fn forced_entries(g: &Game) -> Vec<String> {
    g.attention_snapshot()
        .into_iter()
        .filter(|e| e.kind == "forced_check")
        .map(|e| e.text)
        .collect()
}

#[test]
fn jump_stops_at_wear_grounding_inside_one_sampling_window_and_resumes() {
    let initial = near_limit();
    let landing = initial.due_landing(0).unwrap();
    let mut jumped = initial.clone();
    let result = jumped.jump(10.0 * DAY as f64).unwrap();
    assert_eq!(jumped.clock.elapsed().as_secs(), landing);
    assert_eq!(result.advanced_seconds, landing);
    assert!(result.reason.contains(&jumped.fleet[0].identifier()));
    assert!(result.reason.contains("Pflichtwartung"));
    assert_eq!(
        jumped.fleet[0].status,
        crate::aircraft::AircraftStatus::Maintenance
    );
    assert_eq!(forced_entries(&jumped).len(), 1);
    let mut reference = initial.clone();
    small_steps(&mut reference, landing);
    assert_eq!(state(&jumped), state(&reference));
    let forced = jumped
        .attention_snapshot()
        .into_iter()
        .find(|e| e.kind == "forced_check")
        .unwrap();
    assert_eq!(result.attention, Some(forced));

    let end = jumped.fleet[0].maintenance.as_ref().unwrap().end;
    let cost = jumped.fleet[0].maintenance.as_ref().unwrap().cost_eur;
    let again = jumped.jump(10.0 * DAY as f64).unwrap();
    assert!(again.reason.is_empty(), "{}", again.reason);
    assert!(again.attention.is_none());
    assert!(forced_entries(&jumped).is_empty());
    let total = landing + again.advanced_seconds;
    small_steps(&mut reference, again.advanced_seconds);
    assert_eq!(state(&jumped), state(&reference));
    assert!(total > end);
    let paid: i64 = jumped
        .history
        .as_ref()
        .unwrap()
        .maintenance
        .iter()
        .filter(|p| p.time == landing)
        .map(|p| p.amount)
        .sum();
    assert_eq!(paid, cost);
    assert!(jumped.routes[0].plan.is_some());
}

#[test]
fn waiting_forced_check_stops_the_jump_then_updates_when_staffed() {
    let mut g = near_limit();
    let landing = g.due_landing(0).unwrap();
    let techs: Vec<u32> = g
        .staff
        .employees
        .iter()
        .filter(|e| e.role == crate::staff::Role::Technician)
        .map(|e| e.id)
        .collect();
    for id in techs {
        g.dismiss_staff(id).unwrap();
    }
    let result = g.jump(10.0 * DAY as f64).unwrap();
    assert_eq!(g.clock.elapsed().as_secs(), landing);
    assert!(result.reason.contains("Techniker"));
    assert_eq!(forced_entries(&g).len(), 1);
    assert!(forced_entries(&g)[0].contains("wartet auf Techniker"));
    g.staff_up();
    g.clock.set_running(true);
    g.advance(1.0);
    assert_eq!(forced_entries(&g).len(), 1);
    assert!(forced_entries(&g)[0].contains("frei ab"));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forced.json");
    save::save(&path, &g).unwrap();
    assert_eq!(forced_entries(&save::load(&path).unwrap()), forced_entries(&g));
}

#[test]
fn stopped_jump_names_the_new_entry_among_simultaneous_notices_for_both_entry_points() {
    let mut base = near_limit();
    base.place_order("atr72_600", 1).unwrap();
    let landing = base.due_landing(0).unwrap();
    base.orders[0].delivery_at = landing + 5 * DAY;
    let at = base.orders[0].delivery_at;
    // An unrelated existing notice must not be chosen over the forced check.
    let mut preset = base.clone();
    let r = preset.jump(10.0 * DAY as f64).unwrap();
    let mut commitment = base.clone();
    let c = commitment.jump_to_commitment(at).unwrap();
    for (g, r) in [(&preset, &r), (&commitment, &c)] {
        let forced = g
            .attention_snapshot()
            .into_iter()
            .find(|e| e.kind == "forced_check")
            .unwrap();
        assert_eq!(r.attention, Some(forced));
        assert_eq!(g.clock.elapsed().as_secs(), landing);
    }
}

#[test]
fn financing_stop_and_completed_jump_have_no_attention_target() {
    let mut g = game();
    let done = g.jump(DAY as f64).unwrap();
    assert!(done.reason.is_empty() && done.attention.is_none());
    let mut rent = Game::default();
    rent.found("Rent", "DE", "EDDF").unwrap();
    rent.competitors.clear();
    rent.lease_aircraft("atr72_600").unwrap();
    let shift = 1 - rent.cash();
    rent.airline.as_mut().unwrap().capital_eur += shift;
    rent.history.as_mut().unwrap().opening_balance += shift;
    let r = rent.jump((31 * DAY) as f64).unwrap();
    assert_eq!(r.reason, rent.financing_status());
    assert!(r.attention.is_none());
}
