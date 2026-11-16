//! Shared, integer mean-reverting macroeconomy. Derived period values are cached;
//! no random draws depend on frames, flights, owners or lookup order.
use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::LazyLock,
};

#[derive(Deserialize)]
pub struct Rules {
    pub minimum: i64,
    pub maximum: i64,
    pub volatility: i64,
    pub mean_reversion_divisor: i64,
    pub cadence_seconds: u64,
    pub business_reaction: f64,
    pub leisure_reaction: f64,
    pub cargo_reaction: f64,
    pub downturn_threshold: i64,
    pub boom_threshold: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/world_economy.json"))
        .expect("world economy config");
    assert!(0 < r.minimum && r.minimum <= 10000 && r.maximum >= 10000 && r.maximum <= 20000);
    assert!((0..=10000).contains(&r.volatility));
    assert!(r.mean_reversion_divisor > 0 && r.cadence_seconds >= 86400);
    assert!(r.minimum < r.downturn_threshold && r.downturn_threshold < 10000);
    assert!(10000 < r.boom_threshold && r.boom_threshold < r.maximum);
    for reaction in [r.business_reaction, r.leisure_reaction, r.cargo_reaction] {
        assert!(reaction.is_finite() && (0.0..=3.0).contains(&reaction));
        assert!(factor(r.minimum, reaction) > 0.0);
    }
    r
});

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cycle {
    #[serde(default, skip_serializing_if = "crate::difficulty::Level::is_standard")]
    pub crises: crate::difficulty::Level,
    pub seed: u64,
    pub started_at: u64,
    /// Last delivered period; notification delivery is saved, independent of UI frames.
    pub notified_period: u64,
    /// Events are only announced from this time on; older saves start it at load.
    #[serde(default)]
    pub events_from: u64,
}
impl Default for Cycle {
    fn default() -> Self {
        Self {
            crises: Default::default(),
            seed: 0x45434f4e4f4d59,
            started_at: 0,
            notified_period: 0,
            events_from: 0,
        }
    }
}
thread_local! {
    // The same period is queried by passengers, freight and each fare candidate.
    // Keep the raw index so different crisis settings can share this exact lookup.
    static RECENT: Cell<Option<(u64, usize, i64)>> = const { Cell::new(None) };
    static TIMELINES: RefCell<HashMap<u64, Vec<i64>>> = RefCell::new(HashMap::new());
}
#[cfg(test)]
thread_local! { pub(crate) static ZERO_REACTIONS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
pub fn factor(index: i64, reaction: f64) -> f64 {
    #[cfg(test)]
    let reaction = if ZERO_REACTIONS.with(|v| v.get()) {
        0.0
    } else {
        reaction
    };
    1.0 + (index - 10000) as f64 / 10000.0 * reaction
}
pub fn phase(index: i64) -> &'static str {
    if index <= RULES.downturn_threshold {
        "downturn"
    } else if index >= RULES.boom_threshold {
        "boom"
    } else {
        "normal"
    }
}
impl Cycle {
    pub fn period(&self, time: u64) -> u64 {
        time.saturating_sub(self.started_at) / RULES.cadence_seconds
    }
    pub fn index_at(&self, time: u64) -> i64 {
        let period = self.period(time) as usize;
        if let Some((seed, cached_period, index)) = RECENT.with(Cell::get)
            && seed == self.seed
            && cached_period == period
        {
            return 10000 + self.crises.scale(index - 10000);
        }
        let index = TIMELINES.with(|cache| {
            let mut cache = cache.borrow_mut();
            if !cache.contains_key(&self.seed) && cache.len() >= 4 {
                cache.clear();
            }
            let values = cache.entry(self.seed).or_insert_with(|| vec![10000]);
            while values.len() <= period {
                let mut z = self
                    .seed
                    .wrapping_add((values.len() as u64).wrapping_mul(0x9e3779b97f4a7c15));
                z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
                z ^= z >> 31;
                let shock = (z % (2 * RULES.volatility + 1) as u64) as i64 - RULES.volatility;
                let previous = *values.last().unwrap();
                values.push(
                    (previous + shock + (10000 - previous) / RULES.mean_reversion_divisor)
                        .clamp(RULES.minimum, RULES.maximum),
                );
            }
            values[period]
        });
        RECENT.with(|cache| cache.set(Some((self.seed, period, index))));
        10000 + self.crises.scale(index - 10000)
    }
    pub fn validate(&self, now: u64) -> Result<(), String> {
        if self.started_at > now || self.events_from > now || self.notified_period > self.period(now) {
            return Err("Ungültige Weltwirtschaft.".into());
        }
        Ok(())
    }
    pub fn take_notifications(&mut self, now: u64) -> Vec<&'static str> {
        let end = self.period(now);
        let mut result = Vec::new();
        let mut previous =
            phase(self.index_at(self.started_at + self.notified_period * RULES.cadence_seconds));
        for period in self.notified_period + 1..=end {
            let next = phase(self.index_at(self.started_at + period * RULES.cadence_seconds));
            if next != previous {
                result.push(next);
            }
            previous = next;
        }
        self.notified_period = end;
        result
    }
}

/// Context carried by immutable supply snapshots. Annual planning keeps the
/// current economy while omitting seasonality; departures use their actual date.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Context {
    pub cycle: Cycle,
    pub quoted_at: u64,
}
impl Context {
    pub fn index(&self, time: Option<u64>) -> i64 {
        self.cycle.index_at(time.unwrap_or(self.quoted_at))
    }
}
/// Shared market for one route: season, economy and regional events.
pub fn route_factors(
    context: Option<Context>,
    time: Option<u64>,
    origin: &str,
    destination: &str,
) -> crate::demand::Market {
    with_events(factors(context, time), context, time, origin, destination)
}

/// Annual planning (`time == None`) ignores events like it ignores the season.
pub fn with_events(
    mut market: crate::demand::Market,
    context: Option<Context>,
    time: Option<u64>,
    origin: &str,
    destination: &str,
) -> crate::demand::Market {
    if let (Some(context), Some(time)) = (context, time)
        && let Some(e) = crate::world_events::route_factors(&context.cycle, time, origin, destination)
    {
        market.business *= e.business;
        market.leisure *= e.leisure;
    }
    market
}

pub fn factors(context: Option<Context>, time: Option<u64>) -> crate::demand::Market {
    let mut season = time
        .map(crate::demand::season_factors)
        .unwrap_or(crate::demand::Market {
            business: 1.0,
            leisure: 1.0,
        });
    if let Some(context) = context {
        let index = context.index(time);
        season.business *= factor(index, RULES.business_reaction);
        season.leisure *= factor(index, RULES.leisure_reaction);
    }
    season
}

/// Transfers retain daily seasonality, but never alias different economy periods.
pub fn transfer_time(context: Option<Context>, time: Option<u64>) -> Option<u64> {
    time.map(|t| match context {
        None => (t - t % 86400) % crate::demand::YEAR_SECONDS,
        Some(c) => (t - t % 86400).max(
            c.cycle
                .started_at
                .saturating_add(c.cycle.period(t) * RULES.cadence_seconds),
        ),
    })
}
