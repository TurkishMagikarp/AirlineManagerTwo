use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "display_class")]
    pub class: String,
    pub seats: u32,
    pub belly_cargo_kg: u32,
    #[serde(default)]
    pub main_deck_cargo_kg: u32,
    pub payload_kg: u32,
    pub range_km: u32,
    pub cruise_kmh: u32,
    pub turnaround_minutes: u32,
    #[serde(skip)]
    pub price_eur: i64,
    /// Elapsed game day (0 = start) from which the model can be ordered, bought or leased.
    #[serde(skip)]
    pub entry_day: u64,
    /// First elapsed game day on which new units can no longer be ordered or bought.
    #[serde(skip)]
    pub end_of_production_day: Option<u64>,
    /// Day on which the successor enters the market; used aircraft lose value faster from then on.
    #[serde(skip)]
    pub obsolete_from_day: Option<u64>,
}

const DAY: u64 = 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Upcoming { from_day: u64 },
    OnSale,
    OutOfProduction { since_day: u64 },
}

impl Model {
    pub fn availability(&self, now: u64) -> Availability {
        let day = now / DAY;
        if day < self.entry_day {
            Availability::Upcoming {
                from_day: self.entry_day,
            }
        } else if let Some(end) = self.end_of_production_day.filter(|&end| day >= end) {
            Availability::OutOfProduction { since_day: end }
        } else {
            Availability::OnSale
        }
    }

    /// New units can be ordered or bought from the manufacturer.
    pub fn orderable(&self, now: u64) -> bool {
        self.availability(now) == Availability::OnSale
    }

    /// Leasing stays possible after production ends, but not before the entry date.
    pub fn leasable(&self, now: u64) -> bool {
        !matches!(self.availability(now), Availability::Upcoming { .. })
    }

    pub fn require_orderable(&self, now: u64) -> Result<(), String> {
        match self.availability(now) {
            Availability::OnSale => Ok(()),
            Availability::Upcoming { from_day } => Err(format!(
                "{} ist erst ab Tag {} verfügbar.",
                self.name,
                from_day + 1
            )),
            Availability::OutOfProduction { .. } => {
                Err(format!("{} wird nicht mehr produziert.", self.name))
            }
        }
    }

    pub fn require_leasable(&self, now: u64) -> Result<(), String> {
        match self.availability(now) {
            Availability::Upcoming { from_day } => Err(format!(
                "{} ist erst ab Tag {} verfügbar.",
                self.name,
                from_day + 1
            )),
            _ => Ok(()),
        }
    }
}

#[derive(Deserialize)]
pub struct Lifecycle {
    #[serde(default)]
    pub entry_day: u64,
    pub end_of_production_day: Option<u64>,
    pub successor: Option<String>,
}

#[derive(Deserialize)]
pub struct LifecycleRules {
    /// Days before an entry or end-of-production date on which the attention note appears.
    pub announce_days: u64,
    /// Days the note stays after the event.
    pub notice_visible_days: u64,
    pub models: HashMap<String, Lifecycle>,
}

pub static LIFECYCLE: LazyLock<LifecycleRules> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../config/aircraft_lifecycle.json"))
        .expect("aircraft lifecycle config")
});

fn display_class<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let class = String::deserialize(deserializer)?;
    if class.trim().is_empty() {
        return Err(serde::de::Error::custom("Flugzeugklasse fehlt."));
    }
    Ok(class)
}

static MODELS: LazyLock<Result<Vec<Model>, String>> = LazyLock::new(|| {
    let mut models: Vec<Model> =
        serde_json::from_str(include_str!("../../data/aircraft_models.json"))
            .map_err(|_| "Flugzeugkatalog ist ungültig.".to_string())?;
    let prices: HashMap<String, i64> =
        serde_json::from_str(include_str!("../../config/aircraft_prices.json"))
            .map_err(|_| "Flugzeugpreise sind ungültig.".to_string())?;
    let lifecycle = &*LIFECYCLE;
    if lifecycle.announce_days == 0 || lifecycle.notice_visible_days == 0 {
        return Err("Flugzeug-Lebenszyklus ist ungültig.".into());
    }
    let entry_days: HashMap<&str, u64> = models
        .iter()
        .map(|m| {
            (
                m.id.as_str(),
                lifecycle.models.get(&m.id).map_or(0, |l| l.entry_day),
            )
        })
        .collect();
    if lifecycle
        .models
        .keys()
        .any(|id| !entry_days.contains_key(id.as_str()))
    {
        return Err("Flugzeug-Lebenszyklus nennt ein unbekanntes Modell.".into());
    }
    let mut obsolete = HashMap::new();
    for (id, l) in &lifecycle.models {
        let successor_entry = match &l.successor {
            Some(s) => Some(
                *entry_days
                    .get(s.as_str())
                    .ok_or("Nachfolgemodell unbekannt.")?,
            ),
            None => None,
        };
        if l.end_of_production_day
            .is_some_and(|end| end <= l.entry_day)
            || successor_entry.is_some_and(|e| e <= l.entry_day)
            || l.successor.as_deref() == Some(id.as_str())
        {
            return Err("Flugzeug-Lebenszyklus enthält ungültige Termine.".into());
        }
        obsolete.insert(id.clone(), successor_entry);
    }
    let mut ids = HashSet::new();
    for model in &mut models {
        if let Some(l) = lifecycle.models.get(&model.id) {
            model.entry_day = l.entry_day;
            model.end_of_production_day = l.end_of_production_day;
            model.obsolete_from_day = obsolete[&model.id];
        }
        model.price_eur = *prices
            .get(&model.id)
            .filter(|&&price| price > 0)
            .ok_or_else(|| format!("Gültiger Kaufpreis fehlt für {}.", model.id))?;
        if model.id.is_empty()
            || model.name.is_empty()
            || model.class.trim().is_empty()
            || (model.seats == 0) != (model.main_deck_cargo_kg > 0)
            || (model.seats > 0 && model.belly_cargo_kg == 0)
            || model.payload_kg < model.main_deck_cargo_kg
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
    fn display_class_is_required_and_nonempty() {
        let catalog: serde_json::Value =
            serde_json::from_str(include_str!("../../data/aircraft_models.json")).unwrap();
        for class in [
            serde_json::Value::Null,
            serde_json::json!(""),
            serde_json::json!("   "),
        ] {
            let mut value = catalog[0].clone();
            value["class"] = class;
            assert!(serde_json::from_value::<Model>(value).is_err());
        }
        let mut value = catalog[0].clone();
        value.as_object_mut().unwrap().remove("class");
        assert!(serde_json::from_value::<Model>(value).is_err());
    }

    #[test]
    fn real_model_catalog_and_configured_prices_are_valid() {
        assert_eq!(models().unwrap().len(), 13);
        assert_eq!(model("a320_200").unwrap().seats, 180);
        assert_eq!(model("b737_800").unwrap().seats, 189);
        assert_eq!(model("atr72_600").unwrap().range_km, 1370);
        for model in models().unwrap() {
            assert!(model.price_eur > 0);
        }
        assert_eq!(model("a320_200").unwrap().class, "Narrowbody");
        assert_eq!(model("b737_800").unwrap().class, "Narrowbody");
        assert_eq!(model("atr72_600").unwrap().class, "Turboprop");
        assert!(model("not_a_plane").is_err());
    }
}
