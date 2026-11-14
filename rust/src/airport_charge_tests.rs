use crate::{airports, economy, game::Game, operations, save};

fn game() -> Game {
    let mut g = Game::default();
    g.found("Airport Audit", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}

#[test]
fn airport_charges_cover_endpoints_empty_full_rounding_and_return() {
    let g = game();
    let mut route = g.routes[0].clone();
    for model in crate::aircraft::models().unwrap() {
        for a in &airports::catalog().unwrap().airports {
            for b in &airports::catalog().unwrap().airports {
                if a.icao == b.icao {
                    continue;
                }
                route.origin_icao = a.icao.clone();
                route.destination_icao = b.icao.clone();
                for passengers in [0, 1, i64::from(model.seats)] {
                    // Baseline tariffs remain in config; independent arithmetic
                    // verifies each endpoint rounds up separately.
                    let config: serde_json::Value =
                        serde_json::from_str(include_str!("../../config/economy.json")).unwrap();
                    let base = config["models"][&model.id]["airport_base_eur"]
                        .as_i64()
                        .unwrap();
                    let tariff =
                        base + config["airport_eur_per_passenger"].as_i64().unwrap() * passengers;
                    let expected = (tariff * i64::from(a.charge_percent) + 99) / 100
                        + (tariff * i64::from(b.charge_percent) + 99) / 100;
                    assert_eq!(
                        economy::airport_bill(&route, &model.id, passengers).unwrap(),
                        expected
                    );
                    let mut reversed = route.clone();
                    std::mem::swap(&mut reversed.origin_icao, &mut reversed.destination_icao);
                    assert_eq!(
                        economy::airport_bill(&reversed, &model.id, passengers).unwrap(),
                        expected
                    );
                }
            }
        }
    }
    let mut regional = g.routes[0].clone();
    regional.origin_icao = "EDLP".into();
    regional.destination_icao = "EDFH".into();
    assert!(economy::airport_bill(&regional, "atr72_600", 0).unwrap() > 0);
    assert!(
        economy::airport_bill(&g.routes[0], "atr72_600", 40).unwrap()
            > 2 * economy::airport_bill(&regional, "atr72_600", 40).unwrap()
    );
    assert!(economy::airport_bill(&regional, "atr72_600", -1).is_err());
    assert!(economy::airport_bill(&regional, "atr72_600", 73).is_err());
    regional.origin_icao = "XXXX".into();
    assert!(economy::airport_bill(&regional, "atr72_600", 0).is_err());
    for index in [0, 1] {
        let q = operations::departure_quote(&g.routes[0], "atr72_600", index, &g.market_routes())
            .unwrap();
        assert_eq!(
            q.totals.airport,
            economy::airport_bill(&g.routes[0], "atr72_600", q.totals.passengers).unwrap()
        );
    }
}

#[test]
fn airport_charges_v23_identity_migration_freezes_all_owners_and_next_departure_uses_catalog() {
    let mut old = game();
    assert!(old.advance(21_700.0));
    assert!(old.finance.totals.flights > 0);
    assert!(old.routes[0].plan.as_ref().unwrap().airborne.is_some());
    // A real legacy tariff, deliberately different from the new endpoint cost.
    old.routes[0]
        .plan
        .as_mut()
        .unwrap()
        .airborne
        .as_mut()
        .unwrap()
        .totals
        .airport = 777;
    assert_eq!(old.competitors.len(), 2);
    for c in &mut old.competitors {
        c.game.routes[0]
            .plan
            .as_mut()
            .unwrap()
            .airborne
            .as_mut()
            .unwrap()
            .totals
            .airport = 888;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v23.json");
    save::save(&path, &old).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(v["version"], 30);
    v["version"] = 23.into();
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let mut migrated = save::load(&path).unwrap();
    assert_eq!(
        serde_json::to_value(&migrated).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    for (a, b) in old.competitors.iter().zip(&migrated.competitors) {
        assert_eq!(a.game.cash(), b.game.cash());
    }
    assert_eq!(old.cash(), migrated.cash());
    let first = migrated.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .clone()
        .unwrap();
    assert!(migrated.advance((first.landing - 21_700) as f64));
    assert_eq!(
        migrated
            .finance
            .recent
            .iter()
            .find(|r| r.landing == first.landing)
            .unwrap()
            .totals
            .airport,
        777
    );
    let plan = migrated.routes[0].plan.as_ref().unwrap();
    let next = plan.first_departure + plan.completed_flights * plan.interval_seconds;
    assert!(migrated.advance((next - first.landing) as f64));
    let quote = migrated.routes[0]
        .plan
        .as_ref()
        .unwrap()
        .airborne
        .as_ref()
        .unwrap();
    assert_eq!(
        quote.totals.airport,
        economy::airport_bill(&migrated.routes[0], "atr72_600", quote.totals.passengers).unwrap()
    );
    save::save(&path, &migrated).unwrap();
    assert_eq!(
        serde_json::to_value(save::load(&path).unwrap()).unwrap(),
        serde_json::to_value(&migrated).unwrap()
    );
}

#[test]
fn airport_charges_steps_speeds_pause_and_restart_match() {
    let initial = game();
    let mut reference = initial.clone();
    assert!(reference.advance(7.0 * 86400.0));
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        for _ in 0..7 {
            assert!(g.advance(86400.0 / f64::from(speed)));
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("restart.json");
            save::save(&path, &g).unwrap();
            g = save::load(&path).unwrap();
            g.clock.set_running(false);
            let paused = serde_json::to_value(&g).unwrap();
            assert!(g.advance(1e6));
            assert_eq!(paused, serde_json::to_value(&g).unwrap());
            g.clock.set_running(true);
        }
        g.clock.set_speed(1);
        assert_eq!(
            serde_json::to_value(&g).unwrap(),
            serde_json::to_value(&reference).unwrap()
        );
    }
}

#[test]
fn airport_charges_planning_preview_uses_selected_cabin_without_mutating_state() {
    let mut g = Game::default();
    g.found("Planning", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.set_cabin_layout(1, 5).unwrap();
    let before = serde_json::to_value(&g).unwrap();
    let q = g.planning_quote("EDDF", "EDDM", 1).unwrap();
    assert!(q.passengers > 0);
    assert_eq!(q.seats, 67);
    assert_eq!(
        q.airport,
        economy::airport_bill(
            &crate::routes::Route {
                origin_icao: "EDDF".into(),
                destination_icao: "EDDM".into(),
                ..game().routes[0].clone()
            },
            "atr72_600",
            q.passengers
        )
        .unwrap()
    );
    assert_eq!(before, serde_json::to_value(&g).unwrap());
    assert!(g.planning_quote("XXXX", "EDDM", 1).is_err());
}

#[test]
fn airport_charges_ten_year_ai_comparison_stays_solvent_and_bounded() {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            economy::LEGACY_AIRPORT_CHARGES.with(|v| v.set(false));
        }
    }
    let _reset = Reset;
    let mut new = game();
    let mut legacy = new.clone();
    let dir = tempfile::tempdir().unwrap();
    for year in 1..=10 {
        assert!(new.advance(365.0 * 86400.0));
        economy::LEGACY_AIRPORT_CHARGES.with(|v| v.set(true));
        assert!(legacy.advance(365.0 * 86400.0));
        economy::LEGACY_AIRPORT_CHARGES.with(|v| v.set(false));
        assert_eq!(new.competitors.len(), 2);
        for (a, b) in new.competitors.iter().zip(&legacy.competitors) {
            assert!(!a.game.insolvent() && !a.game.distressed());
            assert!(a.game.finance.totals.flights > 0);
            assert!(
                a.game.cash() > b.game.cash() / 2 && a.game.cash() < b.game.cash() * 3 / 2,
                "AI cash diverged: year={year} new={} legacy={}",
                a.game.cash(),
                b.game.cash()
            );
            println!(
                "AIRPORT year={year} airline={} cash_new={} cash_legacy={}",
                a.game.airline.as_ref().unwrap().name,
                a.game.cash(),
                b.game.cash()
            );
        }
        let path = dir.path().join("annual.json");
        save::save(&path, &new).unwrap();
        new = save::load(&path).unwrap();
    }
}
