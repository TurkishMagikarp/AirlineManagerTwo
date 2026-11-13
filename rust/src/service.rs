//! Incremental onboard costs relative to the neutral, legacy Standard service.
use crate::{economy::Totals, game::Game};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Basic,
    #[default]
    Standard,
    Premium,
}
#[derive(Deserialize)]
pub struct Rate {
    pub economy_eur: i64,
    pub business_eur: i64,
    pub attractiveness: f64,
}
#[derive(Deserialize)]
pub struct Rules {
    basic: Rate,
    standard: Rate,
    premium: Rate,
    pub ai_business_premium_min_eur: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules =
        serde_json::from_str(include_str!("../../config/service.json")).expect("service config");
    assert!(
        r.standard.economy_eur == 0
            && r.standard.business_eur == 0
            && r.standard.attractiveness == 1.0
    );
    assert!((-10..0).contains(&r.basic.economy_eur));
    assert!((-50..r.basic.economy_eur).contains(&r.basic.business_eur));
    assert!((1..=20).contains(&r.premium.economy_eur));
    assert!((r.premium.economy_eur + 1..=100).contains(&r.premium.business_eur));
    assert!((0.8..1.0).contains(&r.basic.attractiveness));
    assert!((1.0..=1.2).contains(&r.premium.attractiveness));
    assert!(r.ai_business_premium_min_eur > 0);
    r
});
impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Standard => "standard",
            Self::Premium => "premium",
        }
    }
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "basic" => Ok(Self::Basic),
            "standard" => Ok(Self::Standard),
            "premium" => Ok(Self::Premium),
            _ => Err("Unbekannter Bordservice.".into()),
        }
    }
    pub fn rate(self) -> &'static Rate {
        match self {
            Self::Basic => &RULES.basic,
            Self::Standard => &RULES.standard,
            Self::Premium => &RULES.premium,
        }
    }
    pub fn cost(self, passengers: i64, business: i64) -> i64 {
        (passengers - business) * self.rate().economy_eur + business * self.rate().business_eur
    }
}
/// Signed savings are valid within the schema limits used by config validation.
/// Do not use current rates: later tuning must preserve frozen historical costs.
/// Use wide arithmetic even for corrupt aggregate totals loaded from disk.
pub fn valid(t: &Totals) -> bool {
    let economy = i128::from(t.passengers) - i128::from(t.business_passengers);
    let business = i128::from(t.business_passengers);
    let cost = i128::from(t.service);
    economy >= 0
        && business >= 0
        && cost >= economy * -10 + business * -50
        && cost <= economy * 20 + business * 100
}
impl Game {
    pub fn set_service_level(&mut self, id: u32, level: Level) -> Result<(), String> {
        self.require_active()?;
        if !self
            .routes
            .iter()
            .any(|r| r.id == id && r.parent_route_id.is_none())
        {
            return Err("Unbekannte Route.".into());
        }
        for r in self.routes.iter_mut().filter(|r| r.public_id() == id) {
            r.service_level = level;
        }
        Ok(())
    }
    pub fn review_service(&mut self) {
        let changes: Vec<_> = self
            .routes
            .iter()
            .filter(|r| r.parent_route_id.is_none())
            .map(|r| {
                let business = self
                    .routes
                    .iter()
                    .filter(|s| s.public_id() == r.id)
                    .any(|s| {
                        self.fleet
                            .iter()
                            .any(|p| Some(p.id) == s.aircraft_id && p.business_seats > 0)
                    });
                let level = if business
                    && r.business_ticket_eur - r.ticket_eur >= RULES.ai_business_premium_min_eur
                {
                    Level::Premium
                } else {
                    Level::Standard
                };
                (r.id, level)
            })
            .collect();
        for (id, level) in changes {
            let _ = self.set_service_level(id, level);
        }
    }
}
/// v18 introduces required service fields for all owners, archives and snapshots.
pub fn migrate(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(o) => {
            if o.contains_key("origin_icao") && o.contains_key("destination_icao") {
                o.insert("service_level".into(), "standard".into());
            }
            if o.contains_key("fuel") && o.contains_key("airport") && o.contains_key("crew") {
                o.insert("service".into(), 0.into());
            }
            for v in o.values_mut() {
                migrate(v);
            }
        }
        serde_json::Value::Array(a) => {
            for v in a {
                migrate(v);
            }
        }
        _ => {}
    }
}
