//! Repeating shuttle service. Integer event boundaries, independent of frame size.
use crate::{
    aircraft::{self, AircraftStatus},
    game::Game,
    routes::Route,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlightPlan {
    pub first_departure: u64,
    pub interval_seconds: u64,
    pub completed_flights: u64,
}

pub struct Movement {
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
    let (origin, destination) = if index % 2 == 0 {
        (&route.origin_icao, &route.destination_icao)
    } else {
        (&route.destination_icao, &route.origin_icao)
    };
    let landed = started && now >= landing;
    Ok(Movement {
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
        if self.routes.iter().any(|r| r.id == id && r.plan.is_some()) {
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
        if plane.location_icao != route.origin_icao || plane.status != AircraftStatus::Parked {
            return Err(
                "Das Flugzeug muss am Startflughafen geparkt sein; keine automatische Überführung."
                    .into(),
            );
        }
        let (flight, ground) = timings(route, &plane.model_id)?;
        let interval = u64::from(interval_minutes) * 60;
        if delay_minutes == 0 || interval < flight + ground {
            return Err(format!(
                "Erster Abflug frühestens in 1 Minute; Abstand mindestens {} Minuten (Flug + Bodenzeit).",
                (flight + ground).div_ceil(60)
            ));
        }
        let first = self
            .clock
            .elapsed()
            .as_secs()
            .checked_add(
                u64::from(delay_minutes) * 60 + u64::from(self.clock.elapsed().subsec_nanos() != 0),
            )
            .filter(|t| *t <= u64::MAX / 2)
            .ok_or("Flugplan-Zeitüberlauf.")?;
        self.routes.iter_mut().find(|r| r.id == id).unwrap().plan = Some(FlightPlan {
            first_departure: first,
            interval_seconds: interval,
            completed_flights: 0,
        });
        Ok(())
    }

    pub fn stop_schedule(&mut self, id: u32) -> Result<(), String> {
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
        self.routes.iter_mut().find(|r| r.id == id).unwrap().plan = None;
        Ok(())
    }

    pub fn advance(&mut self, seconds: f64) -> bool {
        // Stage the clock and all derived states before committing, including overflow errors.
        let mut clock = crate::clock::Clock::from_state(
            self.clock.elapsed(),
            self.clock.is_running(),
            self.clock.speed(),
        )
        .unwrap();
        if !clock.advance(seconds) {
            return false;
        }
        let mut updates = Vec::new();
        let mut finance = self.finance.clone();
        let mut delta = 0i64;
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
            let Ok(state) = movement(route, &self.fleet[p].model_id, clock.elapsed().as_secs())
            else {
                return false;
            };
            let plan = route.plan.as_ref().unwrap();
            let count = state.completed - plan.completed_flights;
            if count > 0 {
                let Ok(totals) = crate::economy::quote(route, &self.fleet[p].model_id) else {
                    return false;
                };
                if finance.totals.add(&totals, count).is_none() {
                    return false;
                }
                let Some(change) = i64::try_from(count)
                    .ok()
                    .and_then(|n| totals.profit().checked_mul(n))
                    .and_then(|n| delta.checked_add(n))
                else {
                    return false;
                };
                delta = change;
                let flight = timings(route, &self.fleet[p].model_id).unwrap().0;
                for index in plan.completed_flights.max(
                    state
                        .completed
                        .saturating_sub(crate::economy::HISTORY_LIMIT as u64),
                )..state.completed
                {
                    let (a, b) = if index % 2 == 0 {
                        (&route.origin_icao, &route.destination_icao)
                    } else {
                        (&route.destination_icao, &route.origin_icao)
                    };
                    finance.recent.push(crate::economy::FlightResult {
                        route_id: route.id,
                        landing: plan.first_departure + index * plan.interval_seconds + flight,
                        origin: a.clone(),
                        destination: b.clone(),
                        ticket_eur: route.ticket_eur,
                        totals: totals.clone(),
                    });
                }
            }
            updates.push((i, p, state));
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
        if let Some(balance) = balance {
            self.airline.as_mut().unwrap().capital_eur = balance;
        }
        self.clock = clock;
        for (r, p, state) in updates {
            self.fleet[p].status = state.status;
            self.fleet[p].location_icao = state.location;
            self.routes[r].plan.as_mut().unwrap().completed_flights = state.completed;
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
                let expected = movement(route, &plane.model_id, self.clock.elapsed().as_secs())?;
                if plane.status != expected.status
                    || plane.location_icao != expected.location
                    || route.plan.as_ref().unwrap().completed_flights != expected.completed
                {
                    return Err("Widersprüchlicher Flugzustand im Spielstand.".into());
                }
            } else if plane.status != AircraftStatus::Parked {
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
        g.buy_aircraft("atr72_600").unwrap();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.schedule(1, 1, 120).unwrap();
        g
    }

    #[test]
    fn simultaneous_services_and_minimum_interval_keep_exact_boundaries() {
        let mut g = game();
        g.buy_aircraft("atr72_600").unwrap();
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
        g.advance(60.0 + 7200.0 * 1_000_000.0);
        assert_eq!(
            g.routes[0].plan.as_ref().unwrap().completed_flights,
            1_000_000
        );
        assert_eq!(g.fleet[0].status, AircraftStatus::InFlight);
        assert_eq!(g.finance.totals.flights, 1_000_000);
        assert_eq!(g.finance.recent.len(), crate::economy::HISTORY_LIMIT);
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
