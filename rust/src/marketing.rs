//! Exact pro-rata EUR accrual and daily fixed-point company awareness.
use crate::{
    game::Game,
    history::{History, Purchase},
};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
pub const DAY: u64 = 86400;
pub const SCALE: i64 = 1_000_000;
#[derive(Deserialize)]
pub struct Rules {
    pub month_days: u64,
    pub max_budget_eur: i64,
    half_saturation_eur: i64,
    response_days: i64,
    max_choice_bonus: f64,
    ai_max_budget_eur: i64,
    ai_cash_divisor: i64,
    ai_reserve_eur: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/marketing.json"))
        .expect("marketing config");
    assert!((1..=365).contains(&r.month_days));
    assert!((1..=1_000_000_000).contains(&r.max_budget_eur));
    assert!(r.half_saturation_eur > 0 && (2..=3650).contains(&r.response_days));
    assert!((0.0..=0.1).contains(&r.max_choice_bonus));
    assert!((0..=r.max_budget_eur).contains(&r.ai_max_budget_eur));
    assert!(r.ai_cash_divisor >= 100 && r.ai_reserve_eur > 0);
    r
});
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Marketing {
    pub budget_eur: i64,
    /// Millionths of one point (0..100).
    pub awareness: i64,
    pub trend: i64,
    pub settled_at: u64,
    /// Fractional EUR numerator, denominator = configured month seconds.
    pub remainder: u64,
    /// Budget-seconds accumulated in the current absolute day.
    pub exposure: u64,
}
pub fn attractiveness(awareness: i64) -> f64 {
    1.0 + RULES.max_choice_bonus * awareness as f64 / (100 * SCALE) as f64
}
impl History {
    pub fn marketing_through(&self, time: u64) -> i128 {
        self.marketing_costs
            .iter()
            .filter(|p| p.time <= time)
            .map(|p| i128::from(p.amount))
            .sum()
    }
}
impl Game {
    /// Local route/aircraft IDs may coincide across owners. Put this owner first
    /// so the allocator identifies its offer even when every commercial key matches.
    pub fn market_owner_key(&self) -> String {
        // The simulation root and its AI leaves have separate namespaces: the
        // player may choose an AI's name. A standalone company has no rivals.
        let namespace = if self.competitors.is_empty() {
            "company"
        } else {
            "player"
        };
        format!(
            "{namespace}:{}",
            self.airline.as_ref().map_or("", |a| a.name.as_str())
        )
    }
    pub fn prioritize_market(&self, supply: &[crate::demand::Offer]) -> Vec<crate::demand::Offer> {
        let owner = self.market_owner_key();
        let mut supply = supply.to_vec();
        supply.sort_by_key(|o| o.owner != owner);
        supply
    }
    pub fn set_marketing_budget(&mut self, budget: i64) -> Result<(), String> {
        self.require_active()?;
        if self.airline.is_none() || !(0..=RULES.max_budget_eur).contains(&budget) {
            return Err("Ungültiges monatliches Marketingbudget.".into());
        }
        // Settle the old rate first; awareness changes only at day boundaries.
        self.settle_marketing()?;
        self.marketing.budget_eur = budget;
        self.observe_distress();
        Ok(())
    }
    pub fn next_marketing_event(&self) -> Option<u64> {
        if self.airline.is_none()
            || self.insolvent()
            || (self.marketing.budget_eur == 0
                && self.marketing.awareness == 0
                && self.marketing.exposure == 0)
        {
            return None;
        }
        (self.marketing.settled_at / DAY)
            .checked_add(1)?
            .checked_mul(DAY)
    }
    fn settle_marketing(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        let m = &self.marketing;
        let seconds = now
            .checked_sub(m.settled_at)
            .ok_or("Marketingzeit ungültig.")?;
        let numerator = u128::from(m.remainder) + m.budget_eur as u128 * u128::from(seconds);
        let period = u128::from(RULES.month_days * DAY);
        let amount = i64::try_from(numerator / period).map_err(|_| "Marketingüberlauf.")?;
        let exposure =
            u64::try_from(u128::from(m.exposure) + m.budget_eur as u128 * u128::from(seconds))
                .map_err(|_| "Marketingüberlauf.")?;
        let balance = self
            .airline
            .as_ref()
            .ok_or("Airline fehlt.")?
            .capital_eur
            .checked_sub(amount)
            .ok_or("Marketingüberlauf.")?;
        let h = self.history.as_mut().ok_or("Finanzhistorie fehlt.")?;
        if amount > 0 {
            h.marketing_costs.push(Purchase { time: now, amount });
        }
        self.airline.as_mut().unwrap().capital_eur = balance;
        self.marketing.remainder = (numerator % period) as u64;
        self.marketing.exposure = exposure;
        self.marketing.settled_at = now;
        Ok(())
    }
    pub fn apply_marketing(&mut self) -> Result<(), String> {
        let Some(due) = self.next_marketing_event() else {
            return Ok(());
        };
        if due > self.clock.elapsed().as_secs() {
            return Ok(());
        }
        self.settle_marketing()?;
        let m = &mut self.marketing;
        let budget = i128::from(m.exposure) / i128::from(DAY);
        let target = (100 * i128::from(SCALE) * budget
            / (budget + i128::from(RULES.half_saturation_eur))) as i64;
        let difference = target - m.awareness;
        let change = difference / RULES.response_days;
        m.trend = if change == 0 {
            difference.signum()
        } else {
            change
        };
        m.awareness += m.trend;
        m.exposure = 0;
        self.observe_distress();
        if self.distressed() {
            self.marketing.budget_eur = 0;
        }
        Ok(())
    }
    pub fn review_marketing(&mut self) -> Result<(), String> {
        let cash = self.airline.as_ref().map_or(0, |a| a.capital_eur);
        let budget = if self.distressed() || self.resolution.deadline.is_some() {
            0
        } else {
            ((cash - RULES.ai_reserve_eur).max(0) / RULES.ai_cash_divisor)
                .min(RULES.ai_max_budget_eur)
        };
        self.set_marketing_budget(budget)
    }
    pub fn validate_marketing(&self) -> Result<(), String> {
        let m = &self.marketing;
        let now = self.clock.elapsed().as_secs();
        let invalid = !(0..=RULES.max_budget_eur).contains(&m.budget_eur)
            || !(0..=100 * SCALE).contains(&m.awareness)
            || m.trend.abs_diff(0) > (100 * SCALE / RULES.response_days) as u64
            || self.next_marketing_event().is_some_and(|due| due <= now)
            || m.settled_at > now
            || m.remainder >= RULES.month_days * DAY
            || u128::from(m.exposure)
                > RULES.max_budget_eur as u128 * u128::from(m.settled_at % DAY)
            || (self.airline.is_none() && m != &Marketing::default());
        if invalid {
            return Err("Ungültige Marketingdaten.".into());
        }
        if let Some(h) = &self.history
            && (h
                .marketing_costs
                .iter()
                .any(|p| p.amount <= 0 || p.time < h.since || p.time > now)
                || h.marketing_costs.windows(2).any(|p| p[0].time > p[1].time))
        {
            return Err("Ungültiges Marketingarchiv.".into());
        }
        Ok(())
    }
}
/// All owners start at load time with zero budget/awareness, no retroactive costs.
pub fn migrate(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(o) => {
            if o.contains_key("clock") {
                let now = o["clock"]["elapsed_seconds"]
                    .as_u64()
                    .unwrap_or_else(|| o["clock"]["elapsed"]["secs"].as_u64().unwrap_or(0));
                o.insert(
                    "marketing".into(),
                    serde_json::to_value(Marketing {
                        settled_at: if o.get("airline").is_none_or(|a| a.is_null()) {
                            0
                        } else {
                            now
                        },
                        ..Default::default()
                    })
                    .unwrap(),
                );
            }
            for child in o.values_mut() {
                migrate(child);
            }
        }
        serde_json::Value::Array(a) => {
            for child in a {
                migrate(child);
            }
        }
        _ => {}
    }
}
