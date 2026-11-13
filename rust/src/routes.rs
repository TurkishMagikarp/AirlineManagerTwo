//! Route ownership and assignments live only here; distance is derived from the catalog.
use crate::{aircraft, airports, game::Game};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub id: u32,
    pub origin_icao: String,
    pub destination_icao: String,
    pub aircraft_id: Option<u32>,
    /// Internal service belongs to this public route; roots have no parent.
    #[serde(default)]
    pub parent_route_id: Option<u32>,
    /// Finish the current flight and turnaround before releasing this aircraft.
    #[serde(default)]
    pub remove_at: Option<u64>,
    #[serde(default)]
    pub rotation_phase: Option<u64>,
    #[serde(default = "crate::economy::default_ticket")]
    pub ticket_eur: i64,
    #[serde(default)]
    pub plan: Option<crate::operations::FlightPlan>,
}

pub fn distance_km(origin: &str, destination: &str) -> Result<f64, String> {
    let catalog = airports::catalog()?;
    let find = |icao: &str| {
        catalog
            .airports
            .iter()
            .find(|a| a.icao == icao)
            .ok_or_else(|| "Bitte verfügbare Start- und Zielflughäfen auswählen.".to_string())
    };
    let a = find(origin)?;
    let b = find(destination)?;
    if origin == destination {
        return Err("Start und Ziel müssen verschieden sein.".into());
    }
    let lat = (b.latitude - a.latitude).to_radians();
    let lon = (b.longitude - a.longitude).to_radians();
    let h = (lat / 2.0).sin().powi(2)
        + a.latitude.to_radians().cos() * b.latitude.to_radians().cos() * (lon / 2.0).sin().powi(2);
    Ok(6371.0088 * 2.0 * h.clamp(0.0, 1.0).sqrt().asin())
}

impl Route {
    pub fn public_id(&self) -> u32 {
        self.parent_route_id.unwrap_or(self.id)
    }
}

impl Game {
    pub fn check_route(
        &self,
        origin: &str,
        destination: &str,
        aircraft_id: u32,
        except_route: Option<u32>,
    ) -> Result<f64, String> {
        if self.airline.is_none() {
            return Err("Bitte zuerst eine Airline gründen.".into());
        }
        let distance = distance_km(origin, destination)?;
        let plane = self
            .fleet
            .iter()
            .find(|p| p.id == aircraft_id)
            .ok_or_else(|| "Bitte ein eigenes verfügbares Flugzeug auswählen.".to_string())?;
        if self
            .routes
            .iter()
            .any(|r| Some(r.id) != except_route && r.aircraft_id == Some(aircraft_id))
        {
            return Err("Dieses Flugzeug ist bereits einer anderen Route zugewiesen.".into());
        }
        let range = aircraft::model(&plane.model_id)?.range_km;
        if distance > f64::from(range) {
            return Err(format!(
                "Reichweite überschritten: {:.1} km Strecke, {} km Reichweite.",
                distance, range
            ));
        }
        Ok(distance)
    }

    pub fn create_route(
        &mut self,
        origin: &str,
        destination: &str,
        aircraft_id: u32,
    ) -> Result<(), String> {
        self.require_solvent()?;
        self.check_route(origin, destination, aircraft_id, None)?;
        let next = self
            .next_route_id
            .checked_add(1)
            .ok_or_else(|| "Keine weitere Routenkennung verfügbar.".to_string())?;
        self.routes.push(Route {
            id: self.next_route_id,
            origin_icao: origin.into(),
            destination_icao: destination.into(),
            aircraft_id: Some(aircraft_id),
            parent_route_id: None,
            remove_at: None,
            rotation_phase: None,
            ticket_eur: crate::economy::default_ticket(),
            plan: None,
        });
        self.next_route_id = next;
        Ok(())
    }

    pub fn assign_route(&mut self, route_id: u32, aircraft_id: Option<u32>) -> Result<(), String> {
        self.require_active()?;
        self.require_no_plan(route_id)?;
        let route = self
            .routes
            .iter()
            .find(|r| r.id == route_id)
            .ok_or_else(|| "Unbekannte Route.".to_string())?;
        if let Some(id) = aircraft_id {
            self.check_route(
                &route.origin_icao,
                &route.destination_icao,
                id,
                Some(route_id),
            )?;
        }
        self.route_mut(route_id)?.aircraft_id = aircraft_id;
        Ok(())
    }

    /// Add an independent service to a public route. Departure quotes are
    /// immutable; future phases change only after flight and turnaround.
    pub fn add_route_aircraft(&mut self, route_id: u32, aircraft_id: u32) -> Result<(), String> {
        self.require_solvent()?;
        let root = self
            .routes
            .iter()
            .find(|r| r.id == route_id && r.parent_route_id.is_none())
            .ok_or("Unbekannte Route.")?
            .clone();
        self.check_route(&root.origin_icao, &root.destination_icao, aircraft_id, None)?;
        let mut staged = self.clone();
        let id = staged.next_route_id;
        staged.next_route_id = id
            .checked_add(1)
            .ok_or("Keine weitere Routenkennung verfügbar.")?;
        let template = self
            .routes
            .iter()
            .find(|r| r.public_id() == route_id && r.plan.is_some() && r.remove_at.is_none());
        let mut service = root.clone();
        service.id = id;
        service.parent_route_id = Some(route_id);
        service.aircraft_id = Some(aircraft_id);
        service.plan = None;
        service.remove_at = None;
        service.rotation_phase = None;
        staged.routes.push(service);
        if let Some(h) = &mut staged.history {
            h.route_groups.insert(id, route_id);
        }
        if let Some(template) = template {
            let plan = template.plan.as_ref().unwrap();
            let minutes = u32::try_from(plan.interval_seconds.div_ceil(60))
                .map_err(|_| "Flugplan-Zeitüberlauf.")?;
            staged.schedule_service(id, 1, minutes)?;
            staged
                .routes
                .last_mut()
                .unwrap()
                .plan
                .as_mut()
                .unwrap()
                .interval_seconds = plan.interval_seconds;
            let new_plane = staged.fleet.iter().find(|p| p.id == aircraft_id).unwrap();
            crate::operations::movement(
                staged.routes.last().unwrap(),
                &new_plane.model_id,
                self.clock.elapsed().as_secs(),
            )?;
            let count = staged
                .routes
                .iter()
                .filter(|r| r.public_id() == route_id && r.plan.is_some() && r.remove_at.is_none())
                .count() as u64;
            let phase = plan.first_departure % plan.interval_seconds;
            for (slot, r) in staged
                .routes
                .iter_mut()
                .filter(|r| r.public_id() == route_id && r.plan.is_some() && r.remove_at.is_none())
                .enumerate()
            {
                r.rotation_phase = Some(
                    (phase + slot as u64 * plan.interval_seconds / count) % plan.interval_seconds,
                );
                let p = r.plan.as_mut().unwrap();
                if self.clock.elapsed().as_secs() < p.first_departure
                    && self.clock.elapsed().as_secs() < plan.first_departure
                {
                    p.first_departure = plan.first_departure;
                }
            }
            staged.apply_rotation_removals()?;
        }
        *self = staged;
        Ok(())
    }

    pub fn remove_route_aircraft(&mut self, route_id: u32, aircraft_id: u32) -> Result<(), String> {
        self.require_active()?;
        let i = self
            .routes
            .iter()
            .position(|r| r.public_id() == route_id && r.aircraft_id == Some(aircraft_id))
            .ok_or("Flugzeug ist dieser Route nicht zugewiesen.")?;
        let plane = self
            .fleet
            .iter()
            .find(|p| p.id == aircraft_id)
            .ok_or("Unbekanntes Flugzeug.")?;
        if self.routes[i].remove_at.is_some() {
            return Ok(());
        }
        if self.routes[i].plan.is_some() && plane.status != aircraft::AircraftStatus::Parked {
            let state = crate::operations::movement(
                &self.routes[i],
                &plane.model_id,
                self.clock.elapsed().as_secs(),
            )?;
            self.routes[i].remove_at = Some(state.ready);
            self.routes[i].rotation_phase = None;
        } else {
            self.routes[i].plan = None;
            self.routes[i].aircraft_id = None;
            self.routes[i].rotation_phase = None;
            if let Some(p) = self.fleet.iter_mut().find(|p| p.id == aircraft_id)
                && let Some(v) = &mut p.maintenance
            {
                v.resume = None;
            }
        }
        Ok(())
    }

    pub fn next_rotation_event(&self) -> Option<u64> {
        self.routes
            .iter()
            .filter_map(|r| {
                if r.remove_at.is_some() {
                    return r.remove_at;
                }
                r.rotation_phase?;
                let plane = self.fleet.iter().find(|p| Some(p.id) == r.aircraft_id)?;
                let now = self.clock.elapsed().as_secs();
                let state = crate::operations::movement(r, &plane.model_id, now).ok()?;
                Some(if plane.status == aircraft::AircraftStatus::Parked {
                    now
                } else {
                    state.ready
                })
            })
            .min()
    }

    pub fn apply_rotation_removals(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        for r in &mut self.routes {
            if r.remove_at.is_some_and(|t| t <= now) {
                let p = self
                    .fleet
                    .iter_mut()
                    .find(|p| Some(p.id) == r.aircraft_id)
                    .ok_or("Unbekanntes Flugzeug.")?;
                if r.plan.is_some() {
                    // The boundary can coincide with the next departure. Discard
                    // that new snapshot; the completed landing is already settled.
                    let state = crate::operations::movement(r, &p.model_id, now.saturating_sub(1))?;
                    p.location_icao = state.destination;
                }
                p.status = aircraft::AircraftStatus::Parked;
                if let Some(v) = &mut p.maintenance {
                    v.resume = None;
                }
                r.plan = None;
                r.aircraft_id = None;
                r.remove_at = None;
                r.rotation_phase = None;
            } else if let Some(phase) = r.rotation_phase {
                let p = self
                    .fleet
                    .iter_mut()
                    .find(|p| Some(p.id) == r.aircraft_id)
                    .ok_or("Unbekanntes Flugzeug.")?;
                let Some(plan) = r.plan.as_ref() else {
                    r.rotation_phase = None;
                    continue;
                };
                let old = crate::operations::movement(r, &p.model_id, now.saturating_sub(1))?;
                if p.status != aircraft::AircraftStatus::Parked && now < old.ready {
                    continue;
                }
                let interval = plan.interval_seconds;
                let started = now >= plan.first_departure;
                let location = if started {
                    old.destination
                } else {
                    p.location_icao.clone()
                };
                let earliest = if started {
                    now.checked_add(1).ok_or("Flugplan-Zeitüberlauf.")?
                } else {
                    plan.first_departure
                };
                let first = earliest
                    .checked_add((phase + interval - earliest % interval) % interval)
                    .filter(|t| *t <= u64::MAX / 2)
                    .ok_or("Flugplan-Zeitüberlauf.")?;
                let prior = plan
                    .prior_flights
                    .checked_add(plan.completed_flights)
                    .ok_or("Landungszähler-Überlauf.")?;
                r.plan = Some(crate::operations::FlightPlan {
                    first_departure: first,
                    interval_seconds: interval,
                    reversed: location == r.destination_icao,
                    prior_flights: prior,
                    ..Default::default()
                });
                r.rotation_phase = None;
                p.status = aircraft::AircraftStatus::Parked;
                p.location_icao = location;
            }
        }
        Ok(())
    }

    pub fn route_mut(&mut self, id: u32) -> Result<&mut Route, String> {
        self.require_active()?;
        self.routes
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| "Unbekannte Route.".to_string())
    }

    pub fn delete_route(&mut self, route_id: u32) -> Result<(), String> {
        self.require_active()?;
        self.require_no_plan(route_id)?;
        let _index = self
            .routes
            .iter()
            .position(|r| r.id == route_id)
            .ok_or_else(|| "Unbekannte Route.".to_string())?;
        self.routes.retain(|r| r.public_id() != route_id);
        Ok(())
    }

    pub fn validate_routes(&self) -> Result<(), String> {
        if self.next_route_id == 0 || (self.airline.is_none() && !self.routes.is_empty()) {
            return Err("Ungültige Routendaten im Spielstand.".into());
        }
        let mut ids = HashSet::new();
        for route in &self.routes {
            if !(0..=100_000).contains(&route.ticket_eur) {
                return Err("Ungültiger Ticketpreis im Spielstand.".into());
            }
            if route.id == 0 || route.id >= self.next_route_id || !ids.insert(route.id) {
                return Err("Ungültige Routenkennung im Spielstand.".into());
            }
            if let Some(t) = route.remove_at {
                let plane = self
                    .fleet
                    .iter()
                    .find(|p| Some(p.id) == route.aircraft_id)
                    .ok_or("Ungültige Freigabe.")?;
                let state = crate::operations::movement(
                    route,
                    &plane.model_id,
                    self.clock.elapsed().as_secs(),
                )?;
                if t <= self.clock.elapsed().as_secs()
                    || t != state.ready
                    || route.rotation_phase.is_some()
                {
                    return Err("Ungültige Freigabezeit.".into());
                }
            }
            if let Some(phase) = route.rotation_phase
                && route
                    .plan
                    .as_ref()
                    .is_none_or(|p| phase >= p.interval_seconds)
            {
                return Err("Ungültige Rotationsphase.".into());
            }
            if let Some(parent) = route.parent_route_id {
                let root = self
                    .routes
                    .iter()
                    .find(|r| r.id == parent && r.parent_route_id.is_none())
                    .ok_or("Ungültige Routengruppe.")?;
                if root.origin_icao != route.origin_icao
                    || root.destination_icao != route.destination_icao
                    || root.ticket_eur != route.ticket_eur
                    || route.id <= parent
                {
                    return Err("Widersprüchliche Routengruppe.".into());
                }
            }
            distance_km(&route.origin_icao, &route.destination_icao)?;
            if let Some(id) = route.aircraft_id {
                self.check_route(
                    &route.origin_icao,
                    &route.destination_icao,
                    id,
                    Some(route.id),
                )?;
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
        g.found("Route Air", "DE", "EDDF").unwrap();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g
    }

    #[test]
    fn distance_is_symmetric_and_matches_known_airport_pairs() {
        let d = distance_km("EDDF", "EDDM").unwrap();
        assert!((d - 300.0).abs() < 5.0);
        assert!((d - distance_km("EDDM", "EDDF").unwrap()).abs() < 1e-9);
        assert!((distance_km("KJFK", "KLAX").unwrap() - 3974.0).abs() < 10.0);
    }

    #[test]
    fn invalid_routes_are_transactional_and_do_not_consume_ids() {
        let mut g = game();
        for (a, b, id) in [
            ("EDDF", "EDDF", 1),
            ("XXXX", "EDDM", 1),
            ("EDDF", "XXXX", 1),
            ("EDDF", "LTFM", 1),
            ("EDDF", "EDDM", 99),
        ] {
            assert!(g.create_route(a, b, id).is_err());
            assert!(g.routes.is_empty());
            assert_eq!(g.next_route_id, 1);
        }
        assert!(Game::default().create_route("EDDF", "EDDM", 1).is_err());
        g.next_route_id = u32::MAX;
        assert!(g.create_route("EDDF", "EDDM", 1).is_err());
        assert!(g.routes.is_empty());
    }

    #[test]
    fn reassignment_checks_range_before_releasing_previous_aircraft() {
        let mut g = game();
        g.airline.as_mut().unwrap().capital_eur = 8_000_000;
        g.buy_aircraft("a320_200").unwrap();
        g.staff_up();
        g.create_route("KJFK", "KLAX", 3).unwrap();
        let before = g.routes.clone();
        assert!(
            g.assign_route(1, Some(1))
                .unwrap_err()
                .contains("Reichweite")
        );
        assert_eq!(g.routes, before);
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.validate_routes().unwrap();
    }

    #[test]
    fn assignments_are_exclusive_and_deletion_or_unassigning_releases_aircraft() {
        let mut g = game();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        assert!(g.create_route("LTFM", "LTFJ", 1).is_err());
        g.create_route("LTFM", "LTFJ", 2).unwrap();
        let before = g.routes.clone();
        assert!(g.assign_route(1, Some(2)).is_err());
        assert_eq!(g.routes, before);
        g.assign_route(1, Some(1)).unwrap();
        g.assign_route(2, None).unwrap();
        g.assign_route(1, Some(2)).unwrap();
        g.assign_route(2, Some(1)).unwrap();
        g.delete_route(1).unwrap();
        g.assign_route(2, Some(2)).unwrap();
        assert!(g.delete_route(999).is_err());
        assert!(g.assign_route(999, None).is_err());
        g.validate_routes().unwrap();
        g.found("Fresh", "DE", "EDDF").unwrap();
        assert!(g.routes.is_empty());
        assert_eq!(g.next_route_id, 1);
    }
}
