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
        "event",
        "weather",
        "alliance",
        "charter_expiring",
        "charter_failed",
    ] {
        assert!(r.priorities.contains_key(key), "Priorität fehlt: {key}");
    }
    assert!(r.negative_cash_threshold_eur <= 0);
    r
});

#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub order_id: u64,
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
                order_id: 0,
                kind,
                priority: RULES.priorities[kind],
                text,
                page,
                route_id,
                aircraft_id,
                employee_id,
            });
        };
        for role in staff::Role::ALL {
            let group = self.labour_group(role);
            if let Some(end) = group.strike_until {
                add(
                    "strike",
                    format!(
                        "{} · Streik bis {}",
                        role.plural(),
                        operations::display_time(end)
                    ),
                    "staff",
                    0,
                    0,
                    role as u32 + 1,
                );
            } else if let Some(start) = group.warning_at {
                add(
                    "strike_warning",
                    format!(
                        "{} · Streikwarnung ab {} · Gehalt anheben",
                        role.plural(),
                        operations::display_time(start)
                    ),
                    "staff",
                    0,
                    0,
                    role as u32 + 1,
                );
            }
        }
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
        let now = self.clock.elapsed().as_secs();
        let mine: Vec<&str> = std::iter::once(airline.home_airport_icao.as_str())
            .chain(self.routes.iter().flat_map(|r| {
                [r.origin_icao.as_str(), r.destination_icao.as_str()]
            }))
            .collect();
        let mut announced = Vec::new();
        for e in crate::world_events::visible_at(&self.world_economy, now) {
            if mine.iter().any(|icao| e.touches(icao)) {
                announced.push(Entry {
                    order_id: e.id,
                    kind: "event",
                    priority: RULES.priorities["event"],
                    text: e.text(),
                    page: "overview",
                    route_id: 0,
                    aircraft_id: 0,
                    employee_id: 0,
                });
            }
        }
        for e in crate::weather::visible_at(&crate::weather::Weather::of(&self.world_economy), now) {
            if mine.contains(&e.icao) {
                announced.push(Entry {
                    order_id: e.id,
                    kind: "weather",
                    priority: RULES.priorities["weather"],
                    text: e.attention_text(),
                    page: "overview",
                    route_id: 0,
                    aircraft_id: 0,
                    employee_id: 0,
                });
            }
        }
        for p in &self.fleet {
            if let Ok(n) = staff::needs(&p.model_id)
                && self.planned_route(p.id).is_some()
                && (self.rated_count(&n.rating) < self.rated_busy(&n.rating)
                    || self.staff_count(staff::Role::CabinCrew) < self.reserved_crew().1)
            {
                add(
                    "crew",
                    format!(
                        "{} · Personal nach Kündigungen nachbesetzen ({})",
                        p.identifier(),
                        n.rating
                    ),
                    "staff",
                    0,
                    p.id,
                    0,
                );
            }
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
                        "{} · {} Minuten Verspätung{}",
                        p.identifier(),
                        state.delay_seconds.div_ceil(60),
                        if state.weather_seconds > 0 { " (Wetter)" } else { "" }
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
                if self.charter_job(p.id).is_none() {
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
        for o in &self.orders {
            if o.held_since.is_some() {
                entries.push(Entry {
                    order_id: o.id,
                    kind: "delivery",
                    priority: RULES.priorities["delivery"],
                    text: format!(
                        "Herstellerlieferung #{} wartet: {} EUR Restzahlung, Frist {}",
                        o.id,
                        o.unit_price - o.deposit,
                        operations::display_time(
                            o.delivery_at + crate::orders::RULES.grace_days * 86400
                        )
                    ),
                    page: "fleet",
                    route_id: 0,
                    aircraft_id: 0,
                    employee_id: 0,
                });
            }
        }
        for n in &self.alliances.notices {
            if n.at + crate::alliances::RULES.notice_visible_days * 86400 >= now {
                entries.push(Entry {
                    order_id: n.id,
                    kind: "alliance",
                    priority: RULES.priorities["alliance"],
                    text: n.text.clone(),
                    page: "routes",
                    route_id: 0,
                    aircraft_id: 0,
                    employee_id: 0,
                });
            }
        }
        let warning = crate::charters::RULES.expiry_warning_hours * 3600;
        let idle: Vec<u32> = self
            .fleet
            .iter()
            .filter(|p| p.status == AircraftStatus::Parked && self.require_ferry_aircraft(p.id).is_ok())
            .map(|p| p.id)
            .collect();
        if !idle.is_empty() {
            for o in self.open_charters().iter().filter(|o| o.expires - now <= warning) {
                if let Some(&id) = idle.iter().find(|&&id| self.quote_charter_offer(o, id).is_ok()) {
                    entries.push(Entry {
                        order_id: o.id,
                        kind: "charter_expiring",
                        priority: RULES.priorities["charter_expiring"],
                        text: format!(
                            "Charterangebot {} · {} EUR · läuft ab {} · passt zu AM-{:06}",
                            o.label(),
                            o.fee_eur,
                            operations::display_time(o.expires),
                            id
                        ),
                        page: "fleet",
                        route_id: 0,
                        aircraft_id: 0,
                        employee_id: 0,
                    });
                }
            }
        }
        for o in self.charters.recent.iter().filter(|o| !o.delivered) {
            entries.push(Entry {
                order_id: o.offer_id,
                kind: "charter_failed",
                priority: RULES.priorities["charter_failed"],
                text: format!(
                    "Charter {} gescheitert: {} · Vertragsstrafe {} EUR",
                    o.label,
                    o.reason,
                    -o.cash_eur
                ),
                page: "fleet",
                route_id: 0,
                aircraft_id: 0,
                employee_id: 0,
            });
        }
        entries.extend(announced);
        entries.sort_by_key(|e| {
            (
                e.priority,
                e.kind,
                e.route_id,
                e.aircraft_id,
                e.employee_id,
                e.order_id,
            )
        });
        entries
    }
}
