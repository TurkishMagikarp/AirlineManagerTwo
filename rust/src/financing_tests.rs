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
