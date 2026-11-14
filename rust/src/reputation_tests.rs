use crate::{
    game::Game,
    reputation::{self, DAY, Reputation, SCALE},
    save,
};
fn game() -> Game {
    let mut g = Game::default();
    g.found("Punctual Air", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("reputation.json");
    save::save(&path, g).unwrap();
    save::load(&path).unwrap()
}
#[test]
fn reputation_neutral_bad_run_recovery_idle_and_bounds() {
    let mut r = Reputation::default();
    assert_eq!(reputation::attractiveness(r.score), 1.0);
    r.close_day();
    assert_eq!(r, Reputation::default());
    for day in 0..42 {
        r.observe(100, 100);
        r.close_day();
        assert!(r.trend < 0);
        assert!(r.score >= -SCALE);
        if day == 0 {
            assert!(r.score > -SCALE / 10);
        }
    }
    assert!(r.score < -SCALE * 8 / 10);
    let bad = r.score;
    r.close_day();
    assert_eq!(r.score, bad);
    assert_eq!(r.trend, 0);
    for _ in 0..84 {
        r.observe(100, 0);
        r.close_day();
        assert!(r.trend > 0);
    }
    assert!(r.score > SCALE * 9 / 10);
    for late in [0, 100] {
        for _ in 0..365 {
            r.observe(100, late);
            r.close_day();
            assert!((-SCALE..=SCALE).contains(&r.score));
            assert!((0.97..=1.03).contains(&reputation::attractiveness(r.score)));
        }
    }
    r.score = SCALE + 1;
    assert!(r.validate().is_err());
}
#[test]
fn reputation_v27_neutral_migration_preserves_airborne_and_all_other_state() {
    let mut g = game();
    assert!(g.advance(21_700.0));
    assert!(g.routes[0].plan.as_ref().unwrap().airborne.is_some());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v27.json");
    save::save(&path, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["version"] = 27.into();
    v["state"].as_object_mut().unwrap().remove("reputation");
    for c in v["state"]["competitors"].as_array_mut().unwrap() {
        c["game"].as_object_mut().unwrap().remove("reputation");
    }
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let migrated = save::load(&path).unwrap();
    g.reputation = Reputation::default();
    for c in &mut g.competitors {
        c.game.reputation = Reputation::default();
    }
    assert_eq!(state(&migrated), state(&g));
    let mut loaded = restart(&migrated);
    assert!(loaded.advance((30 * DAY) as f64));
    assert!(g.advance((30 * DAY) as f64));
    assert_eq!(state(&loaded), state(&g));
}
#[test]
fn reputation_steps_speeds_pause_restart_and_ai_parity() {
    let initial = game();
    let mut expected = initial.clone();
    assert!(expected.advance((60 * DAY) as f64));
    assert!(expected.reputation.score > 0);
    for c in &expected.competitors {
        assert!(c.game.reputation.score > 0);
        assert!(
            expected
                .market_routes()
                .iter()
                .filter(|o| o.owner == c.game.market_owner_key())
                .all(|o| o.reputation == c.game.reputation.score)
        );
    }
    // The same pure daily rule consumes player and AI observations.
    let mut player = Game::default();
    let mut ai = initial.competitors[0].game.clone();
    ai.reputation = Reputation::default();
    for _ in 0..30 {
        for g in [&mut player, &mut *ai] {
            g.reputation.observe(10, 7);
            g.reputation.close_day();
        }
        assert_eq!(player.reputation, ai.reputation);
    }
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        g.clock.set_running(false);
        let paused = state(&g);
        assert!(g.advance(100000.0));
        assert_eq!(paused, state(&g));
        g.clock.set_running(true);
        assert!(g.advance(123457.0 / f64::from(speed)));
        g = restart(&g);
        let mut left = 60 * DAY - 123457;
        while left > 0 {
            let n = left.min(10003);
            assert!(g.advance(n as f64 / f64::from(speed)));
            left -= n;
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected));
    }
}
#[test]
fn reputation_choice_is_gentle_and_never_changes_caps_or_price_limits() {
    let g = game();
    let route = &g.routes[0];
    let mut offers = g.market_routes();
    for score in [-SCALE, 0, SCALE] {
        for o in &mut offers {
            o.reputation = score;
        }
        let estimate = crate::demand::estimate_at(route, 72, &offers, DAY).unwrap();
        assert!((0..=72).contains(&estimate.per_departure));
        let mut expensive = route.clone();
        expensive.ticket_eur = 100_000;
        let mut expensive_offer = crate::demand::Offer::new(&expensive, 72);
        expensive_offer.reputation = score;
        assert_eq!(
            crate::demand::estimate_at(&expensive, 72, &[expensive_offer], DAY)
                .unwrap()
                .per_departure,
            0
        );
    }
    // One daily departure avoids hiding the small weight change in integer
    // passenger rounding; abundant seats avoid capacity saturation.
    let mut route = route.clone();
    route.ticket_eur = 70;
    route.plan.as_mut().unwrap().interval_seconds = 43200;
    let mut loads = Vec::new();
    for score in [-SCALE, 0, SCALE] {
        let mut supply = vec![crate::demand::Offer::new(&route, 10_000)];
        supply[0].reputation = score;
        loads.push(
            crate::demand::estimate_at(&route, 10_000, &supply, DAY)
                .unwrap()
                .daily_passengers,
        );
    }
    // A positive bonus may already reach the unchanged willingness ceiling.
    assert!(loads[0] < loads[1] && loads[1] <= loads[2], "{loads:?}");
    assert!(loads[2] / loads[0] < 1.07);
}
#[test]
fn reputation_ten_year_ai_revenue_comparison_is_modest() {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            reputation::NEUTRAL_CHOICE.with(|v| v.set(false));
        }
    }
    let _reset = Reset;
    let mut current = game();
    let mut neutral = current.clone();
    for year in 1..=10 {
        assert!(current.advance((365 * DAY) as f64));
        reputation::NEUTRAL_CHOICE.with(|v| v.set(true));
        assert!(neutral.advance((365 * DAY) as f64));
        reputation::NEUTRAL_CHOICE.with(|v| v.set(false));
        assert_eq!(current.competitors.len(), 2);
        assert_eq!(neutral.competitors.len(), 2);
        for (a, b) in current.competitors.iter().zip(&neutral.competitors) {
            assert!(!a.game.distressed() && !a.game.insolvent());
            let revenue = a.game.finance.totals.revenue;
            let baseline = b.game.finance.totals.revenue;
            let delta = (revenue - baseline) as f64 / baseline as f64;
            assert!(delta.abs() < 0.10, "year={year} revenue change={delta}");
            println!(
                "REPUTATION year={year} airline={} revenue={revenue} neutral={baseline} delta={delta:.5} cash={}",
                a.game.airline.as_ref().unwrap().name,
                a.game.cash()
            );
        }
        current = restart(&current);
        neutral = restart(&neutral);
    }
}

#[test]
fn reputation_attention_only_when_low_and_falling() {
    let mut g = game();
    g.reputation.score = -600_000;
    g.reputation.trend = -100;
    assert!(
        g.attention_snapshot()
            .iter()
            .any(|e| e.kind == "reputation")
    );
    for trend in [0, 100] {
        g.reputation.trend = trend;
        assert!(
            !g.attention_snapshot()
                .iter()
                .any(|e| e.kind == "reputation")
        );
    }
    g.reputation.score = 0;
    g.reputation.trend = -100;
    assert!(
        !g.attention_snapshot()
            .iter()
            .any(|e| e.kind == "reputation")
    );
}
