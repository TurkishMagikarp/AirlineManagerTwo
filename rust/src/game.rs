use crate::{
    aircraft::{self, Aircraft, AircraftStatus},
    airports,
    clock::Clock,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    #[serde(default)]
    pub ancillary_policy: crate::ancillaries::Policy,
    pub loyalty: crate::loyalty::Loyalty,
    #[serde(default)]
    pub orders: Vec<crate::orders::Order>,
    #[serde(default = "initial_order_id")]
    pub next_order_id: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub codeshares: Vec<crate::codeshare::Agreement>,
    #[serde(default, skip_serializing_if = "crate::alliances::State::is_empty")]
    pub alliances: crate::alliances::State,
    #[serde(
        default,
        skip_serializing_if = "crate::difficulty::Settings::is_standard"
    )]
    pub difficulty: crate::difficulty::Settings,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pricing: Vec<crate::pricing::Autopilot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestones: Option<crate::milestones::State>,
    /// Unsaved dependency epoch. Unique epochs also distinguish diverging clones.
    #[serde(skip)]
    pub(crate) commercial_revision: u64,
    #[serde(skip)]
    pub(crate) reputation_revision: u64,
    /// Derived dispatch markets survive frame/event boundaries. Unsaved: a load
    /// reconstructs them from commercial state, never from airborne results.
    #[serde(skip)]
    pub(crate) departure_markets: crate::economy::DepartureMarkets,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ferries: Vec<crate::operations::Ferry>,
    #[serde(default)]
    pub used_market: crate::trading::Market,
    /// Derived snapshot of other owners, refreshed before each AI review.
    #[serde(skip)]
    pub external_slots: std::collections::BTreeMap<String, u64>,
    #[serde(default)]
    pub maintenance_bases: Vec<crate::maintenance::Base>,
    pub insurance: crate::insurance::Policy,
    pub fuel_contract: Option<crate::fuel_contracts::Contract>,
    pub clock: Clock,
    #[serde(default)]
    pub fuel_market: crate::economy::FuelMarket,
    #[serde(default)]
    pub world_economy: crate::world_economy::Cycle,
    pub airline: Option<Airline>,
    pub fleet: Vec<Aircraft>,
    pub next_aircraft_id: u32,
    #[serde(with = "crate::route_save")]
    pub routes: Vec<crate::routes::Route>,
    pub next_route_id: u32,
    pub finance: crate::economy::Finance,
    pub history: Option<crate::history::History>,
    #[serde(default)]
    pub competitors: Vec<crate::competition::Competitor>,
    #[serde(default)]
    pub staff: crate::staff::Staff,
    #[serde(default)]
    pub financing: crate::financing::Financing,
    #[serde(default)]
    pub tax: crate::tax::Tax,
    pub marketing: crate::marketing::Marketing,
    #[serde(default)]
    pub reputation: crate::reputation::Reputation,
    #[serde(default)]
    pub resolution: crate::financing::Resolution,
}

pub(crate) fn initial_order_id() -> u64 {
    1
}

fn next_market_revision() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
impl Game {
    pub(crate) fn commercial_changed(&mut self) {
        self.commercial_revision = next_market_revision();
    }
    pub(crate) fn reputation_changed(&mut self) {
        self.reputation_revision = next_market_revision();
    }
    pub(crate) fn supply_revision(&self) -> u64 {
        std::iter::once(self)
            .chain(self.competitors.iter().map(|c| &*c.game))
            .map(|g| g.commercial_revision.max(g.reputation_revision))
            .max()
            .unwrap()
    }
    pub(crate) fn supply_structure_revision(&self) -> u64 {
        self.competitors
            .iter()
            .map(|c| c.game.commercial_revision)
            .fold(self.commercial_revision, u64::max)
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            ancillary_policy: Default::default(),
            loyalty: Default::default(),
            orders: Vec::new(),
            next_order_id: 1,
            codeshares: Vec::new(),
            alliances: Default::default(),
            difficulty: Default::default(),
            pricing: Vec::new(),
            milestones: None,
            departure_markets: Default::default(),
            commercial_revision: 0,
            reputation_revision: 0,
            ferries: Vec::new(),
            used_market: Default::default(),
            external_slots: Default::default(),
            maintenance_bases: Vec::new(),
            insurance: Default::default(),
            fuel_contract: None,
            clock: Clock::default(),
            fuel_market: Default::default(),
            world_economy: Default::default(),
            airline: None,
            fleet: Vec::new(),
            next_aircraft_id: 1,
            routes: Vec::new(),
            next_route_id: 1,
            finance: crate::economy::Finance::default(),
            history: None,
            competitors: Vec::new(),
            staff: crate::staff::Staff::default(),
            financing: Default::default(),
            tax: Default::default(),
            marketing: Default::default(),
            reputation: Default::default(),
            resolution: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Airline {
    pub name: String,
    pub country_code: String,
    pub home_airport_icao: String,
    // Current balance (may be negative after operating losses); legacy field name.
    pub capital_eur: i64,
}

pub fn starting_capital() -> Result<i64, String> {
    Ok(crate::difficulty::RULES.starting_capital_eur)
}

impl Airline {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name != self.name.trim()
            || self.name.chars().count() > 80
            || self.name.chars().any(char::is_control)
        {
            return Err(
                "Bitte einen Airline-Namen mit 1 bis 80 Zeichen ohne Steuerzeichen eingeben."
                    .into(),
            );
        }
        let catalog = airports::catalog()?;
        if !catalog
            .countries
            .iter()
            .any(|c| c.code == self.country_code)
        {
            return Err("Bitte ein verfügbares Heimatland auswählen.".into());
        }
        let airport = catalog
            .airports
            .iter()
            .find(|a| a.icao == self.home_airport_icao)
            .ok_or_else(|| "Bitte einen verfügbaren Heimatflughafen auswählen.".to_string())?;
        if airport.country_code != self.country_code {
            return Err("Der Heimatflughafen muss im gewählten Heimatland liegen.".into());
        }
        Ok(())
    }
}

impl Game {
    /// Build and validate everything before replacing the existing game.
    pub fn found(&mut self, name: &str, country: &str, airport: &str) -> Result<(), String> {
        self.found_with_levels(name, country, airport, "standard", "standard", "standard")
    }

    pub fn found_with_levels(
        &mut self,
        name: &str,
        country: &str,
        airport: &str,
        capital: &str,
        competition: &str,
        crises: &str,
    ) -> Result<(), String> {
        let difficulty = crate::difficulty::Settings::parse(capital, competition, crises)?;
        self.require_active()?;
        let airline = Airline {
            name: name.trim().to_string(),
            country_code: country.to_string(),
            home_airport_icao: airport.to_string(),
            capital_eur: crate::difficulty::RULES.capital_eur[difficulty.capital.key()],
        };
        airline.validate()?;
        crate::milestones::rules()?;
        let mut competitors = if difficulty.competition == crate::difficulty::Level::Standard {
            crate::competition::new_competitors(airport)?
        } else {
            crate::competition::new_competitors_with_level(airport, difficulty.competition)?
        };
        let fuel_market = crate::economy::FuelMarket {
            crises: difficulty.crises,
            ..Default::default()
        };
        let world_economy = crate::world_economy::Cycle {
            crises: difficulty.crises,
            ..Default::default()
        };
        for c in &mut competitors {
            c.game.difficulty = difficulty;
            c.game.fuel_market = fuel_market.clone();
            c.game.world_economy = world_economy;
        }
        *self = Self {
            difficulty,
            fuel_market,
            world_economy,
            competitors,
            milestones: Some(Default::default()),
            history: Some(crate::history::History::new(0, airline.capital_eur)),
            airline: Some(airline),
            ..Self::default()
        };
        Ok(())
    }

    /// All checks precede the single synchronous purchase: no debit or ID on failure.
    pub fn buy_aircraft(&mut self, model_id: &str) -> Result<(), String> {
        self.require_solvent()?;
        let airline = self
            .airline
            .as_mut()
            .ok_or_else(|| "Bitte zuerst eine Airline gründen.".to_string())?;
        let model = aircraft::model(model_id)?;
        if airline.capital_eur < model.price_eur {
            return Err(format!(
                "Nicht genügend Guthaben: {} EUR benötigt, {} EUR verfügbar.",
                model.price_eur, airline.capital_eur
            ));
        }
        let next_id = self
            .next_aircraft_id
            .checked_add(1)
            .ok_or_else(|| "Keine weitere Flugzeugkennung verfügbar.".to_string())?;
        let plane = Aircraft {
            business_seats: 0,
            id: self.next_aircraft_id,
            model_id: model.id.clone(),
            location_icao: airline.home_airport_icao.clone(),
            status: AircraftStatus::Parked,
            purchase_price_eur: model.price_eur,
            acquired_at: Some(self.clock.elapsed().as_secs()),
            purchased_at: None,
            lease: None,
            wear: Default::default(),
            maintenance: None,
        };
        self.fleet.push(plane);
        if let Some(h) = &mut self.history {
            h.purchases.push(crate::history::Purchase {
                time: self.clock.elapsed().as_secs(),
                amount: model.price_eur,
            });
        }
        airline.capital_eur -= model.price_eur;
        self.next_aircraft_id = next_id;
        Ok(())
    }

    pub fn validate_fleet(&self) -> Result<(), String> {
        self.validate_trading()?;
        self.validate_orders()?;
        if self.next_aircraft_id == 0 || (self.airline.is_none() && !self.fleet.is_empty()) {
            return Err("Ungültige Flottendaten im Spielstand.".into());
        }
        let airports = airports::catalog()?;
        let mut ids = HashSet::new();
        for plane in &self.fleet {
            crate::cabins::layout(plane)?;
            if plane.id == 0
                || plane.id >= self.next_aircraft_id
                || !ids.insert(plane.id)
                || plane.purchase_price_eur <= 0
                || !airports
                    .airports
                    .iter()
                    .any(|airport| airport.icao == plane.location_icao)
            {
                return Err(
                    "Ungültige Flugzeugkennung, Kaufdaten oder Standort im Spielstand.".into(),
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn founding_creates_valid_company_with_configured_capital_and_fresh_clock() {
        let mut game = Game::default();
        game.clock.set_running(true);
        game.clock.advance(50.0);
        game.found("  Anadolu 日本 Air  ", "TR", "LTFM").unwrap();
        let airline = game.airline.as_ref().unwrap();
        assert_eq!(airline.name, "Anadolu 日本 Air");
        assert_eq!(airline.home_airport_icao, "LTFM");
        assert_eq!(airline.capital_eur, starting_capital().unwrap());
        assert!(!game.clock.is_running());
        assert_eq!(game.clock.elapsed().as_secs(), 0);
        assert_eq!(game.clock.speed(), 1);
    }

    #[test]
    fn invalid_founding_does_not_replace_existing_company_or_time() {
        let mut game = Game::default();
        game.found("Original", "DE", "EDDF").unwrap();
        game.clock.set_running(true);
        game.clock.advance(12.0);
        let original = game.airline.clone();
        for (name, country, airport) in [
            ("   ", "DE", "EDDF"),
            ("bad\nname", "DE", "EDDF"),
            ("Air", "XX", "EDDF"),
            ("Air", "TR", "EDDF"),
            ("Air", "DE", "XXXX"),
            (&"x".repeat(81), "DE", "EDDF"),
        ] {
            assert!(game.found(name, country, airport).is_err());
            assert_eq!(game.airline, original);
            assert_eq!(game.clock.elapsed().as_secs(), 12);
            assert!(game.clock.is_running());
        }
    }

    #[test]
    fn purchases_debit_exact_prices_and_create_individual_home_based_aircraft() {
        let mut game = Game::default();
        game.found("Fleet Air", "DE", "EDDM").unwrap();
        let total: i64 = aircraft::models()
            .unwrap()
            .iter()
            .map(|model| model.price_eur)
            .sum();
        game.airline.as_mut().unwrap().capital_eur = total;
        for model in aircraft::models().unwrap() {
            game.buy_aircraft(&model.id).unwrap();
            let plane = game.fleet.last().unwrap();
            assert_eq!(plane.model_id, model.id);
            assert_eq!(plane.purchase_price_eur, model.price_eur);
            assert_eq!(plane.location_icao, "EDDM");
            assert_eq!(plane.status, AircraftStatus::Parked);
        }
        assert_eq!(game.airline.as_ref().unwrap().capital_eur, 0);
        assert_eq!(game.fleet[0].identifier(), "AM-000001");
        assert_eq!(game.fleet[2].identifier(), "AM-000003");
        game.validate_fleet().unwrap();
    }

    #[test]
    fn failed_purchases_do_not_change_money_fleet_or_identifier_sequence() {
        let mut game = Game::default();
        assert!(game.buy_aircraft("atr72_600").is_err());
        game.found("Fleet Air", "TR", "LTFM").unwrap();
        let price = aircraft::model("atr72_600").unwrap().price_eur;
        game.airline.as_mut().unwrap().capital_eur = price - 1;
        for id in ["atr72_600", "unknown"] {
            assert!(game.buy_aircraft(id).is_err());
            assert_eq!(game.airline.as_ref().unwrap().capital_eur, price - 1);
            assert!(game.fleet.is_empty());
            assert_eq!(game.next_aircraft_id, 1);
        }
        game.airline.as_mut().unwrap().capital_eur = price * 2;
        game.buy_aircraft("atr72_600").unwrap();
        game.buy_aircraft("atr72_600").unwrap();
        assert_eq!(game.fleet[0].id, 1);
        assert_eq!(game.fleet[1].id, 2);
        assert_eq!(game.airline.as_ref().unwrap().capital_eur, 0);
        assert!(game.buy_aircraft("atr72_600").is_err());
        assert_eq!(game.next_aircraft_id, 3);
        game.next_aircraft_id = u32::MAX;
        game.airline.as_mut().unwrap().capital_eur = price;
        assert!(game.buy_aircraft("atr72_600").is_err());
        assert_eq!(game.airline.as_ref().unwrap().capital_eur, price);
        assert_eq!(game.fleet.len(), 2);
        game.found("New Air", "DE", "EDDF").unwrap();
        assert!(game.fleet.is_empty());
        assert_eq!(game.next_aircraft_id, 1);
    }
}
