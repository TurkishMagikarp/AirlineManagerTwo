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
fn curve_at(curve: &[f64; 12], time: u64) -> f64 {
    let month = (time % YEAR_SECONDS) as f64 * 12.0 / YEAR_SECONDS as f64;
    let i = month.floor() as usize;
    let blend = (1.0 - ((month - i as f64) * std::f64::consts::PI).cos()) / 2.0;
    (curve[i] * (1.0 - blend) + curve[(i + 1) % 12] * blend) / (curve.iter().sum::<f64>() / 12.0)
}

pub fn season_factors(time: u64) -> Market {
    Market {
        business: curve_at(&CONFIG.business_monthly, time),
        leisure: curve_at(&CONFIG.leisure_monthly, time),
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

pub fn market_at(route: &Route, price: i64, time: u64) -> Result<Market, String> {
    market_with_season(route, price, Some(time))
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
    pub route: Route,
    pub seats: u32,
    pub business_seats: u32,
    pub cargo_model: Option<String>,
    premium: bool,
    pub awareness: i64,
    pub reputation: i64,
    pub owner: String,
}
impl Offer {
    pub fn new(route: &Route, seats: u32) -> Self {
        Self {
            route: route.clone(),
            seats,
            business_seats: 0,
            cargo_model: None,
            premium: false,
            awareness: 0,
            reputation: 0,
            owner: String::new(),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Estimate {
    pub business: i64,
    pub connecting: i64,
    pub connecting_revenue: i64,
    pub daily_connecting: f64,
    pub connections: Vec<Connection>,
    pub daily_passengers: f64,
    pub daily_spill: f64,
    pub per_departure: i64,
}

fn frequency(route: &Route) -> f64 {
    route
        .plan
        .as_ref()
        .map_or(1.0, |p| 43200.0 / p.interval_seconds as f64)
}

pub(crate) fn same_offer(a: &Route, b: &Route) -> bool {
    same_market(a, b)
        && a.id == b.id
        && a.aircraft_id == b.aircraft_id
        && a.ticket_eur == b.ticket_eur
        && a.business_ticket_eur == b.business_ticket_eur
        && a.cargo_rate_cents == b.cargo_rate_cents
        && a.service_level == b.service_level
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

/// Local, unsaved quote workspace for one route and immutable supply snapshot.
/// Every seasonal quantity and spill result is recomputed at departure time.
struct LocalPrepared {
    services: Vec<Offer>,
    matching: Vec<Offer>,
    own_indices: Vec<usize>,
    service_indices: Vec<usize>,
    inputs: Vec<MarketInputs>,
    base: MarketInputs,
    capacities: Vec<f64>,
    weights: Vec<[f64; 2]>,
    limits: Vec<[f64; 2]>,
    allocated: Vec<[f64; 2]>,
    loads: Vec<i64>,
    frequency: f64,
}
impl LocalPrepared {
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
    }

    pub fn new(route: &Route, seats: u32, routes: &[Offer]) -> Result<Self, String> {
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
        let services = matching.clone();
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
                .then(a.awareness.cmp(&b.awareness))
                .then(a.reputation.cmp(&b.reputation))
        });
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
        Ok(Self {
            services,
            matching,
            own_indices,
            service_indices,
            inputs,
            base,
            capacities,
            weights: vec![[0.0; 2]; len],
            limits: vec![[0.0; 2]; len],
            allocated: vec![[0.0; 2]; len],
            loads: vec![0; service_count],
            frequency: frequency(route),
        })
    }

    pub fn estimate(&mut self, time: Option<u64>) -> (Estimate, &[i64]) {
        let season = time.map(season_factors).unwrap_or(Market {
            business: 1.0,
            leisure: 1.0,
        });
        let base = self.base.at(&season);
        let matching = &self.matching;
        let own_indices = &self.own_indices;
        let service_indices = &self.service_indices;
        let capacities = &self.capacities;
        let weights = &mut self.weights;
        let limits = &mut self.limits;
        let allocated = &mut self.allocated;
        let mut frequency_sum = [0.0_f64; 2];
        let mut weight_sum = [0.0_f64; 2];
        for (j, o) in matching.iter().enumerate() {
            let f = frequency(&o.route);
            let mut m = self.inputs[j].at(&season);
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
                weights[j][i] = crate::marketing::attractiveness(o.awareness)
                    * crate::reputation::attractiveness(o.reputation)
                    * o.route.service_level.rate().attractiveness
                    * f
                    * w
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
        for j in 0..matching.len() {
            for (i, b) in [base.business, base.leisure].into_iter().enumerate() {
                allocated[j][i] = (b * weights[j][i] / (frequency_sum[i].max(1.0) + weight_sum[i]))
                    .min(limits[j][i]);
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
                        add[i] = (spill[i] * weights[j][i] / sums[i])
                            .min(limits[j][i] - allocated[j][i]);
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
        for &j in own_indices {
            let n = rounded_passengers(allocated[j].iter().sum(), &matching[j]);
            per_departure += n;
            if matching[j].premium {
                business += n;
            }
        }
        self.loads.fill(0);
        for (j, offer) in matching.iter().enumerate() {
            self.loads[service_indices[j]] += rounded_passengers(allocated[j].iter().sum(), offer);
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
    pub origin: String,
    pub hub: String,
    pub destination: String,
    pub daily: f64,
}

pub(crate) struct Prepared {
    local: LocalPrepared,
    network: Option<std::rc::Rc<std::cell::RefCell<TransferNetwork>>>,
    target: Option<usize>,
    loads: Vec<i64>,
    transfer_services: Vec<Option<usize>>,
}
impl Prepared {
    pub fn new(route: &Route, seats: u32, routes: &[Offer]) -> Result<Self, String> {
        let local = LocalPrepared::new(route, seats, routes)?;
        let owner = routes
            .iter()
            .find(|o| same_offer(route, &o.route))
            .map(|o| &o.owner);
        let network = if route.plan.is_some() {
            transfer_network(route, routes)?
        } else {
            None
        };
        let target = network.as_ref().and_then(|n| {
            n.borrow().offers.iter().position(|o| {
                same_offer(route, &o.route) && Some(&o.owner) == owner && o.seats == seats
            })
        });
        let transfer_services = local
            .services
            .iter()
            .map(|key| {
                network.as_ref().and_then(|n| {
                    n.borrow()
                        .offers
                        .iter()
                        .position(|o| o.owner == key.owner && same_offer(&o.route, &key.route))
                })
            })
            .collect();
        Ok(Self {
            transfer_services,
            local,
            network,
            target,
            loads: Vec::new(),
        })
    }
    pub(crate) fn reprice_single(&mut self, route: &Route) {
        self.local.reprice_single(route);
        if let (Some(network), Some(target)) = (&self.network, self.target) {
            let mut offers = network.borrow().offers.clone();
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
        let (mut result, loads) = self.local.estimate(time);
        self.loads.clear();
        self.loads.extend_from_slice(loads);
        if let (Some(network), Some(target)) = (&self.network, self.target) {
            let mut n = network.borrow_mut();
            n.estimate(time);
            result.connecting = n.passengers[target];
            result.connecting_revenue = n.revenue[target];
            result.daily_connecting = result.connecting as f64 * self.local.frequency;
            result.per_departure += result.connecting;
            result.daily_passengers += result.daily_connecting;
            for (i, index) in self.transfer_services.iter().enumerate() {
                if let Some(j) = index {
                    self.loads[i] += n.passengers[*j];
                }
            }
            if display {
                for (p, daily) in n.paths.iter().zip(&n.daily) {
                    if *daily > 0.0 && (p.legs[0] == target || p.legs[1] == target) {
                        result.connections.push(Connection {
                            origin: p.origin.clone(),
                            hub: p.hub.clone(),
                            destination: p.destination.clone(),
                            daily: *daily,
                        });
                    }
                }
            }
        }
        (result, &self.loads)
    }
}

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
    local_indices: Vec<Option<usize>>,
    weights: Vec<[[f64; 2]; 2]>,
    outside_weights: [f64; 2],
}
struct TransferNetwork {
    reputation_factors: Vec<f64>,
    offers: Vec<Offer>,
    paths: Vec<TransferPath>,
    markets: Vec<TransferMarket>,
    passengers: Vec<i64>,
    revenue: Vec<i64>,
    daily: Vec<f64>,
    last: Option<Option<u64>>,
    recent: std::collections::VecDeque<TransferAllocation>,
}
struct TransferAllocation {
    time: Option<u64>,
    passengers: Vec<i64>,
    revenue: Vec<i64>,
    daily: Vec<f64>,
}
thread_local! {
    // Derived, bounded and unsaved. Quotes for different owners share the exact
    // same network allocation, independent of supply/airline iteration order.
    static TRANSFERS: std::cell::RefCell<Vec<std::rc::Rc<std::cell::RefCell<TransferNetwork>>>> = const { std::cell::RefCell::new(Vec::new()) };
}
/// Structural connection data survives fare, awareness and reputation changes.
/// Bounded independently of economic snapshots, including networks with no paths.
#[derive(Clone, PartialEq, Eq)]
struct TopologyLeg {
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
    static TOPOLOGIES: std::cell::RefCell<Vec<std::rc::Rc<TransferTopology>>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn transfer_topology(offers: &[Offer]) -> Result<std::rc::Rc<TransferTopology>, String> {
    let legs: Vec<_> = offers
        .iter()
        .map(|o| TopologyLeg {
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
                if a.owner != b.owner
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
                    paths.push(ConnectionGeometry {
                        legs: [i, j],
                        origin: origin.clone(),
                        hub: hub.clone(),
                        destination: destination.clone(),
                        distances: [d1, d2],
                        direct,
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
        let topology = std::rc::Rc::new(TransferTopology {
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

fn transfer_network(
    route: &Route,
    routes: &[Offer],
) -> Result<Option<std::rc::Rc<std::cell::RefCell<TransferNetwork>>>, String> {
    let mut offers: Vec<_> = routes
        .iter()
        .filter(|o| o.route.plan.is_some())
        .map(|o| {
            let mut o = o.clone();
            // Only the interval enters demand. Progress, frozen results and
            // disruption cursors must not invalidate an economic snapshot.
            o.route.plan = Some(crate::operations::FlightPlan {
                interval_seconds: o.route.plan.as_ref().unwrap().interval_seconds,
                ..Default::default()
            });
            o
        })
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
    // Departures and cargo quotes only need networks touching their own market.
    if !topology.paths_for(route).iter().any(|&p| {
        let p = &topology.paths[p];
        let [a, b] = p.legs.map(|i| &offers[i]);
        ((a.route.ticket_eur + b.route.ticket_eur) as f64)
            < p.direct.reference * CONFIG.business_price_limit.max(CONFIG.leisure_price_limit)
    }) {
        return Ok(None);
    }
    TRANSFERS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(n) = cache.iter().find(|n| {
            let n = n.borrow();
            n.offers == offers
                && n.reputation_factors
                    .iter()
                    .zip(&offers)
                    .all(|(factor, o)| *factor == crate::reputation::attractiveness(o.reputation))
        }) {
            return Ok(Some(n.clone()));
        }
        let network = TransferNetwork::new(offers)?;
        if network.paths.is_empty() {
            return Ok(None);
        }
        let n = std::rc::Rc::new(std::cell::RefCell::new(network));
        if cache.len() == 4 {
            cache.remove(0);
        }
        cache.push(n.clone());
        Ok(Some(n))
    })
}
impl TransferNetwork {
    fn new(offers: Vec<Offer>) -> Result<Self, String> {
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
            let (mut x, mut y) = (
                a.route.plan.as_ref().unwrap().interval_seconds,
                b.route.plan.as_ref().unwrap().interval_seconds,
            );
            while y != 0 {
                (x, y) = (y, x % y);
            }
            paths.push(TransferPath {
                legs: [i, j],
                origin: geometry.origin.clone(),
                hub: geometry.hub.clone(),
                destination: geometry.destination.clone(),
                inputs,
                frequency: frequency(&a.route).min(frequency(&b.route)),
                quantum: 43200.0 / x as f64,
                fare: [first_fare, total_fare - first_fare],
            });
        }
        if paths.is_empty() {
            return Ok(Self {
                reputation_factors,
                offers,
                paths,
                markets: Vec::new(),
                passengers: Vec::new(),
                revenue: Vec::new(),
                daily: Vec::new(),
                last: None,
                recent: std::collections::VecDeque::new(),
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
                    local_indices: Vec::new(),
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
                    local_indices: Vec::new(),
                    weights: Vec::new(),
                    outside_weights: [1.0; 2],
                });
            }
        }
        for m in &mut markets {
            if let Some(local) = &m.local {
                m.local_indices = local
                    .matching
                    .iter()
                    .map(|o| {
                        if o.premium {
                            None
                        } else {
                            Some(
                                offers
                                    .iter()
                                    .position(|p| {
                                        p.owner == o.owner && same_offer(&p.route, &o.route)
                                    })
                                    .expect("local network offer"),
                            )
                        }
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
                    let weight = path.frequency * w / (1.0 - w).max(1e-9);
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
            reputation_factors,
            offers,
            paths,
            markets,
            passengers: vec![0; count],
            revenue: vec![0; count],
            daily: vec![0.0; path_count],
            last: None,
            recent: std::collections::VecDeque::new(),
        })
    }
    fn estimate(&mut self, time: Option<u64>) {
        if self.last == Some(time) {
            return;
        }
        self.last = Some(time);
        if let Some(cached) = self.recent.iter().find(|c| c.time == time) {
            self.passengers.copy_from_slice(&cached.passengers);
            self.revenue.copy_from_slice(&cached.revenue);
            self.daily.copy_from_slice(&cached.daily);
            return;
        }
        self.passengers.fill(0);
        self.revenue.fill(0);
        self.daily.fill(0.0);
        let season = time.map(season_factors).unwrap_or(Market {
            business: 1.0,
            leisure: 1.0,
        });
        let mut spare: Vec<f64> = self
            .offers
            .iter()
            .map(|o| frequency(&o.route) * f64::from(o.seats - o.business_seats))
            .collect();
        for m in &mut self.markets {
            let base = m.base.at(&season);
            let mut residual = [base.business, base.leisure];
            if let Some(local) = &mut m.local {
                local.estimate(time);
                for (j, o) in local.matching.iter().enumerate() {
                    for (s, r) in residual.iter_mut().enumerate() {
                        *r = (*r - local.allocated[j][s]).max(0.0);
                    }
                    if !o.premium {
                        let k = m.local_indices[j].expect("economy network offer");
                        spare[k] -= rounded_passengers(local.allocated[j].iter().sum(), o) as f64
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
                        .map(|(&p, w)| w[s][1] * self.reputation_factors[self.paths[p].legs[0]])
                        .sum::<f64>();
                for (&p, weights) in m.paths.iter().zip(&m.weights) {
                    let [w, weight] = weights[s];
                    let weight = weight * self.reputation_factors[self.paths[p].legs[0]];
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
        // Owners often project a whole day's departures one route at a time.
        // Retain a bounded set of exact-time allocations for the next route.
        if self.recent.len() == 256 {
            self.recent.pop_front();
        }
        self.recent.push_back(TransferAllocation {
            time,
            passengers: self.passengers.clone(),
            revenue: self.revenue.clone(),
            daily: self.daily.clone(),
        });
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
            .then(a.awareness.cmp(&b.awareness))
            .then(a.reputation.cmp(&b.reputation))
    });
    let own_indices: Vec<_> = matching
        .iter()
        .enumerate()
        .filter_map(|(i, (_, own, _))| own.then_some(i))
        .collect();
    let service_indices: Vec<_> = matching.iter().map(|(_, _, index)| *index).collect();
    let matching: Vec<_> = matching.into_iter().map(|(o, _, _)| o).collect();
    let at = |route: &Route, price| match time {
        Some(time) => market_at(route, price, time),
        None => market(route, price),
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
            weights[j][i] = crate::marketing::attractiveness(o.awareness)
                * crate::reputation::attractiveness(o.reputation)
                * o.route.service_level.rate().attractiveness
                * f
                * w
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
        assert!(std::rc::Rc::ptr_eq(
            &topology,
            &transfer_topology(&changed).unwrap()
        ));
        changed[0].route.plan.as_mut().unwrap().interval_seconds *= 2;
        assert!(!std::rc::Rc::ptr_eq(
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
                    p.allocated.iter().flatten().sum::<f64>()
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
