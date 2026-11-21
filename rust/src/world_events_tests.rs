use crate::{
    demand,
    game::Game,
    world_economy::{Context, Cycle},
    world_events::{self, DAY, Place, RULES, Type},
};

fn cycle(seed: u64) -> Cycle {
    Cycle {
        seed,
        ..Default::default()
    }
}
fn game() -> Game {
    let mut g = Game::default();
    g.found("Event audit", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.airline.as_mut().unwrap().capital_eur += 100_000_000;
    g.history.as_mut().unwrap().opening_balance += 100_000_000;
    for (id, destination) in [(1, "EDDM"), (2, "LTFM")] {
        g.buy_aircraft("a320_200").unwrap();
        g.staff_up();
        g.create_route("EDDF", destination, id).unwrap();
        g.set_ticket_price(id, if id == 1 { 90 } else { 120 })
            .unwrap();
        g.schedule(id, 1, 240).unwrap();
    }
    g.clock.set_running(true);
    g
}
fn with_seed(mut g: Game, seed: u64) -> Game {
    g.world_economy.seed = seed;
    for c in &mut g.competitors {
        c.game.world_economy = g.world_economy;
    }
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn normal(g: &Game) -> serde_json::Value {
    let mut v = state(g);
    v["clock"]["speed_index"] = 0.into();
    v
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.json");
    crate::save::save(&path, g).unwrap();
    crate::save::load(&path).unwrap()
}
fn advance_to(g: &mut Game, time: u64) -> bool {
    g.clock.set_running(true);
    let ok = g.advance((time - g.clock.elapsed().as_secs()) as f64);
    g.clock.set_running(false);
    ok
}
fn enable() {
    world_events::DISABLED.with(|v| v.set(false));
}
struct Disabled;
impl Disabled {
    fn on() -> Self {
        world_events::DISABLED.with(|v| v.set(true));
        Self
    }
}
impl Drop for Disabled {
    fn drop(&mut self) {
        world_events::DISABLED.with(|v| v.set(false));
    }
}
fn active_at(c: &Cycle, time: u64) -> Vec<world_events::Event> {
    world_events::visible_at(c, time)
        .into_iter()
        .filter(|e| e.is_active(time))
        .collect()
}
/// First seed whose events touch EDDF or EDDM (the player's home market) during `days`.
fn seed_touching(icaos: &[&str], days: std::ops::Range<u64>) -> (u64, u64) {
    for seed in 1..5000 {
        let c = cycle(seed);
        for day in days.clone() {
            let active = active_at(&c, day * DAY);
            if !active.is_empty()
                && active
                    .iter()
                    .all(|e| e.start_day >= 2 && icaos.iter().any(|i| e.touches(i)))
            {
                return (seed, day);
            }
        }
    }
    panic!("no seed");
}

#[test]
fn events_are_stable_for_a_seed_and_independent_of_query_order() {
    enable();
    for seed in [0, 1, 7, u64::MAX] {
        let c = cycle(seed);
        let days: Vec<u64> = (0..3650).step_by(3).collect();
        let forward: Vec<_> = days
            .iter()
            .map(|d| world_events::visible_at(&c, d * DAY + 77))
            .collect();
        let backward: Vec<_> = days
            .iter()
            .rev()
            .map(|d| world_events::visible_at(&c, d * DAY + 77))
            .collect();
        assert!(forward.iter().eq(backward.iter().rev()));
        // The same event has the same identity and dates however it was reached.
        for pair in forward.windows(2) {
            for e in &pair[0] {
                if let Some(same) = pair[1].iter().find(|o| o.id == e.id) {
                    assert_eq!(e, same);
                }
            }
        }
        assert_ne!(
            forward,
            (0..3650u64)
                .step_by(3)
                .map(|d| world_events::visible_at(&cycle(seed.wrapping_add(1)), d * DAY + 77))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn events_have_configured_shapes_and_day_boundaries() {
    enable();
    let c = cycle(11);
    let mut seen = (0, 0);
    for day in 0..3650u64 {
        for e in world_events::visible_at(&c, day * DAY) {
            let days = e.end_day - e.start_day;
            let (kind, warning) = match e.kind {
                Type::Boom => {
                    seen.0 += 1;
                    assert!(matches!(e.place, Place::Airport(_)));
                    (&RULES.boom, 21)
                }
                Type::Slump => {
                    seen.1 += 1;
                    assert!(matches!(e.place, Place::Country(_)));
                    (&RULES.slump, 0)
                }
            };
            assert!((kind.min_days..=kind.max_days).contains(&days));
            assert_eq!(e.announced_at(), e.start_day.saturating_sub(warning) * DAY);
            assert!(e.announced_at() <= day * DAY);
            assert!(!e.is_active(e.starts_at().saturating_sub(1)));
            assert!(e.is_active(e.starts_at()));
            assert!(e.is_active(e.ends_at() - 1));
            assert!(!e.is_active(e.ends_at()));
        }
    }
    assert!(seen.0 > 100 && seen.1 > 30, "{seen:?}");
    // Factor flips exactly at midnight for both endpoints.
    let (seed, day) = seed_touching(&["EDDF", "EDDM", "LTFM"], 10..400);
    let c = cycle(seed);
    let e = active_at(&c, day * DAY)[0];
    let icao = match e.place {
        Place::Airport(a) => a,
        Place::Country(_) => "EDDF",
    };
    let before = world_events::route_factors(&c, e.starts_at().saturating_sub(1), icao, "EDDM");
    let at = world_events::route_factors(&c, e.starts_at(), icao, "EDDM");
    assert!(before.is_none_or(|b| b != at.unwrap()));
    assert!(at.is_some());
    let end = world_events::route_factors(&c, e.ends_at(), icao, "EDDM");
    assert!(end.is_none_or(|f| Some(f) != at));
}

#[test]
fn events_are_few_and_weighted_towards_large_airports() {
    enable();
    let c = cycle(3);
    let mut peak = 0;
    let mut total = 0;
    let mut counts = std::collections::HashMap::new();
    for day in 0..7300u64 {
        let visible = world_events::visible_at(&c, day * DAY);
        peak = peak.max(visible.len());
        total += visible.iter().filter(|e| e.is_active(day * DAY)).count();
        for e in visible.iter().filter(|e| e.start_day == day) {
            if let Place::Airport(a) = e.place {
                *counts.entry(a).or_insert(0u32) += 1;
            }
        }
    }
    let mean = total as f64 / 7300.0;
    assert!((1.0..5.0).contains(&mean), "{mean}");
    assert!(peak <= 20, "{peak}");
    let catalog = crate::airports::catalog().unwrap();
    let large: u32 = catalog
        .airports
        .iter()
        .filter(|a| a.market_size >= 1.0)
        .map(|a| counts.get(a.icao.as_str()).copied().unwrap_or(0))
        .sum();
    let small: u32 = catalog
        .airports
        .iter()
        .filter(|a| a.market_size < 0.6)
        .map(|a| counts.get(a.icao.as_str()).copied().unwrap_or(0))
        .sum();
    let n_large = catalog
        .airports
        .iter()
        .filter(|a| a.market_size >= 1.0)
        .count() as f64;
    let n_small = catalog
        .airports
        .iter()
        .filter(|a| a.market_size < 0.6)
        .count() as f64;
    assert!(large as f64 / n_large > 2.0 * small as f64 / n_small.max(1.0));
}

#[test]
fn event_factor_is_in_the_shared_market_for_local_connecting_and_freight_demand() {
    enable();
    let (seed, day) = seed_touching(&["LTFM"], 5..300);
    let g = with_seed(game(), seed);
    let time = day * DAY + 3600;
    let context = Context {
        cycle: g.world_economy,
        quoted_at: time,
    };
    let events = active_at(&g.world_economy, time);
    assert!(!events.is_empty());
    let f = events
        .iter()
        .map(|e| e.factors())
        .fold((1.0, 1.0, 1.0), |a, f| {
            (a.0 * f.business, a.1 * f.leisure, a.2 * f.cargo)
        });
    let ltfm = &g.routes[1];
    let on = demand::market_in_economy(ltfm, ltfm.ticket_eur, context, Some(time)).unwrap();
    let off = {
        let _d = Disabled::on();
        demand::market_in_economy(ltfm, ltfm.ticket_eur, context, Some(time)).unwrap()
    };
    assert!((on.business / off.business - f.0).abs() < 1e-9);
    assert!((on.leisure / off.leisure - f.1).abs() < 1e-9);
    // A route elsewhere is untouched, and annual planning (no time) ignores events.
    let eddm = &g.routes[0];
    let on_eddm = demand::market_in_economy(eddm, 90, context, Some(time)).unwrap();
    let plan_on = demand::market_in_economy(ltfm, 0, context, None).unwrap();
    let (off_eddm, plan_off) = {
        let _d = Disabled::on();
        (
            demand::market_in_economy(eddm, 90, context, Some(time)).unwrap(),
            demand::market_in_economy(ltfm, 0, context, None).unwrap(),
        )
    };
    assert_eq!(on_eddm.business, off_eddm.business);
    assert_eq!(plan_on.business, plan_off.business);
    assert_eq!(plan_on.leisure, plan_off.leisure);
    // Freight: the same snapshot, scaled by the cargo reaction.
    let offers = g.market_routes();
    let mut offers: Vec<_> = offers
        .into_iter()
        .filter(|o| demand::same_market(ltfm, &o.route))
        .collect();
    for o in &mut offers {
        o.economy = Some(context);
    }
    let loads = vec![0; offers.len()];
    let mut cargo = crate::cargo::Market::new(ltfm, "a320_200", &offers).unwrap();
    let on_kg = cargo.estimate_at(&loads, Some(time));
    let off_kg = {
        let _d = Disabled::on();
        let mut cargo = crate::cargo::Market::new(ltfm, "a320_200", &offers).unwrap();
        cargo.estimate_at(&loads, Some(time))
    };
    if f.2 > 1.0 {
        assert!(on_kg >= off_kg);
    } else {
        assert!(on_kg <= off_kg);
    }
    assert_ne!(f.2, 1.0);
    // Connecting passengers EDDM→LTFM via the hub inherit the same market. Transfer
    // quantisation can hide a small change, so look across several worlds.
    let mut changed = 0;
    for seed in 1..400 {
        let c = cycle(seed);
        let Some(day) = (5..200).find(|d| {
            let a = active_at(&c, d * DAY);
            !a.is_empty() && a.iter().all(|e| e.touches("LTFM"))
        }) else {
            continue;
        };
        let mut g = with_seed(game(), seed);
        g.clock.set_running(false);
        let time = day * DAY + 100;
        let supply = g.market_routes();
        // A distinct snapshot keeps the memoised transfer network of the "on" run apart.
        let mut supply_off = supply.clone();
        for o in &mut supply_off {
            o.economy.as_mut().unwrap().quoted_at += 1;
        }
        let on_c = demand::estimate_at(&g.routes[0], 180, &supply, time).unwrap();
        let off_c = {
            let _d = Disabled::on();
            demand::estimate_at(&g.routes[0], 180, &supply_off, time).unwrap()
        };
        if on_c.daily_connecting != off_c.daily_connecting {
            changed += 1;
            // Local passengers keep seat priority, so the direction is not fixed.
        }
        g.clock.set_running(true);
    }
    assert!(changed >= 3, "{changed}");
}

#[test]
fn disabled_or_zero_reaction_events_are_exactly_neutral() {
    enable();
    let (seed, _) = seed_touching(&["EDDF", "EDDM", "LTFM"], 5..120);
    let initial = with_seed(game(), seed);
    let mut off = initial.clone();
    {
        let _d = Disabled::on();
        assert!(off.advance(120.0 * DAY as f64));
    }
    let mut zero = initial.clone();
    world_events::ZERO_REACTIONS.with(|v| v.set(true));
    assert!(zero.advance(120.0 * DAY as f64));
    world_events::ZERO_REACTIONS.with(|v| v.set(false));
    assert_eq!(state(&off), state(&zero));
    let mut on = initial.clone();
    assert!(on.advance(120.0 * DAY as f64));
    assert_ne!(state(&off), state(&on));
}

#[test]
fn announcement_is_delivered_once_across_jumps_and_save_load() {
    enable();
    let mut found = None;
    'seeds: for seed in 1..400 {
        let c = cycle(seed);
        for day in 40..300 {
            for e in world_events::visible_at(&c, day * DAY) {
                if e.kind == Type::Boom
                    && e.announced_at() == day * DAY
                    && e.touches("EDDF")
                    && active_at(&c, 0).is_empty()
                {
                    found = Some((seed, e));
                    break 'seeds;
                }
            }
        }
    }
    let (seed, e) = found.expect("boom at the home airport");
    let mut g = with_seed(game(), seed);
    g.clock.set_running(false);
    assert!(advance_to(&mut g, e.announced_at() - 3 * DAY));
    let listed = |g: &Game| {
        g.attention_snapshot()
            .iter()
            .filter(|a| a.kind == "event" && a.order_id == e.id)
            .count()
    };
    assert_eq!(listed(&g), 0);
    let mut stopped = None;
    for _ in 0..10 {
        let r = g.jump(DAY as f64).unwrap();
        if !r.reason.is_empty() {
            stopped = Some(r);
            break;
        }
    }
    let stopped = stopped.expect("jump stops on the announcement");
    assert!(stopped.reason.contains("Ereignis"), "{}", stopped.reason);
    assert_eq!(g.clock.elapsed().as_secs(), e.announced_at());
    assert_eq!(listed(&g), 1);
    let mut loaded = restart(&g);
    assert_eq!(listed(&loaded), 1);
    let after = loaded.jump(DAY as f64).unwrap();
    assert!(
        !after.reason.contains(&e.place_name()),
        "repeated: {}",
        after.reason
    );
    assert_eq!(listed(&loaded), 1);
    // Starting day: still one entry; after it ends: gone.
    assert!(advance_to(&mut loaded, e.ends_at()));
    assert_eq!(listed(&loaded), 0);
}

#[test]
fn events_match_across_speeds_step_sizes_jumps_and_restarts() {
    enable();
    let (seed, _) = seed_touching(&["EDDF", "EDDM", "LTFM"], 5..60);
    let initial = with_seed(game(), seed);
    let mut expected = initial.clone();
    assert!(expected.advance(90.0 * DAY as f64));
    for speed in [1, 4, 16] {
        let mut replay = initial.clone();
        replay.clock.set_speed(speed);
        for _ in 0..90 * 24 {
            assert!(replay.advance(3600.0 / f64::from(speed)));
        }
        assert_eq!(normal(&replay), normal(&expected));
    }
    let mut jumped = initial.clone();
    jumped.jump(90.0 * DAY as f64).ok();
    let mut stepped = initial.clone();
    for _ in 0..3 {
        assert!(stepped.advance(30.0 * DAY as f64));
        stepped = restart(&stepped);
    }
    assert_eq!(normal(&stepped), normal(&expected));
    assert_eq!(
        state(&jumped).get("world_economy"),
        state(&expected).get("world_economy")
    );
}

#[test]
fn v42_saves_get_no_retroactive_events() {
    enable();
    let mut g = with_seed(game(), 1);
    // Find a seed that has events running at the save moment.
    let (seed, day) = seed_touching(&["EDDF", "EDDM", "LTFM"], 60..200);
    g = with_seed(g, seed);
    assert!(g.advance((day * DAY + 5000) as f64));
    let now = g.clock.elapsed().as_secs();
    let before = world_events::visible_at(&g.world_economy, now);
    assert!(!before.is_empty());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    crate::save::save(&path, &g).unwrap();
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    old["version"] = 42.into();
    old["state"]["world_economy"]
        .as_object_mut()
        .unwrap()
        .remove("events_from");
    for c in old["state"]["competitors"].as_array_mut().unwrap() {
        c["game"]["world_economy"]
            .as_object_mut()
            .unwrap()
            .remove("events_from");
    }
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let loaded = crate::save::load(&path).unwrap();
    assert_eq!(loaded.world_economy.events_from, now);
    assert!(
        loaded
            .competitors
            .iter()
            .all(|c| c.game.world_economy == loaded.world_economy)
    );
    assert!(active_at(&loaded.world_economy, now).is_empty());
    for t in (0..400).map(|d| now + d * DAY) {
        assert!(
            world_events::visible_at(&loaded.world_economy, t)
                .iter()
                .all(|e| e.announced_at() >= now)
        );
    }
    // Something is eventually announced after loading, and nothing else changed.
    assert!(
        (0..800)
            .any(|d| !world_events::visible_at(&loaded.world_economy, now + d * DAY).is_empty())
    );
    assert_eq!(loaded.finance, g.finance);
    assert_eq!(loaded.routes, g.routes);
    assert_eq!(loaded.cash(), g.cash());
    assert!(
        loaded
            .attention_snapshot()
            .iter()
            .all(|a| a.kind != "event")
    );
}

#[test]
fn annual_planning_ignores_events_for_ai_airlines() {
    enable();
    let (seed, day) = seed_touching(&["EDDF", "EDDM"], 5..300);
    let g = with_seed(game(), seed);
    let route = g.routes[0].clone();
    let context = Context {
        cycle: g.world_economy,
        quoted_at: day * DAY,
    };
    let on = demand::market_in_economy(&route, 0, context, None).unwrap();
    let off = {
        let _d = Disabled::on();
        demand::market_in_economy(&route, 0, context, None).unwrap()
    };
    assert_eq!((on.business, on.leisure), (off.business, off.leisure));
    let timed = demand::market_in_economy(&route, 0, context, Some(day * DAY)).unwrap();
    assert_ne!(timed.total(), off.total() * 0.0 + timed.business + 1.0);
}

#[test]
fn normal_eddf_rivals_two_and_ten_year_comparison_with_events_on_and_off() {
    enable();
    let mut new = Game::default();
    new.found("EDDF baseline", "DE", "EDDF").unwrap();
    new.clock.set_running(true);
    let mut baseline = new.clone();
    for year in 1..=10 {
        assert!(new.advance(365.0 * 86400.0));
        {
            let _d = Disabled::on();
            assert!(baseline.advance(365.0 * 86400.0));
        }
        if year == 2 || year == 10 {
            for (a, b) in new.competitors.iter().zip(&baseline.competitors) {
                println!(
                    "EVENTS year={year} airline={} fleet={}/{} cash={}/{} insolvent={}/{} distressed={}/{}",
                    a.game.airline.as_ref().unwrap().name,
                    a.game.fleet.len(),
                    b.game.fleet.len(),
                    a.game.cash(),
                    b.game.cash(),
                    a.game.insolvent(),
                    b.game.insolvent(),
                    a.game.distressed(),
                    b.game.distressed()
                );
                assert!(!a.game.insolvent() && !a.game.distressed());
                assert!(a.game.cash() > b.game.cash() / 2 && a.game.cash() < b.game.cash() * 3 / 2);
                assert!(a.game.fleet.len().abs_diff(b.game.fleet.len()) <= 2);
            }
            assert_eq!(state(&restart(&new)), state(&new));
        }
    }
}
