//! Provisional deterministic balancing. Whole EUR, rounded up for hourly costs.
use crate::{aircraft, game::Game, operations, routes::Route};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::LazyLock};

pub const HISTORY_LIMIT: usize = 100;
#[derive(Deserialize)]
pub struct Config {
    pub default_ticket_eur: i64,
    base_demand: i64,
    demand_per_1000_km: i64,
    reference_base_eur: i64,
    reference_eur_per_100_km: i64,
    fuel_eur_per_kg: i64,
    airport_base_eur: i64,
    airport_eur_per_passenger: i64,
    models: HashMap<String, Rates>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(price: i64) -> Game {
        let mut g = Game::default();
        g.found("Economy Air", "DE", "EDDF").unwrap();
        g.buy_aircraft("atr72_600").unwrap();
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
        g.routes[0].ticket_eur = 150;
        let middle = quote(&g.routes[0], "atr72_600").unwrap();
        assert_eq!(middle.passengers, 61);
        assert!(middle.passengers > 0 && middle.passengers < middle.seats);
        assert_eq!(middle.revenue, middle.passengers * 150);
        assert!(middle.profit() > 0);
        g.routes[0].ticket_eur = 100000;
        let empty = quote(&g.routes[0], "atr72_600").unwrap();
        assert_eq!(empty.passengers, 0);
        assert!(empty.fuel > 0 && empty.airport > 0 && empty.crew > 0);
        assert_eq!(empty.fuel, middle.fuel);
        assert_eq!(empty.crew, middle.crew);
        assert!(empty.airport < middle.airport);
        let short = quote(&g.routes[0], "a320_200").unwrap();
        g.routes[0].destination_icao = "LTFM".into();
        let long = quote(&g.routes[0], "a320_200").unwrap();
        assert!(long.fuel > short.fuel && long.crew > short.crew);
        g.routes[0].ticket_eur = 150;
        assert!(quote(&g.routes[0], "a320_200").unwrap().passengers > middle.passengers);
    }
    #[test]
    fn landing_debits_once_empty_flights_can_create_debt_and_prices_validate() {
        let mut g = game(100000);
        assert!(g.set_ticket_price(1, 20).is_err());
        let flight = operations::timings(&g.routes[0], "atr72_600").unwrap().0;
        g.airline.as_mut().unwrap().capital_eur = 0;
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
        let mut g = game(150);
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
}
#[derive(Deserialize)]
struct Rates {
    fuel_kg_per_hour: i64,
    crew_eur_per_hour: i64,
}
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let c: Config =
        serde_json::from_str(include_str!("../../config/economy.json")).expect("economy config");
    assert!((0..=100_000).contains(&c.default_ticket_eur));
    for n in [
        c.base_demand,
        c.demand_per_1000_km,
        c.reference_base_eur,
        c.reference_eur_per_100_km,
        c.fuel_eur_per_kg,
        c.airport_base_eur,
        c.airport_eur_per_passenger,
    ] {
        assert!((0..=100_000).contains(&n));
    }
    assert!(c.reference_base_eur > 0);
    for m in aircraft::models().expect("models") {
        let r = &c.models[&m.id];
        assert!(
            (0..=100_000).contains(&r.fuel_kg_per_hour)
                && (0..=100_000).contains(&r.crew_eur_per_hour)
        );
    }
    c
});
pub fn default_ticket() -> i64 {
    CONFIG.default_ticket_eur
}

pub fn rules_text() -> String {
    let c = &*CONFIG;
    format!(
        "Vorläufige Nachfrage: Basis = {} + {} × km / 1000; Referenzpreis = {} + {} × km / 100 EUR.\nNachfrage = Basis × max(0, 2 − Preis / Referenzpreis), begrenzt auf Sitzplätze.\nStrecke auf ganze km gerundet; Basis, Referenzpreis und Passagiere ganzzahlig abgerundet.\nKosten: Treibstoff ({} EUR/kg), Flughäfen ({} EUR + {} EUR/Passagier insgesamt) und Personal nach Flugzeit.\nAbrechnung bei Landung, auch bei leeren Flügen. Modellwerte: config/economy.json.",
        c.base_demand,
        c.demand_per_1000_km,
        c.reference_base_eur,
        c.reference_eur_per_100_km,
        c.fuel_eur_per_kg,
        c.airport_base_eur,
        c.airport_eur_per_passenger
    )
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
}
impl Totals {
    pub fn costs(&self) -> i64 {
        self.fuel + self.airport + self.crew
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
        add!(flights, passengers, seats, revenue, fuel, airport, crew);
        self.fuel
            .checked_add(self.airport)?
            .checked_add(self.crew)?;
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

pub fn quote(route: &Route, model_id: &str) -> Result<Totals, String> {
    let c = &*CONFIG;
    let model = aircraft::model(model_id)?;
    let km =
        crate::routes::distance_km(&route.origin_icao, &route.destination_icao)?.round() as i64;
    let reference = c.reference_base_eur + km * c.reference_eur_per_100_km / 100;
    let base = c.base_demand + km * c.demand_per_1000_km / 1000;
    let demand = base * (2 * reference - route.ticket_eur).max(0) / reference;
    let passengers = demand.min(i64::from(model.seats));
    let seconds = operations::timings(route, model_id)?.0 as i64;
    let rates = &c.models[model_id];
    Ok(Totals {
        flights: 1,
        passengers,
        seats: model.seats.into(),
        revenue: passengers * route.ticket_eur,
        fuel: (seconds * rates.fuel_kg_per_hour * c.fuel_eur_per_kg + 3599) / 3600,
        airport: c.airport_base_eur + c.airport_eur_per_passenger * passengers,
        crew: (seconds * rates.crew_eur_per_hour + 3599) / 3600,
    })
}

impl Game {
    pub fn set_ticket_price(&mut self, id: u32, price: i64) -> Result<(), String> {
        self.require_no_plan(id)?;
        if !(0..=100_000).contains(&price) {
            return Err("Ticketpreis muss zwischen 0 und 100000 EUR liegen.".into());
        }
        self.routes
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or("Unbekannte Route.")?
            .ticket_eur = price;
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
                && t.fuel
                    .checked_add(t.airport)
                    .and_then(|v| v.checked_add(t.crew))
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
        {
            return Err("Widersprüchliche Finanzsummen.".into());
        }
        Ok(())
    }
}
