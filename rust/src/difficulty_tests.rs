use crate::{
    difficulty::{Level, Settings},
    game::Game,
};
const DAY: f64 = 86400.0;
fn game(capital: &str, competition: &str, crises: &str) -> Game {
    let mut g = Game::default();
    g.found_with_levels(
        "Difficulty audit",
        "DE",
        "EDDF",
        capital,
        competition,
        crises,
    )
    .unwrap();
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("world.json");
    crate::save::save(&p, g).unwrap();
    crate::save::load(&p).unwrap()
}
#[test]
fn difficulty_standard_and_independent_axes() {
    let standard = game("standard", "standard", "standard");
    let mut original = Game::default();
    original.found("Difficulty audit", "DE", "EDDF").unwrap();
    assert_eq!(state(&standard), state(&original));
    assert_eq!(standard.competitors.len(), 2);
    for (capital, count) in [("low", 5_000_000), ("high", 20_000_000)] {
        let g = game(capital, "standard", "standard");
        assert_eq!(g.cash(), count);
        assert_eq!(g.fuel_market, standard.fuel_market);
        assert_eq!(g.world_economy, standard.world_economy);
        // Metadata may differ, every actual AI rule/state remains identical.
        for (a, b) in g.competitors.iter().zip(&standard.competitors) {
            let mut a = (*a.game).clone();
            a.difficulty = b.game.difficulty;
            assert_eq!(state(&a), state(&b.game));
        }
    }
    for (level, count) in [("low", 1), ("high", 3)] {
        // Exercise duplicate named/nearest resolution for every starter region.
        for (country, home) in [
            ("DE", "EDDF"),
            ("US", "KJFK"),
            ("BR", "SBGR"),
            ("JP", "RJTT"),
            ("AU", "YSSY"),
            ("ZA", "FAOR"),
        ] {
            let mut g = Game::default();
            g.found_with_levels("Count", country, home, "standard", level, "standard")
                .unwrap();
            assert_eq!(g.competitors.len(), count);
            assert_eq!(g.cash(), standard.cash());
            assert_eq!(g.fuel_market, standard.fuel_market);
            assert_eq!(g.world_economy, standard.world_economy);
            let homes: std::collections::HashSet<_> = g
                .competitors
                .iter()
                .map(|c| &c.game.airline.as_ref().unwrap().home_airport_icao)
                .collect();
            assert_eq!(homes.len(), count);
            for c in &g.competitors {
                assert_eq!(c.game.history.as_ref().unwrap().opening_balance, 10_000_000);
            }
            assert_eq!(state(&restart(&g)), state(&g));
        }
    }
}
#[test]
fn difficulty_crises_scale_only_deviations_and_share_the_world() {
    let base = game("standard", "standard", "standard");
    for (key, level) in [("low", Level::Low), ("high", Level::High)] {
        let g = game("standard", "standard", key);
        assert_eq!(g.cash(), base.cash());
        assert_eq!(g.competitors.len(), base.competitors.len());
        let mut changed_fuel = false;
        let mut changed_cycle = false;
        for day in 0..730 {
            let t = day * 86400;
            let fuel = base.fuel_market.price_at(t);
            let cycle = base.world_economy.index_at(t);
            assert_eq!(
                g.fuel_market.price_at(t),
                base.fuel_market.price_at(0) + level.scale(fuel - base.fuel_market.price_at(0))
            );
            assert_eq!(
                g.world_economy.index_at(t),
                10000 + level.scale(cycle - 10000)
            );
            changed_fuel |= fuel != g.fuel_market.price_at(t);
            changed_cycle |= cycle != g.world_economy.index_at(t);
            for c in &g.competitors {
                assert_eq!(c.game.fuel_market.price_at(t), g.fuel_market.price_at(t));
                assert_eq!(
                    c.game.world_economy.index_at(t),
                    g.world_economy.index_at(t)
                );
            }
        }
        assert!(changed_fuel && changed_cycle);
        for (a, b) in g.competitors.iter().zip(&base.competitors) {
            let mut a = (*a.game).clone();
            a.difficulty = b.game.difficulty;
            a.world_economy = b.game.world_economy;
            a.fuel_market = b.game.fuel_market.clone();
            assert_eq!(state(&a), state(&b.game));
        }
    }
}
#[test]
fn difficulty_invalid_levels_are_transactional() {
    let mut g = game("high", "low", "high");
    let before = state(&g);
    for values in [
        ("typo", "standard", "standard"),
        ("standard", "", "standard"),
        ("low", "high", "HIGH"),
    ] {
        assert!(
            g.found_with_levels("Replace", "DE", "EDDF", values.0, values.1, values.2)
                .is_err()
        );
        assert_eq!(state(&g), before);
    }
}
#[test]
fn difficulty_roundtrip_migration_and_corrupt_world_rejection() {
    let mut g = game("high", "high", "low");
    g.clock.set_running(true);
    assert!(g.advance(33.0 * DAY));
    assert_eq!(state(&restart(&g)), state(&g));
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("old.json");
    let mut standard = game("standard", "standard", "standard");
    standard.clock.set_running(true);
    assert!(standard.advance(33.0 * DAY));
    crate::save::save(&p, &standard).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    v["version"] = 35.into();
    v["state"].as_object_mut().unwrap().remove("difficulty");
    std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
    let mut migrated = crate::save::load(&p).unwrap();
    assert_eq!(migrated.difficulty, Settings::default());
    assert_eq!(state(&migrated), state(&standard));
    assert!(migrated.advance(30.0 * DAY));
    assert!(standard.advance(30.0 * DAY));
    assert_eq!(state(&migrated), state(&standard));
    crate::save::save(&p, &g).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    for field in ["capital", "competition", "crises"] {
        let mut broken = original.clone();
        broken["state"]["difficulty"][field] = "unknown".into();
        std::fs::write(&p, serde_json::to_vec(&broken).unwrap()).unwrap();
        assert!(crate::save::load(&p).is_err());
    }
    let mut broken = original.clone();
    broken["state"]["fuel_market"]["crises"] = "high".into();
    std::fs::write(&p, serde_json::to_vec(&broken).unwrap()).unwrap();
    assert!(crate::save::load(&p).is_err());
}
#[test]
fn difficulty_nonstandard_steps_speeds_restart_match() {
    let mut initial = game("high", "high", "high");
    initial.buy_aircraft("atr72_600").unwrap();
    initial.create_route("EDDF", "EDDM", 1).unwrap();
    initial.staff_up();
    initial.schedule(1, 1, 120).unwrap();
    initial.clock.set_running(true);
    let mut whole = initial.clone();
    assert!(whole.advance(60.0 * DAY));
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        for step in 0..240 {
            assert!(g.advance(DAY / 4.0 / f64::from(speed)));
            if step == 119 {
                g = restart(&g);
            }
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&whole), "speed={speed}");
    }
}
#[test]
fn difficulty_two_year_rival_audit() {
    for (competition, crises) in [
        ("standard", "standard"),
        ("low", "standard"),
        ("high", "standard"),
        ("standard", "low"),
        ("standard", "high"),
    ] {
        let mut g = game("standard", competition, crises);
        g.clock.set_running(true);
        for _ in 0..104 {
            assert!(g.advance(7.0 * DAY));
            for c in &g.competitors {
                assert!(!c.game.distressed() && !c.game.insolvent());
            }
        }
        assert!(g.advance(2.0 * DAY));
        g.validate_competitors().unwrap();
        for c in &g.competitors {
            assert!((1..=6).contains(&c.game.fleet.len()));
            assert!(c.game.cash() > 0 && c.game.cash() < 1_000_000_000);
            println!(
                "DIFFICULTY competition={competition} crises={crises} home={} cash={} profit={} fleet={} routes={}",
                c.game.airline.as_ref().unwrap().home_airport_icao,
                c.game.cash(),
                c.game.finance.totals.profit(),
                c.game.fleet.len(),
                c.game.routes.len()
            );
        }
    }
}
