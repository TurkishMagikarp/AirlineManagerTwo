//! Repeating shuttle service. Integer event boundaries, independent of frame size.
use crate::{
    aircraft::{self, AircraftStatus},
    game::Game,
    routes::Route,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FlightPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disruption: Option<crate::disruptions::Cursor>,
    pub first_departure: u64,
    pub interval_seconds: u64,
    pub completed_flights: u64,
    /// Immutable departure quote. Settled exactly once, at landing.
    #[serde(default)]
    pub airborne: Option<crate::economy::FlightResult>,
    /// Additive (version 10): a plan resumed after maintenance at the
    /// destination starts with the return leg.
    #[serde(default)]
    pub reversed: bool,
    /// Landings before the plan was suspended for maintenance (display only).
    #[serde(default)]
    pub prior_flights: u64,
}

impl FlightPlan {
    /// Whether departure `index` leaves from the route's destination.
    pub fn is_return(&self, index: u64) -> bool {
        (index % 2 == 1) != self.reversed
    }
}

#[cfg(test)]
pub fn departure_quote(
    route: &Route,
    model: &str,
    index: u64,
    routes: &[crate::demand::Offer],
) -> Result<crate::economy::FlightResult, String> {
    departure_quote_in_market(
        route,
        model,
        index,
        routes,
        &crate::economy::FuelMarket::default(),
    )
}

pub fn departure_quote_in_market(
    route: &Route,
    model: &str,
    index: u64,
    routes: &[crate::demand::Offer],
    market: &crate::economy::FuelMarket,
) -> Result<crate::economy::FlightResult, String> {
    let plan = route.plan.as_ref().ok_or("Kein Flugplan.")?;
    if index == plan.completed_flights
        && let Some(f) = &plan.airborne
    {
        return Ok(f.clone());
    }
    let landing = crate::disruptions::landing_at(route, model, index)?;
    let departure = landing
        .checked_sub(timings(route, model)?.0)
        .ok_or("Zeitüberlauf.")?;
    let mut f = departure_quote_at_price(
        route,
        model,
        index,
        routes,
        market.price_at(departure),
        departure,
    )?;
    if plan.disruption.is_some() {
        let scheduled = plan
            .first_departure
            .checked_add(
                index
                    .checked_mul(plan.interval_seconds)
                    .ok_or("Zeitüberlauf.")?,
            )
            .ok_or("Zeitüberlauf.")?;
        crate::disruptions::apply(
            &mut f.totals,
            &crate::disruptions::Leg {
                index,
                scheduled,
                departure,
                landing,
                ready: landing,
            },
        )?;
    }
    Ok(f)
}

fn departure_quote_at_price(
    route: &Route,
    model: &str,
    index: u64,
    routes: &[crate::demand::Offer],
    fuel_cents: i64,
    departure: u64,
) -> Result<crate::economy::FlightResult, String> {
    let mut market = crate::economy::QuoteMarket::new(route, model, routes)?;
    departure_quote_prepared(
        route,
        model,
        index,
        routes,
        fuel_cents,
        departure,
        &mut market,
    )
}

fn departure_quote_prepared(
    route: &Route,
    model: &str,
    index: u64,
    routes: &[crate::demand::Offer],
    fuel_cents: i64,
    departure: u64,
    market: &mut crate::economy::QuoteMarket,
) -> Result<crate::economy::FlightResult, String> {
    let plan = route.plan.as_ref().ok_or("Kein Flugplan.")?;
    // The shared market, timings and endpoint bill are symmetric. Only the
    // displayed endpoints reverse; avoid cloning the plan and frozen quote on
    // every departure. Timings are fixed for this prepared route/model pair.
    let (origin, destination) = if plan.is_return(index) {
        (&route.destination_icao, &route.origin_icao)
    } else {
        (&route.origin_icao, &route.destination_icao)
    };
    let flight = market.flight_seconds;
    let landing = departure
        .checked_add(flight)
        .ok_or("Flugplan-Zeitüberlauf.")?;
    Ok(crate::economy::FlightResult {
        route_id: route.id,
        landing,
        origin: origin.clone(),
        destination: destination.clone(),
        ticket_eur: route.ticket_eur,
        totals: crate::economy::quote_prepared(
            route,
            model,
            routes,
            fuel_cents,
            Some(landing - flight),
            market,
        )?,
    })
}

pub struct Movement {
    pub delay_seconds: u64,
    pub status: AircraftStatus,
    pub location: String,
    pub origin: String,
    pub destination: String,
    pub next_departure: u64,
    pub landing: u64,
    pub ready: u64,
    pub completed: u64,
}

pub fn display_time(seconds: u64) -> String {
    format!(
        "Tag {} {:02}:{:02}:{:02}",
        seconds / 86400 + 1,
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}

pub fn timings(route: &Route, model_id: &str) -> Result<(u64, u64), String> {
    let model = aircraft::model(model_id)?;
    let seconds = (crate::routes::distance_km(&route.origin_icao, &route.destination_icao)?
        / f64::from(model.cruise_kmh)
        * 3600.0)
        .ceil() as u64;
    Ok((seconds.max(1), u64::from(model.turnaround_minutes) * 60))
}

pub fn movement(route: &Route, model: &str, now: u64) -> Result<Movement, String> {
    let plan = route.plan.as_ref().ok_or("Kein Flugplan.")?;
    if plan.disruption.is_some() {
        return crate::disruptions::project(route, model, now).map(|p| p.state);
    }
    let (flight, ground) = timings(route, model)?;
    if plan.interval_seconds < flight + ground || plan.first_departure > u64::MAX / 2 {
        return Err(
            "Flugplan benötigt mindestens Flugzeit plus Bodenzeit zwischen Abflügen.".into(),
        );
    }
    let started = now >= plan.first_departure;
    let index = if started {
        (now - plan.first_departure) / plan.interval_seconds
    } else {
        0
    };
    let departure = plan
        .first_departure
        .checked_add(index * plan.interval_seconds)
        .ok_or("Flugplan-Zeitüberlauf.")?;
    let landing = departure
        .checked_add(flight)
        .ok_or("Flugplan-Zeitüberlauf.")?;
    let ready = landing
        .checked_add(ground)
        .ok_or("Flugplan-Zeitüberlauf.")?;
    let (origin, destination) = if !plan.is_return(index) {
        (&route.origin_icao, &route.destination_icao)
    } else {
        (&route.destination_icao, &route.origin_icao)
    };
    let landed = started && now >= landing;
    Ok(Movement {
        delay_seconds: 0,
        status: if !started || now >= ready {
            AircraftStatus::Parked
        } else if landed {
            AircraftStatus::Ground
        } else {
            AircraftStatus::InFlight
        },
        location: if landed { destination } else { origin }.clone(),
        origin: origin.clone(),
        destination: destination.clone(),
        next_departure: if started {
            departure
                .checked_add(plan.interval_seconds)
                .ok_or("Flugplan-Zeitüberlauf.")?
        } else {
            departure
        },
        landing,
        ready,
        completed: index + u64::from(landed),
    })
}

impl Game {
    pub fn require_no_plan(&self, id: u32) -> Result<(), String> {
        if self
            .routes
            .iter()
            .any(|r| r.public_id() == id && r.plan.is_some())
        {
            return Err("Zuerst den Flugplan beenden.".into());
        }
        Ok(())
    }

    pub fn schedule(
        &mut self,
        id: u32,
        delay_minutes: u32,
        interval_minutes: u32,
    ) -> Result<(), String> {
        self.require_no_plan(id)?;
        let services: Vec<_> = self
            .routes
            .iter()
            .filter(|r| r.public_id() == id && r.aircraft_id.is_some())
            .map(|r| r.id)
            .collect();
        if services.is_empty() {
            return Err("Bitte ein Flugzeug zuweisen.".into());
        }
        let mut staged = self.clone();
        for (slot, service) in services.iter().enumerate() {
            staged.schedule_service(*service, delay_minutes, interval_minutes)?;
            let plan = staged.route_mut(*service)?.plan.as_mut().unwrap();
            plan.first_departure = plan
                .first_departure
                .checked_add(slot as u64 * u64::from(interval_minutes) * 60 / services.len() as u64)
                .filter(|t| *t <= u64::MAX / 2)
                .ok_or("Flugplan-Zeitüberlauf.")?;
        }
        *self = staged;
        Ok(())
    }

    pub(crate) fn schedule_service(
        &mut self,
        id: u32,
        delay_minutes: u32,
        interval_minutes: u32,
    ) -> Result<(), String> {
        self.require_solvent()?;
        if self.routes.iter().any(|r| r.id == id && r.plan.is_some()) {
            return Err("Zuerst den Flugplan beenden.".into());
        }
        let route = self
            .routes
            .iter()
            .find(|r| r.id == id)
            .ok_or("Unbekannte Route.")?;
        let plane = self
            .fleet
            .iter()
            .find(|p| Some(p.id) == route.aircraft_id)
            .ok_or("Bitte ein Flugzeug zuweisen.")?;
        if plane.maintenance.is_some() {
            return Err("Das Flugzeug ist in Wartung oder dafür vorgemerkt.".into());
        }
        if plane
            .wear
            .is_due(crate::maintenance::rules(&plane.model_id)?)
        {
            return Err("Wartung erforderlich: Zustand 0 %. Bitte zuerst warten.".into());
        }
        if plane.location_icao != route.origin_icao || plane.status != AircraftStatus::Parked {
            return Err(
                "Das Flugzeug muss am Startflughafen geparkt sein; keine automatische Überführung."
                    .into(),
            );
        }
        self.require_crew(&plane.model_id)?;
        let (flight, ground) = timings(route, &plane.model_id)?;
        let interval = u64::from(interval_minutes) * 60;
        if delay_minutes == 0 || interval < flight + ground {
            return Err(format!(
                "Erster Abflug frühestens in 1 Minute; Abstand mindestens {} Minuten (Flug + Bodenzeit).",
                (flight + ground).div_ceil(60)
            ));
        }
        self.check_slots(&route.origin_icao, &route.destination_icao, interval)?;
        let first = self
            .clock
            .elapsed()
            .as_secs()
            .checked_add(
                u64::from(delay_minutes) * 60 + u64::from(self.clock.elapsed().subsec_nanos() != 0),
            )
            .filter(|t| *t <= u64::MAX / 2)
            .ok_or("Flugplan-Zeitüberlauf.")?;
        self.route_mut(id)?.plan = Some(FlightPlan {
            first_departure: first,
            interval_seconds: interval,
            ..FlightPlan::default()
        });
        self.initialize_disruptions()?;
        Ok(())
    }

    pub fn stop_schedule(&mut self, id: u32) -> Result<(), String> {
        let ids: Vec<_> = self
            .routes
            .iter()
            .filter(|r| r.public_id() == id)
            .map(|r| r.id)
            .collect();
        if ids.is_empty() {
            return Err("Unbekannte Route.".into());
        }
        let mut staged = self.clone();
        for service in ids {
            staged.stop_service(service)?;
        }
        *self = staged;
        Ok(())
    }

    pub(crate) fn stop_service(&mut self, id: u32) -> Result<(), String> {
        self.require_active()?;
        let route = self
            .routes
            .iter()
            .find(|r| r.id == id)
            .ok_or("Unbekannte Route.")?;
        if self
            .fleet
            .iter()
            .find(|p| Some(p.id) == route.aircraft_id)
            .is_some_and(|plane| plane.status != AircraftStatus::Parked)
        {
            return Err("Flugplan erst nach Landung und Bodenzeit beenden.".into());
        }
        let route = self.route_mut(id)?;
        route.plan = None;
        route.rotation_phase = None;
        route.remove_at = None;
        Ok(())
    }

    pub(crate) fn advance_company_to(
        &mut self,
        clock: crate::clock::Clock,
        supply: &[crate::demand::Offer],
    ) -> bool {
        let owned_supply = self.prioritize_market(supply);
        let supply = owned_supply.as_slice();
        let mut updates = Vec::new();
        let mut finance = self.finance.clone();
        let mut delta = 0i64;
        let mut archive = Vec::new();
        for (i, route) in self
            .routes
            .iter()
            .enumerate()
            .filter(|(_, r)| r.plan.is_some())
        {
            let Some(p) = self
                .fleet
                .iter()
                .position(|p| Some(p.id) == route.aircraft_id)
            else {
                return false;
            };
            let plane = &self.fleet[p];
            let Ok(projection) =
                crate::disruptions::project(route, &plane.model_id, clock.elapsed().as_secs())
            else {
                return false;
            };
            let plan = route.plan.as_ref().unwrap();
            let Ok((flight, _)) = timings(route, &plane.model_id) else {
                return false;
            };
            let Some(wear) = plane.wear.added(projection.landed.len() as u64, flight) else {
                return false;
            };
            let mut market = None;
            let (mut fuel_until, mut fuel_cents) = (0, 0);
            let mut quote = |leg: &crate::disruptions::Leg,
                             market: &mut Option<crate::economy::QuoteMarket>|
             -> Result<crate::economy::FlightResult, String> {
                if leg.index == plan.completed_flights
                    && let Some(f) = &plan.airborne
                {
                    return Ok(f.clone());
                }
                if market.is_none() {
                    *market = Some(crate::economy::QuoteMarket::new(
                        route,
                        &plane.model_id,
                        supply,
                    )?);
                }
                // Quote the actual departure time: seasonal demand, fuel prices
                // and contracts are frozen at dispatch, never at landing.
                if leg.departure >= fuel_until {
                    fuel_cents = self.fuel_market.price_at(leg.departure);
                    fuel_until = self.fuel_market.next_change(leg.departure);
                }
                let mut f = departure_quote_prepared(
                    route,
                    &plane.model_id,
                    leg.index,
                    supply,
                    fuel_cents,
                    leg.departure,
                    market.as_mut().unwrap(),
                )?;
                self.apply_fuel_contract(&mut f, &plane.model_id, leg.departure)?;
                crate::disruptions::apply(&mut f.totals, leg)?;
                Ok(f)
            };
            for leg in &projection.landed {
                let Ok(f) = quote(leg, &mut market) else {
                    return false;
                };
                if finance.totals.add(&f.totals, 1).is_none() {
                    return false;
                }
                let Some(change) = delta.checked_add(f.totals.profit()) else {
                    return false;
                };
                delta = change;
                archive.push(crate::history::Batch {
                    route_id: route.id,
                    first: f.landing,
                    interval: plan.interval_seconds,
                    count: 1,
                    origin: f.origin.clone(),
                    destination: f.destination.clone(),
                    ticket_eur: f.ticket_eur,
                    totals: f.totals.clone(),
                });
                finance.recent.push(f);
            }
            let airborne = match &projection.active {
                Some(leg) => match quote(leg, &mut market) {
                    Ok(f) => Some(f),
                    Err(_) => return false,
                },
                None => None,
            };
            updates.push((i, p, projection, airborne, wear));
        }
        let balance = match &self.airline {
            Some(a) => match a.capital_eur.checked_add(delta) {
                Some(v) => Some(v),
                None => return false,
            },
            None if delta == 0 => None,
            None => return false,
        };
        finance.recent.sort_by_key(|f| (f.landing, f.route_id));
        let excess = finance
            .recent
            .len()
            .saturating_sub(crate::economy::HISTORY_LIMIT);
        finance.recent.drain(..excess);
        self.finance = finance;
        if let Some(h) = &mut self.history {
            for batch in archive {
                h.record(batch);
            }
        }
        if let Some(balance) = balance {
            self.airline.as_mut().unwrap().capital_eur = balance;
        }
        self.clock = clock;
        self.complete_training();
        for (r, p, projection, airborne, wear) in updates {
            self.fleet[p].wear = wear;
            self.fleet[p].status = projection.state.status;
            self.fleet[p].location_icao = projection.state.location;
            let plan = self.routes[r].plan.as_mut().unwrap();
            plan.completed_flights = projection.state.completed;
            plan.airborne = airborne;
            plan.disruption = Some(projection.cursor);
        }
        true
    }

    pub fn validate_operations(&self) -> Result<(), String> {
        for plane in &self.fleet {
            if let Some(route) = self
                .routes
                .iter()
                .find(|r| r.aircraft_id == Some(plane.id) && r.plan.is_some())
            {
                let plan = route.plan.as_ref().unwrap();
                let (flight, ground) = timings(route, &plane.model_id)?;
                if plan.interval_seconds < flight + ground || plan.first_departure > u64::MAX / 2 {
                    return Err("Ungültiger Flugplan im Spielstand.".into());
                }
                crate::disruptions::validate(route, plane)?;
                let expected = movement(route, &plane.model_id, self.clock.elapsed().as_secs())?;
                let airborne = &route.plan.as_ref().unwrap().airborne;
                if (expected.status == AircraftStatus::InFlight) != airborne.is_some() {
                    return Err("Abflugergebnis fehlt oder Flug ist nicht unterwegs.".into());
                }
                if let Some(f) = airborne {
                    let t = &f.totals;
                    let mut checked = crate::economy::Totals::default();
                    if f.route_id != route.id
                        || f.landing != expected.landing
                        || f.origin != expected.origin
                        || f.destination != expected.destination
                        || !(0..=100_000).contains(&f.ticket_eur)
                        || t.flights != 1
                        || t.seats != i64::from(crate::cabins::offer(route, plane)?.seats)
                        || t.business_seats != i64::from(plane.business_seats)
                        || t.passengers < 0
                        || t.passengers > t.seats
                        || t.cargo_kg
                            > crate::cargo::capacity(
                                aircraft::model(&plane.model_id)?,
                                t.passengers,
                            )
                        || !t.valid_cabins(f.ticket_eur)
                        || [t.fuel, t.airport, t.crew, t.overhead]
                            .iter()
                            .any(|v| *v < 0)
                        || checked.add(t, 1).is_none()
                    {
                        return Err("Ungültiges Abflugergebnis im Spielstand.".into());
                    }
                }
                if plane.status != expected.status
                    || plane.location_icao != expected.location
                    || route.plan.as_ref().unwrap().completed_flights != expected.completed
                {
                    return Err("Widersprüchlicher Flugzustand im Spielstand.".into());
                }
            } else if !matches!(
                plane.status,
                AircraftStatus::Parked | AircraftStatus::Maintenance
            ) {
                return Err("Flugzustand ohne Flugplan.".into());
            }
        }
        if self
            .routes
            .iter()
            .any(|r| r.plan.is_some() && r.aircraft_id.is_none())
        {
            return Err("Flugplan ohne Flugzeug.".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::default();
        g.found("Flight Air", "DE", "EDDF").unwrap();
        g.competitors.clear();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.schedule(1, 1, 120).unwrap();
        g
    }

    #[test]
    fn departure_snapshot_survives_supply_changes_and_restart_until_settlement() {
        let mut g = game();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g.create_route("EDDF", "EDDM", 2).unwrap();
        g.schedule(2, 30, 120).unwrap(); // still parked when flight 1 starts
        g.clock.set_running(true);
        assert!(g.advance(60.0));
        let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
        assert_eq!(frozen.origin, "EDDF");
        assert_eq!(frozen.destination, "EDDM");
        assert_eq!(g.finance.totals.flights, 0);
        g.stop_schedule(2).unwrap();
        assert!(
            crate::economy::quote_shared(&g.routes[0], "atr72_600", &g.market_routes())
                .unwrap()
                .passengers
                > frozen.totals.passengers
        );
        assert!(g.advance(10.0));
        assert_eq!(
            g.routes[0].plan.as_ref().unwrap().airborne.as_ref(),
            Some(&frozen)
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snapshot.json");
        crate::save::save(&path, &g).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.routes, g.routes);
        // The first frozen flight differs from all later quotes. Batching must
        // keep that boundary even when fast-forward skips many complete cycles.
        let mut fast = crate::save::load(&path).unwrap();
        let mut slow = crate::save::load(&path).unwrap();
        assert!(fast.advance(86400.0 * 10.0));
        for _ in 0..1440 {
            assert!(slow.advance(600.0));
        }
        assert_eq!(fast.routes, slow.routes);
        assert_eq!(fast.finance, slow.finance);
        assert_eq!(fast.history, slow.history);
        assert_eq!(fast.airline, slow.airline);
        fast.validate_history().unwrap();
        // Changing the future offer after loading must not rewrite the snapshot.
        loaded.schedule(2, 1, 80).unwrap();
        assert!(g.advance((frozen.landing - 70) as f64));
        assert!(loaded.advance((frozen.landing - 70) as f64));
        assert_eq!(g.finance.recent[0], frozen);
        assert_eq!(
            loaded.finance.recent.iter().find(|f| f.route_id == 1),
            Some(&frozen)
        );
        assert!(g.routes[0].plan.as_ref().unwrap().airborne.is_none());
        let totals = g.finance.clone();
        assert!(g.advance(0.0));
        assert_eq!(g.finance, totals);
        assert!(g.advance((7260 - frozen.landing) as f64));
        let back = g.routes[0]
            .plan
            .as_ref()
            .unwrap()
            .airborne
            .as_ref()
            .unwrap();
        assert_eq!((&back.origin[..], &back.destination[..]), ("EDDM", "EDDF"));
        assert!(back.totals.passengers > frozen.totals.passengers);
        g.validate_operations().unwrap();
        g.validate_history().unwrap();
    }

    #[test]
    fn parallel_opposite_services_are_order_independent_with_mixed_steps() {
        let mut large = game();
        large.buy_aircraft("atr72_600").unwrap();
        large.staff_up();
        large.fleet[1].location_icao = "EDDM".into();
        large.create_route("EDDM", "EDDF", 2).unwrap();
        large.schedule(2, 1, 80).unwrap();
        large.clock.set_running(true);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("parallel.json");
        crate::save::save(&path, &large).unwrap();
        let mut small = crate::save::load(&path).unwrap();
        small.routes.reverse();
        assert!(large.advance(60.0));
        let a = large.routes[0]
            .plan
            .as_ref()
            .unwrap()
            .airborne
            .as_ref()
            .unwrap();
        let b = large.routes[1]
            .plan
            .as_ref()
            .unwrap()
            .airborne
            .as_ref()
            .unwrap();
        assert_eq!(a.origin, b.destination);
        assert_eq!(a.destination, b.origin);
        assert_eq!(a.totals.passengers, b.totals.passengers);
        assert!(large.advance(86400.0 * 30.0 - 60.0));
        for _ in 0..8640 {
            assert!(small.advance(300.0));
        }
        assert_eq!(large.finance, small.finance);
        // History is canonical by first landing and route, across fuel blocks.
        assert_eq!(large.history, small.history);
        assert_eq!(large.airline, small.airline);
        large.validate_operations().unwrap();
        large.validate_history().unwrap();
    }

    #[test]
    fn v7_airborne_migration_freezes_once_and_v8_rejects_corrupt_snapshots() {
        let mut g = game();
        g.clock.set_running(true);
        assert!(g.advance(100.0));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        crate::save::save(&path, &g).unwrap();
        let valid: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let mut old = valid.clone();
        old["version"] = 7.into();
        old["state"]["routes"]["routes"][0]["plan"]
            .as_object_mut()
            .unwrap()
            .remove("airborne");
        std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.routes, g.routes);
        assert_eq!(loaded.airline, g.airline);
        assert_eq!(loaded.finance, g.finance);
        assert!(loaded.advance(0.0));
        assert_eq!(loaded.finance, g.finance);
        crate::save::save(&path, &loaded).unwrap();
        for (field, value) in [
            ("landing", serde_json::json!(999)),
            ("origin", serde_json::json!("EDDM")),
            ("route_id", serde_json::json!(2)),
            ("ticket_eur", serde_json::json!(999)),
            (
                "totals",
                serde_json::json!({"flights":1,"passengers":100,"seats":72,"revenue":10000,"fuel":0,"airport":0,"crew":0}),
            ),
        ] {
            let mut bad = valid.clone();
            bad["state"]["routes"]["routes"][0]["plan"]["airborne"][field] = value;
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(crate::save::load(&path).is_err(), "accepted {field}");
        }
        let mut bad = valid;
        bad["state"]["routes"]["routes"][0]["plan"]
            .as_object_mut()
            .unwrap()
            .remove("airborne");
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(crate::save::load(&path).is_err());
    }

    #[test]
    fn simultaneous_services_and_minimum_interval_keep_exact_boundaries() {
        let mut g = game();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g.create_route("EDDF", "EDDM", 2).unwrap();
        g.stop_schedule(1).unwrap();
        let (flight, ground) = timings(&g.routes[0], "atr72_600").unwrap();
        let interval = (flight + ground).div_ceil(60) as u32;
        g.clock.set_running(true);
        g.advance(0.125);
        for id in [1, 2] {
            g.schedule(id, 1, interval).unwrap();
        }
        assert_eq!(g.routes[0].plan.as_ref().unwrap().first_departure, 61);
        g.advance(60.875);
        assert!(g.fleet.iter().all(|p| p.status == AircraftStatus::InFlight));
        g.advance(flight as f64);
        assert!(g.fleet.iter().all(|p| p.status == AircraftStatus::Ground));
        assert_eq!(g.finance.totals.flights, 2);
        assert_eq!(g.finance.recent.len(), 2);
        assert_eq!(g.finance.recent[0].route_id, 1);
        assert_eq!(g.finance.recent[1].route_id, 2);
        g.advance((u64::from(interval) * 60 - flight) as f64);
        assert!(g.fleet.iter().all(|p| p.status == AircraftStatus::InFlight));
        assert!(
            g.routes
                .iter()
                .all(|r| r.plan.as_ref().unwrap().completed_flights == 1)
        );
        g.validate_operations().unwrap();
    }

    #[test]
    fn exact_departure_landing_ground_and_return_boundaries() {
        let mut g = game();
        let (flight, ground) = timings(&g.routes[0], "atr72_600").unwrap();
        assert!((2100..2200).contains(&flight));
        assert_eq!(ground, 1800);
        assert!(g.advance(100000.0)); // paused
        assert_eq!(g.clock.elapsed().as_secs(), 0);
        g.clock.set_running(true);
        g.advance(59.999);
        assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
        g.advance(0.001);
        assert_eq!(g.fleet[0].status, AircraftStatus::InFlight);
        assert!(g.stop_schedule(1).is_err());
        assert!(g.delete_route(1).is_err());
        assert!(g.assign_route(1, None).is_err());
        g.advance(flight as f64);
        assert_eq!(g.fleet[0].status, AircraftStatus::Ground);
        assert_eq!(g.fleet[0].location_icao, "EDDM");
        assert_eq!(g.routes[0].plan.as_ref().unwrap().completed_flights, 1);
        assert!(g.stop_schedule(1).is_err());
        g.advance(ground as f64);
        assert_eq!(g.fleet[0].status, AircraftStatus::Parked);
        g.advance((7200 - flight - ground) as f64);
        assert_eq!(g.fleet[0].status, AircraftStatus::InFlight);
        g.advance(flight as f64);
        assert_eq!(g.fleet[0].location_icao, "EDDF");
        g.advance(ground as f64);
        g.stop_schedule(1).unwrap();
        g.assign_route(1, None).unwrap();
        g.validate_operations().unwrap();
    }

    #[test]
    fn large_steps_equal_small_steps_at_all_speeds_and_never_double_count() {
        for speed in crate::clock::Clock::SPEEDS {
            let mut large = game();
            let mut small = game();
            for g in [&mut large, &mut small] {
                g.clock.set_speed(speed);
                g.clock.set_running(true);
            }
            large.advance(100000.0);
            for _ in 0..10000 {
                small.advance(10.0);
            }
            assert_eq!(large.clock.elapsed(), small.clock.elapsed());
            assert_eq!(large.fleet, small.fleet);
            assert_eq!(large.routes, small.routes);
            assert_eq!(large.finance, small.finance);
            assert_eq!(large.airline, small.airline);
            let before = large.routes.clone();
            large.advance(0.0);
            assert_eq!(large.routes, before);
            large.clock.set_running(false);
            large.advance(999999.0);
            assert_eq!(large.routes, before);
            large.clock.set_running(true);
            for value in [-1.0, f64::NAN, f64::INFINITY, f64::MAX] {
                assert!(!large.advance(value));
                assert_eq!(large.routes, before);
            }
        }
        let mut g = game();
        g.clock.set_running(true);
        // ~228 years of 2-hour shuttles, interrupted only by mandatory checks.
        assert!(g.advance(60.0 + 7200.0 * 1_000_000.0));
        let landings = g.finance.totals.flights as u64;
        assert!(landings > 900_000, "{landings}");
        assert_eq!(g.fleet[0].wear.cycles, landings);
        if let Some(plan) = &g.routes[0].plan {
            assert_eq!(plan.prior_flights + plan.completed_flights, landings);
        }
        assert!(g.fleet[0].wear.checks > 1000);
        assert_eq!(g.finance.recent.len(), crate::economy::HISTORY_LIMIT);
        g.validate_operations().unwrap();
        g.validate_maintenance().unwrap();
        g.validate_history().unwrap();
    }

    #[test]
    fn schedule_validation_and_save_restore_in_every_phase() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("flight.json");
        let mut g = game();
        assert!(g.schedule(1, 1, 120).is_err());
        g.stop_schedule(1).unwrap();
        for (delay, interval) in [(0, 120), (1, 1), (1, 60)] {
            assert!(g.schedule(1, delay, interval).is_err());
            assert!(g.routes[0].plan.is_none());
        }
        g.schedule(1, 1, 120).unwrap();
        let (flight, ground) = timings(&g.routes[0], "atr72_600").unwrap();
        for at in [0, 60, 100, 60 + flight, 60 + flight + ground, 7260, 100000] {
            let mut original = game();
            original.clock.set_running(true);
            original.advance(at as f64 + 0.125);
            for running in [false, true] {
                original.clock.set_running(running);
                crate::save::save(&path, &original).unwrap();
                let mut restored = crate::save::load(&path).unwrap();
                assert_eq!(original.routes, restored.routes);
                assert_eq!(original.fleet, restored.fleet);
                assert_eq!(original.clock.elapsed(), restored.clock.elapsed());
                original.advance(1000.0);
                restored.advance(1000.0);
                assert_eq!(original.routes, restored.routes);
                assert_eq!(original.fleet, restored.fleet);
            }
        }
        let mut wrong = game();
        wrong.stop_schedule(1).unwrap();
        wrong.fleet[0].location_icao = "LTFM".into();
        assert!(wrong.schedule(1, 1, 120).is_err());
    }

    #[test]
    fn corrupt_operation_saves_rejected_and_v4_migrates_without_flights() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("flight.json");
        let mut g = game();
        g.clock.set_running(true);
        g.advance(100.0);
        crate::save::save(&path, &g).unwrap();
        let valid: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for (field, value) in [
            ("interval_seconds", 1),
            ("completed_flights", 1),
            ("first_departure", 1000),
        ] {
            let mut bad = valid.clone();
            bad["state"]["routes"]["routes"][0]["plan"][field] = value.into();
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(crate::save::load(&path).is_err());
        }
        let mut old = valid;
        old["version"] = 4.into();
        old["state"]["routes"]["routes"][0]
            .as_object_mut()
            .unwrap()
            .remove("plan");
        old["state"]["fleet"]["aircraft"][0]["status"] = "parked".into();
        std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let restored = crate::save::load(&path).unwrap();
        assert_eq!(restored.routes.len(), 1);
        assert!(restored.routes[0].plan.is_none());
        assert_eq!(restored.airline, g.airline);
    }
}
