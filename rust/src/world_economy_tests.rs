use crate::{
    game::Game,
    world_economy::{self, Context, Cycle, RULES},
};
const DAY: u64 = 86400;
fn game() -> Game {
    let mut g = Game::default();
    g.found("Cycle audit", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 90).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cycle.json");
    crate::save::save(&path, g).unwrap();
    crate::save::load(&path).unwrap()
}
#[test]
fn world_economy_band_cadence_query_order_and_years() {
    for seed in [0, 1, 6, u64::MAX] {
        let cycle = Cycle {
            seed,
            ..Default::default()
        };
        assert_eq!(cycle.index_at(0), 10000);
        assert_eq!(cycle.index_at(RULES.cadence_seconds - 1), 10000);
        let values: Vec<_> = (0..5200)
            .map(|i| cycle.index_at(i * RULES.cadence_seconds))
            .collect();
        assert!(
            values
                .iter()
                .all(|v| (RULES.minimum..=RULES.maximum).contains(v))
        );
        for i in (0..5200).rev() {
            assert_eq!(
                cycle.index_at(i * RULES.cadence_seconds),
                values[i as usize]
            );
        }
        // Adjacent weeks drift by at most one shock plus mean reversion.
        assert!(values.windows(2).all(|w| (w[1] - w[0]).abs()
            <= RULES.volatility
                + (10000 - RULES.minimum).max(RULES.maximum - 10000)
                    / RULES.mean_reversion_divisor));
    }
}
#[test]
fn world_economy_segments_cargo_and_zero_reactions_preserve_quotes() {
    let g = game();
    let route = &g.routes[0];
    let time = 4 * crate::demand::YEAR_SECONDS + 123;
    let context = Context {
        cycle: g.world_economy,
        quoted_at: time,
    };
    let index = context.index(Some(time));
    let season = crate::demand::season_factors(time);
    let factors = world_economy::factors(Some(context), Some(time));
    assert_eq!(
        factors.business,
        season.business * world_economy::factor(index, RULES.business_reaction)
    );
    assert_eq!(
        factors.leisure,
        season.leisure * world_economy::factor(index, RULES.leisure_reaction)
    );
    assert!(
        (world_economy::factor(index, RULES.business_reaction) - 1.0).abs()
            > (world_economy::factor(index, RULES.leisure_reaction) - 1.0).abs()
    );
    let raw = crate::demand::market_at(route, route.ticket_eur, time).unwrap();
    let scaled =
        crate::demand::market_in_economy(route, route.ticket_eur, context, Some(time)).unwrap();
    assert!(
        (scaled.business - raw.business * world_economy::factor(index, RULES.business_reaction))
            .abs()
            < 1e-10
    );
    assert!(
        (scaled.leisure - raw.leisure * world_economy::factor(index, RULES.leisure_reaction)).abs()
            < 1e-10
    );
    let mut neutral = g.market_routes();
    neutral.retain(|o| crate::demand::same_market(route, &o.route));
    for o in &mut neutral {
        o.economy = None;
        o.seats = 10000;
    }
    let legacy = crate::economy::quote_at_time(route, "atr72_600", &neutral, 80, time).unwrap();
    let mut active = neutral.clone();
    for o in &mut active {
        o.economy = Some(context);
    }
    world_economy::ZERO_REACTIONS.with(|v| v.set(true));
    let zero = crate::economy::quote_at_time(route, "atr72_600", &active, 80, time).unwrap();
    world_economy::ZERO_REACTIONS.with(|v| v.set(false));
    assert_eq!(legacy, zero);
    let mut cargo = crate::cargo::Market::new(route, "atr72_600", &active).unwrap();
    let mut plain = crate::cargo::Market::new(route, "atr72_600", &neutral).unwrap();
    let loads = vec![0; active.len()];
    let expected =
        plain.estimate(&loads) as f64 * world_economy::factor(index, RULES.cargo_reaction);
    assert!((cargo.estimate_at(&loads, Some(time)) as f64 - expected).abs() < 2.0);
    // Transfer cache dates must distinguish years and a migrated intraday boundary.
    assert_ne!(
        world_economy::transfer_time(Some(context), Some(time)),
        world_economy::transfer_time(Some(context), Some(time + crate::demand::YEAR_SECONDS))
    );
    let offset = Context {
        cycle: Cycle {
            started_at: 123,
            ..Default::default()
        },
        quoted_at: 0,
    };
    assert_ne!(
        world_economy::transfer_time(Some(offset), Some(7 * DAY + 122)),
        world_economy::transfer_time(Some(offset), Some(7 * DAY + 123))
    );
}
#[test]
fn world_economy_steps_speeds_pause_jumps_restart_and_notifications() {
    let initial = game();
    let mut fast = initial.clone();
    assert!(fast.advance(2.0 * crate::demand::YEAR_SECONDS as f64));
    for speed in [1, 4, 16] {
        let mut daily = initial.clone();
        daily.clock.set_speed(speed);
        daily.clock.set_running(false);
        assert!(daily.advance(100000.0));
        assert_eq!(daily.clock.elapsed().as_secs(), 0);
        daily.clock.set_running(true);
        for day in 0..730 {
            assert!(daily.advance(DAY as f64 / f64::from(speed)));
            if day == 364 {
                daily = restart(&daily);
            }
        }
        daily.clock.set_speed(1);
        assert_eq!(
            serde_json::to_value(&fast).unwrap(),
            serde_json::to_value(&daily).unwrap()
        );
    }
    let now = fast.clock.elapsed().as_secs();
    let notifications = fast.world_economy.take_notifications(now);
    assert!(!notifications.is_empty());
    assert!(fast.world_economy.take_notifications(now).is_empty());
    let mut loaded = restart(&fast);
    assert!(loaded.world_economy.take_notifications(now).is_empty());
}
#[test]
fn world_economy_v33_migration_is_neutral_for_first_day_and_shared() {
    let mut g = game();
    assert!(g.advance(123.0 * DAY as f64 + 4567.0));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    crate::save::save(&path, &g).unwrap();
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    old["version"] = 33.into();
    old["state"]
        .as_object_mut()
        .unwrap()
        .remove("world_economy");
    for c in old["state"]["competitors"].as_array_mut().unwrap() {
        c["game"].as_object_mut().unwrap().remove("world_economy");
    }
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let mut loaded = crate::save::load(&path).unwrap();
    let now = loaded.clock.elapsed().as_secs();
    assert_eq!(loaded.world_economy.started_at, now);
    assert_eq!(loaded.world_economy.index_at(now + DAY), 10000);
    assert_eq!(loaded.finance, g.finance);
    assert_eq!(loaded.routes, g.routes);
    assert!(
        loaded
            .competitors
            .iter()
            .all(|c| c.game.world_economy == loaded.world_economy)
    );
    let mut replay = restart(&loaded);
    assert!(loaded.advance(100.0 * DAY as f64));
    assert!(replay.advance(100.0 * DAY as f64));
    assert_eq!(
        serde_json::to_value(&loaded).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
}

#[test]
fn world_economy_weak_start_two_year_audit() {
    let mut g = game();
    g.world_economy.seed = 349;
    for c in &mut g.competitors {
        c.game.world_economy = g.world_economy;
    }
    let mut lowest = 10000;
    let mut sum_index = 0_i64;
    let mut distress_days = 0;
    for _ in 0..730 {
        assert!(g.advance(DAY as f64));
        let index = g.world_economy.index_at(g.clock.elapsed().as_secs());
        lowest = lowest.min(index);
        sum_index += index;
        distress_days += usize::from(g.distressed());
        assert_eq!(g.competitors.len(), 2);
        assert!(g.competitors.iter().all(|c| !c.game.distressed()
            && !c.game.insolvent()
            && c.game.resolution.deadline.is_none()));
    }
    println!(
        "WEAK_START min_index={lowest} mean_percent={:.3} player_cash={} profit={} distress_days={distress_days}",
        sum_index as f64 / 730.0 / 100.0,
        g.cash(),
        g.finance.totals.profit()
    );
    for c in &g.competitors {
        println!(
            "WEAK_RIVAL {} cash={} fleet={} routes={}",
            c.game.airline.as_ref().unwrap().name,
            c.game.cash(),
            c.game.fleet.len(),
            c.game.routes.len()
        );
    }
}

#[test]
fn world_economy_phase_changes_do_not_add_attention_or_stop_jumps() {
    let mut g = Game::default();
    g.found("Quiet macro", "DE", "EDDF").unwrap();
    g.competitors.clear();
    let attention = g.attention_snapshot();
    let mut reference = g.clone();
    reference.clock.set_running(true);
    assert!(reference.advance(364.0 * DAY as f64));
    for _ in 0..52 {
        let result = g.jump(7.0 * DAY as f64).unwrap();
        assert_eq!(result.advanced_seconds, 7 * DAY);
    }
    assert_eq!(
        g.world_economy.index_at(364 * DAY),
        reference.world_economy.index_at(364 * DAY)
    );
    assert_eq!(g.attention_snapshot().len(), attention.len());
    assert!(!g.world_economy.take_notifications(364 * DAY).is_empty());
}

impl Game {
    /// Older subsystem migration fixtures also acquire the v34 neutral timeline.
    pub(crate) fn migrate_world_economy_fixture(&mut self) {
        self.world_economy = Cycle {
            seed: self.fuel_market.seed ^ 0x45434f4e4f4d59,
            started_at: self.clock.elapsed().as_secs(),
            notified_period: 0,
        };
        for c in &mut self.competitors {
            c.game.world_economy = self.world_economy;
        }
    }
}

#[test]
fn world_economy_intraday_transfer_reservations_never_overbook() {
    let mut g = game();
    g.set_ticket_price(1, 71).unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.create_route("EDDF", "EDDM", 2).unwrap();
    g.set_ticket_price(2, 134).unwrap();
    g.staff_up();
    g.schedule(2, 1, 120).unwrap();
    g.airline.as_mut().unwrap().capital_eur += 100_000_000_000;
    g.history.as_mut().unwrap().opening_balance += 100_000_000_000;
    // Before this fix a rival's day-2232 local load plus a cached daily
    // connection put 153 passengers in 152 seats and made the save invalid.
    assert!(g.advance(7.0 * crate::demand::YEAR_SECONDS as f64));
    g.validate_history().unwrap();
    g.validate_competitors().unwrap();
    assert!(
        g.competitors
            .iter()
            .any(|c| c.game.finance.totals.connecting_passengers > 0)
    );
    restart(&g).validate_history().unwrap();
}

#[test]
fn world_economy_rejects_future_timelines_cursors_and_split_worlds() {
    let g = game();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid-cycle.json");
    crate::save::save(&path, &g).unwrap();
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for kind in 0..5 {
        let mut invalid = original.clone();
        match kind {
            0 => invalid["state"]["world_economy"]["started_at"] = 1.into(),
            1 => invalid["state"]["world_economy"]["notified_period"] = 1.into(),
            2 => invalid["state"]["competitors"][0]["game"]["world_economy"]["seed"] = 1.into(),
            3 => {
                invalid["state"]
                    .as_object_mut()
                    .unwrap()
                    .remove("world_economy");
            }
            _ => {
                invalid["state"]["competitors"][0]["game"]["world_economy"]["notified_period"] =
                    1.into()
            }
        }
        std::fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(crate::save::load(&path).is_err(), "kind={kind}");
    }
}
