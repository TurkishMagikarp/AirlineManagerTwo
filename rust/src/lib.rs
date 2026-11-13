mod aircraft;
mod airports;
mod clock;
mod competition;
mod demand;
mod economy;
mod game;
mod history;
mod maintenance;
mod map;
mod operations;
#[cfg(test)]
mod quality;
mod routes;
mod save;
mod staff;
#[cfg(test)]
mod staff_tests;

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
    /// Departure snapshots, not live estimates or already settled finances.
    #[func]
    fn get_active_flights(&self) -> Array<VarDictionary> {
        self.game
            .routes
            .iter()
            .filter_map(|r| r.plan.as_ref()?.airborne.as_ref())
            .map(flight_dictionary)
            .collect()
    }

    #[func]
    fn get_finance_history_info(&self) -> VarDictionary {
        self.game
            .history
            .as_ref()
            .map(|h| {
                vdict! {
                    "since" => operations::display_time(h.since).as_str(),
                    "batch_count" => h.batches.len() as i64
                }
            })
            .unwrap_or_default()
    }

    #[func]
    fn get_route_finances(&self) -> Array<VarDictionary> {
        let Some(h) = &self.game.history else {
            return Array::new();
        };
        let mut rows = std::collections::BTreeMap::new();
        for r in &self.game.routes {
            rows.insert(
                r.id,
                (
                    r.origin_icao.as_str(),
                    r.destination_icao.as_str(),
                    true,
                    economy::Totals::default(),
                ),
            );
        }
        for b in &h.batches {
            let row = rows.entry(b.route_id).or_insert((
                b.origin.as_str(),
                b.destination.as_str(),
                false,
                economy::Totals::default(),
            ));
            // Archive totals are validated on load; skip rather than panic over FFI.
            let _ = row.3.add(&b.totals, b.count);
        }
        rows.into_iter()
            .map(|(id, (a, b, active, t))| {
                vdict! {
                    "route_id" => id, "origin" => a, "destination" => b, "active" => active,
                    "flights" => t.flights, "revenue_eur" => t.revenue, "costs_eur" => t.costs(),
                    "profit_eur" => t.profit(), "load_percent" => t.load_percent()
                }
            })
            .collect()
    }

    #[func]
    fn get_route_flight_page(&self, route_id: u32, offset: i64) -> Array<VarDictionary> {
        let Some(h) = &self.game.history else {
            return Array::new();
        };
        h.page(route_id, offset.max(0) as u64, 50)
            .iter()
            .map(flight_dictionary)
            .collect()
    }

    #[func]
    fn get_finance_series(&self, route_id: u32) -> Array<VarDictionary> {
        let Some(h) = &self.game.history else {
            return Array::new();
        };
        let now = self.game.clock.elapsed().as_secs();
        let span = now - h.since;
        let steps = span.clamp(1, 63);
        let mut previous = economy::Totals::default();
        (0..=steps).map(|i| {
            let time = h.since + (u128::from(span) * u128::from(i) / u128::from(steps)) as u64;
            let t = h.totals(route_id, time);
            let seats = t.seats - previous.seats;
            let load = if seats == 0 { -1.0 } else { (t.passengers - previous.passengers) as f64 * 100.0 / seats as f64 };
            let row = vdict! { "seconds" => time as i64, "time" => operations::display_time(time).as_str(),
                "balance" => h.balance(time), "revenue" => t.revenue - previous.revenue,
                "costs" => t.costs() - previous.costs(), "profit" => t.profit() - previous.profit(),
                "load" => load };
            previous = t;
            row
        }).collect()
    }

    #[func]
    fn get_map_airports(&self) -> Array<VarDictionary> {
        airports::catalog()
            .map(|c| {
                c.airports
                    .iter()
                    .map(|a| {
                        vdict! {
                            "icao" => a.icao.as_str(), "name" => a.name.as_str(),
                            "position" => Vector2::new(a.longitude as f32, a.latitude as f32)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[func]
    fn get_map_routes(&self) -> Array<VarDictionary> {
        self.game
            .routes
            .iter()
            .filter_map(|r| {
                let a = map::airport(&r.origin_icao)?;
                let b = map::airport(&r.destination_icao)?;
                let points: PackedVector2Array = (0..=128)
                    .map(|i| {
                        let (lon, lat) = map::position(a, b, f64::from(i) / 128.0);
                        Vector2::new(lon as f32, lat as f32)
                    })
                    .collect();
                Some(vdict! { "id" => r.id, "points" => &points, "active" => r.plan.is_some() })
            })
            .collect()
    }

    #[func]
    fn get_map_flights(&self) -> Array<VarDictionary> {
        map::flights(&self.game)
            .iter()
            .filter_map(|f| {
                let p = self.game.fleet.iter().find(|p| p.id == f.aircraft_id)?;
                let model = aircraft::model(&p.model_id).ok()?;
                Some(vdict! { "aircraft_id" => f.aircraft_id, "route_id" => f.route_id,
                "flight_number" => format!("AM-{}-{}", f.route_id, f.departure).as_str(),
                "identifier" => p.identifier().as_str(), "model" => model.name.as_str(),
                "origin" => f.origin.as_str(), "destination" => f.destination.as_str(),
                "progress" => f.progress, "arrival" => operations::display_time(f.landing).as_str(),
                "position" => Vector2::new(f.position.0 as f32, f.position.1 as f32),
                "ahead" => Vector2::new(f.ahead.0 as f32, f.ahead.1 as f32) })
            })
            .collect()
    }

    #[func]
    fn get_economy_rules(&self) -> GString {
        economy::rules_text().as_str().into()
    }

    #[func]
    fn set_ticket_price(&mut self, route_id: u32, price: i64) -> GString {
        self.game
            .set_ticket_price(route_id, price)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_financial_summary(&self) -> VarDictionary {
        let t = &self.game.finance.totals;
        let purchases: i64 = self
            .game
            .fleet
            .iter()
            .map(|p| i128::from(p.purchase_price_eur))
            .sum::<i128>()
            .min(i128::from(i64::MAX)) as i64;
        let maintenance = self
            .game
            .history
            .as_ref()
            .map(|h| h.maintenance_through(u64::MAX).min(i128::from(i64::MAX)) as i64)
            .unwrap_or(0);
        let payroll = self
            .game
            .history
            .as_ref()
            .map(|h| h.payroll_through(u64::MAX).min(i128::from(i64::MAX)) as i64)
            .unwrap_or(0);
        vdict! { "flights" => t.flights, "passengers" => t.passengers, "revenue_eur" => t.revenue,
        "maintenance_eur" => maintenance, "payroll_eur" => payroll,
        "fuel_eur" => t.fuel, "airport_eur" => t.airport, "crew_eur" => t.crew, "overhead_eur" => t.overhead,
        "costs_eur" => t.costs(), "profit_eur" => t.profit(), "purchases_eur" => purchases,
        "load_percent" => t.load_percent() }
    }

    #[func]
    fn get_flight_results(&self) -> Array<VarDictionary> {
        self.game
            .finance
            .recent
            .iter()
            .rev()
            .map(flight_dictionary)
            .collect()
    }

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
                    aircraft::AircraftStatus::Parked | aircraft::AircraftStatus::Maintenance => {
                        format!("Geplant · bereit in {}", s.location)
                    }
                };
                let prior = route.plan.as_ref().map(|p| p.prior_flights).unwrap_or(0);
                let hangar = plane
                    .maintenance
                    .as_ref()
                    .map(|v| {
                        format!(
                            " · Wartung ab Landung bis {}",
                            operations::display_time(v.end)
                        )
                    })
                    .unwrap_or_default();
                format!("{} · {} Landungen{}", phase, s.completed + prior, hangar)
            } else if let Some(v) = plane
                .maintenance
                .as_ref()
                .filter(|_| plane.status == aircraft::AircraftStatus::Maintenance)
            {
                let resume = v.resume.as_ref().is_some_and(|r| r.route_id == route.id);
                format!(
                    "In Wartung bis {}{}",
                    operations::display_time(v.end),
                    if resume {
                        " · Flugplan wird danach fortgesetzt"
                    } else {
                        ""
                    }
                )
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
    fn get_competitors(&self) -> Array<VarDictionary> {
        self.game
            .competitors
            .iter()
            .map(|c| {
                let g = &c.game;
                let routes = g
                    .routes
                    .iter()
                    .map(|r| {
                        format!(
                            "{} ↔ {} · {} EUR · {}",
                            r.origin_icao,
                            r.destination_icao,
                            r.ticket_eur,
                            if r.plan.is_some() { "aktiv" } else { "inaktiv" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let fleet = g
                    .fleet
                    .iter()
                    .map(|p| {
                        let condition = maintenance::rules(&p.model_id)
                            .map(|r| p.wear.condition_permille(r) as f64 / 10.0)
                            .unwrap_or(0.0);
                        let hangar = match (&p.maintenance, &p.status) {
                            (Some(v), aircraft::AircraftStatus::Maintenance) => {
                                format!(" · in Wartung bis {}", operations::display_time(v.end))
                            }
                            (Some(_), _) => " · Wartung nach Landung".into(),
                            _ => String::new(),
                        };
                        format!(
                            "{} · {} · {} · Kauf {} EUR · Zustand {:.0} % · {} Checks{}",
                            p.identifier(),
                            p.model_id,
                            p.location_icao,
                            p.purchase_price_eur,
                            condition,
                            p.wear.checks,
                            hangar
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let purchases = g
                    .history
                    .as_ref()
                    .unwrap()
                    .purchases
                    .iter()
                    .map(|p| format!("{}: {} EUR", operations::display_time(p.time), p.amount))
                    .collect::<Vec<_>>()
                    .join(" · ");
                vdict! { "airline" => g.airline.as_ref().unwrap().name.as_str(),
                "balance" => g.airline.as_ref().unwrap().capital_eur,
                "fleet_size" => g.fleet.len() as i64, "routes" => routes.as_str(),
                "maintenance_eur" => g.history.as_ref().map(|h| h.maintenance_through(u64::MAX).min(i128::from(i64::MAX)) as i64).unwrap_or(0),
                "fleet" => fleet.as_str(), "purchases" => purchases.as_str(),
                "last_action" => c.last_action.as_str() }
            })
            .collect()
    }

    #[func]
    fn get_route_competition(&self, route_id: u32) -> Array<VarDictionary> {
        let Some(route) = self.game.routes.iter().find(|r| r.id == route_id) else {
            return Array::new();
        };
        struct Offer<'a> {
            airline: &'a str,
            route: &'a routes::Route,
            model: &'a str,
            seats: u32,
            /// Departures per day in each direction.
            frequency: f64,
            passengers: f64,
            action: &'a str,
        }
        let supply = self.game.market_routes();
        let mut offers = Vec::new();
        for (company, action) in std::iter::once((&self.game, "Spieler")).chain(
            self.game
                .competitors
                .iter()
                .map(|c| (&*c.game, c.last_action.as_str())),
        ) {
            let Some(airline) = &company.airline else {
                continue;
            };
            for r in company
                .routes
                .iter()
                .filter(|r| demand::same_market(route, r))
            {
                let Some(plane) = company.fleet.iter().find(|p| Some(p.id) == r.aircraft_id) else {
                    continue;
                };
                let Ok(model) = aircraft::model(&plane.model_id) else {
                    continue;
                };
                let frequency = r
                    .plan
                    .as_ref()
                    .map(|p| 43200.0 / p.interval_seconds as f64)
                    .unwrap_or(0.0);
                let passengers = demand::passengers(r, &supply)
                    .unwrap_or(0)
                    .min(i64::from(model.seats)) as f64
                    * frequency;
                offers.push(Offer {
                    airline: airline.name.as_str(),
                    route: r,
                    model: &plane.model_id,
                    seats: model.seats,
                    frequency,
                    passengers,
                    action,
                });
            }
        }
        let total: f64 = offers.iter().map(|o| o.passengers).sum();
        offers
            .into_iter()
            .map(|o| {
                vdict! {
                    "airline" => o.airline, "ticket_eur" => o.route.ticket_eur, "model" => o.model,
                    "seats" => o.seats, "departures_per_day_per_direction" => o.frequency,
                    "passengers_per_day" => o.passengers,
                    "share_percent" => if total > 0.0 { o.passengers * 100.0 / total } else { 0.0 },
                    "active" => o.route.plan.is_some(), "last_action" => o.action
                }
            })
            .collect()
    }

    #[func]
    fn get_routes(&self) -> Array<VarDictionary> {
        let supply = self.game.market_routes();
        let now = self.game.clock.elapsed().as_secs();
        self.game
            .routes
            .iter()
            .filter_map(|r| {
                let market = demand::market(r, r.ticket_eur).ok()?;
                let base = demand::market(r, 0).ok()?;
                let actual = self
                    .game
                    .history
                    .as_ref()
                    .map(|h| h.totals(r.id, now))
                    .unwrap_or_default();
                Some(vdict! {
                    "id" => r.id, "origin_icao" => r.origin_icao.as_str(),
                    "destination_icao" => r.destination_icao.as_str(),
                    "distance_km" => routes::distance_km(&r.origin_icao, &r.destination_icao).unwrap_or(0.0),
                    "aircraft_id" => r.aircraft_id.unwrap_or(0), "has_plan" => r.plan.is_some(), "ticket_eur" => r.ticket_eur,
                    "daily_demand" => market.total().floor() as i64,
                    "base_daily_market" => base.total().floor() as i64,
                    "daily_business" => market.business.floor() as i64,
                    "daily_leisure" => market.leisure.floor() as i64,
                    "estimated_passengers" => if r.plan.is_some() { demand::passengers(r, &supply).unwrap_or(0) } else { 0 },
                    "actual_flights" => actual.flights, "actual_load_percent" => actual.load_percent(),
                    "maintenance_status" => self.route_maintenance_status(r).as_str(),
                    "staff_status" => self.route_staff_status(r).as_str(),
                    "can_schedule" => r.plan.is_none() && r.aircraft_id.is_some() && self.route_maintenance_status(r).is_empty() && self.route_staff_status(r).is_empty()
                })
            })
            .collect()
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
                    "status" => match plane.status { aircraft::AircraftStatus::Parked => "parked", aircraft::AircraftStatus::InFlight => "in_flight", aircraft::AircraftStatus::Ground => "ground", aircraft::AircraftStatus::Maintenance => "maintenance" },
                    "purchase_price_eur" => plane.purchase_price_eur,
                    "range_km" => aircraft::model(&plane.model_id).map(|m| m.range_km).unwrap_or(0),
                    "route_id" => self.game.routes.iter().find(|r| r.aircraft_id == Some(plane.id)).map(|r| r.id).unwrap_or(0)
                }
            })
            .zip(self.game.fleet.iter().enumerate())
            .map(|(mut row, (p, plane))| {
                row.extend_dictionary(&self.maintenance_info(p, plane), true);
                row
            })
            .collect()
    }

    /// Empty string means success. Airborne aircraft enter the hangar after landing.
    #[func]
    fn request_maintenance(&mut self, aircraft_id: u32) -> GString {
        self.game
            .request_maintenance(aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    /// Empty string means success. Only orders that have not started yet.
    #[func]
    fn cancel_maintenance(&mut self, aircraft_id: u32) -> GString {
        self.game
            .cancel_maintenance(aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    /// Every employee with role, salary and derived employment status.
    #[func]
    fn get_staff(&self) -> Array<VarDictionary> {
        let now = self.game.clock.elapsed().as_secs();
        let activity = self.game.staff_activity();
        self.game
            .staff
            .employees
            .iter()
            .zip(activity)
            .map(|(e, a)| {
                vdict! {
                    "id" => e.id, "name" => e.name.as_str(),
                    "role" => e.role.key(), "role_name" => e.role.label(),
                    "salary_eur" => e.salary_eur,
                    "status" => match a { staff::Activity::Available => "available", staff::Activity::Flying => "flying", staff::Activity::Hangar => "hangar" },
                    "status_text" => a.label(),
                    "can_dismiss" => a == staff::Activity::Available,
                    "hired" => operations::display_time(e.hired_at).as_str(),
                    "days_employed" => (now.saturating_sub(e.hired_at) / 86_400) as i64
                }
            })
            .collect()
    }

    /// Per-role headcount, shortages and what blocks each aircraft.
    #[func]
    fn get_staff_overview(&self) -> VarDictionary {
        let g = &self.game;
        let (idle_pilots, idle_cabin) = self.idle_crew_demand();
        let waiting_techs: u32 = g
            .fleet
            .iter()
            .filter(|p| {
                p.status == aircraft::AircraftStatus::Maintenance
                    && p.maintenance.as_ref().is_some_and(|v| !v.staffed)
            })
            .filter_map(|p| staff::needs(&p.model_id).ok())
            .map(|n| n.technicians)
            .sum();
        let mut roles = Array::<VarDictionary>::new();
        let mut total_missing = 0i64;
        for role in staff::Role::ALL {
            let employed = g.staff_count(role);
            let available = g.available_staff(role);
            let missing = match role {
                staff::Role::Pilot => idle_pilots.saturating_sub(available),
                staff::Role::CabinCrew => idle_cabin.saturating_sub(available),
                staff::Role::Technician => g
                    .technician_target()
                    .saturating_sub(employed)
                    .max(waiting_techs.saturating_sub(available)),
            };
            total_missing += i64::from(missing);
            roles.push(&vdict! {
                "role" => role.key(), "name" => role.plural(), "singular" => role.label(),
                "employed" => i64::from(employed), "busy" => i64::from(g.busy_staff(role)),
                "available" => i64::from(available), "missing" => i64::from(missing),
                "salary_eur" => staff::salary(role).unwrap_or(0),
                "monthly_cost_eur" => g.staff.employees.iter().filter(|e| e.role == role).map(|e| e.salary_eur).sum::<i64>()
            });
        }
        let next_pay = g
            .next_payroll_event()
            .map(operations::display_time)
            .unwrap_or_else(|| "—".to_string());
        let mut aircraft_rows = Array::<VarDictionary>::new();
        for p in &g.fleet {
            let Ok(n) = staff::needs(&p.model_id) else {
                continue;
            };
            let reserved = g.planned_route(p.id).is_some()
                || p.maintenance.as_ref().is_some_and(|v| v.resume.is_some());
            let (state, problem) = match &p.maintenance {
                Some(v) if p.status == aircraft::AircraftStatus::Maintenance && !v.staffed => (
                    "waiting_technicians",
                    format!(
                        "Wartung wartet auf {} Techniker (frei: {})",
                        n.technicians,
                        g.available_staff(staff::Role::Technician)
                    ),
                ),
                Some(_) if p.status == aircraft::AircraftStatus::Maintenance => {
                    ("hangar", String::new())
                }
                _ if reserved => ("crewed", String::new()),
                _ => match g.require_crew(&p.model_id) {
                    Ok(()) => ("ready", String::new()),
                    Err(e) => ("short", e),
                },
            };
            aircraft_rows.push(&vdict! {
                "id" => p.id, "identifier" => p.identifier().as_str(),
                "model_name" => aircraft::model(&p.model_id).map(|m| m.name.as_str()).unwrap_or(""),
                "pilots" => i64::from(n.pilots), "cabin_crew" => i64::from(n.cabin_crew),
                "technicians" => i64::from(n.technicians),
                "state" => state, "problem" => problem.as_str()
            });
        }
        let mut overview = vdict! {
            "employees" => g.staff.employees.len() as i64,
            "monthly_payroll_eur" => g.monthly_payroll(),
            "next_payroll" => next_pay.as_str(),
            "missing_total" => total_missing,
            "payroll_paid_eur" => g.history.as_ref().map(|h| h.payroll_through(u64::MAX).min(i128::from(i64::MAX)) as i64).unwrap_or(0),
            "max_batch" => i64::from(staff::max_hire_batch())
        };
        overview.set("roles", &roles);
        overview.set("aircraft", &aircraft_rows);
        overview
    }

    /// Empty string means success.
    #[func]
    fn hire_staff(&mut self, role: GString, count: i64) -> GString {
        let result = staff::Role::parse(&role.to_string()).and_then(|role| {
            u32::try_from(count)
                .map_err(|_| "Ungültige Anzahl.".to_string())
                .and_then(|n| self.game.hire_staff(role, n))
        });
        result.err().unwrap_or_default().as_str().into()
    }

    /// Hire exactly the staff missing for aircraft that cannot fly yet.
    #[func]
    fn hire_missing_staff(&mut self) -> GString {
        let g = &self.game;
        let (pilots, cabin) = self.idle_crew_demand();
        let techs = g
            .technician_target()
            .saturating_sub(g.staff_count(staff::Role::Technician));
        let hires = [
            (
                staff::Role::Pilot,
                pilots.saturating_sub(g.available_staff(staff::Role::Pilot)),
            ),
            (
                staff::Role::CabinCrew,
                cabin.saturating_sub(g.available_staff(staff::Role::CabinCrew)),
            ),
            (staff::Role::Technician, techs),
        ];
        if hires.iter().all(|h| h.1 == 0) {
            return "Es fehlt kein Personal.".into();
        }
        self.game
            .hire_batch(&hires, true)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    /// Empty string means success. Only available (not assigned) staff.
    #[func]
    fn dismiss_staff(&mut self, employee_id: u32) -> GString {
        self.game
            .dismiss_staff(employee_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
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

impl SimulationClock {
    /// Why the route's aircraft cannot start a new flight plan; empty if it can.
    fn route_maintenance_status(&self, route: &routes::Route) -> String {
        let Some(plane) = self
            .game
            .fleet
            .iter()
            .find(|p| Some(p.id) == route.aircraft_id)
        else {
            return String::new();
        };
        match &plane.maintenance {
            Some(v) if plane.status == aircraft::AircraftStatus::Maintenance && !v.staffed => {
                "In Wartung · wartet auf Techniker (Personal-Tab)".to_string()
            }
            Some(v) if plane.status == aircraft::AircraftStatus::Maintenance => format!(
                "{} bis {}{}",
                if v.unscheduled {
                    "In Pflichtwartung"
                } else {
                    "In Wartung"
                },
                operations::display_time(v.end),
                if v.resume.as_ref().is_some_and(|r| r.route_id == route.id) {
                    " · Flugplan wird danach automatisch fortgesetzt"
                } else {
                    ""
                }
            ),
            Some(v) => format!(
                "Wartung beauftragt · beginnt nach der Landung {}",
                operations::display_time(v.start)
            ),
            None if route.plan.is_none()
                && maintenance::rules(&plane.model_id).is_ok_and(|r| plane.wear.is_due(r)) =>
            {
                "Wartung erforderlich (Zustand 0 %) · im Flotte-Tab warten".into()
            }
            None => String::new(),
        }
    }

    /// Crew needed by parked aircraft without a flight plan.
    fn idle_crew_demand(&self) -> (u32, u32) {
        let g = &self.game;
        let (mut pilots, mut cabin) = (0, 0);
        for p in g.fleet.iter().filter(|p| {
            p.status == aircraft::AircraftStatus::Parked
                && g.planned_route(p.id).is_none()
                && p.maintenance.is_none()
        }) {
            if let Ok(n) = staff::needs(&p.model_id) {
                pilots += n.pilots;
                cabin += n.cabin_crew;
            }
        }
        (pilots, cabin)
    }

    /// Missing crew that keeps the route's aircraft from flying; empty if none.
    fn route_staff_status(&self, route: &routes::Route) -> String {
        let Some(plane) = self
            .game
            .fleet
            .iter()
            .find(|p| Some(p.id) == route.aircraft_id)
        else {
            return String::new();
        };
        if route.plan.is_some() || plane.maintenance.is_some() {
            return String::new();
        }
        self.game
            .require_crew(&plane.model_id)
            .err()
            .unwrap_or_default()
    }

    fn maintenance_info(&self, p: usize, plane: &aircraft::Aircraft) -> VarDictionary {
        let Ok(r) = maintenance::rules(&plane.model_id) else {
            return VarDictionary::new();
        };
        let w = &plane.wear;
        let hours = |s: u64| s as f64 / 3600.0;
        let quote = self.game.quote_maintenance(plane.id);
        let (state, visit) = match &plane.maintenance {
            Some(v) if plane.status == aircraft::AircraftStatus::Maintenance => ("active", Some(v)),
            Some(v) => ("pending", Some(v)),
            None => ("none", None),
        };
        let shown = visit.cloned().or_else(|| quote.as_ref().ok().cloned());
        let due = self.game.due_landing(p);
        vdict! {
            "condition_percent" => w.condition_permille(r) as f64 / 10.0,
            "flight_hours" => hours(w.flight_seconds), "cycles" => w.cycles as i64,
            "checks" => w.checks as i64,
            "check_flight_hours" => hours(w.check_seconds), "check_cycles" => w.check_cycles as i64,
            "interval_flight_hours" => r.interval_flight_hours as i64,
            "interval_cycles" => r.interval_cycles as i64,
            "maintenance_due" => w.is_due(r),
            "maintenance_state" => state,
            "maintenance_waiting_staff" => visit.is_some_and(|v| !v.staffed) && plane.status == aircraft::AircraftStatus::Maintenance,
            "maintenance_unscheduled" => visit.is_some_and(|v| v.unscheduled),
            "maintenance_start" => shown.as_ref().map(|v| operations::display_time(v.start)).unwrap_or_default().as_str(),
            "maintenance_end" => shown.as_ref().map(|v| operations::display_time(v.end)).unwrap_or_default().as_str(),
            "maintenance_end_seconds" => shown.as_ref().map(|v| v.end as i64).unwrap_or(-1),
            "maintenance_cost_eur" => shown.as_ref().map(|v| v.cost_eur).unwrap_or_else(|| maintenance::cost(w, r, false).unwrap_or(0)),
            "maintenance_hours" => maintenance::duration(r, false).unwrap_or(0) as f64 / 3600.0,
            "unscheduled_cost_eur" => maintenance::cost(w, r, true).unwrap_or(0),
            "unscheduled_hours" => maintenance::duration(r, true).unwrap_or(0) as f64 / 3600.0,
            "due_time" => due.map(operations::display_time).unwrap_or_default().as_str(),
            "can_request_maintenance" => quote.is_ok(),
            "can_cancel_maintenance" => state == "pending" && !visit.is_some_and(|v| v.unscheduled),
            "maintenance_hint" => quote.err().unwrap_or_default().as_str()
        }
    }
}

fn flight_dictionary(f: &economy::FlightResult) -> VarDictionary {
    let t = &f.totals;
    vdict! { "route_id" => f.route_id, "landing" => operations::display_time(f.landing).as_str(),
    "origin" => f.origin.as_str(), "destination" => f.destination.as_str(), "ticket_eur" => f.ticket_eur,
    "passengers" => t.passengers, "seats" => t.seats, "load_percent" => t.load_percent(),
    "revenue_eur" => t.revenue, "fuel_eur" => t.fuel, "airport_eur" => t.airport,
    "crew_eur" => t.crew, "overhead_eur" => t.overhead, "costs_eur" => t.costs(), "profit_eur" => t.profit() }
}
