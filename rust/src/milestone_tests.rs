use crate::{clock::Clock, game::Game, milestones::*, save};
use std::time::Duration;
const DAY: u64 = 86400;
fn game() -> Game {
    let mut g = Game::default();
    g.found("Goals", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g
}
#[test]
fn milestones_daily_once_sticky_notifications_and_reset() {
    let mut g = game();
    g.competitors.clear();
    let mut jumped = g.clone();
    let result = jumped.jump(DAY as f64).unwrap();
    assert_eq!(result.advanced_seconds, DAY);
    assert!(result.reason.is_empty());
    assert!(g.milestones.as_ref().unwrap().reached.is_empty());
    g.clock.set_running(true);
    assert!(g.advance((DAY - 1) as f64));
    assert!(g.milestones.as_ref().unwrap().reached.is_empty());
    assert!(g.advance(1.0));
    assert_eq!(g.milestones.as_ref().unwrap().reached["fleet_1"], DAY);
    let pending = g.take_milestone_notifications();
    assert!(pending.contains(&"fleet_1".to_string()));
    assert!(g.take_milestone_notifications().is_empty());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("delivered.json");
    save::save(&path, &g).unwrap();
    g = save::load(&path).unwrap();
    assert!(g.take_milestone_notifications().is_empty());
    let reached = g.milestones.as_ref().unwrap().reached.clone();
    g.routes.clear();
    g.fleet.clear();
    assert!(g.advance(DAY as f64));
    for (id, time) in reached {
        assert_eq!(g.milestones.as_ref().unwrap().reached[&id], time);
    }
    g.found("Reset", "DE", "EDDF").unwrap();
    assert_eq!(g.milestones, Some(State::default()));
}
#[test]
fn milestones_all_kinds_thresholds_and_streak_reset() {
    let mut g = game();
    g.reputation.score = 600_000;
    g.finance.totals.passengers = 1_000_000;
    g.finance.totals.connecting_passengers = 1;
    g.finance.totals.revenue = 10_000_000;
    for day in 1..=30 {
        g.finance.totals.flights += 1;
        g.clock = Clock::from_state(Duration::from_secs(day * DAY), true, 1).unwrap();
        g.evaluate_milestones();
    }
    let s = g.milestones.as_ref().unwrap();
    for id in [
        "fleet_1",
        "routes_1",
        "passengers_1000",
        "passengers_1000000",
        "connection",
        "revenue_10000000",
        "debt_free",
        "reputation",
    ] {
        assert!(s.reached.contains_key(id), "{id}");
    }
    assert_eq!(s.reached["reputation"], 30 * DAY);
    g.reputation.score = 0;
    g.clock = Clock::from_state(Duration::from_secs(31 * DAY), true, 1).unwrap();
    g.evaluate_milestones();
    assert_eq!(g.milestones.as_ref().unwrap().reputation_days, 0);
    // Network counts public routes once, both endpoints, and only active plans.
    let route = g.routes[0].clone();
    for i in 2..=15 {
        let mut p = g.fleet[0].clone();
        p.id = i;
        g.fleet.push(p);
        let mut r = route.clone();
        r.id = i;
        r.destination_icao = [
            "KJFK", "EGLL", "LFPG", "LEMD", "LIRF", "EHAM", "LOWW", "LSZH", "OMDB", "RJTT", "VHHH",
            "WSSS", "SBGR", "YSSY",
        ][(i - 2) as usize]
            .into();
        g.routes.push(r);
    }
    g.history
        .as_mut()
        .unwrap()
        .taxes
        .push(crate::tax::Assessment {
            time: 365 * DAY,
            profit: 1,
            closing_profit: 1,
            rate_bps: 0,
            amount: 0,
            loss: 0,
        });
    g.clock = Clock::from_state(Duration::from_secs(365 * DAY), true, 1).unwrap();
    g.evaluate_milestones();
    for r in rules().unwrap() {
        let first_day = [
            "fleet_1",
            "routes_1",
            "passengers_1000",
            "passengers_1000000",
            "connection",
            "revenue_10000000",
            "debt_free",
        ]
        .contains(&r.id.as_str());
        let expected = if first_day {
            DAY
        } else if r.id == "reputation" {
            30 * DAY
        } else {
            365 * DAY
        };
        assert_eq!(
            g.milestones.as_ref().unwrap().reached.get(&r.id),
            Some(&expected),
            "{}",
            r.id
        );
    }
    let old = g.milestones.as_ref().unwrap().reached.clone();
    g.fleet.clear();
    g.routes.clear();
    g.finance.totals = Default::default();
    // Lose every instantaneous condition, then sample another midnight.
    g.milestones.as_mut().unwrap().previous_revenue = 0;
    g.milestones.as_mut().unwrap().previous_flights = 0;
    g.clock = Clock::from_state(Duration::from_secs(366 * DAY), true, 1).unwrap();
    g.evaluate_milestones();
    assert_eq!(g.milestones.as_ref().unwrap().reached, old);
    let pending = g.take_milestone_notifications();
    assert_eq!(pending.len(), rules().unwrap().len());
    assert!(g.take_milestone_notifications().is_empty());
}
#[test]
fn milestones_speeds_phases_steps_jump_reload_and_rivals_unchanged() {
    let initial = game();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goals.json");
    let mut reference = initial.clone();
    reference.clock.set_running(true);
    assert!(reference.advance((40 * DAY) as f64));
    let mut disabled = initial.clone();
    disabled.milestones = None;
    disabled.clock.set_running(true);
    assert!(disabled.advance((40 * DAY) as f64));
    assert_eq!(
        serde_json::to_value(&reference.competitors).unwrap(),
        serde_json::to_value(&disabled.competitors).unwrap()
    );
    assert_eq!(reference.finance, disabled.finance);
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        assert!(g.advance(12345.0));
        assert_eq!(g.clock.elapsed(), Duration::ZERO);
        g.clock.set_running(true);
        for day in 0..40 {
            // Restart before departure, airborne, turnaround and midnight.
            for step in [30, 1800, 1800, DAY - 3630] {
                assert!(g.advance(step as f64 / speed as f64));
                if day < 2 {
                    save::save(&path, &g).unwrap();
                    g = save::load(&path).unwrap();
                }
            }
        }
        assert_eq!(g.milestones, reference.milestones);
        let mut jumped = initial.clone();
        while jumped.clock.elapsed().as_secs() < 40 * DAY {
            let remaining = 40 * DAY - jumped.clock.elapsed().as_secs();
            assert!(
                jumped
                    .jump(remaining.min(7 * DAY) as f64)
                    .unwrap()
                    .advanced_seconds
                    > 0
            );
        }
        assert_eq!(jumped.milestones, reference.milestones);
    }
}
#[test]
fn milestones_v32_migration_and_bad_config_or_state() {
    assert!(parse("").is_err());
    assert!(parse("[]").is_err());
    for text in [
        r#"[{"id":"a","kind":"fleet","threshold":0}]"#,
        r#"[{"id":"a","kind":"unknown","threshold":1}]"#,
        r#"[{"id":"a","kind":"fleet","threshold":1},{"id":"a","kind":"fleet","threshold":2}]"#,
    ] {
        assert!(parse(text).is_err());
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    assert!(load_rules(&path).is_err());
    std::fs::write(&path, "broken json").unwrap();
    assert!(load_rules(&path).is_err());
    let mut g = game();
    g.clock.set_running(true);
    assert!(g.advance(12345.0));
    save::save(&path, &g).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["version"] = 32.into();
    value["state"].as_object_mut().unwrap().remove("milestones");
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let mut migrated = save::load(&path).unwrap();
    let s = migrated.milestones.as_ref().unwrap();
    assert_eq!(s.reached["fleet_1"], 12345);
    assert!(!s.reached.contains_key("fleet_5"));
    assert!(migrated.take_milestone_notifications().is_empty());
    assert!(migrated.advance(DAY as f64));
    migrated
        .milestones
        .as_mut()
        .unwrap()
        .reached
        .insert("fleet_5".into(), u64::MAX);
    assert!(save::save(&path, &migrated).is_err());
}

#[test]
fn milestones_two_year_rival_rows_and_ten_year_threshold_calibration() {
    for (country, home) in [("DE", "EDDF"), ("BR", "SBGR"), ("JP", "RJTT")] {
        let mut enabled = Game::default();
        enabled.found("Audit", country, home).unwrap();
        let mut disabled = enabled.clone();
        disabled.milestones = None;
        enabled.clock.set_running(true);
        disabled.clock.set_running(true);
        assert!(enabled.advance((730 * DAY) as f64));
        assert!(disabled.advance((730 * DAY) as f64));
        assert_eq!(
            serde_json::to_value(&enabled.competitors).unwrap(),
            serde_json::to_value(&disabled.competitors).unwrap()
        );
        assert_eq!(enabled.finance, disabled.finance);
        for c in &enabled.competitors {
            println!(
                "MILESTONE RIVAL {home} {} cash={} profit={} fleet={} routes={}",
                c.game.airline.as_ref().unwrap().name,
                c.game.cash(),
                c.game.finance.totals.profit(),
                c.game.fleet.len(),
                c.game.routes.len()
            );
            assert!(c.game.milestones.is_none());
        }
    }
    let mut g = game();
    g.set_ticket_price(1, 90).unwrap();
    g.clock.set_running(true);
    for year in 1..=10 {
        assert!(g.advance((365 * DAY) as f64));
        if [2, 10].contains(&year) {
            println!(
                "MILESTONE CALIBRATION year={year} pax={} revenue={} reached={:?}",
                g.finance.totals.passengers,
                g.finance.totals.revenue,
                g.milestones.as_ref().unwrap().reached
            );
        }
    }
}

#[test]
fn milestones_live_progress_does_not_latch_or_overflow() {
    let mut g = game();
    let values: std::collections::BTreeMap<_, _> = rules()
        .unwrap()
        .iter()
        .zip(g.milestone_values())
        .map(|(r, v)| (r.id.as_str(), v))
        .collect();
    assert_eq!(values["fleet_1"], 1);
    assert_eq!(values["routes_1"], 1);
    assert!(g.milestones.as_ref().unwrap().reached.is_empty());
    g.milestones.as_mut().unwrap().previous_revenue = 100;
    g.milestones.as_mut().unwrap().annual_revenue = 100;
    g.finance.totals.revenue = i64::MAX;
    let annual = rules()
        .unwrap()
        .iter()
        .zip(g.milestone_values())
        .find(|(r, _)| r.kind == Kind::AnnualRevenue)
        .unwrap()
        .1;
    assert_eq!(annual, i64::MAX);
    assert!(g.milestones.as_ref().unwrap().reached.is_empty());
}
