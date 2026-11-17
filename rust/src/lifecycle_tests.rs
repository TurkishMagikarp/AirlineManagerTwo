use crate::{
    aircraft::{self, Availability},
    clock::Clock,
    game::Game,
    save,
    trading::hull_value_at,
};
use std::time::Duration;
const DAY: u64 = 86400;

fn at(g: &mut Game, seconds: u64) {
    g.clock = Clock::from_state(Duration::from_secs(seconds), false, 1).unwrap();
}
fn run_to(g: &mut Game, seconds: u64) {
    g.clock.set_running(true);
    while g.clock.elapsed().as_secs() < seconds {
        let step = (seconds - g.clock.elapsed().as_secs()).min(DAY);
        assert!(g.advance(step as f64));
    }
}
fn game() -> Game {
    let mut g = Game::default();
    g.found("Lifecycle Audit", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.history.as_mut().unwrap().opening_balance += 5_000_000_000;
    g.airline.as_mut().unwrap().capital_eur += 5_000_000_000;
    g
}
fn state(g: &Game) -> serde_json::Value {
    let mut v = serde_json::to_value(g).unwrap();
    v["clock"]["speed_index"] = 0.into();
    v
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lifecycle.json");
    save::save(&path, g).unwrap();
    save::load(&path).unwrap()
}
fn names(g: &Game) -> Vec<String> {
    g.attention_snapshot().into_iter().map(|e| e.text).collect()
}

#[test]
fn new_models_burn_less_cost_more_and_share_ratings_with_their_predecessors() {
    let fuel = |id: &str| crate::economy::fuel_kg_per_hour(id);
    for (old, new) in [
        ("a320_200", "a320neo"),
        ("b737_800", "b737_max8"),
        ("atr72_600", "atr72_evo"),
        ("a330_300", "a330_900"),
    ] {
        let (o, n) = (aircraft::model(old).unwrap(), aircraft::model(new).unwrap());
        assert!(fuel(new) < fuel(old), "{new} burns less");
        assert!(n.price_eur > o.price_eur, "{new} costs more");
        assert!(n.range_km >= o.range_km);
        assert_eq!(o.obsolete_from_day, Some(n.entry_day));
        assert!(o.end_of_production_day.unwrap() > n.entry_day);
        assert_eq!(
            crate::staff::needs(old).unwrap().rating,
            crate::staff::needs(new).unwrap().rating
        );
    }
    assert!(aircraft::model("a320neo").unwrap().entry_day <= 3 * 365);
    assert_eq!(aircraft::model("b777_300er").unwrap().entry_day, 0);
    assert_eq!(
        aircraft::model("b777_300er").unwrap().end_of_production_day,
        None
    );
}

#[test]
fn entry_day_gates_orders_purchases_and_leases_to_the_second() {
    let neo = aircraft::model("a320neo").unwrap();
    let entry = neo.entry_day * DAY;
    let mut g = game();
    at(&mut g, entry - 1);
    assert_eq!(
        neo.availability(entry - 1),
        Availability::Upcoming {
            from_day: neo.entry_day
        }
    );
    assert!(g.quote_order("a320neo", 1).is_err());
    assert!(g.place_order("a320neo", 1).is_err());
    assert!(g.buy_aircraft("a320neo").is_err());
    assert!(g.lease_aircraft("a320neo").is_err());
    assert!(g.orders.is_empty() && g.fleet.is_empty());
    at(&mut g, entry);
    g.place_order("a320neo", 1).unwrap();
    g.buy_aircraft("a320neo").unwrap();
    g.lease_aircraft("a320neo").unwrap();
    assert_eq!((g.orders.len(), g.fleet.len()), (1, 2));
    g.validate_fleet().unwrap();
}

#[test]
fn end_of_production_stops_new_units_but_not_leases_sales_or_open_orders() {
    let old = aircraft::model("a320_200").unwrap();
    let end = old.end_of_production_day.unwrap() * DAY;
    let mut g = game();
    run_to(&mut g, end - 1);
    g.place_order("a320_200", 1).unwrap();
    let delivery = g.orders[0].delivery_at;
    assert!(delivery > end);
    run_to(&mut g, end);
    assert!(g.quote_order("a320_200", 1).is_err());
    assert!(g.buy_aircraft("a320_200").is_err());
    g.lease_aircraft("a320_200").unwrap();
    let leased = g.fleet[0].id;
    g.return_aircraft(leased).unwrap();
    assert_eq!(g.used_market.listings.len(), 1);
    let listing = g.used_market.listings[0].id;
    g.buy_used_aircraft(listing).unwrap();
    g.clock.set_running(true);
    assert!(g.advance((delivery - end) as f64));
    assert!(
        g.orders.is_empty(),
        "open order for an out-of-production model is delivered"
    );
    assert_eq!(
        g.fleet.iter().filter(|p| p.model_id == "a320_200").count(),
        2
    );
}

#[test]
fn used_aircraft_of_the_old_generation_lose_value_faster_once_the_successor_exists() {
    let plane = |model: &str| crate::aircraft::Aircraft {
        business_seats: 0,
        id: 1,
        model_id: model.into(),
        location_icao: "EDDF".into(),
        status: aircraft::AircraftStatus::Parked,
        purchase_price_eur: 1,
        acquired_at: Some(0),
        purchased_at: None,
        lease: None,
        wear: Default::default(),
        maintenance: None,
    };
    let share = |model: &str, from: u64, to: u64| {
        let p = plane(model);
        let price = i128::from(aircraft::model(model).unwrap().price_eur);
        (i128::from(hull_value_at(&p, from * DAY).unwrap())
            - i128::from(hull_value_at(&p, to * DAY).unwrap()))
            * 10_000
            / price
    };
    // The A320-200 successor arrives on day 730; the 777-300ER has none.
    assert_eq!(share("a320_200", 100, 465), share("b777_300er", 100, 465));
    assert!(share("a320_200", 800, 1165) > share("b777_300er", 800, 1165) + 300);
    assert!(share("a320neo", 800, 1165) == share("b777_300er", 800, 1165));
}

#[test]
fn attention_announces_entry_and_end_of_production_in_a_window() {
    let neo = aircraft::model("a320neo").unwrap();
    let old = aircraft::model("a320_200").unwrap();
    let rules = &aircraft::LIFECYCLE;
    let mut g = game();
    let has = |g: &Game, needle: &str| names(g).iter().any(|t| t.contains(needle));
    at(&mut g, (neo.entry_day - rules.announce_days - 1) * DAY);
    assert!(!has(&g, "A320neo"));
    at(&mut g, (neo.entry_day - rules.announce_days) * DAY);
    assert!(
        names(&g)
            .iter()
            .any(|t| t.contains("A320neo") && t.contains("Bestellungen ab Tag 731"))
    );
    at(
        &mut g,
        (neo.entry_day + rules.notice_visible_days - 1) * DAY,
    );
    assert!(has(&g, "bestellbar"));
    at(&mut g, (neo.entry_day + rules.notice_visible_days) * DAY);
    assert!(!has(&g, "A320neo"));
    let end = old.end_of_production_day.unwrap();
    at(&mut g, (end - 10) * DAY);
    assert!(
        names(&g)
            .iter()
            .any(|t| t.contains("A320-200") && t.contains("Produktion endet an Tag"))
    );
    at(&mut g, end * DAY);
    assert!(has(&g, "nicht mehr neu bestellbar"));
}

#[test]
fn old_saves_get_the_same_timeline_as_a_new_game() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    for (version, day, expect_neo, expect_old) in [
        (45, 100, false, true),
        (45, 800, true, true),
        (30, 2000, true, false),
    ] {
        let mut g = game();
        run_to(&mut g, day * DAY);
        save::save(&path, &g).unwrap();
        let mut old: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        old["version"] = version.into();
        std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let mut loaded = save::load(&path).unwrap();
        assert_eq!(
            loaded.buy_aircraft("a320neo").is_ok(),
            expect_neo,
            "v{version} day {day}"
        );
        assert_eq!(
            loaded.buy_aircraft("a320_200").is_ok(),
            expect_old,
            "v{version} day {day}"
        );
        assert_eq!(loaded.buy_aircraft("a350_900").is_ok(), day >= 1825);
        loaded.validate_fleet().unwrap();
    }
}

#[test]
fn steps_speeds_and_restarts_across_an_entry_date_give_identical_state() {
    let entry = aircraft::model("a320neo").unwrap().entry_day * DAY;
    let make = || {
        let mut g = game();
        g.buy_aircraft("atr72_600").unwrap();
        run_to(&mut g, entry - 6 * 3600);
        g
    };
    let mut fine = make();
    for _ in 0..12 {
        assert!(fine.advance(1800.0));
    }
    fine.place_order("a320neo", 1).unwrap();
    for _ in 0..24 * 70 {
        assert!(fine.advance(3600.0));
    }
    let mut coarse = make();
    assert!(coarse.advance(6.0 * 3600.0));
    coarse = restart(&coarse);
    coarse.place_order("a320neo", 1).unwrap();
    assert!(coarse.advance(35.0 * 86400.0));
    coarse = restart(&coarse);
    assert!(coarse.advance(35.0 * 86400.0));
    assert_eq!(state(&fine), state(&coarse));
    assert_eq!(
        fine.fleet
            .iter()
            .filter(|p| p.model_id == "a320neo")
            .count(),
        1
    );
}

#[test]
fn ai_airlines_follow_the_same_dates_over_ten_years_and_stay_solvent() {
    let mut g = Game::default();
    g.found("Lifecycle Rival Audit", "DE", "EDDF").unwrap();
    g.clock.set_running(true);
    let mut seen_orders = std::collections::BTreeSet::new();
    let mut fleet_models = std::collections::BTreeMap::<String, usize>::new();
    for day in 1..=3650u64 {
        assert!(g.advance(DAY as f64));
        for (i, c) in g.competitors.iter().enumerate() {
            for o in &c.game.orders {
                if !seen_orders.insert((i, o.id)) {
                    continue;
                }
                let m = aircraft::model(&o.model_id).unwrap();
                assert!(
                    o.ordered_at / DAY >= m.entry_day,
                    "AI ordered {} before entry",
                    m.id
                );
                assert!(
                    m.end_of_production_day
                        .is_none_or(|end| o.ordered_at / DAY < end),
                    "AI ordered {} after end of production",
                    m.id
                );
            }
            for p in &c.game.fleet {
                let m = aircraft::model(&p.model_id).unwrap();
                assert!(p.acquired_at.unwrap() / DAY >= m.entry_day);
            }
        }
        if day % 365 == 0 || day == 3650 {
            fleet_models.clear();
            for c in &g.competitors {
                for p in &c.game.fleet {
                    *fleet_models.entry(p.model_id.clone()).or_default() += 1;
                }
            }
            println!(
                "LIFECYCLE_AUDIT year={} fleet={fleet_models:?} cash={:?} insolvent={:?}",
                day / 365,
                g.competitors
                    .iter()
                    .map(|c| c.game.cash())
                    .collect::<Vec<_>>(),
                g.competitors
                    .iter()
                    .map(|c| c.game.insolvent())
                    .collect::<Vec<_>>()
            );
        }
    }
    g.validate_competitors().unwrap();
    assert!(
        g.competitors.iter().all(|c| !c.game.insolvent()),
        "no AI airline goes broke"
    );
}
