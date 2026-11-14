//! Read-only player attention snapshot; never persisted or applied to AI decisions.
use crate::{aircraft::AircraftStatus, game::Game, operations, staff};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Deserialize)]
struct Rules {
    negative_cash_threshold_eur: i64,
    priorities: BTreeMap<String, u32>,
}
static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/attention.json"))
        .expect("Aufmerksamkeitskonfiguration ungültig");
    for key in [
        "negative_cash",
        "crew",
        "delay",
        "technicians",
        "salary_reserve",
        "idle",
        "training",
    ] {
        assert!(r.priorities.contains_key(key), "Priorität fehlt: {key}");
    }
    assert!(r.negative_cash_threshold_eur <= 0);
    r
});

#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub kind: &'static str,
    pub priority: u32,
    pub text: String,
    pub page: &'static str,
    pub route_id: u32,
    pub aircraft_id: u32,
    pub employee_id: u32,
}

impl Game {
    pub fn attention_snapshot(&self) -> Vec<Entry> {
        let Some(airline) = &self.airline else {
            return Vec::new();
        };
        let mut entries = Vec::new();
        let mut add = |kind, text, page, route_id, aircraft_id, employee_id| {
            entries.push(Entry {
                kind,
                priority: RULES.priorities[kind],
                text,
                page,
                route_id,
                aircraft_id,
                employee_id,
            });
        };
        if airline.capital_eur < RULES.negative_cash_threshold_eur {
            add(
                "negative_cash",
                format!("Guthaben negativ: {} EUR", airline.capital_eur),
                "finances",
                0,
                0,
                0,
            );
        } else if let Ok(reserve) = self.salary_reserve()
            && airline.capital_eur < reserve
        {
            add(
                "salary_reserve",
                format!(
                    "Guthaben unter Personalreserve: {} / {} EUR (offene Gehälter und Personalreserve)",
                    airline.capital_eur, reserve
                ),
                "finances",
                0,
                0,
                0,
            );
        }
        if self.reputation.score <= crate::disruptions::RULES.reputation_attention_score
            && self.reputation.trend < 0
        {
            add(
                "reputation",
                "Pünktlichkeitsruf sinkt: Verspätungen schwächen die Nachfrage.".into(),
                "fleet",
                0,
                0,
                0,
            );
        }
        let nearly_full: Vec<_> = crate::airports::catalog()
            .unwrap()
            .airports
            .iter()
            .filter(|a| {
                a.icao == airline.home_airport_icao
                    || self
                        .routes
                        .iter()
                        .any(|r| r.origin_icao == a.icao || r.destination_icao == a.icao)
            })
            .filter(|a| {
                a.daily_departure_slots.is_some_and(|cap| {
                    self.slot_usage(&a.icao) * 100
                        >= u64::from(cap) * u64::from(crate::slots::RULES.near_full_percent)
                })
            })
            .map(|a| self.slot_text(&a.icao))
            .collect();
        if !nearly_full.is_empty() {
            add("slots", nearly_full.join(" · "), "routes", 0, 0, 0);
        }
        for p in &self.fleet {
            if let Some(i) = self.planned_route(p.id)
                && let Ok(state) = operations::movement(
                    &self.routes[i],
                    &p.model_id,
                    self.clock.elapsed().as_secs(),
                )
                && state.delay_seconds > crate::disruptions::RULES.attention_threshold_seconds
            {
                add(
                    "delay",
                    format!(
                        "{} · {} Minuten Verspätung",
                        p.identifier(),
                        state.delay_seconds.div_ceil(60)
                    ),
                    "fleet",
                    self.routes[i].public_id(),
                    p.id,
                    0,
                );
            }
            if p.status == AircraftStatus::Maintenance
                && p.maintenance.as_ref().is_some_and(|v| !v.staffed)
            {
                add(
                    "technicians",
                    format!(
                        "{} · In Wartung · wartet auf Techniker (Personal-Tab)",
                        p.identifier()
                    ),
                    "fleet",
                    0,
                    p.id,
                    0,
                );
            }
            if p.status == AircraftStatus::Parked
                && p.maintenance.is_none()
                && self.planned_route(p.id).is_none()
            {
                for r in self
                    .routes
                    .iter()
                    .filter(|r| r.aircraft_id == Some(p.id) && r.plan.is_none())
                {
                    if let Err(reason) = self.require_crew(&p.model_id) {
                        add(
                            "crew",
                            format!(
                                "{} · {} → {} · {}",
                                p.identifier(),
                                r.origin_icao,
                                r.destination_icao,
                                reason
                            ),
                            "routes",
                            r.public_id(),
                            p.id,
                            0,
                        );
                    }
                }
                add(
                    "idle",
                    format!("{} · Geparkt · kein Flugplan", p.identifier()),
                    "fleet",
                    0,
                    p.id,
                    0,
                );
            }
        }
        for e in &self.staff.employees {
            if e.role == staff::Role::Pilot
                && let Some(t) = &e.training
            {
                add(
                    "training",
                    format!(
                        "{} · Umschulung → {} bis {}",
                        e.name,
                        t.rating,
                        operations::display_time(t.ends_at)
                    ),
                    "staff",
                    0,
                    0,
                    e.id,
                );
            }
        }
        entries.sort_by_key(|e| (e.priority, e.kind, e.route_id, e.aircraft_id, e.employee_id));
        entries
    }
}
