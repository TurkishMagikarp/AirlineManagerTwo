//! Carbon charge on burned fuel. The price is a pure function of the departure
//! time (fixed yearly steps), so it needs no saved state and no randomness.
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
struct Config {
    co2_kg_per_kg_fuel: f64,
    start_eur_per_tonne: i64,
    yearly_step_eur_per_tonne: i64,
    cap_eur_per_tonne: i64,
    year_days: u64,
}
struct Rules {
    co2_milli: i64,
    start: i64,
    step: i64,
    cap: i64,
    year_seconds: u64,
}
static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let c: Config = serde_json::from_str(include_str!("../../config/emissions.json"))
        .expect("emissions config");
    assert!((0.0..=10.0).contains(&c.co2_kg_per_kg_fuel));
    assert!((0..=10_000).contains(&c.start_eur_per_tonne));
    assert!((0..=10_000).contains(&c.yearly_step_eur_per_tonne));
    assert!((c.start_eur_per_tonne..=10_000).contains(&c.cap_eur_per_tonne));
    assert!((1..=3650).contains(&c.year_days));
    Rules {
        co2_milli: (c.co2_kg_per_kg_fuel * 1000.0).round() as i64,
        start: c.start_eur_per_tonne,
        step: c.yearly_step_eur_per_tonne,
        cap: c.cap_eur_per_tonne,
        year_seconds: c.year_days * 86_400,
    }
});

// Scoped to the current test thread: reproduces a world from before the charge.
#[cfg(test)]
thread_local! { pub(crate) static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

/// EUR per tonne CO2 for a departure at `time` seconds of game time.
pub fn price_at(time: u64) -> i64 {
    #[cfg(test)]
    if DISABLED.with(|disabled| disabled.get()) {
        return 0;
    }
    let steps = i64::try_from(time / RULES.year_seconds).unwrap_or(i64::MAX);
    RULES
        .start
        .saturating_add(RULES.step.saturating_mul(steps))
        .min(RULES.cap)
}

pub fn year_seconds() -> u64 {
    RULES.year_seconds
}

pub fn max_price() -> i64 {
    RULES.cap
}

/// True when `time` lies in a year whose price is higher than the year before.
pub fn stepped_at(time: u64) -> bool {
    time >= RULES.year_seconds && price_at(time) > price_at(time - RULES.year_seconds)
}

/// Whole EUR, rounded up, from fuel quantity scaled by 3600 (as in `fuel_bill`).
pub fn charge(fuel_kg_times_3600: i128, price_eur_per_tonne: i64) -> i64 {
    let micro = fuel_kg_times_3600 * i128::from(RULES.co2_milli) * i128::from(price_eur_per_tonne);
    i64::try_from((micro + 3_599_999_999) / 3_600_000_000).unwrap_or(i64::MAX)
}

/// Cruise CO2 in grams per unit of capacity and km (`capacity` = seats, or
/// tonnes of payload for freighters), for comparing models before ordering.
pub fn grams_per_km(fuel_kg_per_hour: i64, cruise_kmh: u32, capacity: f64) -> f64 {
    if cruise_kmh == 0 || capacity <= 0.0 {
        return 0.0;
    }
    fuel_kg_per_hour as f64 * RULES.co2_milli as f64 / (f64::from(cruise_kmh) * capacity)
}
