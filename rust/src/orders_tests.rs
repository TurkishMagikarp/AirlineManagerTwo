use crate::{
    aircraft::{self, AircraftStatus},
    game::Game,
    orders::RULES,
    save,
};
const DAY: u64 = 86400;
fn game() -> Game {
    let mut g = Game::default();
    g.found("Factory Audit", "DE", "EDDF").unwrap();
    g.competitors.clear();
    cash(&mut g, 1_000_000_000);
    g
}
fn cash(g: &mut Game, amount: i64) {
    g.history.as_mut().unwrap().opening_balance += amount - g.cash();
    g.airline.as_mut().unwrap().capital_eur = amount;
}
fn state(g: &Game) -> serde_json::Value {
    let mut v = serde_json::to_value(g).unwrap();
    v["clock"]["speed_index"] = 0.into();
    v
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orders.json");
    save::save(&path, g).unwrap();
    save::load(&path).unwrap()
}
fn advance(g: &mut Game, seconds: u64, small: bool) {
    let speed = f64::from(g.clock.speed());
    let mut remaining = seconds;
    while remaining > 0 {
        let step = if small {
            remaining.min(3600)
        } else {
            remaining
        };
        assert!(g.advance(step as f64 / speed));
        remaining -= step;
    }
}
#[test]
fn orders_fractional_prices_round_down_once_and_balance_is_exact_remainder() {
    for (price, discount, share, expected) in [
        (101, 750, 2000, (93, 18, 75)),
        (103, 500, 3333, (97, 32, 65)),
        (1, 0, 2000, (1, 1, 0)),
        (
            999_999_999,
            999,
            1999,
            (900_099_999, 179_929_989, 720_170_010),
        ),
    ] {
        let actual = crate::orders::price_terms(price, discount, share);
        assert_eq!(actual, expected);
        assert_eq!(actual.1 + actual.2, actual.0);
    }
}
#[test]
fn orders_tiers_prices_rounding_deposits_and_failures_are_exact() {
    let mut g = game();
    for m in aircraft::models().unwrap() {
        g.clock = crate::clock::Clock::from_state(
            std::time::Duration::from_secs(m.entry_day * 86400),
            false,
            1,
        )
        .unwrap();
        let mut previous = m.price_eur;
        for quantity in 1..=RULES.max_open_orders as u32 {
            let q = g.quote_order(&m.id, quantity).unwrap();
            let discount = RULES
                .discount_tiers
                .iter()
                .filter(|t| t.0 <= quantity)
                .last()
                .unwrap()
                .1;
            let expected = (i128::from(m.price_eur) * i128::from(10000 - discount) / 10000) as i64;
            assert_eq!(q.unit_price, expected);
            assert!(q.unit_price <= previous && q.unit_price < m.price_eur);
            previous = q.unit_price;
            let expected_deposit =
                (i128::from(expected) * i128::from(RULES.deposit_bps) / 10000).max(1) as i64;
            assert_eq!(q.deposit_per_aircraft, expected_deposit);
            assert_eq!(q.balance + q.deposit_per_aircraft, expected);
            assert_eq!(q.delivery_dates.len(), quantity as usize);
            assert!(
                q.delivery_dates
                    .windows(2)
                    .all(|w| w[1] - w[0] == RULES.stagger_days * DAY)
            );
        }
    }
    g.clock = crate::clock::Clock::default();
    assert!(
        g.quote_order("atr72_600", 1).unwrap().delivery_dates[0]
            < g.quote_order("a330_300", 1).unwrap().delivery_dates[0]
    );
    assert!(
        g.quote_order("atr72_600", 1).unwrap().delivery_dates[0]
            < g.quote_order("atr72_600f", 1).unwrap().delivery_dates[0]
    );
    for (model, qty) in [
        ("missing", 1),
        ("atr72_600", 0),
        ("atr72_600", RULES.max_open_orders as u32 + 1),
    ] {
        let before = state(&g);
        assert!(g.place_order(model, qty).is_err());
        assert_eq!(state(&g), before);
    }
    let q = g.quote_order("atr72_600", 3).unwrap();
    let opening = g.cash();
    g.place_order("atr72_600", 3).unwrap();
    assert_eq!(g.cash(), opening - 3 * q.deposit_per_aircraft);
    assert!(g.fleet.is_empty());
    assert_eq!(
        g.history.as_ref().unwrap().purchases.last().unwrap().amount,
        3 * q.deposit_per_aircraft
    );
    assert_eq!(
        g.orders.iter().map(|o| o.id).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    g.validate_history().unwrap();
    cash(&mut g, 0);
    let before = state(&g);
    assert!(g.place_order("atr72_600", 1).is_err());
    assert_eq!(state(&g), before);
    let mut full = game();
    full.place_order("atr72_600", RULES.max_open_orders as u32)
        .unwrap();
    let before = state(&full);
    assert!(full.place_order("atr72_600", 1).is_err());
    assert_eq!(state(&full), before);
    assert!(Game::default().place_order("atr72_600", 1).is_err());
}
#[test]
fn orders_staggered_deliveries_are_new_home_based_and_ignore_slot_admission() {
    let mut g = game();
    // Slot admission constrains schedules, never purchases or parked deliveries.
    g.external_slots.insert("EDDF".into(), 1_000_000);
    let q = g.quote_order("atr72_600", 3).unwrap();
    g.place_order("atr72_600", 3).unwrap();
    g.clock.set_running(true);
    advance(&mut g, q.delivery_dates[0] - 1, false);
    assert!(g.fleet.is_empty());
    advance(&mut g, 1, false);
    assert_eq!(g.fleet.len(), 1);
    assert_eq!(g.orders.len(), 2);
    for (index, &at) in q.delivery_dates.iter().enumerate() {
        if index > 0 {
            let left = at - g.clock.elapsed().as_secs();
            advance(&mut g, left, false);
        }
        assert_eq!(g.fleet.len(), index + 1);
        let p = &g.fleet[index];
        assert_eq!(p.id, index as u32 + 1);
        assert_eq!(p.location_icao, "EDDF");
        assert_eq!(p.status, AircraftStatus::Parked);
        assert_eq!(p.acquired_at, Some(at));
        assert_eq!(p.purchase_price_eur, q.unit_price);
        assert!(p.lease.is_none() && p.maintenance.is_none());
        assert_eq!(p.wear.flight_seconds, 0);
        assert_eq!(
            g.history.as_ref().unwrap().purchases.last().unwrap().amount,
            q.balance
        );
        assert_eq!(
            g.history.as_ref().unwrap().purchases.last().unwrap().time,
            at
        );
        g.validate_fleet().unwrap();
        g.validate_history().unwrap();
        g.validate_staff().unwrap();
        assert!(g.staff.employees.is_empty());
    }
    assert!(g.orders.is_empty());
    let investment: i64 = g
        .history
        .as_ref()
        .unwrap()
        .purchases
        .iter()
        .map(|p| p.amount)
        .sum();
    assert_eq!(investment, 3 * q.unit_price);
    assert_eq!(state(&restart(&g)), state(&g));
}
#[test]
fn orders_held_delivery_retries_then_expires_and_manual_cancel_loses_deposit() {
    let mut initial = game();
    initial.place_order("atr72_600", 1).unwrap();
    let due = initial.orders[0].delivery_at;
    let deposit = initial.orders[0].deposit;
    cash(&mut initial, 0);
    initial.clock.set_running(true);
    advance(&mut initial, due, false);
    assert!(initial.fleet.is_empty());
    assert_eq!(initial.orders[0].held_since, Some(due));
    assert!(
        initial
            .attention_snapshot()
            .iter()
            .any(|e| e.kind == "delivery" && e.page == "fleet")
    );
    initial.validate_orders().unwrap();
    initial.validate_history().unwrap();
    assert_eq!(state(&restart(&initial)), state(&initial));
    let mut recover = initial.clone();
    let balance = recover.orders[0].unit_price - deposit;
    cash(&mut recover, balance);
    advance(&mut recover, DAY - 1, true);
    assert!(recover.fleet.is_empty());
    advance(&mut recover, 1, false);
    assert_eq!(recover.fleet.len(), 1);
    assert!(recover.orders.is_empty());
    assert_eq!(recover.cash(), 0);
    recover.validate_history().unwrap();
    let mut expire = initial.clone();
    advance(&mut expire, RULES.grace_days * DAY - 1, false);
    assert_eq!(expire.orders.len(), 1);
    advance(&mut expire, 1, false);
    assert!(expire.orders.is_empty() && expire.fleet.is_empty());
    assert_eq!(
        expire
            .history
            .as_ref()
            .unwrap()
            .purchases
            .iter()
            .map(|p| p.amount)
            .sum::<i64>(),
        deposit
    );
    expire.validate_history().unwrap();
    for mut cancel in [initial, game()] {
        if cancel.orders.is_empty() {
            cancel.place_order("atr72_600", 1).unwrap();
        }
        let amount = cancel.cash();
        let purchases = cancel.history.as_ref().unwrap().purchases.clone();
        let id = cancel.orders[0].id;
        cancel.cancel_order(id).unwrap();
        assert!(cancel.orders.is_empty());
        assert_eq!(cancel.cash(), amount);
        assert_eq!(cancel.history.as_ref().unwrap().purchases, purchases);
        let before = state(&cancel);
        assert!(cancel.cancel_order(id).is_err());
        assert_eq!(state(&cancel), before);
        cancel.validate_history().unwrap();
    }
}
#[test]
fn orders_pause_speeds_steps_jumps_and_restart_preserve_delivery_hold_and_cancel() {
    let mut initial = game();
    initial.place_order("atr72_600", 3).unwrap();
    let due = initial.orders[0].delivery_at;
    let mut expected = None;
    for speed in [1, 4, 16] {
        for small in [false, true] {
            let mut g = initial.clone();
            g.clock.set_speed(speed);
            let before = state(&g);
            assert!(g.advance(1_000_000.0));
            assert_eq!(state(&g), before);
            g = restart(&g);
            g.clock.set_running(true);
            advance(&mut g, due, small);
            assert_eq!(g.fleet.len(), 1);
            g = restart(&g);
            g.cancel_order(3).unwrap();
            cash(&mut g, 0);
            advance(&mut g, RULES.stagger_days * DAY, small);
            assert_eq!(g.orders[0].held_since, Some(due + RULES.stagger_days * DAY));
            g = restart(&g);
            advance(&mut g, RULES.grace_days * DAY, small);
            assert!(g.orders.is_empty());
            g.validate_history().unwrap();
            if let Some(reference) = &expected {
                assert_eq!(&state(&g), reference, "speed={speed} small={small}");
            } else {
                expected = Some(state(&g));
            }
        }
    }
    let mut jumped = initial.clone();
    let first = jumped.jump(due as f64).unwrap();
    assert_eq!(first.advanced_seconds, due);
    assert_eq!(jumped.fleet.len(), 1);
    jumped.cancel_order(3).unwrap();
    cash(&mut jumped, 0);
    let held = jumped.jump((2 * RULES.stagger_days * DAY) as f64).unwrap();
    assert_eq!(held.advanced_seconds, RULES.stagger_days * DAY);
    assert!(!held.reason.is_empty());
    assert!(jumped.orders[0].held_since.is_some());
    jumped = restart(&jumped);
    let cancelled = jumped.jump((RULES.grace_days * DAY) as f64).unwrap();
    assert_eq!(cancelled.advanced_seconds, RULES.grace_days * DAY);
    assert!(jumped.orders.is_empty());
}
#[test]
fn orders_midday_jump_stops_at_the_exact_unpaid_delivery_boundary() {
    let mut g = game();
    g.place_order("atr72_600", 1).unwrap();
    cash(&mut g, 0);
    g.clock.set_running(true);
    advance(&mut g, DAY / 2, false);
    let quote = g.quote_order("atr72_600", 1).unwrap();
    assert_eq!(
        quote.delivery_dates[0],
        (1 + RULES.lead_days["atr72_600"]) * DAY
    );
    g.clock.set_running(false);
    let due = g.orders[0].delivery_at;
    let jump = g.jump((due + DAY) as f64).unwrap();
    assert_eq!(jump.advanced_seconds, due - DAY / 2);
    assert_eq!(g.clock.elapsed().as_secs(), due);
    assert!(!jump.reason.is_empty());
    assert_eq!(g.orders[0].held_since, Some(due));
    assert!(!g.clock.is_running());
    assert_eq!(state(&restart(&g)), state(&g));
}
#[test]
fn orders_v38_migration_is_empty_for_every_owner_and_preserves_money() {
    let mut g = Game::default();
    g.found("Old Factory Audit", "DE", "EDDF").unwrap();
    let before = state(&g);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old-orders.json");
    save::save(&path, &g).unwrap();
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    old["version"] = 38.into();
    let state = old["state"].as_object_mut().unwrap();
    state.remove("orders");
    state.remove("next_order_id");
    for c in state["competitors"].as_array_mut().unwrap() {
        let owner = c["game"].as_object_mut().unwrap();
        owner.remove("orders");
        owner.remove("next_order_id");
    }
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let loaded = save::load(&path).unwrap();
    assert!(loaded.orders.is_empty());
    assert_eq!(loaded.next_order_id, 1);
    assert!(
        loaded
            .competitors
            .iter()
            .all(|c| c.game.orders.is_empty() && c.game.next_order_id == 1)
    );
    assert_eq!(before, crate::orders_tests::state(&loaded));
    loaded.validate_history().unwrap();
    loaded.validate_competitors().unwrap();
}
#[test]
fn orders_corrupt_positions_are_rejected_without_losing_the_save() {
    let mut g = game();
    g.place_order("atr72_600", 1).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid-order.json");
    save::save(&path, &g).unwrap();
    for corruption in 0..7 {
        let mut bad = g.clone();
        match corruption {
            0 => bad.orders[0].id = 0,
            1 => bad.orders[0].deposit = bad.orders[0].unit_price,
            2 => bad.orders[0].delivery_at = 1,
            3 => bad.orders[0].held_since = Some(0),
            4 => bad.orders[0].model_id = "unknown".into(),
            5 => bad.orders.push(bad.orders[0].clone()),
            _ => bad.next_order_id = 0,
        }
        assert!(save::save(&path, &bad).is_err(), "corruption={corruption}");
        assert_eq!(state(&save::load(&path).unwrap()), state(&g));
    }
}
