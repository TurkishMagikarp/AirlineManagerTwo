//! Simple aircraft maintenance, identical for players and AI airlines.
//!
//! Wear grows only at landings: flight time and cycles since the last check.
//! A check is due at the configured flight hours or cycles, whichever comes
//! first. Requested checks take time and money; an aircraft that reaches the
//! limit is grounded at its landing into a longer, more expensive unscheduled
//! check. No components, random failures or condition-dependent flight costs.
//! All values: `config/maintenance.json`.
use crate::{
    aircraft::{self, AircraftStatus},
    game::Game,
    history::Purchase,
    operations::{self, FlightPlan},
    staff::{self, Role},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::LazyLock};

/// Delay between the end of a check and the first departure of a resumed plan.
const RESUME_DELAY: u64 = 60;

#[derive(Deserialize)]
struct Config {
    unscheduled_cost_percent: i64,
    unscheduled_duration_percent: u64,
    models: HashMap<String, Rules>,
}

#[derive(Deserialize)]
pub struct Rules {
    pub interval_flight_hours: u64,
    pub interval_cycles: u64,
    pub duration_hours: u64,
    pub base_cost_eur: i64,
    pub cost_per_flight_hour_eur: i64,
    pub cost_per_cycle_eur: i64,
}

static CONFIG: LazyLock<Result<Config, String>> = LazyLock::new(|| {
    let c: Config = serde_json::from_str(include_str!("../../config/maintenance.json"))
        .map_err(|_| "Wartungskonfiguration ist ungültig.".to_string())?;
    let invalid = || "Wartungskonfiguration enthält ungültige Werte.".to_string();
    if !(100..=1000).contains(&c.unscheduled_cost_percent)
        || !(100..=1000).contains(&c.unscheduled_duration_percent)
    {
        return Err(invalid());
    }
    for m in aircraft::models()? {
        let r = c
            .models
            .get(&m.id)
            .ok_or_else(|| format!("Wartungswerte fehlen für {}.", m.id))?;
        if !(1..=100_000).contains(&r.interval_flight_hours)
            || !(1..=100_000).contains(&r.interval_cycles)
            || !(1..=24 * 60).contains(&r.duration_hours)
            || [
                r.base_cost_eur,
                r.cost_per_flight_hour_eur,
                r.cost_per_cycle_eur,
            ]
            .iter()
            .any(|v| !(0..=10_000_000).contains(v))
        {
            return Err(invalid());
        }
    }
    Ok(c)
});

fn config() -> Result<&'static Config, String> {
    CONFIG.as_ref().map_err(Clone::clone)
}

pub fn rules(model_id: &str) -> Result<&'static Rules, String> {
    config()?
        .models
        .get(model_id)
        .ok_or_else(|| "Unbekanntes Flugzeugmodell.".to_string())
}

/// Lifetime operating data and wear since the last completed check.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Wear {
    pub flight_seconds: u64,
    pub cycles: u64,
    pub check_seconds: u64,
    pub check_cycles: u64,
    pub checks: u64,
}

impl Wear {
    /// Wear after `flights` further landings of `seconds` flight time each.
    pub fn added(&self, flights: u64, seconds: u64) -> Option<Self> {
        let time = flights.checked_mul(seconds)?;
        Some(Self {
            flight_seconds: self.flight_seconds.checked_add(time)?,
            cycles: self.cycles.checked_add(flights)?,
            check_seconds: self.check_seconds.checked_add(time)?,
            check_cycles: self.check_cycles.checked_add(flights)?,
            checks: self.checks,
        })
    }

    pub fn is_due(&self, r: &Rules) -> bool {
        self.check_seconds >= r.interval_flight_hours * 3600
            || self.check_cycles >= r.interval_cycles
    }

    /// 1000 = fresh from a check, 0 = limit reached.
    pub fn condition_permille(&self, r: &Rules) -> u64 {
        let hours =
            u128::from(self.check_seconds) * 1000 / (u128::from(r.interval_flight_hours) * 3600);
        let cycles = u128::from(self.check_cycles) * 1000 / u128::from(r.interval_cycles);
        1000u128.saturating_sub(hours.max(cycles)) as u64
    }

    /// Landings until the check is due (0 = already due).
    pub fn flights_until_due(&self, r: &Rules, flight_seconds: u64) -> u64 {
        if self.is_due(r) {
            return 0;
        }
        let by_time =
            (r.interval_flight_hours * 3600 - self.check_seconds).div_ceil(flight_seconds.max(1));
        by_time.min(r.interval_cycles - self.check_cycles)
    }
}

pub fn cost(wear: &Wear, r: &Rules, unscheduled: bool) -> Result<i64, String> {
    let hours =
        (i128::from(wear.check_seconds) * i128::from(r.cost_per_flight_hour_eur) + 3599) / 3600;
    let base = i128::from(r.base_cost_eur)
        + hours
        + i128::from(wear.check_cycles) * i128::from(r.cost_per_cycle_eur);
    let percent = if unscheduled {
        config()?.unscheduled_cost_percent
    } else {
        100
    };
    i64::try_from((base * i128::from(percent) + 99) / 100)
        .map_err(|_| "Wartungskosten-Überlauf.".to_string())
}

/// Planning value for AI decisions: the regular check after a full interval of
/// `flight_seconds` legs, spread over those landings (rounded up).
pub fn cost_per_flight(model_id: &str, flight_seconds: u64) -> Result<i64, String> {
    let r = rules(model_id)?;
    let flights = Wear::default().flights_until_due(r, flight_seconds).max(1);
    let wear = Wear::default()
        .added(flights, flight_seconds)
        .ok_or("Betriebszeit-Überlauf.")?;
    let flights = i64::try_from(flights).map_err(|_| "Wartungskosten-Überlauf.")?;
    Ok((cost(&wear, r, false)? + flights - 1) / flights)
}

pub fn duration(r: &Rules, unscheduled: bool) -> Result<u64, String> {
    let seconds = r.duration_hours * 3600;
    Ok(if unscheduled {
        (seconds * config()?.unscheduled_duration_percent).div_ceil(100)
    } else {
        seconds
    })
}

/// Flight plan suspended by a check; resumed afterwards if the route is unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resume {
    pub route_id: u32,
    pub interval_seconds: u64,
    /// Landings of the suspended plan, continued by the resumed counter.
    pub flights: u64,
    /// Multi-aircraft service rejoins its original phase after the check.
    #[serde(default)]
    pub phase_seconds: Option<u64>,
}

/// A planned (start in the future, aircraft still airborne) or active check.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Visit {
    pub start: u64,
    pub end: u64,
    /// Fixed when planned, debited exactly once at the start.
    pub cost_eur: i64,
    pub unscheduled: bool,
    #[serde(default)]
    pub resume: Option<Resume>,
    /// Additive (version 11): technicians are assigned. Until then the aircraft
    /// is grounded in the hangar and `end` is only a placeholder.
    #[serde(default = "staffed_default")]
    pub staffed: bool,
}

fn staffed_default() -> bool {
    true
}

impl Game {
    pub(crate) fn planned_route(&self, aircraft_id: u32) -> Option<usize> {
        self.routes
            .iter()
            .position(|r| r.aircraft_id == Some(aircraft_id) && r.plan.is_some())
    }

    /// Landing at which a flying aircraft reaches its limit and is grounded.
    pub fn due_landing(&self, p: usize) -> Option<u64> {
        let plane = &self.fleet[p];
        if plane.maintenance.is_some() {
            return None;
        }
        let route = &self.routes[self.planned_route(plane.id)?];
        let plan = route.plan.as_ref()?;
        let (flight, _) = operations::timings(route, &plane.model_id).ok()?;
        let k = plane
            .wear
            .flights_until_due(rules(&plane.model_id).ok()?, flight);
        if k == 0 && plane.status != AircraftStatus::InFlight {
            return Some(self.clock.elapsed().as_secs());
        }
        plan.completed_flights
            .checked_add(k.saturating_sub(1))?
            .checked_mul(plan.interval_seconds)?
            .checked_add(plan.first_departure)?
            .checked_add(flight)
    }

    /// Next whole-second instant at which any check starts, ends or is forced.
    pub fn next_maintenance_event(&self) -> Option<u64> {
        (0..self.fleet.len())
            .filter_map(|p| match &self.fleet[p].maintenance {
                // Waiting for technicians: ended by a hire or another visit, not by time.
                Some(v) if self.fleet[p].status == AircraftStatus::Maintenance => {
                    v.staffed.then_some(v.end)
                }
                Some(v) => Some(v.start),
                None => self.due_landing(p),
            })
            .min()
    }

    /// The check a request would plan now: after landing if airborne, else immediately.
    pub fn quote_maintenance(&self, aircraft_id: u32) -> Result<Visit, String> {
        let airline = self
            .airline
            .as_ref()
            .ok_or("Bitte zuerst eine Airline gründen.")?;
        let plane = self
            .fleet
            .iter()
            .find(|p| p.id == aircraft_id)
            .ok_or("Unbekanntes Flugzeug.")?;
        if plane.maintenance.is_some() {
            return Err("Wartung ist bereits geplant oder läuft.".into());
        }
        let now = self.clock.elapsed().as_secs();
        let (start, wear) = if plane.status == AircraftStatus::InFlight {
            let route = &self.routes[self
                .planned_route(plane.id)
                .ok_or("Flugzustand ohne Flugplan.")?];
            let state = operations::movement(route, &plane.model_id, now)?;
            let (flight, _) = operations::timings(route, &plane.model_id)?;
            (
                state.landing,
                plane
                    .wear
                    .added(1, flight)
                    .ok_or("Betriebszeit-Überlauf.")?,
            )
        } else {
            (now, plane.wear.clone())
        };
        if wear.check_seconds == 0 && wear.check_cycles == 0 {
            return Err("Frisch gewartet · seit dem letzten Check noch keine Abnutzung.".into());
        }
        let r = rules(&plane.model_id)?;
        let visit = Visit {
            start,
            end: start
                .checked_add(duration(r, false)?)
                .ok_or("Wartungs-Zeitüberlauf.")?,
            cost_eur: cost(&wear, r, false)?,
            unscheduled: false,
            resume: None,
            staffed: true,
        };
        if airline.capital_eur < visit.cost_eur {
            return Err(format!(
                "Nicht genügend Guthaben: {} EUR für die Wartung benötigt, {} EUR verfügbar.",
                visit.cost_eur, airline.capital_eur
            ));
        }
        Ok(visit)
    }

    /// Airborne aircraft finish their flight first; others enter the hangar now.
    pub fn request_maintenance(&mut self, aircraft_id: u32) -> Result<(), String> {
        self.require_active()?;
        let visit = self.quote_maintenance(aircraft_id)?;
        let p = self.fleet.iter().position(|p| p.id == aircraft_id).unwrap();
        let immediate = visit.start <= self.clock.elapsed().as_secs();
        self.fleet[p].maintenance = Some(visit);
        if immediate && let Err(error) = self.begin_visit(p) {
            self.fleet[p].maintenance = None;
            return Err(error);
        }
        Ok(())
    }

    /// Withdraw an order that has not started (aircraft still airborne). Nothing
    /// was debited yet and the flight plan simply continues. Running checks and
    /// forced (unscheduled) groundings cannot be cancelled.
    pub fn cancel_maintenance(&mut self, aircraft_id: u32) -> Result<(), String> {
        self.require_active()?;
        let plane = self
            .fleet
            .iter_mut()
            .find(|p| p.id == aircraft_id)
            .ok_or("Unbekanntes Flugzeug.")?;
        match &plane.maintenance {
            None => Err("Keine Wartung geplant.".into()),
            Some(_) if plane.status == AircraftStatus::Maintenance => {
                Err("Die Wartung läuft bereits und wird regulär abgeschlossen.".into())
            }
            Some(v) if v.unscheduled => Err("Pflichtwartung kann nicht storniert werden.".into()),
            Some(_) => {
                plane.maintenance = None;
                Ok(())
            }
        }
    }

    /// All fallible checks precede the first mutation.
    pub(crate) fn begin_visit(&mut self, p: usize) -> Result<(), String> {
        let plane = &self.fleet[p];
        if plane.status == AircraftStatus::InFlight {
            return Err("Flugzeug ist noch in der Luft.".into());
        }
        let mut visit = plane.maintenance.clone().ok_or("Keine Wartung geplant.")?;
        let balance = self
            .airline
            .as_ref()
            .ok_or("Airline fehlt.")?
            .capital_eur
            .checked_sub(visit.cost_eur)
            .ok_or("Finanzüberlauf.")?;
        let route = self.planned_route(plane.id);
        if let Some(i) = route {
            let plan = self.routes[i].plan.as_ref().unwrap();
            if plan.airborne.is_some() {
                return Err("Flugzeug ist noch in der Luft.".into());
            }
            visit.resume = Some(Resume {
                route_id: self.routes[i].id,
                interval_seconds: plan.interval_seconds,
                phase_seconds: self.routes[i].rotation_phase.or_else(|| {
                    self.routes
                        .iter()
                        .any(|r| {
                            r.public_id() == self.routes[i].public_id() && r.id != self.routes[i].id
                        })
                        .then_some(plan.first_departure % plan.interval_seconds)
                }),
                flights: plan
                    .prior_flights
                    .checked_add(plan.completed_flights)
                    .ok_or("Landungszähler-Überlauf.")?,
            });
            self.routes[i].plan = None;
            self.routes[i].rotation_phase = None;
            if self.routes[i].remove_at.take().is_some() {
                self.routes[i].aircraft_id = None;
                visit.resume = None;
            }
        }
        if let Some(h) = &mut self.history {
            h.maintenance.push(Purchase {
                time: visit.start,
                amount: visit.cost_eur,
            });
        }
        // Without enough free technicians the aircraft waits grounded in the hangar.
        visit.staffed =
            self.available_staff(Role::Technician) >= staff::needs(&plane.model_id)?.technicians;
        self.airline.as_mut().unwrap().capital_eur = balance;
        self.fleet[p].status = AircraftStatus::Maintenance;
        self.fleet[p].maintenance = Some(visit);
        Ok(())
    }

    fn finish_visit(&mut self, p: usize) -> Result<(), String> {
        let distressed = self.distressed();
        let visit = self.fleet[p]
            .maintenance
            .take()
            .ok_or("Keine Wartung aktiv.")?;
        let plane = &mut self.fleet[p];
        plane.status = AircraftStatus::Parked;
        plane.wear.check_seconds = 0;
        plane.wear.check_cycles = 0;
        plane.wear.checks = plane.wear.checks.saturating_add(1);
        let Some(resume) = visit.resume.filter(|_| !distressed) else {
            return Ok(());
        };
        let (id, model, location) = (
            plane.id,
            plane.model_id.clone(),
            plane.location_icao.clone(),
        );
        // Deleted, reassigned or rescheduled routes are not resumed.
        let Some(i) = self
            .routes
            .iter()
            .position(|r| r.id == resume.route_id && r.aircraft_id == Some(id) && r.plan.is_none())
        else {
            return Ok(());
        };
        let mut route = self.routes[i].clone();
        if location != route.origin_icao && location != route.destination_icao {
            return Ok(());
        }
        let mut first = visit
            .end
            .checked_add(RESUME_DELAY)
            .ok_or("Flugplan-Zeitüberlauf.")?;
        if let Some(phase) = resume.phase_seconds {
            first = first
                .checked_add(
                    (phase + resume.interval_seconds - first % resume.interval_seconds)
                        % resume.interval_seconds,
                )
                .ok_or("Flugplan-Zeitüberlauf.")?;
        }
        route.plan = Some(FlightPlan {
            first_departure: first,
            interval_seconds: resume.interval_seconds,
            reversed: location == route.destination_icao,
            prior_flights: resume.flights,
            ..FlightPlan::default()
        });
        // A changed catalog may no longer fit the interval: then stay parked.
        if operations::movement(&route, &model, visit.end).is_ok() {
            self.routes[i] = route;
        }
        Ok(())
    }

    /// Process every maintenance event at or before the current second.
    pub(crate) fn apply_maintenance(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        for p in 0..self.fleet.len() {
            let plane = &self.fleet[p];
            match &plane.maintenance {
                Some(v) if plane.status == AircraftStatus::Maintenance => {
                    if v.staffed && v.end <= now {
                        self.finish_visit(p)?;
                    }
                }
                Some(v) => {
                    if v.start <= now {
                        self.begin_visit(p)?;
                    }
                }
                None => {
                    if plane.status != AircraftStatus::InFlight
                        && self.due_landing(p).is_some_and(|t| t <= now)
                    {
                        let r = rules(&plane.model_id)?;
                        self.fleet[p].maintenance = Some(Visit {
                            start: now,
                            end: now
                                .checked_add(duration(r, true)?)
                                .ok_or("Wartungs-Zeitüberlauf.")?,
                            cost_eur: cost(&plane.wear, r, true)?,
                            unscheduled: true,
                            resume: None,
                            staffed: true,
                        });
                        self.begin_visit(p)?;
                    }
                }
            }
        }
        // Technicians freed by finished checks serve waiting aircraft.
        self.assign_waiting_technicians()
    }

    pub fn validate_maintenance(&self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        let invalid = || "Ungültige Wartungsdaten im Spielstand.".to_string();
        for plane in &self.fleet {
            rules(&plane.model_id)?;
            let w = &plane.wear;
            if w.check_seconds > w.flight_seconds || w.check_cycles > w.cycles {
                return Err(invalid());
            }
            let planned = self.planned_route(plane.id);
            match &plane.maintenance {
                None if plane.status == AircraftStatus::Maintenance => return Err(invalid()),
                None => {}
                Some(v) => {
                    if (v.staffed && v.end <= v.start) || v.cost_eur < 0 {
                        return Err(invalid());
                    }
                    if plane.status == AircraftStatus::Maintenance {
                        if v.start > now || (v.staffed && v.end <= now) || planned.is_some() {
                            return Err(invalid());
                        }
                        // The route may since have been deleted, but never be from the future.
                        if let Some(r) = &v.resume
                            && (r.interval_seconds == 0
                                || r.phase_seconds
                                    .is_some_and(|phase| phase >= r.interval_seconds)
                                || r.route_id == 0
                                || r.route_id >= self.next_route_id)
                        {
                            return Err(invalid());
                        }
                    } else {
                        let route = &self.routes[planned.ok_or_else(invalid)?];
                        let state = operations::movement(route, &plane.model_id, now)?;
                        if plane.status != AircraftStatus::InFlight
                            || v.start != state.landing
                            || v.unscheduled
                            || v.resume.is_some()
                        {
                            return Err(invalid());
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        let mut g = Game::default();
        g.found("Hangar Air", "DE", "EDDF").unwrap();
        g.competitors.clear();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.schedule(1, 1, 120).unwrap();
        g.clock.set_running(true);
        g
    }

    fn state(g: &Game) -> serde_json::Value {
        serde_json::to_value(g).unwrap()
    }

    fn flight() -> u64 {
        operations::timings(&game().routes[0], "atr72_600")
            .unwrap()
            .0
    }

    fn landings(g: &Game) -> u64 {
        g.routes[0]
            .plan
            .as_ref()
            .map(|p| p.prior_flights + p.completed_flights)
            .unwrap_or(0)
    }

    #[test]
    fn landings_accumulate_exact_flight_time_and_cycles() {
        let mut g = game();
        assert!(g.advance(60.0 + 7200.0 * 10.0));
        let w = &g.fleet[0].wear;
        assert_eq!(w.cycles, 10);
        assert_eq!(w.flight_seconds, 10 * flight());
        assert_eq!((w.check_cycles, w.check_seconds), (10, 10 * flight()));
        let r = rules("atr72_600").unwrap();
        assert!(w.condition_permille(r) < 1000);
        assert!(!w.is_due(r));
        g.validate_maintenance().unwrap();
    }

    #[test]
    fn parked_request_debits_once_blocks_departures_and_resumes_from_location() {
        let mut g = game();
        // First landing at EDDM, then ground time ends: parked between flights.
        assert!(g.advance(60.0 + flight() as f64 + 1800.0 + 10.0));
        assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
        assert_eq!(g.fleet[0].location_icao, "EDDM");
        let before = g.airline.as_ref().unwrap().capital_eur;
        let quote = g.quote_maintenance(1).unwrap();
        let r = rules("atr72_600").unwrap();
        assert_eq!(quote.cost_eur, cost(&g.fleet[0].wear, r, false).unwrap());
        assert_eq!(quote.end - quote.start, r.duration_hours * 3600);
        g.request_maintenance(1).unwrap();
        assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
        assert!(g.routes[0].plan.is_none());
        assert_eq!(
            g.airline.as_ref().unwrap().capital_eur,
            before - quote.cost_eur
        );
        assert!(g.request_maintenance(1).is_err());
        assert!(g.schedule(1, 1, 120).is_err());
        g.validate_operations().unwrap();
        g.validate_maintenance().unwrap();
        g.validate_history().unwrap();
        let flights = g.finance.totals.flights;
        // No departures during the whole check, at any step size.
        let remaining = quote.end - g.clock.elapsed().as_secs();
        let mut small = g.clone();
        assert!(g.advance(remaining as f64 - 1.0));
        for _ in 0..(remaining - 1) / 60 {
            assert!(small.advance(60.0));
        }
        assert!(small.advance(((remaining - 1) % 60) as f64));
        assert_eq!(state(&g), state(&small));
        assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
        assert_eq!(g.finance.totals.flights, flights);
        assert!(g.advance(1.0));
        assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
        assert_eq!(g.fleet[0].wear.check_cycles, 0);
        assert_eq!(g.fleet[0].wear.checks, 1);
        let plan = g.routes[0].plan.as_ref().unwrap();
        assert!(plan.reversed);
        assert_eq!(plan.prior_flights, 1);
        assert_eq!(plan.first_departure, quote.end + RESUME_DELAY);
        // Resumed shuttle departs from where the aircraft actually is.
        assert!(g.advance(RESUME_DELAY as f64 + flight() as f64));
        assert_eq!(g.fleet[0].location_icao, "EDDF");
        assert_eq!(landings(&g), 2);
        assert_eq!(g.finance.recent.last().unwrap().origin, "EDDM");
        g.validate_history().unwrap();
        assert_eq!(g.history.as_ref().unwrap().maintenance.len(), 1);
    }

    #[test]
    fn airborne_request_finishes_the_flight_then_enters_the_hangar() {
        let mut g = game();
        assert!(g.advance(100.0));
        assert_eq!(g.fleet[0].status, AircraftStatus::InFlight);
        let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
        let before = g.airline.as_ref().unwrap().capital_eur;
        g.request_maintenance(1).unwrap();
        let visit = g.fleet[0].maintenance.clone().unwrap();
        assert_eq!(visit.start, frozen.landing);
        assert_eq!(g.fleet[0].status, AircraftStatus::InFlight);
        assert_eq!(g.airline.as_ref().unwrap().capital_eur, before);
        assert!(g.stop_schedule(1).is_err());
        g.validate_maintenance().unwrap();
        // Pending state survives a restart and continues identically.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pending.json");
        crate::save::save(&path, &g).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(state(&g), state(&loaded));
        let hangar = (visit.end - 100 - 1) as f64;
        assert!(g.advance(hangar));
        assert!(loaded.advance(hangar));
        assert_eq!(state(&g), state(&loaded));
        // Exactly the one airborne flight was completed and settled, no further departure.
        assert_eq!(g.finance.recent[0], frozen);
        assert_eq!(g.finance.totals.flights, 1);
        assert_eq!(g.fleet[0].location_icao, "EDDM");
        assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
        assert_eq!(
            g.airline.as_ref().unwrap().capital_eur,
            before + frozen.totals.profit() - visit.cost_eur
        );
        assert!(g.advance(1.0));
        assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
        assert_eq!(g.fleet[0].wear.checks, 1);
        assert!(g.routes[0].plan.as_ref().unwrap().reversed);
        g.validate_history().unwrap();
    }

    #[test]
    fn pending_order_can_be_cancelled_until_the_check_starts() {
        assert!(game().cancel_maintenance(1).is_err(), "nothing ordered");
        for speed in crate::clock::Clock::SPEEDS {
            let mut g = game();
            g.clock.set_speed(speed);
            let untouched = g.clone();
            assert!(g.advance(100.0 / f64::from(speed)));
            let before = g.airline.as_ref().unwrap().capital_eur;
            g.request_maintenance(1).unwrap();
            // Cancelled after a restart: no debit, plan keeps flying as if never ordered.
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("cancel.json");
            crate::save::save(&path, &g).unwrap();
            let mut g = crate::save::load(&path).unwrap();
            g.cancel_maintenance(1).unwrap();
            assert!(g.fleet[0].maintenance.is_none());
            assert_eq!(g.airline.as_ref().unwrap().capital_eur, before);
            g.validate_maintenance().unwrap();
            let mut reference = untouched.clone();
            assert!(reference.advance(100.0 / f64::from(speed)));
            assert_eq!(state(&g), state(&reference));
            for _ in 0..20 {
                assert!(g.advance(3600.0 / f64::from(speed)));
            }
            assert!(reference.advance(20.0 * 3600.0 / f64::from(speed)));
            assert_eq!(state(&g), state(&reference));
            assert_eq!(g.fleet[0].wear.checks, 0);
            assert!(g.history.as_ref().unwrap().maintenance.is_empty());
            // Once the aircraft is in the hangar the check runs to completion.
            loop {
                assert!(g.advance(60.0 / f64::from(speed)));
                if g.fleet[0].status != AircraftStatus::InFlight && g.request_maintenance(1).is_ok()
                {
                    break;
                }
            }
            assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
            let active = state(&g);
            assert!(g.cancel_maintenance(1).is_err());
            assert_eq!(state(&g), active);
        }
    }

    #[test]
    fn planned_cost_per_flight_spreads_one_regular_check_over_its_interval() {
        let r = rules("atr72_600").unwrap();
        let k = Wear::default().flights_until_due(r, flight());
        let wear = Wear::default().added(k, flight()).unwrap();
        let check = cost(&wear, r, false).unwrap();
        let per = cost_per_flight("atr72_600", flight()).unwrap();
        assert!(per > 0);
        let k = k as i64;
        assert!(per * k >= check && (per - 1) * k < check);
    }

    #[test]
    fn reaching_the_limit_forces_a_longer_more_expensive_check_at_landing() {
        let r = rules("atr72_600").unwrap();
        let k = Wear::default().flights_until_due(r, flight());
        assert!(k > 0);
        let mut g = game();
        // Exactly the k-th landing triggers the unscheduled check.
        let landing = 60 + (k - 1) * 7200 + flight();
        assert_eq!(g.due_landing(0), Some(landing));
        assert!(g.advance(landing as f64 - 1.0));
        assert!(g.fleet[0].maintenance.is_none());
        let wear = g.fleet[0].wear.added(1, flight()).unwrap();
        let before = g.airline.as_ref().unwrap().capital_eur;
        assert!(g.advance(1.0));
        let v = g.fleet[0].maintenance.clone().unwrap();
        assert!(v.unscheduled);
        assert_eq!(v.start, landing);
        assert_eq!(v.cost_eur, cost(&wear, r, true).unwrap());
        assert!(v.cost_eur > cost(&wear, r, false).unwrap());
        assert_eq!(v.end - v.start, duration(r, true).unwrap());
        assert!(v.end - v.start > duration(r, false).unwrap());
        let profit = g.finance.recent.last().unwrap().totals.profit();
        assert_eq!(
            g.airline.as_ref().unwrap().capital_eur,
            before + profit - v.cost_eur
        );
        assert_eq!(g.fleet[0].status, AircraftStatus::Maintenance);
        g.validate_maintenance().unwrap();
        g.validate_history().unwrap();
    }

    #[test]
    fn repeated_checks_are_identical_for_any_step_size_speed_and_restart() {
        for speed in crate::clock::Clock::SPEEDS {
            let mut fast = game();
            fast.clock.set_speed(speed);
            let mut slow = fast.clone();
            let mut restarted = fast.clone();
            let span = 400.0 * 86400.0;
            assert!(fast.advance(span / f64::from(speed)));
            for _ in 0..400 {
                assert!(slow.advance(86400.0 / f64::from(speed)));
            }
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("checks.json");
            for _ in 0..8 {
                assert!(restarted.advance(span / 8.0 / f64::from(speed)));
                crate::save::save(&path, &restarted).unwrap();
                restarted = crate::save::load(&path).unwrap();
            }
            assert_eq!(state(&fast), state(&slow));
            assert_eq!(state(&fast), state(&restarted));
            let w = &fast.fleet[0].wear;
            assert!(w.checks >= 2, "checks {}", w.checks);
            assert_eq!(fast.finance.totals.flights as u64, w.cycles);
            let h = fast.history.as_ref().unwrap();
            assert_eq!(
                h.maintenance.len() as u64,
                w.checks + u64::from(fast.fleet[0].maintenance.is_some())
            );
            fast.validate_history().unwrap();
            fast.validate_maintenance().unwrap();
        }
    }

    #[test]
    fn insufficient_funds_and_fresh_aircraft_leave_state_unchanged() {
        let mut g = game();
        let fresh = state(&g);
        assert!(g.request_maintenance(1).unwrap_err().contains("Frisch"));
        assert_eq!(state(&g), fresh);
        assert!(g.advance(60.0 + 7200.0 * 3.0));
        g.airline.as_mut().unwrap().capital_eur = 10;
        let poor = state(&g);
        assert!(g.request_maintenance(1).unwrap_err().contains("Guthaben"));
        assert!(g.request_maintenance(99).is_err());
        assert_eq!(state(&g), poor);
    }

    #[test]
    fn deleted_route_is_not_resumed_and_corrupt_saves_are_rejected() {
        let mut g = game();
        assert!(g.advance(60.0 + 7200.0 * 2.0 - 1.0));
        assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
        g.request_maintenance(1).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hangar.json");
        crate::save::save(&path, &g).unwrap();
        let valid: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for (pointer, value) in [
            (
                "/state/fleet/aircraft/0/status",
                serde_json::json!("parked"),
            ),
            (
                "/state/fleet/aircraft/0/maintenance",
                serde_json::Value::Null,
            ),
            (
                "/state/fleet/aircraft/0/maintenance/end",
                serde_json::json!(0),
            ),
            (
                "/state/fleet/aircraft/0/maintenance/cost_eur",
                serde_json::json!(-1),
            ),
            (
                "/state/fleet/aircraft/0/wear/check_cycles",
                serde_json::json!(999),
            ),
            ("/state/history/maintenance/0/amount", serde_json::json!(1)),
        ] {
            let mut bad = valid.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(crate::save::load(&path).is_err(), "accepted {pointer}");
        }
        g.delete_route(1).unwrap();
        assert!(g.advance(2.0 * 86400.0));
        assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
        assert!(g.routes.is_empty());
        g.validate_maintenance().unwrap();
    }

    #[test]
    fn version_nine_saves_load_with_fresh_aircraft_and_no_maintenance() {
        let mut g = game();
        assert!(g.advance(100.0));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v9.json");
        crate::save::save(&path, &g).unwrap();
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        v["version"] = 9.into();
        let plane = v["state"]["fleet"]["aircraft"][0].as_object_mut().unwrap();
        plane.remove("wear");
        plane.remove("maintenance");
        let plan = v["state"]["routes"]["routes"][0]["plan"]
            .as_object_mut()
            .unwrap();
        plan.remove("reversed");
        plan.remove("prior_flights");
        v["state"]["history"]
            .as_object_mut()
            .unwrap()
            .remove("maintenance");
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.fleet, g.fleet);
        assert_eq!(loaded.routes, g.routes);
        assert_eq!(loaded.airline, g.airline);
        // Pre-v16 saves start their fuel timeline at load time.
        assert_eq!(loaded.fuel_market.started_at, 100);
        g.fuel_market = loaded.fuel_market.clone();
        g.migrate_tax().unwrap();
        g.marketing.settled_at = 100;
        g.migrate_insurance_fixture();
        assert!(loaded.advance(86400.0));
        assert!(g.advance(86400.0));
        assert_eq!(state(&loaded), state(&g));
    }
}
