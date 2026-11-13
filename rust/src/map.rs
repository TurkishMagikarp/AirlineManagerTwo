//! Read-only geographic presentation derived from the authoritative flight clock.
use crate::{aircraft::AircraftStatus, airports, game::Game, operations};

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
