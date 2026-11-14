use crate::{game::Game, insurance::Coverage, operations, save};
fn game() -> Game {
    let mut g = Game::default();
    g.found("Insurance Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
fn restart(g: &Game) -> Game {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("insurance.json");
    save::save(&p, g).unwrap();
    save::load(&p).unwrap()
}
#[test]
fn insurance_premiums_changes_and_restart() {
    let mut a = game();
    a.set_coverage(Coverage::Basic).unwrap();
    let mut b = a.clone();
    assert!(a.advance(100000.0));
    for _ in 0..100 {
        assert!(b.advance(1000.0));
    }
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(restart(&a)).unwrap()
    );
    a.set_coverage(Coverage::Full).unwrap();
    b.set_coverage(Coverage::Full).unwrap();
    assert!(a.advance(100000.0));
    b = restart(&b);
    assert!(b.advance(100000.0));
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
    a.set_coverage(Coverage::None).unwrap();
    assert_eq!(a.insurance.annual_premium, 0);
    a.validate_history().unwrap();
}
// Find an incident at the very first landing by changing only the world seed.
fn incident_game(coverage: Coverage) -> (Game, u64) {
    let mut g = game();
    let landing = 60 + operations::timings(&g.routes[0], "atr72_600").unwrap().0;
    for seed in 0..100000 {
        g.fuel_market.seed = seed;
        if crate::insurance::incident(g.incident_seed(1), 1) {
            g.set_coverage(coverage).unwrap();
            return (g, landing);
        }
    }
    panic!("no deterministic incident seed");
}
#[test]
fn insurance_incident_deductible_grounding_speeds_and_saved_request() {
    for coverage in [Coverage::None, Coverage::Basic, Coverage::Full] {
        let (mut a, landing) = incident_game(coverage);
        assert!(a.advance(61.0));
        a.request_maintenance(1).unwrap();
        let mut b = restart(&a);
        let mut c = a.clone();
        c.clock.set_speed(16);
        assert!(a.advance((landing - 61) as f64));
        for _ in 61..landing {
            assert!(b.advance(1.0));
        }
        assert!(c.advance((landing - 61) as f64 / 16.0));
        c.clock.set_speed(1);
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&c).unwrap()
        );
        assert_eq!(a.insurance.recent.len(), 1);
        let claim = &a.insurance.recent[0];
        assert_eq!(claim.repair, claim.paid + claim.insurer);
        assert_eq!(claim.insurer == 0, coverage == Coverage::None);
        assert_eq!(
            a.fleet[0].status,
            crate::aircraft::AircraftStatus::Maintenance
        );
        assert!(a.sell_aircraft(1).is_err());
        assert_eq!(a.fleet[0].location_icao, "EDDM");
        a = restart(&a);
        let end = a.fleet[0].maintenance.as_ref().unwrap().end;
        assert!(a.advance((end - landing) as f64));
        assert!(a.routes[0].plan.is_some());
        assert_eq!(a.insurance.recent.len(), 1);
        a.validate_history().unwrap();
    }
}
#[test]
fn insurance_insufficient_cash_repairs_are_mandatory_and_disposal_removes_risk() {
    for coverage in [Coverage::None, Coverage::Full] {
        let (mut g, landing) = incident_game(coverage);
        assert!(g.advance(61.0));
        // Set reconciled cash sufficiently negative that the landing's income cannot fund repairs.
        let shift = -50000 - g.cash();
        g.airline.as_mut().unwrap().capital_eur += shift;
        g.history.as_mut().unwrap().opening_balance += shift;
        assert!(g.advance((landing - 61) as f64));
        assert_eq!(g.insurance.recent.len(), 1);
        assert!(g.cash() < 0);
        g.validate_history().unwrap();
        assert_eq!(
            g.fleet[0].status,
            crate::aircraft::AircraftStatus::Maintenance
        );
    }
    let (mut g, _) = incident_game(Coverage::None);
    g.stop_schedule(1).unwrap();
    g.sell_aircraft(1).unwrap();
    assert!(g.next_insurance_event().is_none());
    assert!(g.advance(86400.0));
    assert!(g.insurance.recent.is_empty());
}
#[test]
fn insurance_v22_migration_clears_all_owners_without_changing_cash() {
    let mut g = game();
    g.competitors = crate::competition::initial_competitors().unwrap();
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("v22.json");
    save::save(&p, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    v["version"] = 22.into();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.remove("insurance");
                o.remove("insurance_costs");
                for v in o.values_mut() {
                    strip(v);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    strip(v);
                }
            }
            _ => {}
        }
    }
    strip(&mut v["state"]);
    std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
    let loaded = save::load(&p).unwrap();
    let mut expected = serde_json::to_value(&g).unwrap();
    crate::insurance::migrate(&mut expected);
    assert_eq!(serde_json::to_value(&loaded).unwrap(), expected);
    for (a, b) in std::iter::once((&g, &loaded)).chain(
        g.competitors
            .iter()
            .zip(&loaded.competitors)
            .map(|(a, b)| (a.game.as_ref(), b.game.as_ref())),
    ) {
        assert_eq!(a.cash(), b.cash());
        assert_eq!(a.fleet, b.fleet);
        assert_eq!(b.insurance.coverage, Coverage::None);
        assert!(b.insurance.recent.is_empty());
    }
}

#[test]
fn insurance_claim_renewal_clean_year_decay_and_late_purchase() {
    let (mut g, landing) = incident_game(Coverage::Basic);
    assert!(g.advance(landing as f64));
    assert_eq!(g.insurance.year_claims, 1);
    // No further flights, but renewal must retain the claim even when history is bounded.
    let end = g.fleet[0].maintenance.as_ref().unwrap().end;
    assert!(g.advance((end - landing) as f64));
    g.stop_schedule(1).unwrap();
    assert!(g.advance((365 * 86400 - end) as f64));
    assert_eq!(
        g.insurance.loading_bps,
        crate::insurance::RULES.claim_loading_bps
    );
    assert_eq!(g.insurance.year_claims, 0);
    g = restart(&g);
    assert!(g.advance((365 * 86400) as f64));
    assert_eq!(
        g.insurance.loading_bps,
        crate::insurance::RULES.claim_loading_bps - crate::insurance::RULES.clean_year_decay_bps
    );
    let mut late = Game::default();
    late.found("Late", "DE", "EDDF").unwrap();
    late.competitors.clear();
    late.clock.set_running(true);
    assert!(late.advance((3 * 365 * 86400 + 12345) as f64));
    late.set_coverage(Coverage::Full).unwrap();
    assert!(late.next_insurance_event().unwrap() > late.clock.elapsed().as_secs());
    assert!(late.advance(100000.0));
    late.validate_insurance().unwrap();
}

#[test]
fn insurance_due_check_and_operated_lease_keep_repair_obligations() {
    let (mut g, landing) = incident_game(Coverage::Full);
    let flight = operations::timings(&g.routes[0], "atr72_600").unwrap().0;
    let before = crate::maintenance::rules("atr72_600")
        .unwrap()
        .interval_flight_hours
        * 3600
        - flight;
    g.fleet[0].wear.flight_seconds = before;
    g.fleet[0].wear.check_seconds = before;
    assert!(g.advance(landing as f64));
    assert_eq!(g.insurance.recent.len(), 1);
    assert!(g.fleet[0].maintenance.as_ref().unwrap().cost_eur > 0);
    assert_eq!(g.history.as_ref().unwrap().maintenance.len(), 1);
    g.validate_history().unwrap();

    let mut lease = game();
    lease.stop_schedule(1).unwrap();
    lease.sell_aircraft(1).unwrap();
    lease.lease_aircraft("atr72_600").unwrap();
    lease.assign_route(1, Some(2)).unwrap();
    lease.schedule(1, 1, 120).unwrap();
    for seed in 0..100000 {
        lease.fuel_market.seed = seed;
        if crate::insurance::incident(lease.incident_seed(2), 1) {
            break;
        }
    }
    lease.set_coverage(Coverage::Basic).unwrap();
    assert!(lease.advance(landing as f64));
    assert_eq!(lease.insurance.recent.len(), 1);
    assert!(lease.return_aircraft(2).is_err());
    let end = lease.fleet[0].maintenance.as_ref().unwrap().end;
    assert!(lease.advance((end - landing) as f64));
    lease.return_aircraft(2).unwrap();
    let claims = lease.insurance.recent.clone();
    assert!(lease.advance(86400.0));
    assert_eq!(lease.insurance.recent, claims);
    lease.validate_history().unwrap();
}

#[test]
fn insurance_rejects_corrupt_policy_and_pause_four_speed_match() {
    let (mut a, landing) = incident_game(Coverage::Full);
    let mut b = a.clone();
    b.clock.set_running(false);
    let paused = serde_json::to_value(&b).unwrap();
    assert!(b.advance(99999.0));
    assert_eq!(serde_json::to_value(&b).unwrap(), paused);
    b.clock.set_speed(4);
    b.clock.set_running(true);
    assert!(b.advance(landing as f64 / 4.0));
    b.clock.set_speed(1);
    assert!(a.advance(landing as f64));
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("invalid.json");
    save::save(&p, &a).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    for (key, value) in [
        ("loading_bps", (-1).into()),
        ("remainder", u64::MAX.into()),
        ("annual_premium", (-1).into()),
    ] {
        let mut corrupt = original.clone();
        corrupt["state"]["insurance"][key] = value;
        std::fs::write(&p, serde_json::to_vec(&corrupt).unwrap()).unwrap();
        assert!(save::load(&p).is_err());
    }
    let mut missing = original;
    missing["state"]
        .as_object_mut()
        .unwrap()
        .remove("insurance");
    std::fs::write(&p, serde_json::to_vec(&missing).unwrap()).unwrap();
    assert!(save::load(&p).is_err());
}
