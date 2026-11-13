//! Provisional deterministic balancing. Whole EUR, rounded up for hourly costs.
use crate::{aircraft, game::Game, operations, routes::Route};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::LazyLock};

// Scoped to the current test thread: exercise the real economy continuously at
// the configured upper bound without changing production rules or save format.
#[cfg(test)]
thread_local! { pub(crate) static HIGH_FUEL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

/// One world seed and timeline, shared verbatim by all companies. Prices are
/// derived at departure time; no frame-driven RNG or mutable UI clock.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FuelMarket {
    pub seed: u64,
    pub started_at: u64,
}
impl Default for FuelMarket {
    fn default() -> Self {
        Self {
            seed: 0x4149524c494e45,
            started_at: 0,
        }
    }
}
impl FuelMarket {
    pub fn price_at(&self, time: u64) -> i64 {
        #[cfg(test)]
        if HIGH_FUEL.with(|enabled| enabled.get()) {
            return CONFIG.fuel_max_cents_per_kg;
        }
        let c = &*CONFIG;
        let mut price = c.fuel_cents_per_kg;
        let mut rng = self.seed;
        for _ in 0..time.saturating_sub(self.started_at) / c.fuel_cadence_seconds {
            // SplitMix64, explicitly wrapping and independent of platform RNGs.
            rng = rng.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = rng;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^= z >> 31;
            let shock =
                (z % (2 * c.fuel_volatility_cents + 1) as u64) as i64 - c.fuel_volatility_cents;
            price = (price + shock + (c.fuel_cents_per_kg - price) / c.fuel_mean_reversion_divisor)
                .clamp(c.fuel_min_cents_per_kg, c.fuel_max_cents_per_kg);
        }
        price
    }

    pub fn max_price(&self) -> i64 {
        CONFIG.fuel_max_cents_per_kg
    }

    pub fn next_change(&self, time: u64) -> u64 {
        let c = CONFIG.fuel_cadence_seconds;
        self.started_at
            .saturating_add((time.saturating_sub(self.started_at) / c + 1).saturating_mul(c))
    }
}

pub const HISTORY_LIMIT: usize = 100;
#[derive(Deserialize)]
pub struct Config {
    pub default_ticket_eur: i64,
    reference_base_eur: i64,
    reference_eur_per_100_km: i64,
    fuel_cents_per_kg: i64,
    fuel_min_cents_per_kg: i64,
    fuel_max_cents_per_kg: i64,
    fuel_volatility_cents: i64,
    fuel_mean_reversion_divisor: i64,
    fuel_cadence_seconds: u64,
    airport_eur_per_passenger: i64,
    models: HashMap<String, Rates>,
}

#[derive(Deserialize)]
struct Rates {
    fuel_kg_per_hour: i64,
    fuel_cycle_kg: i64,
    airport_base_eur: i64,
    overhead_eur_per_hour: i64,
    overhead_cycle_eur: i64,
}
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let c: Config =
        serde_json::from_str(include_str!("../../config/economy.json")).expect("economy config");
    assert!((0..=100_000).contains(&c.default_ticket_eur));
    for n in [
        c.reference_base_eur,
        c.reference_eur_per_100_km,
        c.fuel_cents_per_kg,
        c.airport_eur_per_passenger,
    ] {
        assert!((0..=100_000).contains(&n));
    }
    assert!(c.reference_base_eur > 0);
    assert!(0 < c.fuel_min_cents_per_kg && c.fuel_min_cents_per_kg <= c.fuel_cents_per_kg);
    assert!((c.fuel_cents_per_kg..=100_000).contains(&c.fuel_max_cents_per_kg));
    assert!((0..=100_000).contains(&c.fuel_volatility_cents));
    assert!((1..=100_000).contains(&c.fuel_mean_reversion_divisor));
    assert!(c.fuel_cadence_seconds >= 86400);
    for m in aircraft::models().expect("models") {
        let r = &c.models[&m.id];
        for n in [
            r.fuel_kg_per_hour,
            r.fuel_cycle_kg,
            r.airport_base_eur,
            r.overhead_eur_per_hour,
            r.overhead_cycle_eur,
        ] {
            assert!((0..=100_000).contains(&n));
        }
    }
    c
});
pub fn default_ticket() -> i64 {
    CONFIG.default_ticket_eur
}

pub fn rules_text() -> String {
    "Tagesmarkt je Richtung: Flughafengröße, Bedeutung und Entfernung. Geschäftsreisende sind weniger preissensibel als Urlaubsreisende. Alle aktiven Pendelflugpläne derselben Flughafenverbindung teilen die Nachfrage nach Frequenz, auch bei umgekehrter Route. Gleichmäßiger Tagesdurchschnitt, kein Tageskontingent oder Rückstau. Ganze Passagiere abgerundet und auf Sitze begrenzt. Passagiere, Einnahmen, Auslastung und Kosten stehen bei Abflug fest; Abrechnung bei Landung, auch leer. Details: docs/demand.md; Werte: config/demand.json und config/economy.json.".into()
}

pub fn reference_price(km: f64) -> f64 {
    (CONFIG.reference_base_eur + km.round() as i64 * CONFIG.reference_eur_per_100_km / 100) as f64
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Totals {
    pub flights: i64,
    pub passengers: i64,
    pub seats: i64,
    pub revenue: i64,
    pub fuel: i64,
    pub airport: i64,
    pub crew: i64,
    // Additive field: historical version-6 results keep their original costs.
    #[serde(default)]
    pub overhead: i64,
}
impl Totals {
    pub fn costs(&self) -> i64 {
        self.fuel + self.airport + self.crew + self.overhead
    }
    pub fn profit(&self) -> i64 {
        self.revenue - self.costs()
    }
    pub fn load_percent(&self) -> f64 {
        if self.seats == 0 {
            0.0
        } else {
            self.passengers as f64 * 100.0 / self.seats as f64
        }
    }
    pub fn add(&mut self, other: &Self, count: u64) -> Option<()> {
        let count = i64::try_from(count).ok()?;
        macro_rules! add { ($($f:ident),*) => { $(self.$f = self.$f.checked_add(other.$f.checked_mul(count)?)?;)* }; }
        add!(
            flights, passengers, seats, revenue, fuel, airport, crew, overhead
        );
        self.fuel
            .checked_add(self.airport)?
            .checked_add(self.crew)?
            .checked_add(self.overhead)?;
        Some(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlightResult {
    pub route_id: u32,
    pub landing: u64,
    pub origin: String,
    pub destination: String,
    pub ticket_eur: i64,
    pub totals: Totals,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Finance {
    pub totals: Totals,
    pub recent: Vec<FlightResult>,
}

#[cfg(test)]
pub fn quote(route: &Route, model_id: &str) -> Result<Totals, String> {
    quote_shared(
        route,
        model_id,
        &[crate::demand::Offer::new(
            route,
            aircraft::model(model_id)?.seats,
        )],
    )
}

#[cfg(test)]
pub fn quote_shared(
    route: &Route,
    model_id: &str,
    routes: &[crate::demand::Offer],
) -> Result<Totals, String> {
    quote_at_price(route, model_id, routes, CONFIG.fuel_cents_per_kg)
}

pub fn quote_at_price(
    route: &Route,
    model_id: &str,
    routes: &[crate::demand::Offer],
    fuel_cents: i64,
) -> Result<Totals, String> {
    let c = &*CONFIG;
    let model = aircraft::model(model_id)?;
    let demand = crate::demand::estimate(route, model.seats, routes)?.per_departure;
    let passengers = demand.min(i64::from(model.seats));
    let seconds = operations::timings(route, model_id)?.0 as i64;
    let rates = &c.models[model_id];
    Ok(Totals {
        flights: 1,
        passengers,
        seats: model.seats.into(),
        revenue: passengers * route.ticket_eur,
        fuel: ((seconds * rates.fuel_kg_per_hour + rates.fuel_cycle_kg * 3600) * fuel_cents
            + 359999)
            / 360000,
        airport: rates.airport_base_eur + c.airport_eur_per_passenger * passengers,
        // Personnel is paid through payroll. Keep Totals::crew for immutable
        // historical results and departure snapshots from existing saves.
        crew: 0,
        overhead: rates.overhead_cycle_eur + (seconds * rates.overhead_eur_per_hour + 3599) / 3600,
    })
}

impl Game {
    pub fn set_ticket_price(&mut self, id: u32, price: i64) -> Result<(), String> {
        if !(0..=100_000).contains(&price) {
            return Err("Ticketpreis muss zwischen 0 und 100000 EUR liegen.".into());
        }
        self.route_mut(id)?;
        for r in self.routes.iter_mut().filter(|r| r.public_id() == id) {
            r.ticket_eur = price;
        }
        Ok(())
    }
    pub fn validate_finance(&self) -> Result<(), String> {
        let valid = |t: &Totals| {
            t.flights >= 0
                && t.passengers >= 0
                && t.seats >= t.passengers
                && t.revenue >= 0
                && t.fuel >= 0
                && t.airport >= 0
                && t.crew >= 0
                && t.overhead >= 0
                && t.fuel
                    .checked_add(t.airport)
                    .and_then(|v| v.checked_add(t.crew))
                    .and_then(|v| v.checked_add(t.overhead))
                    .is_some()
        };
        if !valid(&self.finance.totals)
            || self.finance.recent.len() > HISTORY_LIMIT
            || self.finance.recent.len() as i64
                != self.finance.totals.flights.min(HISTORY_LIMIT as i64)
            || (self.airline.is_none() && self.finance != Finance::default())
        {
            return Err("Ungültige Finanzdaten.".into());
        }
        let mut sum = Totals::default();
        let mut previous = None;
        for f in &self.finance.recent {
            if !valid(&f.totals)
                || f.totals.flights != 1
                || f.route_id == 0
                || f.route_id >= self.next_route_id
                || f.landing > self.clock.elapsed().as_secs()
                || !(0..=100_000).contains(&f.ticket_eur)
                || f.totals.passengers.checked_mul(f.ticket_eur) != Some(f.totals.revenue)
                || crate::routes::distance_km(&f.origin, &f.destination).is_err()
                || previous.is_some_and(|p| p >= (f.landing, f.route_id))
            {
                return Err("Ungültiges Flugergebnis.".into());
            }
            previous = Some((f.landing, f.route_id));
            sum.add(&f.totals, 1).ok_or("Finanzüberlauf.")?;
        }
        let t = &self.finance.totals;
        if t.flights <= HISTORY_LIMIT as i64 && sum != *t {
            return Err("Unvollständige Finanzsummen.".into());
        }
        if sum.flights > t.flights
            || sum.passengers > t.passengers
            || sum.seats > t.seats
            || sum.revenue > t.revenue
            || sum.fuel > t.fuel
            || sum.airport > t.airport
            || sum.crew > t.crew
            || sum.overhead > t.overhead
        {
            return Err("Widersprüchliche Finanzsummen.".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(price: i64) -> Game {
        let mut g = Game::default();
        g.found("Economy Air", "DE", "EDDF").unwrap();
        g.competitors.clear();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.set_ticket_price(1, price).unwrap();
        g.schedule(1, 1, 120).unwrap();
        g.clock.set_running(true);
        g
    }
    #[test]
    fn prices_distance_capacity_and_costs_are_predictable() {
        let mut g = game(0);
        let full = quote(&g.routes[0], "atr72_600").unwrap();
        assert_eq!(full.passengers, full.seats);
        assert_eq!(full.revenue, 0);
        assert!(full.profit() < 0);
        g.routes[0].ticket_eur = 100;
        let middle = quote(&g.routes[0], "atr72_600").unwrap();
        assert!(middle.passengers > 0 && middle.passengers < middle.seats);
        assert_eq!(middle.revenue, middle.passengers * 100);
        assert!(middle.profit() > 0);
        g.routes[0].ticket_eur = 100000;
        let empty = quote(&g.routes[0], "atr72_600").unwrap();
        assert_eq!(empty.passengers, 0);
        assert!(empty.fuel > 0 && empty.airport > 0);
        assert_eq!(empty.crew, 0);
        assert_eq!(empty.fuel, middle.fuel);
        assert_eq!(empty.crew, middle.crew);
        assert!(empty.airport < middle.airport);
        let short = quote(&g.routes[0], "a320_200").unwrap();
        g.routes[0].destination_icao = "LTFM".into();
        let long = quote(&g.routes[0], "a320_200").unwrap();
        assert!(long.fuel > short.fuel && long.overhead > short.overhead);
        g.routes[0].ticket_eur = 150;
        assert!(quote(&g.routes[0], "a320_200").unwrap().passengers > middle.passengers);
    }
    #[test]
    fn landing_debits_once_empty_flights_can_create_debt_and_prices_validate() {
        let mut g = game(100000);
        assert!(g.set_ticket_price(1, 100000).is_ok());
        let flight = operations::timings(&g.routes[0], "atr72_600").unwrap().0;
        g.airline.as_mut().unwrap().capital_eur = 0;
        // Test-only depletion before the first landing; keep its ledger anchor consistent.
        g.history.as_mut().unwrap().opening_balance = 3_000_000;
        g.advance((60 + flight - 1) as f64);
        assert_eq!(g.finance.totals.flights, 0);
        assert_eq!(g.airline.as_ref().unwrap().capital_eur, 0);
        g.advance(1.0);
        let cost = g.finance.totals.costs();
        assert!(cost > 0);
        assert_eq!(g.airline.as_ref().unwrap().capital_eur, -cost);
        for _ in 0..10 {
            g.advance(0.0);
        }
        assert_eq!(g.finance.totals.flights, 1);
        g.advance(1800.0);
        g.stop_schedule(1).unwrap();
        let before = g.routes.clone();
        for price in [-1, 100001, i64::MAX] {
            assert!(g.set_ticket_price(1, price).is_err());
        }
        assert!(g.set_ticket_price(99, 20).is_err());
        assert_eq!(g.routes, before);
        g.set_ticket_price(1, 150).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("debt.json");
        crate::save::save(&path, &g).unwrap();
        let loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.airline, g.airline);
        assert_eq!(loaded.finance, g.finance);
        assert_eq!(loaded.routes, g.routes);
    }
    #[test]
    fn frame_sizes_16x_save_load_and_history_are_identical() {
        let mut large = game(150);
        let mut small = game(150);
        for g in [&mut large, &mut small] {
            g.buy_aircraft("atr72_600").unwrap();
            g.staff_up();
            g.create_route("EDDF", "EDDM", 2).unwrap();
            g.set_ticket_price(2, 100000).unwrap();
            g.schedule(2, 1, 120).unwrap();
        }
        large.clock.set_speed(16);
        small.clock.set_speed(16);
        let start = large.airline.as_ref().unwrap().capital_eur;
        large.advance(100000.0);
        for _ in 0..10000 {
            small.advance(10.0);
        }
        assert_eq!(large.finance, small.finance);
        assert_eq!(large.airline, small.airline);
        assert_eq!(large.finance.recent.len(), HISTORY_LIMIT);
        assert_eq!(
            large.airline.as_ref().unwrap().capital_eur,
            start + large.finance.totals.profit()
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("money.json");
        crate::save::save(&path, &large).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        large.advance(10000.0);
        loaded.advance(10000.0);
        assert_eq!(large.finance, loaded.finance);
        assert_eq!(large.airline, loaded.airline);
        loaded.clock.set_running(false);
        let before = loaded.finance.clone();
        loaded.advance(10000.0);
        assert_eq!(before, loaded.finance);
    }
    #[test]
    fn v5_migration_never_charges_past_landings_and_new_saves_reject_bad_finance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        let mut g = game(150);
        g.advance(100000.0);
        crate::save::save(&path, &g).unwrap();
        let valid: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let mut old = valid.clone();
        old["version"] = 5.into();
        old["state"].as_object_mut().unwrap().remove("finance");
        old["state"]["routes"]["routes"][0]
            .as_object_mut()
            .unwrap()
            .remove("ticket_eur");
        std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.airline, g.airline);
        assert_eq!(loaded.finance, Finance::default());
        assert_eq!(loaded.routes[0].ticket_eur, default_ticket());
        loaded.advance(0.0);
        assert_eq!(loaded.finance, Finance::default());
        loaded.advance(7200.0);
        assert_eq!(loaded.finance.totals.flights, 1);
        for (field, value) in [
            ("revenue", -1),
            ("passengers", i64::MAX),
            ("crew", i64::MAX),
            ("overhead", -1),
            ("overhead", i64::MAX),
        ] {
            let mut bad = valid.clone();
            bad["state"]["finance"]["totals"][field] = value.into();
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(crate::save::load(&path).is_err());
        }
        let mut bad = valid;
        bad["state"].as_object_mut().unwrap().remove("finance");
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(crate::save::load(&path).is_err());
    }
    #[test]
    fn financial_overflow_rolls_back_clock_flights_and_balance() {
        let mut g = game(100);
        g.airline.as_mut().unwrap().capital_eur = i64::MAX;
        let before = g.routes.clone();
        assert!(!g.advance(10000.0));
        assert_eq!(g.clock.elapsed().as_secs(), 0);
        assert_eq!(g.routes, before);
        assert_eq!(g.finance, Finance::default());
        assert_eq!(g.airline.as_ref().unwrap().capital_eur, i64::MAX);
    }

    #[test]
    fn deleting_routes_preserves_results_and_new_game_resets_finances() {
        let mut g = game(150);
        let flight = operations::timings(&g.routes[0], "atr72_600").unwrap().0;
        g.advance((60 + flight + 1800) as f64);
        g.stop_schedule(1).unwrap();
        let before = g.finance.clone();
        g.delete_route(1).unwrap();
        assert_eq!(before, g.finance);
        g.validate_finance().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deleted.json");
        crate::save::save(&path, &g).unwrap();
        assert_eq!(crate::save::load(&path).unwrap().finance, before);
        g.found("New company", "DE", "EDDF").unwrap();
        assert_eq!(g.finance, Finance::default());
        assert_eq!(
            g.airline.unwrap().capital_eur,
            crate::game::starting_capital().unwrap()
        );
    }

    #[test]
    fn calibrated_route_matrix_and_price_sensitivity() {
        for (origin, destination) in [
            ("EDDF", "EDDM"),
            ("EDDF", "LTFM"),
            ("KJFK", "KLAX"),
            ("LTFM", "LTFJ"),
        ] {
            for model in aircraft::models().unwrap() {
                let km = crate::routes::distance_km(origin, destination).unwrap();
                if km > f64::from(model.range_km) {
                    continue;
                }
                let reference = CONFIG.reference_base_eur
                    + km.round() as i64 * CONFIG.reference_eur_per_100_km / 100;
                let mut previous = i64::MAX;
                let mut prices = vec![
                    0,
                    default_ticket(),
                    reference,
                    reference * 3 / 2,
                    reference * 19 / 10,
                    reference * 2,
                    100_000,
                ];
                prices.sort_unstable();
                prices.dedup();
                for price in prices {
                    let route = Route {
                        id: 1,
                        origin_icao: origin.into(),
                        destination_icao: destination.into(),
                        aircraft_id: Some(1),
                        parent_route_id: None,
                        remove_at: None,
                        rotation_phase: None,
                        ticket_eur: price,
                        plan: None,
                    };
                    let mut route = route;
                    let (flight, ground) = operations::timings(&route, &model.id).unwrap();
                    let interval = (flight + ground).div_ceil(60) * 60;
                    route.plan = Some(operations::FlightPlan {
                        first_departure: 60,
                        interval_seconds: interval,
                        ..Default::default()
                    });
                    let t = quote(&route, &model.id).unwrap();
                    assert!(t.passengers <= previous && t.passengers <= t.seats);
                    previous = t.passengers;
                    assert_eq!(t.costs(), t.fuel + t.airport + t.crew + t.overhead);
                    // Exercise real landings and the ledger for every model/price,
                    // not just isolated quotes. No personal save file is touched.
                    let country = match origin {
                        "KJFK" => "US",
                        "LTFM" => "TR",
                        _ => "DE",
                    };
                    let mut g = Game::default();
                    g.found("Calibration Air", country, origin).unwrap();
                    g.competitors.clear();
                    g.buy_aircraft(&model.id).unwrap();
                    g.staff_up();
                    g.create_route(origin, destination, 1).unwrap();
                    g.set_ticket_price(1, price).unwrap();
                    let (flight, ground) = operations::timings(&g.routes[0], &model.id).unwrap();
                    g.schedule(1, 1, (flight + ground).div_ceil(60) as u32)
                        .unwrap();
                    let balance = g.airline.as_ref().unwrap().capital_eur;
                    g.clock.set_running(true);
                    assert!(g.advance((60 + flight) as f64));
                    assert_eq!(g.finance.totals, t);
                    assert_eq!(g.finance.recent[0].totals, t);
                    assert_eq!(
                        g.airline.as_ref().unwrap().capital_eur,
                        balance + t.profit()
                    );
                    g.validate_finance().unwrap();
                    assert!(t.fuel > 0 && t.airport > 0 && t.overhead > 0);
                    assert_eq!(t.crew, 0);
                    if price == 0 || price >= 2 * reference {
                        assert!(t.profit() < 0);
                    }
                    if price == 100 && destination == "KLAX" {
                        assert!(t.profit() < 0); // full does not mean profitable
                    }
                    if price >= reference * 19 / 10 {
                        assert!(t.load_percent() < 10.0);
                    }
                    // A same-city airport pair is not a healthy airline market.
                    if origin == "LTFM" {
                        assert!(t.profit() < 0);
                    }
                    // Daily market and frequency, not aircraft capacity, determine
                    // passengers. A reference-price flight need not be profitable.
                    println!(
                        "{origin}-{destination} {} {km:.0}km ticket={price} pax={}/{} fuel={} airport={} crew={} overhead={} cost={} profit={}",
                        model.id,
                        t.passengers,
                        t.seats,
                        t.fuel,
                        t.airport,
                        t.crew,
                        t.overhead,
                        t.costs(),
                        t.profit()
                    );
                }
            }
        }
        let mut g = game(100);
        let viable = quote(&g.routes[0], "atr72_600").unwrap();
        g.routes[0].ticket_eur = 140;
        let overpriced = quote(&g.routes[0], "atr72_600").unwrap();
        assert!(viable.profit() > 0 && overpriced.profit() < 0);
        assert!(overpriced.load_percent() < viable.load_percent() && overpriced.passengers > 0);
    }

    #[test]
    fn historical_v6_costs_and_active_flight_survive_rebalancing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v6.json");
        let mut g = game(100);
        g.advance(7261.0); // first result, return flight already airborne
        crate::save::save(&path, &g).unwrap();
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json["version"] = 6.into();
        json["state"].as_object_mut().unwrap().remove("history");
        json["state"]["finance"]["totals"]
            .as_object_mut()
            .unwrap()
            .remove("overhead");
        for result in json["state"]["finance"]["recent"].as_array_mut().unwrap() {
            result["totals"].as_object_mut().unwrap().remove("overhead");
            // Deliberately old costs, not reproducible by today's quote.
            result["totals"]["fuel"] = 123.into();
        }
        json["state"]["finance"]["totals"]["fuel"] = 123.into();
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.airline, g.airline);
        assert_eq!(loaded.routes, g.routes);
        assert_eq!(loaded.fleet, g.fleet);
        assert_eq!(loaded.finance.totals.overhead, 0);
        assert_eq!(loaded.finance.totals.fuel, 123);
        let historical = loaded.finance.recent[0].clone();
        loaded.advance(
            operations::timings(&loaded.routes[0], "atr72_600")
                .unwrap()
                .0 as f64,
        );
        assert_eq!(loaded.finance.totals.flights, 2);
        assert_eq!(loaded.finance.recent[0], historical);
        assert!(loaded.finance.recent[1].totals.overhead > 0);
        crate::save::save(&path, &loaded).unwrap();
        assert_eq!(crate::save::load(&path).unwrap().finance, loaded.finance);
    }
}
