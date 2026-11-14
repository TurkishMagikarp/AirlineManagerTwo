use crate::{
    financing::{Financing, RULES},
    game::Game,
    save,
    staff::Role,
};

const DAY: f64 = 86400.0;

fn fresh() -> Game {
    let mut g = Game::default();
    g.found("Loan Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g
}

fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}

fn cash(g: &mut Game, amount: i64) {
    let shift = amount - g.cash();
    g.airline.as_mut().unwrap().capital_eur = amount;
    // Test fixture opening balance, never a production cash injection.
    g.history.as_mut().unwrap().opening_balance += shift;
}

#[test]
fn financing_borrow_repay_limits_and_history_are_transactional() {
    let mut g = fresh();
    let start = g.cash();
    for amount in [0, -1, RULES.maximum_loan_eur + 1] {
        let before = state(&g);
        assert!(g.borrow(amount).is_err());
        assert_eq!(state(&g), before);
    }
    g.borrow(1_200_000).unwrap();
    assert_eq!(g.cash(), start + 1_200_000);
    assert_eq!(g.financing.installment, 50_000);
    let before = state(&g);
    assert!(g.borrow(1).is_err());
    assert!(g.repay(1_200_001).is_err());
    assert_eq!(state(&g), before);
    g.repay(200_000).unwrap();
    assert_eq!(g.financing.debt, 1_000_000);
    assert_eq!(g.financing.overdue, 0);
    g.validate_financing().unwrap();
    g.validate_history().unwrap();
    g.repay(1_000_000).unwrap();
    assert_eq!(g.financing, Financing::default());
    assert_eq!(g.cash(), start);
    g.validate_financing().unwrap();
    g.validate_history().unwrap();
    g.borrow(100).unwrap();
}

#[test]
fn financing_interest_due_dates_and_early_repayment() {
    let mut g = fresh();
    g.borrow(1_200_000).unwrap();
    let start = g.cash();
    let interest = g.loan_interest();
    assert_eq!(interest, 5918);
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY - 1.0));
    assert_eq!(g.cash(), start);
    assert!(g.advance(1.0));
    assert_eq!(g.cash(), start - interest - 50_000);
    assert_eq!(g.financing.debt, 1_150_000);
    assert_eq!(g.financing.next_due, 60 * 86400);
    assert_eq!(
        g.history.as_ref().unwrap().interest_through(u64::MAX),
        i128::from(interest)
    );
    g.validate_history().unwrap();
    g.validate_financing().unwrap();
    g.repay(g.financing.debt).unwrap();
    let paid = g.cash();
    assert!(g.advance(100.0 * DAY));
    assert_eq!(g.cash(), paid);
}

#[test]
fn financing_insufficient_cash_distress_and_recovery_without_free_credit() {
    let mut g = fresh();
    cash(&mut g, 0);
    assert!(g.buy_aircraft("atr72_600").is_err());
    assert!(g.hire_staff(Role::Pilot, 1).is_err());
    g.borrow(120_000).unwrap();
    // Consume all but EUR 1 through a real purchase-journal fixture.
    g.history
        .as_mut()
        .unwrap()
        .purchases
        .push(crate::history::Purchase {
            time: 0,
            amount: 119_999,
        });
    g.airline.as_mut().unwrap().capital_eur = 1;
    let before = state(&g);
    assert!(g.repay(2).is_err());
    assert_eq!(state(&g), before);
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY));
    assert!(g.distressed());
    assert!(g.cash() < 0);
    assert_eq!(g.financing.overdue, 5000);
    assert!(g.buy_aircraft("atr72_600").is_err());
    assert!(g.borrow(1_000_000).is_err());
    g.validate_history().unwrap();
    g.validate_financing().unwrap();
    // Restored operating liquidity clears arrears without erasing the loan.
    cash(&mut g, 10_000);
    g.apply_financing().unwrap();
    assert!(!g.distressed());
    assert_eq!(g.financing.overdue, 0);
    assert_eq!(g.financing.debt, 115_000);
    assert_eq!(g.cash(), 5000);
    g.validate_financing().unwrap();
    g.validate_history().unwrap();

    // A legacy deficit without a facility can recover using the same capped loan.
    let mut legacy = fresh();
    cash(&mut legacy, -10_000);
    assert!(legacy.borrow(9999).is_err());
    legacy.borrow(100_000).unwrap();
    assert!(!legacy.distressed());
    assert_eq!(legacy.cash(), 90_000);
    legacy.validate_history().unwrap();
}

fn loss_service() -> Game {
    let mut g = fresh();
    g.buy_aircraft("atr72_600").unwrap();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.equip_staff_for("atr72_600").unwrap();
    g.set_ticket_price(1, 0).unwrap();
    g.schedule(1, 1, 120).unwrap();
    // Correctly rebase the fixture after buying the plane.
    let purchases: i64 = g
        .history
        .as_ref()
        .unwrap()
        .purchases
        .iter()
        .map(|p| p.amount)
        .sum();
    g.airline.as_mut().unwrap().capital_eur = 1;
    g.history.as_mut().unwrap().opening_balance = purchases + 1;
    g.clock.set_running(true);
    g
}

#[test]
fn financing_loss_flights_finish_and_stop_identically_at_all_speeds() {
    let mut large = loss_service();
    assert!(large.advance(3.0 * DAY));
    assert!(large.distressed());
    assert!(large.routes[0].plan.is_none());
    assert_eq!(large.finance.totals.flights, 1);
    large.validate_operations().unwrap();
    large.validate_history().unwrap();
    for speed in [1, 4, 16] {
        let mut small = loss_service();
        small.clock.set_speed(speed);
        for _ in 0..4320 {
            assert!(small.advance(60.0 / f64::from(speed)));
        }
        small.clock.set_speed(1);
        assert_eq!(state(&small), state(&large));
    }
    // Rescue, not an automatic restart; the player still chooses what to operate.
    large.borrow(1_000_000).unwrap();
    assert!(!large.distressed());
    assert!(large.routes[0].plan.is_none());
}

#[test]
fn financing_zero_gap_turnaround_cancels_only_the_next_departure() {
    let mut g = loss_service();
    let (flight, ground) = crate::operations::timings(&g.routes[0], "atr72_600").unwrap();
    g.routes[0].plan.as_mut().unwrap().interval_seconds = flight + ground;
    let mut small = g.clone();
    assert!(g.advance(3.0 * DAY));
    for _ in 0..4320 {
        assert!(small.advance(60.0));
    }
    assert_eq!(state(&g), state(&small));
    assert_eq!(g.finance.totals.flights, 1);
    assert!(g.routes[0].plan.is_none());
    assert_eq!(g.fleet[0].location_icao, "EDDM");
    g.validate_operations().unwrap();
    g.validate_history().unwrap();
}

#[test]
fn financing_amortizes_fully_once_and_does_not_charge_after_payoff() {
    let mut g = fresh();
    let opening = g.cash();
    g.borrow(1_200_001).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(720.0 * DAY));
    assert_eq!(g.financing, Financing::default());
    let h = g.history.as_ref().unwrap();
    let interest = h.interest_through(u64::MAX) as i64;
    assert_eq!(g.cash(), opening - interest);
    assert_eq!(h.financing.iter().filter(|e| e.principal < 0).count(), 24);
    assert!(h.financing.last().unwrap().principal.abs() < 50_001);
    let paid = g.finance.clone();
    let balance = g.cash();
    assert!(g.advance(60.0 * DAY));
    assert_eq!(g.cash(), balance);
    assert_eq!(g.finance, paid);
    g.validate_history().unwrap();
    g.validate_financing().unwrap();
}

#[test]
fn financing_save_load_migration_corruption_and_time_acceleration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("loans.json");
    let mut initial = fresh();
    initial.borrow(2_400_000).unwrap();
    initial.clock.set_running(true);
    let mut expected = initial.clone();
    assert!(expected.advance(93.0 * DAY));
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        for day in 0..93 {
            assert!(g.advance(DAY / f64::from(speed)));
            if day == 31 {
                save::save(&path, &g).unwrap();
                g = save::load(&path).unwrap();
            }
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected));
    }
    save::save(&path, &expected).unwrap();
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for field in ["debt", "installment", "next_due", "overdue"] {
        let mut bad = original.clone();
        bad["state"]["financing"][field] = serde_json::json!(-1);
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(save::load(&path).is_err(), "{field}");
    }
    let old = fresh();
    save::save(&path, &old).unwrap();
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    legacy["version"] = serde_json::json!(11);
    legacy["state"].as_object_mut().unwrap().remove("financing");
    legacy["state"]["history"]
        .as_object_mut()
        .unwrap()
        .remove("financing");
    std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert_eq!(state(&save::load(&path).unwrap()), state(&old));
}

#[test]
fn financing_ai_borrows_for_existing_costs_restricts_growth_and_repays() {
    let mut g = Game::default();
    g.found("AI loans", "DE", "EDDF").unwrap();
    for c in &mut g.competitors {
        // Start with no service: low liquidity triggers one capped reserve loan.
        c.game.routes.clear();
        c.game.fleet.clear();
        c.route_reviews.clear();
        c.game.staff = Default::default();
        c.game.hire_batch(&[(Role::Pilot, 1)], false).unwrap();
        c.game.finance = Default::default();
        c.game.history = Some(crate::history::History::new(0, 0));
        c.game.airline.as_mut().unwrap().capital_eur = 0;
    }
    g.clock.set_running(true);
    assert!(g.advance(7.0 * DAY));
    for c in &g.competitors {
        assert!(c.game.financing.debt > 0);
        assert!(c.game.fleet.is_empty());
        c.game.validate_financing().unwrap();
        c.game.validate_history().unwrap();
    }
    // Real excess liquidity permits early repayment, not a permanent subsidy.
    for c in &mut g.competitors {
        cash(&mut c.game, 5_000_000);
    }
    assert!(g.advance(7.0 * DAY));
    assert!(g.competitors.iter().all(|c| c.game.financing.debt == 0));
    g.validate_competitors().unwrap();
}

#[test]
fn financing_ai_distress_and_loans_match_across_restart_and_all_speeds() {
    let mut initial = Game::default();
    initial.found("AI distress", "DE", "EDDF").unwrap();
    // Aircraft sales are now a recovery source. Keep this fixture beyond both
    // liquidation proceeds and the capped rescue facility.
    let liquidation: i64 = initial.competitors[0]
        .game
        .fleet
        .iter()
        .map(|p| initial.competitors[0].game.disposal_quote(p.id).unwrap())
        .sum();
    cash(
        &mut initial.competitors[0].game,
        -RULES.maximum_loan_eur - liquidation - 1,
    );
    cash(&mut initial.competitors[1].game, 100_000);
    initial.clock.set_running(true);
    let mut expected = initial.clone();
    assert!(expected.advance(45.0 * DAY));
    let broken = &expected.competitors[0].game;
    assert!(broken.distressed());
    assert_eq!(broken.financing.debt, 0);
    assert!(broken.routes.iter().all(|r| r.plan.is_none()));
    assert!(broken.staff.employees.len() < initial.competitors[0].game.staff.employees.len());
    expected.validate_competitors().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ai-loans.json");
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        for day in 0..45 {
            assert!(g.advance(DAY / f64::from(speed)));
            if day == 14 {
                save::save(&path, &g).unwrap();
                g = save::load(&path).unwrap();
            }
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected));
    }
}

#[test]
fn restructuring_grace_deadline_terminal_save_and_management_lock() {
    let mut g = fresh();
    cash(&mut g, -1);
    g.clock.set_running(true);
    let grace = RULES.distress_grace_days as f64 * DAY;
    let window = RULES.restructuring_days as f64 * DAY;
    assert!(g.advance(grace - 1.0));
    assert_eq!(g.resolution.distress_since, Some(0));
    assert_eq!(g.resolution.deadline, None);
    assert!(g.advance(1.0));
    assert_eq!(g.resolution.deadline, Some((grace + window) as u64));
    assert!(!g.insolvent());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("restructuring.json");
    save::save(&path, &g).unwrap();
    let mut loaded = save::load(&path).unwrap();
    assert_eq!(state(&loaded), state(&g));
    assert!(loaded.advance(window - 1.0));
    assert!(!loaded.insolvent());
    assert!(loaded.advance(1000.0));
    assert!(loaded.insolvent());
    assert_eq!(loaded.clock.elapsed().as_secs(), (grace + window) as u64);
    assert!(!loaded.clock.is_running());
    let terminal = state(&loaded);
    assert!(loaded.advance(100.0 * DAY));
    assert!(loaded.found("Restart", "DE", "EDDF").is_err());
    assert!(loaded.borrow(100).is_err());
    assert!(loaded.repay(1).is_err());
    assert!(loaded.buy_aircraft("atr72_600").is_err());
    assert!(loaded.hire_staff(Role::Pilot, 1).is_err());
    assert!(loaded.reschedule_loan().is_err());
    assert_eq!(state(&loaded), terminal);
    save::save(&path, &loaded).unwrap();
    assert_eq!(state(&save::load(&path).unwrap()), terminal);
}

#[test]
fn restructuring_recovery_resets_grace_and_rescue_reconciles_without_cash_injection() {
    let mut g = fresh();
    g.borrow(1_200_000).unwrap();
    cash(&mut g, 1);
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY));
    assert!(g.advance(RULES.distress_grace_days as f64 * DAY));
    assert!(g.can_reschedule());
    let debt = g.financing.debt;
    let fee = (debt * RULES.reschedule_fee_bps + 9999) / 10000;
    // Cover the fee but not overdue principal: rescheduling can actually rescue
    // this airline by deferring arrears, rather than manufacturing cash.
    cash(&mut g, fee + 1);
    assert!(g.distressed());
    let start = g.cash();
    g.reschedule_loan().unwrap();
    assert_eq!(g.cash(), start - fee);
    assert_eq!(g.financing.debt, debt);
    assert_eq!(g.financing.overdue, 0);
    assert_eq!(
        g.financing.installment,
        (debt + RULES.reschedule_term_periods - 1) / RULES.reschedule_term_periods
    );
    assert_eq!(
        g.financing.next_due,
        g.clock.elapsed().as_secs() + RULES.repayment_period_days * 86400
    );
    assert!(!g.distressed());
    assert_eq!(g.resolution.deadline, None);
    assert_eq!(g.resolution.distress_since, None);
    g.validate_financing().unwrap();
    g.validate_history().unwrap();
    let entry = g.history.as_ref().unwrap().financing.last().unwrap();
    assert_eq!(
        (entry.cash, entry.principal, entry.interest),
        (-fee, 0, fee)
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("used-rescue.json");
    save::save(&path, &g).unwrap();
    let restored = save::load(&path).unwrap();
    assert_eq!(state(&restored), state(&g));
    g = restored;
    let before = state(&g);
    assert!(g.reschedule_loan().is_err());
    assert_eq!(state(&g), before);
    cash(&mut g, -1);
    let now = g.clock.elapsed().as_secs();
    assert!(g.advance(RULES.distress_grace_days as f64 * DAY));
    assert_eq!(g.resolution.distress_since, Some(now));
    assert!(g.resolution.deadline.is_some());
    assert!(!g.can_reschedule());
}

#[test]
fn restructuring_sale_recovers_and_failed_rescue_keeps_original_deadline() {
    let mut g = fresh();
    g.buy_aircraft("atr72_600").unwrap();
    cash(&mut g, -1);
    g.clock.set_running(true);
    assert!(g.advance(RULES.distress_grace_days as f64 * DAY));
    g.sell_aircraft(1).unwrap();
    assert!(!g.distressed());
    assert_eq!(g.resolution.deadline, None);
    assert!(g.advance((RULES.restructuring_days + 1) as f64 * DAY));
    assert!(!g.insolvent());

    let mut g = fresh();
    g.borrow(120_000).unwrap();
    cash(&mut g, -1);
    g.clock.set_running(true);
    assert!(g.advance(RULES.distress_grace_days as f64 * DAY));
    let deadline = g.resolution.deadline;
    g.reschedule_loan().unwrap();
    assert!(g.distressed());
    assert_eq!(g.resolution.deadline, deadline);
    assert!(g.advance(RULES.restructuring_days as f64 * DAY));
    assert!(g.insolvent());
}

#[test]
fn restructuring_ai_exit_and_player_terminal_are_deterministic_across_steps_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("exit.json");
    for player in [false, true] {
        let mut initial = Game::default();
        initial.found("Exit audit", "DE", "EDDF").unwrap();
        if player {
            cash(&mut initial, -1);
        } else {
            cash(&mut initial.competitors[0].game, -100_000_000);
        }
        initial.clock.set_running(true);
        let days = RULES.distress_grace_days + RULES.restructuring_days + 10;
        let mut expected = initial.clone();
        assert!(expected.advance(days as f64 * DAY));
        if player {
            assert!(expected.insolvent());
        } else {
            assert_eq!(expected.competitors.len(), 1);
            assert!(
                expected
                    .competitors
                    .iter()
                    .all(|c| c.game.airline.as_ref().unwrap().name != "Regional Wings")
            );
            // No removed routes remain in the market or flight snapshot.
            assert_eq!(
                expected.market_routes().len(),
                expected.routes.len() + expected.competitors[0].game.routes.len()
            );
            expected.validate_competitors().unwrap();
            expected.validate_history().unwrap();
        }
        for speed in [1, 4, 16] {
            for step in [DAY, DAY / 7.0] {
                let mut g = initial.clone();
                g.clock.set_speed(speed);
                // Integer-second partitions avoid floating-point fixture drift.
                let step = step.floor();
                let mut remaining = days * 86400;
                let mut saved = false;
                while remaining > 0 {
                    let dt = remaining.min(step as u64);
                    assert!(g.advance(dt as f64 / f64::from(speed)));
                    remaining -= dt;
                    if !saved
                        && g.clock.elapsed().as_secs() >= (RULES.distress_grace_days + 1) * 86400
                    {
                        save::save(&path, &g).unwrap();
                        g = save::load(&path).unwrap();
                        saved = true;
                    }
                }
                g.clock.set_speed(1);
                assert_eq!(
                    state(&g),
                    state(&expected),
                    "player={player} speed={speed} step={step}"
                );
                save::save(&path, &g).unwrap();
                assert_eq!(state(&save::load(&path).unwrap()), state(&g));
            }
        }
    }
}

#[test]
fn restructuring_v13_migration_starts_fresh_grace_without_replaying_costs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v13.json");
    let mut g = fresh();
    cash(&mut g, -10);
    g.clock =
        crate::clock::Clock::from_state(std::time::Duration::from_secs(200 * 86400), false, 1)
            .unwrap();
    save::save(&path, &g).unwrap();
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(original["version"], 29);
    let mut old = original.clone();
    old["version"] = serde_json::json!(13);
    old["state"].as_object_mut().unwrap().remove("resolution");
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let mut restored = save::load(&path).unwrap();
    assert_eq!(restored.fuel_market.started_at, 200 * 86400);
    g.fuel_market = restored.fuel_market.clone();
    g.migrate_tax().unwrap();
    g.marketing.settled_at = g.clock.elapsed().as_secs();
    g.migrate_insurance_fixture();
    assert_eq!(state(&restored), state(&g));
    restored.clock.set_running(true);
    assert!(restored.advance(1.0));
    assert_eq!(restored.resolution.distress_since, Some(200 * 86400));
    assert_eq!(restored.cash(), -10);
    for resolution in [
        serde_json::json!({"distress_since":null,"deadline":1,"rescheduled":false,"insolvent_at":null}),
        serde_json::json!({"distress_since":999999999,"deadline":null,"rescheduled":false,"insolvent_at":null}),
    ] {
        let mut bad = original.clone();
        bad["state"]["resolution"] = resolution;
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(save::load(&path).is_err());
    }
}

#[test]
fn restructuring_recovery_before_grace_and_at_deadline_wins() {
    let mut g = fresh();
    cash(&mut g, -1);
    g.clock.set_running(true);
    assert!(g.advance(RULES.distress_grace_days as f64 * DAY - 1.0));
    g.borrow(100).unwrap();
    assert_eq!(g.resolution.distress_since, None);
    assert_eq!(g.resolution.deadline, None);
    assert!(g.advance(1.0));
    assert!(!g.insolvent());
    assert_eq!(g.resolution.deadline, None);

    // Simulate a cash-settlement event exactly at the deadline. Final settled
    // solvency, rather than the opening distressed balance, decides the result.
    let mut g = fresh();
    cash(&mut g, -1);
    g.clock.set_running(true);
    assert!(g.advance(RULES.distress_grace_days as f64 * DAY));
    let end = g.resolution.deadline.unwrap();
    g.clock =
        crate::clock::Clock::from_state(std::time::Duration::from_secs(end), true, 1).unwrap();
    cash(&mut g, 0);
    g.apply_financing().unwrap();
    assert!(!g.insolvent());
    assert_eq!(g.resolution.distress_since, None);
    assert_eq!(g.resolution.deadline, None);
    g.validate_financing().unwrap();
}

#[test]
fn restructuring_ai_uses_the_same_one_time_paid_reschedule() {
    let mut world = Game::default();
    world.found("AI rescue audit", "DE", "EDDF").unwrap();
    world.competitors.truncate(1);
    let mut broken = fresh();
    broken.borrow(120_000).unwrap();
    cash(&mut broken, -1);
    world.competitors[0].game = Box::new(broken);
    world.clock.set_running(true);
    assert!(world.advance(35.0 * DAY));
    let g = &world.competitors[0].game;
    assert!(g.resolution.rescheduled);
    assert!(!g.can_reschedule());
    assert!(g.distressed());
    assert_eq!(
        g.resolution.deadline,
        Some((RULES.distress_grace_days + RULES.restructuring_days) * 86400)
    );
    let fee = (g.financing.debt * RULES.reschedule_fee_bps + 9999) / 10000;
    assert!(
        g.history
            .as_ref()
            .unwrap()
            .financing
            .iter()
            .any(|e| e.time == 35 * 86400
                && e.cash == -fee
                && e.principal == 0
                && e.interest == fee)
    );
    world.validate_competitors().unwrap();
    assert!(world.advance(100.0 * DAY));
    assert!(world.competitors.is_empty());
}
