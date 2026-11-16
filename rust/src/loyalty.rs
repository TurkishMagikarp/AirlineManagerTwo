//! Airline-wide membership, daily integer accrual and immutable departure rewards.
use crate::{economy::Totals, game::Game, marketing::DAY};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
const BPS: i64 = 10_000;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    #[default]
    None,
    Basic,
    Generous,
}
#[derive(Deserialize)]
pub struct Rate {
    pub economy_eur: i64,
    pub business_eur: i64,
    pub enrolment_bps: i64,
}
#[derive(Deserialize)]
pub struct Rules {
    none: Rate,
    basic: Rate,
    generous: Rate,
    pub daily_lapse_bps: i64,
    pub saturation_members: i64,
    pub max_bonus_bps: i64,
    pub ai_reserve_eur: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules =
        serde_json::from_str(include_str!("../../config/loyalty.json")).expect("loyalty config");
    assert!(r.none.economy_eur == 0 && r.none.business_eur == 0 && r.none.enrolment_bps == 0);
    for rate in [&r.basic, &r.generous] {
        assert!((0..=100).contains(&rate.economy_eur));
        assert!((0..=100).contains(&rate.business_eur));
        assert!((0..=BPS).contains(&rate.enrolment_bps));
    }
    assert!((1..=BPS).contains(&r.daily_lapse_bps));
    assert!((1..=1_000_000_000).contains(&r.saturation_members));
    assert!((0..=600).contains(&r.max_bonus_bps));
    assert!(r.ai_reserve_eur > 0);
    r
});
impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Basic => "basic",
            Self::Generous => "generous",
        }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "none" => Ok(Self::None),
            "basic" => Ok(Self::Basic),
            "generous" => Ok(Self::Generous),
            _ => Err("Unbekannte Treuestufe.".into()),
        }
    }
    pub fn rate(self) -> &'static Rate {
        match self {
            Self::None => &RULES.none,
            Self::Basic => &RULES.basic,
            Self::Generous => &RULES.generous,
        }
    }
    #[inline]
    pub fn cost(self, passengers: i64, business: i64) -> i64 {
        if self == Self::None {
            return 0;
        }
        let r = self.rate();
        (passengers - business) * r.economy_eur + business * r.business_eur
    }
    #[inline]
    pub fn enrolment(self, passengers: i64) -> i64 {
        if self == Self::None {
            0
        } else {
            passengers * self.rate().enrolment_bps
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loyalty {
    pub level: Level,
    pub members: i64,
    /// Whole-member change at the last daily close.
    pub trend: i64,
    /// Enrolment numerator from immutable results landed since the last close.
    pub pending: i64,
    pub enrolment_remainder: i64,
    pub lapse_remainder: i64,
    pub closed_day: u64,
}
/// Zero members preserve the legacy floating-point operation order and avoid config access.
#[inline]
pub fn weight(members: i64, weight: f64) -> f64 {
    if members == 0 {
        weight
    } else {
        weight * (1.0 + bonus(members))
    }
}
pub fn bonus(members: i64) -> f64 {
    if members == 0 {
        return 0.0;
    }
    RULES.max_bonus_bps as f64 / BPS as f64 * members.min(RULES.saturation_members) as f64
        / RULES.saturation_members as f64
}
/// Two-leg paths average the operators' bonuses, preserving the same cap.
#[inline]
pub fn path_weight(members: [i64; 2], weight: f64) -> f64 {
    if members == [0, 0] {
        weight
    } else {
        weight * (1.0 + (bonus(members[0]) + bonus(members[1])) / 2.0)
    }
}
pub fn valid(t: &Totals) -> bool {
    t.loyalty_cost >= 0
        && i128::from(t.loyalty_cost) <= i128::from(t.passengers) * 100
        && t.loyalty_enrolment >= 0
        && i128::from(t.loyalty_enrolment) <= i128::from(t.passengers) * i128::from(BPS)
}
impl Game {
    pub fn set_loyalty_level(&mut self, level: Level) -> Result<(), String> {
        self.require_active()?;
        if self.loyalty.level != level {
            self.loyalty.level = level;
            for r in &mut self.routes {
                r.loyalty_level = level;
            }
            self.commercial_changed();
        }
        Ok(())
    }
    /// Called after every owner has consumed the same pre-close supply snapshot.
    /// Existing reputation boundaries already split long advances at absolute days.
    pub fn apply_loyalty(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        let l = &mut self.loyalty;
        if !now.is_multiple_of(DAY) || now / DAY <= l.closed_day {
            return Ok(());
        }
        if l.pending == 0 && l.members == 0 && l.enrolment_remainder == 0 && l.lapse_remainder == 0
        {
            l.trend = 0;
            return Ok(());
        }
        let enrolled = l
            .pending
            .checked_add(l.enrolment_remainder)
            .ok_or("Treueüberlauf.")?;
        let lapse = i128::from(l.members) * i128::from(RULES.daily_lapse_bps)
            + i128::from(l.lapse_remainder);
        let change = enrolled / BPS - (lapse / i128::from(BPS)) as i64;
        let old_weight_members = l.members.min(RULES.saturation_members);
        l.members = l.members.checked_add(change).ok_or("Treueüberlauf.")?;
        l.trend = change;
        l.enrolment_remainder = enrolled % BPS;
        l.lapse_remainder = (lapse % i128::from(BPS)) as i64;
        l.pending = 0;
        l.closed_day = now / DAY;
        if old_weight_members != l.members.min(RULES.saturation_members) {
            self.commercial_changed();
        }
        Ok(())
    }
    /// Weekly, no RNG: reward only with healthy cash and no restructuring deadline.
    pub fn review_loyalty(&mut self) {
        #[cfg(test)]
        if LEGACY_AI.with(|v| v.get()) {
            return;
        }
        let level = if !self.distressed()
            && self.resolution.deadline.is_none()
            && self.cash() >= RULES.ai_reserve_eur
        {
            Level::Basic
        } else {
            Level::None
        };
        let _ = self.set_loyalty_level(level);
    }
    pub fn validate_loyalty(&self) -> Result<(), String> {
        let l = &self.loyalty;
        if l.members < 0
            || l.pending < 0
            || l.trend.abs_diff(0) > i64::MAX as u64
            || !(0..BPS).contains(&l.enrolment_remainder)
            || !(0..BPS).contains(&l.lapse_remainder)
            || l.closed_day > self.clock.elapsed().as_secs() / DAY
            || i128::from(l.pending) > i128::from(self.finance.totals.passengers) * i128::from(BPS)
            || (self.airline.is_none() && l != &Loyalty::default())
        {
            return Err("Ungültige Treuedaten.".into());
        }
        Ok(())
    }
}
#[cfg(test)]
thread_local! { pub(crate) static LEGACY_AI: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
/// Pin historical/regression fixtures to the pre-loyalty AI network and ledger.
#[cfg(test)]
pub(crate) struct LegacyAi(bool);
#[cfg(test)]
impl Drop for LegacyAi {
    fn drop(&mut self) {
        LEGACY_AI.with(|v| v.set(self.0));
    }
}
#[cfg(test)]
pub(crate) fn legacy_ai_scenario() -> LegacyAi {
    LegacyAi(LEGACY_AI.with(|v| v.replace(true)))
}
/// v41 → v42: no retroactive membership, rewards or airborne enrolment.
pub fn migrate(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(o) => {
            if o.contains_key("clock") {
                o.insert(
                    "loyalty".into(),
                    serde_json::to_value(Loyalty::default()).unwrap(),
                );
            }
            if o.contains_key("origin_icao") {
                o.insert("loyalty_level".into(), "none".into());
            }
            if o.contains_key("fuel") && o.contains_key("revenue") {
                o.insert("loyalty_cost".into(), 0.into());
                o.insert("loyalty_enrolment".into(), 0.into());
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
