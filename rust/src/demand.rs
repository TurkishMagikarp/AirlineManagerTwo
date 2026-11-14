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
    let willingness = |limit: f64| (1.0 - price as f64 / (reference * limit)).clamp(0.0, 1.0);
    let season = time.map(season_factors).unwrap_or(Market {
        business: 1.0,
        leisure: 1.0,
    });
    Ok(Market {
        business: season.business * base * share * willingness(c.business_price_limit),
        leisure: season.leisure * base * (1.0 - share) * willingness(c.leisure_price_limit),
    })
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
            owner: String::new(),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Estimate {
    pub business: i64,
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

/// One allocation supplies the flight quote and every cargo payload limit.
pub fn estimate_with_loads(
    route: &Route,
    seats: u32,
    routes: &[Offer],
    time: Option<u64>,
) -> Result<(Estimate, Vec<i64>), String> {
    let mut loads = Vec::new();
    let estimate = estimate_with_season(route, seats, routes, time, Some(&mut loads))?;
    Ok((estimate, loads))
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
