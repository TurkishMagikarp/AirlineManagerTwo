use crate::{clock::Clock, demand, economy, game::Game};
use std::time::Duration;
const DAY: u64 = 86400;

fn game() -> Game {
    let mut g = Game::default();
    g.found("Seasons", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 90).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}

#[test]
fn seasonal_curve_continuity_mean_peaks_and_year_wrap() {
    let year = demand::YEAR_SECONDS;
    let mut business = 0.0;
    let mut leisure = 0.0;
    for hour in 0..365 * 24 {
        let t = hour * 3600;
        let f = demand::season_factors(t);
        business += f.business;
        leisure += f.leisure;
        let next = demand::season_factors(t + 1);
        assert!((f.business - next.business).abs() < 0.00001);
        assert!((f.leisure - next.leisure).abs() < 0.00001);
        let repeated = demand::season_factors(t + 10 * year);
        assert_eq!(f.business, repeated.business);
        assert_eq!(f.leisure, repeated.leisure);
    }
    assert!((business / 8760.0 - 1.0).abs() < 1e-9);
    assert!((leisure / 8760.0 - 1.0).abs() < 1e-9);
    for month in 0..=12 {
        let t = month * year / 12;
        let a = demand::season_factors((t + year - 1) % year);
        let b = demand::season_factors((t + 1) % year);
        assert!((a.business - b.business).abs() < 1e-8);
        assert!((a.leisure - b.leisure).abs() < 1e-8);
    }
    let winter = demand::season_factors(year / 12);
    let summer = demand::season_factors(year / 2);
    let holidays = demand::season_factors(year * 11 / 12);
    assert!(winter.leisure < 0.8 && summer.leisure > 1.3 && holidays.leisure > 1.15);
    assert!(winter.business > summer.business && summer.business > holidays.business);
}

#[test]
fn seasonal_market_scales_only_segments_and_preserves_price_response() {
    let g = game();
    let r = &g.routes[0];
    for day in [0, 45, 180, 350] {
        let factor = demand::season_factors(day * DAY);
        for price in [0, 50, 90, 150, 100_000] {
            let annual = demand::market(r, price).unwrap();
            let seasonal = demand::market_at(r, price, day * DAY).unwrap();
            assert!((seasonal.business - annual.business * factor.business).abs() < 1e-9);
            assert!((seasonal.leisure - annual.leisure * factor.leisure).abs() < 1e-9);
        }
    }
}

#[test]
fn seasonal_full_year_matches_steps_speeds_pause_and_airborne_restart() {
    let mut large = game();
    let initial = large.clone();
    assert!(large.advance(demand::YEAR_SECONDS as f64));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("seasonal.json");
    for speed in [1, 4, 16] {
        let mut stepped = initial.clone();
        stepped.clock.set_running(false);
        let paused = state(&stepped);
        assert!(stepped.advance(1e8));
        assert_eq!(paused, state(&stepped));
        stepped.clock.set_speed(speed);
        stepped.clock.set_running(true);
        // Restart in flight in the summer trough for business demand.
        let checkpoint = 180 * DAY + 300;
        assert!(stepped.advance(checkpoint as f64 / speed as f64));
        let frozen = stepped.routes[0].plan.as_ref().unwrap().airborne.clone();
        assert!(frozen.is_some());
        crate::save::save(&path, &stepped).unwrap();
        stepped = crate::save::load(&path).unwrap();
        assert_eq!(frozen, stepped.routes[0].plan.as_ref().unwrap().airborne);
        let mut remaining = demand::YEAR_SECONDS - checkpoint;
        while remaining > 0 {
            let step = remaining.min(19 * 3600 + 17);
            assert!(stepped.advance(step as f64 / speed as f64));
            remaining -= step;
        }
        stepped.clock.set_speed(1);
        assert_eq!(state(&large), state(&stepped), "speed={speed}");
    }
}

#[test]
fn seasonal_old_save_keeps_preseason_airborne_quote_and_history() {
    let mut g = game();
    // A save from before seasonality has the same v16 shape. Recreate its
    // annual-demand departure result at a nontrivial point of the year.
    g.competitors.clear();
    g.stop_schedule(1).unwrap();
    g.clock = Clock::from_state(Duration::from_secs(180 * DAY), true, 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    assert!(g.advance(300.0));
    let annual = economy::quote_at_price(
        &g.routes[0],
        "atr72_600",
        &g.market_routes(),
        g.fuel_market.price_at(180 * DAY + 60),
    )
    .unwrap();
    g.routes[0]
        .plan
        .as_mut()
        .unwrap()
        .airborne
        .as_mut()
        .unwrap()
        .totals = annual;
    let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
    let history = g.history.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preseason.json");
    crate::save::save(&path, &g).unwrap();
    let mut loaded = crate::save::load(&path).unwrap();
    assert_eq!(history, loaded.history);
    assert_eq!(
        frozen,
        loaded.routes[0]
            .plan
            .as_ref()
            .unwrap()
            .airborne
            .clone()
            .unwrap()
    );
    assert!(loaded.advance(3600.0));
    assert_eq!(loaded.finance.recent[0], frozen);
}
