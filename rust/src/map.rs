//! Read-only geographic presentation derived from the authoritative flight clock.
use crate::{aircraft::AircraftStatus, airports, game::Game, operations};

/// Public scheduled offers only. No operational results or financial state.
/// Frequency is departures per day in each direction (services alternate ends).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct CompetitorRoute {
    pub airline: String,
    pub origin_icao: String,
    pub destination_icao: String,
    pub departures_per_day_per_direction: f64,
    pub ticket_eur: i64,
    pub business_ticket_eur: i64,
    pub model: String,
}

pub fn competitor_routes(game: &Game) -> Vec<CompetitorRoute> {
    let mut rows: Vec<CompetitorRoute> = Vec::new();
    for company in &game.competitors {
        let g = &company.game;
        let Some(airline) = &g.airline else { continue };
        for route in &g.routes {
            let Some(plan) = &route.plan else { continue };
            let Some(plane) = g.fleet.iter().find(|p| Some(p.id) == route.aircraft_id) else {
                continue;
            };
            let Ok(model) = crate::aircraft::model(&plane.model_id) else {
                continue;
            };
            let frequency = 43200.0 / plan.interval_seconds as f64;
            if let Some(row) = rows.iter_mut().find(|r| {
                r.airline == airline.name
                    && r.origin_icao == route.origin_icao
                    && r.destination_icao == route.destination_icao
                    && r.ticket_eur == route.ticket_eur
                    && r.business_ticket_eur == route.business_ticket_eur
                    && r.model == model.name
            }) {
                row.departures_per_day_per_direction += frequency;
            } else {
                rows.push(CompetitorRoute {
                    airline: airline.name.clone(),
                    origin_icao: route.origin_icao.clone(),
                    destination_icao: route.destination_icao.clone(),
                    departures_per_day_per_direction: frequency,
                    ticket_eur: route.ticket_eur,
                    business_ticket_eur: route.business_ticket_eur,
                    model: model.name.clone(),
                });
            }
        }
    }
    rows
}

/// Great-circle interpolation, including a deterministic plane for antipodal points.
pub fn position(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let unit = |(lon, lat): (f64, f64)| {
        let (lon, lat) = (lon.to_radians(), lat.to_radians());
        [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()]
    };
    let u = unit(a);
    let v = unit(b);
    let dot = u
        .iter()
        .zip(v)
        .map(|(x, y)| x * y)
        .sum::<f64>()
        .clamp(-1.0, 1.0);
    let angle = dot.acos();
    if angle < 1e-10 {
        return a;
    }
    let mut tangent = [v[0] - dot * u[0], v[1] - dot * u[1], v[2] - dot * u[2]];
    let norm = tangent.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm < 1e-10 {
        tangent = if u[2].abs() < 0.9 {
            [-u[1], u[0], 0.0]
        } else {
            [0.0, -u[2], u[1]]
        };
    }
    let norm = tangent.iter().map(|x| x * x).sum::<f64>().sqrt();
    let p: [f64; 3] =
        std::array::from_fn(|i| u[i] * (angle * t).cos() + tangent[i] / norm * (angle * t).sin());
    (
        p[1].atan2(p[0]).to_degrees(),
        p[2].atan2(p[0].hypot(p[1])).to_degrees(),
    )
}

pub fn airport(icao: &str) -> Option<(f64, f64)> {
    airports::catalog()
        .ok()?
        .airports
        .iter()
        .find(|a| a.icao == icao)
        .map(|a| (a.longitude, a.latitude))
}

#[derive(Debug, PartialEq)]
pub struct Flight {
    pub aircraft_id: u32,
    pub route_id: u32,
    pub origin: String,
    pub destination: String,
    pub departure: u64,
    pub landing: u64,
    pub progress: f64,
    pub position: (f64, f64),
    pub ahead: (f64, f64),
}

pub fn flights(game: &Game) -> Vec<Flight> {
    let now = game.clock.elapsed();
    game.routes
        .iter()
        .filter_map(|r| {
            let plane = game.fleet.iter().find(|p| Some(p.id) == r.aircraft_id)?;
            let state = operations::movement(r, &plane.model_id, now.as_secs()).ok()?;
            if state.status != AircraftStatus::InFlight {
                return None;
            }
            let duration = operations::timings(r, &plane.model_id).ok()?.0;
            let departure = state.landing - duration;
            let progress =
                ((now.as_secs_f64() - departure as f64) / duration as f64).clamp(0.0, 1.0);
            let a = airport(&state.origin)?;
            let b = airport(&state.destination)?;
            Some(Flight {
                aircraft_id: plane.id,
                route_id: r.public_id(),
                origin: state.origin,
                destination: state.destination,
                departure,
                landing: state.landing,
                progress,
                position: position(a, b, progress),
                ahead: position(a, b, (progress + 0.0001).min(1.0)),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_boundaries_speeds_pause_return_and_restore() {
        for speed in [1, 4, 16] {
            let mut g = Game::default();
            g.found("Map Air", "DE", "EDDF").unwrap();
            g.buy_aircraft("atr72_600").unwrap();
            g.staff_up();
            g.create_route("EDDF", "EDDM", 1).unwrap();
            g.schedule(1, 1, 120).unwrap();
            assert!(flights(&g).is_empty());
            g.clock.set_speed(speed);
            g.clock.set_running(true);
            assert!(g.advance(60.0 / f64::from(speed)));
            assert_eq!(flights(&g)[0].progress, 0.0);
            let duration = operations::timings(&g.routes[0], "atr72_600").unwrap().0;
            assert!(g.advance(duration as f64 * 0.5 / f64::from(speed)));
            assert!((flights(&g)[0].progress - 0.5).abs() < 1e-8);
            g.clock.set_running(false);
            let before = flights(&g);
            assert!(g.advance(999.0));
            assert_eq!(flights(&g), before);
            let path = std::env::temp_dir()
                .join(format!("airline-map-{}-{speed}.json", std::process::id()));
            crate::save::save(&path, &g).unwrap();
            let restored = crate::save::load(&path).unwrap();
            std::fs::remove_file(path).unwrap();
            assert_eq!(flights(&restored), before);
            g.clock.set_running(true);
            assert!(g.advance(duration as f64 * 0.5 / f64::from(speed)));
            assert!(flights(&g).is_empty());
            assert!(g.advance((7260.0 - g.clock.elapsed().as_secs_f64()) / f64::from(speed)));
            assert_eq!(flights(&g)[0].origin, "EDDM");
            assert_eq!(flights(&g)[0].position, airport("EDDM").unwrap());
        }
    }

    #[test]
    fn great_circle_and_dateline() {
        assert_eq!(position((10.0, 20.0), (30.0, 40.0), 0.0), (10.0, 20.0));
        assert_eq!(position((10.0, 20.0), (30.0, 40.0), 1.0), (30.0, 40.0));
        let mid = position((170.0, 0.0), (-170.0, 0.0), 0.5);
        assert!((mid.0.abs() - 180.0).abs() < 1e-8 && mid.1.abs() < 1e-8);
        assert!(position((0.0, 0.0), (180.0, 0.0), 0.5).0.is_finite());
        let mid = position((-70.0, 40.0), (20.0, 50.0), 0.5);
        assert!(mid.1 > 50.0);
    }
}

#[cfg(test)]
mod competitor_tests {
    use super::*;

    fn game() -> Game {
        let mut g = Game::default();
        g.found("Map public", "DE", "EDDF").unwrap();
        g
    }

    #[test]
    fn competitor_snapshot_matches_schedules_opens_drops_and_public_allowlist() {
        let mut g = game();
        let before = serde_json::to_value(&g).unwrap();
        let rows = competitor_routes(&g);
        assert_eq!(rows.len(), 2);
        for (row, company) in rows.iter().zip(&g.competitors) {
            let r = &company.game.routes[0];
            assert_eq!(row.airline, company.game.airline.as_ref().unwrap().name);
            assert_eq!(row.origin_icao, r.origin_icao);
            assert_eq!(row.destination_icao, r.destination_icao);
            assert_eq!(row.ticket_eur, r.ticket_eur);
            assert_eq!(row.business_ticket_eur, r.business_ticket_eur);
            assert_eq!(
                row.departures_per_day_per_direction,
                43200.0 / r.plan.as_ref().unwrap().interval_seconds as f64
            );
            assert_eq!(
                row.model,
                crate::aircraft::model(&company.game.fleet[0].model_id)
                    .unwrap()
                    .name
            );
            let value = serde_json::to_value(row).unwrap();
            let mut keys: Vec<_> = value
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                [
                    "airline",
                    "business_ticket_eur",
                    "departures_per_day_per_direction",
                    "destination_icao",
                    "model",
                    "origin_icao",
                    "ticket_eur"
                ]
            );
        }
        assert_eq!(serde_json::to_value(&g).unwrap(), before);
        let mut multiple = g.clone();
        let ai = &mut multiple.competitors[0].game;
        ai.buy_aircraft("atr72_600").unwrap();
        ai.equip_staff_for("atr72_600").unwrap();
        ai.add_route_aircraft(1, 2).unwrap();
        let doubled = competitor_routes(&multiple);
        assert_eq!(doubled.len(), rows.len());
        assert_eq!(
            doubled[0].departures_per_day_per_direction,
            rows[0].departures_per_day_per_direction * 2.0
        );
        let ai = &mut g.competitors[0].game;
        ai.stop_schedule(1).unwrap();
        assert_eq!(competitor_routes(&g).len(), 1);
        g.competitors[0].game.schedule(1, 1, 360).unwrap();
        assert_eq!(competitor_routes(&g), rows);
        let ai = &mut g.competitors[0].game;
        ai.buy_aircraft("atr72_600").unwrap();
        ai.equip_staff_for("atr72_600").unwrap();
        ai.create_route("EDDF", "EDLP", 2).unwrap();
        ai.schedule(2, 1, 360).unwrap();
        assert_eq!(competitor_routes(&g).len(), 3);
        ai_drop(&mut g);
        assert_eq!(competitor_routes(&g), rows);
    }

    fn ai_drop(g: &mut Game) {
        g.competitors[0].game.stop_schedule(2).unwrap();
        g.competitors[0].game.delete_route(2).unwrap();
    }

    #[test]
    fn competitor_snapshot_steps_speeds_restart_and_state_are_unchanged() {
        let mut initial = game();
        initial.clock.set_running(true);
        let mut reference = initial.clone();
        assert!(reference.advance(15.0 * 86400.0));
        for speed in [1, 4, 16] {
            let mut g = initial.clone();
            g.clock.set_speed(speed);
            for _ in 0..360 {
                let before = serde_json::to_value(&g).unwrap();
                let snapshot = competitor_routes(&g);
                assert_eq!(snapshot, competitor_routes(&g));
                assert_eq!(serde_json::to_value(&g).unwrap(), before);
                assert!(g.advance(3600.0 / f64::from(speed)));
            }
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("competitor-map.json");
            crate::save::save(&path, &g).unwrap();
            let mut loaded = crate::save::load(&path).unwrap();
            assert_eq!(competitor_routes(&g), competitor_routes(&loaded));
            assert!(g.advance(86400.0 / f64::from(speed)));
            assert!(loaded.advance(86400.0 / f64::from(speed)));
            assert_eq!(
                serde_json::to_value(&g).unwrap(),
                serde_json::to_value(&loaded).unwrap()
            );
            g.clock.set_speed(1);
            let mut control = reference.clone();
            assert!(control.advance(86400.0));
            assert_eq!(competitor_routes(&g), competitor_routes(&control));
            assert_eq!(
                serde_json::to_value(&g).unwrap(),
                serde_json::to_value(&control).unwrap()
            );
        }
    }
}
