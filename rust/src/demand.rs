//! Seasonal daily O&D market. No backlog, aircraft-size dependence or per-flight spawning.
use crate::{airports, routes::Route};
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
struct Config {
    daily_market: f64,
    full_distance_km: f64,
    distance_decay_km: f64,
    business_price_limit: f64,
    leisure_price_limit: f64,
    transfer_penalty: f64,
    transfer_max_detour: f64,
    transfer_min_leg_km: f64,
    business_monthly: [f64; 12],
    leisure_monthly: [f64; 12],
}
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let c: Config =
        serde_json::from_str(include_str!("../../config/demand.json")).expect("demand config");
    for n in [
        c.daily_market,
        c.full_distance_km,
        c.distance_decay_km,
        c.business_price_limit,
        c.leisure_price_limit,
    ] {
        assert!(n.is_finite() && n > 0.0 && n <= 100_000.0);
    }
    assert!((0.0..1.0).contains(&c.transfer_penalty));
    assert!((1.0..=3.0).contains(&c.transfer_max_detour));
    assert!((1.0..=1000.0).contains(&c.transfer_min_leg_km));
    for curve in [&c.business_monthly, &c.leisure_monthly] {
        assert!(
            curve
                .iter()
                .all(|v| v.is_finite() && *v > 0.0 && *v <= 10.0)
        );
    }
    c
});

pub const YEAR_SECONDS: u64 = 365 * 86400;

/// Twelve equally spaced knots, starting on simulation day 1 (January).
/// Cosine interpolation is continuous with zero slope at every knot, including
/// year wrap. Normalization makes the continuous annual mean exactly one.
fn curve_at(curve: &[f64; 12], time: u64, mean: f64) -> f64 {
    let month = (time % YEAR_SECONDS) as f64 * 12.0 / YEAR_SECONDS as f64;
    let i = month.floor() as usize;
    let blend = (1.0 - ((month - i as f64) * std::f64::consts::PI).cos()) / 2.0;
    (curve[i] * (1.0 - blend) + curve[(i + 1) % 12] * blend) / mean
}

static SEASON_MEANS: LazyLock<Market> = LazyLock::new(|| Market {
    business: CONFIG.business_monthly.iter().sum::<f64>() / 12.0,
    leisure: CONFIG.leisure_monthly.iter().sum::<f64>() / 12.0,
});
pub fn season_factors(time: u64) -> Market {
    Market {
        business: curve_at(&CONFIG.business_monthly, time, SEASON_MEANS.business),
        leisure: curve_at(&CONFIG.leisure_monthly, time, SEASON_MEANS.leisure),
    }
}

pub struct Market {
    pub business: f64,
    pub leisure: f64,
}
impl Market {
    pub fn total(&self) -> f64 {
        self.business + self.leisure
    }
}

/// Annual demand used for strategic planning, with the same market formula.
pub fn market(route: &Route, price: i64) -> Result<Market, String> {
    market_with_season(route, price, None)
}

#[cfg(test)]
pub fn market_at(route: &Route, price: i64, time: u64) -> Result<Market, String> {
    market_with_season(route, price, Some(time))
}

pub fn market_in_economy(
    route: &Route,
    price: i64,
    context: crate::world_economy::Context,
    time: Option<u64>,
) -> Result<Market, String> {
    Ok(MarketInputs::new(route, price)?.at(&crate::world_economy::route_factors(
        Some(context),
        time,
        &route.origin_icao,
        &route.destination_icao,
    )))
}

fn market_with_season(route: &Route, price: i64, time: Option<u64>) -> Result<Market, String> {
    let inputs = MarketInputs::new(route, price)?;
    Ok(inputs.at(&time.map(season_factors).unwrap_or(Market {
        business: 1.0,
        leisure: 1.0,
    })))
}

/// Geography and price response are constant within an offer snapshot. Keep
/// factors separate to preserve the original floating-point evaluation order.
#[derive(Clone, Copy)]
struct MarketInputs {
    km: f64,
    reference: f64,
    base: f64,
    share: f64,
    business: f64,
    leisure: f64,
}
thread_local! {
    static GEOGRAPHY: std::cell::RefCell<std::collections::HashMap<String, std::collections::HashMap<String, MarketInputs>>> = std::cell::RefCell::new(std::collections::HashMap::new());
}
impl MarketInputs {
    fn new(route: &Route, price: i64) -> Result<Self, String> {
        if let Some(base) = GEOGRAPHY.with(|cache| {
            cache
                .borrow()
                .get(&route.origin_icao)
                .and_then(|pairs| pairs.get(&route.destination_icao))
                .copied()
        }) {
            return Ok(base.with_price(price));
        }
        let base = Self::geography(route)?;
        GEOGRAPHY.with(|cache| {
            cache
                .borrow_mut()
                .entry(route.origin_icao.clone())
                .or_default()
                .insert(route.destination_icao.clone(), base);
        });
        Ok(base.with_price(price))
    }
    fn geography(route: &Route) -> Result<Self, String> {
        let airports = &airports::catalog()?.airports;
        let a = airports
            .iter()
            .find(|a| a.icao == route.origin_icao)
            .ok_or("Unbekannter Flughafen.")?;
        let b = airports
            .iter()
            .find(|a| a.icao == route.destination_icao)
            .ok_or("Unbekannter Flughafen.")?;
        let km = crate::routes::distance_km(&a.icao, &b.icao)?;
        let c = &*CONFIG;
        let base = c.daily_market
            * (a.market_size * b.market_size).sqrt()
            * (km / c.full_distance_km).min(1.0)
            / (1.0 + km / c.distance_decay_km);
        let share = f64::from(a.business_percent + b.business_percent) / 200.0;
        let reference = crate::economy::reference_price(km);
        Ok(Self {
            km,
            reference,
            base,
            share,
            business: 0.0,
            leisure: 0.0,
        }
        .with_price(0))
    }
    /// All offers in a prepared market have the same endpoints. Reuse their
    /// geography, preserving the exact willingness and allocation arithmetic.
    fn with_price(&self, price: i64) -> Self {
        let willingness =
            |limit: f64| (1.0 - price as f64 / (self.reference * limit)).clamp(0.0, 1.0);
        Self {
            km: self.km,
            reference: self.reference,
            base: self.base,
            share: self.share,
            business: willingness(CONFIG.business_price_limit),
            leisure: willingness(CONFIG.leisure_price_limit),
        }
    }
    fn at(&self, season: &Market) -> Market {
        Market {
            business: season.business * self.base * self.share * self.business,
            leisure: season.leisure * self.base * (1.0 - self.share) * self.leisure,
        }
    }
}

/// Ephemeral supply snapshot: capacity is derived from the owning fleet, never saved.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub codeshare_partners: Vec<String>,
    pub economy: Option<crate::world_economy::Context>,
    pub route: Route,
    pub seats: u32,
    pub business_seats: u32,
    pub cargo_model: Option<String>,
    premium: bool,
    pub awareness: i64,
    pub alliance_bonus_bps: i64,
    pub loyalty_members: i64,
    pub reputation: i64,
    pub owner: String,
}
impl Offer {
    fn alliance_choice(&self) -> f64 {
        1.0 + self.alliance_bonus_bps as f64 / 10_000.0
    }
    fn allocation_order(&self, other: &Self) -> std::cmp::Ordering {
        self.route
            .ticket_eur
            .cmp(&other.route.ticket_eur)
            .then_with(|| frequency(&self.route).total_cmp(&frequency(&other.route)))
            .then(self.seats.cmp(&other.seats))
            .then(self.premium.cmp(&other.premium))
            .then(self.route.service_level.cmp(&other.route.service_level))
            .then(
                self.route
                    .ancillary_policy
                    .cmp(&other.route.ancillary_policy),
            )
            .then(self.loyalty_members.cmp(&other.loyalty_members))
            .then(self.awareness.cmp(&other.awareness))
            .then(self.alliance_bonus_bps.cmp(&other.alliance_bonus_bps))
            .then(self.reputation.cmp(&other.reputation))
    }
    fn connects_with(&self, other: &Self) -> bool {
        self.owner == other.owner
            || (self.codeshare_partners.contains(&other.owner)
                && other.codeshare_partners.contains(&self.owner))
    }
    /// Only commercial fields enter demand; never copy frozen results or
    /// disruption cursors into a prepared market.
    pub(crate) fn commercial_clone(&self) -> Self {
        let r = &self.route;
        Self {
            codeshare_partners: self.codeshare_partners.clone(),
            economy: self.economy,
            route: Route {
                id: r.id,
                origin_icao: r.origin_icao.clone(),
                destination_icao: r.destination_icao.clone(),
                aircraft_id: r.aircraft_id,
                parent_route_id: r.parent_route_id,
                cargo_rate_cents: r.cargo_rate_cents,
                ancillary_policy: r.ancillary_policy,
                loyalty_level: r.loyalty_level,
                service_level: r.service_level,
                ticket_eur: r.ticket_eur,
                business_ticket_eur: r.business_ticket_eur,
                remove_at: None,
                rotation_phase: None,
                plan: r.plan.as_ref().map(|p| crate::operations::FlightPlan {
                    interval_seconds: p.interval_seconds,
                    ..Default::default()
                }),
            },
            seats: self.seats,
            business_seats: self.business_seats,
            cargo_model: self.cargo_model.clone(),
            premium: self.premium,
            awareness: self.awareness,
            alliance_bonus_bps: self.alliance_bonus_bps,
            loyalty_members: self.loyalty_members,
            reputation: self.reputation,
            owner: self.owner.clone(),
        }
    }
    pub fn new(route: &Route, seats: u32) -> Self {
        Self {
            codeshare_partners: Vec::new(),
            economy: None,
            route: route.clone(),
            seats,
            business_seats: 0,
            cargo_model: None,
            premium: false,
            awareness: 0,
            alliance_bonus_bps: 0,
            loyalty_members: 0,
            reputation: 0,
            owner: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Estimate {
    pub codeshare: Vec<crate::codeshare::Receipt>,
    pub business: i64,
    pub connecting: i64,
    pub connecting_revenue: i64,
    pub daily_connecting: f64,
    pub connections: Vec<Connection>,
    pub daily_passengers: f64,
    pub daily_spill: f64,
    pub per_departure: i64,
}

#[inline(always)]
fn frequency(route: &Route) -> f64 {
    route
        .plan
        .as_ref()
        .map_or(1.0, |p| 43200.0 / p.interval_seconds as f64)
}

#[inline(always)]
pub(crate) fn same_offer(a: &Route, b: &Route) -> bool {
    same_market(a, b)
        && a.id == b.id
        && a.aircraft_id == b.aircraft_id
        && a.ticket_eur == b.ticket_eur
        && a.business_ticket_eur == b.business_ticket_eur
        && a.cargo_rate_cents == b.cargo_rate_cents
        && a.service_level == b.service_level
        && a.ancillary_policy == b.ancillary_policy
        && a.loyalty_level == b.loyalty_level
        && a.plan.as_ref().map(|p| p.interval_seconds)
            == b.plan.as_ref().map(|p| p.interval_seconds)
}

/// Daily rates per direction. Redistribute only seat spill, preserving the outside
/// option and each segment's willingness ceiling. No booking state or randomness.
#[cfg(test)]
pub fn estimate(route: &Route, seats: u32, routes: &[Offer]) -> Result<Estimate, String> {
    estimate_with_season(route, seats, routes, None, None)
}

pub fn estimate_at(
    route: &Route,
    seats: u32,
    routes: &[Offer],
    time: u64,
) -> Result<Estimate, String> {
    estimate_with_season(route, seats, routes, Some(time), None)
}

#[cfg(test)]
/// Passenger loads for each active offer in one city pair, in input order.
/// Shares the exact cabin allocator and rounding with individual flight quotes.
pub fn market_passengers(routes: &[Offer], time: Option<u64>) -> Result<Vec<i64>, String> {
    let Some(first) = routes.first() else {
        return Ok(Vec::new());
    };
    let mut loads = Vec::new();
    estimate_with_season(&first.route, first.seats, routes, time, Some(&mut loads))?;
    Ok(loads)
}

#[inline(always)]
fn rounded_passengers(daily: f64, offer: &Offer) -> i64 {
    let per_flight = daily / frequency(&offer.route);
    let cap = f64::from(offer.seats);
    if (per_flight - cap).abs() < 1e-9 {
        cap as i64
    } else {
        per_flight.floor().min(cap) as i64
    }
}

fn estimate_with_season(
    route: &Route,
    seats: u32,
    routes: &[Offer],
    time: Option<u64>,
    loads: Option<&mut Vec<i64>>,
) -> Result<Estimate, String> {
    let mut prepared = Prepared::new(route, seats, routes)?;
    let (estimate, allocated) = prepared.estimate_display(time);
    if let Some(loads) = loads {
        loads.clear();
        loads.extend_from_slice(allocated);
    }
    Ok(estimate)
}

// Keep each cabin's allocation scratch together instead of allocating and
// repeatedly indexing three parallel vectors in every spill round.
#[derive(Clone, Default)]
struct LocalAllocation {
    weights: [f64; 2],
    limits: [f64; 2],
    allocated: [f64; 2],
}

/// Local, unsaved quote workspace for one route and immutable supply snapshot.
/// Every seasonal quantity and spill result is recomputed at departure time.
#[derive(Clone)]
struct LocalPrepared {
    services: Vec<Offer>,
    requested: Vec<f64>,
    matching: Vec<Offer>,
    own_indices: Vec<usize>,
    service_indices: Vec<usize>,
    inputs: Vec<MarketInputs>,
    base: MarketInputs,
    capacities: Vec<f64>,
    allocations: Vec<LocalAllocation>,
    loads: Vec<i64>,
    frequency: f64,
    frequencies: Vec<f64>,
    choices: Vec<f64>,
    #[cfg(test)]
    neutral_choice: bool,
}
impl LocalPrepared {
    fn refresh_constants(&mut self) {
        for (j, o) in self.matching.iter().enumerate() {
            let f = frequency(&o.route);
            self.frequencies[j] = f;
            self.choices[j] = crate::loyalty::weight(
                o.loyalty_members,
                o.route.ancillary_policy.weight(
                    crate::marketing::attractiveness(o.awareness)
                        * crate::reputation::attractiveness(o.reputation)
                        * o.route.service_level.rate().attractiveness
                        * o.alliance_choice()
                        * f,
                ),
            );
        }
        #[cfg(test)]
        {
            self.neutral_choice = crate::reputation::NEUTRAL_CHOICE.with(|v| v.get());
        }
    }

    /// With one service and ordered cabin fares, keep the
    /// same geography, capacities and allocation storage; refresh willingness.
    pub(crate) fn reprice_single(&mut self, route: &Route) {
        assert_eq!(self.loads.len(), 1);
        assert_eq!(self.own_indices.len(), self.matching.len());
        debug_assert_eq!(self.frequency, frequency(route));
        // A single service has at most economy + business. Other offer keys
        // (frequency, quality, awareness, reputation) remain identical.
        assert!(route.business_ticket_eur >= route.ticket_eur);
        for offer in &mut self.matching {
            offer.route.ticket_eur = if offer.premium {
                route.business_ticket_eur
            } else {
                route.ticket_eur
            };
            offer.route.business_ticket_eur = route.business_ticket_eur;
        }
        // Zero/equal fares can swap the two cabins' canonical order.
        self.matching.sort_by(|a, b| {
            a.route
                .ticket_eur
                .cmp(&b.route.ticket_eur)
                .then(a.seats.cmp(&b.seats))
                .then(a.premium.cmp(&b.premium))
        });
        for (i, offer) in self.matching.iter().enumerate() {
            self.capacities[i] = self.frequency * f64::from(offer.seats);
            self.inputs[i] = self.base.with_price(if offer.premium {
                (route.business_ticket_eur as f64 / crate::cabins::RULES.business_price_multiplier)
                    .ceil() as i64
            } else {
                route.ticket_eur
            });
        }
        self.refresh_constants();
    }

    /// Reuse the fixed cabins, geography and allocation storage across an AI
    /// fare sweep, including competing services. Keep canonical reduction order.
    fn canonicalize(&mut self, own: usize) {
        let mut ordered: Vec<_> = self
            .matching
            .drain(..)
            .zip(self.service_indices.drain(..))
            .collect();
        ordered.sort_by(|(a, _), (b, _)| a.allocation_order(b));
        self.own_indices.clear();
        for (i, (offer, service)) in ordered.into_iter().enumerate() {
            if service == own {
                self.own_indices.push(i);
            }
            self.capacities[i] = frequency(&offer.route) * f64::from(offer.seats);
            self.inputs[i] = self.base.with_price(if offer.premium {
                (offer.route.ticket_eur as f64 / crate::cabins::RULES.business_price_multiplier)
                    .ceil() as i64
            } else {
                offer.route.ticket_eur
            });
            self.matching.push(offer);
            self.service_indices.push(service);
        }
        self.refresh_constants();
    }

    fn set_reputation(&mut self, score: impl Fn(&str) -> Option<i64>) {
        let own = self.service_indices[self.own_indices[0]];
        let mut changed = false;
        for offer in &mut self.matching {
            if let Some(value) = score(&offer.owner)
                && offer.reputation != value
            {
                offer.reputation = value;
                changed = true;
            }
        }
        for offer in &mut self.services {
            if let Some(value) = score(&offer.owner) {
                offer.reputation = value;
            }
        }
        if changed {
            if self
                .matching
                .windows(2)
                .all(|pair| pair[0].allocation_order(&pair[1]) != std::cmp::Ordering::Greater)
            {
                // Reputation changes choice weights, not prices or capacities.
                // Keep their storage and reduction order when already canonical.
                self.refresh_constants();
            } else {
                self.canonicalize(own);
            }
        }
    }

    fn reprice_direct(&mut self, route: &Route) {
        if self.loads.len() == 1 {
            self.reprice_single(route);
            return;
        }
        let own = self.service_indices[self.own_indices[0]];
        if let Some(service) = self.services.get_mut(own) {
            service.route.ticket_eur = route.ticket_eur;
            service.route.business_ticket_eur = route.business_ticket_eur;
        }
        for (j, offer) in self.matching.iter_mut().enumerate() {
            if self.service_indices[j] == own {
                offer.route.ticket_eur = if offer.premium {
                    route.business_ticket_eur
                } else {
                    route.ticket_eur
                };
                offer.route.business_ticket_eur = route.business_ticket_eur;
            }
        }
        self.canonicalize(own);
    }

    pub fn new(route: &Route, seats: u32, routes: &[Offer]) -> Result<Self, String> {
        Self::prepare(route, seats, routes, true)
    }
    fn prepare(
        route: &Route,
        seats: u32,
        routes: &[Offer],
        keep_services: bool,
    ) -> Result<Self, String> {
        let mut matching: Vec<_> = routes
            .iter()
            .filter(|o| o.route.plan.is_some() && same_market(route, &o.route))
            .map(Offer::commercial_clone)
            .collect();
        let own = routes.iter().find(|o| same_offer(route, &o.route));
        if own.is_some_and(|o| o.business_seats > seats) {
            return Err("Ungültige Kabinenkapazität.".into());
        }
        if !matching
            .iter()
            .any(|o| o.seats == seats && same_offer(route, &o.route))
        {
            let mut own = own
                .map(Offer::commercial_clone)
                .unwrap_or_else(|| Offer::new(route, seats));
            own.seats = seats;
            if let Some(o) = matching.iter_mut().find(|o| same_offer(route, &o.route)) {
                *o = own;
            } else {
                matching.push(own);
            }
        }
        let services = if keep_services {
            matching.clone()
        } else {
            Vec::new()
        };
        // Both cabins participate in this same segmented allocator and spill loop.
        // Premium has no leisure weight; unsatisfied business can spill to economy.
        let service_count = matching.len();
        let mut cabins = Vec::with_capacity(service_count * 2);
        let own_index = matching
            .iter()
            .position(|o| o.seats == seats && same_offer(route, &o.route))
            .unwrap();
        for (index, mut economy) in matching.into_iter().enumerate() {
            let is_own = index == own_index;
            let business = if economy.business_seats > 0 {
                let mut business = economy.clone();
                business.seats = business.business_seats;
                business.route.ticket_eur = business.route.business_ticket_eur;
                business.premium = true;
                Some(business)
            } else {
                None
            };
            economy.seats -= economy.business_seats;
            cabins.push((economy, is_own, index));
            if let Some(business) = business {
                cabins.push((business, is_own, index));
            }
        }
        // Tag cabins as they are built, avoiding intermediate offer copies.
        let mut matching = cabins;
        // Economic keys give a canonical floating-point reduction order even when
        // owner order, route IDs or direction differ. Equal keys are interchangeable.
        matching.sort_by(|(a, _, _), (b, _, _)| a.allocation_order(b));
        let own_indices: Vec<_> = matching
            .iter()
            .enumerate()
            .filter_map(|(i, (_, own, _))| own.then_some(i))
            .collect();
        let service_indices: Vec<_> = matching.iter().map(|(_, _, index)| *index).collect();
        let matching: Vec<_> = matching.into_iter().map(|(o, _, _)| o).collect();
        let base = MarketInputs::new(route, 0)?;
        let inputs = matching
            .iter()
            .map(|o| {
                base.with_price(if o.premium {
                    (o.route.ticket_eur as f64 / crate::cabins::RULES.business_price_multiplier)
                        .ceil() as i64
                } else {
                    o.route.ticket_eur
                })
            })
            .collect();
        let capacities = matching
            .iter()
            .map(|o| frequency(&o.route) * f64::from(o.seats))
            .collect();
        let len = matching.len();
        let mut prepared = Self {
            requested: vec![0.0; services.len()],
            frequencies: vec![0.0; len],
            choices: vec![0.0; len],
            #[cfg(test)]
            neutral_choice: false,
            services,
            matching,
            own_indices,
            service_indices,
            inputs,
            base,
            capacities,
            allocations: vec![LocalAllocation::default(); len],
            loads: vec![0; service_count],
            frequency: frequency(route),
        };
        prepared.refresh_constants();
        Ok(prepared)
    }

    fn for_service(&self, service: usize) -> Estimate {
        let mut daily = 0.0;
        let mut per_departure = 0;
        let mut business = 0;
        for (j, offer) in self.matching.iter().enumerate() {
            if self.service_indices[j] == service {
                let allocated = self.allocations[j].allocated[0] + self.allocations[j].allocated[1];
                daily += allocated;
                let count = rounded_passengers(allocated, offer);
                per_departure += count;
                if offer.premium {
                    business += count;
                }
            }
        }
        Estimate {
            business,
            per_departure,
            daily_passengers: per_departure as f64 * frequency(&self.services[service].route),
            daily_spill: (self.requested[service] - daily).max(0.0),
            ..Default::default()
        }
    }

    pub fn estimate(&mut self, time: Option<u64>) -> (Estimate, &[i64]) {
        #[cfg(test)]
        if self.neutral_choice != crate::reputation::NEUTRAL_CHOICE.with(|v| v.get()) {
            self.refresh_constants();
        }
        let (origin, destination) = self
            .matching
            .first()
            .map_or(("", ""), |o| (&*o.route.origin_icao, &*o.route.destination_icao));
        let season = crate::world_economy::route_factors(
            self.matching.iter().find_map(|o| o.economy),
            time,
            origin,
            destination,
        );
        let base = self.base.at(&season);
        let matching = &self.matching;
        let allocations = &mut self.allocations;
        let mut frequency_sum = [0.0_f64; 2];
        let mut weight_sum = [0.0_f64; 2];
        for (j, (o, allocation)) in matching.iter().zip(allocations.iter_mut()).enumerate() {
            let f = self.frequencies[j];
            let choice = self.choices[j];
            let mut m = self.inputs[j].at(&season);
            if o.seats == 0 {
                m.business = 0.0;
                m.leisure = 0.0;
            }
            if o.premium {
                m.leisure = 0.0;
            }
            for (i, (n, b)) in [(m.business, base.business), (m.leisure, base.leisure)]
                .into_iter()
                .enumerate()
            {
                let w = if b > 0.0 { n / b } else { 0.0 };
                if w > 0.0 && !o.premium {
                    frequency_sum[i] += f;
                }
                allocation.weights[i] = choice * w / (1.0 - w).max(1e-9)
                    * if o.premium {
                        crate::cabins::RULES.business_preference
                    } else {
                        1.0
                    };
                weight_sum[i] += allocation.weights[i];
                // Rare flights cannot bank days of willing demand.
                allocation.limits[i] = n * f.min(1.0);
            }
        }
        for allocation in allocations.iter_mut() {
            for (i, b) in [base.business, base.leisure].into_iter().enumerate() {
                allocation.allocated[i] = (b * allocation.weights[i]
                    / (frequency_sum[i].max(1.0) + weight_sum[i]))
                    .min(allocation.limits[i]);
            }
        }
        let matching = &self.matching;
        let own_indices = &self.own_indices;
        let service_indices = &self.service_indices;
        let capacities = &self.capacities;

        self.requested.fill(0.0);
        if !self.requested.is_empty() {
            for (allocation, &service) in allocations.iter().zip(service_indices) {
                self.requested[service] += allocation.allocated[0] + allocation.allocated[1];
            }
        }
        let requested = own_indices
            .iter()
            .map(|&j| allocations[j].allocated[0] + allocations[j].allocated[1])
            .sum::<f64>();
        let mut spill = [0.0; 2];
        let mut has_spare = false;
        for (allocation, &capacity) in allocations.iter_mut().zip(capacities) {
            let total = allocation.allocated[0] + allocation.allocated[1];
            let scale = if total > capacity {
                capacity / total
            } else {
                1.0
            };
            for (i, excess) in spill.iter_mut().enumerate() {
                *excess += allocation.allocated[i] * (1.0 - scale);
                allocation.allocated[i] *= scale;
            }
            has_spare |= capacity - (allocation.allocated[0] + allocation.allocated[1]) > 1e-9;
        }
        // Each non-final round saturates at least one seat/segment ceiling.
        for _ in 0..matching.len() * 3 + 1 {
            // Empty spill or no eligible spare capacity makes the original
            // round a no-op. Keep tiny nonzero spills and eligible capacity on
            // the original path, including all its rounding.
            if spill == [0.0; 2] || !has_spare {
                break;
            }
            let mut sums = [0.0; 2];
            for (allocation, &capacity) in allocations.iter().zip(capacities) {
                if capacity - (allocation.allocated[0] + allocation.allocated[1]) > 1e-9 {
                    for (i, sum) in sums.iter_mut().enumerate() {
                        if allocation.limits[i] - allocation.allocated[i] > 1e-9 {
                            *sum += allocation.weights[i];
                        }
                    }
                }
            }
            let mut accepted = [0.0; 2];
            for (allocation, &capacity) in allocations.iter_mut().zip(capacities) {
                let spare =
                    (capacity - (allocation.allocated[0] + allocation.allocated[1])).max(0.0);
                if spare <= 1e-9 {
                    continue;
                }
                let mut add = [0.0; 2];
                for i in 0..2 {
                    if sums[i] > 0.0 && allocation.limits[i] - allocation.allocated[i] > 1e-9 {
                        add[i] = (spill[i] * allocation.weights[i] / sums[i])
                            .min(allocation.limits[i] - allocation.allocated[i]);
                    }
                }
                let total = add[0] + add[1];
                let scale = if total > spare { spare / total } else { 1.0 };
                for i in 0..2 {
                    let n = add[i] * scale;
                    allocation.allocated[i] += n;
                    accepted[i] += n;
                }
            }
            if (accepted[0] + accepted[1]) <= 1e-9 {
                break;
            }
            for i in 0..2 {
                spill[i] = (spill[i] - accepted[i]).max(0.0);
            }
        }
        let daily = own_indices
            .iter()
            .map(|&j| allocations[j].allocated[0] + allocations[j].allocated[1])
            .sum::<f64>();
        let mut business = 0;
        let mut per_departure = 0;
        for &j in own_indices {
            let n = rounded_passengers(
                allocations[j].allocated[0] + allocations[j].allocated[1],
                &matching[j],
            );
            per_departure += n;
            if matching[j].premium {
                business += n;
            }
        }
        self.loads.fill(0);
        for (j, offer) in matching.iter().enumerate() {
            self.loads[service_indices[j]] += rounded_passengers(
                allocations[j].allocated[0] + allocations[j].allocated[1],
                offer,
            );
        }
        (
            Estimate {
                business,
                daily_passengers: per_departure as f64 * self.frequency,
                daily_spill: (requested - daily).max(0.0),
                per_departure,
                ..Estimate::default()
            },
            self.loads.as_slice(),
        )
    }
}

/// Displayed daily rates are per direction; both directions share this symmetric
/// rate model. A path is identified by its ultimate endpoints and transfer airport.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Connection {
    pub partner: String,
    pub origin: String,
    pub hub: String,
    pub destination: String,
    pub daily: f64,
}

#[derive(Clone)]
pub(crate) struct Prepared {
    local: Option<LocalPrepared>,
    network: Option<std::sync::Arc<std::sync::Mutex<TransferNetwork>>>,
    target: Option<usize>,
    loads: Vec<i64>,
    transfer_services: Vec<Option<usize>>,
    transfer_loads: Vec<i64>,
    service_seats: Vec<u32>,
    quotes: std::collections::HashMap<Option<u64>, (Estimate, Vec<i64>)>,
    reputation_revision: Option<u64>,
    reputation_scores: Vec<i64>,
}
impl Prepared {
    pub(crate) fn direct(route: &Route, seats: u32, routes: &[Offer]) -> Result<Self, String> {
        Ok(Self {
            local: Some(LocalPrepared::prepare(route, seats, routes, false)?),
            network: None,
            target: None,
            loads: Vec::new(),
            transfer_services: Vec::new(),
            transfer_loads: Vec::new(),
            service_seats: Vec::new(),
            quotes: Default::default(),
            reputation_revision: None,
            reputation_scores: Vec::new(),
        })
    }
    pub fn new(route: &Route, seats: u32, routes: &[Offer]) -> Result<Self, String> {
        Self::in_supply(route, seats, routes, &mut None, None)
    }
    pub(crate) fn in_supply(
        route: &Route,
        seats: u32,
        routes: &[Offer],
        shared: &mut Option<SharedTransferSupply>,
        revision: Option<u64>,
    ) -> Result<Self, String> {
        let own = routes.iter().find(|o| same_offer(route, &o.route));
        let owner = own.map(|o| &o.owner);
        let eligible = own.is_some_and(|a| {
            a.seats > a.business_seats
                && routes.iter().any(|b| {
                    a.connects_with(b)
                        && b.route.plan.is_some()
                        && b.seats > b.business_seats
                        && !same_market(route, &b.route)
                        && [&route.origin_icao, &route.destination_icao]
                            .iter()
                            .any(|airport| {
                                **airport == b.route.origin_icao
                                    || **airport == b.route.destination_icao
                            })
                })
        });
        if route.plan.is_none() || !eligible {
            return Self::direct(route, seats, routes);
        }
        if shared.is_none() {
            *shared = Some(match revision {
                Some(revision) => dispatch_transfer_supply(revision, routes)?,
                None => std::sync::Arc::new(std::sync::Mutex::new(TransferSupply::new(routes)?)),
            });
        }
        let network = shared
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .network_for(route)?;
        if network.is_none() {
            return Self::direct(route, seats, routes);
        }
        let target = network.as_ref().and_then(|n| {
            n.lock().unwrap().offers.iter().position(|o| {
                same_offer(route, &o.route) && Some(&o.owner) == owner && o.seats == seats
            })
        });
        if target.is_none() {
            return Self::direct(route, seats, routes);
        }
        let reputation_scores = network
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .offers
            .iter()
            .map(|o| {
                routes
                    .iter()
                    .find(|key| key.owner == o.owner)
                    .map_or(o.reputation, |key| key.reputation)
            })
            .collect();
        let transfer_services = routes
            .iter()
            .filter(|o| o.route.plan.is_some() && same_market(route, &o.route))
            .map(|key| {
                network
                    .as_ref()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .offers
                    .iter()
                    .position(|o| o.owner == key.owner && same_offer(&o.route, &key.route))
            })
            .collect();
        Ok(Self {
            reputation_revision: None,
            reputation_scores,
            quotes: Default::default(),
            service_seats: routes
                .iter()
                .filter(|o| o.route.plan.is_some() && same_market(route, &o.route))
                .map(|o| o.seats)
                .collect(),
            transfer_services,
            transfer_loads: Vec::new(),
            local: Some(LocalPrepared::prepare(route, seats, routes, false)?),
            network,
            target,
            loads: Vec::new(),
        })
    }
    pub(crate) fn set_reputation(&mut self, revision: u64, scores: &[(String, i64)]) {
        if self.reputation_revision == Some(revision) {
            return;
        }
        if let Some(local) = &mut self.local {
            local.set_reputation(|owner| {
                scores
                    .iter()
                    .find(|(key, _)| key == owner)
                    .map(|(_, score)| *score)
            });
        }
        if let Some(network) = &self.network {
            let n = network.lock().unwrap();
            self.reputation_scores.clear();
            self.reputation_scores.extend(n.offers.iter().map(|o| {
                scores
                    .iter()
                    .find(|(owner, _)| *owner == o.owner)
                    .map_or(o.reputation, |(_, score)| *score)
            }));
        }
        self.quotes.clear();
        self.reputation_revision = Some(revision);
    }
    pub(crate) fn reprice_direct(&mut self, route: &Route) {
        assert!(self.network.is_none());
        self.local.as_mut().unwrap().reprice_direct(route);
    }
    #[cfg(test)]
    pub(crate) fn reprice_single(&mut self, route: &Route) {
        self.quotes.clear();
        if let Some(local) = &mut self.local {
            local.reprice_single(route);
        }
        if let (Some(network), Some(target)) = (&self.network, self.target) {
            let mut offers = network.lock().unwrap().offers.clone();
            offers[target].route.ticket_eur = route.ticket_eur;
            offers[target].route.business_ticket_eur = route.business_ticket_eur;
            // Rebuild with new willingness; never retain a transfer quote at old fares.
            *self = Self::new(route, offers[target].seats, &offers).expect("valid repriced supply");
        }
    }
    pub fn estimate(&mut self, time: Option<u64>) -> (Estimate, &[i64]) {
        self.estimate_inner(time, false)
    }
    fn estimate_display(&mut self, time: Option<u64>) -> (Estimate, &[i64]) {
        self.estimate_inner(time, true)
    }
    fn estimate_inner(&mut self, time: Option<u64>, display: bool) -> (Estimate, &[i64]) {
        if self.network.is_none() {
            return self.local.as_mut().unwrap().estimate(time);
        }

        // Only transfers use a daily season. Direct demand keeps the exact
        // departure time, including when a daily transfer quote is reused.
        let (mut result, local_loads) = self.local.as_mut().unwrap().estimate(time);
        self.loads.clear();
        self.loads.extend_from_slice(local_loads);
        let day = crate::world_economy::transfer_time(
            self.local
                .as_ref()
                .and_then(|l| l.matching.iter().find_map(|o| o.economy)),
            time,
        );
        if self.quotes.len() >= 512 {
            self.quotes.clear();
        }
        let mut transfer = if !display && let Some(cached) = self.quotes.get(&day) {
            self.transfer_loads.clone_from(&cached.1);
            cached.0.clone()
        } else {
            let mut transfer = Estimate::default();
            self.transfer_loads.clear();
            if let (Some(network), Some(target)) = (&self.network, self.target) {
                let mut n = network.lock().unwrap();
                n.estimate_for(day, self.reputation_revision, &self.reputation_scores);
                transfer.connecting = n.passengers[target];
                transfer.connecting_revenue = n.revenue[target];
                transfer.codeshare = n.codeshare_for(target);
                transfer.daily_connecting =
                    transfer.connecting as f64 * frequency(&n.offers[target].route);
                for index in &self.transfer_services {
                    self.transfer_loads
                        .push(n.passengers[index.expect("network service")]);
                }
                if display {
                    for (p, daily) in n.paths.iter().zip(&n.daily) {
                        if *daily > 0.0 && (p.legs[0] == target || p.legs[1] == target) {
                            transfer.connections.push(Connection {
                                partner: n.partner_for(p, target),
                                origin: p.origin.clone(),
                                hub: p.hub.clone(),
                                destination: p.destination.clone(),
                                daily: *daily,
                            });
                        }
                    }
                }
            }
            if !display {
                self.quotes
                    .insert(day, (transfer.clone(), self.transfer_loads.clone()));
            }
            transfer
        };
        // A daily transfer reservation can meet stronger local demand later
        // that day. Reallocate the complete network at the actual departure
        // instant instead of overbooking or discarding one leg of a journey.
        if self
            .loads
            .iter()
            .zip(&self.transfer_loads)
            .zip(&self.service_seats)
            .any(|((&local, &connecting), &seats)| local + connecting > i64::from(seats))
            && let (Some(network), Some(target)) = (&self.network, self.target)
        {
            let mut n = network.lock().unwrap();
            n.estimate_for(day, self.reputation_revision, &self.reputation_scores);
            n.estimate_exact(time);
            transfer.connecting = n.passengers[target];
            transfer.connecting_revenue = n.revenue[target];
            transfer.codeshare = n.codeshare_for(target);
            transfer.daily_connecting =
                transfer.connecting as f64 * frequency(&n.offers[target].route);
            for (load, index) in self.transfer_loads.iter_mut().zip(&self.transfer_services) {
                *load = n.passengers[index.expect("network service")];
            }
            if display {
                transfer.connections.clear();
                for (p, daily) in n.paths.iter().zip(&n.daily) {
                    if *daily > 0.0 && (p.legs[0] == target || p.legs[1] == target) {
                        transfer.connections.push(Connection {
                            partner: n.partner_for(p, target),
                            origin: p.origin.clone(),
                            hub: p.hub.clone(),
                            destination: p.destination.clone(),
                            daily: *daily,
                        });
                    }
                }
            }
        }
        result.codeshare = transfer.codeshare;
        result.connecting = transfer.connecting;
        result.connecting_revenue = transfer.connecting_revenue;
        result.daily_connecting = transfer.daily_connecting;
        result.connections = transfer.connections;
        result.per_departure += result.connecting;
        result.daily_passengers += result.daily_connecting;
        for (load, connecting) in self.loads.iter_mut().zip(&self.transfer_loads) {
            *load += *connecting;
        }
        (result, &self.loads)
    }
}

#[cfg(test)]
thread_local! { pub(crate) static TRANSFER_COUNTS: std::cell::Cell<(u64, u64, u64)> = const { std::cell::Cell::new((0, 0, 0)) }; }
struct TransferPath {
    legs: [usize; 2],
    origin: String,
    hub: String,
    destination: String,
    inputs: MarketInputs,
    frequency: f64,
    quantum: f64,
    fare: [i64; 2],
}
struct TransferMarket {
    endpoints: Route,
    local: Option<LocalPrepared>,
    base: MarketInputs,
    paths: Vec<usize>,
    services: Vec<usize>,
    weights: Vec<[[f64; 2]; 2]>,
    outside_weights: [f64; 2],
}
struct TransferNetwork {
    reputation_factors: Vec<f64>,
    reputation_revision: Option<u64>,
    offers: Vec<Offer>,
    paths: Vec<TransferPath>,
    markets: Vec<TransferMarket>,
    passengers: Vec<i64>,
    revenue: Vec<i64>,
    daily: Vec<f64>,
    last: Option<Option<u64>>,
    recent: std::collections::HashMap<Option<u64>, TransferAllocation>,
    locals: Vec<Estimate>,
}
struct TransferAllocation {
    locals: Vec<Estimate>,
    passengers: Vec<i64>,
    revenue: Vec<i64>,
    daily: Vec<f64>,
}
thread_local! {
    // Derived, bounded and unsaved. Quotes for different owners share the exact
    // same network allocation, independent of supply/airline iteration order.
    static TRANSFERS: std::cell::RefCell<Vec<std::sync::Arc<std::sync::Mutex<TransferNetwork>>>> = const { std::cell::RefCell::new(Vec::new()) };
}
/// Structural connection data survives fare, awareness and reputation changes.
/// Bounded independently of economic snapshots, including networks with no paths.
#[derive(Clone, PartialEq, Eq)]
struct TopologyLeg {
    partners: Vec<String>,
    owner: String,
    id: u32,
    origin: String,
    destination: String,
    interval: u64,
    economy: bool,
}
struct ConnectionGeometry {
    legs: [usize; 2],
    origin: String,
    hub: String,
    destination: String,
    distances: [f64; 2],
    direct: MarketInputs,
    frequency: f64,
    quantum: f64,
    ceilings: std::sync::Mutex<std::collections::BTreeMap<i64, bool>>,
}
impl ConnectionGeometry {
    fn can_fill(&self, offers: &[Offer]) -> bool {
        let fare = offers[self.legs[0]].route.ticket_eur + offers[self.legs[1]].route.ticket_eur;
        let mut ceilings = self.ceilings.lock().unwrap();
        if let Some(result) = ceilings.get(&fare) {
            return *result;
        }
        static MAXIMUM: LazyLock<Market> = LazyLock::new(|| {
            let maximum = |curve: &[f64; 12]| {
                curve.iter().copied().fold(0.0, f64::max) / (curve.iter().sum::<f64>() / 12.0)
            };
            Market {
                business: maximum(&CONFIG.business_monthly),
                leisure: maximum(&CONFIG.leisure_monthly),
            }
        });
        let ceiling = self.direct.with_price(fare).at(&MAXIMUM).total()
            * CONFIG.transfer_penalty
            * self.frequency.min(1.0);
        // A strict maximum-season bound, including the original rounding margin.
        // Reputation and operational progress cannot affect this proof.
        let result = ceiling >= self.quantum * (1.0 - 1e-12);
        ceilings.insert(fare, result);
        result
    }
}
struct TransferTopology {
    legs: Vec<TopologyLeg>,
    paths: Vec<ConnectionGeometry>,
    by_market: std::collections::HashMap<String, std::collections::HashMap<String, Vec<usize>>>,
}
impl TransferTopology {
    fn paths_for(&self, route: &Route) -> &[usize] {
        let (a, b) = if route.origin_icao < route.destination_icao {
            (&route.origin_icao, &route.destination_icao)
        } else {
            (&route.destination_icao, &route.origin_icao)
        };
        self.by_market
            .get(a)
            .and_then(|destinations| destinations.get(b))
            .map_or(&[], Vec::as_slice)
    }
}
thread_local! {
    static TOPOLOGIES: std::cell::RefCell<Vec<std::sync::Arc<TransferTopology>>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn transfer_topology(offers: &[Offer]) -> Result<std::sync::Arc<TransferTopology>, String> {
    let legs: Vec<_> = offers
        .iter()
        .map(|o| TopologyLeg {
            partners: o.codeshare_partners.clone(),
            owner: o.owner.clone(),
            id: o.route.id,
            origin: o.route.origin_icao.clone(),
            destination: o.route.destination_icao.clone(),
            interval: o.route.plan.as_ref().map_or(0, |p| p.interval_seconds),
            economy: o.seats > o.business_seats,
        })
        .collect();
    TOPOLOGIES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(t) = cache.iter().find(|t| t.legs == legs) {
            return Ok(t.clone());
        }
        let mut paths = Vec::new();
        for (i, a) in offers.iter().enumerate() {
            for (j, b) in offers.iter().enumerate().skip(i + 1) {
                if !a.connects_with(b)
                    || same_market(&a.route, &b.route)
                    || !legs[i].economy
                    || !legs[j].economy
                    || legs[i].interval == 0
                    || legs[j].interval == 0
                {
                    continue;
                }
                for (hub, origin) in [
                    (&a.route.origin_icao, &a.route.destination_icao),
                    (&a.route.destination_icao, &a.route.origin_icao),
                ] {
                    let destination = if *hub == b.route.origin_icao {
                        &b.route.destination_icao
                    } else if *hub == b.route.destination_icao {
                        &b.route.origin_icao
                    } else {
                        continue;
                    };
                    let d1 = MarketInputs::new(&a.route, 0)?.km;
                    let d2 = MarketInputs::new(&b.route, 0)?.km;
                    let mut endpoints = a.route.clone();
                    endpoints.origin_icao = origin.clone();
                    endpoints.destination_icao = destination.clone();
                    let direct = MarketInputs::new(&endpoints, 0)?;
                    if d1.min(d2) < CONFIG.transfer_min_leg_km
                        || (d1 + d2) / direct.km > CONFIG.transfer_max_detour
                    {
                        continue;
                    }
                    let (mut x, mut y) = (legs[i].interval, legs[j].interval);
                    while y != 0 {
                        (x, y) = (y, x % y);
                    }
                    paths.push(ConnectionGeometry {
                        legs: [i, j],
                        origin: origin.clone(),
                        hub: hub.clone(),
                        destination: destination.clone(),
                        distances: [d1, d2],
                        direct,
                        frequency: frequency(&a.route).min(frequency(&b.route)),
                        quantum: 43200.0 / x as f64,
                        ceilings: Default::default(),
                    });
                }
            }
        }
        let mut by_market: std::collections::HashMap<
            String,
            std::collections::HashMap<String, Vec<usize>>,
        > = Default::default();
        for (p, path) in paths.iter().enumerate() {
            for leg in path.legs {
                let route = &offers[leg].route;
                let (a, b) = if route.origin_icao < route.destination_icao {
                    (&route.origin_icao, &route.destination_icao)
                } else {
                    (&route.destination_icao, &route.origin_icao)
                };
                by_market
                    .entry(a.clone())
                    .or_default()
                    .entry(b.clone())
                    .or_default()
                    .push(p);
            }
        }
        let topology = std::sync::Arc::new(TransferTopology {
            legs,
            paths,
            by_market,
        });
        if cache.len() == 128 {
            cache.remove(0);
        }
        cache.push(topology.clone());
        Ok(topology)
    })
}

pub(crate) type SharedTransferSupply = std::sync::Arc<std::sync::Mutex<TransferSupply>>;
thread_local! {
    /// A memo of the latest event snapshot. Old snapshots stay alive through
    /// their owners; replacing this handle cannot evict a retained quote.
    static DISPATCH_SUPPLY: std::cell::RefCell<Option<(u64, SharedTransferSupply)>> = const { std::cell::RefCell::new(None) };
}
fn dispatch_transfer_supply(
    revision: u64,
    routes: &[Offer],
) -> Result<SharedTransferSupply, String> {
    DISPATCH_SUPPLY.with(|memo| {
        let mut memo = memo.borrow_mut();
        if let Some((key, supply)) = &*memo
            && *key == revision
        {
            return Ok(supply.clone());
        }
        let supply = std::sync::Arc::new(std::sync::Mutex::new(TransferSupply::new(routes)?));
        *memo = Some((revision, supply.clone()));
        Ok(supply)
    })
}

/// Canonical supply and network are prepared once per commercial epoch, shared
/// by all route markets of a company, including markets with zero transfers.
#[derive(Clone)]
pub(crate) struct TransferSupply {
    offers: std::sync::Arc<Vec<Offer>>,
    topology: std::sync::Arc<TransferTopology>,
    network: Option<std::sync::Arc<std::sync::Mutex<TransferNetwork>>>,
}
impl TransferSupply {
    fn new(routes: &[Offer]) -> Result<Self, String> {
        let mut offers: Vec<_> = routes
            .iter()
            .filter(|o| o.route.plan.is_some())
            .map(Offer::commercial_clone)
            .collect();
        offers.sort_by(|a, b| {
            a.owner
                .cmp(&b.owner)
                .then(a.route.id.cmp(&b.route.id))
                .then(a.route.origin_icao.cmp(&b.route.origin_icao))
                .then(a.route.destination_icao.cmp(&b.route.destination_icao))
                .then(a.route.ticket_eur.cmp(&b.route.ticket_eur))
                .then(a.seats.cmp(&b.seats))
        });
        let topology = transfer_topology(&offers)?;
        Ok(Self {
            offers: std::sync::Arc::new(offers),
            topology,
            network: None,
        })
    }
    fn network_for(
        &mut self,
        route: &Route,
    ) -> Result<Option<std::sync::Arc<std::sync::Mutex<TransferNetwork>>>, String> {
        let offers = &*self.offers;
        let topology = &self.topology;
        // Departures and cargo quotes only need networks touching their own market.
        if !topology
            .paths_for(route)
            .iter()
            .any(|&p| topology.paths[p].can_fill(offers))
        {
            return Ok(None);
        }
        if let Some(network) = &self.network {
            return Ok(Some(network.clone()));
        }
        let network = TRANSFERS.with(|cache| -> Result<_, String> {
            let mut cache = cache.borrow_mut();
            if let Some(n) = cache.iter().find(|n| {
                let n = n.lock().unwrap();
                n.offers == *offers
                    && n.reputation_factors.iter().zip(offers).all(|(factor, o)| {
                        *factor == crate::reputation::attractiveness(o.reputation)
                    })
            }) {
                return Ok(Some(n.clone()));
            }
            let network = TransferNetwork::new(offers.clone())?;
            if network.paths.is_empty() {
                return Ok(None);
            }
            let n = std::sync::Arc::new(std::sync::Mutex::new(network));
            if cache.len() == 4 {
                cache.remove(0);
            }
            cache.push(n.clone());
            Ok(Some(n))
        })?;
        self.network = network.clone();
        Ok(network)
    }
}
impl TransferNetwork {
    fn partner_for(&self, path: &TransferPath, target: usize) -> String {
        let other = if path.legs[0] == target {
            path.legs[1]
        } else {
            path.legs[0]
        };
        if self.offers[other].owner == self.offers[target].owner {
            String::new()
        } else {
            self.offers[other].owner.clone()
        }
    }
    fn codeshare_for(&self, target: usize) -> Vec<crate::codeshare::Receipt> {
        let mut receipts = Vec::new();
        if self.offers[target].codeshare_partners.is_empty() {
            return receipts;
        }
        for (path, daily) in self.paths.iter().zip(&self.daily) {
            if !path.legs.contains(&target) || *daily == 0.0 {
                continue;
            }
            let partner = self.partner_for(path, target);
            if partner.is_empty() {
                continue;
            }
            let leg = usize::from(path.legs[1] == target);
            let units = (daily / path.quantum).round();
            let passengers =
                (units * (path.quantum / frequency(&self.offers[target].route)).round()) as i64;
            crate::codeshare::add_receipt(
                &mut receipts,
                &crate::codeshare::Receipt {
                    partner,
                    passengers,
                    revenue: passengers * path.fare[leg],
                },
                1,
            )
            .expect("bounded quote");
        }
        receipts
    }

    fn new(offers: Vec<Offer>) -> Result<Self, String> {
        #[cfg(test)]
        TRANSFER_COUNTS.with(|c| {
            let (n, a, timed) = c.get();
            c.set((n + 1, a, timed));
        });
        let reputation_factors: Vec<_> = offers
            .iter()
            .map(|o| crate::reputation::attractiveness(o.reputation))
            .collect();
        let topology = transfer_topology(&offers)?;
        let mut paths = Vec::new();
        for geometry in &topology.paths {
            let [i, j] = geometry.legs;
            let a = &offers[i];
            let b = &offers[j];
            let [d1, d2] = geometry.distances;
            let total_fare = a.route.ticket_eur + b.route.ticket_eur;
            let inputs = geometry.direct.with_price(total_fare);
            if inputs.business == 0.0 && inputs.leisure == 0.0 {
                continue;
            }
            let first_fare = (total_fare as f64 * d1 / (d1 + d2)).floor() as i64;
            paths.push(TransferPath {
                legs: [i, j],
                origin: geometry.origin.clone(),
                hub: geometry.hub.clone(),
                destination: geometry.destination.clone(),
                inputs,
                frequency: geometry.frequency,
                quantum: geometry.quantum,
                fare: [first_fare, total_fare - first_fare],
            });
        }
        if paths.is_empty() {
            return Ok(Self {
                reputation_factors,
                reputation_revision: None,
                offers,
                paths,
                markets: Vec::new(),
                passengers: Vec::new(),
                revenue: Vec::new(),
                daily: Vec::new(),
                last: None,
                recent: std::collections::HashMap::new(),
                locals: Vec::new(),
            });
        }
        let mut markets: Vec<TransferMarket> = Vec::new();
        for (i, p) in paths.iter().enumerate() {
            let mut endpoints = offers[p.legs[0]].route.clone();
            endpoints.origin_icao = p.origin.clone();
            endpoints.destination_icao = p.destination.clone();
            if let Some(m) = markets
                .iter_mut()
                .find(|m| same_market(&m.endpoints, &endpoints))
            {
                m.paths.push(i);
            } else {
                let local = offers
                    .iter()
                    .find(|o| same_market(&o.route, &endpoints))
                    .map(|o| LocalPrepared::new(&o.route, o.seats, &offers))
                    .transpose()?;
                markets.push(TransferMarket {
                    base: MarketInputs::new(&endpoints, 0)?,
                    endpoints,
                    local,
                    paths: vec![i],
                    services: Vec::new(),
                    weights: Vec::new(),
                    outside_weights: [1.0; 2],
                });
            }
        }
        // Also prepare leg markets to reserve their local economy passengers.
        for (_, o) in offers
            .iter()
            .enumerate()
            .filter(|(i, _)| paths.iter().any(|p| p.legs.contains(i)))
        {
            if !markets.iter().any(|m| same_market(&m.endpoints, &o.route)) {
                markets.push(TransferMarket {
                    endpoints: o.route.clone(),
                    base: MarketInputs::new(&o.route, 0)?,
                    local: Some(LocalPrepared::new(&o.route, o.seats, &offers)?),
                    paths: Vec::new(),
                    services: Vec::new(),
                    weights: Vec::new(),
                    outside_weights: [1.0; 2],
                });
            }
        }
        for m in &mut markets {
            if let Some(local) = &mut m.local {
                m.services = local
                    .services
                    .iter()
                    .map(|key| {
                        offers
                            .iter()
                            .position(|o| o.owner == key.owner && same_offer(&o.route, &key.route))
                            .expect("network local service")
                    })
                    .collect();
            }
            let mut frequencies = [0.0_f64; 2];
            for &p in &m.paths {
                let path = &paths[p];
                let mut weights = [[0.0; 2]; 2];
                for (segment, willingness) in [path.inputs.business, path.inputs.leisure]
                    .into_iter()
                    .enumerate()
                {
                    let w = willingness * CONFIG.transfer_penalty;
                    let weight = crate::loyalty::path_weight(
                        [
                            offers[path.legs[0]].loyalty_members,
                            offers[path.legs[1]].loyalty_members,
                        ],
                        offers[path.legs[0]].route.ancillary_policy.weight(
                            offers[path.legs[1]]
                                .route
                                .ancillary_policy
                                .weight(path.frequency * w / (1.0 - w).max(1e-9)),
                        ),
                    );
                    weights[segment] = [w, weight];
                    if w > 0.0 {
                        frequencies[segment] += path.frequency;
                    }
                }
                m.weights.push(weights);
            }
            for s in 0..2 {
                m.outside_weights[s] = frequencies[s].max(1.0);
            }
        }
        let count = offers.len();
        let path_count = paths.len();
        Ok(Self {
            reputation_revision: None,
            locals: vec![Estimate::default(); offers.len()],
            reputation_factors,
            offers,
            paths,
            markets,
            passengers: vec![0; count],
            revenue: vec![0; count],
            daily: vec![0.0; path_count],
            last: None,
            recent: std::collections::HashMap::new(),
        })
    }
    fn estimate_for(&mut self, time: Option<u64>, revision: Option<u64>, scores: &[i64]) {
        let changed = if let Some(revision) = revision {
            self.reputation_revision != Some(revision)
        } else {
            self.reputation_revision.is_some()
                || self
                    .offers
                    .iter()
                    .zip(scores)
                    .enumerate()
                    .any(|(i, (o, score))| {
                        o.reputation != *score
                            || self.reputation_factors[i]
                                != crate::reputation::attractiveness(*score)
                    })
        };
        if changed {
            for ((offer, factor), score) in self
                .offers
                .iter_mut()
                .zip(&mut self.reputation_factors)
                .zip(scores)
            {
                offer.reputation = *score;
                *factor = crate::reputation::attractiveness(*score);
            }
            let offers = &self.offers;
            for market in &mut self.markets {
                if let Some(local) = &mut market.local {
                    local.set_reputation(|owner| {
                        offers
                            .iter()
                            .find(|o| o.owner == owner)
                            .map(|o| o.reputation)
                    });
                }
            }
            self.last = None;
            self.recent.clear();
            self.reputation_revision = revision;
        }
        self.estimate(time);
    }

    fn estimate(&mut self, time: Option<u64>) {
        let time =
            crate::world_economy::transfer_time(self.offers.iter().find_map(|o| o.economy), time);
        self.estimate_exact(time);
    }

    fn estimate_exact(&mut self, time: Option<u64>) {
        if self.recent.len() >= 512 {
            self.recent.clear();
        }
        if self.last == Some(time) {
            return;
        }
        self.last = Some(time);
        if let Some(cached) = self.recent.get(&time) {
            self.locals.clone_from(&cached.locals);
            self.passengers.copy_from_slice(&cached.passengers);
            self.revenue.copy_from_slice(&cached.revenue);
            self.daily.copy_from_slice(&cached.daily);
            return;
        }
        #[cfg(test)]
        TRANSFER_COUNTS.with(|c| {
            let (n, a, timed) = c.get();
            c.set((n, a + 1, timed + u64::from(time.is_some())));
        });
        self.passengers.fill(0);
        self.revenue.fill(0);
        self.daily.fill(0.0);
        let economy = self.offers.iter().find_map(|o| o.economy);
        let season = crate::world_economy::factors(economy, time);
        let mut spare: Vec<f64> = self
            .offers
            .iter()
            .map(|o| frequency(&o.route) * f64::from(o.seats - o.business_seats))
            .collect();
        for m in &mut self.markets {
            let base = if economy.is_some() && time.is_some() {
                m.base.at(&crate::world_economy::with_events(
                    Market {
                        business: season.business,
                        leisure: season.leisure,
                    },
                    economy,
                    time,
                    &m.endpoints.origin_icao,
                    &m.endpoints.destination_icao,
                ))
            } else {
                m.base.at(&season)
            };
            let mut residual = [base.business, base.leisure];
            if let Some(local) = &mut m.local {
                local.estimate(time);
                for (service, &index) in m.services.iter().enumerate() {
                    self.locals[index] = local.for_service(service);
                }
                for (j, o) in local.matching.iter().enumerate() {
                    for (s, r) in residual.iter_mut().enumerate() {
                        *r = (*r - local.allocations[j].allocated[s]).max(0.0);
                    }
                    if !o.premium {
                        let k = m.services[local.service_indices[j]];
                        spare[k] -= rounded_passengers(
                            local.allocations[j].allocated[0] + local.allocations[j].allocated[1],
                            o,
                        ) as f64
                            * frequency(&o.route);
                    }
                }
            }
            for (s, remaining) in residual.into_iter().enumerate() {
                // One airline factor for the whole journey, as for a direct offer.
                let denominator = m.outside_weights[s]
                    + m.paths
                        .iter()
                        .zip(&m.weights)
                        .map(|(&p, w)| {
                            w[s][1]
                                * self.reputation_factors[self.paths[p].legs[0]]
                                    .min(self.reputation_factors[self.paths[p].legs[1]])
                        })
                        .sum::<f64>();
                for (&p, weights) in m.paths.iter().zip(&m.weights) {
                    let [w, weight] = weights[s];
                    let weight = weight
                        * self.reputation_factors[self.paths[p].legs[0]]
                            .min(self.reputation_factors[self.paths[p].legs[1]]);
                    self.daily[p] += (remaining * weight / denominator)
                        .min(remaining * w * self.paths[p].frequency.min(1.0));
                }
            }
        }
        let mut requested = vec![0.0; self.offers.len()];
        for (p, n) in self.paths.iter().zip(&self.daily) {
            for leg in p.legs {
                requested[leg] += n;
            }
        }
        for (i, p) in self.paths.iter().enumerate() {
            let scale = p
                .legs
                .iter()
                .map(|&k| {
                    if requested[k] > 0.0 {
                        (spare[k] / requested[k]).clamp(0.0, 1.0)
                    } else {
                        0.0
                    }
                })
                .fold(1.0, f64::min);
            // Common rate quantum gives identical daily travellers/revenue on
            // both legs even with unequal intervals. No rounding creates demand.
            let units = (self.daily[i] * scale / p.quantum).floor();
            self.daily[i] = units * p.quantum;
            for (j, k) in p.legs.into_iter().enumerate() {
                let count = (units * (p.quantum / frequency(&self.offers[k].route)).round()) as i64;
                self.passengers[k] += count;
                self.revenue[k] += count * p.fare[j];
            }
        }
        // Sharing a snapshot cannot evict another route's quote. Seasonal
        // keys also reuse identical departures in later model years.
        self.recent.insert(
            time,
            TransferAllocation {
                locals: self.locals.clone(),
                passengers: self.passengers.clone(),
                revenue: self.revenue.clone(),
                daily: self.daily.clone(),
            },
        );
    }
}

// Pre-optimization allocator retained only as an exact regression oracle.
#[cfg(test)]
fn estimate_reference(
    route: &Route,
    seats: u32,
    routes: &[Offer],
    time: Option<u64>,
    loads: Option<&mut Vec<i64>>,
) -> Result<Estimate, String> {
    let mut matching: Vec<_> = routes
        .iter()
        .filter(|o| o.route.plan.is_some() && same_market(route, &o.route))
        .cloned()
        .collect();
    let mut own = routes
        .iter()
        .find(|o| same_offer(route, &o.route))
        .cloned()
        .unwrap_or_else(|| Offer::new(route, seats));
    own.seats = seats;
    if own.business_seats > seats {
        return Err("Ungültige Kabinenkapazität.".into());
    }
    if !matching
        .iter()
        .any(|o| o.seats == seats && same_offer(route, &o.route))
    {
        if let Some(o) = matching.iter_mut().find(|o| same_offer(route, &o.route)) {
            *o = own;
        } else {
            matching.push(own);
        }
    }
    // Both cabins participate in this same segmented allocator and spill loop.
    // Premium has no leisure weight; unsatisfied business can spill to economy.
    let mut own_indices = Vec::new();
    let service_count = matching.len();
    let mut service_indices = Vec::new();
    let mut cabins = Vec::new();
    let own_index = matching
        .iter()
        .position(|o| o.seats == seats && same_offer(route, &o.route))
        .unwrap();
    for (index, o) in matching.into_iter().enumerate() {
        let is_own = index == own_index;
        let mut economy = o.clone();
        economy.seats -= o.business_seats;
        if is_own {
            own_indices.push(cabins.len());
        }
        service_indices.push(index);
        cabins.push(economy);
        if o.business_seats > 0 {
            let mut business = o;
            business.seats = business.business_seats;
            business.route.ticket_eur = business.route.business_ticket_eur;
            business.premium = true;
            if is_own {
                own_indices.push(cabins.len());
            }
            service_indices.push(index);
            cabins.push(business);
        }
    }
    // Preserve identity independently of equal prices/IDs across owners.
    let mut matching: Vec<_> = cabins
        .into_iter()
        .enumerate()
        .map(|(i, o)| (o, own_indices.contains(&i), service_indices[i]))
        .collect();
    // Economic keys give a canonical floating-point reduction order even when
    // owner order, route IDs or direction differ. Equal keys are interchangeable.
    matching.sort_by(|(a, _, _), (b, _, _)| {
        a.route
            .ticket_eur
            .cmp(&b.route.ticket_eur)
            .then_with(|| frequency(&a.route).total_cmp(&frequency(&b.route)))
            .then(a.seats.cmp(&b.seats))
            .then(a.premium.cmp(&b.premium))
            .then(a.route.service_level.cmp(&b.route.service_level))
            .then(a.route.ancillary_policy.cmp(&b.route.ancillary_policy))
            .then(a.loyalty_members.cmp(&b.loyalty_members))
            .then(a.awareness.cmp(&b.awareness))
            .then(a.alliance_bonus_bps.cmp(&b.alliance_bonus_bps))
            .then(a.reputation.cmp(&b.reputation))
    });
    let own_indices: Vec<_> = matching
        .iter()
        .enumerate()
        .filter_map(|(i, (_, own, _))| own.then_some(i))
        .collect();
    let service_indices: Vec<_> = matching.iter().map(|(_, _, index)| *index).collect();
    let matching: Vec<_> = matching.into_iter().map(|(o, _, _)| o).collect();
    let season = crate::world_economy::route_factors(
        matching.iter().find_map(|o| o.economy),
        time,
        &route.origin_icao,
        &route.destination_icao,
    );
    let at = |route: &Route, price| -> Result<Market, String> {
        Ok(MarketInputs::new(route, price)?.at(&season))
    };
    let base = at(route, 0)?;
    let mut weights = vec![[0.0; 2]; matching.len()];
    let mut limits = weights.clone();
    let mut frequency_sum = [0.0_f64; 2];
    let mut weight_sum = [0.0_f64; 2];
    let mut capacities = Vec::new();
    for (j, o) in matching.iter().enumerate() {
        let f = frequency(&o.route);
        capacities.push(f * f64::from(o.seats));
        let mut m = at(
            &o.route,
            if o.premium {
                (o.route.ticket_eur as f64 / crate::cabins::RULES.business_price_multiplier).ceil()
                    as i64
            } else {
                o.route.ticket_eur
            },
        )?;
        if o.seats == 0 {
            m.business = 0.0;
            m.leisure = 0.0;
        }
        if o.premium {
            m.leisure = 0.0;
        }
        for (i, (n, b)) in [(m.business, base.business), (m.leisure, base.leisure)]
            .into_iter()
            .enumerate()
        {
            let w = if b > 0.0 { n / b } else { 0.0 };
            if w > 0.0 && !o.premium {
                frequency_sum[i] += f;
            }
            weights[j][i] = crate::loyalty::weight(
                o.loyalty_members,
                o.route.ancillary_policy.weight(
                    crate::marketing::attractiveness(o.awareness)
                        * crate::reputation::attractiveness(o.reputation)
                        * o.route.service_level.rate().attractiveness
                        * o.alliance_choice()
                        * f,
                ),
            ) * w
                / (1.0 - w).max(1e-9)
                * if o.premium {
                    crate::cabins::RULES.business_preference
                } else {
                    1.0
                };
            weight_sum[i] += weights[j][i];
            // Rare flights cannot bank days of willing demand.
            limits[j][i] = n * f.min(1.0);
        }
    }
    let mut allocated = weights.clone();
    for j in 0..matching.len() {
        for (i, b) in [base.business, base.leisure].into_iter().enumerate() {
            allocated[j][i] =
                (b * weights[j][i] / (frequency_sum[i].max(1.0) + weight_sum[i])).min(limits[j][i]);
        }
    }
    let requested = own_indices
        .iter()
        .map(|&j| allocated[j].iter().sum::<f64>())
        .sum::<f64>();
    let mut spill = [0.0; 2];
    for j in 0..matching.len() {
        let total = allocated[j].iter().sum::<f64>();
        let scale = if total > capacities[j] {
            capacities[j] / total
        } else {
            1.0
        };
        for (i, excess) in spill.iter_mut().enumerate() {
            *excess += allocated[j][i] * (1.0 - scale);
            allocated[j][i] *= scale;
        }
    }
    // Each non-final round saturates at least one seat/segment ceiling.
    for _ in 0..matching.len() * 3 + 1 {
        let mut sums = [0.0; 2];
        for j in 0..matching.len() {
            if capacities[j] - allocated[j].iter().sum::<f64>() > 1e-9 {
                for (i, sum) in sums.iter_mut().enumerate() {
                    if limits[j][i] - allocated[j][i] > 1e-9 {
                        *sum += weights[j][i];
                    }
                }
            }
        }
        let mut accepted = [0.0; 2];
        for j in 0..matching.len() {
            let spare = (capacities[j] - allocated[j].iter().sum::<f64>()).max(0.0);
            if spare <= 1e-9 {
                continue;
            }
            let mut add = [0.0; 2];
            for i in 0..2 {
                if sums[i] > 0.0 && limits[j][i] - allocated[j][i] > 1e-9 {
                    add[i] =
                        (spill[i] * weights[j][i] / sums[i]).min(limits[j][i] - allocated[j][i]);
                }
            }
            let total = add.iter().sum::<f64>();
            let scale = if total > spare { spare / total } else { 1.0 };
            for i in 0..2 {
                let n = add[i] * scale;
                allocated[j][i] += n;
                accepted[i] += n;
            }
        }
        if accepted.iter().sum::<f64>() <= 1e-9 {
            break;
        }
        for i in 0..2 {
            spill[i] = (spill[i] - accepted[i]).max(0.0);
        }
    }
    let daily = own_indices
        .iter()
        .map(|&j| allocated[j].iter().sum::<f64>())
        .sum::<f64>();
    let mut business = 0;
    let mut per_departure = 0;
    for &j in &own_indices {
        let n = rounded_passengers(allocated[j].iter().sum(), &matching[j]);
        per_departure += n;
        if matching[j].premium {
            business += n;
        }
    }
    if let Some(loads) = loads {
        loads.resize(service_count, 0);
        for (j, offer) in matching.iter().enumerate() {
            loads[service_indices[j]] += rounded_passengers(allocated[j].iter().sum(), offer);
        }
    }
    Ok(Estimate {
        business,
        daily_passengers: per_departure as f64 * frequency(route),
        daily_spill: (requested - daily).max(0.0),
        per_departure,
        ..Estimate::default()
    })
}

#[cfg(test)]
pub fn passengers(route: &Route, routes: &[Offer]) -> Result<i64, String> {
    let seats = routes
        .iter()
        .find(|o| same_offer(route, &o.route))
        .ok_or("Angebot ohne Sitzkapazität.")?
        .seats;
    Ok(estimate(route, seats, routes)?.per_departure)
}

#[inline(always)]
pub fn same_market(a: &Route, b: &Route) -> bool {
    (a.origin_icao == b.origin_icao && a.destination_icao == b.destination_icao)
        || (a.origin_icao == b.destination_icao && a.destination_icao == b.origin_icao)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{economy, game::Game, operations};

    fn game(count: u32, interval: u32) -> Game {
        let mut g = Game::default();
        g.found("Daily Market", "DE", "EDDF").unwrap();
        g.competitors.clear(); // Isolated market fixture; multi-company coverage is in competition.rs.
        for id in 1..=count {
            g.buy_aircraft("atr72_600").unwrap();
            g.staff_up();
            g.create_route("EDDF", "EDDM", id).unwrap();
            g.set_ticket_price(id, 100).unwrap();
            g.schedule(id, 1, interval).unwrap();
        }
        g.clock.set_running(true);
        g
    }

    #[test]
    fn reputation_refresh_matches_fresh_allocation_with_stable_and_changed_order() {
        let mut offers = vec![transfer_offers()[0].clone(); 2];
        offers[0].owner = "first".into();
        offers[1].owner = "second".into();
        offers[1].route.id += 100;
        let route = offers[0].route.clone();
        let seats = offers[0].seats;
        let mut retained = LocalPrepared::new(&route, seats, &offers).unwrap();
        for scores in [
            [400000, 600000],
            [450000, 650000],
            [800000, 650000],
            [650000, 650000],
            [400000, 600000],
        ] {
            for (offer, score) in offers.iter_mut().zip(scores) {
                offer.reputation = score;
            }
            retained.set_reputation(|owner| {
                offers
                    .iter()
                    .find(|o| o.owner == owner)
                    .map(|o| o.reputation)
            });
            let mut fresh = LocalPrepared::new(&route, seats, &offers).unwrap();
            for time in [None, Some(123), Some(2345678)] {
                let (actual, loads) = retained.estimate(time);
                let (expected, expected_loads) = fresh.estimate(time);
                assert_eq!(actual, expected);
                assert_eq!(loads, expected_loads);
            }
        }
    }

    #[test]
    fn direct_fare_sweep_matches_fresh_quotes_with_competing_cabins() {
        let mut supply = transfer_offers();
        supply.truncate(1);
        supply[0].business_seats = 20;
        supply[0].route.business_ticket_eur =
            crate::cabins::premium_fare(supply[0].route.ticket_eur);
        let mut rival = supply[0].clone();
        rival.owner = "rival".into();
        rival.route.id += 99;
        rival.seats += 30;
        rival.awareness = 5_000_000;
        rival.reputation = -250_000;
        supply.push(rival);
        let mut prepared = LocalPrepared::new(&supply[0].route, supply[0].seats, &supply).unwrap();
        for fare in [300, 0, 200, 100, 0, 100_000, 300] {
            supply[0].route.ticket_eur = fare;
            supply[0].route.business_ticket_eur = crate::cabins::premium_fare(fare);
            prepared.reprice_direct(&supply[0].route);
            let mut fresh = LocalPrepared::new(&supply[0].route, supply[0].seats, &supply).unwrap();
            for time in [None, Some(60), Some(200 * 86400 + 123)] {
                assert_eq!(
                    prepared.estimate(time),
                    fresh.estimate(time),
                    "fare={fare} time={time:?}"
                );
                for (service, offer) in supply.iter().enumerate() {
                    let mut individual =
                        LocalPrepared::new(&offer.route, offer.seats, &supply).unwrap();
                    assert_eq!(
                        prepared.for_service(service),
                        individual.estimate(time).0,
                        "fare={fare} service={service}"
                    );
                }
            }
        }
    }

    fn transfer_offers() -> Vec<Offer> {
        let g = game(1, 720);
        let template = &g.routes[0];
        [
            ("EDDM", "EDDF", "hub"),
            ("EDDF", "KJFK", "hub"),
            ("EDDM", "KJFK", "rival"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (a, b, owner))| {
            let mut r = template.clone();
            r.id = (i + 1) as u32;
            r.origin_icao = a.into();
            r.destination_icao = b.into();
            r.ticket_eur = if i == 0 { 100 } else { 200 };
            let mut o = Offer::new(&r, 1000);
            o.owner = owner.into();
            o
        })
        .collect()
    }

    #[test]
    fn transfer_daily_cache_preserves_exact_local_quotes_and_day_boundaries() {
        let offers = transfer_offers();
        let mut stored = Prepared::new(&offers[0].route, offers[0].seats, &offers).unwrap();
        let mut direct = Prepared::direct(&offers[0].route, offers[0].seats, &offers).unwrap();
        let mut local_changes = 0;
        for day in 0..365 {
            let start = day * 86400;
            let morning = direct.estimate(Some(start)).0;
            let first = stored.estimate(Some(start)).0;
            for offset in [1, 43200, 86399] {
                let time = Some(start + offset);
                let local = direct.estimate(time).0;
                let quote = stored.estimate(time).0;
                assert_eq!(quote.per_departure - quote.connecting, local.per_departure);
                assert_eq!(quote.business, local.business);
                assert_eq!(quote.daily_spill, local.daily_spill);
                assert_eq!(
                    quote.daily_passengers - quote.daily_connecting,
                    local.daily_passengers
                );
                assert_eq!(quote.connecting, first.connecting);
                assert_eq!(quote.connecting_revenue, first.connecting_revenue);
                assert_eq!(quote.daily_connecting, first.daily_connecting);
                local_changes += usize::from(local != morning);
            }
        }
        assert!(
            local_changes > 0,
            "fixture must expose intraday local season changes"
        );
        assert_eq!(stored.quotes.len(), 365);
        let at_start = stored.estimate(Some(0)).0;
        assert_eq!(stored.estimate(Some(YEAR_SECONDS)).0, at_start);
        assert_eq!(stored.quotes.len(), 365);
        let network = stored.network.as_ref().unwrap().lock().unwrap();
        assert_eq!(network.recent.len(), 365);
    }

    #[test]
    fn transfer_zero_ceiling_and_stored_quotes_match_fresh_after_cache_churn() {
        for interval in [7200, 7260, 14400, 14520, 86400] {
            let mut offers = transfer_offers();
            offers[1].route.plan.as_mut().unwrap().interval_seconds = interval;
            let mut stored = Prepared::new(&offers[0].route, offers[0].seats, &offers).unwrap();
            for month in 0..24 {
                let time = Some(month * YEAR_SECONDS / 12 + 123);
                let actual = stored.estimate(time).0;
                // Bypass both the eligibility shortcut and shared network cache.
                let mut fresh = Prepared::new(&offers[0].route, offers[0].seats, &offers).unwrap();
                let network = TransferNetwork::new(offers.clone()).unwrap();
                fresh.target = Some(0);
                fresh.network = Some(std::sync::Arc::new(std::sync::Mutex::new(network)));
                let expected = fresh.estimate(time).0;
                assert_eq!(actual, expected, "interval={interval}, month={month}");
            }
            let before = stored.estimate(Some(123)).0;
            // More than the old 128-entry topology limit cannot change a quote.
            for id in 1000..1140 {
                let mut unrelated = offers.clone();
                unrelated[0].route.id = id;
                transfer_topology(&unrelated).unwrap();
            }
            assert_eq!(stored.estimate(Some(YEAR_SECONDS + 123)).0, before);
            let mut fresh = Prepared::new(&offers[0].route, offers[0].seats, &offers).unwrap();
            assert_eq!(fresh.estimate(Some(123)).0, before);
        }
    }

    #[test]
    fn transfer_topology_survives_quotes_and_invalidates_structural_changes() {
        let offers = transfer_offers();
        let topology = transfer_topology(&offers).unwrap();
        assert_eq!(topology.paths.len(), 1);
        assert_eq!(topology.paths_for(&offers[0].route), &[0]);
        assert_eq!(topology.paths_for(&offers[1].route), &[0]);
        assert!(topology.paths_for(&offers[2].route).is_empty());
        let mut reversed = offers[0].route.clone();
        std::mem::swap(&mut reversed.origin_icao, &mut reversed.destination_icao);
        assert_eq!(topology.paths_for(&reversed), &[0]);
        let mut changed = offers.clone();
        changed[0].route.ticket_eur += 10;
        changed[0].reputation = 1_000_000;
        changed[0].awareness = 100;
        changed[0].route.plan.as_mut().unwrap().completed_flights = 20;
        assert!(std::sync::Arc::ptr_eq(
            &topology,
            &transfer_topology(&changed).unwrap()
        ));
        changed[0].route.plan.as_mut().unwrap().interval_seconds *= 2;
        assert!(!std::sync::Arc::ptr_eq(
            &topology,
            &transfer_topology(&changed).unwrap()
        ));
        for case in 0..4 {
            let mut changed = offers.clone();
            match case {
                0 => changed[1].route.plan = None,
                1 => changed[1].owner = "different airline".into(),
                2 => changed[1].business_seats = changed[1].seats,
                _ => changed[1].route.origin_icao = "KLAX".into(),
            }
            assert!(
                transfer_topology(&changed).unwrap().paths.is_empty(),
                "case={case}"
            );
        }
    }

    #[test]
    fn transfer_reputation_is_one_gentle_journey_factor_and_cache_respects_neutral_choice() {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                crate::reputation::NEUTRAL_CHOICE.with(|v| v.set(false));
            }
        }
        let _reset = Reset;
        let mut offers = transfer_offers();
        offers.truncate(2);
        offers[0].route.origin_icao = "EGLL".into();
        offers[1].route.destination_icao = "LTFM".into();
        offers[1].route.ticket_eur = 100;
        for o in &mut offers {
            o.seats = 100_000;
            o.reputation = crate::reputation::SCALE;
        }
        let mut rival = offers.clone();
        for o in &mut rival {
            o.owner = "another hub airline".into();
            o.reputation = 0;
        }
        offers.extend(rival);
        let mut od = offers[0].route.clone();
        od.destination_icao = "LTFM".into();
        let base = market(&od, 0).unwrap();
        // Two uncapped paths share this O&D. Select a fare where integer
        // rounding distinguishes a single factor from a squared factor.
        let expected = |fare, factor| {
            let willing = market(&od, fare).unwrap();
            [
                (base.business, willing.business),
                (base.leisure, willing.leisure),
            ]
            .iter()
            .map(|(b, n)| {
                let w = n / b * CONFIG.transfer_penalty;
                let weight = w / (1.0 - w);
                b * weight * factor / (2.0 + weight + weight * factor)
            })
            .sum::<f64>()
            .floor() as i64
        };
        let fare = (1..=200)
            .find(|fare| expected(fare + 100, 1.03) != expected(fare + 100, 1.03 * 1.03))
            .expect("fixture must detect a squared factor");
        offers[0].route.ticket_eur = fare;
        offers[2].route.ticket_eur = fare;
        let positive = estimate(&offers[0].route, offers[0].seats, &offers)
            .unwrap()
            .connecting;
        assert_eq!(positive, expected(fare + 100, 1.03));
        crate::reputation::NEUTRAL_CHOICE.with(|v| v.set(true));
        let neutral = estimate(&offers[0].route, offers[0].seats, &offers)
            .unwrap()
            .connecting;
        let mut fresh = TransferNetwork::new(offers.clone()).unwrap();
        fresh.estimate(None);
        assert_eq!(neutral, fresh.passengers[0]);
        assert_eq!(neutral, expected(fare + 100, 1.0));
        crate::reputation::NEUTRAL_CHOICE.with(|v| v.set(false));
        assert_eq!(
            estimate(&offers[0].route, offers[0].seats, &offers)
                .unwrap()
                .connecting,
            positive
        );
    }

    #[test]
    fn codeshare_mixed_allocation_capacity_rounding_fares_and_lower_reputation() {
        let same_owner = transfer_offers();
        let mut mixed = same_owner.clone();
        mixed[1].owner = "partner".into();
        assert_eq!(
            estimate(&mixed[0].route, mixed[0].seats, &mixed)
                .unwrap()
                .connecting,
            0
        );
        mixed[0].codeshare_partners = vec!["partner".into()];
        assert_eq!(
            estimate(&mixed[0].route, mixed[0].seats, &mixed)
                .unwrap()
                .connecting,
            0
        );
        mixed[1].codeshare_partners = vec!["hub".into()];
        for seats in [40, 300, 1000] {
            for unequal in [false, true] {
                let mut supply = mixed.clone();
                supply[0].seats = seats;
                if unequal {
                    supply[1].route.plan.as_mut().unwrap().interval_seconds *= 2;
                }
                let mut n = TransferNetwork::new(supply.clone()).unwrap();
                for time in [None, Some(0), Some(180 * 86400)] {
                    n.estimate(time);
                    assert_eq!(n.paths.len(), 1);
                    let path = &n.paths[0];
                    let [a, b] = path.legs;
                    let units = (n.daily[0] / path.quantum).round();
                    assert_eq!(
                        n.passengers[a] as f64 * frequency(&supply[a].route),
                        n.passengers[b] as f64 * frequency(&supply[b].route)
                    );
                    assert_eq!(n.daily[0], units * path.quantum);
                    let d1 = MarketInputs::new(&supply[a].route, 0).unwrap().km;
                    let d2 = MarketInputs::new(&supply[b].route, 0).unwrap().km;
                    assert_eq!(path.fare[0], (300.0 * d1 / (d1 + d2)).floor() as i64);
                    assert_eq!(path.fare[0] + path.fare[1], 300);
                    for (i, o) in supply.iter().enumerate() {
                        let e =
                            estimate_with_season(&o.route, o.seats, &supply, time, None).unwrap();
                        let local = LocalPrepared::new(&o.route, o.seats, &supply)
                            .unwrap()
                            .estimate(time)
                            .0;
                        assert_eq!(e.per_departure - e.connecting, local.per_departure);
                        assert!(e.per_departure <= i64::from(o.seats));
                        assert_eq!(e.business, local.business);
                        if i < 2 && e.connecting > 0 {
                            assert_eq!(e.codeshare.len(), 1);
                            assert_eq!(e.codeshare[0].passengers, n.passengers[i]);
                            assert_eq!(e.codeshare[0].revenue, n.revenue[i]);
                        }
                    }
                }
            }
        }
        // With neutral scores, splitting ownership changes no allocation or money.
        for i in 0..2 {
            let original =
                estimate(&same_owner[i].route, same_owner[i].seats, &same_owner).unwrap();
            let split = estimate(&mixed[i].route, mixed[i].seats, &mixed).unwrap();
            assert!(split.connecting > 0);
            assert_eq!(original.connecting, split.connecting);
            assert_eq!(original.connecting_revenue, split.connecting_revenue);
        }
        mixed[0].reputation = -500_000;
        mixed[1].reputation = 500_000;
        let mut low = mixed.clone();
        low[1].reputation = -500_000;
        let mut neutral = mixed.clone();
        neutral[0].reputation = 0;
        neutral[1].reputation = 0;
        let mut a = TransferNetwork::new(mixed).unwrap();
        a.estimate(None);
        let mut b = TransferNetwork::new(low).unwrap();
        b.estimate(None);
        let mut c = TransferNetwork::new(neutral).unwrap();
        c.estimate(None);
        assert_eq!(a.daily, b.daily);
        assert!(a.daily[0] <= c.daily[0]);
    }

    #[test]
    fn transfers_conserve_od_markets_reserve_locals_and_split_revenue() {
        let offers = transfer_offers();
        let mut network = TransferNetwork::new(offers.clone()).unwrap();
        assert_eq!(network.paths.len(), 1);
        for time in [None, Some(0), Some(180 * 86400)] {
            network.estimate(time);
            assert!(network.daily[0] > 0.0);
            let mut carried = 0.0;
            for (i, o) in offers.iter().enumerate() {
                let mut local = LocalPrepared::new(&o.route, o.seats, &offers).unwrap();
                let baseline = local.estimate(time).0;
                let actual = estimate_with_season(&o.route, o.seats, &offers, time, None).unwrap();
                assert_eq!(
                    actual.per_departure - actual.connecting,
                    baseline.per_departure
                );
                assert_eq!(actual.business, baseline.business);
                assert!(actual.per_departure <= i64::from(o.seats));
                assert_eq!(actual.connecting, network.passengers[i]);
                if i == 2 {
                    carried += actual.daily_passengers;
                }
            }
            carried += network.daily[0];
            let base = market_with_season(&offers[2].route, 0, time)
                .unwrap()
                .total();
            assert!(carried <= base + 1e-9);
            assert_eq!(network.passengers[0], network.passengers[1]);
            let count = network.passengers[0];
            assert_eq!(network.revenue[0] + network.revenue[1], count * 300);
            let mut reverse = offers.clone();
            reverse.reverse();
            for o in &offers {
                assert_eq!(
                    estimate_with_season(&o.route, o.seats, &offers, time, None).unwrap(),
                    estimate_with_season(&o.route, o.seats, &reverse, time, None).unwrap()
                );
            }
        }
    }

    #[test]
    fn transfers_require_active_same_owner_sensible_geometry_and_economy_seats() {
        let offers = transfer_offers();
        for case in 0..6 {
            let mut supply = offers.clone();
            match case {
                0 => supply[1].route.plan = None,
                1 => supply[1].owner = "other".into(),
                2 => {
                    supply[0].route.origin_icao = "EDDM".into();
                    supply[0].route.destination_icao = "KLAX".into();
                    supply[1].route.origin_icao = "KLAX".into();
                }
                3 => supply[0].route.origin_icao = "EDFH".into(),
                4 => {
                    supply[0].business_seats = supply[0].seats;
                }
                _ => supply[0].route.ticket_eur = 100_000,
            }
            let e = estimate(&supply[0].route, supply[0].seats, &supply).unwrap();
            assert_eq!(e.connecting, 0, "case={case}");
        }
    }

    #[test]
    fn transfers_limit_both_legs_and_unequal_frequencies_with_no_owner_bonus() {
        let mut offers = transfer_offers();
        offers[0].route.plan.as_mut().unwrap().interval_seconds = 21600;
        offers[1].route.plan.as_mut().unwrap().interval_seconds = 64800;
        offers[0].seats = 180;
        offers[1].seats = 500;
        // A second airline supplies the identical two-legged offer.
        let mut other = offers[..2].to_vec();
        for o in &mut other {
            o.owner = "other".into();
        }
        offers.extend(other);
        let mut n = TransferNetwork::new(offers.clone()).unwrap();
        n.estimate(None);
        for (i, o) in offers.iter().enumerate() {
            let local = LocalPrepared::new(&o.route, o.seats, &offers)
                .unwrap()
                .estimate(None)
                .0;
            assert!(n.passengers[i] + local.per_departure <= i64::from(o.seats));
        }
        let mut sums = [0.0; 2];
        for (p, rate) in n.paths.iter().zip(&n.daily) {
            assert!(*rate <= p.frequency * 180.0);
            for (j, k) in p.legs.iter().enumerate() {
                sums[j] += n.passengers[*k] as f64 * frequency(&offers[*k].route);
            }
        }
        assert_eq!(sums[0], sums[1]);
        assert_eq!(n.passengers[0], n.passengers[3]);
        assert_eq!(n.passengers[1], n.passengers[4]);
    }

    #[test]
    fn transfers_five_airline_overlapping_networks_remain_bounded_in_every_market() {
        let template = transfer_offers().remove(0).route;
        let mut offers = Vec::new();
        for owner in 0..5 {
            for (i, destination) in ["EDDM", "KJFK", "LTFM", "EHAM", "LEMD", "EGLL"]
                .into_iter()
                .enumerate()
            {
                let mut route = template.clone();
                route.id = i as u32 + 1;
                route.origin_icao = "EDDF".into();
                route.destination_icao = destination.into();
                route.ticket_eur = 60 + owner * 20;
                route.plan.as_mut().unwrap().interval_seconds = (120 + i as u64 * 60) * 60;
                let mut o = Offer::new(&route, 180);
                o.business_seats = 12;
                o.owner = format!("owner{owner}");
                offers.push(o);
            }
        }
        // A direct offer competes with several same-airline connecting paths.
        let mut direct = offers[0].clone();
        direct.owner = "direct".into();
        direct.route.origin_icao = "EDDM".into();
        direct.route.destination_icao = "KJFK".into();
        offers.push(direct);
        let mut n = TransferNetwork::new(offers.clone()).unwrap();
        assert!(n.paths.len() > 10);
        for time in [Some(0), Some(180 * 86400)] {
            n.estimate(time);
            for (i, o) in offers.iter().enumerate() {
                let e = LocalPrepared::new(&o.route, o.seats, &offers)
                    .unwrap()
                    .estimate(time)
                    .0;
                assert!(e.per_departure + n.passengers[i] <= i64::from(o.seats));
                assert!(
                    e.per_departure - e.business + n.passengers[i]
                        <= i64::from(o.seats - o.business_seats)
                );
            }
            for m in &mut n.markets {
                let local = m.local.as_mut().map_or(0.0, |p| {
                    p.estimate(time);
                    p.allocations.iter().flat_map(|a| a.allocated).sum::<f64>()
                });
                let transfer = m.paths.iter().map(|&p| n.daily[p]).sum::<f64>();
                assert!(
                    local + transfer
                        <= market_with_season(&m.endpoints, 0, time).unwrap().total() + 1e-8
                );
            }
        }
    }

    #[test]
    fn prepared_market_matches_original_allocator_exactly_across_seasons() {
        let g = game(3, 120);
        for fare in [0, 70, 134, 100_000] {
            let mut supply = g.market_routes();
            supply[0].route.ticket_eur = fare;
            supply[1].business_seats = 12;
            supply[1].awareness = 23_000_000;
            supply[0].loyalty_members = 10_000;
            supply[1].loyalty_members = 200_000;
            supply[0].reputation = -850_000;
            supply[1].reputation = 650_000;
            supply[2].route.plan.as_mut().unwrap().interval_seconds = 7 * 86400;
            for own in 0..supply.len() {
                let o = &supply[own];
                let mut prepared = Prepared::new(&o.route, o.seats, &supply).unwrap();
                for day in (0..=730).step_by(7).chain([0, 1, 365, 0]) {
                    let time = Some(day * 86400 + 12345);
                    let mut expected_loads = Vec::new();
                    let expected = estimate_reference(
                        &o.route,
                        o.seats,
                        &supply,
                        time,
                        Some(&mut expected_loads),
                    )
                    .unwrap();
                    let (actual, loads) = prepared.estimate(time);
                    assert_eq!(actual, expected, "fare={fare}, own={own}, day={day}");
                    assert_eq!(loads, expected_loads);
                }
            }
        }
    }

    #[test]
    fn seat_spill_reaches_cheaper_and_pricier_offers_but_respects_limits() {
        let g = game(2, 720);
        for rival_price in [20, 100] {
            let mut a = g.routes[0].clone();
            let mut b = g.routes[1].clone();
            a.ticket_eur = 60;
            b.ticket_eur = rival_price;
            let uncapped = vec![Offer::new(&a, 10_000), Offer::new(&b, 10_000)];
            let limited = vec![Offer::new(&a, 1), Offer::new(&b, 10_000)];
            let before = estimate(&b, 10_000, &uncapped).unwrap();
            let after = estimate(&b, 10_000, &limited).unwrap();
            assert!(after.daily_passengers > before.daily_passengers);
            assert!(estimate(&a, 1, &limited).unwrap().daily_spill > 0.0);
            assert_eq!(passengers(&a, &limited).unwrap(), 1);
            let carried = after.daily_passengers + 1.0;
            assert!(carried <= market(&a, 0).unwrap().total());
            // Input order, IDs and the return direction do not affect economic shares.
            let mut reverse = limited.clone();
            reverse.reverse();
            assert_eq!(
                passengers(&b, &limited).unwrap(),
                passengers(&b, &reverse).unwrap()
            );
            std::mem::swap(&mut b.origin_icao, &mut b.destination_icao);
            assert_eq!(
                passengers(&b, &limited).unwrap(),
                passengers(&b, &reverse).unwrap()
            );
        }
        let mut a = g.routes[0].clone();
        let mut b = g.routes[1].clone();
        a.ticket_eur = 0;
        b.ticket_eur = 100_000;
        let blocked = vec![Offer::new(&a, 1), Offer::new(&b, 10_000)];
        assert_eq!(passengers(&b, &blocked).unwrap(), 0);
        // Leisure spill cannot enter a business-only offer above its price limit.
        b.ticket_eur =
            (crate::economy::reference_price(crate::routes::distance_km("EDDF", "EDDM").unwrap())
                * CONFIG.leisure_price_limit)
                .ceil() as i64;
        let business_only = vec![Offer::new(&a, 1), Offer::new(&b, 10_000)];
        assert_eq!(market(&b, b.ticket_eur).unwrap().leisure, 0.0);
        assert!(
            estimate(&b, 10_000, &business_only)
                .unwrap()
                .daily_passengers
                <= market(&b, b.ticket_eur).unwrap().business
        );
    }

    #[test]
    fn all_full_and_single_offer_are_capacity_bounded_without_inventing_demand() {
        let g = game(2, 1440);
        let offers: Vec<_> = g.routes.iter().map(|r| Offer::new(r, 1)).collect();
        for o in &offers {
            let e = estimate(&o.route, o.seats, &offers).unwrap();
            assert_eq!(e.per_departure, 1);
            assert!(e.daily_spill > 0.0);
        }
        let r = &g.routes[0];
        let sole = [Offer::new(r, 10_000)];
        let e = estimate(r, 10_000, &sole).unwrap();
        assert_eq!(
            e.per_departure,
            market(r, r.ticket_eur).unwrap().total().floor() as i64
        );
        assert_eq!(e.daily_spill, 0.0);
        assert_eq!(passengers(r, &[Offer::new(r, 1)]).unwrap(), 1);
    }

    #[test]
    fn spilling_service_matches_pause_speeds_steps_and_restart() {
        let mut g = game(2, 1440);
        g.set_ticket_price(1, 0).unwrap();
        assert!(
            estimate(&g.routes[0], 72, &g.market_routes())
                .unwrap()
                .daily_spill
                > 0.0
        );
        let mut fast = g.clone();
        assert!(fast.advance(30.0 * 86400.0));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("spill.json");
        for speed in [1, 4, 16] {
            let mut stepped = g.clone();
            stepped.clock.set_running(false);
            let paused = serde_json::to_value(&stepped).unwrap();
            assert!(stepped.advance(86400.0));
            assert_eq!(paused, serde_json::to_value(&stepped).unwrap());
            stepped.clock.set_speed(speed);
            stepped.clock.set_running(true);
            for day in 0..30 {
                for _ in 0..24 {
                    assert!(stepped.advance(3600.0 / f64::from(speed)));
                }
                if day == 7 {
                    crate::save::save(&path, &stepped).unwrap();
                    stepped = crate::save::load(&path).unwrap();
                }
            }
            stepped.clock.set_speed(1);
            assert_eq!(
                serde_json::to_value(&fast).unwrap(),
                serde_json::to_value(&stepped).unwrap()
            );
        }
    }

    #[test]
    fn unrelated_market_does_not_dilute_supply() {
        let g = game(1, 120);
        let r = &g.routes[0];
        let mut other = r.clone();
        other.destination_icao = "LTFM".into();
        let offers = vec![Offer::new(r, 72), Offer::new(&other, 72)];
        assert_eq!(
            passengers(r, &offers).unwrap(),
            passengers(r, &g.market_routes()).unwrap()
        );
    }

    #[test]
    fn airport_size_distance_and_segments_define_market_not_seats() {
        let g = game(1, 120);
        let mut r = g.routes[0].clone();
        let large = market(&r, 0).unwrap().total();
        r.origin_icao = "EDLP".into();
        r.destination_icao = "EDFH".into();
        let regional = market(&r, 0).unwrap().total();
        assert!(large > regional * 5.0);
        r.origin_icao = "EDDF".into();
        let mixed = market(&r, 0).unwrap().total();
        assert!(large > mixed && mixed > regional);
        r.origin_icao = "LTFM".into();
        r.destination_icao = "LTFJ".into();
        assert!(market(&r, 0).unwrap().total() < large); // tiny distance
        let r = &g.routes[0];
        let reference = economy::reference_price(
            crate::routes::distance_km(&r.origin_icao, &r.destination_icao).unwrap(),
        );
        let free = market(r, 0).unwrap();
        let paid = market(r, reference as i64).unwrap();
        assert!(paid.business / free.business > paid.leisure / free.leisure);
        let jet = economy::quote_shared(r, "a320_200", &g.market_routes()).unwrap();
        let atr = economy::quote_shared(r, "atr72_600", &g.market_routes()).unwrap();
        assert!(jet.passengers >= atr.passengers);
        let mut previous = free.total();
        for price in [50, 100, 150, 200, 100_000] {
            let demand = market(r, price).unwrap().total();
            assert!(demand <= previous);
            previous = demand;
        }
        assert_eq!(previous, 0.0);
    }

    #[test]
    fn actual_landings_cover_large_mixed_and_regional_airports_at_different_prices() {
        let mut free_passengers = Vec::new();
        for (a, b) in [("EDDF", "EDDM"), ("EDDF", "EDFH"), ("EDLP", "EDFH")] {
            let mut previous = i64::MAX;
            for price in [0, 100, 100_000] {
                let mut g = Game::default();
                g.found("Size Matrix", "DE", a).unwrap();
                g.buy_aircraft("atr72_600").unwrap();
                g.staff_up();
                g.create_route(a, b, 1).unwrap();
                g.set_ticket_price(1, price).unwrap();
                g.schedule(1, 1, 120).unwrap();
                g.clock.set_running(true);
                g.advance(86400.0);
                let t = &g.finance.totals;
                assert_eq!(t.flights, 12);
                assert!(t.passengers <= previous && t.passengers <= t.seats);
                assert_eq!(t.revenue - t.cargo_revenue, t.passengers * price);
                previous = t.passengers;
                if price == 0 {
                    free_passengers.push(t.passengers);
                }
                if price == 100_000 {
                    assert_eq!(t.passengers, 0);
                }
                g.validate_finance().unwrap();
                g.validate_history().unwrap();
            }
        }
        assert!(free_passengers[0] > free_passengers[1]);
        assert!(free_passengers[1] > free_passengers[2]);
    }

    #[test]
    fn multiple_flights_share_daily_market_instead_of_multiplying_it() {
        let mut one = game(1, 120);
        let mut two = game(2, 120);
        let daily = market(&one.routes[0], 0).unwrap().total();
        one.advance(86400.0);
        two.advance(86400.0);
        assert_eq!(one.finance.totals.flights, 12);
        assert_eq!(two.finance.totals.flights, 24);
        assert!(two.finance.totals.passengers as f64 <= 2.0 * daily);
        assert!(two.finance.totals.load_percent() < one.finance.totals.load_percent());
        // More frequent departures consume smaller slices of the same daily flow.
        let mut frequent = game(1, 80);
        frequent.advance(86400.0);
        assert!(frequent.finance.totals.flights > one.finance.totals.flights);
        assert!(frequent.finance.totals.passengers as f64 <= daily * 2.0);
    }

    #[test]
    fn reversed_routes_share_market_and_unplanned_routes_do_not_dilute_it() {
        let mut g = game(2, 120);
        let before = passengers(&g.routes[0], &g.market_routes()).unwrap();
        let r = &mut g.routes[1];
        std::mem::swap(&mut r.origin_icao, &mut r.destination_icao);
        assert_eq!(
            passengers(&g.routes[0], &g.market_routes()).unwrap(),
            before
        );
        g.routes[1].plan = None;
        assert!(passengers(&g.routes[0], &g.market_routes()).unwrap() > before);
        g.routes[1].plan = Some(operations::FlightPlan {
            first_departure: 60,
            interval_seconds: 7200,
            ..Default::default()
        });
        g.routes[1].ticket_eur = 100_000;
        assert_eq!(passengers(&g.routes[1], &g.market_routes()).unwrap(), 0);
        // An unaffordable competitor no longer takes customers from the cheap offer.
        assert!(passengers(&g.routes[0], &g.market_routes()).unwrap() > before);
    }

    #[test]
    fn rare_flights_do_not_bank_days_of_demand() {
        let g = game(1, 10080);
        assert_eq!(
            passengers(&g.routes[0], &g.market_routes()).unwrap(),
            (market(&g.routes[0], 100).unwrap().total().floor() as i64).min(72)
        );
    }

    #[test]
    fn shared_market_survives_restart_and_schedule_changes_without_rewriting_history() {
        let mut g = game(2, 120);
        let flight = operations::timings(&g.routes[0], "atr72_600").unwrap().0;
        g.advance((60 + flight + 1800) as f64);
        let old = g.finance.recent.clone();
        g.stop_schedule(2).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shared.json");
        crate::save::save(&path, &g).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        g.advance(86400.0);
        for _ in 0..864 {
            loaded.advance(100.0);
        }
        assert_eq!(g.finance, loaded.finance);
        assert_eq!(g.history, loaded.history);
        assert_eq!(&g.finance.recent[..old.len()], old.as_slice());
        let last = g.finance.recent.last().unwrap();
        assert!(last.totals.passengers > old[0].totals.passengers);
        crate::save::save(&path, &loaded).unwrap();
    }
}
