//! Daily fixed-point punctuality reputation. Only newly settled flights count;
//! no wall time, historical replay, or dependence on frame/route ordering.
use crate::disruptions::RULES;
use serde::{Deserialize, Serialize};
pub const DAY: u64 = 86400;
pub const SCALE: i64 = 1_000_000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reputation {
    /// Signed millionths: -1 = consistently late, 0 = neutral, +1 = on time.
    pub score: i64,
    pub trend: i64,
    pub flights: i64,
    pub late: i64,
}
pub fn next_day(now: u64) -> u64 {
    (now / DAY + 1).saturating_mul(DAY)
}
pub fn attractiveness(score: i64) -> f64 {
    #[cfg(test)]
    if NEUTRAL_CHOICE.with(|v| v.get()) {
        return 1.0;
    }
    1.0 + score.clamp(-SCALE, SCALE) as f64 / SCALE as f64 * RULES.reputation_max_choice_bps as f64
        / 10_000.0
}
#[cfg(test)]
thread_local! {
    pub static NEUTRAL_CHOICE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
impl Reputation {
    pub fn observe(&mut self, flights: i64, late: i64) {
        self.flights += flights;
        self.late += late;
    }
    pub fn close_day(&mut self) {
        self.trend = 0;
        if self.flights > 0 {
            let otp = (i128::from(self.flights - self.late) * i128::from(SCALE)
                / i128::from(self.flights)) as i64;
            let neutral = RULES.reputation_neutral_otp_per_million;
            let denominator = if otp >= neutral {
                SCALE - neutral
            } else {
                neutral
            };
            let target = (otp - neutral) * SCALE / denominator;
            self.trend = (target - self.score) / RULES.reputation_response_days;
            self.score += self.trend;
        }
        self.flights = 0;
        self.late = 0;
    }
    pub fn percent(&self) -> f64 {
        (attractiveness(self.score) - 1.0) * 100.0
    }
    pub fn trend_text(&self) -> &'static str {
        match self.trend.cmp(&0) {
            std::cmp::Ordering::Less => "sinkend",
            std::cmp::Ordering::Equal => "stabil",
            std::cmp::Ordering::Greater => "steigend",
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(-SCALE..=SCALE).contains(&self.score)
            || !(-2 * SCALE..=2 * SCALE).contains(&self.trend)
            || self.flights < 0
            || self.late < 0
            || self.late > self.flights
        {
            return Err("Ungültiger Pünktlichkeitsruf.".into());
        }
        Ok(())
    }
}
/// Missing fields in old owners start neutral. Never derive reputation from
/// lifetime financial totals; preserve additive fields in compatibility fixtures.
pub fn migrate(state: &mut serde_json::Value) {
    if state.get("reputation").is_none() {
        state["reputation"] = serde_json::to_value(Reputation::default()).unwrap();
    }
    if let Some(competitors) = state["competitors"].as_array_mut() {
        for company in competitors {
            migrate(&mut company["game"]);
        }
    }
}
