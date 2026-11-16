//! Airline-wide fees; quoted with the operating carrier, frozen at departure.
use crate::{economy::Totals, game::Game};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    #[default]
    Included,
    Standard,
    Aggressive,
}
#[derive(Deserialize)]
pub struct Rate {
    pub economy_eur: i64,
    pub business_eur: i64,
    pub attractiveness: f64,
}
#[derive(Deserialize)]
pub struct Rules {
    included: Rate,
    standard: Rate,
    aggressive: Rate,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/ancillaries.json"))
        .expect("ancillaries config");
    assert!(
        r.included.economy_eur == 0
            && r.included.business_eur == 0
            && r.included.attractiveness == 1.0
    );
    for rate in [&r.standard, &r.aggressive] {
        assert!((1..=100).contains(&rate.economy_eur));
        assert!((1..=100).contains(&rate.business_eur));
        assert!((0.8..1.0).contains(&rate.attractiveness));
    }
    assert!(r.aggressive.economy_eur > r.standard.economy_eur);
    assert!(r.aggressive.business_eur > r.standard.business_eur);
    assert!(r.aggressive.attractiveness < r.standard.attractiveness);
    r
});
impl Policy {
    pub fn name(self) -> &'static str {
        match self {
            Self::Included => "included",
            Self::Standard => "standard",
            Self::Aggressive => "aggressive",
        }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "included" => Ok(Self::Included),
            "standard" => Ok(Self::Standard),
            "aggressive" => Ok(Self::Aggressive),
            _ => Err("Unbekannte Zusatzgebühren.".into()),
        }
    }
    pub fn rate(self) -> &'static Rate {
        match self {
            Self::Included => &RULES.included,
            Self::Standard => &RULES.standard,
            Self::Aggressive => &RULES.aggressive,
        }
    }
    #[inline]
    pub fn revenue(self, passengers: i64, business: i64) -> i64 {
        if self == Self::Included {
            return 0;
        }
        let r = self.rate();
        (passengers - business) * r.economy_eur + business * r.business_eur
    }
    /// Preserve the legacy floating-point operation order for Included.
    #[inline]
    pub fn weight(self, weight: f64) -> f64 {
        if self == Self::Included {
            weight
        } else {
            weight * self.rate().attractiveness
        }
    }
}
/// Schema bounds, independent of future config tuning and safe for aggregate saves.
pub fn valid(t: &Totals) -> bool {
    t.ancillary >= 0
        && t.ancillary <= t.revenue
        && i128::from(t.ancillary) <= i128::from(t.passengers) * 100
}
impl Game {
    pub fn set_ancillary_policy(&mut self, policy: Policy) -> Result<(), String> {
        self.require_active()?;
        if self.ancillary_policy != policy {
            self.ancillary_policy = policy;
            for route in &mut self.routes {
                route.ancillary_policy = policy;
            }
            self.commercial_changed();
        }
        Ok(())
    }
    /// Stable weekly choice: fees only while cash is negative. No RNG or fare edits.
    pub fn review_ancillaries(&mut self) {
        #[cfg(test)]
        if LEGACY_AI.with(|v| v.get()) {
            return;
        }
        let policy = if self.cash() < 0 {
            Policy::Standard
        } else {
            Policy::Included
        };
        let _ = self.set_ancillary_policy(policy);
    }
}
#[cfg(test)]
thread_local! { pub(crate) static LEGACY_AI: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
/// v40 has no fees, including every frozen result and every AI owner.
pub fn migrate(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(o) => {
            if o.contains_key("routes") || o.contains_key("origin_icao") {
                o.insert("ancillary_policy".into(), "included".into());
            }
            if o.contains_key("fuel") && o.contains_key("revenue") {
                o.insert("ancillary".into(), 0.into());
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
