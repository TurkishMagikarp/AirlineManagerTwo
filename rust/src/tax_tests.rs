use crate::{clock::Clock, demand::YEAR_SECONDS as YEAR, game::Game, save};
use std::time::Duration;

fn game() -> Game {
    let mut g = Game::default();
    g.found("Tax Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
// Synthetic settled accrual at a given instant, with a fully reconciled ledger.
fn profit(g: &mut Game, time: u64, amount: i64) {
    g.clock = Clock::from_state(Duration::from_secs(time), true, 1).unwrap();
    let h = g.history.as_mut().unwrap();
    if amount < 0 {
        h.maintenance.push(crate::history::Purchase {
            time,
            amount: -amount,
        });
    } else {
        g.next_route_id = 2;
        let t = crate::economy::Totals {
            flights: 1,
            passengers: 1,
            seats: 1,
            revenue: amount,
            ..Default::default()
        };
        g.finance.totals.add(&t, 1).unwrap();
        g.finance.recent.push(crate::economy::FlightResult {
            route_id: 1,
            landing: time,
            origin: "EDDF".into(),
            destination: "EDDM".into(),
            ticket_eur: amount,
            totals: t.clone(),
        });
        h.record(crate::history::Batch {
            route_id: 1,
            first: time,
            interval: 1,
            count: 1,
            origin: "EDDF".into(),
            destination: "EDDM".into(),
            ticket_eur: amount,
            totals: t,
        });
    }
    g.airline.as_mut().unwrap().capital_eur += amount;
}
#[test]
fn tax_profit_loss_carry_rounding_country_default_and_distress() {
    assert_eq!(crate::tax::rate("ZZ"), 2500);
    assert_eq!(crate::tax::rate("TR"), 2500);
    let mut g = game();
    profit(&mut g, 1, -1000);
    assert!(g.advance((YEAR - 1) as f64));
    assert_eq!(g.tax_loss(), 1000);
    assert_eq!(g.history.as_ref().unwrap().tax_through(u64::MAX), 0);
    profit(&mut g, YEAR + 1, 2005);
    assert!(g.advance((YEAR - 1) as f64));
    let a = g.history.as_ref().unwrap().taxes.last().unwrap();
    assert_eq!((a.profit, a.amount, a.loss), (2005, 302, 0));
    g.validate_tax().unwrap();
    g.validate_history().unwrap();
    // Profit exists but cash was spent on assets: use the normal distress path.
    let mut g = game();
    profit(&mut g, 1, 1000);
    let shift = g.cash() - 100;
    g.airline.as_mut().unwrap().capital_eur -= shift;
    g.history.as_mut().unwrap().opening_balance -= shift;
    assert!(g.advance((YEAR - 1) as f64));
    assert_eq!(g.cash(), -200);
    assert!(g.distressed());
    assert_eq!(g.resolution.distress_since, Some(YEAR));
    g.validate_history().unwrap();
}
#[test]
fn tax_large_steps_pause_speeds_restart_and_old_migration() {
    let mut base = game();
    base.buy_aircraft("atr72_600").unwrap();
    base.staff_up();
    base.create_route("EDDF", "EDDM", 1).unwrap();
    base.set_ticket_price(1, 110).unwrap();
    base.schedule(1, 1, 120).unwrap();
    let mut fast = base.clone();
    assert!(fast.advance((YEAR * 2 + 123) as f64));
    assert_eq!(fast.history.as_ref().unwrap().taxes.len(), 2);
    assert!(fast.history.as_ref().unwrap().tax_through(u64::MAX) > 0);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tax.json");
    for speed in [1, 4, 16] {
        let mut g = base.clone();
        g.clock.set_speed(speed);
        g.clock.set_running(false);
        let paused = state(&g);
        assert!(g.advance(YEAR as f64));
        assert_eq!(state(&g), paused);
        g.clock.set_running(true);
        for day in 0..730 {
            assert!(g.advance(86400.0 / f64::from(speed)));
            if day == 180 || day == 500 {
                save::save(&path, &g).unwrap();
                g = save::load(&path).unwrap();
            }
        }
        assert!(g.advance(123.0 / f64::from(speed)));
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&fast));
    }
    assert!(base.advance((YEAR / 2) as f64));
    save::save(&path, &base).unwrap();
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    old["version"] = 18.into();
    old["state"].as_object_mut().unwrap().remove("tax");
    old["state"]["history"]
        .as_object_mut()
        .unwrap()
        .remove("taxes");
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let mut migrated = save::load(&path).unwrap();
    assert_eq!(migrated.tax_loss(), 0);
    assert_eq!(migrated.tax.since, YEAR / 2);
    let opening = migrated
        .history
        .as_ref()
        .unwrap()
        .pretax_profit(u64::MAX)
        .unwrap();
    assert_eq!(migrated.tax.opening_profit, opening);
    assert!(migrated.advance((YEAR / 2) as f64));
    let a = migrated.history.as_ref().unwrap().taxes.last().unwrap();
    assert_eq!(
        a.profit,
        migrated
            .history
            .as_ref()
            .unwrap()
            .pretax_profit(YEAR)
            .unwrap()
            - opening
    );
    save::save(&path, &migrated).unwrap();
    let valid: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for field in ["amount", "loss", "time", "profit"] {
        let mut bad = valid.clone();
        bad["state"]["history"]["taxes"][0][field] = (-1).into();
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(save::load(&path).is_err(), "accepted bad {field}");
    }
}
#[test]
fn tax_ai_uses_shared_assessment_and_migrates_from_load_point() {
    let mut g = game();
    g.competitors = crate::competition::initial_competitors().unwrap();
    assert!(g.advance((YEAR / 2) as f64));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ai-tax.json");
    save::save(&path, &g).unwrap();
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    old["version"] = 18.into();
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let mut g = save::load(&path).unwrap();
    assert!(g.advance((YEAR / 2) as f64));
    assert_eq!(g.competitors.len(), 2);
    for c in &g.competitors {
        let h = c.game.history.as_ref().unwrap();
        assert_eq!(c.game.tax.since, YEAR / 2);
        let a = h.taxes.last().unwrap();
        assert_eq!(a.rate_bps, 3000);
        assert_eq!(a.amount, (a.profit.max(0) * 3000 + 5000) / 10_000);
        assert!(a.amount > 0);
        assert!(!c.game.distressed());
        c.game.validate_tax().unwrap();
    }
}

#[test]
fn tax_actions_after_closing_at_the_same_second_belong_to_next_year() {
    let mut g = game();
    profit(&mut g, 1, 1000);
    assert!(g.advance((YEAR - 1) as f64));
    assert_eq!(g.history.as_ref().unwrap().taxes[0].amount, 300);
    profit(&mut g, YEAR, -200);
    g.validate_tax().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boundary.json");
    save::save(&path, &g).unwrap();
    let mut g = save::load(&path).unwrap();
    assert!(g.advance(YEAR as f64));
    assert_eq!(g.history.as_ref().unwrap().taxes[1].profit, -200);
    assert_eq!(g.tax_loss(), 200);
}
