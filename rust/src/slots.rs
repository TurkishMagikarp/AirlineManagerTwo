//! Slots are derived from timetables, never from actual departures or the clock.
//! A shuttle alternates endpoints: reserve the maximum departures per rolling
//! day at each endpoint (ceil(86400 / (2 * interval))). No banking or grace expiry.
use crate::{airports, game::Game};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Deserialize)]
pub struct Rules {
    pub near_full_percent: u32,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let rules: Rules = serde_json::from_str(include_str!("../../config/slots.json")).unwrap();
    assert!((1..=100).contains(&rules.near_full_percent));
    rules
});

pub fn departures(interval: u64) -> u64 {
    86400u64.div_ceil(interval.saturating_mul(2).max(1))
}

impl Game {
    pub(crate) fn own_slots(&self) -> BTreeMap<String, u64> {
        let mut usage = BTreeMap::new();
        for r in &self.routes {
            if r.remove_at.is_some() {
                continue;
            }
            let interval = r.plan.as_ref().map(|p| p.interval_seconds).or_else(|| {
                self.fleet
                    .iter()
                    .find(|p| Some(p.id) == r.aircraft_id)
                    .and_then(|p| p.maintenance.as_ref())
                    .and_then(|v| v.resume.as_ref())
                    .filter(|v| v.route_id == r.id)
                    .map(|v| v.interval_seconds)
            });
            if let Some(interval) = interval {
                for airport in [&r.origin_icao, &r.destination_icao] {
                    *usage.entry(airport.clone()).or_default() += departures(interval);
                }
            }
        }
        usage
    }

    pub(crate) fn slots_except_competitor(&self, except: usize) -> BTreeMap<String, u64> {
        let mut usage = self.own_slots();
        for (i, c) in self.competitors.iter().enumerate() {
            if i != except {
                for (a, n) in c.game.own_slots() {
                    *usage.entry(a).or_default() += n;
                }
            }
        }
        usage
    }

    pub fn slot_usage(&self, airport: &str) -> u64 {
        self.own_slots().get(airport).copied().unwrap_or(0)
            + self.external_slots.get(airport).copied().unwrap_or(0)
            + self
                .competitors
                .iter()
                .map(|c| c.game.own_slots().get(airport).copied().unwrap_or(0))
                .sum::<u64>()
    }

    pub fn slot_text(&self, airport: &str) -> String {
        match airports::catalog()
            .unwrap()
            .airports
            .iter()
            .find(|a| a.icao == airport)
            .and_then(|a| a.daily_departure_slots)
        {
            Some(cap) => {
                let used = self.slot_usage(airport);
                format!(
                    "{airport}: {used} Slots belegt · {} von {cap} frei/Tag",
                    u64::from(cap).saturating_sub(used)
                )
            }
            None => format!("{airport}: Slots unbegrenzt"),
        }
    }

    pub(crate) fn check_slots(
        &self,
        origin: &str,
        destination: &str,
        interval: u64,
    ) -> Result<(), String> {
        for code in [origin, destination] {
            if let Some(a) = airports::catalog()?
                .airports
                .iter()
                .find(|a| a.icao == code)
                && let Some(cap) = a.daily_departure_slots
            {
                let used = self.slot_usage(code);
                if used.saturating_add(departures(interval)) > u64::from(cap) {
                    return Err(format!(
                        "{} is full: {} of {cap} daily slots free ({} required)",
                        a.iata.as_deref().unwrap_or(code),
                        u64::from(cap).saturating_sub(used),
                        departures(interval)
                    ));
                }
            }
        }
        Ok(())
    }
}
