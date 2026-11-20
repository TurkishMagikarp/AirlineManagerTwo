//! Shared fare search; persistent cursors belong to public routes, never to UI clocks.
use crate::{aircraft::Aircraft, demand::Offer, economy, game::Game, routes::Route};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Rules {
    pub fare_review_seconds: u64,
    pub fare_step_eur: i64,
    pub fare_sweep_steps: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/competition.json")).unwrap();
    assert!(r.fare_review_seconds > 0 && (1..=100_000).contains(&r.fare_step_eur));
    assert!((1..=100).contains(&r.fare_sweep_steps));
    r
});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    pub at: u64,
    pub old: i64,
    pub new: i64,
    pub old_business: Option<i64>,
    pub new_business: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Autopilot {
    pub route_id: u32,
    pub next_review: Option<u64>,
    pub last_change: Option<Change>,
}
#[derive(Debug)]
pub struct Suggestion {
    pub fare: i64,
    pub business: Option<i64>,
    pub current_profit: i64,
    pub profit: i64,
}

/// Ascending candidates, with an explicit preference for the current fare on ties.
fn search(
    old: i64,
    steps: i64,
    mut quote: impl FnMut(i64) -> Result<i64, String>,
) -> Result<(i64, i64), String> {
    let mut best = (old, i64::MIN);
    for offset in -steps..=steps {
        let price = (old + offset * RULES.fare_step_eur).clamp(0, 100_000);
        let profit = quote(price)?;
        if profit > best.1 || (profit == best.1 && price == old) {
            best = (price, profit);
        }
    }
    Ok(best)
}

/// The existing AI hypothetical direct market is intentionally preserved verbatim.
/// Player forecasts additionally include today's season, awareness and connections.
pub fn review(
    game: &Game,
    route: &Route,
    plane: &Aircraft,
    supply: &[Offer],
    forecast: bool,
) -> Result<Suggestion, String> {
    let fuel = game.fuel_market.price_at(game.clock.elapsed().as_secs());
    let carbon = crate::emissions::price_at(game.clock.elapsed().as_secs());
    let mut candidate = route.clone();
    let mut fare_supply: Vec<_> = supply
        .iter()
        .filter(|o| crate::demand::same_market(&o.route, route))
        .cloned()
        .collect();
    if let Some(i) = fare_supply.iter().position(|o| &o.route == route) {
        fare_supply.remove(i);
    }
    fare_supply.push(game.economy_offer(&candidate, plane)?);
    let mut market = economy::QuoteMarket::direct(&candidate, &plane.model_id, &fare_supply)?;
    let current_profit = if forecast {
        game.forecast_fare(route.public_id(), None)?
    } else {
        0
    };
    let (fare, profit) = search(
        route.ticket_eur,
        if forecast { RULES.fare_sweep_steps } else { 1 },
        |price| {
            if forecast {
                return game.forecast_fare(route.public_id(), Some(price));
            }
            candidate.ticket_eur = price;
            if plane.business_seats > 0 {
                candidate.business_ticket_eur = crate::cabins::premium_fare(price);
            }
            market.reprice_direct(&candidate);
            Ok(economy::quote_prepared(
                &candidate,
                &plane.model_id,
                supply,
                fuel,
                carbon,
                None,
                &mut market,
            )?
            .profit())
        },
    )?;
    Ok(Suggestion {
        fare,
        business: (plane.business_seats > 0).then(|| crate::cabins::premium_fare(fare)),
        current_profit,
        profit,
    })
}

impl Game {
    /// Mean quote per scheduled aircraft departure, matching the route UI's averages.
    fn forecast_fare(&self, id: u32, price: Option<i64>) -> Result<i64, String> {
        let mut supply = self.market_routes();
        let owner = self.market_owner_key();
        if let Some(price) = price {
            for offer in &mut supply {
                if offer.owner == owner && offer.route.public_id() == id {
                    offer.route.ticket_eur = price;
                    if offer.business_seats > 0 {
                        offer.route.business_ticket_eur = crate::cabins::premium_fare(price);
                    }
                }
            }
        }
        let now = self.clock.elapsed().as_secs();
        let mut total = 0i128;
        let mut count = 0;
        for offer in supply
            .iter()
            .filter(|o| o.owner == owner && o.route.public_id() == id && o.route.plan.is_some())
        {
            let plane = self
                .fleet
                .iter()
                .find(|p| Some(p.id) == offer.route.aircraft_id)
                .ok_or("Flugzeug fehlt")?;
            let mut market = economy::QuoteMarket::new(&offer.route, &plane.model_id, &supply)?;
            let mut quote = economy::quote_prepared(
                &offer.route,
                &plane.model_id,
                &supply,
                self.fuel_market.price_at(now),
                crate::emissions::price_at(now),
                Some(now),
                &mut market,
            )?;
            if let Some(c) = self.fuel_contract.as_ref().filter(|c| c.active_at(now)) {
                let price = self.fuel_market.price_at(now) * i64::from(100 - c.share_percent)
                    + c.locked_cents * i64::from(c.share_percent);
                quote.fuel = economy::fuel_bill(&offer.route, &plane.model_id, price)?;
            }
            total += i128::from(quote.profit());
            count += 1;
        }
        if count == 0 {
            return Err("Kein aktiver Flugplan".into());
        }
        Ok((total / count) as i64)
    }
    pub fn fare_suggestion(&self, id: u32) -> Result<Option<Suggestion>, String> {
        let Some(route) = self
            .routes
            .iter()
            .find(|r| r.public_id() == id && r.plan.is_some())
        else {
            return Ok(None);
        };
        let Some(plane) = self.fleet.iter().find(|p| Some(p.id) == route.aircraft_id) else {
            return Ok(None);
        };
        let mut result = review(self, route, plane, &self.market_routes(), true)?;
        if self.routes.iter().filter(|r| r.public_id() == id).any(|r| {
            self.fleet
                .iter()
                .any(|p| Some(p.id) == r.aircraft_id && p.business_seats > 0)
        }) {
            result.business = Some(crate::cabins::premium_fare(result.fare));
        }
        Ok(Some(result))
    }
    pub fn apply_fare_suggestion(&mut self, id: u32) -> Result<(), String> {
        if let Some(s) = self.fare_suggestion(id)? {
            self.set_ticket_price(id, s.fare)?;
            if let Some(business) = s.business {
                self.set_business_ticket_price(id, business)?;
            }
        }
        Ok(())
    }
    pub fn set_pricing_autopilot(&mut self, id: u32, enabled: bool) -> Result<(), String> {
        self.require_active()?;
        if !self
            .routes
            .iter()
            .any(|r| r.id == id && r.parent_route_id.is_none())
        {
            return Err("Unbekannte Route".into());
        }
        let next = enabled.then(|| {
            (self.clock.elapsed().as_secs() / RULES.fare_review_seconds + 1)
                * RULES.fare_review_seconds
        });
        if let Some(p) = self.pricing.iter_mut().find(|p| p.route_id == id) {
            if p.next_review.is_some() != enabled {
                p.next_review = next;
            }
        } else if enabled {
            self.pricing.push(Autopilot {
                route_id: id,
                next_review: next,
                last_change: None,
            });
        }
        Ok(())
    }
    pub fn next_pricing_event(&self) -> Option<u64> {
        self.pricing.iter().filter_map(|p| p.next_review).min()
    }
    pub fn apply_pricing(&mut self) -> Result<(), String> {
        // Financing can end the company at this same event boundary. Do not
        // let a now-forbidden fare write roll back the terminal transition.
        if self.insolvent() {
            for p in &mut self.pricing {
                p.next_review = None;
            }
            return Ok(());
        }
        let now = self.clock.elapsed().as_secs();
        for i in 0..self.pricing.len() {
            if !self.pricing[i].next_review.is_some_and(|t| t <= now) {
                continue;
            }
            let id = self.pricing[i].route_id;
            if let Some(route) = self
                .routes
                .iter()
                .find(|r| r.public_id() == id && r.plan.is_some())
                && let Some(plane) = self.fleet.iter().find(|p| Some(p.id) == route.aircraft_id)
            {
                let old = route.ticket_eur;
                let old_business = route.business_ticket_eur;
                let mut suggestion = review(self, route, plane, &self.market_routes(), false)?;
                if self.routes.iter().filter(|r| r.public_id() == id).any(|r| {
                    self.fleet
                        .iter()
                        .any(|p| Some(p.id) == r.aircraft_id && p.business_seats > 0)
                }) {
                    suggestion.business = Some(crate::cabins::premium_fare(suggestion.fare));
                }
                self.set_ticket_price(id, suggestion.fare)?;
                if let Some(b) = suggestion.business {
                    self.set_business_ticket_price(id, b)?;
                }
                if suggestion.fare != old || suggestion.business.is_some_and(|b| b != old_business)
                {
                    self.pricing[i].last_change = Some(Change {
                        at: now,
                        old,
                        new: suggestion.fare,
                        old_business: suggestion.business.map(|_| old_business),
                        new_business: suggestion.business,
                    });
                }
            }
            self.pricing[i].next_review = Some(
                now.checked_add(RULES.fare_review_seconds)
                    .ok_or("Zeitüberlauf")?,
            );
        }
        Ok(())
    }
    pub fn validate_pricing(&self) -> Result<(), String> {
        let mut ids = std::collections::HashSet::new();
        let now = self.clock.elapsed().as_secs();
        for p in &self.pricing {
            if !ids.insert(p.route_id)
                || !self
                    .routes
                    .iter()
                    .any(|r| r.id == p.route_id && r.parent_route_id.is_none())
                || p.next_review
                    .is_some_and(|t| t <= now || t - now > RULES.fare_review_seconds)
                || p.last_change.as_ref().is_some_and(|c| {
                    c.at > now
                        || !(0..=100_000).contains(&c.old)
                        || !(0..=100_000).contains(&c.new)
                        || c.old_business.is_some_and(|b| !(0..=100_000).contains(&b))
                        || c.new_business.is_some_and(|b| !(0..=100_000).contains(&b))
                })
            {
                return Err("Ungültiger Preisautopilot".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pricing_ties_keep_current_and_other_ties_choose_lower() {
        assert_eq!(super::search(90, 1, |_| Ok(123)).unwrap(), (90, 123));
        assert_eq!(
            super::search(90, 1, |p| Ok(if p == 90 { 0 } else { 1 })).unwrap(),
            (80, 1)
        );
    }
}
