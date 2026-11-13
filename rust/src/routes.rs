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
            ticket_eur: crate::economy::default_ticket(),
            plan: None,
        });
        self.next_route_id = next;
        Ok(())
    }

    pub fn assign_route(&mut self, route_id: u32, aircraft_id: Option<u32>) -> Result<(), String> {
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

    pub fn route_mut(&mut self, id: u32) -> Result<&mut Route, String> {
        self.routes
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| "Unbekannte Route.".to_string())
    }

    pub fn delete_route(&mut self, route_id: u32) -> Result<(), String> {
        self.require_no_plan(route_id)?;
        let index = self
            .routes
            .iter()
            .position(|r| r.id == route_id)
            .ok_or_else(|| "Unbekannte Route.".to_string())?;
        self.routes.remove(index);
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
        g.buy_aircraft("atr72_600").unwrap();
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
