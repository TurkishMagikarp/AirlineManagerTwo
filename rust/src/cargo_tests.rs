use crate::{cargo, economy, game::Game, save};

fn game() -> Game {
    let mut g = Game::default();
    g.found("Freight", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("a320_200").unwrap();
    g.staff_up();
    g.create_route("EDDF", "LTFM", 1).unwrap();
    g.set_ticket_price(1, 210).unwrap();
    g.schedule(1, 1, 360).unwrap();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("cargo.json");
    save::save(&p, g).unwrap();
    save::load(&p).unwrap()
}
fn quote(g: &Game) -> economy::Totals {
    economy::quote_at_time(
        &g.routes[0],
        "a320_200",
        &g.market_routes(),
        economy::fuel_mean(),
        g.clock.elapsed().as_secs(),
    )
    .unwrap()
}

#[test]
fn cargo_payload_prices_and_shared_spill_are_bounded() {
    let mut g = game();
    let m = crate::aircraft::model("a320_200").unwrap();
    assert_eq!(cargo::capacity(m, 180), 0);
    assert_eq!(cargo::capacity(m, 179), 100);
    assert_eq!(cargo::capacity(m, 0), i64::from(m.belly_cargo_kg));
    let solo = quote(&g);
    assert!(solo.cargo_kg > 0);
    assert!(solo.cargo_revenue < (solo.revenue - solo.cargo_revenue) / 10);
    let mut rival = g.clone();
    rival.routes[0].id = 2;
    rival.fleet[0].id = 2;
    rival.routes[0].aircraft_id = Some(2);
    let mut supply = g.market_routes();
    supply.extend(rival.market_routes());
    let a = cargo::estimate(&g.routes[0], "a320_200", &supply, Some(0)).unwrap();
    let b = cargo::estimate(&rival.routes[0], "a320_200", &supply, Some(0)).unwrap();
    assert_eq!(a, b);
    assert!(a < solo.cargo_kg);
    assert!(a + b <= solo.cargo_kg + 1);
    // Expensive freight loses all volume; the rival can receive its spill.
    g.set_cargo_rate(1, cargo::RULES.maximum_rate_cents)
        .unwrap();
    let mut supply = g.market_routes();
    supply.extend(rival.market_routes());
    assert_eq!(
        cargo::estimate(&g.routes[0], "a320_200", &supply, Some(0)).unwrap(),
        0
    );
    assert!(cargo::estimate(&rival.routes[0], "a320_200", &supply, Some(0)).unwrap() > b);
    let before = state(&g);
    assert!(g.set_cargo_rate(1, -1).is_err());
    assert!(
        g.set_cargo_rate(1, cargo::RULES.maximum_rate_cents + 1)
            .is_err()
    );
    assert_eq!(state(&g), before);
    // Free passengers fill the aircraft, leaving no payload for freight.
    g.set_ticket_price(1, 0).unwrap();
    g.routes[0].plan.as_mut().unwrap().interval_seconds = 86400;
    assert_eq!(quote(&g).passengers, 180);
    assert_eq!(quote(&g).cargo_kg, 0);
}

#[test]
fn cargo_frozen_departures_maintenance_sale_and_restart() {
    let mut g = game();
    assert!(g.advance(60.0));
    let airborne = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
    assert!(airborne.totals.cargo_kg > 0);
    g.set_cargo_rate(1, cargo::RULES.maximum_rate_cents)
        .unwrap();
    assert_eq!(
        g.routes[0].plan.as_ref().unwrap().airborne.as_ref(),
        Some(&airborne)
    );
    let before = state(&g);
    assert!(g.sell_aircraft(1).is_err());
    assert_eq!(before, state(&g));
    g.request_maintenance(1).unwrap();
    g = restart(&g);
    assert!(g.advance((airborne.landing - 60) as f64));
    assert_eq!(
        g.finance.totals.cargo_revenue,
        airborne.totals.cargo_revenue
    );
    assert_eq!(g.finance.totals.flights, 1);
    assert_eq!(
        g.fleet[0].status,
        crate::aircraft::AircraftStatus::Maintenance
    );
    assert_eq!(quote(&g).cargo_kg, 0);
    assert!(g.advance(3.0 * 86400.0));
    assert_eq!(
        g.finance.totals.cargo_revenue,
        airborne.totals.cargo_revenue
    );
    // Sell on the ground after cancelling the resumed plan; no future revenue.
    g.stop_schedule(1).unwrap();
    g.sell_aircraft(1).unwrap();
    let settled = g.finance.clone();
    assert!(g.advance(86400.0));
    assert_eq!(g.finance, settled);
    restart(&g).validate_history().unwrap();
}

#[test]
fn cargo_v21_migration_preserves_money_and_airborne_quotes() {
    let mut g = game();
    g.set_cargo_rate(1, cargo::RULES.maximum_rate_cents)
        .unwrap();
    g.competitors = crate::competition::initial_competitors().unwrap();
    for c in &mut g.competitors {
        for r in &mut c.game.routes {
            r.cargo_rate_cents = cargo::RULES.maximum_rate_cents;
        }
    }
    assert!(g.advance(22000.0));
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("old.json");
    save::save(&p, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    v["version"] = 21.into();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.remove("cargo_rate_cents");
                o.remove("cargo_kg");
                o.remove("cargo_revenue");
                for v in o.values_mut() {
                    strip(v);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    strip(v);
                }
            }
            _ => (),
        }
    }
    strip(&mut v);
    std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
    let loaded = save::load(&p).unwrap();
    assert_eq!(loaded.cash(), g.cash());
    assert_eq!(loaded.finance, g.finance);
    assert_eq!(loaded.routes[0].cargo_rate_cents, cargo::default_rate());
    assert_eq!(
        loaded.routes[0].plan.as_ref().unwrap().airborne,
        g.routes[0].plan.as_ref().unwrap().airborne
    );
    assert_eq!(loaded.finance.totals.cargo_revenue, 0);
    for (a, b) in loaded.competitors.iter().zip(&g.competitors) {
        assert_eq!(a.game.cash(), b.game.cash());
        assert_eq!(a.game.finance, b.game.finance);
        assert!(
            a.game
                .routes
                .iter()
                .all(|r| r.cargo_rate_cents == cargo::default_rate())
        );
    }
    restart(&loaded);
}

#[test]
fn cargo_speeds_steps_save_load_ai_two_years_and_tax() {
    let mut baseline = game();
    // Restore the same rivals used in normal games.
    baseline.competitors = crate::competition::initial_competitors().unwrap();
    // Keep the player solvent so the global clock reaches both tax boundaries.
    baseline.airline.as_mut().unwrap().capital_eur += 100_000_000;
    baseline.history.as_mut().unwrap().opening_balance += 100_000_000;
    let original = baseline.clone();
    let total = 2.0 * crate::demand::YEAR_SECONDS as f64;
    assert!(baseline.advance(total));
    assert!(baseline.finance.totals.cargo_revenue > 0);
    for c in &baseline.competitors {
        let t = &c.game.finance.totals;
        assert!(t.flights > 0 && t.cargo_revenue > 0);
        assert!(t.cargo_revenue < (t.revenue - t.cargo_revenue) / 10);
        assert!(
            c.game
                .routes
                .iter()
                .all(|r| r.cargo_rate_cents == cargo::default_rate())
        );
        c.game.validate_history().unwrap();
        c.game.validate_tax().unwrap();
        assert_eq!(c.game.history.as_ref().unwrap().taxes.len(), 2);
        assert!(!c.game.distressed());
        assert!(c.game.cash().abs() < 1_000_000_000);
    }
    for speed in [1, 4, 16] {
        let mut stepped = original.clone();
        stepped.clock.set_speed(speed);
        for i in 0..730 {
            assert!(stepped.advance(86400.0 / f64::from(speed)));
            if i == 370 {
                stepped = restart(&stepped);
            }
        }
        stepped.clock.set_speed(1);
        assert_eq!(state(&stepped), state(&baseline));
    }
    baseline.validate_tax().unwrap();
    restart(&baseline);
}

#[test]
fn cargo_daily_market_is_conserved_for_mixed_rates_frequencies_and_reverse_routes() {
    let mut a = game();
    a.set_ticket_price(1, 100_000).unwrap(); // payload available, passenger market unchanged
    let mut b = a.clone();
    b.routes[0].id = 2;
    b.fleet[0].id = 2;
    b.routes[0].aircraft_id = Some(2);
    let origin = b.routes[0].origin_icao.clone();
    b.routes[0].origin_icao = b.routes[0].destination_icao.clone();
    b.routes[0].destination_icao = origin;
    let base = cargo::daily_demand(&a.routes[0]).unwrap();
    for interval in [21600, 86400, 7 * 86400] {
        b.routes[0].plan.as_mut().unwrap().interval_seconds = interval;
        for rate in [0, 150, 300, 600, 10000] {
            b.routes[0].cargo_rate_cents = rate;
            let mut supply = a.market_routes();
            supply.extend(b.market_routes());
            let kg_a = cargo::estimate(&a.routes[0], "a320_200", &supply, None).unwrap();
            let kg_b = cargo::estimate(&b.routes[0], "a320_200", &supply, None).unwrap();
            assert!(kg_a as f64 * 2.0 + kg_b as f64 * 43200.0 / interval as f64 <= base);
            assert!(kg_b <= 1800);
            if rate >= 600 {
                assert_eq!(kg_b, 0);
            }
        }
    }
    b.routes[0].plan = None;
    assert_eq!(quote(&b).cargo_kg, 0);
    let mut supply = a.market_routes();
    supply.extend(b.market_routes());
    assert_eq!(
        cargo::estimate(&a.routes[0], "a320_200", &supply, None).unwrap(),
        cargo::estimate(&a.routes[0], "a320_200", &a.market_routes(), None).unwrap()
    );
}

#[test]
fn cargo_income_enters_tax_base_and_impossible_airborne_payload_is_rejected() {
    let mut g = game();
    g.advance(60.0);
    let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
    assert!(frozen.totals.cargo_revenue > 0);
    let mut corrupt = g.clone();
    corrupt.routes[0]
        .plan
        .as_mut()
        .unwrap()
        .airborne
        .as_mut()
        .unwrap()
        .totals
        .cargo_kg = 1_000_000;
    assert!(corrupt.validate_operations().is_err());
    let before = g.history.as_ref().unwrap().pretax_profit(u64::MAX).unwrap();
    assert!(g.advance((frozen.landing - 60) as f64));
    let taxed_profit = g.history.as_ref().unwrap().pretax_profit(u64::MAX).unwrap();
    assert_eq!(taxed_profit - before, frozen.totals.profit());
    let mut tickets_only = g.history.as_ref().unwrap().clone();
    for batch in &mut tickets_only.batches {
        batch.totals.revenue -= batch.totals.cargo_revenue;
        batch.totals.cargo_revenue = 0;
        batch.totals.cargo_kg = 0;
    }
    let tickets_only: crate::history::History =
        serde_json::from_value(serde_json::to_value(tickets_only).unwrap()).unwrap();
    assert_eq!(
        taxed_profit - tickets_only.pretax_profit(u64::MAX).unwrap(),
        frozen.totals.cargo_revenue
    );
    assert_eq!(
        g.finance.totals.revenue - g.finance.totals.cargo_revenue,
        g.finance.totals.passengers * g.routes[0].ticket_eur
    );
}

#[test]
fn cargo_shared_passenger_loads_match_each_owners_quote_with_colliding_local_ids() {
    let a = game();
    for model in ["a320_200", "b737_800"] {
        for business in [0, 12] {
            let mut b = a.clone();
            b.airline.as_mut().unwrap().name = "Other owner".into();
            b.fleet[0].model_id = model.into();
            b.fleet[0].business_seats = business;
            for fare in [70, 210, 100_000] {
                b.routes[0].ticket_eur = fare;
                for interval in [21600, 43200] {
                    b.routes[0].plan.as_mut().unwrap().interval_seconds = interval;
                    let mut supply = a.market_routes();
                    supply.extend(b.market_routes());
                    for time in [0, 7 * 86400, 180 * 86400] {
                        let loads = crate::demand::market_passengers(&supply, Some(time)).unwrap();
                        assert_eq!(loads.len(), supply.len());
                        for (i, o) in supply.iter().enumerate() {
                            let mut own_first = supply.clone();
                            own_first.swap(0, i);
                            let quoted =
                                crate::demand::estimate_at(&o.route, o.seats, &own_first, time)
                                    .unwrap();
                            assert_eq!(
                                loads[i], quoted.per_departure,
                                "model={model} cabin={business} fare={fare} time={time}"
                            );
                        }
                    }
                }
            }
        }
    }
}
