//! Optional player-only goals, sampled after settlement at fixed game midnights.
//! No rewards, economic writes, attention entries or flight-loop work.
use crate::{game::Game, reputation::DAY};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Fleet,
    ActiveRoutes,
    Destinations,
    Passengers,
    LongHaulKm,
    ConnectingPassengers,
    ProfitableYears,
    AnnualRevenue,
    DebtFreeFleet,
    ReputationDays,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: String,
    pub kind: Kind,
    pub threshold: i64,
}
pub fn parse(source: &str) -> Result<Vec<Definition>, String> {
    let rules: Vec<Definition> = serde_json::from_str(source)
        .map_err(|_| "Ungültige Meilenstein-Konfiguration.".to_string())?;
    let mut ids = BTreeSet::new();
    if rules.is_empty()
        || rules.iter().any(|r| {
            r.threshold <= 0
                || r.id.is_empty()
                || !r
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                || !ids.insert(&r.id)
        })
    {
        return Err("Ungültige Meilenstein-Schwellen oder Kennungen.".into());
    }
    Ok(rules)
}
pub fn load_rules(path: &std::path::Path) -> Result<Vec<Definition>, String> {
    std::fs::read_to_string(path)
        .map_err(|_| "Meilenstein-Konfiguration fehlt.".to_string())
        .and_then(|s| parse(&s))
}
static RULES: LazyLock<Result<Vec<Definition>, String>> = LazyLock::new(|| {
    // Cargo tests start in rust/; the game starts at the project root.
    let path = if std::env::current_dir()
        .ok()
        .is_some_and(|p| p.ends_with("rust"))
    {
        "../config/milestones.json"
    } else {
        "config/milestones.json"
    };
    load_rules(std::path::Path::new(path))
});
pub fn rules() -> Result<&'static [Definition], String> {
    RULES.as_ref().map(Vec::as_slice).map_err(Clone::clone)
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub reached: BTreeMap<String, u64>,
    pub notified: BTreeSet<String>,
    pub last_day: u64,
    pub previous_revenue: i64,
    pub previous_flights: i64,
    pub annual_revenue: i64,
    pub reputation_days: i64,
}
impl Game {
    pub fn milestone_values(&self) -> Vec<i64> {
        let Some(state) = &self.milestones else {
            return Vec::new();
        };
        let active: Vec<_> = self.routes.iter().filter(|r| r.plan.is_some()).collect();
        let routes: BTreeSet<_> = active.iter().map(|r| r.public_id()).collect();
        let destinations: BTreeSet<_> = active
            .iter()
            .flat_map(|r| [&r.origin_icao, &r.destination_icao])
            .collect();
        let longest = active
            .iter()
            .filter_map(|r| crate::routes::distance_km(&r.origin_icao, &r.destination_icao).ok())
            .fold(0.0_f64, f64::max) as i64;
        let profitable =
            self.history
                .as_ref()
                .map_or(0, |h| h.taxes.iter().filter(|a| a.profit > 0).count()) as i64;
        rules()
            .unwrap_or_default()
            .iter()
            .map(|r| match r.kind {
                Kind::Fleet => self.fleet.len() as i64,
                Kind::ActiveRoutes => routes.len() as i64,
                Kind::Destinations => destinations.len() as i64,
                Kind::Passengers => self.finance.totals.passengers,
                Kind::LongHaulKm => longest,
                Kind::ConnectingPassengers => self.finance.totals.connecting_passengers,
                Kind::ProfitableYears => profitable,
                Kind::AnnualRevenue => {
                    state.annual_revenue + (self.finance.totals.revenue - state.previous_revenue)
                }
                Kind::DebtFreeFleet => i64::from(
                    !self.fleet.is_empty()
                        && self.financing.debt == 0
                        && self.financing.overdue == 0,
                ),
                Kind::ReputationDays => state.reputation_days,
            })
            .collect()
    }
    pub(crate) fn evaluate_milestones(&mut self) {
        let now = self.clock.elapsed().as_secs();
        let Some(state) = &mut self.milestones else {
            return;
        };
        if now / DAY <= state.last_day {
            return;
        }
        state.last_day = now / DAY;
        state.annual_revenue += self.finance.totals.revenue - state.previous_revenue;
        state.previous_revenue = self.finance.totals.revenue;
        if self.reputation.score >= 500_000 && self.finance.totals.flights > state.previous_flights
        {
            state.reputation_days += 1;
        } else {
            state.reputation_days = 0;
        }
        state.previous_flights = self.finance.totals.flights;
        self.record_milestones(now);
        if now.is_multiple_of(crate::demand::YEAR_SECONDS) {
            self.milestones.as_mut().unwrap().annual_revenue = 0;
        }
    }
    fn record_milestones(&mut self, now: u64) {
        let values = self.milestone_values();
        let Some(state) = &mut self.milestones else {
            return;
        };
        for (rule, value) in rules().unwrap_or_default().iter().zip(values) {
            if value >= rule.threshold {
                state.reached.entry(rule.id.clone()).or_insert(now);
            }
        }
    }
    pub(crate) fn migrate_milestones(&mut self) -> Result<(), String> {
        rules()?;
        if self.airline.is_none() {
            return Ok(());
        }
        let now = self.clock.elapsed().as_secs();
        let year_start = now / crate::demand::YEAR_SECONDS * crate::demand::YEAR_SECONDS;
        let previous = self.history.as_ref().map_or(0, |h| {
            h.prior_totals.revenue + h.totals(0, year_start).revenue
        });
        self.milestones = Some(State {
            last_day: now / DAY,
            previous_revenue: self.finance.totals.revenue,
            previous_flights: self.finance.totals.flights,
            annual_revenue: self.finance.totals.revenue.saturating_sub(previous),
            ..Default::default()
        });
        self.record_milestones(now);
        let state = self.milestones.as_mut().unwrap();
        state.notified = state.reached.keys().cloned().collect();
        Ok(())
    }
    pub fn validate_milestones(&self) -> Result<(), String> {
        rules()?;
        let Some(s) = &self.milestones else {
            return Ok(());
        };
        if self.airline.is_none()
            || s.last_day > self.clock.elapsed().as_secs() / DAY
            || s.reputation_days < 0
            || s.annual_revenue < 0
            || s.annual_revenue > s.previous_revenue
            || s.reputation_days as u64 > s.last_day
            || s.previous_revenue < 0
            || s.previous_flights < 0
            || s.previous_revenue > self.finance.totals.revenue
            || s.previous_flights > self.finance.totals.flights
            || s.reached.iter().any(|(id, time)| {
                !rules().unwrap().iter().any(|r| &r.id == id)
                    || *time > self.clock.elapsed().as_secs()
            })
            || s.notified.iter().any(|id| !s.reached.contains_key(id))
        {
            return Err("Ungültige Meilensteine im Spielstand.".into());
        }
        Ok(())
    }
    pub fn take_milestone_notifications(&mut self) -> Vec<String> {
        let Some(s) = &mut self.milestones else {
            return Vec::new();
        };
        let pending: Vec<_> = s
            .reached
            .keys()
            .filter(|id| !s.notified.contains(*id))
            .cloned()
            .collect();
        s.notified.extend(pending.iter().cloned());
        pending
    }
}
