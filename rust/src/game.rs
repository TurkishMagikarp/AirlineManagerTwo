use crate::{
    aircraft::{self, Aircraft, AircraftStatus},
    airports,
    clock::Clock,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub clock: Clock,
    pub airline: Option<Airline>,
    pub fleet: Vec<Aircraft>,
    pub next_aircraft_id: u32,
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
}

impl Default for Game {
    fn default() -> Self {
        Self {
            clock: Clock::default(),
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

#[derive(Deserialize)]
struct NewGameConfig {
    starting_capital_eur: i64,
}

pub fn starting_capital() -> Result<i64, String> {
    let config: NewGameConfig = serde_json::from_str(include_str!("../../config/new_game.json"))
        .map_err(|_| "Startkapital-Konfiguration ist ungültig.".to_string())?;
    if config.starting_capital_eur < 0 {
        return Err("Startkapital darf nicht negativ sein.".into());
    }
    Ok(config.starting_capital_eur)
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
        let airline = Airline {
            name: name.trim().to_string(),
            country_code: country.to_string(),
            home_airport_icao: airport.to_string(),
            capital_eur: starting_capital()?,
        };
        airline.validate()?;
        let competitors = crate::competition::initial_competitors()?;
        *self = Self {
            competitors,
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
            id: self.next_aircraft_id,
            model_id: model.id.clone(),
            location_icao: airline.home_airport_icao.clone(),
            status: AircraftStatus::Parked,
            purchase_price_eur: model.price_eur,
            acquired_at: Some(self.clock.elapsed().as_secs()),
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
        if self.next_aircraft_id == 0 || (self.airline.is_none() && !self.fleet.is_empty()) {
            return Err("Ungültige Flottendaten im Spielstand.".into());
        }
        let airports = airports::catalog()?;
        let mut ids = HashSet::new();
        for plane in &self.fleet {
            aircraft::model(&plane.model_id)?;
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
