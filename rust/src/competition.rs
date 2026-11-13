//! Small, deterministic companies using exactly the player's operations and ledger.
use crate::{clock::Clock, game::Game, routes::Route};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const REVIEW: u64 = 7 * 86400;
/// Weekly reviews a stopped service waits before the market is checked again.
const RETRY_AFTER: u32 = 4;
/// Departure interval used by AI services, in minutes.
const AI_INTERVAL_MINUTES: u32 = 360;
const MAX_FLEET: usize = 6;
const RESERVE: i64 = 1_000_000;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RouteReview {
    pub route_id: u32,
    pub profit: i64,
    pub losses: u32,
    /// Landings at the last review; the weekly result is charged the planned
    /// maintenance share of the landings since. `None` in older saves: that
    /// first week is judged without it.
    #[serde(default)]
    pub flights: Option<u64>,
}

/// Planned check cost per landing on `route`, part of every AI profit estimate.
fn maintenance_per_flight(route: &Route, model: &str) -> Result<i64, String> {
    let (flight, _) = crate::operations::timings(route, model)?;
    crate::maintenance::cost_per_flight(model, flight)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Competitor {
    pub game: Box<Game>,
    pub next_review: u64,
    pub losing_reviews: u32,
    pub last_profit: i64,
    pub last_action: String,
    /// Reviews since the last service was stopped; restart is checked at RETRY_AFTER.
    #[serde(default)]
    pub idle_reviews: u32,
    #[serde(default)]
    pub route_reviews: Vec<RouteReview>,
    #[serde(default)]
    pub growth_reviews: u32,
}

pub fn initial_competitors() -> Result<Vec<Competitor>, String> {
    initial_competitors_at(Duration::ZERO)
}

pub fn initial_competitors_at(now: Duration) -> Result<Vec<Competitor>, String> {
    let mut companies = Vec::new();
    for (name, origin, destination, model, price) in [
        ("Regional Wings", "EDDF", "EDDM", "atr72_600", 90),
        ("Continental Sky", "EDDF", "LTFM", "a320_200", 150),
    ] {
        let airline = crate::game::Airline {
            name: name.into(),
            country_code: "DE".into(),
            home_airport_icao: origin.into(),
            capital_eur: crate::game::starting_capital()?,
        };
        airline.validate()?;
        let mut g = Game {
            clock: Clock::from_state(now, false, 1).unwrap(),
            history: Some(crate::history::History::new(
                now.as_secs(),
                airline.capital_eur,
            )),
            airline: Some(airline),
            ..Game::default()
        };
        g.buy_aircraft(model)?;
        g.create_route(origin, destination, 1)?;
        g.set_ticket_price(1, price)?;
        g.schedule(1, 1, AI_INTERVAL_MINUTES)?;
        companies.push(Competitor {
            game: Box::new(g),
            next_review: now
                .as_secs()
                .checked_add(REVIEW)
                .ok_or("KI-Zeitüberlauf.")?,
            losing_reviews: 0,
            last_profit: 0,
            last_action: "Flugbetrieb aufgenommen".into(),
            idle_reviews: 0,
            route_reviews: Vec::new(),
            growth_reviews: 0,
        });
    }
    Ok(companies)
}

impl Game {
    pub fn market_routes(&self) -> Vec<Route> {
        self.routes
            .iter()
            .chain(self.competitors.iter().flat_map(|c| c.game.routes.iter()))
            .cloned()
            .collect()
    }

    pub fn validate_competitors(&self) -> Result<(), String> {
        if self.competitors.len() > 4 || (self.airline.is_none() && !self.competitors.is_empty()) {
            return Err("Ungültige KI-Airlines.".into());
        }
        let mut names = std::collections::HashSet::new();
        for c in &self.competitors {
            let g = &c.game;
            if !g.clock.is_valid()
                || !g.competitors.is_empty()
                || g.clock.elapsed() != self.clock.elapsed()
                || c.next_review <= self.clock.elapsed().as_secs()
                || c.next_review - self.clock.elapsed().as_secs() > REVIEW
                || c.losing_reviews > 2
                || c.idle_reviews > RETRY_AFTER
                || c.last_action.len() > 200
                || c.growth_reviews > RETRY_AFTER
            {
                return Err("Widersprüchlicher KI-Zustand.".into());
            }
            let a = g.airline.as_ref().ok_or("KI-Airline fehlt.")?;
            if !names.insert(&a.name) {
                return Err("Doppelte KI-Airline.".into());
            }
            a.validate()?;
            g.validate_fleet()?;
            g.validate_routes()?;
            g.validate_operations()?;
            g.validate_maintenance()?;
            g.validate_finance()?;
            g.validate_history()?;
            let mut reviewed = std::collections::HashSet::new();
            for r in &c.route_reviews {
                if r.losses > 2
                    || !reviewed.insert(r.route_id)
                    || !g.routes.iter().any(|route| route.id == r.route_id)
                {
                    return Err("Ungültige KI-Streckenprüfung.".into());
                }
            }
            if g.history.is_none() {
                return Err("KI-Finanzhistorie fehlt.".into());
            }
        }
        Ok(())
    }

    pub fn advance(&mut self, seconds: f64) -> bool {
        let mut target = self.clock.clone();
        if !target.advance(seconds) {
            return false;
        }
        if target.elapsed() == self.clock.elapsed() {
            return true;
        }
        // Events, reviews and cash postings are on whole-second boundaries.
        // Subsecond frames need only update the authoritative clocks, not clone
        // every company's fleet, recent results and lifetime archive.
        if target.elapsed().as_secs() == self.clock.elapsed().as_secs() {
            for c in &mut self.competitors {
                c.game.clock = Clock::from_state(target.elapsed(), true, 1).unwrap();
            }
            self.clock = target;
            return true;
        }
        if self.competitors.is_empty()
            && self
                .next_maintenance_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
        {
            return self.advance_company_to(target, &self.routes.clone());
        }
        // Transactional across all companies: no partial time or cash updates.
        let mut staged = self.clone();
        while staged.clock.elapsed() < target.elapsed() {
            // Older saves may already be parked after a deferred loss review.
            // Close these at the current instant, never move the clock backward.
            for c in &mut staged.competitors {
                let parked: Vec<_> = c
                    .route_reviews
                    .iter()
                    .filter(|r| r.losses >= 2)
                    .filter_map(|r| {
                        let route = c
                            .game
                            .routes
                            .iter()
                            .find(|route| route.id == r.route_id && route.plan.is_some())?;
                        let plane = c
                            .game
                            .fleet
                            .iter()
                            .find(|p| Some(p.id) == route.aircraft_id)?;
                        (plane.status == crate::aircraft::AircraftStatus::Parked)
                            .then(|| (route.id, plane.id, plane.location_icao.clone()))
                    })
                    .collect();
                for (route, plane, location) in parked {
                    if c.close_loss_route(route, plane, location).is_err() {
                        return false;
                    }
                }
            }
            let review = staged
                .competitors
                .iter()
                .map(|c| c.next_review)
                .min()
                .unwrap_or(u64::MAX);
            // Maintenance starts, ends and forced groundings of every company.
            let maintenance = std::iter::once(staged.next_maintenance_event())
                .chain(
                    staged
                        .competitors
                        .iter()
                        .map(|c| c.game.next_maintenance_event()),
                )
                .flatten()
                .min()
                .unwrap_or(u64::MAX);
            // A loss review can occur in flight every week at the same phase.
            // Stop at the next ground-time end, not at an arbitrary later review.
            let closures: Vec<_> = staged
                .competitors
                .iter()
                .enumerate()
                .flat_map(|(i, c)| {
                    c.route_reviews
                        .iter()
                        .filter(|r| r.losses >= 2)
                        .filter_map(move |r| {
                            let route = c
                                .game
                                .routes
                                .iter()
                                .find(|route| route.id == r.route_id && route.plan.is_some())?;
                            let plane = c
                                .game
                                .fleet
                                .iter()
                                .find(|p| Some(p.id) == route.aircraft_id)?;
                            let state = crate::operations::movement(
                                route,
                                &plane.model_id,
                                c.game.clock.elapsed().as_secs(),
                            )
                            .ok()?;
                            Some((i, route.id, plane.id, state.ready, state.destination))
                        })
                })
                .collect();
            let boundary = closures
                .iter()
                .map(|c| c.3)
                .min()
                .unwrap_or(review)
                .min(review)
                .min(maintenance);
            // An event already due (e.g. after loading) never moves the clock back.
            let end = target
                .elapsed()
                .min(Duration::from_secs(boundary).max(staged.clock.elapsed()));
            let supply = staged.market_routes();
            // Exact integer/nanosecond target, not repeated floating-point conversion.
            if !staged.advance_company_to(Clock::from_state(end, true, 1).unwrap(), &supply) {
                return false;
            }
            for c in &mut staged.competitors {
                if !c
                    .game
                    .advance_company_to(Clock::from_state(end, true, 1).unwrap(), &supply)
                {
                    return false;
                }
            }
            for (i, route, plane, ready, destination) in closures {
                if Duration::from_secs(ready) == end {
                    let c = &mut staged.competitors[i];
                    // Also handles a zero idle gap: do not dispatch another flight
                    // at the exact instant the previous ground time ends.
                    if c.close_loss_route(route, plane, destination).is_err() {
                        return false;
                    }
                }
            }
            if staged.apply_maintenance().is_err()
                || staged
                    .competitors
                    .iter_mut()
                    .any(|c| c.game.apply_maintenance().is_err())
            {
                return false;
            }
            // All companies settle against the same immutable offer snapshot first.
            for i in 0..staged.competitors.len() {
                let supply = staged.market_routes();
                let c = &mut staged.competitors[i];
                if c.next_review == end.as_secs() && !c.review(&supply) {
                    return false;
                }
            }
        }
        staged.clock = target;
        *self = staged;
        true
    }
}

impl Competitor {
    fn close_loss_route(
        &mut self,
        route: u32,
        plane: u32,
        destination: String,
    ) -> Result<(), String> {
        self.game.route_mut(route)?.plan = None;
        let p = self
            .game
            .fleet
            .iter_mut()
            .find(|p| p.id == plane)
            .ok_or("KI-Flugzeug fehlt")?;
        p.status = crate::aircraft::AircraftStatus::Parked;
        p.location_icao = destination;
        self.game.delete_route(route)?;
        self.route_reviews.retain(|r| r.route_id != route);
        self.idle_reviews = 0;
        self.growth_reviews = 0;
        self.last_action = "Verluststrecke nach Landung und Bodenzeit geschlossen".into();
        Ok(())
    }

    fn review(&mut self, supply: &[Route]) -> bool {
        let mut other_supply = supply.to_vec();
        for r in &self.game.routes {
            if let Some(i) = other_supply.iter().position(|o| o == r) {
                other_supply.remove(i);
            }
        }
        let maintenance = self.game.history.as_ref().map_or(0, |h| {
            h.maintenance_through(self.game.clock.elapsed().as_secs())
                .clamp(0, i128::from(i64::MAX)) as i64
        });
        let Some(profit) = self.game.finance.totals.profit().checked_sub(maintenance) else {
            return false;
        };
        let Some(period) = profit.checked_sub(self.last_profit) else {
            return false;
        };
        self.last_profit = profit;
        self.losing_reviews = if period < 0 {
            (self.losing_reviews + 1).min(2)
        } else {
            0
        };
        let mut closed = Vec::new();
        for i in 0..self.game.routes.len() {
            let r = &self.game.routes[i];
            if r.plan.is_none() {
                // Suspended for maintenance: resumes on its own afterwards.
                if self
                    .game
                    .fleet
                    .iter()
                    .any(|p| Some(p.id) == r.aircraft_id && p.maintenance.is_some())
                {
                    continue;
                }
                if !self.retry(i, supply) {
                    return false;
                }
                continue;
            }
            let id = r.id;
            let Some(plane) = self.game.fleet.iter().find(|p| Some(p.id) == r.aircraft_id) else {
                return false;
            };
            let Ok(maintenance) = maintenance_per_flight(r, &plane.model_id) else {
                return false;
            };
            let totals = self
                .game
                .history
                .as_ref()
                .unwrap()
                .totals(id, self.game.clock.elapsed().as_secs());
            let (total, flights) = (totals.profit(), totals.flights.max(0) as u64);
            let previous = self
                .route_reviews
                .iter()
                .find(|s| s.route_id == id)
                .cloned()
                .unwrap_or(RouteReview {
                    flights: Some(0),
                    ..Default::default()
                });
            let landed = previous.flights.map_or(0, |f| flights.saturating_sub(f));
            let week = i128::from(total)
                - i128::from(previous.profit)
                - i128::from(landed) * i128::from(maintenance);
            let losses = if week < 0 {
                (previous.losses + 1).min(2)
            } else {
                0
            };
            self.route_reviews.retain(|s| s.route_id != id);
            self.route_reviews.push(RouteReview {
                route_id: id,
                profit: total,
                losses,
                flights: Some(flights),
            });
            if losses >= 2 && self.game.stop_schedule(id).is_ok() {
                self.last_action = "Verlustverbindung nach zwei Verlustwochen eingestellt".into();
                self.idle_reviews = 0;
                self.growth_reviews = 0;
                closed.push(id);
                continue;
            }
            let r = &self.game.routes[i];
            let Some(plane) = self.game.fleet.iter().find(|p| Some(p.id) == r.aircraft_id) else {
                return false;
            };
            // Evaluate nearby fares with the same demand/cost functions as players.
            let old = r.ticket_eur;
            let mut best = (i64::MIN, old);
            for price in [old.saturating_sub(10).max(0), old, (old + 10).min(100_000)] {
                let mut candidate = r.clone();
                candidate.ticket_eur = price;
                let mut offers = supply.to_vec();
                // Replace precisely this offer, not an extra fictional service.
                if let Some(index) = offers.iter().position(|o| o == r) {
                    offers.remove(index);
                }
                offers.push(candidate.clone());
                let Ok(t) = crate::economy::quote_shared(&candidate, &plane.model_id, &offers)
                else {
                    return false;
                };
                if t.profit() > best.0 || (t.profit() == best.0 && price == old) {
                    best = (t.profit(), price);
                }
            }
            if self.game.set_ticket_price(id, best.1).is_err() {
                return false;
            }
            self.last_action = format!("Preisprüfung: {old} → {} EUR", best.1);
        }
        for id in &closed {
            if self.game.delete_route(*id).is_err() {
                return false;
            }
            self.route_reviews.retain(|s| s.route_id != *id);
        }
        if !closed.is_empty() {
            self.last_action = format!(
                "{} Verluststrecke(n) nach zwei Verlustwochen geschlossen · Flugzeuge bleiben erhalten",
                closed.len()
            );
        }
        self.growth_reviews = (self.growth_reviews + 1).min(RETRY_AFTER);
        if closed.is_empty() && self.growth_reviews >= RETRY_AFTER {
            self.growth_reviews = 0;
            if self.expand(&other_supply).is_err() {
                return false;
            }
        }
        let Some(next) = self.next_review.checked_add(REVIEW) else {
            return false;
        };
        self.plan_maintenance(next);
        self.next_review = next;
        true
    }

    /// Same rules and prices as the player: request a regular check for every
    /// aircraft that would otherwise reach its limit before the next review.
    /// Without enough money the unscheduled check follows automatically.
    fn plan_maintenance(&mut self, next_review: u64) {
        for p in 0..self.game.fleet.len() {
            let id = self.game.fleet[p].id;
            if self.game.due_landing(p).is_some_and(|t| t <= next_review)
                && self.game.request_maintenance(id).is_ok()
            {
                self.last_action =
                    format!("Wartung für {} eingeplant", self.game.fleet[p].identifier());
            }
        }
    }

    /// Bounded catalog search, no flight loop or separate economic model.
    fn expand(&mut self, supply: &[Route]) -> Result<(), String> {
        let g = &self.game;
        let airline = g.airline.as_ref().ok_or("KI-Airline fehlt")?;
        // Caller removed precisely our pre-review offers, including closed routes.
        let mut offers = supply.to_vec();
        offers.extend(g.routes.iter().cloned());
        let mut committed_weekly_costs = 0;
        for route in g.routes.iter().filter(|r| r.plan.is_some()) {
            let plane = g
                .fleet
                .iter()
                .find(|p| Some(p.id) == route.aircraft_id)
                .ok_or("KI-Flugzeug fehlt")?;
            let t = crate::economy::quote_shared(route, &plane.model_id, &offers)?;
            committed_weekly_costs += (t.costs() + maintenance_per_flight(route, &plane.model_id)?)
                * (REVIEW / route.plan.as_ref().unwrap().interval_seconds) as i64;
        }
        let free: Vec<_> = g
            .fleet
            .iter()
            .filter(|p| {
                p.status == crate::aircraft::AircraftStatus::Parked
                    && p.maintenance.is_none()
                    && crate::maintenance::rules(&p.model_id).is_ok_and(|r| !p.wear.is_due(r))
                    && !g.routes.iter().any(|r| r.aircraft_id == Some(p.id))
            })
            .collect();
        let mut options: Vec<_> = free
            .iter()
            .map(|p| (Some(p.id), p.model_id.as_str(), p.location_icao.as_str(), 0))
            .collect();
        if free.is_empty() && g.fleet.len() < MAX_FLEET {
            for m in crate::aircraft::models()? {
                if airline.capital_eur.saturating_sub(m.price_eur) >= RESERVE {
                    options.push((None, &m.id, &airline.home_airport_icao, m.price_eur));
                }
            }
        }
        let mut best = None;
        let mut best_week = 0;
        for (plane, model, origin, cost) in options {
            let m = crate::aircraft::model(model)?;
            for airport in &crate::airports::catalog()?.airports {
                if airport.icao == origin {
                    continue;
                }
                let mut r = Route {
                    id: g.next_route_id,
                    origin_icao: origin.into(),
                    destination_icao: airport.icao.clone(),
                    aircraft_id: plane,
                    ticket_eur: 0,
                    plan: None,
                };
                if g.routes
                    .iter()
                    .any(|old| crate::demand::same_market(old, &r))
                {
                    continue;
                }
                let km = crate::routes::distance_km(origin, &airport.icao)?;
                if km > f64::from(m.range_km) {
                    continue;
                }
                let (flight, ground) = crate::operations::timings(&r, model)?;
                let maintenance = crate::maintenance::cost_per_flight(model, flight)?;
                let interval = u64::from(AI_INTERVAL_MINUTES).max((flight + ground).div_ceil(60));
                r.plan = Some(crate::operations::FlightPlan {
                    first_departure: g.clock.elapsed().as_secs() + 60,
                    interval_seconds: interval * 60,
                    ..Default::default()
                });
                for step in 3..=15 {
                    r.ticket_eur = (crate::economy::reference_price(km) * f64::from(step) / 10.0)
                        .round() as i64;
                    let mut market = offers.clone();
                    market.push(r.clone());
                    let t = crate::economy::quote_shared(&r, model, &market)?;
                    let week = (t.profit() - maintenance) * (REVIEW / (interval * 60)) as i64;
                    // Reserve also covers four weeks of gross operating costs.
                    let buffer = ((committed_weekly_costs
                        + (t.costs() + maintenance) * (REVIEW / (interval * 60)) as i64)
                        * 4)
                    .max(RESERVE);
                    if week > best_week
                        && week >= 5_000
                        && week * 104 >= cost
                        && airline.capital_eur.saturating_sub(cost) >= buffer
                    {
                        best_week = week;
                        best = Some((plane, model.to_string(), r.clone(), interval as u32));
                    }
                }
            }
        }
        let Some((plane, model, route, interval)) = best else {
            self.last_action =
                "Expansion geprüft · Budget/Reserve oder Markt nicht rentabel".into();
            return Ok(());
        };
        let plane = match plane {
            Some(id) => id,
            None => {
                self.game.buy_aircraft(&model)?;
                self.game.fleet.last().unwrap().id
            }
        };
        self.game
            .create_route(&route.origin_icao, &route.destination_icao, plane)?;
        self.game.set_ticket_price(route.id, route.ticket_eur)?;
        self.game.schedule(route.id, 1, interval)?;
        self.last_action = format!(
            "{}–{} eröffnet · {} · erwartet {best_week} EUR/Woche",
            route.origin_icao,
            route.destination_icao,
            if route.aircraft_id.is_some() {
                "freie Maschine"
            } else {
                "Flugzeug gekauft"
            }
        );
        Ok(())
    }

    /// A stopped service waits RETRY_AFTER reviews, then restarts at the most
    /// profitable fare in the current shared market, or waits another period.
    /// Returns false only on an internal error.
    fn retry(&mut self, i: usize, supply: &[Route]) -> bool {
        self.idle_reviews = (self.idle_reviews + 1).min(RETRY_AFTER);
        if self.idle_reviews < RETRY_AFTER {
            return true;
        }
        self.idle_reviews = 0;
        let r = &self.game.routes[i];
        let Some(plane) = self.game.fleet.iter().find(|p| Some(p.id) == r.aircraft_id) else {
            return false;
        };
        let model = plane.model_id.clone();
        let location = plane.location_icao.clone();
        let Ok(km) = crate::routes::distance_km(&r.origin_icao, &r.destination_icao) else {
            return false;
        };
        let reference = crate::economy::reference_price(km);
        let now = self.game.clock.elapsed().as_secs();
        let mut candidate = r.clone();
        // The aircraft stays where it last landed; serve the route from there.
        if location == candidate.destination_icao {
            std::mem::swap(&mut candidate.origin_icao, &mut candidate.destination_icao);
        }
        candidate.plan = Some(crate::operations::FlightPlan {
            first_departure: now.saturating_add(60),
            interval_seconds: u64::from(AI_INTERVAL_MINUTES) * 60,
            ..Default::default()
        });
        let Ok(maintenance) = maintenance_per_flight(&candidate, &model) else {
            return false;
        };
        let mut best: Option<(i64, i64)> = None;
        for step in 3..=15 {
            let price = ((reference * f64::from(step) / 10.0).round() as i64).clamp(0, 100_000);
            candidate.ticket_eur = price;
            let mut offers = supply.to_vec();
            offers.push(candidate.clone());
            let Ok(t) = crate::economy::quote_shared(&candidate, &model, &offers) else {
                return false;
            };
            let profit = t.profit() - maintenance;
            if best.is_none_or(|(best, _)| profit > best) {
                best = Some((profit, price));
            }
        }
        let Some((profit, price)) = best.filter(|(profit, _)| *profit > 0) else {
            self.last_action = "Markt erneut geprüft · Wiedereinstieg noch nicht rentabel".into();
            return true;
        };
        let id = r.id;
        let route = &mut self.game.routes[i];
        route.origin_icao = candidate.origin_icao;
        route.destination_icao = candidate.destination_icao;
        if self.game.set_ticket_price(id, price).is_err()
            || self.game.schedule(id, 1, AI_INTERVAL_MINUTES).is_err()
        {
            return false;
        }
        self.losing_reviews = 0;
        self.last_action =
            format!("Verbindung wieder aufgenommen zu {price} EUR (erwartet {profit} EUR je Flug)");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        let mut g = Game::default();
        g.found("Player", "DE", "EDDF").unwrap();
        g.buy_aircraft("atr72_600").unwrap();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.set_ticket_price(1, 90).unwrap();
        g.schedule(1, 1, 360).unwrap();
        g.clock.set_running(true);
        g
    }

    fn state(g: &Game) -> serde_json::Value {
        serde_json::to_value(g).unwrap()
    }

    #[test]
    fn airborne_loss_reviews_close_after_ground_time_even_across_restart() {
        for interval in [360, 66, 0] {
            let mut g = game();
            let c = &mut g.competitors[0];
            c.game.stop_schedule(1).unwrap();
            c.game.set_ticket_price(1, 100_000).unwrap();
            c.game.schedule(1, 359, interval.max(66)).unwrap();
            if interval == 0 {
                let (flight, ground) =
                    crate::operations::timings(&c.game.routes[0], "atr72_600").unwrap();
                c.game.routes[0].plan.as_mut().unwrap().interval_seconds = flight + ground;
            }
            assert!(g.advance(2.0 * REVIEW as f64));
            let mut small = g.clone();
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("closing.json");
            crate::save::save(&path, &g).unwrap();
            let mut loaded = crate::save::load(&path).unwrap();
            assert!(g.advance(86400.0));
            for _ in 0..1440 {
                assert!(small.advance(60.0));
            }
            assert!(loaded.advance(86400.0));
            assert!(
                g.competitors[0].game.routes.is_empty(),
                "losses must not wait for a parked weekly phase"
            );
            assert_eq!(state(&g), state(&small));
            assert_eq!(state(&g), state(&loaded));
            g.validate_competitors().unwrap();
        }
        // A pre-fix save may have reached a parked, fractional-second state
        // while still waiting for a third weekly review.
        let mut g = game();
        assert!(g.advance(86400.125));
        let c = &mut g.competitors[0];
        assert_eq!(
            c.game.fleet[0].status,
            crate::aircraft::AircraftStatus::Parked
        );
        c.route_reviews = vec![RouteReview {
            route_id: 1,
            profit: 0,
            losses: 2,
            flights: None,
        }];
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy-pending.json");
        crate::save::save(&path, &g).unwrap();
        let mut g = crate::save::load(&path).unwrap();
        assert!(g.advance(1.0));
        assert!(g.competitors[0].game.routes.is_empty());
        g.validate_competitors().unwrap();
    }

    #[test]
    fn previous_v9_saves_default_new_review_fields_and_reject_corruption() {
        let g = game();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("previous-v9.json");
        crate::save::save(&path, &g).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for c in value["state"]["competitors"].as_array_mut().unwrap() {
            c.as_object_mut().unwrap().remove("route_reviews");
            c.as_object_mut().unwrap().remove("growth_reviews");
        }
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(state(&g), state(&crate::save::load(&path).unwrap()));
        value["state"]["competitors"][0]["route_reviews"] = serde_json::json!([
            {"route_id": 999, "profit": 0, "losses": 2}
        ]);
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(crate::save::load(&path).is_err());
    }

    #[test]
    fn expansion_reuses_parked_aircraft_and_competes_for_existing_market() {
        let mut c = initial_competitors().unwrap().remove(0);
        c.game.stop_schedule(1).unwrap();
        c.game.delete_route(1).unwrap();
        let before = c.game.airline.as_ref().unwrap().capital_eur;
        let mut rival = game().routes[0].clone();
        rival.ticket_eur = 90;
        c.expand(&[rival.clone()]).unwrap();
        assert_eq!(c.game.fleet.len(), 1);
        assert_eq!(c.game.airline.as_ref().unwrap().capital_eur, before);
        assert_eq!(c.game.routes.len(), 1);
        let r = &c.game.routes[0];
        assert_eq!(r.origin_icao, c.game.fleet[0].location_icao);
        let isolated = crate::demand::passengers(r, std::slice::from_ref(r)).unwrap();
        let shared = crate::demand::passengers(r, &[r.clone(), rival.clone()]).unwrap();
        assert!(shared < isolated);
        let base = crate::demand::market(r, 0).unwrap().total();
        let combined =
            (shared + crate::demand::passengers(&rival, &[r.clone(), rival.clone()]).unwrap()) * 2;
        assert!(combined as f64 <= base);
    }

    #[test]
    fn expansion_spends_real_cash_and_obeys_budget_range_and_fleet_limit() {
        let mut g = game();
        assert!(g.advance(52.0 * REVIEW as f64));
        let c = g
            .competitors
            .iter()
            .find(|c| c.game.fleet.len() > 1)
            .expect("a profitable airline should eventually expand");
        assert!(c.game.routes.len() > 1);
        assert!(c.game.airline.as_ref().unwrap().capital_eur >= RESERVE);
        assert_eq!(
            c.game
                .history
                .as_ref()
                .unwrap()
                .balance(c.game.clock.elapsed().as_secs()),
            c.game.airline.as_ref().unwrap().capital_eur
        );
        g.validate_competitors().unwrap();
        let mut poor = initial_competitors().unwrap().remove(0);
        poor.game.airline.as_mut().unwrap().capital_eur = 100;
        let fleet = poor.game.fleet.clone();
        poor.expand(&[]).unwrap();
        assert_eq!(poor.game.fleet, fleet);
        assert_eq!(poor.game.airline.as_ref().unwrap().capital_eur, 100);
        assert!(g.advance(365.0 * 86400.0 * 5.0));
        for c in &g.competitors {
            assert!(c.game.fleet.len() <= MAX_FLEET);
            c.game.validate_routes().unwrap();
            assert!(c.game.history.as_ref().unwrap().batches.len() < 5000);
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("expanded.json");
        crate::save::save(&path, &g).unwrap();
        assert_eq!(state(&g), state(&crate::save::load(&path).unwrap()));
    }

    #[test]
    fn ai_airlines_pay_for_regular_checks_before_reaching_the_limit() {
        let mut g = game();
        g.competitors.truncate(1);
        let mut fast = g.clone();
        let mut forced = false;
        for _ in 0..365 {
            assert!(g.advance(86400.0));
            forced |= g.competitors[0]
                .game
                .fleet
                .iter()
                .any(|p| p.maintenance.as_ref().is_some_and(|v| v.unscheduled));
        }
        assert!(fast.advance(365.0 * 86400.0));
        assert_eq!(state(&g), state(&fast));
        let c = &g.competitors[0].game;
        assert!(!forced, "funded AI should plan checks before the limit");
        assert!(c.fleet.iter().map(|p| p.wear.checks).sum::<u64>() >= 2);
        let h = c.history.as_ref().unwrap();
        assert!(!h.maintenance.is_empty());
        let r = crate::maintenance::rules("atr72_600").unwrap();
        assert!(h.maintenance.iter().all(|m| m.amount >= r.base_cost_eur));
        assert_eq!(
            h.balance(c.clock.elapsed().as_secs()),
            c.airline.as_ref().unwrap().capital_eur
        );
        g.validate_competitors().unwrap();
        // A broke AI cannot request a check; the forced one follows at landing.
        let c = &mut g.competitors[0];
        let shift = c.game.airline.as_ref().unwrap().capital_eur + 1_000_000_000;
        c.game.history.as_mut().unwrap().opening_balance -= shift;
        c.game.airline.as_mut().unwrap().capital_eur -= shift;
        let mut forced = false;
        for _ in 0..200 {
            assert!(g.advance(86400.0));
            forced |= g.competitors[0]
                .game
                .fleet
                .iter()
                .any(|p| p.maintenance.as_ref().is_some_and(|v| v.unscheduled));
        }
        assert!(forced, "unfunded aircraft are grounded at the limit");
        g.validate_competitors().unwrap();
    }

    #[test]
    fn route_reviews_charge_the_planned_maintenance_share_per_landing() {
        let mut g = game();
        g.competitors.truncate(1);
        assert!(g.advance(3.0 * 86400.0));
        let supply = g.market_routes();
        let base = g.competitors[0].clone();
        let route = base.game.routes[0].clone();
        let maintenance = maintenance_per_flight(&route, &base.game.fleet[0].model_id).unwrap();
        assert!(maintenance > 1);
        let t = base
            .game
            .history
            .as_ref()
            .unwrap()
            .totals(route.id, 3 * 86400);
        assert!(t.flights > 1);
        let losses = |gain: i64, flights: Option<u64>| {
            let mut c = base.clone();
            c.route_reviews = vec![RouteReview {
                route_id: route.id,
                profit: t.profit() - gain,
                losses: 0,
                flights,
            }];
            assert!(c.review(&supply));
            let r = c
                .route_reviews
                .iter()
                .find(|r| r.route_id == route.id)
                .unwrap();
            assert_eq!(r.flights, Some(t.flights as u64));
            r.losses
        };
        let since = Some(t.flights as u64 - 1);
        // One landing earned less than its maintenance share: a losing week.
        assert_eq!(losses(maintenance - 1, since), 1);
        assert_eq!(losses(maintenance, since), 0);
        // Older saves without a landing count judge that first week as before.
        assert_eq!(losses(1, None), 0);
    }

    #[test]
    fn losses_are_measured_per_route_not_hidden_by_profitable_routes() {
        let mut g = game();
        let c = &mut g.competitors[0];
        c.game.buy_aircraft("atr72_600").unwrap();
        c.game.create_route("EDDF", "EDDM", 2).unwrap();
        c.game.schedule(2, 1, 360).unwrap();
        c.game.routes[0].ticket_eur = 100_000;
        assert!(g.advance(2.0 * REVIEW as f64));
        let c = &g.competitors[0];
        assert!(!c.game.routes.iter().any(|r| r.id == 1));
        assert!(c.game.routes.iter().any(|r| r.id == 2 && r.plan.is_some()));
        assert_eq!(c.game.fleet.len(), 2);
        assert!(
            c.game
                .history
                .as_ref()
                .unwrap()
                .totals(1, 2 * REVIEW)
                .flights
                > 0
        );
        g.validate_competitors().unwrap();
    }

    #[test]
    fn common_market_price_frequency_capacity_and_separate_ledgers() {
        let mut g = game();
        let route = &g.routes[0];
        let supply = g.market_routes();
        let own = crate::demand::passengers(route, &supply).unwrap();
        let rival = &g.competitors[0].game.routes[0];
        assert_eq!(own, crate::demand::passengers(rival, &supply).unwrap());
        let base = crate::demand::market(route, 0).unwrap().total();
        assert!((own * 4) as f64 <= base); // two airlines × two departures/direction
        let mut cheap = route.clone();
        cheap.ticket_eur = 50;
        let offers = vec![cheap.clone(), rival.clone()];
        let cheap_pax = crate::demand::passengers(&cheap, &offers).unwrap();
        let rival_pax = crate::demand::passengers(rival, &offers).unwrap();
        assert!(cheap_pax > own && rival_pax < own);
        let cheap_market = crate::demand::market(&cheap, 0).unwrap().total();
        assert!(((cheap_pax + rival_pax) * 2) as f64 <= cheap_market);
        let mut frequent = route.clone();
        frequent.plan.as_mut().unwrap().interval_seconds /= 2;
        let offers = vec![frequent.clone(), rival.clone()];
        let f = crate::demand::passengers(&frequent, &offers).unwrap();
        let r = crate::demand::passengers(rival, &offers).unwrap();
        assert!(f * 4 > r * 2); // frequency changes daily share, not market size
        assert!((f * 4 + r * 2) as f64 <= base);
        let mut free = route.clone();
        free.ticket_eur = 0;
        free.plan.as_mut().unwrap().interval_seconds = 86400;
        let t = crate::economy::quote(&free, "atr72_600").unwrap();
        assert_eq!(t.passengers, t.seats); // spare demand cannot exceed aircraft capacity
        assert!(crate::economy::quote(&free, "a320_200").unwrap().passengers > t.passengers);
        let before = g.airline.as_ref().unwrap().capital_eur;
        assert!(g.advance(86400.0));
        assert_eq!(g.finance.totals.flights, 4);
        assert_eq!(g.competitors[0].game.finance.totals.flights, 4);
        assert_eq!(
            g.airline.as_ref().unwrap().capital_eur,
            before + g.finance.totals.profit()
        );
        g.validate_competitors().unwrap();
        g.validate_history().unwrap();
    }

    #[test]
    fn mixed_fares_and_frequencies_never_duplicate_the_base_market() {
        let g = game();
        let base = crate::demand::market(&g.routes[0], 0).unwrap().total();
        for p in [0, 50, 90, 130, 100_000] {
            for q in [0, 50, 90, 130, 100_000] {
                for interval in [7200, 21600, 86400 * 7] {
                    let mut a = g.routes[0].clone();
                    let mut b = g.competitors[0].game.routes[0].clone();
                    a.ticket_eur = p;
                    b.ticket_eur = q;
                    b.plan.as_mut().unwrap().interval_seconds = interval;
                    std::mem::swap(&mut b.origin_icao, &mut b.destination_icao);
                    let offers = vec![a.clone(), b.clone()];
                    let daily: f64 = offers
                        .iter()
                        .map(|r| {
                            let frequency =
                                43200.0 / r.plan.as_ref().unwrap().interval_seconds as f64;
                            crate::demand::passengers(r, &offers).unwrap() as f64 * frequency
                        })
                        .sum();
                    assert!(
                        daily <= base,
                        "duplicated market at fares {p}/{q}, interval {interval}"
                    );
                }
            }
        }
    }

    #[test]
    fn speed_steps_reviews_and_restart_produce_identical_results() {
        for speed in Clock::SPEEDS {
            let mut fast = game();
            fast.clock.set_speed(speed);
            let mut slow = fast.clone();
            assert!(fast.advance(30.0 * 86400.0 / f64::from(speed)));
            for _ in 0..720 {
                assert!(slow.advance(3600.0 / f64::from(speed)));
            }
            assert_eq!(state(&fast), state(&slow));
            assert_ne!(fast.competitors[0].last_action, "Flugbetrieb aufgenommen");
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("multi.json");
            crate::save::save(&path, &fast).unwrap();
            let mut loaded = crate::save::load(&path).unwrap();
            assert_eq!(state(&fast), state(&loaded));
            assert!(loaded.advance(0.0));
            assert_eq!(state(&fast), state(&loaded));
            assert!(fast.advance(86400.0));
            assert!(loaded.advance(86400.0));
            assert_eq!(state(&fast), state(&loaded));
            fast.clock.set_running(false);
            let paused = state(&fast);
            assert!(fast.advance(1e9));
            assert_eq!(paused, state(&fast));
        }
    }

    #[test]
    fn losses_close_service_without_erasing_aircraft_or_finances() {
        let mut g = game();
        // An overpriced rival runs empty but pays exactly the normal operating costs.
        g.competitors[0].game.routes[0].ticket_eur = 100_000;
        assert!(g.advance(2.0 * REVIEW as f64));
        let c = &g.competitors[0];
        assert!(c.game.routes.is_empty());
        assert_eq!(c.game.fleet.len(), 1);
        assert!(c.game.finance.totals.profit() < 0);
        assert_eq!(c.game.finance.totals.passengers, 0);
        let flights = c.game.finance.totals.flights;
        assert!(g.advance(REVIEW as f64));
        assert_eq!(g.competitors[0].game.finance.totals.flights, flights);
        g.validate_competitors().unwrap();
        // After RETRY_AFTER idle reviews the rival re-enters at a viable fare.
        assert!(g.advance(f64::from(RETRY_AFTER - 1) * REVIEW as f64));
        let c = &g.competitors[0];
        let r = &c.game.routes[0];
        assert!(r.plan.is_some(), "{}", c.last_action);
        assert!(r.ticket_eur < 100_000);
        assert_eq!(c.losing_reviews, 0);
        let restarted = c.game.finance.totals.flights;
        assert!(g.advance(86400.0));
        assert!(g.competitors[0].game.finance.totals.flights > restarted);
        g.validate_competitors().unwrap();
        // The restarted state survives a save round trip.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("retry.json");
        crate::save::save(&path, &g).unwrap();
        assert_eq!(state(&g), state(&crate::save::load(&path).unwrap()));
    }

    #[test]
    fn unprofitable_markets_keep_rivals_idle_until_the_next_retry() {
        let mut g = game();
        g.competitors[0].game.routes[0].ticket_eur = 100_000;
        assert!(g.advance(2.0 * REVIEW as f64));
        assert!(g.competitors[0].game.routes.is_empty());
        // Make every fare unprofitable by flooding the market with free seats.
        for _ in 0..3 {
            g.airline.as_mut().unwrap().capital_eur += 200_000_000;
            g.history.as_mut().unwrap().opening_balance += 200_000_000;
            g.buy_aircraft("a320_200").unwrap();
            let id = g.fleet.last().unwrap().id;
            g.create_route("EDDF", "EDDM", id).unwrap();
            let route = g.routes.last().unwrap().id;
            g.set_ticket_price(route, 0).unwrap();
            g.schedule(route, 1, 180).unwrap();
        }
        // No budget for a different market either; free seats block the old market.
        let c = &mut g.competitors[0];
        c.game.airline.as_mut().unwrap().capital_eur = 0;
        let h = c.game.history.as_mut().unwrap();
        h.opening_balance -= h.balance(c.game.clock.elapsed().as_secs());
        assert!(g.advance(f64::from(RETRY_AFTER) * REVIEW as f64));
        let c = &g.competitors[0];
        assert!(c.game.routes.is_empty());
        assert!(
            c.last_action.contains("nicht rentabel"),
            "{}",
            c.last_action
        );
        g.validate_competitors().unwrap();
    }

    #[test]
    fn old_saves_keep_departure_snapshot_and_start_rivals_only_at_load_time() {
        let mut g = game();
        g.competitors.clear();
        assert!(g.advance(100.25));
        let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v8.json");
        crate::save::save(&path, &g).unwrap();
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        v["version"] = 8.into();
        v["state"].as_object_mut().unwrap().remove("competitors");
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.finance, g.finance);
        assert_eq!(loaded.airline, g.airline);
        assert_eq!(loaded.routes, g.routes);
        assert_eq!(loaded.competitors.len(), 2);
        assert!(
            loaded
                .competitors
                .iter()
                .all(|c| c.game.finance.totals.flights == 0)
        );
        assert!(loaded.advance((frozen.landing - 100) as f64));
        assert_eq!(loaded.finance.recent[0], frozen);
        crate::save::save(&path, &loaded).unwrap();
        assert_eq!(state(&loaded), state(&crate::save::load(&path).unwrap()));
    }

    #[test]
    fn price_reviews_preserve_airborne_quotes_for_both_owners() {
        let mut g = game();
        g.competitors[0].next_review = 60;
        assert!(g.advance(59.0));
        let supply = g.market_routes();
        let r = &g.competitors[0].game.routes[0];
        let frozen = crate::operations::departure_quote(r, "atr72_600", 0, &supply).unwrap();
        assert!(g.advance(1.0));
        assert_eq!(
            g.competitors[0].game.routes[0]
                .plan
                .as_ref()
                .unwrap()
                .airborne
                .as_ref(),
            Some(&frozen)
        );
        let player = g.routes[0].plan.as_ref().unwrap().airborne.clone();
        g.set_ticket_price(1, 100_000).unwrap();
        assert_eq!(g.routes[0].plan.as_ref().unwrap().airborne, player);
        assert!(g.advance((frozen.landing - 60) as f64));
        assert_eq!(g.competitors[0].game.finance.recent[0], frozen);
        assert_eq!(g.finance.recent[0], player.unwrap());
        g.validate_competitors().unwrap();
    }

    #[test]
    fn invalid_ai_saves_and_overflow_cannot_partially_commit() {
        let mut g = game();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai.json");
        crate::save::save(&path, &g).unwrap();
        let valid: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for (field, value) in [
            ("next_review", serde_json::json!(0)),
            ("losing_reviews", serde_json::json!(3)),
        ] {
            let mut bad = valid.clone();
            bad["state"]["competitors"][0][field] = value;
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(crate::save::load(&path).is_err());
        }
        for (pointer, value) in [
            (
                "/state/competitors/0/game/clock/speed_index",
                serde_json::json!(99),
            ),
            (
                "/state/competitors/0/game/routes/0/aircraft_id",
                serde_json::json!(99),
            ),
            ("/state/competitors/0/game/airline", serde_json::Value::Null),
            (
                "/state/competitors/0/game/competitors",
                valid["state"]["competitors"].clone(),
            ),
        ] {
            let mut bad = valid.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(
                crate::save::load(&path).is_err(),
                "accepted corrupt AI: {pointer}"
            );
        }
        g.competitors[0].game.airline.as_mut().unwrap().capital_eur = i64::MIN;
        g.competitors[0].game.routes[0].ticket_eur = 100_000;
        let before = state(&g);
        assert!(!g.advance(86400.0));
        assert_eq!(state(&g), before);
    }
}
