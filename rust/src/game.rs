use crate::{airports, clock::Clock};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Game {
    pub clock: Clock,
    pub airline: Option<Airline>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Airline {
    pub name: String,
    pub country_code: String,
    pub home_airport_icao: String,
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
        if self.capital_eur < 0 {
            return Err("Ungültiges Kapital im Spielstand.".into());
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
        *self = Self {
            clock: Clock::default(),
            airline: Some(airline),
        };
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
}
