use crate::{fuel_contracts::DISABLE_AI, game::Game, save};
const DAY: u64 = 86400;
fn game() -> Game {
    let mut g = Game::default();
    g.found("Hedge Air", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 90).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("contract.json");
    save::save(&p, g).unwrap();
    save::load(&p).unwrap()
}
fn cash(g: &mut Game, amount: i64) {
    let change = g.cash() - amount;
    g.history
        .as_mut()
        .unwrap()
        .purchases
        .push(crate::history::Purchase {
            time: g.clock.elapsed().as_secs(),
            amount: change,
        });
    g.airline.as_mut().unwrap().capital_eur = amount;
}
#[test]
fn contracts_boundary_blend_validation_and_transactional_management() {
    let mut g = game();
    g.competitors.clear();
    let before = state(&g);
    assert!(g.start_fuel_contract(10, 3).is_err());
    assert!(g.start_fuel_contract(25, 4).is_err());
    assert_eq!(state(&g), before);
    g.start_fuel_contract(50, 3).unwrap();
    assert_eq!(g.fuel_contract.as_ref().unwrap().locked_cents, 82);
    let before = state(&g);
    assert!(g.start_fuel_contract(25, 6).is_err());
    assert_eq!(state(&g), before);
    let c = g.fuel_contract.clone().unwrap();
    let supply = g.market_routes();
    // Test both directions of price protection using real weekly market values.
    let mut savings = false;
    let mut extra = false;
    for departure in [0, 7 * DAY, c.ends_at - 1, c.ends_at, c.ends_at + 1] {
        let mut f =
            crate::operations::departure_quote(&g.routes[0], "atr72_600", 0, &supply).unwrap();
        let market = g.fuel_market.price_at(departure);
        f.totals =
            crate::economy::quote_at_time(&g.routes[0], "atr72_600", &supply, market, departure)
                .unwrap();
        let original = f.totals.fuel;
        g.apply_fuel_contract(&mut f, "atr72_600", departure)
            .unwrap();
        if departure < c.ends_at {
            let expected = crate::economy::fuel_bill(
                &g.routes[0],
                "atr72_600",
                50 * market + 50 * c.locked_cents,
            )
            .unwrap();
            assert_eq!(f.totals.fuel, expected);
            assert_eq!(f.totals.fuel_contract_effect, expected - original);
            savings |= expected < original;
            extra |= expected > original;
        } else {
            assert_eq!(f.totals.fuel, original);
            assert_eq!(f.totals.fuel_contract_effect, 0);
        }
    }
    assert!(savings && extra);
    assert!(g.advance((c.ends_at - 1) as f64));
    assert!(g.active_fuel_contract().is_some());
    assert!(g.advance(1.0));
    assert!(g.active_fuel_contract().is_none());
    g.start_fuel_contract(75, 12).unwrap();
    assert_eq!(state(&restart(&g)), state(&g));
    g.fuel_contract = None;
    cash(&mut g, -1);
    let before = state(&g);
    assert!(g.start_fuel_contract(25, 3).is_err());
    assert_eq!(state(&g), before);
    cash(&mut g, 1_000_000);
    g.resolution.deadline = Some(g.clock.elapsed().as_secs() + DAY);
    assert!(g.start_fuel_contract(25, 3).is_err());
}
#[test]
fn contracts_steps_pause_speeds_expiry_departures_and_restart() {
    let mut initial = game();
    initial.competitors.clear();
    // Start while airborne, mid-week: the existing snapshot must stay unchanged.
    assert!(initial.advance(60.0));
    let frozen = initial.routes[0].plan.as_ref().unwrap().airborne.clone();
    initial.clock.set_running(false);
    initial.start_fuel_contract(75, 3).unwrap();
    assert!(initial.advance(123456.0));
    assert_eq!(initial.routes[0].plan.as_ref().unwrap().airborne, frozen);
    initial.clock.set_running(true);
    let seconds = 100 * DAY;
    let mut expected = initial.clone();
    assert!(expected.advance(seconds as f64));
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        let mut remaining = seconds;
        let mut saved = false;
        while remaining > 0 {
            let step = remaining.min(17777);
            assert!(g.advance(step as f64 / speed as f64));
            remaining -= step;
            if !saved && remaining < seconds / 2 {
                g = restart(&g);
                saved = true;
            }
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected), "speed={speed}");
    }
    // An actual scheduled departure exactly at expiry must be full market.
    let mut g = game();
    g.competitors.clear();
    g.stop_schedule(1).unwrap();
    g.start_fuel_contract(50, 3).unwrap();
    let end = g.fuel_contract.as_ref().unwrap().ends_at;
    g.schedule(1, (end / 60) as u32, 120).unwrap();
    assert!(g.advance(end as f64));
    let f = g.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .as_ref()
        .unwrap();
    assert_eq!(f.totals.fuel_contract_effect, 0);
    assert_eq!(
        f.landing
            - crate::operations::timings(&g.routes[0], "atr72_600")
                .unwrap()
                .0,
        end
    );
}
#[test]
fn contracts_migration_corruption_and_ai_insolvent_exit() {
    let mut g = game();
    g.start_fuel_contract(25, 6).unwrap();
    g.competitors[0].game.start_fuel_contract(50, 12).unwrap();
    assert!(g.advance(300.0));
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("migration.json");
    save::save(&p, &g).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    let mut old = original.clone();
    old["version"] = 20.into();
    crate::fuel_contracts::migrate(&mut old["state"]);
    std::fs::write(&p, serde_json::to_vec(&old).unwrap()).unwrap();
    let migrated = save::load(&p).unwrap();
    assert!(migrated.fuel_contract.is_none());
    assert!(
        migrated
            .competitors
            .iter()
            .all(|c| c.game.fuel_contract.is_none())
    );
    assert_eq!(migrated.finance, g.finance);
    assert_eq!(
        migrated.routes[0].plan.as_ref().unwrap().airborne,
        g.routes[0].plan.as_ref().unwrap().airborne
    );
    for field in ["locked_cents", "ends_at", "share_percent"] {
        let mut bad = original.clone();
        bad["state"]["fuel_contract"][field] = 999.into();
        std::fs::write(&p, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(save::load(&p).is_err());
    }
    cash(&mut g.competitors[0].game, -100_000_000);
    let mut daily = g.clone();
    let days = crate::financing::RULES.distress_grace_days
        + crate::financing::RULES.restructuring_days
        + 10;
    assert!(g.advance((days * DAY) as f64));
    for _ in 0..days {
        assert!(daily.advance(DAY as f64));
    }
    assert_eq!(g.competitors.len(), 1);
    assert_eq!(state(&g), state(&daily));
    assert_eq!(state(&restart(&g)), state(&g));
}
#[test]
fn contracts_ten_year_ai_comparison() {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            DISABLE_AI.with(|v| v.set(false));
        }
    }
    let _reset = Reset;
    let mut hedged = game();
    let mut baseline = hedged.clone();
    let mut observed = false;
    for year in 1..=10 {
        for _ in 0..365 {
            DISABLE_AI.with(|v| v.set(false));
            assert!(hedged.advance(DAY as f64));
            DISABLE_AI.with(|v| v.set(true));
            assert!(baseline.advance(DAY as f64));
            for g in [&hedged, &baseline] {
                assert_eq!(g.competitors.len(), 2);
                assert!(
                    g.competitors
                        .iter()
                        .all(|c| !c.game.distressed() && c.game.resolution.deadline.is_none())
                );
            }
            observed |= hedged
                .competitors
                .iter()
                .any(|c| c.game.active_fuel_contract().is_some());
        }
        for (h, b) in hedged.competitors.iter().zip(&baseline.competitors) {
            let difference = h.game.cash().abs_diff(b.game.cash());
            // No material balance drift: at most 5% of the control's cash.
            assert!(
                difference <= (b.game.cash().unsigned_abs() / 20).max(100_000),
                "year={year} hedged={} baseline={}",
                h.game.cash(),
                b.game.cash()
            );
            println!(
                "CONTRACT_AI year={year} {} hedged={} baseline={} effect={}",
                h.game.airline.as_ref().unwrap().name,
                h.game.cash(),
                b.game.cash(),
                h.game.finance.totals.fuel_contract_effect
            );
        }
        hedged = restart(&hedged);
        baseline = restart(&baseline);
    }
    assert!(observed);
}
