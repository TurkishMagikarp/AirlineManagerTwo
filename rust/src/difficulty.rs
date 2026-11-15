//! Provisional founding policy only; no AI handicaps or running-game setters.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Low,
    #[default]
    Standard,
    High,
}
impl Level {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "low" => Ok(Self::Low),
            "standard" => Ok(Self::Standard),
            "high" => Ok(Self::High),
            _ => Err("Ungültige Schwierigkeit / invalid difficulty.".into()),
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Standard => "standard",
            Self::High => "high",
        }
    }
    pub fn is_standard(&self) -> bool {
        *self == Self::Standard
    }
    pub fn scale(self, deviation: i64) -> i64 {
        // The legacy world needs no config lookup on the quote hot path.
        if self.is_standard() {
            return deviation;
        }
        deviation * RULES.crisis_percent[self.key()] / 100
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub capital: Level,
    pub competition: Level,
    pub crises: Level,
}
impl Settings {
    pub fn parse(capital: &str, competition: &str, crises: &str) -> Result<Self, String> {
        Ok(Self {
            capital: Level::parse(capital)?,
            competition: Level::parse(competition)?,
            crises: Level::parse(crises)?,
        })
    }
    pub fn is_standard(&self) -> bool {
        *self == Self::default()
    }
}
#[derive(Deserialize)]
pub struct Rules {
    pub starting_capital_eur: i64,
    pub capital_eur: BTreeMap<String, i64>,
    pub competition_hubs: BTreeMap<String, Vec<String>>,
    pub crisis_percent: BTreeMap<String, i64>,
    pub presets: BTreeMap<String, [Level; 3]>,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules =
        serde_json::from_str(include_str!("../../config/new_game.json")).expect("founding config");
    for key in ["low", "standard", "high"] {
        assert!(r.capital_eur[key] >= 0);
        assert!((1..=200).contains(&r.crisis_percent[key]));
        assert!((1..=4).contains(&r.competition_hubs[key].len()));
    }
    assert_eq!(r.capital_eur["standard"], r.starting_capital_eur);
    assert_eq!(r.crisis_percent["standard"], 100);
    assert_eq!(r.presets["standard"], [Level::Standard; 3]);
    r
});
impl crate::game::Game {
    pub fn validate_difficulty(&self) -> Result<(), String> {
        if self.fuel_market.crises != self.difficulty.crises
            || self.world_economy.crises != self.difficulty.crises
        {
            return Err("Widersprüchliche Schwierigkeit.".into());
        }
        Ok(())
    }
}
