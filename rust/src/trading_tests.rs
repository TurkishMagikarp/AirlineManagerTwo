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
    cash(&mut g, i64::MIN + 40_000_000); // Leave room in fixture opening balance.
    g.clock.set_running(true);
    let before = state(&g);
    assert!(!g.advance(50000.0 * DAY));
    assert_eq!(state(&g), before);
}
