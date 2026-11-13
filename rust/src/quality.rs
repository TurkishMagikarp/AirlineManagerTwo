//! Reproducible long-run audit, kept separate from economic rules.
use crate::{clock::Clock, game::Game};
use std::time::Instant;

fn scenario(price: Option<i64>) -> Game {
    let mut g = Game::default();
    g.found("Audit", "DE", "EDDF").unwrap();
    if let Some(price) = price {
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.set_ticket_price(1, price).unwrap();
        g.schedule(1, 1, 120).unwrap();
    }
    g.clock.set_running(true);
    g
}

#[test]
fn ten_year_scenarios_reconcile_and_survive_annual_restart() {
    let dir = tempfile::tempdir().unwrap();
    for price in [None, Some(0), Some(90), Some(100_000)] {
        let start = Instant::now();
        let mut fast = scenario(price);
        let mut daily = fast.clone();
        for year in 1..=10 {
            assert!(fast.advance(365.0 * 86400.0));
            for _ in 0..365 {
                assert!(daily.advance(86400.0));
            }
            assert_eq!(
                serde_json::to_value(&fast).unwrap(),
                serde_json::to_value(&daily).unwrap()
            );
            fast.validate_history().unwrap();
            fast.validate_competitors().unwrap();
            let path = dir.path().join("annual.json");
            crate::save::save(&path, &fast).unwrap();
            let bytes = std::fs::metadata(&path).unwrap().len();
            daily = crate::save::load(&path).unwrap();
            if [1, 5, 10].contains(&year) {
                println!(
                    "AUDIT player price={price:?} year={year} cash={} profit={}",
                    fast.airline.as_ref().unwrap().capital_eur,
                    fast.finance.totals.profit()
                );
                for c in &fast.competitors {
                    let g = &c.game;
                    println!(
                        "AUDIT price={price:?} year={year} airline={} cash={} profit={} fleet={} routes={} batches={} save_bytes={bytes}",
                        g.airline.as_ref().unwrap().name,
                        g.airline.as_ref().unwrap().capital_eur,
                        g.finance.totals.profit(),
                        g.fleet.len(),
                        g.routes.len(),
                        g.history.as_ref().unwrap().batches.len()
                    );
                }
            }
        }
        println!(
            "AUDIT scenario={price:?} elapsed_ms={}",
            start.elapsed().as_millis()
        );
        let before = serde_json::to_value(&fast).unwrap();
        assert!(fast.advance(0.0));
        assert_eq!(before, serde_json::to_value(&fast).unwrap());
        fast.clock.set_running(false);
        let paused = serde_json::to_value(&fast).unwrap();
        assert!(fast.advance(1e9));
        assert_eq!(paused, serde_json::to_value(&fast).unwrap());
    }
}

#[test]
fn ten_years_of_daily_fare_changes_remain_saveable_and_lossless() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("churn.json");
    let mut g = scenario(Some(90));
    crate::save::save(&path, &g).unwrap();
    for day in 0..3650 {
        g.set_ticket_price(1, if day % 2 == 0 { 70 } else { 90 })
            .unwrap();
        assert!(g.advance(86400.0));
    }
    g.validate_history().unwrap();
    println!(
        "CHURN batches={} state_bytes={}",
        g.history.as_ref().unwrap().batches.len(),
        serde_json::to_vec_pretty(&g).unwrap().len()
    );
    crate::save::save(&path, &g).unwrap();
    let bytes = std::fs::metadata(&path).unwrap().len();
    println!("CHURN save_bytes={bytes}");
    assert!(bytes < 3 * 1024 * 1024);
    let mut loaded = crate::save::load(&path).unwrap();
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        serde_json::to_value(&loaded).unwrap()
    );
    assert!(g.advance(86400.0));
    assert!(loaded.advance(86400.0));
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        serde_json::to_value(&loaded).unwrap()
    );
    let mut benchmark = loaded.clone();
    benchmark.clock.set_speed(16);
    let start = Instant::now();
    for _ in 0..1024 {
        assert!(benchmark.advance(1.0 / 64.0));
    }
    println!(
        "CHURN speed=16 us_per_frame={:.2}",
        start.elapsed().as_micros() as f64 / 1024.0
    );
}

#[test]
fn mixed_three_company_markets_remain_bounded_on_every_airport_pair() {
    let airports = &crate::airports::catalog().unwrap().airports;
    let template = scenario(Some(90)).routes.remove(0);
    for (i, a) in airports.iter().enumerate() {
        for b in airports.iter().skip(i + 1) {
            for prices in [[0, 0, 0], [0, 100, 200], [50, 90, 100_000], [100_000; 3]] {
                let mut routes = vec![template.clone(); 3];
                for (index, route) in routes.iter_mut().enumerate() {
                    route.origin_icao = a.icao.clone();
                    route.destination_icao = b.icao.clone();
                    route.ticket_eur = prices[index];
                    route.plan.as_mut().unwrap().interval_seconds = [7200, 21600, 604800][index];
                    if index == 1 {
                        std::mem::swap(&mut route.origin_icao, &mut route.destination_icao);
                    }
                }
                let base = crate::demand::market(&routes[0], 0).unwrap().total();
                let daily: f64 = routes
                    .iter()
                    .map(|r| {
                        let q = crate::economy::quote_shared(r, "atr72_600", &routes).unwrap();
                        assert!((0..=q.seats).contains(&q.passengers));
                        q.passengers as f64 * 43200.0
                            / r.plan.as_ref().unwrap().interval_seconds as f64
                    })
                    .sum();
                assert!(
                    daily <= base + 1e-9,
                    "{}–{} duplicated market",
                    a.icao,
                    b.icao
                );
            }
        }
    }
}

#[test]
fn subsecond_frames_match_single_step_at_every_speed() {
    for speed in Clock::SPEEDS {
        let mut frames = scenario(Some(90));
        frames.clock.set_speed(speed);
        assert!(frames.advance(365.0 * 86400.0 / f64::from(speed)));
        let mut single = frames.clone();
        let start = Instant::now();
        for _ in 0..8192 {
            assert!(frames.advance(1.0 / 64.0));
        }
        println!(
            "FRAMES speed={speed} us_per_frame={:.2}",
            start.elapsed().as_micros() as f64 / 8192.0
        );
        assert!(single.advance(128.0));
        assert_eq!(
            serde_json::to_value(&frames).unwrap(),
            serde_json::to_value(&single).unwrap()
        );
    }
}
