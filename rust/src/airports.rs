use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Country {
    pub code: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct Airport {
    pub icao: String,
    pub iata: Option<String>,
    pub name: String,
    pub country_code: String,
    pub latitude: f64,
    pub longitude: f64,
    /// Provisional catchment/importance index, not observed route traffic.
    pub market_size: f64,
    pub business_percent: u32,
    /// Percentage of the model landing + passenger tariff, charged at this endpoint.
    pub charge_percent: u32,
    #[serde(default)]
    pub daily_departure_slots: Option<u32>,
}

#[derive(Deserialize)]
pub struct Catalog {
    pub countries: Vec<Country>,
    pub airports: Vec<Airport>,
}

// One catalog, stable ICAO/ISO identifiers, no duplicated lists in Godot or saves.
// A later worldwide data source can replace this loader without changing airline IDs.
static CATALOG: LazyLock<Result<Catalog, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../data/airports.json"))
        .map_err(|_| "Flughafenkatalog konnte nicht gelesen werden.".to_string())
});

pub fn catalog() -> Result<&'static Catalog, String> {
    CATALOG.as_ref().map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_has_unique_identifiers_and_valid_countries() {
        let catalog = catalog().unwrap();
        let mut countries = HashSet::new();
        let mut airports = HashSet::new();
        for country in &catalog.countries {
            assert_eq!(country.code.len(), 2);
            assert!(!country.name.is_empty());
            assert!(countries.insert(&country.code));
        }
        for airport in &catalog.airports {
            assert_eq!(airport.icao.len(), 4);
            assert!(!airport.name.is_empty());
            assert!(airports.insert(&airport.icao));
            assert!(countries.contains(&airport.country_code));
            assert!(airport.latitude.is_finite() && airport.latitude.abs() <= 90.0);
            assert!(airport.longitude.is_finite() && airport.longitude.abs() <= 180.0);
            assert!(
                airport.market_size.is_finite() && (0.01..=10.0).contains(&airport.market_size)
            );
            assert!(airport.daily_departure_slots.is_none_or(|cap| cap > 0));
            assert!(airport.business_percent <= 100);
            assert!((1..=200).contains(&airport.charge_percent));
        }
        assert!(airports.contains(&"EDDF".to_string()));
        assert!(airports.contains(&"LTFM".to_string()));
    }
}
