//! Small, deterministic companies using exactly the player's operations and ledger.
use crate::{clock::Clock, game::Game, routes::Route};
use serde::{Deserialize, Serialize};
use std::{sync::LazyLock, time::Duration};

/// Weekly reviews a stopped service waits before the market is checked again.
const RETRY_AFTER: u32 = 4;
/// Departure interval used by AI services, in minutes.
const AI_INTERVAL_MINUTES: u32 = 360;
const MAX_FLEET: usize = 6;
const RESERVE: i64 = 1_000_000;

#[derive(Deserialize)]
struct FrequencyRules {
    max_aircraft_per_route: usize,
    load_factor_percent: f64,
    starter_hubs: Vec<StarterHub>,
    remote_hubs: Vec<String>,
}

#[derive(Deserialize)]
struct StarterHub {
    home: String,
    destination: String,
    model: String,
    ticket_eur: i64,
}

static FREQUENCY: LazyLock<FrequencyRules> = LazyLock::new(|| {
    let rules: FrequencyRules = serde_json::from_str(include_str!("../../config/competition.json"))
        .expect("AI frequency rules");
    assert!((1..=MAX_FLEET).contains(&rules.max_aircraft_per_route));
    assert!((0.0..=100.0).contains(&rules.load_factor_percent));
    assert!(rules.starter_hubs.len() >= 2);
    let homes: std::collections::HashSet<_> = rules.starter_hubs.iter().map(|h| &h.home).collect();
    assert_eq!(homes.len(), rules.starter_hubs.len());
    assert!(rules.remote_hubs.len() >= 2 && rules.remote_hubs.iter().all(|h| homes.contains(h)));
    rules
});

#[derive(Clone)]
struct ExpansionQuote {
    route: Route,
    interval: u32,
    business_seats: u32,
    layout_fee: i64,
    profit: i64,
    costs: i64,
}

struct ExpansionChoice {
    used: Option<u64>,
    plane: Option<u32>,
    model: String,
    quote: ExpansionQuote,
    leased: bool,
    score: i64,
}

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

/// Weekly salaries of the crew a service of `model` ties up, plus the technicians
/// still missing for the fleet: part of every AI expansion estimate.
fn staffing_per_week(game: &Game, model: &str) -> Result<i64, String> {
    use crate::staff::{self, Role};
    let n = staff::needs(model)?;
    let (pilots, cabin) = (i64::from(n.pilots), i64::from(n.cabin_crew));
    let techs = i64::from(
        game.technician_target()
            .max(n.technicians)
            .saturating_sub(game.staff_count(Role::Technician)),
    );
    let period = staff::period_seconds()? as i64;
    let monthly = pilots * game.pay_salary(Role::Pilot)
        + cabin * game.pay_salary(Role::CabinCrew)
        + techs * game.pay_salary(Role::Technician);
    Ok(monthly * crate::pricing::RULES.fare_review_seconds as i64 / period)
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

#[cfg(test)]
pub fn initial_competitors() -> Result<Vec<Competitor>, String> {
    initial_competitors_at(Duration::ZERO)
}

/// New-game policy only. Legacy migrations use the original Frankfurt setup.
/// Catalogue order is never a tie breaker: equal distances use stable ICAOs.
pub fn new_competitors(player_home: &str) -> Result<Vec<Competitor>, String> {
    #[cfg(test)]
    if crate::airports::LEGACY_SCENARIO.with(|active| active.get()) {
        return initial_competitors();
    }
    new_competitors_with_level(player_home, crate::difficulty::Level::Standard)
}

pub fn new_competitors_with_level(
    player_home: &str,
    level: crate::difficulty::Level,
) -> Result<Vec<Competitor>, String> {
    #[cfg(test)]
    if level == crate::difficulty::Level::Standard
        && crate::airports::LEGACY_SCENARIO.with(|active| active.get())
    {
        return initial_competitors();
    }
    let mut hubs = FREQUENCY
        .starter_hubs
        .iter()
        .map(|hub| {
            let km = if hub.home == player_home {
                0.0
            } else {
                crate::routes::distance_km(player_home, &hub.home)?
            };
            Ok((hub, km))
        })
        .collect::<Result<Vec<_>, String>>()?;
    hubs.sort_by(|(a, ka), (b, kb)| ka.total_cmp(kb).then(a.home.cmp(&b.home)));
    let near = hubs.first().ok_or("KI-Startflughafen fehlt")?.0;
    let far = hubs
        .iter()
        .rev()
        .find(|(hub, _)| FREQUENCY.remote_hubs.contains(&hub.home))
        .ok_or("Entfernter KI-Startflughafen fehlt")?
        .0;
    if near.home == far.home {
        return Err("KI-Startflughäfen müssen verschieden sein".into());
    }
    let mut chosen = Vec::new();
    for choice in &crate::difficulty::RULES.competition_hubs[level.key()] {
        let hub = match choice.as_str() {
            "nearest" => near,
            "remote" => far,
            home => FREQUENCY
                .starter_hubs
                .iter()
                .find(|h| h.home == home)
                .ok_or("KI-Startflughafen fehlt")?,
        };
        // A named extra can already be the nearest hub for the player's home.
        let hub = if chosen.iter().any(|h: &&StarterHub| h.home == hub.home) {
            hubs.iter()
                .map(|(h, _)| *h)
                .find(|h| !chosen.iter().any(|c| c.home == h.home))
                .ok_or("KI-Startflughafen fehlt")?
        } else {
            hub
        };
        chosen.push(hub);
    }
    let names = [
        "Regional Wings",
        "Continental Sky",
        "Oceanic Air",
        "Global Wings",
    ];
    let starters: Vec<_> = chosen
        .iter()
        .enumerate()
        .map(|(i, h)| {
            (
                names[i],
                h.home.as_str(),
                h.destination.as_str(),
                h.model.as_str(),
                h.ticket_eur,
            )
        })
        .collect();
    starter_competitors(Duration::ZERO, &starters)
}

pub fn initial_competitors_at(now: Duration) -> Result<Vec<Competitor>, String> {
    starter_competitors(
        now,
        &[
            ("Regional Wings", "EDDF", "EDDM", "atr72_600", 90),
            ("Continental Sky", "EDDF", "LTFM", "a320_200", 150),
        ],
    )
}

fn starter_competitors(
    now: Duration,
    starters: &[(&str, &str, &str, &str, i64)],
) -> Result<Vec<Competitor>, String> {
    let mut companies = Vec::new();
    for &(name, origin, destination, model, price) in starters {
        let country = &crate::airports::catalog()?
            .airports
            .iter()
            .find(|a| a.icao == origin)
            .ok_or("KI-Startflughafen fehlt")?
            .country_code;
        let airline = crate::game::Airline {
            name: name.into(),
            country_code: country.clone(),
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
        g.tax.since = now.as_secs();
        g.marketing.settled_at = now.as_secs();
        g.buy_aircraft(model)?;
        g.insurance.settled_at = now.as_secs();
        g.insurance.renewed_at = now.as_secs();
        g.set_coverage(crate::insurance::Coverage::Basic)?;
        g.equip_staff_for(model)?;
        g.create_route(origin, destination, 1)?;
        g.set_ticket_price(1, price)?;
        g.schedule(1, 1, AI_INTERVAL_MINUTES)?;
        companies.push(Competitor {
            game: Box::new(g),
            next_review: now
                .as_secs()
                .checked_add(crate::pricing::RULES.fare_review_seconds)
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

/// First-version AI planning uses the direct market. Actual departures for all
/// owners still use the shared transfer allocator; planning underestimates hubs.
fn candidate_supply(
    route: &crate::routes::Route,
    supply: &[crate::demand::Offer],
) -> Vec<crate::demand::Offer> {
    supply
        .iter()
        .filter(|o| crate::demand::same_market(&o.route, route))
        .cloned()
        .collect()
}

fn planning_quote(
    route: &crate::routes::Route,
    model: &str,
    supply: &[crate::demand::Offer],
    fuel: i64,
) -> Result<crate::economy::Totals, String> {
    let mut prepared = crate::economy::QuoteMarket::direct(route, model, supply)?;
    crate::economy::quote_prepared(route, model, supply, fuel, None, &mut prepared)
}

impl Game {
    pub(crate) fn market_offer(
        &self,
        route: &crate::routes::Route,
        plane: &crate::aircraft::Aircraft,
    ) -> Result<crate::demand::Offer, String> {
        let mut offer = self.economy_offer(route, plane)?;
        offer.awareness = self.marketing.awareness;
        offer.loyalty_members = if self.loyalty.members == 0 {
            0
        } else {
            self.loyalty
                .members
                .min(crate::loyalty::RULES.saturation_members)
        };
        offer.reputation = self.reputation.score;
        offer.owner = self.market_owner_key();
        Ok(offer)
    }

    pub(crate) fn economy_offer(
        &self,
        route: &crate::routes::Route,
        plane: &crate::aircraft::Aircraft,
    ) -> Result<crate::demand::Offer, String> {
        let mut offer = crate::cabins::offer(route, plane)?;
        offer.economy = Some(crate::world_economy::Context {
            cycle: self.world_economy,
            quoted_at: self.clock.elapsed().as_secs(),
        });
        Ok(offer)
    }

    pub fn market_routes(&self) -> Vec<crate::demand::Offer> {
        let mut offers: Vec<_> = std::iter::once(self)
            .chain(self.competitors.iter().map(|c| &*c.game))
            .flat_map(|g| {
                g.routes.iter().filter_map(|r| {
                    let plane = g.fleet.iter().find(|p| Some(p.id) == r.aircraft_id)?;
                    let offer = g.market_offer(r, plane).ok()?;
                    Some(offer)
                })
            })
            .collect();
        self.apply_codeshares(&mut offers);
        offers
    }

    pub fn validate_competitors(&self) -> Result<(), String> {
        self.validate_difficulty()?;
        if self.competitors.len() > 4 || (self.airline.is_none() && !self.competitors.is_empty()) {
            return Err("Ungültige KI-Airlines.".into());
        }
        let mut names = std::collections::HashSet::new();
        for c in &self.competitors {
            let g = &c.game;
            if !g.clock.is_valid()
                || g.difficulty != self.difficulty
                || g.world_economy.crises != self.world_economy.crises
                || g.fuel_market != self.fuel_market
                || g.world_economy.seed != self.world_economy.seed
                || g.world_economy.started_at != self.world_economy.started_at
                || g.world_economy.events_from != self.world_economy.events_from
                || !g.used_market.listings.is_empty()
                || !g.competitors.is_empty()
                || !g.codeshares.is_empty()
                || g.clock.elapsed() != self.clock.elapsed()
                || c.next_review <= self.clock.elapsed().as_secs()
                || c.next_review - self.clock.elapsed().as_secs()
                    > crate::pricing::RULES.fare_review_seconds
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
            g.world_economy.validate(g.clock.elapsed().as_secs())?;
            g.validate_fleet()?;
            g.validate_routes()?;
            g.validate_operations()?;
            g.validate_maintenance()?;
            g.validate_staff()?;
            g.validate_financing()?;
            g.validate_tax()?;
            g.validate_fuel_contract()?;
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
        if self.insolvent() {
            return true;
        }
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
        if self
            .next_pricing_event()
            .is_none_or(|t| t > target.elapsed().as_secs())
            && self
                .next_order_event()
                .is_none_or(|t| t > target.elapsed().as_secs())
            && self.ferries.is_empty()
            && crate::reputation::next_day(self.clock.elapsed().as_secs())
                > target.elapsed().as_secs()
            && self.competitors.is_empty()
            && self
                .routes
                .iter()
                .all(|r| r.plan.as_ref().is_none_or(|p| p.disruption.is_some()))
            && self
                .next_insurance_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
            && self
                .next_marketing_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
            && self
                .next_tax_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
            && self
                .next_financing_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
            && self
                .next_maintenance_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
            && self
                .next_rotation_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
            && self
                .next_training_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
            && self
                .next_labour_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
            && self
                .next_payroll_event()
                .is_none_or(|t| Duration::from_secs(t) > target.elapsed())
        {
            return self.advance_company_to(
                target,
                &self.market_routes(),
                self.supply_revision(),
                self.supply_structure_revision(),
            );
        }
        // Transactional across all companies: no partial time or cash updates.
        let mut staged = self.clone();
        let mut cached_supply = Vec::new();
        let mut cached_revision = None;
        #[cfg(test)]
        let mut stalled = (0, staged.clock.elapsed());
        while staged.clock.elapsed() < target.elapsed() {
            #[cfg(test)]
            {
                if stalled.1 == staged.clock.elapsed() {
                    stalled.0 += 1;
                } else {
                    stalled = (0, staged.clock.elapsed());
                }
                assert!(
                    stalled.0 < 5,
                    "Simulation event loop made no progress at {:?}",
                    staged.clock.elapsed()
                );
            }

            if staged.initialize_disruptions().is_err()
                || staged
                    .competitors
                    .iter_mut()
                    .any(|c| c.game.initialize_disruptions().is_err())
            {
                return false;
            }
            // Older saves may already be parked after a deferred loss review.
            // Close these at the current instant, never move the clock backward.
            for c in &mut staged.competitors {
                let parked: Vec<_> = c
                    .route_reviews
                    .iter()
                    .filter(|r| r.losses >= 2)
                    .filter_map(|r| {
                        let route = c.game.routes.iter().find(|route| {
                            route.id == r.route_id
                                && route.plan.is_some()
                                && !c
                                    .game
                                    .routes
                                    .iter()
                                    .any(|s| s.parent_route_id == Some(route.public_id()))
                        })?;
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
            let maintenance = std::iter::once(staged.next_order_event())
                .chain(std::iter::once(staged.next_pricing_event()))
                .chain(std::iter::once(staged.next_ferry_event()))
                .chain(std::iter::once(staged.next_maintenance_event()))
                .chain(std::iter::once(staged.next_rotation_event()))
                .chain(std::iter::once(staged.next_payroll_event()))
                .chain(std::iter::once(staged.next_labour_event()))
                .chain(std::iter::once(staged.next_training_event()))
                .chain(std::iter::once(staged.next_financing_event()))
                .chain(std::iter::once(staged.next_tax_event()))
                .chain(std::iter::once(staged.next_marketing_event()))
                .chain(std::iter::once(staged.next_insurance_event()))
                .chain(staged.competitors.iter().flat_map(|c| {
                    [
                        c.game.next_order_event(),
                        c.game.next_ferry_event(),
                        c.game.next_maintenance_event(),
                        c.game.next_rotation_event(),
                        c.game.next_payroll_event(),
                        c.game.next_labour_event(),
                        c.game.next_training_event(),
                        c.game.next_financing_event(),
                        c.game.next_tax_event(),
                        c.game.next_marketing_event(),
                        c.game.next_insurance_event(),
                    ]
                }))
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
                            let route = c.game.routes.iter().find(|route| {
                                route.id == r.route_id
                                    && route.plan.is_some()
                                    && !c
                                        .game
                                        .routes
                                        .iter()
                                        .any(|s| s.parent_route_id == Some(route.public_id()))
                            })?;
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
                .min(maintenance)
                .min(crate::reputation::next_day(
                    staged.clock.elapsed().as_secs(),
                ));
            // An event already due (e.g. after loading) never moves the clock back.
            let end = target
                .elapsed()
                .min(Duration::from_secs(boundary).max(staged.clock.elapsed()));
            let revision = staged.supply_revision();
            let structure_revision = staged.supply_structure_revision();
            if cached_revision != Some(revision) {
                cached_supply = staged.market_routes();
                cached_revision = Some(revision);
            }
            let supply = &cached_supply;
            // Exact integer/nanosecond target, not repeated floating-point conversion.
            if !staged.advance_company_to(
                Clock::from_state(end, true, 1).unwrap(),
                supply,
                revision,
                structure_revision,
            ) {
                return false;
            }
            for c in &mut staged.competitors {
                if !c.game.advance_company_to(
                    Clock::from_state(end, true, 1).unwrap(),
                    supply,
                    revision,
                    structure_revision,
                ) {
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
            for c in &mut staged.competitors {
                if c.game.ai_labour_policy().is_err() {
                    return false;
                }
            }
            if staged.apply_insurance().is_err()
                || staged.apply_rotation_removals().is_err()
                || staged
                    .apply_ferries()
                    .and_then(|()| staged.apply_maintenance())
                    .is_err()
                || staged.apply_payroll().is_err()
                || staged.apply_labour_events().is_err()
                || staged.apply_marketing().is_err()
                || staged.apply_loyalty().is_err()
                || staged.apply_financing().is_err()
                || staged.apply_orders().is_err()
                || staged.competitors.iter_mut().any(|c| {
                    c.game.apply_insurance().is_err()
                        || c.game.apply_rotation_removals().is_err()
                        || c.game
                            .apply_ferries()
                            .and_then(|()| c.game.apply_maintenance())
                            .is_err()
                        || c.game.apply_payroll().is_err()
                        || c.game.apply_labour_events().is_err()
                        || c.game.apply_marketing().is_err()
                        || c.game.apply_loyalty().is_err()
                        || c.game.apply_financing().is_err()
                        || c.game.apply_orders().is_err()
                })
            {
                return false;
            }
            staged.review_codeshares();
            // Remove the whole AI company, including aircraft, plans, staff and
            // event sources. Its already settled flights remain in other ledgers.
            let previous_companies = staged.competitors.len();
            staged.competitors.retain(|c| !c.game.insolvent());
            if staged.competitors.len() != previous_companies {
                staged.commercial_changed();
            }
            // All companies settle against the same immutable offer snapshot first.
            for i in 0..staged.competitors.len() {
                if staged.competitors[i].next_review != end.as_secs() {
                    continue;
                }
                // Reviews also inspect current operational progress and replace
                // exact offers, so take a fresh snapshot at this commercial event.
                let supply = staged.market_routes();
                let external = staged.slots_except_competitor(i);
                let c = &mut staged.competitors[i];
                c.game.external_slots = external;
                if c.next_review == end.as_secs() {
                    std::mem::swap(&mut staged.used_market, &mut c.game.used_market);
                    if !c.review(&supply) {
                        return false;
                    }
                    std::mem::swap(&mut staged.used_market, &mut c.game.used_market);
                }
            }
            if staged.apply_pricing().is_err() {
                return false;
            }
            staged.review_codeshares();
            if end.as_secs().is_multiple_of(crate::reputation::DAY) && end.subsec_nanos() == 0 {
                staged.evaluate_milestones();
            }
            if staged.insolvent() {
                staged.clock.set_speed(target.speed());
                *self = staged;
                return true;
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

    fn review(&mut self, supply: &[crate::demand::Offer]) -> bool {
        let owned_supply = self.game.prioritize_market(supply);
        let supply = owned_supply.as_slice();
        if !self.game.distressed()
            && self.game.insurance.coverage == crate::insurance::Coverage::None
            && self
                .game
                .set_coverage(crate::insurance::Coverage::Basic)
                .is_err()
        {
            return false;
        }
        self.game.review_loyalty();
        self.game.review_fuel_contract();
        if self.game.review_marketing().is_err() {
            return false;
        }
        if self.game.can_reschedule() && self.game.reschedule_loan().is_err() {
            return false;
        }
        if self.game.distressed() {
            // Dispose only idle parked aircraft, using exactly the player's quotes.
            let idle: Vec<_> = self
                .game
                .fleet
                .iter()
                .filter(|p| {
                    p.status == crate::aircraft::AircraftStatus::Parked && p.maintenance.is_none()
                })
                .filter(|p| {
                    !self
                        .game
                        .routes
                        .iter()
                        .any(|r| r.aircraft_id == Some(p.id) && r.plan.is_some())
                })
                .map(|p| (p.id, p.lease.is_some()))
                .collect();
            for (id, leased) in idle {
                let routes: Vec<_> = self
                    .game
                    .routes
                    .iter()
                    .filter(|r| r.aircraft_id == Some(id))
                    .map(|r| r.id)
                    .collect();
                let result = if leased {
                    self.game.return_aircraft(id)
                } else {
                    self.game.sell_aircraft(id)
                };
                if result.is_err() {
                    return false;
                }
                for route in routes {
                    if self.game.delete_route(route).is_err() {
                        return false;
                    }
                    self.route_reviews.retain(|r| r.route_id != route);
                }
                if self.game.apply_financing().is_err() {
                    return false;
                }
                if !self.game.distressed() {
                    break;
                }
            }
            // Reduce idle payroll before sizing a rescue loan.
            if self.game.dismiss_surplus().is_err() {
                return false;
            }
        }
        // Rescue existing services, never borrow to buy aircraft. Same capped
        // facility and eligibility as the player; no grants or hidden overdraft.
        let reserve = crate::financing::RULES
            .low_cash_warning_eur
            .max(self.game.monthly_payroll().saturating_mul(2));
        if self.game.cash() < reserve
            && self.game.financing.debt == 0
            && (self.game.monthly_payroll() > 0 || self.game.cash() < 0)
        {
            let amount = reserve.saturating_sub(self.game.cash());
            if self.game.borrow(amount).is_ok() {
                self.last_action =
                    "Liquiditätskredit für bestehende Verpflichtungen aufgenommen".into();
            }
        }
        if self.game.distressed() {
            if self.game.dismiss_surplus().is_err() {
                return false;
            }
            self.game.review_ancillaries();
            self.last_action =
                "Finanznot: Expansion ausgesetzt, ungenutztes Personal abgebaut".into();
            let Some(next) = self
                .next_review
                .checked_add(crate::pricing::RULES.fare_review_seconds)
            else {
                return false;
            };
            self.next_review = next;
            return true;
        }
        let surplus = self.game.cash().saturating_sub(reserve.saturating_mul(2));
        let repayment = surplus.min(self.game.financing.debt);
        if repayment > 0 && self.game.repay(repayment).is_err() {
            return false;
        }
        let mut other_supply = supply.to_vec();
        for r in &self.game.routes {
            if let Some(i) = other_supply.iter().position(|o| &o.route == r) {
                other_supply.remove(i);
            }
        }
        self.game.review_cabins();
        self.game.review_service();
        self.game.review_ancillaries();
        let mut updated_supply = other_supply.clone();
        updated_supply.extend(self.game.market_routes());
        let supply = updated_supply.as_slice();
        let maintenance = self.game.history.as_ref().map_or(0, |h| {
            (h.maintenance_through(self.game.clock.elapsed().as_secs())
                + h.payroll_through(self.game.clock.elapsed().as_secs())
                + h.interest_through(self.game.clock.elapsed().as_secs())
                + h.rent_through(self.game.clock.elapsed().as_secs()))
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
        let grouped = match self.review_rotations(supply) {
            Ok(ids) => ids,
            Err(_) => return false,
        };
        let mut closed = Vec::new();
        for i in 0..self.game.routes.len() {
            let r = &self.game.routes[i];
            if grouped.contains(&r.public_id()) {
                continue;
            }
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
            let rent = plane.lease.as_ref().map_or(0, |l| {
                i128::from(l.monthly) * i128::from(crate::pricing::RULES.fare_review_seconds)
                    / i128::from(l.period)
            });
            // A seasonal trough alone must not retire an annually viable route.
            // Real losses still count when annual demand cannot cover the service.
            let fuel = self
                .game
                .fuel_market
                .price_at(self.game.clock.elapsed().as_secs());
            let Ok(mut fare_market) =
                crate::economy::QuoteMarket::direct(r, &plane.model_id, supply)
            else {
                return false;
            };
            let Ok(annual) = crate::economy::quote_prepared(
                r,
                &plane.model_id,
                supply,
                fuel,
                None,
                &mut fare_market,
            ) else {
                return false;
            };
            let annual_week = i128::from(annual.profit() - maintenance)
                * i128::from(crate::pricing::RULES.fare_review_seconds)
                / i128::from(r.plan.as_ref().unwrap().interval_seconds)
                - rent;
            let losses = if week - rent < 0 && annual_week < 0 {
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
            let old = r.ticket_eur;
            let Ok(best) = crate::pricing::review(&self.game, r, plane, supply, false) else {
                return false;
            };
            let business_fare =
                (plane.business_seats > 0).then(|| crate::cabins::premium_fare(best.fare));
            if self.game.set_ticket_price(id, best.fare).is_err()
                || business_fare
                    .is_some_and(|fare| self.game.set_business_ticket_price(id, fare).is_err())
            {
                return false;
            }
            self.last_action = format!("Preisprüfung: {old} → {} EUR", best.fare);
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
        let Some(next) = self
            .next_review
            .checked_add(crate::pricing::RULES.fare_review_seconds)
        else {
            return false;
        };
        self.plan_maintenance(next);
        // Staff without a service is let go; the hangar crew stays.
        if self.game.dismiss_surplus().is_err() {
            return false;
        }
        self.next_review = next;
        true
    }

    /// Review the whole public route. Retire at most one service after two
    /// weak reviews; the shared removal API settles airborne flights safely.
    fn review_rotations(&mut self, supply: &[crate::demand::Offer]) -> Result<Vec<u32>, String> {
        let mut groups = Vec::new();
        for r in &self.game.routes {
            if let Some(root) = r.parent_route_id
                && !groups.contains(&root)
            {
                groups.push(root);
            }
        }
        for root in &groups {
            let services: Vec<_> = self
                .game
                .routes
                .iter()
                .filter(|r| r.public_id() == *root && r.aircraft_id.is_some())
                .cloned()
                .collect();
            if services.is_empty() {
                let ids: Vec<_> = self
                    .game
                    .routes
                    .iter()
                    .filter(|r| r.public_id() == *root)
                    .map(|r| r.id)
                    .collect();
                self.game.delete_route(*root)?;
                self.route_reviews.retain(|r| !ids.contains(&r.route_id));
                continue;
            }
            if services.iter().any(|r| {
                r.remove_at.is_some()
                    || r.plan.is_none()
                    || self
                        .game
                        .fleet
                        .iter()
                        .any(|p| Some(p.id) == r.aircraft_id && p.maintenance.is_some())
            }) {
                continue;
            }
            let mut week = 0i128;
            let mut expected = crate::economy::Totals::default();
            let mut freight_capacity = 0u64;
            let mut annual_week = 0i128;
            let mut observed = false;
            let mut previous_losses = 2;
            for r in &services {
                let plane = self
                    .game
                    .fleet
                    .iter()
                    .find(|p| Some(p.id) == r.aircraft_id)
                    .ok_or("KI-Flugzeug fehlt")?;
                // History::totals aggregates public routes; review each service
                // exactly once while retaining its existing save-compatible cursor.
                let totals = self
                    .game
                    .history
                    .as_ref()
                    .unwrap()
                    .settled_service_totals(r.id);
                let previous = self
                    .route_reviews
                    .iter()
                    .find(|v| v.route_id == r.id)
                    .cloned()
                    .unwrap_or(RouteReview {
                        flights: Some(0),
                        ..Default::default()
                    });
                let landed = previous
                    .flights
                    .map_or(0, |f| (totals.flights as u64).saturating_sub(f));
                observed |= landed > 0;
                week += i128::from(totals.profit())
                    - i128::from(previous.profit)
                    - i128::from(landed) * i128::from(maintenance_per_flight(r, &plane.model_id)?)
                    - i128::from(staffing_per_week(&self.game, &plane.model_id)?);
                if let Some(l) = &plane.lease {
                    week -= i128::from(l.monthly)
                        * i128::from(crate::pricing::RULES.fare_review_seconds)
                        / i128::from(l.period);
                }
                let quote = planning_quote(
                    r,
                    &plane.model_id,
                    supply,
                    self.game
                        .fuel_market
                        .price_at(self.game.clock.elapsed().as_secs()),
                )?;
                annual_week +=
                    i128::from(quote.profit() - maintenance_per_flight(r, &plane.model_id)?)
                        * i128::from(crate::pricing::RULES.fare_review_seconds)
                        / i128::from(r.plan.as_ref().unwrap().interval_seconds)
                        - i128::from(staffing_per_week(&self.game, &plane.model_id)?);
                if let Some(l) = &plane.lease {
                    annual_week -= i128::from(l.monthly)
                        * i128::from(crate::pricing::RULES.fare_review_seconds)
                        / i128::from(l.period);
                }
                freight_capacity +=
                    u64::from(crate::aircraft::model(&plane.model_id)?.main_deck_cargo_kg);
                expected.add(&quote, 1).ok_or("KI-Finanzüberlauf")?;
                previous_losses = previous_losses.min(previous.losses);
                self.route_reviews.retain(|v| v.route_id != r.id);
                self.route_reviews.push(RouteReview {
                    route_id: r.id,
                    profit: totals.profit(),
                    losses: 0,
                    flights: Some(totals.flights as u64),
                });
            }
            let losses = if observed
                && ((week < 0 && annual_week < 0)
                    || (if expected.seats == 0 && freight_capacity > 0 {
                        expected.cargo_kg as f64 * 100.0 / freight_capacity as f64
                    } else {
                        expected.load_percent()
                    }) < FREQUENCY.load_factor_percent)
            {
                (previous_losses + 1).min(2)
            } else {
                0
            };
            // Keep legacy deferred whole-route closure away from grouped services.
            for review in &mut self.route_reviews {
                if services.iter().any(|r| r.id == review.route_id) {
                    review.losses = losses.min(1);
                }
            }
            if losses >= 2 {
                let service = services.last().unwrap();
                self.game
                    .remove_route_aircraft(*root, service.aircraft_id.unwrap())?;
                for review in &mut self.route_reviews {
                    if services.iter().any(|r| r.id == review.route_id) {
                        review.losses = 0;
                    }
                }
                self.growth_reviews = 0;
                self.last_action =
                    "Schwache Verbindung: eine Maschine nach Bodenzeit freigeben".into();
            }
        }
        Ok(groups)
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
    /// Evaluate each route/fare once for an aircraft and location. Acquisition
    /// price, wear backlog and rent do not affect the shared market quote.
    fn expansion_quotes(
        &self,
        offers: &[crate::demand::Offer],
        model: &str,
        origin: &str,
        plane: Option<u32>,
    ) -> Result<Vec<ExpansionQuote>, String> {
        let g = &self.game;
        let m = crate::aircraft::model(model)?;
        let staffing = staffing_per_week(g, model)?;
        let fuel = g.fuel_market.price_at(g.clock.elapsed().as_secs());
        let mut quotes = Vec::new();
        let mut candidates = Vec::new();
        for root in g.routes.iter().filter(|r| r.parent_route_id.is_none()) {
            let services: Vec<_> = g
                .routes
                .iter()
                .filter(|r| r.public_id() == root.id && r.aircraft_id.is_some())
                .collect();
            if services.len() >= FREQUENCY.max_aircraft_per_route
                || services.is_empty()
                || root.origin_icao != origin
                || services
                    .iter()
                    .any(|r| r.plan.is_none() || r.remove_at.is_some())
            {
                continue;
            }
            let mut strong = true;
            for r in &services {
                let p = g
                    .fleet
                    .iter()
                    .find(|p| Some(p.id) == r.aircraft_id)
                    .ok_or("KI-Flugzeug fehlt")?;
                let q = planning_quote(
                    r,
                    &p.model_id,
                    offers,
                    self.game
                        .fuel_market
                        .price_at(self.game.clock.elapsed().as_secs()),
                )?;
                let rent = p.lease.as_ref().map_or(0, |l| {
                    (l.monthly * crate::pricing::RULES.fare_review_seconds as i64 + l.period as i64
                        - 1)
                        / l.period as i64
                });
                strong &= q.model_load_percent(crate::aircraft::model(&p.model_id)?)
                    >= FREQUENCY.load_factor_percent
                    && (q.profit() - maintenance_per_flight(r, &p.model_id)?)
                        * (crate::pricing::RULES.fare_review_seconds
                            / r.plan.as_ref().unwrap().interval_seconds)
                            as i64
                        > staffing_per_week(g, &p.model_id)? + rent;
            }
            if strong {
                let mut candidate = root.clone();
                candidate.id = g.next_route_id;
                candidate.parent_route_id = Some(root.id);
                candidate.aircraft_id = plane;
                candidate.remove_at = None;
                candidate.rotation_phase = None;
                candidates.push(candidate);
            }
        }
        for airport in &crate::airports::catalog()?.airports {
            if airport.icao == origin
                || crate::routes::distance_km(origin, &airport.icao)? > f64::from(m.range_km)
            {
                continue;
            }
            let r = Route {
                cargo_rate_cents: crate::cargo::default_rate(),
                ancillary_policy: g.ancillary_policy,
                loyalty_level: g.loyalty.level,
                service_level: Default::default(),
                business_ticket_eur: crate::cabins::default_business_fare(),
                id: g.next_route_id,
                origin_icao: origin.into(),
                destination_icao: airport.icao.clone(),
                aircraft_id: plane,
                parent_route_id: None,
                remove_at: None,
                rotation_phase: None,
                ticket_eur: 0,
                plan: None,
            };
            if !g
                .routes
                .iter()
                .any(|old| crate::demand::same_market(old, &r))
            {
                candidates.push(r);
            }
        }
        for mut r in candidates {
            let km = crate::routes::distance_km(origin, &r.destination_icao)?;
            if km > f64::from(m.range_km) {
                continue;
            }
            let (flight, ground) = crate::operations::timings(&r, model)?;
            let maintenance = crate::maintenance::cost_per_flight(model, flight)?;
            let interval = r.plan.as_ref().map_or(
                u64::from(AI_INTERVAL_MINUTES).max((flight + ground).div_ceil(60)),
                |p| p.interval_seconds / 60,
            );
            if interval * 60 < flight + ground {
                continue;
            }
            r.plan = Some(crate::operations::FlightPlan {
                first_departure: g.clock.elapsed().as_secs() + 60,
                interval_seconds: interval * 60,
                ..Default::default()
            });
            if g.check_slots(&r.origin_icao, &r.destination_icao, interval * 60)
                .is_err()
            {
                continue;
            }
            let existing = candidate_supply(&r, offers);
            let existing_plane = plane.and_then(|id| g.fleet.iter().find(|p| p.id == id));
            let mut cabin_plane = crate::aircraft::Aircraft {
                business_seats: 0,
                id: 0,
                model_id: model.into(),
                location_icao: origin.into(),
                status: crate::aircraft::AircraftStatus::Parked,
                purchase_price_eur: 0,
                acquired_at: None,
                purchased_at: None,
                lease: None,
                wear: Default::default(),
                maintenance: None,
            };
            let layouts = match existing_plane {
                Some(p) => vec![p.business_seats],
                None if m.seats == 0 => vec![0],
                None => crate::cabins::RULES
                    .ai_expansion_business_space_percent
                    .iter()
                    .map(|percent| {
                        m.seats * percent / 100 / crate::cabins::RULES.business_space_ratio
                    })
                    .collect(),
            };
            for business in layouts {
                cabin_plane.business_seats = business;
                let mut market = existing.clone();
                market.push(g.economy_offer(&r, &cabin_plane)?);
                let mut single_quote: Option<crate::economy::QuoteMarket> = None;
                for step in 3..=15 {
                    if (r.parent_route_id.is_some() || m.seats == 0) && step != 3 {
                        continue;
                    }
                    if r.parent_route_id.is_none() {
                        r.ticket_eur = (crate::economy::reference_price(km) * f64::from(step)
                            / 10.0)
                            .round() as i64;
                    }
                    if r.parent_route_id.is_none() {
                        r.business_ticket_eur = crate::cabins::premium_fare(r.ticket_eur);
                    }
                    let candidate = market.last_mut().unwrap();
                    candidate.route.ticket_eur = r.ticket_eur;
                    candidate.route.business_ticket_eur = r.business_ticket_eur;
                    if let Some(prepared) = &mut single_quote {
                        prepared.reprice_direct(&r);
                    } else {
                        single_quote =
                            Some(crate::economy::QuoteMarket::direct(&r, model, &market)?);
                    }
                    let t = crate::economy::quote_prepared(
                        &r,
                        model,
                        &market,
                        fuel,
                        None,
                        single_quote.as_mut().unwrap(),
                    )?;
                    // Charge the loss of revenue on our existing services too:
                    // an added aircraft must earn its keep after market dilution.
                    let mut dilution = 0;
                    if r.parent_route_id.is_some() {
                        for old in g
                            .routes
                            .iter()
                            .filter(|old| old.plan.is_some() && crate::demand::same_market(old, &r))
                        {
                            let p = g
                                .fleet
                                .iter()
                                .find(|p| Some(p.id) == old.aircraft_id)
                                .ok_or("KI-Flugzeug fehlt")?;
                            let before = planning_quote(
                                old,
                                &p.model_id,
                                offers,
                                self.game
                                    .fuel_market
                                    .price_at(self.game.clock.elapsed().as_secs()),
                            )?;
                            let after = planning_quote(
                                old,
                                &p.model_id,
                                &market,
                                self.game
                                    .fuel_market
                                    .price_at(self.game.clock.elapsed().as_secs()),
                            )?;
                            dilution += (before.profit() - after.profit())
                                * (crate::pricing::RULES.fare_review_seconds
                                    / old.plan.as_ref().unwrap().interval_seconds)
                                    as i64;
                        }
                        if t.model_load_percent(m) < FREQUENCY.load_factor_percent {
                            continue;
                        }
                    }
                    quotes.push(ExpansionQuote {
                        route: r.clone(),
                        interval: interval as u32,
                        business_seats: business,
                        layout_fee: if existing_plane.is_none() && business > 0 {
                            crate::cabins::RULES.layout_fee_eur
                        } else {
                            0
                        },
                        profit: (t.profit() - maintenance)
                            * (crate::pricing::RULES.fare_review_seconds / (interval * 60)) as i64
                            - staffing
                            - dilution,
                        costs: staffing
                            + (t.costs() + maintenance)
                                * (crate::pricing::RULES.fare_review_seconds / (interval * 60))
                                    as i64,
                    });
                }
            }
        }
        Ok(quotes)
    }

    fn expand(&mut self, supply: &[crate::demand::Offer]) -> Result<(), String> {
        if self.game.distressed() || self.game.financing.debt > 0 {
            return Ok(());
        }
        let g = &self.game;
        let airline = g.airline.as_ref().ok_or("KI-Airline fehlt")?;
        // Caller removed precisely our pre-review offers, including closed routes.
        let mut offers = supply.to_vec();
        offers.extend(g.market_routes());
        let mut committed_weekly_costs = g.monthly_payroll() * 7 / 30;
        committed_weekly_costs += g
            .fleet
            .iter()
            .filter_map(|p| p.lease.as_ref())
            .map(|l| {
                (l.monthly * crate::pricing::RULES.fare_review_seconds as i64 + l.period as i64 - 1)
                    / l.period as i64
            })
            .sum::<i64>();
        for route in g.routes.iter().filter(|r| r.plan.is_some()) {
            let plane = g
                .fleet
                .iter()
                .find(|p| Some(p.id) == route.aircraft_id)
                .ok_or("KI-Flugzeug fehlt")?;
            let t = planning_quote(
                route,
                &plane.model_id,
                &offers,
                self.game
                    .fuel_market
                    .price_at(self.game.clock.elapsed().as_secs()),
            )?;
            committed_weekly_costs += (t.costs() + maintenance_per_flight(route, &plane.model_id)?)
                * (crate::pricing::RULES.fare_review_seconds
                    / route.plan.as_ref().unwrap().interval_seconds) as i64;
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
            .map(|p| {
                (
                    None,
                    0,
                    Some(p.id),
                    p.model_id.as_str(),
                    p.location_icao.as_str(),
                    0,
                    false,
                    p.lease.as_ref().map_or(0, |l| {
                        (l.monthly * crate::pricing::RULES.fare_review_seconds as i64
                            + l.period as i64
                            - 1)
                            / l.period as i64
                    }),
                )
            })
            .collect();
        if free.is_empty() && g.orders.is_empty() && g.fleet.len() + g.orders.len() < MAX_FLEET {
            for l in &g.used_market.listings {
                let p = &l.aircraft;
                let rules = crate::maintenance::rules(&p.model_id)?;
                // A due aircraft needs a check before it can earn; leave these to the player.
                if p.wear.is_due(rules) || p.business_seats != 0 {
                    continue;
                }
                let cost = g.used_price(l)?;
                let backlog = crate::maintenance::cost(&p.wear, rules, false)?;
                if airline
                    .capital_eur
                    .saturating_sub(cost)
                    .saturating_sub(backlog)
                    >= RESERVE
                {
                    options.push((
                        Some(l.id),
                        backlog,
                        None,
                        p.model_id.as_str(),
                        airline.home_airport_icao.as_str(),
                        cost,
                        false,
                        0,
                    ));
                }
            }
            for m in crate::aircraft::models()? {
                if airline.capital_eur.saturating_sub(m.price_eur) >= RESERVE {
                    options.push((
                        None,
                        0,
                        None,
                        &m.id,
                        &airline.home_airport_icao,
                        m.price_eur,
                        false,
                        0,
                    ));
                } else {
                    let (deposit, monthly) = crate::trading::lease_terms(&m.id)?;
                    if airline.capital_eur.saturating_sub(deposit) >= RESERVE {
                        let period = crate::trading::RULES.lease_period_days * 86400;
                        let rent = (monthly * crate::pricing::RULES.fare_review_seconds as i64
                            + period as i64
                            - 1)
                            / period as i64;
                        options.push((
                            None,
                            0,
                            None,
                            &m.id,
                            &airline.home_airport_icao,
                            deposit,
                            true,
                            rent,
                        ));
                    }
                }
            }
        }
        let mut quotes = Vec::new();
        let mut best = None;
        let mut best_week = 0;
        for (used, backlog, plane, model, origin, cost, leased, rent) in options {
            let m = crate::aircraft::model(model)?;
            // Preserve the existing new/lease fallback and route ranking. Evaluate
            // used capital relative to that same-model alternative, over the
            // existing 104-week horizon, including the residual hull values.
            let capital_adjustment = if let Some(id) = used {
                let listing = g.used_market.listings.iter().find(|l| l.id == id).unwrap();
                let now = g.clock.elapsed().as_secs();
                let horizon = now.saturating_add(104 * crate::pricing::RULES.fare_review_seconds);
                let residual = crate::trading::hull_value_at(&listing.aircraft, horizon)?;
                let alternative_net = if airline.capital_eur.saturating_sub(m.price_eur) >= RESERVE
                {
                    let mut fresh = listing.aircraft.clone();
                    fresh.acquired_at = Some(now);
                    fresh.wear = Default::default();
                    m.price_eur - crate::trading::hull_value_at(&fresh, horizon)?
                } else {
                    crate::trading::lease_terms(model)?.0
                };
                (cost - residual - alternative_net) / 104
            } else {
                0
            };
            let key = (model, origin, plane);
            let index = match quotes.iter().position(|(k, _)| *k == key) {
                Some(index) => index,
                None => {
                    quotes.push((key, self.expansion_quotes(&offers, model, origin, plane)?));
                    quotes.len() - 1
                }
            };
            for q in &quotes[index].1 {
                let week = q.profit - rent - (backlog + 103) / 104;
                let buffer =
                    ((committed_weekly_costs + q.costs + if plane.is_none() { rent } else { 0 })
                        * 4)
                    .max(RESERVE);
                let score = week - capital_adjustment;
                if score > best_week
                    && week >= 5_000
                    && week * 104 >= cost + q.layout_fee
                    && airline
                        .capital_eur
                        .saturating_sub(cost)
                        .saturating_sub(backlog)
                        .saturating_sub(q.layout_fee)
                        >= buffer
                {
                    best_week = score;
                    best = Some(ExpansionChoice {
                        used,
                        plane,
                        model: model.to_string(),
                        quote: q.clone(),
                        leased,
                        score,
                    });
                }
            }
        }
        let Some(choice) = best else {
            self.last_action =
                "Expansion geprüft · Budget/Reserve oder Markt nicht rentabel".into();
            return Ok(());
        };
        self.acquire_expansion(choice)
    }

    /// Execute the selected quote with the same player acquisition/cabin APIs.
    fn acquire_expansion(&mut self, choice: ExpansionChoice) -> Result<(), String> {
        let ExpansionChoice {
            used,
            plane,
            model,
            quote,
            leased,
            score: best_week,
        } = choice;
        let ExpansionQuote {
            route,
            interval,
            business_seats: business,
            ..
        } = quote;
        // Only an otherwise affordable new purchase may become a factory order.
        // Existing services earn during the lead time; the normal expansion search
        // configures the delivered, parked aircraft at the following weekly review.
        if plane.is_none()
            && used.is_none()
            && !leased
            && self.game.orders.is_empty()
            && self.game.fleet.len() >= crate::orders::RULES.ai_min_fleet
            && (self.game.fleet.len() + 1).is_multiple_of(crate::orders::RULES.ai_order_every)
            && self.game.fleet.len() < crate::orders::RULES.ai_max_fleet
            && self.game.fleet.len() < MAX_FLEET
        {
            self.game.place_order(&model, 1)?;
            self.last_action = format!("{} beim Hersteller bestellt · Anzahlung bezahlt", model);
            return Ok(());
        }
        // Hire first (atomic): without cash for the crew the expansion is dropped.
        if let Err(error) = self.game.equip_staff_for(&model) {
            self.last_action = format!("Expansion abgebrochen · {error}");
            return Ok(());
        }
        let plane = match plane {
            Some(id) => id,
            None => {
                if let Some(id) = used {
                    self.game.buy_used_aircraft(id)?;
                } else if leased {
                    self.game.lease_aircraft(&model)?;
                } else {
                    self.game.buy_aircraft(&model)?;
                }
                self.game.fleet.last().unwrap().id
            }
        };
        self.game.set_cabin_layout(plane, business)?;
        if let Some(root) = route.parent_route_id {
            self.game.add_route_aircraft(root, plane)?;
        } else {
            self.game
                .create_route(&route.origin_icao, &route.destination_icao, plane)?;
            self.game.set_ticket_price(route.id, route.ticket_eur)?;
            self.game
                .set_business_ticket_price(route.id, route.business_ticket_eur)?;
            self.game.schedule(route.id, 1, interval)?;
        }
        self.last_action = format!(
            "{}–{} {} · {} · erwartet {best_week} EUR/Woche",
            route.origin_icao,
            route.destination_icao,
            if route.parent_route_id.is_some() {
                "verstärkt"
            } else {
                "eröffnet"
            },
            if route.aircraft_id.is_some() {
                "freie Maschine"
            } else if used.is_some() {
                "Gebrauchtflugzeug gekauft"
            } else if leased {
                "Flugzeug geleast"
            } else {
                "Flugzeug gekauft"
            }
        );
        Ok(())
    }

    /// A stopped service waits RETRY_AFTER reviews, then restarts at the most
    /// profitable fare in the current shared market, or waits another period.
    /// Returns false only on an internal error.
    fn retry(&mut self, i: usize, supply: &[crate::demand::Offer]) -> bool {
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
        if self
            .game
            .check_slots(
                &r.origin_icao,
                &r.destination_icao,
                u64::from(AI_INTERVAL_MINUTES) * 60,
            )
            .is_err()
        {
            // Free the aircraft for another market on the next review.
            let id = r.id;
            if self.game.delete_route(id).is_err() {
                return false;
            }
            self.route_reviews.retain(|r| r.route_id != id);
            return true;
        }
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
        let mut offers = candidate_supply(&candidate, supply);
        offers.push(self.game.economy_offer(&candidate, plane).unwrap());
        let Ok(mut fare_market) = crate::economy::QuoteMarket::direct(&candidate, &model, &offers)
        else {
            return false;
        };
        let fuel = self.game.fuel_market.price_at(now);
        let mut best: Option<(i64, i64)> = None;
        for step in 3..=15 {
            let price = ((reference * f64::from(step) / 10.0).round() as i64).clamp(0, 100_000);
            candidate.ticket_eur = price;
            if plane.business_seats > 0 {
                candidate.business_ticket_eur = crate::cabins::premium_fare(price);
            }
            fare_market.reprice_direct(&candidate);
            let Ok(t) = crate::economy::quote_prepared(
                &candidate,
                &model,
                &offers,
                fuel,
                None,
                &mut fare_market,
            ) else {
                return false;
            };
            let rent = plane.lease.as_ref().map_or(0, |l| {
                (l.monthly * candidate.plan.as_ref().unwrap().interval_seconds as i64
                    + l.period as i64
                    - 1)
                    / l.period as i64
            });
            let profit = t.profit() - maintenance - rent;
            if best.is_none_or(|(best, _)| profit > best) {
                best = Some((profit, price));
            }
        }
        let Some((profit, price)) = best.filter(|(profit, _)| *profit > 0) else {
            self.last_action = "Markt erneut geprüft · Wiedereinstieg noch nicht rentabel".into();
            return true;
        };
        let id = r.id;
        if let Err(error) = self.game.equip_staff_for(&model) {
            self.last_action = format!("Wiedereinstieg abgebrochen · {error}");
            return true;
        }
        let route = &mut self.game.routes[i];
        route.origin_icao = candidate.origin_icao;
        route.destination_icao = candidate.destination_icao;
        if self.game.set_ticket_price(id, price).is_err()
            || self
                .game
                .set_business_ticket_price(id, crate::cabins::premium_fare(price))
                .is_err()
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
    #[test]
    fn ai_planning_is_direct_only_while_actual_flights_keep_transfers() {
        let mut g = crate::game::Game::default();
        g.found("Planning audit", "DE", "EDDF").unwrap();
        g.competitors.clear();
        g.airline.as_mut().unwrap().capital_eur += 100_000_000;
        g.history.as_mut().unwrap().opening_balance += 100_000_000;
        for (id, destination, fare) in [(1, "EDDM", 90), (2, "LTFM", 120)] {
            g.buy_aircraft("a320_200").unwrap();
            g.staff_up();
            g.create_route("EDDF", destination, id).unwrap();
            g.set_ticket_price(id, fare).unwrap();
            g.schedule(id, 1, 240).unwrap();
        }
        let supply = g.market_routes();
        crate::demand::TRANSFER_COUNTS.with(|c| c.set((0, 0, 0)));
        let planned = super::planning_quote(&g.routes[0], "a320_200", &supply, 100).unwrap();
        let direct = super::candidate_supply(&g.routes[0], &supply);
        assert_eq!(
            planned,
            crate::economy::quote_at_price(&g.routes[0], "a320_200", &direct, 100).unwrap()
        );
        assert_eq!(planned.connecting_passengers, 0);
        crate::demand::TRANSFER_COUNTS.with(|c| assert_eq!(c.get(), (0, 0, 0)));
        g.clock.set_running(true);
        assert!(g.advance(60.0));
        assert!(
            g.routes[0]
                .plan
                .as_ref()
                .unwrap()
                .airborne
                .as_ref()
                .unwrap()
                .totals
                .connecting_passengers
                > 0
        );
    }

    /// Diagnostic only: quotes use the actual expansion search.
    #[test]
    #[ignore]
    fn widebody_ai_expansion_quote_audit() {
        for (country, home) in [("DE", "EDDF"), ("BR", "SBGR"), ("JP", "RJTT")] {
            let mut world = crate::game::Game::default();
            world.found("Worldwide audit", country, home).unwrap();
            world.clock.set_running(true);
            assert!(world.advance(730.0 * 86400.0));
            let supply = world.market_routes();
            for c in &world.competitors {
                let origin = &c.game.airline.as_ref().unwrap().home_airport_icao;
                for model in ["a330_300", "b777_300er"] {
                    let quotes = c.expansion_quotes(&supply, model, origin, None).unwrap();
                    let best_long = quotes
                        .iter()
                        .filter(|q| {
                            crate::routes::distance_km(origin, &q.route.destination_icao).unwrap()
                                > 6200.0
                        })
                        .max_by_key(|q| q.profit);
                    println!(
                        "AI_WIDEBODY player={home} company={} model={model} cash={} fleet={} best_longhaul={:?}",
                        c.game.airline.as_ref().unwrap().name,
                        c.game.cash(),
                        c.game.fleet.len(),
                        best_long.map(|q| (
                            &q.route.destination_icao,
                            q.route.ticket_eur,
                            q.interval,
                            q.business_seats,
                            q.route.business_ticket_eur,
                            q.profit
                        ))
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "weekly acquisition and final service report"]
    fn cabin_expansion_behavior_audit() {
        for (country, home) in [("DE", "EDDF"), ("BR", "SBGR"), ("JP", "RJTT")] {
            let mut g = Game::default();
            g.found("Cabin audit", country, home).unwrap();
            let supply = g.market_routes();
            for c in &g.competitors {
                let origin = &c.game.airline.as_ref().unwrap().home_airport_icao;
                let targets: &[(&str, &str)] = match origin.as_str() {
                    "EDDF" => &[("b737_800", "OMDB"), ("a330_300", "EGLL")],
                    "SBGR" => &[("b737_800", "SCEL"), ("a330_300", "EGLL")],
                    "RJTT" => &[
                        ("b737_800", "VHHH"),
                        ("a320_200", "WSSS"),
                        ("a330_300", "EGLL"),
                    ],
                    _ => &[],
                };
                for (model, destination) in targets {
                    let quotes = c.expansion_quotes(&supply, model, origin, None).unwrap();
                    for business in [false, true] {
                        let best = quotes
                            .iter()
                            .filter(|q| {
                                q.route.destination_icao == *destination
                                    && (q.business_seats > 0) == business
                            })
                            .max_by_key(|q| q.profit);
                        if let Some(q) = best {
                            println!(
                                "CABIN_COMPARE player={home} {origin}-{destination} {model} J={} fare={}/{} week_before_rent={} fee={}",
                                q.business_seats,
                                q.route.ticket_eur,
                                q.route.business_ticket_eur,
                                q.profit,
                                q.layout_fee
                            );
                        }
                    }
                }
            }
            g.clock.set_running(true);
            for week in 1..=104 {
                let sizes: Vec<_> = g.competitors.iter().map(|c| c.game.fleet.len()).collect();
                assert!(g.advance(7.0 * 86400.0));
                for (c, before) in g.competitors.iter().zip(sizes) {
                    if c.game.fleet.len() > before {
                        let p = c.game.fleet.last().unwrap();
                        println!(
                            "CABIN_BUY player={home} week={week} company={} model={} J={} lease={} action={}",
                            c.game.airline.as_ref().unwrap().name,
                            p.model_id,
                            p.business_seats,
                            p.lease.is_some(),
                            c.last_action
                        );
                    }
                }
            }
            assert!(g.advance(2.0 * 86400.0));
            for c in &g.competitors {
                for r in &c.game.routes {
                    let p = c
                        .game
                        .fleet
                        .iter()
                        .find(|p| Some(p.id) == r.aircraft_id)
                        .unwrap();
                    println!(
                        "CABIN_ROUTE player={home} company={} {}-{} model={} cabin={:?} fare={}/{} active={}",
                        c.game.airline.as_ref().unwrap().name,
                        r.origin_icao,
                        r.destination_icao,
                        p.model_id,
                        crate::cabins::layout(p).unwrap(),
                        r.ticket_eur,
                        r.business_ticket_eur,
                        r.plan.is_some()
                    );
                }
            }
        }
    }

    #[test]
    fn widebody_expansion_installs_quoted_cabin_fare_and_books_fee() {
        for leased in [false, true] {
            let mut c = initial_competitors().unwrap().remove(0);
            c.game.stop_schedule(1).unwrap();
            c.game.routes.clear();
            c.game.next_route_id = 1;
            c.game.sell_aircraft(1).unwrap();
            // Execution fixture only; no production budget or selection override.
            let shift = 200_000_000 - c.game.cash();
            c.game.airline.as_mut().unwrap().capital_eur += shift;
            c.game.history.as_mut().unwrap().opening_balance += shift;
            let quotes = c.expansion_quotes(&[], "a330_300", "EDDF", None).unwrap();
            let quote = quotes
                .iter()
                .filter(|q| q.route.destination_icao == "KJFK")
                .max_by_key(|q| q.profit)
                .unwrap()
                .clone();
            assert!(quote.business_seats > 0);
            assert_eq!(
                quote.route.business_ticket_eur,
                crate::cabins::premium_fare(quote.route.ticket_eur)
            );
            let mut expected = c.game.clone();
            expected.equip_staff_for("a330_300").unwrap();
            if leased {
                expected.lease_aircraft("a330_300").unwrap();
            } else {
                expected.buy_aircraft("a330_300").unwrap();
            }
            let cash = expected.cash();
            let fare = quote.route.business_ticket_eur;
            let business = quote.business_seats;
            let interval = quote.interval;
            c.acquire_expansion(ExpansionChoice {
                used: None,
                plane: None,
                model: "a330_300".into(),
                quote,
                leased,
                score: 0,
            })
            .unwrap();
            assert_eq!(c.game.cash(), cash - crate::cabins::RULES.layout_fee_eur);
            assert_eq!(c.game.fleet[0].business_seats, business);
            assert_eq!(
                crate::cabins::layout(&c.game.fleet[0]).unwrap(),
                (
                    300 - business * crate::cabins::RULES.business_space_ratio,
                    business
                )
            );
            assert_eq!(c.game.routes[0].business_ticket_eur, fare);
            assert_eq!(
                c.game.routes[0].plan.as_ref().unwrap().interval_seconds,
                u64::from(interval) * 60
            );
            let entry = c
                .game
                .history
                .as_ref()
                .unwrap()
                .aircraft_cash
                .last()
                .unwrap();
            assert_eq!(entry.kind, crate::trading::Kind::Cabin);
            assert_eq!(entry.cash, -crate::cabins::RULES.layout_fee_eur);
            c.game.review_cabins();
            assert_eq!(c.game.fleet[0].business_seats, business);
            assert_eq!(c.game.routes[0].business_ticket_eur, fare);
            c.game.clock.set_running(true);
            assert!(c.game.advance(60.0));
            assert_eq!(
                c.game.fleet[0].status,
                crate::aircraft::AircraftStatus::InFlight
            );
            assert!(
                c.game.routes[0]
                    .plan
                    .as_ref()
                    .unwrap()
                    .airborne
                    .as_ref()
                    .unwrap()
                    .totals
                    .business_passengers
                    > 0
            );
            c.game.validate_history().unwrap();
        }
    }

    #[test]
    fn configured_starters_cover_running_costs_even_at_maximum_fuel() {
        for hub in &FREQUENCY.starter_hubs {
            let c = starter_competitors(
                Duration::ZERO,
                &[(
                    "Starter",
                    &hub.home,
                    &hub.destination,
                    &hub.model,
                    hub.ticket_eur,
                )],
            )
            .unwrap()
            .remove(0);
            let route = &c.game.routes[0];
            let quote = crate::economy::quote_at_price(
                route,
                &hub.model,
                &c.game.market_routes(),
                c.game.fuel_market.max_price(),
            )
            .unwrap();
            let week = (quote.profit() - maintenance_per_flight(route, &hub.model).unwrap())
                * (crate::pricing::RULES.fare_review_seconds
                    / route.plan.as_ref().unwrap().interval_seconds) as i64
                - c.game.monthly_payroll() * 7 / 30;
            assert!(
                week > 0,
                "{} starter loses {week} EUR/week at maximum fuel",
                hub.home
            );
        }
    }

    #[test]
    fn worldwide_starters_are_local_and_distant_and_old_companies_load_verbatim() {
        for airport in &crate::airports::catalog().unwrap().airports {
            let mut g = Game::default();
            g.found("Worldwide", &airport.country_code, &airport.icao)
                .unwrap();
            assert_eq!(g.competitors.len(), 2);
            let homes: Vec<_> = g
                .competitors
                .iter()
                .map(|c| c.game.airline.as_ref().unwrap().home_airport_icao.as_str())
                .collect();
            assert_ne!(homes[0], homes[1]);
            let distance = |hub: &str| {
                if hub == airport.icao {
                    0.0
                } else {
                    crate::routes::distance_km(&airport.icao, hub).unwrap()
                }
            };
            assert!(
                FREQUENCY
                    .starter_hubs
                    .iter()
                    .all(|h| distance(homes[0]) <= distance(&h.home))
            );
            assert!(crate::routes::distance_km(homes[0], homes[1]).unwrap() > 5_000.0);
            for c in &g.competitors {
                c.game.validate_history().unwrap();
                c.game.validate_operations().unwrap();
                assert_eq!(c.game.fleet.len(), 1);
                assert!(c.game.routes[0].plan.is_some());
            }
        }
        // Save v30 already persists every competitor. New-game configuration
        // must never relocate, recreate or reprice these legacy companies.
        let mut old = game();
        old.competitors = initial_competitors().unwrap();
        assert!(old.advance(45.0 * 86400.0));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy-rivals.json");
        crate::save::save(&path, &old).unwrap();
        let loaded = crate::save::load(&path).unwrap();
        assert_eq!(state(&old), state(&loaded));
        assert!(
            loaded
                .competitors
                .iter()
                .all(|c| c.game.airline.as_ref().unwrap().home_airport_icao == "EDDF")
        );
    }

    #[test]
    fn legacy_companies_remain_solvent_with_cabin_search() {
        let mut g = Game::default();
        g.found("Legacy market", "DE", "EDDF").unwrap();
        g.competitors = initial_competitors().unwrap();
        g.clock.set_running(true);
        for _ in 0..104 {
            assert!(g.advance(7.0 * 86400.0));
            for c in &g.competitors {
                assert!(c.game.cash() > 0 && !c.game.distressed());
                assert!(c.game.fleet.len() <= MAX_FLEET);
            }
        }
        assert!(g.advance(2.0 * 86400.0));
        g.validate_competitors().unwrap();
        for c in &g.competitors {
            assert!(c.game.fleet.len() > 1);
            c.game.validate_history().unwrap();
        }
    }

    #[test]
    fn used_market_ai_growth_uses_shared_purchase_and_wear() {
        let mut c = initial_competitors().unwrap().remove(0);
        c.game.stop_schedule(1).unwrap();
        c.game.routes.clear();
        c.game.next_route_id = 1;
        c.game.fleet[0].wear = c.game.fleet[0].wear.added(40, 3600).unwrap();
        let wear = c.game.fleet[0].wear.clone();
        c.game.sell_aircraft(1).unwrap();
        let shift = 100_000_000 - c.game.cash();
        c.game.history.as_mut().unwrap().opening_balance += shift;
        c.game.airline.as_mut().unwrap().capital_eur = 100_000_000;
        for model in ["a320_200", "b737_800"] {
            c.game.buy_aircraft(model).unwrap();
            let p = c.game.fleet.last_mut().unwrap();
            p.wear = wear.clone();
            let id = p.id;
            c.game.sell_aircraft(id).unwrap();
        }
        let listings = c.game.used_market.listings.clone();
        let before = c.game.cash();
        c.expand(&[]).unwrap();
        assert_eq!(c.game.fleet.len(), 1);
        let bought = &c.game.fleet[0];
        let listing = listings
            .iter()
            .find(|l| l.aircraft.model_id == bought.model_id)
            .unwrap();
        let price = c.game.used_price(listing).unwrap();
        assert_eq!(bought.wear, wear);
        assert_eq!(bought.purchase_price_eur, price);
        assert_eq!(
            c.game.cash(),
            before
                - price
                - if bought.business_seats > 0 {
                    crate::cabins::RULES.layout_fee_eur
                } else {
                    0
                }
        );
        assert_eq!(c.game.used_market.listings.len(), 2);
        assert!(c.last_action.contains("Gebrauchtflugzeug"));
        c.game.validate_history().unwrap();
    }

    fn frequency_ai() -> Competitor {
        let mut c = initial_competitors().unwrap().remove(0);
        c.game.stop_schedule(1).unwrap();
        c.game.routes[0].destination_icao = "EDDM".into();
        c.game.set_ticket_price(1, 90).unwrap();
        c.game.schedule(1, 1, 360).unwrap();
        c.game.buy_aircraft("atr72_600").unwrap();
        c.game.equip_staff_for("atr72_600").unwrap();
        c
    }

    #[test]
    fn manufacturer_ai_orders_selected_growth_once_and_waits_for_delivery() {
        let mut c = frequency_ai();
        let fleet = c.game.fleet.len();
        let cash = c.game.cash();
        let staff = c.game.staff.clone();
        let quote = c.game.quote_order("atr72_600", 1).unwrap();
        let routes = c.game.routes.clone();
        c.acquire_expansion(ExpansionChoice {
            used: None,
            plane: None,
            model: "atr72_600".into(),
            quote: ExpansionQuote {
                route: routes[0].clone(),
                interval: 360,
                business_seats: 0,
                layout_fee: 0,
                profit: 100_000,
                costs: 50_000,
            },
            leased: false,
            score: 100_000,
        })
        .unwrap();
        assert_eq!(c.game.fleet.len(), fleet);
        assert_eq!(c.game.orders.len(), 1);
        assert_eq!(c.game.cash(), cash - quote.deposit_per_aircraft);
        assert_eq!(
            serde_json::to_value(&c.game.staff).unwrap(),
            serde_json::to_value(staff).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&c.game.routes).unwrap(),
            serde_json::to_value(routes).unwrap()
        );
        // The second parked aircraft is already owned. Let the ordinary planner
        // use it, then ensure pending factory positions prevent duplicate buying.
        c.expand(&[]).unwrap();
        c.expand(&[]).unwrap();
        assert_eq!(c.game.orders.len(), 1);
        assert_eq!(c.game.fleet.len(), fleet);
        c.game.validate_history().unwrap();
    }

    #[test]
    fn manufacturer_restructuring_cancels_player_and_ai_orders_without_refund() {
        let mut player = Game::default();
        player.found("Factory rescue", "DE", "EDDF").unwrap();
        let ai = initial_competitors().unwrap().remove(0).game;
        for mut game in [player, *ai] {
            game.place_order("atr72_600", 1).unwrap();
            let cash = game.cash();
            let history = serde_json::to_value(&game.history).unwrap();
            game.financing.overdue = 1;
            game.observe_distress();
            let start = crate::financing::RULES.distress_grace_days * 86400;
            game.clock = Clock::from_state(Duration::from_secs(start), true, 1).unwrap();
            game.apply_resolution();
            assert!(game.orders.is_empty());
            assert_eq!(game.cash(), cash);
            assert_eq!(serde_json::to_value(&game.history).unwrap(), history);
            assert!(game.resolution.deadline.is_some());
            game.clock = Clock::from_state(
                Duration::from_secs(start + crate::financing::RULES.restructuring_days * 86400),
                true,
                1,
            )
            .unwrap();
            game.apply_resolution();
            assert!(game.insolvent());
            assert!(game.orders.is_empty());
        }
    }

    #[test]
    fn seasonal_ai_growth_uses_annual_view_and_weak_weeks_keep_viable_rotations() {
        let mut results = Vec::new();
        for day in [45, 180, 350] {
            let mut c = frequency_ai();
            c.game.clock = Clock::from_state(Duration::from_secs(day * 86400), true, 1).unwrap();
            c.expand(&[]).unwrap();
            results.push(
                c.game
                    .routes
                    .iter()
                    .map(|r| {
                        (
                            r.origin_icao.clone(),
                            r.destination_icao.clone(),
                            r.ticket_eur,
                            r.aircraft_id,
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }
        assert!(results.windows(2).all(|w| w[0] == w[1]));
        let mut c = frequency_ai();
        c.game.add_route_aircraft(1, 2).unwrap();
        c.game.clock.set_running(true);
        assert!(c.game.advance(3.0 * 86400.0));
        let supply = c.game.market_routes();
        for r in &c.game.routes {
            let annual = crate::economy::quote_at_price(
                r,
                "atr72_600",
                &supply,
                c.game
                    .fuel_market
                    .price_at(c.game.clock.elapsed().as_secs()),
            )
            .unwrap();
            assert!(annual.load_percent() >= FREQUENCY.load_factor_percent);
        }
        for _ in 0..3 {
            // Model a weak observed period without altering the annual market.
            c.route_reviews.clear();
            for r in &c.game.routes {
                c.route_reviews.push(RouteReview {
                    route_id: r.id,
                    profit: 10_000_000,
                    losses: 1,
                    flights: Some(0),
                });
            }
            c.review_rotations(&supply).unwrap();
            assert_eq!(c.game.routes.len(), 2);
            assert!(c.game.routes.iter().all(|r| r.remove_at.is_none()));
        }
    }

    #[test]
    fn frequency_growth_uses_shared_assignment_and_respects_cap() {
        // This scenario isolates the original city-pair choices; worldwide growth has its own audit.
        let _catalogue = crate::airports::original_catalogue_scenario();
        let mut c = frequency_ai();
        c.expand(&[]).unwrap();
        assert_eq!(c.game.routes.len(), 2);
        assert_eq!(
            c.game.routes[1].parent_route_id,
            Some(1),
            "{}",
            c.last_action
        );
        assert_eq!(c.game.routes[1].aircraft_id, Some(2));
        assert!(c.game.routes.iter().all(|r| r.plan.is_some()));
        let departures: Vec<_> = c
            .game
            .routes
            .iter()
            .map(|r| r.plan.as_ref().unwrap().first_departure)
            .collect();
        assert_eq!(departures[1] - departures[0], 10800);
        c.game.buy_aircraft("atr72_600").unwrap();
        c.expand(&[]).unwrap();
        assert_eq!(
            c.game
                .routes
                .iter()
                .filter(|r| r.public_id() == 1 && r.aircraft_id.is_some())
                .count(),
            FREQUENCY.max_aircraft_per_route
        );
        c.game.validate_routes().unwrap();
        c.game.validate_operations().unwrap();
    }

    #[test]
    fn frequency_growth_can_buy_or_lease_and_weak_loads_reduce_service() {
        for leased in [false, true] {
            let mut c = frequency_ai();
            c.game.sell_aircraft(2).unwrap();
            // Existing unassigned public routes reserve the other markets, so
            // this fixture isolates the purchase/lease path into the busy route.
            for airport in &crate::airports::catalog().unwrap().airports {
                if ["EDDF", "EDDM"].contains(&airport.icao.as_str()) {
                    continue;
                }
                let mut route = c.game.routes[0].clone();
                route.id = c.game.next_route_id;
                c.game.next_route_id += 1;
                route.destination_icao = airport.icao.clone();
                route.aircraft_id = None;
                route.plan = None;
                c.game.routes.push(route);
            }
            let capital = if leased { 1_500_000 } else { 100_000_000 };
            let shift = capital - c.game.cash();
            c.game.history.as_mut().unwrap().opening_balance += shift;
            c.game.airline.as_mut().unwrap().capital_eur = capital;
            c.expand(&[]).unwrap();
            if let Some(order) = c.game.orders.first() {
                // Healthy outright growth may now wait for the manufacturer.
                let until_delivery = order.delivery_at - c.game.clock.elapsed().as_secs();
                assert!(c.game.advance(until_delivery as f64));
                c.expand(&[]).unwrap();
            }
            assert_eq!(
                c.game.routes.last().unwrap().parent_route_id,
                Some(1),
                "{}",
                c.last_action
            );
            assert_eq!(c.game.fleet.last().unwrap().lease.is_some(), leased);
        }
        let mut g = game();
        let mut c = frequency_ai();
        c.game.add_route_aircraft(1, 2).unwrap();
        c.game.set_cabin_layout(1, 6).unwrap();
        c.game.set_cabin_layout(2, 6).unwrap();
        // Select a profitable but weak fare for the actual two-class capacity.
        // A fixed all-economy fare no longer describes the same load factor.
        let weak_price = (60..=200).find(|&price| {
            c.game.set_ticket_price(1, price).unwrap();
            c.game
                .set_business_ticket_price(1, crate::cabins::premium_fare(price))
                .unwrap();
            c.game.routes.iter().all(|r| {
                let q =
                    crate::economy::quote_shared(r, "atr72_600", &c.game.market_routes()).unwrap();
                q.profit() > 0 && q.load_percent() < FREQUENCY.load_factor_percent
            })
        });
        assert!(
            weak_price.is_some(),
            "fixture needs profitable weak service"
        );
        g.competitors = vec![c];
        assert!(g.advance(2.0 * crate::pricing::RULES.fare_review_seconds as f64 + 86400.0));
        assert_eq!(
            g.competitors[0]
                .game
                .routes
                .iter()
                .filter(|r| r.aircraft_id.is_some())
                .count(),
            1
        );
    }

    #[test]
    fn frequency_review_keeps_a_service_during_maintenance() {
        let mut c = frequency_ai();
        c.game.add_route_aircraft(1, 2).unwrap();
        c.game.fleet[1].wear = c.game.fleet[1].wear.added(1, 3600).unwrap();
        c.game.request_maintenance(2).unwrap();
        for _ in 0..3 {
            c.review_rotations(&c.game.market_routes()).unwrap();
        }
        assert_eq!(
            c.game
                .routes
                .iter()
                .filter(|r| r.aircraft_id.is_some())
                .count(),
            2
        );
        assert!(c.game.fleet[1].maintenance.is_some());
    }

    #[test]
    fn frequency_growth_skips_maintenance_wrong_location_and_weak_market() {
        for case in 0..3 {
            let mut c = frequency_ai();
            match case {
                0 => {
                    c.game.fleet[1].wear = c.game.fleet[1].wear.added(1, 3600).unwrap();
                    c.game.request_maintenance(2).unwrap();
                }
                1 => {
                    c.game.fleet[1].location_icao = "EDDM".into();
                }
                _ => {
                    c.game.set_ticket_price(1, 100_000).unwrap();
                }
            }
            c.expand(&[]).unwrap();
            assert!(c.game.routes.iter().all(|r| r.parent_route_id.is_none()));
        }
    }

    #[test]
    fn frequency_weak_airborne_service_retires_once_across_steps_speeds_and_restart() {
        let mut g = game();
        let mut c = frequency_ai();
        c.game.add_route_aircraft(1, 2).unwrap();
        c.game.set_ticket_price(1, 100_000).unwrap();
        // Ensure the second weekly review happens in flight.
        c.game.stop_schedule(1).unwrap();
        c.game.schedule(1, 179, 360).unwrap();
        g.competitors = vec![c];
        assert!(g.advance(2.0 * crate::pricing::RULES.fare_review_seconds as f64));
        let c = &g.competitors[0];
        assert_eq!(
            c.game
                .routes
                .iter()
                .filter(|r| r.remove_at.is_some())
                .count(),
            1
        );
        assert_eq!(
            c.game
                .routes
                .iter()
                .filter(|r| r.aircraft_id.is_some())
                .count(),
            2
        );
        let retained = c.game.routes[0].plan.clone();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rotation.json");
        crate::save::save(&path, &g).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        let mut small = g.clone();
        let mut fast = g.clone();
        let mut medium = g.clone();
        fast.clock.set_speed(16);
        medium.clock.set_speed(4);
        assert!(g.advance(86400.0));
        assert!(loaded.advance(86400.0));
        assert!(fast.advance(86400.0 / 16.0));
        assert!(medium.advance(86400.0 / 4.0));
        for _ in 0..1440 {
            assert!(small.advance(60.0));
        }
        fast.clock.set_speed(1);
        medium.clock.set_speed(1);
        assert_eq!(state(&g), state(&loaded));
        assert_eq!(state(&g), state(&small));
        assert_eq!(state(&g), state(&fast));
        assert_eq!(state(&g), state(&medium));
        let c = &g.competitors[0];
        assert_eq!(
            c.game
                .routes
                .iter()
                .filter(|r| r.aircraft_id.is_some())
                .count(),
            1
        );
        assert_eq!(
            c.game.routes[0].plan.as_ref().unwrap().first_departure,
            retained.unwrap().first_departure
        );
        assert!(c.game.routes[1].plan.is_none());
        g.validate_competitors().unwrap();
        // A grouped route with one survivor can still close after further losses.
        assert!(g.advance(3.0 * crate::pricing::RULES.fare_review_seconds as f64));
        assert!(g.competitors[0].game.routes.is_empty());
    }

    #[test]
    fn trading_ai_leases_only_affordable_profitable_growth_and_sells_in_distress() {
        let mut c = super::initial_competitors().unwrap().remove(0);
        c.game.routes.clear();
        c.game.fleet.clear();
        c.game.next_aircraft_id = 1;
        c.game.next_route_id = 1;
        c.game.staff = Default::default();
        c.game.finance = Default::default();
        c.game.airline.as_mut().unwrap().capital_eur = 1_500_000;
        c.game.history = Some(crate::history::History::new(0, 1_500_000));
        c.expand(&[]).unwrap();
        assert_eq!(c.game.fleet.len(), 1);
        assert!(c.game.fleet[0].lease.is_some());
        assert!(c.game.cash() >= super::RESERVE);
        c.game.validate_history().unwrap();
        // No grant: the exact contractual deposit was charged.
        let deposit = c.game.fleet[0].lease.as_ref().unwrap().deposit;
        assert_eq!(
            c.game.cash(),
            1_500_000
                - deposit
                - if c.game.fleet[0].business_seats > 0 {
                    crate::cabins::RULES.layout_fee_eur
                } else {
                    0
                }
        );
        let poor = c.game.cash();
        c.game.airline.as_mut().unwrap().capital_eur = 100;
        c.expand(&[]).unwrap();
        assert_eq!(c.game.fleet.len(), 1);
        c.game.airline.as_mut().unwrap().capital_eur = poor;

        // Same return rule for AI: clear the parked service and return its deposit.
        c.game.stop_schedule(c.game.routes[0].id).unwrap();
        let shift = -50_000 - c.game.cash();
        c.game.history.as_mut().unwrap().opening_balance += shift;
        c.game.airline.as_mut().unwrap().capital_eur = -50_000;
        let quote = c.game.disposal_quote(1).unwrap();
        assert!(c.review(&[]));
        assert!(c.game.fleet.is_empty());
        assert!(c.game.routes.is_empty());
        assert_eq!(c.game.cash(), -50_000 + quote);
        assert_eq!(c.game.financing.debt, 0);
        c.game.validate_history().unwrap();

        let mut c = super::initial_competitors().unwrap().remove(0);
        c.game.stop_schedule(1).unwrap();
        let quote = c.game.disposal_quote(1).unwrap();
        let shift = -50_000 - c.game.cash();
        c.game.history.as_mut().unwrap().opening_balance += shift;
        c.game.airline.as_mut().unwrap().capital_eur = -50_000;
        assert!(c.review(&[]));
        assert!(c.game.fleet.is_empty());
        assert_eq!(c.game.cash(), quote - 50_000);
        assert_eq!(c.game.financing.debt, 0);
        c.game.validate_history().unwrap();
    }

    #[test]
    fn trading_ai_lease_economy_is_bounded_and_deterministic_over_two_years() {
        let mut g = super::tests::game();
        for c in &mut g.competitors {
            c.game.routes.clear();
            c.game.fleet.clear();
            c.game.staff = Default::default();
            c.game.finance = Default::default();
            c.game.airline.as_mut().unwrap().capital_eur = 1_500_000;
            c.game.history = Some(crate::history::History::new(0, 1_500_000));
        }
        let mut expected = g.clone();
        assert!(expected.advance(730.0 * 86400.0));
        g.clock.set_speed(16);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai-leases.json");
        for day in 0..730 {
            assert!(g.advance(86400.0 / 16.0));
            if day == 360 {
                crate::save::save(&path, &g).unwrap();
                g = crate::save::load(&path).unwrap();
            }
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected));
        g.validate_competitors().unwrap();
        let mut rents = 0;
        for c in &g.competitors {
            assert!(c.game.fleet.len() <= super::MAX_FLEET);
            assert!(
                c.game.cash() >= 0,
                "AI failed to recover: {}",
                c.game.cash()
            );
            let h = c.game.history.as_ref().unwrap();
            rents += h.rent_through(u64::MAX);
            c.game.validate_history().unwrap();
            c.game.validate_financing().unwrap();
        }
        assert!(rents > 0, "scenario must exercise actual rent payments");
    }
    use super::*;

    fn game() -> Game {
        let mut g = Game::default();
        g.found("Player", "DE", "EDDF").unwrap();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
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
            assert!(g.advance(2.0 * crate::pricing::RULES.fare_review_seconds as f64));
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
        // This scenario isolates the original city-pair choices; worldwide growth has its own audit.
        let _catalogue = crate::airports::original_catalogue_scenario();
        let mut c = initial_competitors().unwrap().remove(0);
        c.game.stop_schedule(1).unwrap();
        c.game.delete_route(1).unwrap();
        let before = c.game.airline.as_ref().unwrap().capital_eur;
        let mut rival = game().routes[0].clone();
        rival.ticket_eur = 90;
        c.expand(&[crate::demand::Offer::new(&rival, 72)]).unwrap();
        assert_eq!(c.game.fleet.len(), 1);
        assert_eq!(c.game.airline.as_ref().unwrap().capital_eur, before);
        assert_eq!(c.game.routes.len(), 1);
        let r = &c.game.routes[0];
        assert_eq!(r.origin_icao, c.game.fleet[0].location_icao);
        let isolated =
            crate::demand::passengers(r, &[crate::demand::Offer::new(r, 10_000)]).unwrap();
        let shared = crate::demand::passengers(
            r,
            &[
                crate::demand::Offer::new(r, 10_000),
                crate::demand::Offer::new(&rival, 10_000),
            ],
        )
        .unwrap();
        assert!(shared < isolated);
        let base = crate::demand::market(r, 0).unwrap().total();
        let combined = (shared
            + crate::demand::passengers(
                &rival,
                &[
                    crate::demand::Offer::new(r, 10_000),
                    crate::demand::Offer::new(&rival, 10_000),
                ],
            )
            .unwrap())
            * 2;
        assert!(combined as f64 <= base);
    }

    #[test]
    fn expansion_spends_real_cash_and_obeys_budget_range_and_fleet_limit() {
        let mut g = game();
        assert!(g.advance(52.0 * crate::pricing::RULES.fare_review_seconds as f64));
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
            let batches = c.game.history.as_ref().unwrap().batches.len();
            assert!(
                batches < 5000,
                "{}: {batches} archive batches for {} flights",
                c.game.airline.as_ref().unwrap().name,
                c.game.finance.totals.flights
            );
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
            forced |= g.competitors[0].game.fleet.iter().any(|p| {
                p.maintenance
                    .as_ref()
                    .is_some_and(|v| v.unscheduled && v.cost_eur > 0)
            });
        }
        assert!(fast.advance(365.0 * 86400.0));
        assert_eq!(state(&g), state(&fast));
        let c = &g.competitors[0].game;
        assert!(!forced, "funded AI should plan checks before the limit");
        assert!(c.fleet.iter().map(|p| p.wear.checks).sum::<u64>() >= 2);
        let h = c.history.as_ref().unwrap();
        assert!(!h.maintenance.is_empty());
        let r = crate::maintenance::rules("atr72_600").unwrap();
        assert!(h.maintenance.iter().all(|m| m.amount >= r.base_cost_eur
            || (m.amount == 0 && c.insurance.recent.iter().any(|claim| claim.time == m.time))));
        assert_eq!(
            h.balance(c.clock.elapsed().as_secs()),
            c.airline.as_ref().unwrap().capital_eur
        );
        g.validate_competitors().unwrap();
        // An unrescuable AI now suspends service before accumulating more wear.
        let c = &mut g.competitors[0];
        let shift = c.game.airline.as_ref().unwrap().capital_eur + 1_000_000_000;
        c.game.history.as_mut().unwrap().opening_balance -= shift;
        c.game.airline.as_mut().unwrap().capital_eur -= shift;
        let mut forced = false;
        for _ in 0..200 {
            assert!(g.advance(86400.0));
            if let Some(c) = g.competitors.first() {
                forced |= c.game.fleet.iter().any(|p| {
                    p.maintenance
                        .as_ref()
                        .is_some_and(|v| v.unscheduled && v.cost_eur > 0)
                });
                assert!(c.game.routes.iter().all(|r| r.plan.is_none()));
            }
        }
        assert!(
            !forced,
            "distress must stop flights before another forced check"
        );
        assert!(
            g.competitors.is_empty(),
            "unrescued AI must leave the market"
        );
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
        let losses = |gain: i64, flights: Option<u64>, annual_loss: bool| {
            let mut c = base.clone();
            if annual_loss {
                c.game.routes[0].ticket_eur = 100_000;
            }
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
        // A healthy annual plan tolerates a seasonal loss; an unviable one does not.
        assert_eq!(losses(maintenance - 1, since, false), 0);
        // One landing earned less than its maintenance share: a losing week.
        assert_eq!(losses(maintenance - 1, since, true), 1);
        assert_eq!(losses(maintenance, since, true), 0);
        // Older saves without a landing count judge that first week as before.
        assert_eq!(losses(1, None, true), 0);
    }

    #[test]
    fn losses_are_measured_per_route_not_hidden_by_profitable_routes() {
        let mut g = game();
        let c = &mut g.competitors[0];
        c.game.buy_aircraft("atr72_600").unwrap();
        c.game.staff_up();
        c.game.create_route("EDDF", "EDDM", 2).unwrap();
        c.game.schedule(2, 1, 360).unwrap();
        c.game.routes[0].ticket_eur = 100_000;
        assert!(g.advance(2.0 * crate::pricing::RULES.fare_review_seconds as f64));
        let c = &g.competitors[0];
        assert!(!c.game.routes.iter().any(|r| r.id == 1));
        assert!(c.game.routes.iter().any(|r| r.id == 2 && r.plan.is_some()));
        assert_eq!(c.game.fleet.len(), 2);
        assert!(
            c.game
                .history
                .as_ref()
                .unwrap()
                .totals(1, 2 * crate::pricing::RULES.fare_review_seconds)
                .flights
                > 0
        );
        g.validate_competitors().unwrap();
    }

    #[test]
    fn common_market_price_frequency_capacity_and_separate_ledgers() {
        let mut g = game();
        let route = &g.routes[0];
        // Isolate fare/frequency choice with ample seats. Actual departures below
        // still use the real fleet capacity, including spill redistribution.
        let mut supply = g.market_routes();
        for o in &mut supply {
            o.seats = 10_000;
        }
        let own = crate::demand::passengers(route, &supply).unwrap();
        let rival = &g.competitors[0].game.routes[0];
        assert_eq!(own, crate::demand::passengers(rival, &supply).unwrap());
        let base = crate::demand::market(route, 0).unwrap().total();
        assert!((own * 4) as f64 <= base); // two airlines × two departures/direction
        let mut cheap = route.clone();
        cheap.ticket_eur = 50;
        let offers = vec![
            crate::demand::Offer::new(&cheap, 10_000),
            crate::demand::Offer::new(&rival, 10_000),
        ];
        let cheap_pax = crate::demand::passengers(&cheap, &offers).unwrap();
        let rival_pax = crate::demand::passengers(rival, &offers).unwrap();
        assert!(cheap_pax > own && rival_pax < own);
        let cheap_market = crate::demand::market(&cheap, 0).unwrap().total();
        assert!(((cheap_pax + rival_pax) * 2) as f64 <= cheap_market);
        let mut frequent = route.clone();
        frequent.plan.as_mut().unwrap().interval_seconds /= 2;
        let offers = vec![
            crate::demand::Offer::new(&frequent, 10_000),
            crate::demand::Offer::new(&rival, 10_000),
        ];
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
                    let offers = vec![
                        crate::demand::Offer::new(&a, 72),
                        crate::demand::Offer::new(&b, 72),
                    ];
                    let daily: f64 = offers
                        .iter()
                        .map(|r| {
                            let frequency =
                                43200.0 / r.route.plan.as_ref().unwrap().interval_seconds as f64;
                            crate::demand::passengers(&r.route, &offers).unwrap() as f64 * frequency
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
        assert!(g.advance(2.0 * crate::pricing::RULES.fare_review_seconds as f64));
        let c = &g.competitors[0];
        assert!(c.game.routes.is_empty());
        assert_eq!(c.game.fleet.len(), 1);
        assert!(c.game.finance.totals.profit() < 0);
        assert_eq!(c.game.finance.totals.passengers, 0);
        let flights = c.game.finance.totals.flights;
        assert!(g.advance(crate::pricing::RULES.fare_review_seconds as f64));
        assert_eq!(g.competitors[0].game.finance.totals.flights, flights);
        g.validate_competitors().unwrap();
        // After RETRY_AFTER idle reviews the rival re-enters at a viable fare.
        assert!(g.advance(
            f64::from(RETRY_AFTER - 1) * crate::pricing::RULES.fare_review_seconds as f64
        ));
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
        let _catalogue = crate::airports::original_catalogue_scenario();
        let mut c = initial_competitors().unwrap().remove(0);
        let mut free = c.game.routes[0].clone();
        free.id = 999;
        free.aircraft_id = Some(999);
        free.ticket_eur = 0;
        free.business_ticket_eur = 0;
        let mut offer = crate::demand::Offer::new(&free, 100_000);
        offer.business_seats = 50_000;
        c.game.stop_schedule(1).unwrap();
        c.game.set_cabin_layout(1, 7).unwrap();
        // Isolate retry cadence from expansion: two-class quotes can now
        // legitimately fund a different market during a whole-world advance.
        for review in 1..=RETRY_AFTER {
            assert!(c.retry(0, &[offer.clone()]));
            assert!(c.game.routes[0].plan.is_none());
            assert_eq!(c.idle_reviews, review % RETRY_AFTER);
        }
        assert!(c.last_action.contains("nicht rentabel"));
        // Once the obstruction disappears, still wait the complete retry period.
        for _ in 1..RETRY_AFTER {
            assert!(c.retry(0, &[]));
            assert!(c.game.routes[0].plan.is_none());
        }
        assert!(c.retry(0, &[]));
        assert!(c.game.routes[0].plan.is_some());
        c.game.validate_operations().unwrap();
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
                "/state/competitors/0/game/routes/0/aircraft_ids/0",
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
        // An already airborne loss is unavoidable even in distress.
        assert!(g.advance(120.0));
        g.competitors[0].game.airline.as_mut().unwrap().capital_eur = i64::MIN;
        let frozen = g.competitors[0].game.routes[0]
            .plan
            .as_mut()
            .unwrap()
            .airborne
            .as_mut()
            .unwrap();
        frozen.ticket_eur = 0;
        frozen.totals.revenue = 0;
        let before = state(&g);
        assert!(!g.advance(86400.0));
        assert_eq!(state(&g), before);
    }
}

#[cfg(test)]
mod slot_admission_tests {
    #[test]
    fn slots_ai_full_hub_skips_purchase_and_full_destination_uses_other_markets() {
        let mut c = super::initial_competitors().unwrap().remove(0);
        let before = c.game.fleet.len();
        c.game.external_slots.insert("EDDF".into(), 120);
        c.expand(&[]).unwrap();
        assert_eq!(c.game.fleet.len(), before);
        assert_eq!(c.game.routes.len(), 1);
        let mut stopped = c.clone();
        stopped.game.stop_schedule(1).unwrap();
        stopped.idle_reviews = super::RETRY_AFTER - 1;
        stopped.route_reviews.push(super::RouteReview {
            route_id: 1,
            ..Default::default()
        });
        assert!(stopped.retry(0, &[]));
        assert!(stopped.game.routes.is_empty());
        assert!(stopped.route_reviews.is_empty());
        c.game.external_slots.clear();
        c.game.external_slots.insert("LTFM".into(), 120);
        c.game.external_slots.insert("EDDF".into(), 108);
        // Existing EDDF service holds six; at most one more six-slot service fits.
        c.expand(&[]).unwrap();
        assert!(c.game.slot_usage("EDDF") <= 120);
        assert!(c.game.routes.iter().all(|r| r.destination_icao != "LTFM"));
        assert!(
            c.game.routes.len() > 1,
            "AI should find another profitable destination"
        );
    }
}

#[cfg(test)]
mod manufacturer_audit {
    #[test]
    fn manufacturer_orders_eddf_rival_comparison_two_and_ten_years() {
        let mut game = crate::game::Game::default();
        game.found("Manufacturer Audit", "DE", "EDDF").unwrap();
        game.clock.set_running(true);
        for day in 1..=3650 {
            assert!(game.advance(86400.0));
            if day == 730 || day == 3650 {
                for rival in &game.competitors {
                    println!(
                        "MANUFACTURER_AUDIT day={day} airline={} fleet={} cash={} insolvent={} distressed={}",
                        rival.game.airline.as_ref().unwrap().name,
                        rival.game.fleet.len(),
                        rival.game.cash(),
                        rival.game.insolvent(),
                        rival.game.distressed()
                    );
                }
            }
        }
        game.validate_competitors().unwrap();
    }
}
