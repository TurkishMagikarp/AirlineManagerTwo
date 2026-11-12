//! Stationary daily O&D market. No backlog, aircraft-size dependence or per-flight spawning.
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
    c
});

pub struct Market {
    pub business: f64,
    pub leisure: f64,
}
impl Market {
    pub fn total(&self) -> f64 {
        self.business + self.leisure
    }
}

pub fn market(route: &Route, price: i64) -> Result<Market, String> {
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
    Ok(Market {
        business: base * share * willingness(c.business_price_limit),
        leisure: base * (1.0 - share) * willingness(c.leisure_price_limit),
    })
}

fn same_market(a: &Route, b: &Route) -> bool {
    (a.origin_icao == b.origin_icao && a.destination_icao == b.destination_icao)
        || (a.origin_icao == b.destination_icao && a.destination_icao == b.origin_icao)
}

/// Each shuttle offers 43200 / interval departures/day/direction. Each departure
/// gets an equal slice; expensive flights leave their unsold slice unused.
/// A minimum divisor of one prevents banking several days of demand on rare flights.
pub fn passengers(route: &Route, routes: &[Route]) -> Result<i64, String> {
    let frequency: f64 = routes
        .iter()
        .filter(|r| r.plan.is_some() && same_market(route, r))
        .map(|r| 43200.0 / r.plan.as_ref().unwrap().interval_seconds as f64)
        .sum();
    Ok((market(route, route.ticket_eur)?.total() / frequency.max(1.0)).floor() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{economy, game::Game, operations};

    fn game(count: u32, interval: u32) -> Game {
        let mut g = Game::default();
        g.found("Daily Market", "DE", "EDDF").unwrap();
        for id in 1..=count {
            g.buy_aircraft("atr72_600").unwrap();
            g.create_route("EDDF", "EDDM", id).unwrap();
            g.set_ticket_price(id, 100).unwrap();
            g.schedule(id, 1, interval).unwrap();
        }
        g.clock.set_running(true);
        g
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
        let jet = economy::quote_shared(r, "a320_200", &g.routes).unwrap();
        let atr = economy::quote_shared(r, "atr72_600", &g.routes).unwrap();
        assert_eq!(jet.passengers, atr.passengers);
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
                g.create_route(a, b, 1).unwrap();
                g.set_ticket_price(1, price).unwrap();
                g.schedule(1, 1, 120).unwrap();
                g.clock.set_running(true);
                g.advance(86400.0);
                let t = &g.finance.totals;
                assert_eq!(t.flights, 12);
                assert!(t.passengers <= previous && t.passengers <= t.seats);
                assert_eq!(t.revenue, t.passengers * price);
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
        let daily = market(&one.routes[0], 100).unwrap().total();
        one.advance(86400.0);
        two.advance(86400.0);
        assert_eq!(one.finance.totals.flights, 12);
        assert_eq!(two.finance.totals.flights, 24);
        assert!(two.finance.totals.passengers <= one.finance.totals.passengers);
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
        let before = passengers(&g.routes[0], &g.routes).unwrap();
        let r = &mut g.routes[1];
        std::mem::swap(&mut r.origin_icao, &mut r.destination_icao);
        assert_eq!(passengers(&g.routes[0], &g.routes).unwrap(), before);
        g.routes[1].plan = None;
        assert!(passengers(&g.routes[0], &g.routes).unwrap() >= before * 2);
        g.routes[1].plan = Some(operations::FlightPlan {
            first_departure: 60,
            interval_seconds: 7200,
            completed_flights: 0,
        });
        g.routes[1].ticket_eur = 100_000;
        assert_eq!(passengers(&g.routes[1], &g.routes).unwrap(), 0);
        // No redistribution of unsold quotas in this first version.
        assert_eq!(passengers(&g.routes[0], &g.routes).unwrap(), before);
    }

    #[test]
    fn rare_flights_do_not_bank_days_of_demand() {
        let g = game(1, 10080);
        assert_eq!(
            passengers(&g.routes[0], &g.routes).unwrap(),
            market(&g.routes[0], 100).unwrap().total().floor() as i64
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
