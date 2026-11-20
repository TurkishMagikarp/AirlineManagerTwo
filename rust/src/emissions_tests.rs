use crate::{aircraft, clock::Clock, economy, emissions, game::Game, save};
use std::time::Duration;
const DAY: u64 = 86400;
const YEAR: u64 = 365 * DAY;

fn at(g: &mut Game, seconds: u64) {
    let running = g.clock.is_running();
    g.clock = Clock::from_state(Duration::from_secs(seconds), running, 1).unwrap();
}
fn game_at(start: u64) -> Game {
    let mut g = Game::default();
    g.found("Carbon", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.history.as_mut().unwrap().opening_balance += 5_000_000_000;
    g.airline.as_mut().unwrap().capital_eur += 5_000_000_000;
    at(&mut g, start);
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 90).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
fn state(g: &Game) -> serde_json::Value {
    let mut v = serde_json::to_value(g).unwrap();
    v["clock"]["speed_index"] = 0.into();
    v
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("carbon.json");
    save::save(&path, g).unwrap();
    save::load(&path).unwrap()
}

#[test]
fn price_follows_a_fixed_yearly_schedule_up_to_the_cap() {
    let start = emissions::price_at(0);
    assert!(start > 0);
    assert_eq!(emissions::price_at(YEAR - 1), start);
    let step = emissions::price_at(YEAR) - start;
    assert!(step > 0);
    assert_eq!(emissions::price_at(2 * YEAR - 1), start + step);
    assert_eq!(emissions::price_at(2 * YEAR), start + 2 * step);
    assert_eq!(emissions::price_at(1000 * YEAR), emissions::max_price());
    assert_eq!(emissions::price_at(u64::MAX), emissions::max_price());
    let mut last = 0;
    for year in 0..60 {
        let price = emissions::price_at(year * YEAR);
        assert!(price >= last && price <= emissions::max_price());
        last = price;
    }
}

#[test]
fn charge_is_identical_across_speeds_steps_and_a_save_across_the_price_step() {
    let start = YEAR - DAY;
    let make = || game_at(start);
    let mut large = make();
    assert!(large.advance((3 * DAY) as f64));
    let mut small = make();
    for _ in 0..72 {
        assert!(small.advance(3600.0));
    }
    let mut fast = make();
    fast.clock.set_speed(16);
    for _ in 0..(3 * DAY / 16 / 60) {
        assert!(fast.advance(60.0));
    }
    fast.clock.set_speed(1);
    // Save and restart while a flight that left at the old price is airborne.
    let mut saved = make();
    while saved.clock.elapsed().as_secs() < YEAR - 120 {
        assert!(saved.advance(60.0));
    }
    assert!(saved.clock.elapsed().as_secs() < YEAR);
    saved = restart(&saved);
    let remaining = start + 3 * DAY - saved.clock.elapsed().as_secs();
    assert!(saved.advance(remaining as f64));
    saved = restart(&saved);
    assert_eq!(state(&large), state(&small));
    assert_eq!(state(&large), state(&fast));
    assert_eq!(state(&large), state(&saved));
    let t = &large.finance.totals;
    assert!(t.carbon > 0 && t.flights > 20);
    let route = &large.routes[0];
    let flight = crate::operations::timings(route, "atr72_600").unwrap().0;
    let mut prices = std::collections::BTreeSet::new();
    for f in &large.finance.recent {
        let price = emissions::price_at(f.landing - flight);
        prices.insert(price);
        assert_eq!(
            f.totals.carbon,
            economy::carbon_bill(route, "atr72_600", price).unwrap()
        );
        assert!(f.totals.costs() > f.totals.carbon);
    }
    assert_eq!(prices.len(), 2, "flights before and after the step");
    assert_eq!(
        t.carbon,
        large
            .finance
            .recent
            .iter()
            .map(|f| f.totals.carbon)
            .sum::<i64>()
    );
}

#[test]
fn old_and_new_generation_aircraft_pay_in_the_ratio_of_their_fuel_burn() {
    let g = game_at(0);
    let route = &g.routes[0];
    let mut route = route.clone();
    route.destination_icao = "LTFM".into();
    for price in [15, 60, 100] {
        for (old, new) in [
            ("a320_200", "a320neo"),
            ("b737_800", "b737_max8"),
            ("a330_300", "a330_900"),
        ] {
            let (o, n) = (
                economy::carbon_bill(&route, old, price).unwrap(),
                economy::carbon_bill(&route, new, price).unwrap(),
            );
            assert!(o > n, "{old} pays more than {new}");
            let (fo, fn_) = (
                economy::fuel_bill(&route, old, 8000).unwrap(),
                economy::fuel_bill(&route, new, 8000).unwrap(),
            );
            // carbon ∝ burned fuel: o/n == fo/fn up to whole-EUR rounding.
            let lhs = i128::from(o) * i128::from(fn_);
            let rhs = i128::from(n) * i128::from(fo);
            assert!((lhs - rhs).abs() * 100 <= lhs, "{old}/{new} at {price}");
        }
    }
    let (a320, neo) = (
        aircraft::model("a320_200").unwrap(),
        aircraft::model("a320neo").unwrap(),
    );
    assert!(economy::co2_grams_per_km(neo) < economy::co2_grams_per_km(a320));
    // Per seat the old type costs proportionally more.
    let per_seat = |m: &str, seats: u32| {
        economy::carbon_bill(&route, m, 100).unwrap() as f64 / f64::from(seats)
    };
    assert!(per_seat("a320_200", a320.seats) > per_seat("a320neo", neo.seats) * 1.1);
    assert_eq!(economy::carbon_bill(&route, "a320_200", 0).unwrap(), 0);
}

#[test]
fn flights_pay_cash_ferries_and_planning_include_the_charge() {
    let mut g = game_at(3 * YEAR);
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    let planned = g.planning_quote("EDDF", "EDDM", 2).unwrap();
    assert_eq!(
        planned.carbon,
        economy::carbon_bill(&g.routes[0], "atr72_600", emissions::price_at(3 * YEAR)).unwrap()
    );
    assert!(planned.carbon > 0);
    let cash = g.airline.as_ref().unwrap().capital_eur;
    assert!(g.advance((2 * DAY) as f64));
    let t = &g.finance.totals;
    assert!(t.carbon > 0);
    let paid: i64 = g.finance.recent.iter().map(|f| f.totals.profit()).sum();
    assert_eq!(g.airline.as_ref().unwrap().capital_eur - cash, paid);
    assert_eq!(
        t.costs(),
        t.fuel + t.carbon + t.airport + t.crew + t.overhead + t.service + t.care + t.loyalty_cost
    );
    g.validate_finance().unwrap();
    let back = restart(&g);
    assert_eq!(state(&g), state(&back));
}

#[test]
fn old_save_follows_the_new_schedule_without_back_charges() {
    emissions::DISABLED.with(|d| d.set(true));
    let mut g = game_at(2 * YEAR - DAY);
    assert!(g.advance(DAY as f64 / 2.0));
    assert!(g.finance.totals.flights > 5);
    assert_eq!(g.finance.totals.carbon, 0);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v46.json");
    save::save(&path, &g).unwrap();
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    old["version"] = 46.into();
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    emissions::DISABLED.with(|d| d.set(false));
    let mut loaded = save::load(&path).unwrap();
    let loaded_at = loaded.clock.elapsed().as_secs();
    let flights_before = loaded.finance.totals.flights;
    assert_eq!(loaded.finance, g.finance);
    let frozen = loaded.routes[0].plan.as_ref().unwrap().airborne.clone();
    assert!(frozen.is_none_or(|f| f.totals.carbon == 0));
    // The same dates give the same price as in a new game.
    assert_eq!(
        emissions::price_at(loaded_at),
        emissions::price_at(YEAR),
        "a new game has this price on this date"
    );
    assert!(loaded.advance((3 * DAY) as f64));
    let t = &loaded.finance.totals;
    assert!(t.flights > flights_before + 20 && t.carbon > 0);
    let route = &loaded.routes[0];
    let flight = crate::operations::timings(route, "atr72_600").unwrap().0;
    let mut charged = 0;
    for f in loaded.finance.recent.iter().rev() {
        if f.landing - flight < loaded_at {
            assert_eq!(f.totals.carbon, 0, "no back-charge for past departures");
        } else {
            assert_eq!(
                f.totals.carbon,
                economy::carbon_bill(route, "atr72_600", emissions::price_at(f.landing - flight))
                    .unwrap()
            );
            charged += f.totals.carbon;
        }
    }
    assert_eq!(charged, t.carbon);
    loaded.validate_finance().unwrap();
    assert_eq!(state(&loaded), state(&restart(&loaded)));
}

#[test]
fn attention_announces_the_step_before_and_after_it() {
    let mut g = game_at(0);
    let notes = |g: &Game| -> Vec<String> {
        g.attention_snapshot()
            .into_iter()
            .filter(|e| e.kind == "carbon_step")
            .map(|e| e.text)
            .collect()
    };
    at(&mut g, 100 * DAY);
    assert!(notes(&g).is_empty());
    at(&mut g, YEAR - 20 * DAY);
    let before = notes(&g);
    assert_eq!(before.len(), 1);
    assert!(before[0].contains(&format!("{} EUR/t", emissions::price_at(YEAR))));
    at(&mut g, YEAR + DAY);
    let after = notes(&g);
    assert_eq!(after.len(), 1);
    assert!(after[0].starts_with(&format!(
        "CO2-Preis jetzt {} EUR/t",
        emissions::price_at(YEAR)
    )));
    at(&mut g, YEAR + 100 * DAY);
    assert!(notes(&g).is_empty());
}

#[test]
fn ai_airlines_buy_new_types_and_stay_solvent_for_ten_years_with_the_charge() {
    let mut g = Game::default();
    g.found("Carbon Rival Audit", "DE", "EDDF").unwrap();
    g.clock.set_running(true);
    let mut owned = std::collections::BTreeSet::new();
    let mut new_generation = vec![0usize; g.competitors.len()];
    for day in 1..=3650u64 {
        assert!(g.advance(DAY as f64));
        for (i, c) in g.competitors.iter().enumerate() {
            for p in &c.game.fleet {
                if owned.insert((i, p.id)) && aircraft::model(&p.model_id).unwrap().entry_day > 0 {
                    new_generation[i] += 1;
                }
            }
        }
        if [3 * 365, 6 * 365, 3650].contains(&day) {
            for c in &g.competitors {
                let mut mix = std::collections::BTreeMap::<&str, usize>::new();
                for p in &c.game.fleet {
                    *mix.entry(p.model_id.as_str()).or_default() += 1;
                }
                println!(
                    "AI_REPORT year={} airline={} fleet={mix:?} capital={} carbon_paid={}",
                    day / 365,
                    c.game.airline.as_ref().unwrap().name,
                    c.game.cash(),
                    c.game.finance.totals.carbon, // CARBON
                );
            }
        }
    }
    for (i, c) in g.competitors.iter().enumerate() {
        println!(
            "AI_REPORT final airline={} new_generation_aircraft={} capital={} insolvent={}",
            c.game.airline.as_ref().unwrap().name,
            new_generation[i],
            c.game.cash(),
            c.game.insolvent()
        );
        assert!(
            !c.game.insolvent(),
            "{} stays solvent",
            c.game.airline.as_ref().unwrap().name
        );
        assert!(c.game.cash() > 0);
        assert!(c.game.finance.totals.carbon > 0);
    }
    g.validate_competitors().unwrap();
}
