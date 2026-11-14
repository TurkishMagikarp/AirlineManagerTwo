use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub seats: u32,
    pub belly_cargo_kg: u32,
    pub payload_kg: u32,
    pub range_km: u32,
    pub cruise_kmh: u32,
    pub turnaround_minutes: u32,
    #[serde(skip)]
    pub price_eur: i64,
}

static MODELS: LazyLock<Result<Vec<Model>, String>> = LazyLock::new(|| {
    let mut models: Vec<Model> =
        serde_json::from_str(include_str!("../../data/aircraft_models.json"))
            .map_err(|_| "Flugzeugkatalog ist ungültig.".to_string())?;
    let prices: HashMap<String, i64> =
        serde_json::from_str(include_str!("../../config/aircraft_prices.json"))
            .map_err(|_| "Flugzeugpreise sind ungültig.".to_string())?;
    let mut ids = HashSet::new();
    for model in &mut models {
        model.price_eur = *prices
            .get(&model.id)
            .filter(|&&price| price > 0)
            .ok_or_else(|| format!("Gültiger Kaufpreis fehlt für {}.", model.id))?;
        if model.id.is_empty()
            || model.name.is_empty()
            || model.seats == 0
            || model.belly_cargo_kg == 0
            || model.payload_kg < model.belly_cargo_kg
            || model.range_km == 0
            || model.cruise_kmh == 0
            || model.turnaround_minutes == 0
            || !ids.insert(model.id.clone())
        {
            return Err("Flugzeugkatalog enthält ungültige oder doppelte Modelle.".into());
        }
    }
    if models.is_empty() {
        return Err("Flugzeugkatalog ist leer.".into());
    }
    Ok(models)
});

pub fn models() -> Result<&'static [Model], String> {
    MODELS.as_ref().map(Vec::as_slice).map_err(Clone::clone)
}

pub fn model(id: &str) -> Result<&'static Model, String> {
    models()?
        .iter()
        .find(|model| model.id == id)
        .ok_or_else(|| "Unbekanntes Flugzeugmodell.".to_string())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AircraftStatus {
    Parked,
    InFlight,
    Ground,
    /// In the hangar; cannot depart until the check ends.
    Maintenance,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aircraft {
    #[serde(default)]
    pub business_seats: u32,
    pub id: u32,
    pub model_id: String,
    pub location_icao: String,
    pub status: AircraftStatus,
    // Historical amount actually paid, unaffected by subsequent price changes.
    pub purchase_price_eur: i64,
    /// Simulation seconds since founding; version 13.
    #[serde(default)]
    pub acquired_at: Option<u64>,
    /// Date paid by the current owner; original acquisition remains the age origin.
    #[serde(default)]
    pub purchased_at: Option<u64>,
    #[serde(default)]
    pub lease: Option<crate::trading::Lease>,
    /// Additive (version 10): older saves start without recorded wear.
    #[serde(default)]
    pub wear: crate::maintenance::Wear,
    #[serde(default)]
    pub maintenance: Option<crate::maintenance::Visit>,
}

impl Aircraft {
    pub fn identifier(&self) -> String {
        format!("AM-{:06}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_model_catalog_and_configured_prices_are_valid() {
        assert_eq!(models().unwrap().len(), 3);
        assert_eq!(model("a320_200").unwrap().seats, 180);
        assert_eq!(model("b737_800").unwrap().seats, 189);
        assert_eq!(model("atr72_600").unwrap().range_km, 1370);
        for model in models().unwrap() {
            assert!(model.price_eur > 0);
        }
        assert!(model("not_a_plane").is_err());
    }
}
