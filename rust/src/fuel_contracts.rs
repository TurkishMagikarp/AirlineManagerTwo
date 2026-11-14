//! Fixed-price supply, no storage, upfront payment or early cancellation.
use crate::game::Game;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
#[derive(Deserialize)]
pub struct Rules {
    pub premium_bps: i64,
    pub shares_percent: Vec<u32>,
    pub terms_months: Vec<u32>,
    pub month_days: u64,
    pub ai_share_percent: u32,
    pub ai_term_months: u32,
    pub ai_reserve_eur: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/fuel_contracts.json"))
        .expect("fuel contracts config");
    assert!((0..=1000).contains(&r.premium_bps));
    assert!(!r.shares_percent.is_empty() && r.shares_percent.iter().all(|s| (1..100).contains(s)));
    assert!(!r.terms_months.is_empty() && r.terms_months.iter().all(|t| (1..=120).contains(t)));
    assert!((1..=365).contains(&r.month_days));
    assert!(
        r.shares_percent.contains(&r.ai_share_percent)
            && r.terms_months.contains(&r.ai_term_months)
    );
    assert!(r.ai_reserve_eur > 0);
    r
});
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contract {
    pub started_at: u64,
    pub ends_at: u64,
    pub share_percent: u32,
    pub months: u32,
    pub locked_cents: i64,
}
impl Contract {
    pub fn active_at(&self, time: u64) -> bool {
        self.started_at <= time && time < self.ends_at
    }
}
impl Game {
    pub fn active_fuel_contract(&self) -> Option<&Contract> {
        self.fuel_contract
            .as_ref()
            .filter(|c| c.active_at(self.clock.elapsed().as_secs()))
    }
    pub fn start_fuel_contract(&mut self, share: u32, months: u32) -> Result<(), String> {
        self.require_solvent()?;
        if self.airline.is_none()
            || self.resolution.deadline.is_some()
            || self.active_fuel_contract().is_some()
            || !RULES.shares_percent.contains(&share)
            || !RULES.terms_months.contains(&months)
        {
            return Err("Kein neuer Treibstoffvertrag möglich (Anteil, Laufzeit, aktiver Vertrag oder Restrukturierung).".into());
        }
        let now = self.clock.elapsed().as_secs();
        let ends_at = now
            .checked_add(u64::from(months) * RULES.month_days * 86400)
            .ok_or("Vertragszeitüberlauf.")?;
        self.fuel_contract = Some(Contract {
            started_at: now,
            ends_at,
            share_percent: share,
            months,
            locked_cents: (self.fuel_market.price_at(now) * (10000 + RULES.premium_bps) + 9999)
                / 10000,
        });
        Ok(())
    }
    pub fn review_fuel_contract(&mut self) {
        #[cfg(test)]
        if DISABLE_AI.with(|v| v.get()) {
            return;
        }
        if self.active_fuel_contract().is_none()
            && self.cash() >= RULES.ai_reserve_eur
            && self.fuel_market.price_at(self.clock.elapsed().as_secs())
                < crate::economy::fuel_mean()
        {
            // A rejected offer is harmless: distressed/restructuring owners cannot sign.
            let _ = self.start_fuel_contract(RULES.ai_share_percent, RULES.ai_term_months);
        }
    }
    /// Reprice the unrounded physical fuel quantity, round once to whole EUR.
    /// Signed effect is billed cost minus the same flight's full market cost.
    pub fn apply_fuel_contract(
        &self,
        result: &mut crate::economy::FlightResult,
        model: &str,
        departure: u64,
    ) -> Result<(), String> {
        if let Some(c) = self
            .fuel_contract
            .as_ref()
            .filter(|c| c.active_at(departure))
        {
            let market = self.fuel_market.price_at(departure);
            let price = market * i64::from(100 - c.share_percent)
                + c.locked_cents * i64::from(c.share_percent);
            let route = self
                .routes
                .iter()
                .find(|r| r.id == result.route_id)
                .ok_or("Route fehlt.")?;
            let fuel = crate::economy::fuel_bill(route, model, price)?;
            result.totals.fuel_contract_effect = fuel - result.totals.fuel;
            result.totals.fuel = fuel;
        }
        Ok(())
    }
    pub fn validate_fuel_contract(&self) -> Result<(), String> {
        if let Some(c) = &self.fuel_contract {
            let duration = u64::from(c.months) * RULES.month_days * 86400;
            if self.airline.is_none()
                || c.started_at > self.clock.elapsed().as_secs()
                || !RULES.shares_percent.contains(&c.share_percent)
                || !RULES.terms_months.contains(&c.months)
                || c.started_at.checked_add(duration) != Some(c.ends_at)
                || c.locked_cents
                    != (self.fuel_market.price_at(c.started_at) * (10000 + RULES.premium_bps)
                        + 9999)
                        / 10000
            {
                return Err("Ungültiger Treibstoffvertrag.".into());
            }
        }
        Ok(())
    }
}
#[cfg(test)]
thread_local! { pub(crate) static DISABLE_AI: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
/// Old saves have no commitments, including AI companies.
pub fn migrate(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(o) => {
            if o.contains_key("clock") {
                o.insert("fuel_contract".into(), serde_json::Value::Null);
            }
            for value in o.values_mut() {
                migrate(value);
            }
        }
        serde_json::Value::Array(a) => {
            for value in a {
                migrate(value);
            }
        }
        _ => {}
    }
}
