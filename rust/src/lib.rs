mod aircraft;
mod airports;
mod clock;
mod game;
mod operations;
mod routes;
mod save;

use game::Game;
use godot::prelude::*;

struct AirlineSimulation;

// SAFETY: All extension classes are registered through godot-rust's macros.
#[gdextension]
unsafe impl ExtensionLibrary for AirlineSimulation {}

/// Thin Godot adapter. Historical class name kept for existing callers.
#[derive(GodotClass)]
#[class(base = RefCounted)]
struct SimulationClock {
    game: Game,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for SimulationClock {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            game: Game::default(),
            base,
        }
    }
}

#[godot_api]
impl SimulationClock {
    #[func]
    fn schedule_route(
        &mut self,
        route_id: u32,
        delay_minutes: u32,
        interval_minutes: u32,
    ) -> GString {
        self.game
            .schedule(route_id, delay_minutes, interval_minutes)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn stop_schedule(&mut self, route_id: u32) -> GString {
        self.game
            .stop_schedule(route_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_operations(&self) -> Array<VarDictionary> {
        let mut rows = Vec::new();
        for route in &self.game.routes {
            let Some(plane) = self
                .game
                .fleet
                .iter()
                .find(|p| Some(p.id) == route.aircraft_id)
            else {
                continue;
            };
            let Ok((flight, ground)) = operations::timings(route, &plane.model_id) else {
                continue;
            };
            let state =
                operations::movement(route, &plane.model_id, self.game.clock.elapsed().as_secs())
                    .ok();
            let text = if let Some(s) = &state {
                let phase = match s.status {
                    aircraft::AircraftStatus::InFlight => format!(
                        "Unterwegs {} → {} · Landung {}",
                        s.origin,
                        s.destination,
                        operations::display_time(s.landing)
                    ),
                    aircraft::AircraftStatus::Ground => format!(
                        "Gelandet in {} · Bodenzeit bis {}",
                        s.location,
                        operations::display_time(s.ready)
                    ),
                    aircraft::AircraftStatus::Parked => {
                        format!("Geplant · bereit in {}", s.location)
                    }
                };
                format!("{} · {} Landungen", phase, s.completed)
            } else {
                "Kein Flugplan".into()
            };
            rows.push((state.as_ref().map(|s| s.next_departure).unwrap_or(u64::MAX), vdict! {
                "route_id" => route.id, "aircraft_id" => plane.id,
                "identifier" => plane.identifier().as_str(), "status_text" => text.as_str(),
                "has_plan" => route.plan.is_some(),
                "flight_seconds" => flight as i64, "ground_seconds" => ground as i64,
                "completed_flights" => state.as_ref().map(|s| s.completed as i64).unwrap_or(0),
                "next_departure_seconds" => state.as_ref().map(|s| s.next_departure as i64).unwrap_or(-1),
                "next_departure" => state.as_ref().map(|s| operations::display_time(s.next_departure)).unwrap_or_else(|| "—".into()).as_str(),
                "landing_seconds" => state.as_ref().map(|s| s.landing as i64).unwrap_or(-1),
                "ready_seconds" => state.as_ref().map(|s| s.ready as i64).unwrap_or(-1),
            }));
        }
        rows.sort_by_key(|(time, _)| *time);
        rows.into_iter().map(|(_, row)| row).collect()
    }

    #[func]
    fn preview_route(
        &self,
        origin: GString,
        destination: GString,
        aircraft_id: u32,
        route_id: u32,
    ) -> VarDictionary {
        let a = origin.to_string();
        let b = destination.to_string();
        let distance = routes::distance_km(&a, &b).unwrap_or(0.0);
        let error = self
            .game
            .check_route(&a, &b, aircraft_id, (route_id != 0).then_some(route_id))
            .err()
            .unwrap_or_default();
        vdict! { "distance_km" => distance, "error" => error.as_str() }
    }

    #[func]
    fn create_route(&mut self, origin: GString, destination: GString, aircraft_id: u32) -> GString {
        self.game
            .create_route(&origin.to_string(), &destination.to_string(), aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn assign_route(&mut self, route_id: u32, aircraft_id: u32) -> GString {
        self.game
            .assign_route(route_id, (aircraft_id != 0).then_some(aircraft_id))
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn delete_route(&mut self, route_id: u32) -> GString {
        self.game
            .delete_route(route_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_routes(&self) -> Array<VarDictionary> {
        self.game.routes.iter().map(|r| vdict! {
            "id" => r.id, "origin_icao" => r.origin_icao.as_str(),
            "destination_icao" => r.destination_icao.as_str(),
            "distance_km" => routes::distance_km(&r.origin_icao, &r.destination_icao).unwrap_or(0.0),
            "aircraft_id" => r.aircraft_id.unwrap_or(0), "has_plan" => r.plan.is_some()
        }).collect()
    }

    #[func]
    fn get_aircraft_models(&self) -> Array<VarDictionary> {
        let Ok(models) = aircraft::models() else {
            return Array::new();
        };
        models.iter().map(|model| vdict! {
            "id" => model.id.as_str(), "name" => model.name.as_str(),
            "seats" => model.seats, "range_km" => model.range_km, "price_eur" => model.price_eur
        }).collect()
    }

    #[func]
    fn buy_aircraft(&mut self, model_id: GString) -> GString {
        match self.game.buy_aircraft(&model_id.to_string()) {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn get_fleet(&self) -> Array<VarDictionary> {
        let airports = airports::catalog().ok();
        self.game
            .fleet
            .iter()
            .map(|plane| {
                let name = aircraft::model(&plane.model_id)
                    .map(|model| model.name.as_str())
                    .unwrap_or("");
                let location_name = airports
                    .and_then(|catalog| {
                        catalog
                            .airports
                            .iter()
                            .find(|airport| airport.icao == plane.location_icao)
                    })
                    .map(|airport| airport.name.as_str())
                    .unwrap_or("");
                vdict! {
                    "id" => plane.id, "identifier" => plane.identifier().as_str(),
                    "model_id" => plane.model_id.as_str(), "model_name" => name,
                    "location_icao" => plane.location_icao.as_str(),
                    "location_name" => location_name,
                    "status" => match plane.status { aircraft::AircraftStatus::Parked => "parked", aircraft::AircraftStatus::InFlight => "in_flight", aircraft::AircraftStatus::Ground => "ground" },
                    "purchase_price_eur" => plane.purchase_price_eur,
                    "range_km" => aircraft::model(&plane.model_id).map(|m| m.range_km).unwrap_or(0),
                    "route_id" => self.game.routes.iter().find(|r| r.aircraft_id == Some(plane.id)).map(|r| r.id).unwrap_or(0)
                }
            })
            .collect()
    }

    #[func]
    fn get_countries(&self) -> Array<VarDictionary> {
        let Ok(catalog) = airports::catalog() else {
            return Array::new();
        };
        catalog
            .countries
            .iter()
            .map(|country| {
                vdict! {
                    "code" => country.code.as_str(), "name" => country.name.as_str()
                }
            })
            .collect()
    }

    #[func]
    fn get_airports(&self, country_code: GString) -> Array<VarDictionary> {
        let Ok(catalog) = airports::catalog() else {
            return Array::new();
        };
        catalog
            .airports
            .iter()
            .filter(|a| a.country_code == country_code.to_string())
            .map(|airport| {
                vdict! {
                    "icao" => airport.icao.as_str(),
                    "name" => airport.name.as_str(),
                    "iata" => airport.iata.as_deref().unwrap_or("")
                }
            })
            .collect()
    }

    #[func]
    fn get_starting_capital(&self) -> GString {
        match game::starting_capital() {
            Ok(amount) => format!("{amount} EUR").as_str().into(),
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn has_airline(&self) -> bool {
        self.game.airline.is_some()
    }

    #[func]
    fn found_airline(&mut self, name: GString, country: GString, airport: GString) -> GString {
        match self.game.found(
            &name.to_string(),
            &country.to_string(),
            &airport.to_string(),
        ) {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn get_airline_info(&self) -> VarDictionary {
        let Some(airline) = &self.game.airline else {
            return VarDictionary::new();
        };
        let Ok(catalog) = airports::catalog() else {
            return VarDictionary::new();
        };
        let country = catalog
            .countries
            .iter()
            .find(|c| c.code == airline.country_code);
        let airport = catalog
            .airports
            .iter()
            .find(|a| a.icao == airline.home_airport_icao);
        vdict! {
            "name" => airline.name.as_str(),
            "country_code" => airline.country_code.as_str(),
            "country_name" => country.map(|c| c.name.as_str()).unwrap_or(""),
            "airport_icao" => airline.home_airport_icao.as_str(),
            "airport_name" => airport.map(|a| a.name.as_str()).unwrap_or(""),
            "capital_eur" => airline.capital_eur
        }
    }

    /// Empty string means success; errors are safe, user-visible messages.
    #[func]
    fn save_game(&self, path: GString) -> GString {
        match save::save(std::path::Path::new(&path.to_string()), &self.game) {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn load_game(&mut self, path: GString) -> GString {
        match save::load(std::path::Path::new(&path.to_string())) {
            Ok(game) => {
                self.game = game;
                GString::new()
            }
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn start(&mut self) {
        self.game.clock.set_running(true);
    }

    #[func]
    fn pause(&mut self) {
        self.game.clock.set_running(false);
    }

    #[func]
    fn is_running(&self) -> bool {
        self.game.clock.is_running()
    }

    #[func]
    fn set_speed(&mut self, multiplier: i64) -> bool {
        u32::try_from(multiplier).is_ok_and(|speed| self.game.clock.set_speed(speed))
    }

    #[func]
    fn get_speed(&self) -> i64 {
        i64::from(self.game.clock.speed())
    }

    #[func]
    fn advance(&mut self, real_seconds: f64) -> bool {
        self.game.advance(real_seconds)
    }

    #[func]
    fn get_elapsed_seconds(&self) -> f64 {
        self.game.clock.elapsed().as_secs_f64()
    }

    #[func]
    fn get_display_time(&self) -> GString {
        self.game.clock.display_time().as_str().into()
    }
}
