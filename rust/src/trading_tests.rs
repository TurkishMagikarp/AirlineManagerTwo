use crate::{
    aircraft::AircraftStatus,
    game::Game,
    save,
    trading::{self, Kind},
};
const DAY: f64 = 86400.0;
fn fresh() -> Game {
    let mut g = Game::default();
    g.found("Trading Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g
}
fn state(g: &Game) -> serde_json::Value {
    let mut v = serde_json::to_value(g).unwrap();
    v["clock"]["speed_index"] = 0.into();
    v
}
fn cash(g: &mut Game, amount: i64) {
    g.history.as_mut().unwrap().opening_balance += amount - g.cash();
    g.airline.as_mut().unwrap().capital_eur = amount;
}
fn service(leased: bool) -> Game {
    let mut g = fresh();
    if leased {
        g.lease_aircraft("atr72_600").unwrap();
    } else {
        g.buy_aircraft("atr72_600").unwrap();
    }
    g.equip_staff_for("atr72_600").unwrap();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}

#[test]
fn trading_sales_quote_current_list_age_condition_and_clean_routes() {
    let mut g = service(false);
    let new = g.disposal_quote(1).unwrap();
    assert_eq!(new, 2_400_000);
    g.fleet[0].purchase_price_eur = 1; // Quote uses today's catalog, not historical cost.
    assert_eq!(g.disposal_quote(1).unwrap(), new);
    let flight = crate::operations::timings(&g.routes[0], "atr72_600")
        .unwrap()
        .0;
    assert!(g.advance((60 + flight) as f64));
    assert_eq!(g.fleet[0].status, AircraftStatus::Ground);
    let quote = g.disposal_quote(1).unwrap();
    assert!(quote < new);
    let before = g.cash();
    g.sell_aircraft(1).unwrap();
    assert_eq!(g.cash(), before + quote);
    assert!(g.fleet.is_empty());
    assert!(g.routes[0].aircraft_id.is_none() && g.routes[0].plan.is_none());
    assert_eq!(
        g.history
            .as_ref()
            .unwrap()
            .aircraft_cash
            .last()
            .unwrap()
            .kind,
        Kind::Sale
    );
    g.validate_history().unwrap();
    g.validate_routes().unwrap();
    let before = state(&g);
    assert!(g.sell_aircraft(1).is_err());
    assert_eq!(before, state(&g));

    let mut g = fresh();
    g.buy_aircraft("atr72_600").unwrap();
    g.clock.set_running(true);
    assert!(g.advance(365.0 * DAY));
    assert_eq!(g.disposal_quote(1).unwrap(), 2_250_000);
    let r = crate::maintenance::rules("atr72_600").unwrap();
    g.fleet[0].wear.check_cycles = r.interval_cycles;
    assert_eq!(g.disposal_quote(1).unwrap(), 1_125_000);
}

#[test]
fn trading_airborne_pending_and_active_checks_refuse_without_mutation() {
    for leased in [false, true] {
        let mut g = service(leased);
        assert!(g.advance(60.0));
        assert_eq!(g.fleet[0].status, AircraftStatus::InFlight);
        let before = state(&g);
        assert!(g.disposal_quote(1).unwrap_err().contains("Luft"));
        assert!(g.sell_aircraft(1).is_err());
        assert!(g.return_aircraft(1).is_err());
        assert_eq!(state(&g), before);
        // Even a parked aircraft with an outstanding hangar order is refused.
        g.fleet[0].status = AircraftStatus::Parked;
        g.fleet[0].maintenance = Some(crate::maintenance::Visit {
            start: 100,
            end: 200,
            cost_eur: 100,
            unscheduled: false,
            resume: None,
            staffed: true,
        });
        for status in [AircraftStatus::Parked, AircraftStatus::Maintenance] {
            g.fleet[0].status = status;
            let before = state(&g);
            assert!(g.disposal_quote(1).unwrap_err().contains("Wartung"));
            assert!(g.sell_aircraft(1).is_err());
            assert!(g.return_aircraft(1).is_err());
            assert_eq!(state(&g), before);
        }
    }
}

#[test]
fn trading_leases_charge_deposit_rent_and_damage_without_free_money() {
    let mut g = fresh();
    let start = g.cash();
    let (deposit, monthly) = trading::lease_terms("atr72_600").unwrap();
    g.lease_aircraft("atr72_600").unwrap();
    assert_eq!(g.cash(), start - deposit);
    assert!(g.history.as_ref().unwrap().purchases.is_empty());
    let before = state(&g);
    assert!(g.sell_aircraft(1).unwrap_err().contains("Leasing"));
    assert_eq!(state(&g), before);
    g.clock.set_running(true);
    assert!(g.advance(30.0 * DAY - 1.0));
    assert_eq!(g.cash(), start - deposit);
    assert!(g.advance(1.0));
    assert_eq!(g.cash(), start - deposit - monthly);
    assert_eq!(g.disposal_quote(1).unwrap(), deposit);
    g.fleet[0].wear.check_cycles = crate::maintenance::rules("atr72_600")
        .unwrap()
        .interval_cycles;
    let refund = g.disposal_quote(1).unwrap();
    assert_eq!(refund, 0);
    g.return_aircraft(1).unwrap();
    assert!(g.next_lease_event().is_none());
    let returned = g.cash();
    assert!(g.advance(90.0 * DAY));
    assert_eq!(g.cash(), returned);
    assert!(g.return_aircraft(1).is_err());
    g.validate_history().unwrap();
    let mut g = fresh();
    g.lease_aircraft("atr72_600").unwrap();
    g.return_aircraft(1).unwrap();
    assert_eq!(g.cash(), start); // Full deposit back once, no sale arbitrage.
    cash(&mut g, deposit - 1);
    let before = state(&g);
    assert!(g.lease_aircraft("atr72_600").is_err());
    assert_eq!(state(&g), before);
}

#[test]
fn trading_insufficient_rent_causes_distress_and_stops_departures() {
    let mut g = service(true);
    // Schedule first departure after the bill so a missed payment prevents it.
    g.stop_schedule(1).unwrap();
    g.schedule(1, 30 * 1440 + 1, 120).unwrap();
    cash(&mut g, 0);
    assert!(g.advance(30.0 * DAY));
    assert!(g.distressed());
    assert!(g.cash() < 0);
    assert_eq!(g.fleet[0].lease.as_ref().unwrap().next_due, 60 * 86400);
    assert!(g.routes[0].plan.is_none());
    assert!(g.lease_aircraft("atr72_600").is_err());
    let before = g.cash();
    g.return_aircraft(1).unwrap();
    assert!(g.cash() > before);
    g.validate_history().unwrap();
}

#[test]
fn trading_lease_boundaries_all_speeds_steps_restart_and_corrupt_saves() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lease.json");
    let mut base = fresh();
    base.clock.set_running(true);
    assert!(base.advance(123.0));
    base.lease_aircraft("atr72_600").unwrap();
    base.lease_aircraft("a320_200").unwrap();
    cash(&mut base, 40_000); // Eventually in distress: unpaid rent is never forgiven.
    let mut expected = base.clone();
    assert!(expected.advance(95.0 * DAY));
    for speed in [1, 4, 16] {
        let mut g = base.clone();
        g.clock.set_speed(speed);
        for n in 0..190 {
            assert!(g.advance(DAY / 2.0 / f64::from(speed)));
            if n == 72 {
                save::save(&path, &g).unwrap();
                let restored = save::load(&path).unwrap();
                assert_eq!(state(&g), state(&restored));
                g = restored;
            }
        }
        assert_eq!(state(&expected), state(&g));
        g.validate_history().unwrap();
        g.validate_fleet().unwrap();
    }
    save::save(&path, &expected).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for (field, value) in [
        ("monthly", 0),
        ("deposit", -1),
        ("period", 0),
        ("next_due", 123),
    ] {
        let mut bad = saved.clone();
        bad["state"]["fleet"]["aircraft"][0]["lease"][field] = value.into();
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(save::load(&path).is_err(), "accepted {field}");
    }
    let mut bad = saved;
    bad["state"]["fleet"]["aircraft"][0]
        .as_object_mut()
        .unwrap()
        .remove("acquired_at");
    std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(save::load(&path).is_err());
}

#[test]
fn trading_v12_migrates_player_and_ai_acquisition_without_changing_balance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    let mut g = fresh();
    g.buy_aircraft("atr72_600").unwrap();
    g.competitors = crate::competition::initial_competitors().unwrap();
    save::save(&path, &g).unwrap();
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    old["version"] = 12.into();
    fn strip(g: &mut serde_json::Value, fleet_key: bool) {
        let planes = if fleet_key {
            &mut g["fleet"]["aircraft"]
        } else {
            &mut g["fleet"]
        };
        for p in planes.as_array_mut().unwrap() {
            p.as_object_mut().unwrap().remove("acquired_at");
            p.as_object_mut().unwrap().remove("lease");
        }
        g["history"]
            .as_object_mut()
            .unwrap()
            .remove("aircraft_cash");
    }
    strip(&mut old["state"], true);
    for c in old["state"]["competitors"].as_array_mut().unwrap() {
        strip(&mut c["game"], false);
    }
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let loaded = save::load(&path).unwrap();
    g.migrate_insurance_fixture();
    assert_eq!(state(&g), state(&loaded));
}

#[test]
fn trading_overflow_cannot_remove_aircraft_or_partially_post() {
    let mut g = fresh();
    g.buy_aircraft("atr72_600").unwrap();
    g.airline.as_mut().unwrap().capital_eur = i64::MAX;
    let before = state(&g);
    assert!(g.sell_aircraft(1).is_err());
    assert_eq!(state(&g), before);
    let mut g = fresh();
    g.lease_aircraft("atr72_600").unwrap();
    // Trigger overflow on the first rent, before the insolvency deadline can
    // terminate this company. Use a wide fixture calculation to avoid overflow
    // in the test helper's intermediate opening-balance subtraction.
    let balance = i64::MIN + 1;
    let opening = i128::from(g.history.as_ref().unwrap().opening_balance) + i128::from(balance)
        - i128::from(g.cash());
    g.history.as_mut().unwrap().opening_balance = opening.try_into().unwrap();
    g.airline.as_mut().unwrap().capital_eur = balance;
    g.clock.set_running(true);
    let before = state(&g);
    assert!(!g.advance(50000.0 * DAY));
    assert_eq!(state(&g), before);
}

#[test]
fn used_market_sale_rebuy_preserves_airframe_and_posts_purchase() {
    let mut g = fresh();
    g.buy_aircraft("atr72_600").unwrap();
    g.clock.set_running(true);
    assert!(g.advance(10.0 * DAY));
    g.fleet[0].wear = g.fleet[0].wear.added(120, 3600).unwrap();
    let original = g.fleet[0].clone();
    let hull = g.hull_value(&original).unwrap();
    g.sell_aircraft(1).unwrap();
    let listing = g.used_market.listings[0].clone();
    let price = g.used_price(&listing).unwrap();
    assert_eq!(
        price,
        hull * (10000 + trading::RULES.dealer_markup_bps) / 10000
    );
    assert_eq!(listing.aircraft.wear, original.wear);
    assert!(g.advance(20.0 * DAY));
    assert!(g.used_price(&listing).unwrap() < price);
    let before = g.cash();
    let paid = g.used_price(&listing).unwrap();
    g.buy_used_aircraft(listing.id).unwrap();
    let p = &g.fleet[0];
    assert_eq!(p.wear, original.wear);
    assert_eq!(p.acquired_at, original.acquired_at);
    assert_eq!(p.purchased_at, Some(30 * 86400));
    assert_eq!(p.purchase_price_eur, paid);
    assert_eq!(p.lease, None);
    assert_ne!(p.id, original.id);
    assert_eq!(g.cash(), before - paid);
    assert_eq!(
        g.history.as_ref().unwrap().purchases.last().unwrap().amount,
        paid
    );
    assert!(g.used_market.listings.is_empty());
    g.validate_history().unwrap();
    g.validate_fleet().unwrap();
    let before = state(&g);
    assert!(g.buy_used_aircraft(listing.id).is_err());
    assert_eq!(state(&g), before);
}

#[test]
fn used_market_lease_returns_caps_expiry_and_failed_buys() {
    let mut g = fresh();
    for _ in 0..=trading::RULES.market_size_cap {
        g.lease_aircraft("atr72_600").unwrap();
        let id = g.fleet.last().unwrap().id;
        g.return_aircraft(id).unwrap();
    }
    assert_eq!(g.used_market.listings.len(), trading::RULES.market_size_cap);
    assert_eq!(g.used_market.listings[0].id, 2);
    assert_eq!(g.used_market.listings[0].seller, "Leasinggesellschaft");
    assert!(
        g.used_market
            .listings
            .iter()
            .all(|l| l.aircraft.lease.is_none())
    );
    cash(&mut g, 1);
    let before = state(&g);
    assert!(g.buy_used_aircraft(2).is_err());
    assert_eq!(state(&g), before);
    let expiry = trading::RULES.market_expiry_days as f64 * DAY;
    // Running an empty airline needs no staff or operating money.
    g.clock.set_running(true);
    assert!(g.advance(expiry - 1.0));
    assert!(!g.used_market.listings.is_empty());
    let balance = g.cash();
    assert!(g.advance(1.0));
    assert!(g.used_market.listings.is_empty());
    assert_eq!(g.cash(), balance);
}

#[test]
fn used_market_speeds_steps_pause_restart_migration_and_corruption() {
    let mut seed = fresh();
    seed.buy_aircraft("atr72_600").unwrap();
    seed.sell_aircraft(1).unwrap();
    seed.clock.set_running(true);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("used.json");
    let mut expected = seed.clone();
    assert!(expected.advance(40.0 * DAY));
    for speed in [1, 4, 16] {
        let mut g = seed.clone();
        g.clock.set_speed(speed);
        for day in 0..40 {
            assert!(g.advance(DAY / f64::from(speed)));
            if day == 19 {
                save::save(&path, &g).unwrap();
                g = save::load(&path).unwrap();
                g.clock.set_running(false);
                let paused = state(&g);
                g.advance(DAY);
                assert_eq!(state(&g), paused);
                g.clock.set_running(true);
            }
        }
        assert_eq!(state(&g), state(&expected));
    }
    save::save(&path, &expected).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for field in ["id", "listed_at"] {
        let mut bad = saved.clone();
        bad["state"]["used_market"]["listings"][0][field] = u64::MAX.into();
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(save::load(&path).is_err());
    }
    let mut old = saved;
    old["version"] = 29.into();
    old["state"].as_object_mut().unwrap().remove("used_market");
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let migrated = save::load(&path).unwrap();
    assert!(migrated.used_market.listings.is_empty());
    assert_eq!(migrated.cash(), expected.cash());
    assert_eq!(
        migrated.history.unwrap().purchases,
        expected.history.unwrap().purchases
    );
}

#[test]
fn used_market_shared_ai_two_year_economy_and_determinism() {
    let mut g = fresh();
    cash(&mut g, 100_000_000);
    for model in ["atr72_600", "a320_200", "b737_800"] {
        g.buy_aircraft(model).unwrap();
        let p = g.fleet.last_mut().unwrap();
        p.wear = p.wear.added(40, 3600).unwrap();
        let id = p.id;
        g.sell_aircraft(id).unwrap();
    }
    g.competitors = crate::competition::initial_competitors().unwrap();
    for c in &mut g.competitors {
        c.game.stop_schedule(1).unwrap();
        c.game.routes.clear();
        c.game.sell_aircraft(1).unwrap();
        // Transfer AI sale through the same market handoff used at reviews.
        for l in c.game.used_market.listings.drain(..) {
            let mut l = l;
            l.id = g.used_market.next_id;
            g.used_market.next_id += 1;
            g.used_market.listings.push(l);
        }
    }
    g.clock.set_running(true);
    let mut expected = g.clone();
    assert!(expected.advance(730.0 * DAY));
    assert_eq!(expected.competitors.len(), 2);
    assert!(
        expected
            .competitors
            .iter()
            .any(|c| c.game.fleet.iter().any(|p| p.purchased_at.is_some()))
    );
    for c in &expected.competitors {
        assert!(c.game.cash() > 0 && c.game.cash() < 1_000_000_000);
        assert!(c.game.fleet.len() <= 6);
        c.game.validate_history().unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared-market.json");
    for speed in [1, 4, 16] {
        let mut run = g.clone();
        run.clock.set_speed(speed);
        for day in 0..730 {
            assert!(run.advance(DAY / f64::from(speed)));
            if day == 90 {
                save::save(&path, &run).unwrap();
                run = save::load(&path).unwrap();
            }
        }
        assert_eq!(state(&run), state(&expected));
        run.validate_competitors().unwrap();
    }
}

#[test]
fn used_market_ai_disposal_is_visible_to_player_and_other_owners() {
    for leased in [false, true] {
        let mut g = fresh();
        g.competitors = crate::competition::initial_competitors().unwrap();
        g.competitors.truncate(1);
        let c = &mut g.competitors[0];
        c.game.stop_schedule(1).unwrap();
        c.game.routes.clear();
        if leased {
            c.game.fleet.clear();
            c.game.lease_aircraft("atr72_600").unwrap();
        }
        c.game.fleet[0].wear = c.game.fleet[0].wear.added(50, 3600).unwrap();
        let wear = c.game.fleet[0].wear.clone();
        let seller = if leased {
            "Leasinggesellschaft".to_string()
        } else {
            c.game.airline.as_ref().unwrap().name.clone()
        };
        cash(&mut c.game, -50_000);
        g.clock.set_running(true);
        assert!(g.advance(7.0 * DAY));
        let listing = g
            .used_market
            .listings
            .iter()
            .find(|l| l.seller == seller)
            .unwrap();
        assert_eq!(listing.aircraft.wear, wear);
        assert!(listing.aircraft.lease.is_none());
        let id = listing.id;
        assert!(g.competitors[0].game.used_market.listings.is_empty());
        g.validate_competitors().unwrap();
        g.buy_used_aircraft(id).unwrap();
        assert_eq!(g.fleet[0].wear, wear);
        g.validate_history().unwrap();
    }
}
