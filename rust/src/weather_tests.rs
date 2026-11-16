use crate::{
    airports,
    game::Game,
    weather::{self, DAY, RULES, Weather},
};

fn w(seed: u64) -> Weather {
    Weather { seed, from: 0 }
}
fn enable() {
    weather::DISABLED.with(|v| v.set(false));
}
struct Disabled;
impl Disabled {
    fn on() -> Self {
        weather::DISABLED.with(|v| v.set(true));
        Self
    }
}
impl Drop for Disabled {
    fn drop(&mut self) {
        weather::DISABLED.with(|v| v.set(false));
    }
}
fn game() -> Game {
    let mut g = Game::default();
    g.found("Weather audit", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.airline.as_mut().unwrap().capital_eur += 100_000_000;
    g.history.as_mut().unwrap().opening_balance += 100_000_000;
    for (id, destination) in [(1, "EDDM"), (2, "LTFM")] {
        g.buy_aircraft("a320_200").unwrap();
        g.staff_up();
        g.create_route("EDDF", destination, id).unwrap();
        g.set_ticket_price(id, if id == 1 { 90 } else { 120 }).unwrap();
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
    let path = dir.path().join("weather.json");
    crate::save::save(&path, g).unwrap();
    crate::save::load(&path).unwrap()
}
fn clear_weather(g: &mut Game) {
    for r in &mut g.routes {
        if let Some(c) = r.plan.as_mut().and_then(|p| p.disruption.as_mut()) {
            c.weather = None;
        }
    }
}
/// First seed with weather at EDDF on the first scheduled departure day.
fn seed_with_weather_at_start() -> (u64, u64) {
    for seed in 1..5000 {
        let c = w(seed);
        for day in 0..3 {
            let d = weather::delay_seconds(&c, "EDDF", day * DAY + 3600);
            if d > 0 {
                return (seed, d);
            }
        }
    }
    panic!("no seed");
}

#[test]
fn disabled_weather_is_exactly_neutral() {
    // Tests run with weather off unless enabled explicitly.
    for seed in [1, 2, 3] {
        assert!(weather::visible_at(&w(seed), 5 * DAY).is_empty());
        assert_eq!(weather::delay_seconds(&w(seed), "EDDF", 5 * DAY), 0);
    }
    let mut with = with_seed(game(), 7);
    let mut without = with.clone();
    clear_weather(&mut without);
    assert!(with.routes.iter().all(|r| r.plan.as_ref().unwrap().disruption.as_ref().unwrap().weather.is_some()));
    assert!(with.advance(120.0 * DAY as f64) && without.advance(120.0 * DAY as f64));
    assert_eq!(state(&with)["finance"], state(&without)["finance"]);
    assert_eq!(state(&with)["fleet"], state(&without)["fleet"]);
    assert_eq!(with.cash(), without.cash());
    assert!(with.attention_snapshot().iter().all(|a| a.kind != "weather"));
}

#[test]
fn episodes_are_pure_functions_with_configured_shapes() {
    enable();
    let catalog = airports::catalog().unwrap();
    for seed in [0, 1, 7, u64::MAX] {
        let c = w(seed);
        let days: Vec<u64> = (0..1460).step_by(2).collect();
        let forward: Vec<_> = days.iter().map(|d| weather::visible_at(&c, d * DAY + 77)).collect();
        let backward: Vec<_> = days.iter().rev().map(|d| weather::visible_at(&c, d * DAY + 80000)).collect();
        assert!(forward.iter().eq(backward.iter().rev()));
        for (i, day) in days.iter().enumerate() {
            for e in &forward[i] {
                let a = catalog.airports.iter().find(|a| a.icao == e.icao).unwrap();
                let k = e.kind;
                assert!((k.min_days..=k.max_days).contains(&(e.end_day - e.start_day)));
                assert!((k.min_delay_seconds..=k.max_delay_seconds).contains(&e.delay_seconds));
                assert!((k.min_abs_latitude..=k.max_abs_latitude).contains(&a.latitude.abs()));
                assert_eq!(e.announced_at(), e.start_day.saturating_sub(RULES.warning_days) * DAY);
                assert!(e.announced_at() <= day * DAY && day * DAY < e.ends_at());
                assert!(e.is_active(e.starts_at()) && !e.is_active(e.ends_at()));
                let shift = if a.latitude < 0.0 { RULES.southern_shift_days } else { 0 };
                let doy = (e.start_day + 365 - shift) % 365;
                let (s, t) = (k.season_start_day, k.season_end_day);
                assert!(if s <= t { (s..t).contains(&doy) } else { doy >= s || doy < t });
                // Delay applies on active days only, at midnight boundaries.
                if e.is_active(day * DAY) {
                    assert!(weather::delay_seconds(&c, e.icao, day * DAY) >= e.delay_seconds);
                }
            }
        }
        assert_ne!(
            forward,
            days.iter().map(|d| weather::visible_at(&w(seed.wrapping_add(1)), d * DAY + 77)).collect::<Vec<_>>()
        );
    }
}

#[test]
fn weather_follows_latitude_and_season_and_stays_mild() {
    enable();
    let c = w(3);
    let mut bad_days = std::collections::HashMap::new();
    for day in 0..3650u64 {
        for icao in ["EDDF", "SBGR", "WSSS", "YSSY", "OMDB"] {
            if weather::delay_seconds(&c, icao, day * DAY) > 0 {
                *bad_days.entry((icao, day % 365 / 91)).or_insert(0u32) += 1;
            }
        }
    }
    let total = |icao: &str| bad_days.iter().filter(|((i, _), _)| *i == icao).map(|(_, n)| n).sum::<u32>();
    // Northern airport: far more bad days in winter quarters than summer.
    let eddf_winter = bad_days.get(&("EDDF", 0)).copied().unwrap_or(0) + bad_days.get(&("EDDF", 3)).copied().unwrap_or(0);
    let eddf_summer = bad_days.get(&("EDDF", 1)).copied().unwrap_or(0) + bad_days.get(&("EDDF", 2)).copied().unwrap_or(0);
    assert!(eddf_winter > eddf_summer * 2, "{bad_days:?}");
    for icao in ["EDDF", "SBGR", "WSSS", "YSSY", "OMDB"] {
        let share = f64::from(total(icao)) / 3650.0;
        println!("WEATHER bad-day share {icao}: {share:.3}");
        assert!(share < 0.12, "{icao} {share}");
    }
    assert!(total("EDDF") > 50, "{}", total("EDDF"));
}

#[test]
fn weather_is_derived_once_per_day() {
    enable();
    let c = w(5);
    weather::BUILDS.with(|b| b.set(0));
    for day in 0..40u64 {
        for s in (0..86400).step_by(900) {
            weather::visible_at(&c, day * DAY + s);
        }
    }
    assert_eq!(weather::BUILDS.with(|b| b.get()), 40);
}

#[test]
fn weather_delays_a_departure_and_the_rotation_behind_it() {
    enable();
    let (seed, delay) = seed_with_weather_at_start();
    let mut g = with_seed(game(), seed);
    let mut calm = g.clone();
    clear_weather(&mut calm);
    let model = "a320_200";
    let route = |g: &Game| g.routes[0].clone();
    let first = route(&g).plan.as_ref().unwrap().first_departure;
    let day_delay = weather::delay_seconds(&w(seed), "EDDF", first);
    // Scheduled departures that day pick up the weather delay at EDDF.
    let hit = crate::disruptions::project(&route(&g), model, first + 1).unwrap().state;
    let none = crate::disruptions::project(&route(&calm), model, first + 1).unwrap().state;
    if day_delay > 0 {
        assert!(hit.delay_seconds >= none.delay_seconds + day_delay.min(delay));
        assert_eq!(hit.weather_seconds, day_delay.min(hit.delay_seconds));
        assert_eq!(none.weather_seconds, 0);
        let landing = |g: &Game| crate::disruptions::landing_at(&route(g), model, 0).unwrap();
        assert_eq!(landing(&g), landing(&calm) + day_delay);
        // Recovery: the return leg leaves later too, or the rotation was slack.
        let back = |g: &Game| crate::disruptions::landing_at(&route(g), model, 1).unwrap();
        assert!(back(&g) >= back(&calm));
    }
    // Delay flows into punctuality totals.
    assert!(g.advance(30.0 * DAY as f64) && calm.advance(30.0 * DAY as f64));
    assert!(g.finance.totals.delay_seconds >= calm.finance.totals.delay_seconds);
}

#[test]
fn player_and_ai_share_airports_days_and_delays() {
    enable();
    let mut g = Game::default();
    g.found("Same sky", "DE", "EDDF").unwrap();
    assert!(!g.competitors.is_empty());
    assert!(g.advance(400.0 * DAY as f64));
    let mine = Weather::of(&g.world_economy);
    for c in &g.competitors {
        assert_eq!(Weather::of(&c.game.world_economy), mine);
        for r in &c.game.routes {
            if let Some(cursor) = r.plan.as_ref().and_then(|p| p.disruption.as_ref()) {
                assert_eq!(cursor.weather, Some(mine));
            }
        }
    }
    let ai_routes: Vec<_> = g.competitors.iter().flat_map(|c| c.game.routes.iter()).filter(|r| r.plan.is_some()).collect();
    assert!(!ai_routes.is_empty());
    let mut hit = 0;
    for r in ai_routes {
        let cursor = r.plan.as_ref().unwrap().disruption.as_ref().unwrap();
        for day in 400..1400u64 {
            let t = day * DAY + 3600;
            let ai = cursor.weather_delay(r, 0, t);
            assert_eq!(ai, weather::delay_seconds(&mine, &r.origin_icao, t));
            hit += usize::from(ai > 0);
        }
    }
    assert!(hit > 0);
}

#[test]
fn weather_matches_across_speeds_step_sizes_jumps_and_restarts() {
    enable();
    let (seed, _) = seed_with_weather_at_start();
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
    let mut stepped = initial.clone();
    for _ in 0..3 {
        assert!(stepped.advance(30.0 * DAY as f64));
        stepped = restart(&stepped);
    }
    assert_eq!(normal(&stepped), normal(&expected));
    let mut jumped = initial.clone();
    jumped.jump(90.0 * DAY as f64).ok();
    assert_eq!(state(&jumped)["world_economy"], state(&expected)["world_economy"]);
}

#[test]
fn v43_saves_get_no_retroactive_weather() {
    enable();
    let mut g = with_seed(game(), 1);
    assert!(g.advance(300.0 * DAY as f64 + 5000.0));
    let now = g.clock.elapsed().as_secs();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    crate::save::save(&path, &g).unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    old["version"] = 43.into();
    let strip = |game: &mut serde_json::Value| {
        game["world_economy"].as_object_mut().unwrap().remove("weather_from");
        for r in game["routes"]["routes"].as_array_mut().into_iter().flatten() {
            if let Some(c) = r.pointer_mut("/plan/disruption") {
                c.as_object_mut().unwrap().remove("weather");
            }
        }
    };
    strip(&mut old["state"]);
    for c in old["state"]["competitors"].as_array_mut().unwrap() {
        strip(&mut c["game"]);
    }
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let loaded = crate::save::load(&path).unwrap();
    assert_eq!(loaded.world_economy.weather_from, now);
    let adopted = Weather::of(&loaded.world_economy);
    for r in &loaded.routes {
        assert_eq!(r.plan.as_ref().unwrap().disruption.as_ref().unwrap().weather, Some(adopted));
    }
    for c in &loaded.competitors {
        assert_eq!(c.game.world_economy, loaded.world_economy);
        for r in &c.game.routes {
            if let Some(cursor) = r.plan.as_ref().and_then(|p| p.disruption.as_ref()) {
                assert_eq!(cursor.weather, Some(adopted));
            }
        }
    }
    let catalog = airports::catalog().unwrap();
    for t in (now..now + RULES.warning_days * DAY).step_by(3600) {
        for a in &catalog.airports {
            assert_eq!(weather::delay_seconds(&adopted, &a.icao, t), 0);
        }
    }
    for d in 0..400 {
        assert!(weather::visible_at(&adopted, now + d * DAY).iter().all(|e| e.announced_at() >= now));
    }
    assert!((0..800).any(|d| !weather::visible_at(&adopted, now + d * DAY).is_empty()));
    assert_eq!(loaded.finance, g.finance);
    assert_eq!(loaded.cash(), g.cash());
    assert!(loaded.attention_snapshot().iter().all(|a| a.kind != "weather"));
}

#[test]
fn attention_lists_weather_at_airports_the_player_flies_from() {
    enable();
    let mut found = None;
    'seeds: for seed in 1..3000 {
        let c = w(seed);
        for day in 3..120 {
            if weather::visible_at(&c, day * DAY).iter().any(|e| e.icao == "EDDF" && e.announced_at() == day * DAY) {
                found = Some((seed, day));
                break 'seeds;
            }
        }
    }
    let (seed, day) = found.unwrap();
    let mut g = with_seed(game(), seed);
    assert!(g.advance((day * DAY + 100) as f64));
    let entries: Vec<_> = g.attention_snapshot().into_iter().filter(|a| a.kind == "weather").collect();
    assert!(!entries.is_empty());
    assert!(entries.iter().all(|e| e.text.starts_with("Wetter in")));
}

#[test]
fn ten_year_ai_comparison_with_weather_on_and_off() {
    enable();
    let mut new = Game::default();
    new.found("Weather baseline", "DE", "EDDF").unwrap();
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
                let (ta, tb) = (&a.game.finance.totals, &b.game.finance.totals);
                println!(
                    "WEATHER year={year} airline={} fleet={}/{} cash={}/{} insolvent={}/{} late={}/{} of {}/{} delay_s={}/{}",
                    a.game.airline.as_ref().unwrap().name,
                    a.game.fleet.len(), b.game.fleet.len(),
                    a.game.cash(), b.game.cash(),
                    a.game.insolvent(), b.game.insolvent(),
                    ta.late_flights, tb.late_flights, ta.punctuality_flights, tb.punctuality_flights,
                    ta.delay_seconds, tb.delay_seconds
                );
                assert!(!a.game.insolvent() && !a.game.distressed());
                assert!(a.game.cash() > b.game.cash() / 2 && a.game.cash() < b.game.cash() * 3 / 2);
                assert!(a.game.fleet.len().abs_diff(b.game.fleet.len()) <= 2);
            }
            assert_eq!(state(&restart(&new)), state(&new));
        }
    }
}
