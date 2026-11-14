//! Shared daily belly freight market. Quotes are frozen by the flight engine.
use crate::{aircraft, demand, economy::Totals, game::Game, routes::Route};
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Rules {
    pub default_rate_cents: i64,
    pub maximum_rate_cents: i64,
    zero_demand_rate_cents: i64,
    kg_per_base_passenger: f64,
    passenger_and_bags_kg: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules =
        serde_json::from_str(include_str!("../../config/cargo.json")).expect("cargo config");
    assert!(r.default_rate_cents > 0 && r.default_rate_cents < r.zero_demand_rate_cents);
    assert!(r.zero_demand_rate_cents <= r.maximum_rate_cents && r.maximum_rate_cents <= 10000);
    assert!(r.kg_per_base_passenger.is_finite() && (0.0..=10.0).contains(&r.kg_per_base_passenger));
    assert!((1..=200).contains(&r.passenger_and_bags_kg));
    r
});
pub fn default_rate() -> i64 {
    RULES.default_rate_cents
}
pub fn capacity(model: &aircraft::Model, passengers: i64) -> i64 {
    i64::from(model.belly_cargo_kg)
        .min((i64::from(model.payload_kg) - passengers * RULES.passenger_and_bags_kg).max(0))
}
pub fn revenue(kg: i64, rate: i64) -> i64 {
    kg * rate / 100
}
pub fn valid(t: &Totals) -> bool {
    t.cargo_kg >= 0
        && t.cargo_revenue >= 0
        && t.cargo_revenue <= t.revenue
        && i128::from(t.cargo_revenue)
            <= i128::from(t.cargo_kg) * i128::from(RULES.maximum_rate_cents) / 100
}

pub fn daily_demand(route: &Route) -> Result<f64, String> {
    Ok(demand::market(route, 0)?.total() * RULES.kg_per_base_passenger)
}

/// Water-fill available payload, preserving the outside option for expensive rates.
/// Daily demand is per direction, like passengers; infrequent flights cannot bank it.
#[cfg(test)]
pub fn estimate(
    route: &Route,
    model_id: &str,
    supply: &[demand::Offer],
    time: Option<u64>,
) -> Result<i64, String> {
    let offers: Vec<_> = supply
        .iter()
        .filter(|o| o.route.plan.is_some() && demand::same_market(route, &o.route))
        .cloned()
        .collect();
    let passengers = demand::market_passengers(&offers, time)?;
    estimate_loaded(route, model_id, supply, &passengers)
}

struct Share {
    frequency: f64,
    willingness: f64,
    limit: f64,
    allocated: f64,
}

pub fn estimate_loaded(
    route: &Route,
    model_id: &str,
    supply: &[demand::Offer],
    passengers: &[i64],
) -> Result<i64, String> {
    if route.plan.is_none() {
        return Ok(0);
    }
    let base = daily_demand(route)?;
    let offers: Vec<_> = supply
        .iter()
        .filter(|o| o.route.plan.is_some() && demand::same_market(route, &o.route))
        .collect();
    let mut shares = Vec::with_capacity(offers.len());
    let mut target = None;
    for (i, o) in offers.iter().enumerate() {
        let own = demand::same_offer(route, &o.route)
            && o.cargo_model
                .as_deref()
                .is_none_or(|model| model == model_id);
        if own && target.is_none() {
            target = Some(i);
        }
        let f = 43200.0 / o.route.plan.as_ref().unwrap().interval_seconds as f64;
        let w = (1.0 - o.route.cargo_rate_cents as f64 / RULES.zero_demand_rate_cents as f64)
            .clamp(0.0, 1.0);
        let pax = passengers[i];
        // Synthetic planning offers may omit model metadata; actual fleet offers always carry it.
        let model = aircraft::model(o.cargo_model.as_deref().unwrap_or(model_id))?;
        shares.push(Share {
            frequency: f,
            willingness: w,
            limit: (capacity(model, pax) as f64 * f).min(base * w),
            allocated: 0.0,
        });
    }
    let Some(target) = target else {
        return Ok(0);
    };
    // Shippers can choose the cheapest available airline. An overpriced
    // competitor must not destroy demand that an affordable carrier can serve.
    let mut remaining = base * shares.iter().map(|s| s.willingness).fold(0.0, f64::max);
    for _ in 0..=shares.len() {
        let weight: f64 = shares
            .iter()
            .filter(|s| s.allocated < s.limit)
            .map(|s| s.frequency * s.willingness)
            .sum();
        // Match the passenger allocator's tolerance: rounding dust cannot fill
        // another kilogram and must not trigger repeated spill rounds.
        if weight <= 0.0 || remaining <= 1e-9 {
            break;
        }
        let mut used = 0.0;
        for s in &mut shares {
            let amount = (remaining * s.frequency * s.willingness / weight)
                .min((s.limit - s.allocated).max(0.0));
            s.allocated += amount;
            used += amount;
        }
        remaining = (remaining - used).max(0.0);
    }
    let own = &shares[target];
    Ok((own.allocated / own.frequency.max(1.0)).floor() as i64)
}
impl Game {
    pub fn set_cargo_rate(&mut self, id: u32, rate: i64) -> Result<(), String> {
        self.require_active()?;
        if !(0..=RULES.maximum_rate_cents).contains(&rate) {
            return Err("Ungültiger Frachtpreis.".into());
        }
        if !self
            .routes
            .iter()
            .any(|r| r.id == id && r.parent_route_id.is_none())
        {
            return Err("Unbekannte Route.".into());
        }
        for r in self.routes.iter_mut().filter(|r| r.public_id() == id) {
            r.cargo_rate_cents = rate;
        }
        Ok(())
    }
}
