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
