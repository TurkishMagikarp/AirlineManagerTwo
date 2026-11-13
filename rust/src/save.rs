//! Versioned disk format, deliberately separate from the live clock and Godot.
use crate::aircraft::Aircraft;
use crate::clock::Clock;
use crate::game::{Airline, Game};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

const VERSION: u32 = 9;
// Bounded input, but allow multi-year archives with frequent legitimate changes.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct SaveGame {
    version: u32,
    state: GameState,
}

// Future simulation data belongs here, not in the UI or a serialized Godot object.
#[derive(Serialize, Deserialize)]
struct GameState {
    competitors: Option<Vec<crate::competition::Competitor>>,
    clock: SavedClock,
    airline: Option<Airline>,
    fleet: Option<SavedFleet>,
    routes: Option<SavedRoutes>,
    finance: Option<crate::economy::Finance>,
    history: Option<crate::history::History>,
}

#[derive(Serialize, Deserialize)]
struct SavedRoutes {
    routes: Vec<crate::routes::Route>,
    next_route_id: u32,
}

#[derive(Serialize, Deserialize)]
struct SavedFleet {
    aircraft: Vec<Aircraft>,
    next_aircraft_id: u32,
}

#[derive(Serialize, Deserialize)]
struct SavedClock {
    elapsed_seconds: u64,
    elapsed_nanoseconds: u32,
    running: bool,
    speed: u32,
}

pub fn save(path: &Path, game: &Game) -> Result<(), String> {
    if let Some(airline) = &game.airline {
        airline.validate()?;
    }
    game.validate_fleet()?;
    game.validate_routes()?;
    game.validate_operations()?;
    game.validate_finance()?;
    game.validate_history()?;
    game.validate_competitors()?;
    let clock = &game.clock;
    let data = SaveGame {
        version: VERSION,
        state: GameState {
            competitors: Some(game.competitors.clone()),
            history: game.history.clone(),
            finance: Some(game.finance.clone()),
            airline: game.airline.clone(),
            routes: Some(SavedRoutes {
                routes: game.routes.clone(),
                next_route_id: game.next_route_id,
            }),
            fleet: Some(SavedFleet {
                aircraft: game.fleet.clone(),
                next_aircraft_id: game.next_aircraft_id,
            }),
            clock: SavedClock {
                elapsed_seconds: clock.elapsed().as_secs(),
                elapsed_nanoseconds: clock.elapsed().subsec_nanos(),
                running: clock.is_running(),
                speed: clock.speed(),
            },
        },
    };
    let bytes = serde_json::to_vec(&data)
        .map_err(|error| format!("Spielstand konnte nicht erzeugt werden: {error}"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Spielstand ist zu groß.".into());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("Speicherordner nicht verfügbar: {error}"))?;
    // Same directory/filesystem: replace only after the entire new file is on disk.
    // tempfile also replaces existing files on Windows without deleting them first.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("Temporäre Speicherdatei nicht verfügbar: {error}"))?;
    temporary
        .write_all(&bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| format!("Spielstand konnte nicht geschrieben werden: {error}"))?;
    temporary
        .persist(path)
        .map_err(|error| format!("Spielstand konnte nicht ersetzt werden: {}", error.error))?;
    Ok(())
}

pub fn load(path: &Path) -> Result<Game, String> {
    let file = File::open(path).map_err(|error| format!("Spielstand nicht lesbar: {error}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Spielstand nicht lesbar: {error}"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Spielstand ist zu groß.".into());
    }
    let data: SaveGame = serde_json::from_slice(&bytes)
        .map_err(|_| "Spielstand ist beschädigt oder unvollständig.".to_string())?;
    if !(1..=VERSION).contains(&data.version) {
        return Err(format!(
            "Nicht unterstützte Spielstand-Version: {}.",
            data.version
        ));
    }
    // Version 1 had only a clock; never invent an airline during migration.
    if data.version == 1 && data.state.airline.is_some() {
        return Err("Unerwartete Airline-Daten im alten Spielstand.".into());
    }
    if let Some(airline) = &data.state.airline {
        airline.validate()?;
    }
    let fleet = if data.version >= 3 {
        data.state
            .fleet
            .ok_or_else(|| "Flottendaten fehlen im Spielstand.".to_string())?
    } else {
        if data.state.fleet.is_some() {
            return Err("Unerwartete Flottendaten im alten Spielstand.".into());
        }
        SavedFleet {
            aircraft: Vec::new(),
            next_aircraft_id: 1,
        }
    };
    let routes = if data.version >= 4 {
        data.state
            .routes
            .ok_or_else(|| "Routendaten fehlen im Spielstand.".to_string())?
    } else {
        if data.state.routes.is_some() {
            return Err("Unerwartete Routendaten im alten Spielstand.".into());
        }
        SavedRoutes {
            routes: Vec::new(),
            next_route_id: 1,
        }
    };
    let clock = data.state.clock;
    if clock.elapsed_nanoseconds >= 1_000_000_000 {
        return Err("Ungültige Spielzeit im Spielstand.".into());
    }
    let clock = Clock::from_state(
        Duration::new(clock.elapsed_seconds, clock.elapsed_nanoseconds),
        clock.running,
        clock.speed,
    )
    .ok_or_else(|| "Ungültige Geschwindigkeit im Spielstand.".to_string())?;
    let finance = if data.version >= 6 {
        data.state
            .finance
            .ok_or("Finanzdaten fehlen im Spielstand.")?
    } else {
        crate::economy::Finance::default()
    };
    let mut game = Game {
        competitors: if data.version >= 9 {
            data.state
                .competitors
                .ok_or("KI-Airlines fehlen im Spielstand.")?
        } else {
            Vec::new()
        },
        history: if data.version >= 7 {
            if data.state.airline.is_some() && data.state.history.is_none() {
                return Err("Finanzhistorie fehlt.".into());
            }
            data.state.history
        } else {
            data.state.airline.as_ref().map(|a| {
                let mut h = crate::history::History::new(clock.elapsed().as_secs(), a.capital_eur);
                h.prior_totals = finance.totals.clone();
                h
            })
        },
        finance,
        clock,
        airline: data.state.airline,
        fleet: fleet.aircraft,
        next_aircraft_id: fleet.next_aircraft_id,
        routes: routes.routes,
        next_route_id: routes.next_route_id,
    };
    game.validate_fleet()?;
    game.validate_routes()?;
    if data.version < 5 && game.routes.iter().any(|r| r.plan.is_some()) {
        return Err("Unerwarteter Flugplan im alten Spielstand.".into());
    }
    if data.version < 8 {
        // Old saves never stored departure quotes. Freeze the current estimate
        // once on migration; do not replay or charge already completed flights.
        for i in 0..game.routes.len() {
            let route = &game.routes[i];
            if route.plan.is_some() {
                let plane = game
                    .fleet
                    .iter()
                    .find(|p| Some(p.id) == route.aircraft_id)
                    .ok_or("Flugplan ohne Flugzeug.")?;
                let state = crate::operations::movement(
                    route,
                    &plane.model_id,
                    game.clock.elapsed().as_secs(),
                )?;
                let airborne = if state.status == crate::aircraft::AircraftStatus::InFlight {
                    Some(crate::operations::departure_quote(
                        route,
                        &plane.model_id,
                        state.completed,
                        &game.routes,
                    )?)
                } else {
                    None
                };
                game.routes[i].plan.as_mut().unwrap().airborne = airborne;
            }
        }
    }
    game.validate_operations()?;
    game.validate_finance()?;
    game.validate_history()?;
    game.validate_competitors()?;
    if data.version < 9 && game.airline.is_some() {
        game.competitors = crate::competition::initial_competitors_at(game.clock.elapsed())?;
    }
    Ok(game)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_roundtrip_migration_and_rejecting_corrupt_assignments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("routes.json");
        let mut game = Game::default();
        game.found("Route Save", "DE", "EDDF").unwrap();
        game.buy_aircraft("atr72_600").unwrap();
        game.buy_aircraft("atr72_600").unwrap();
        game.create_route("EDDF", "EDDM", 1).unwrap();
        game.create_route("LTFM", "LTFJ", 2).unwrap();
        save(&path, &game).unwrap();
        let mut restored = load(&path).unwrap();
        assert_eq!(restored.routes, game.routes);
        assert_eq!(restored.next_route_id, 3);
        assert!(restored.create_route("EDDF", "EDDM", 1).is_err());
        restored.assign_route(2, None).unwrap();
        restored.assign_route(1, Some(2)).unwrap();
        restored.delete_route(2).unwrap();
        restored.create_route("EDDM", "EDDF", 1).unwrap();
        assert_eq!(restored.routes[1].id, 3);
        restored.assign_route(1, None).unwrap();
        save(&path, &restored).unwrap();
        assert_eq!(load(&path).unwrap().routes, restored.routes);
        save(&path, &game).unwrap();
        let valid: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for (field, value) in [
            ("id", serde_json::json!(0)),
            ("id", serde_json::json!(2)),
            ("origin_icao", serde_json::json!("XXXX")),
            ("destination_icao", serde_json::json!("EDDF")),
            ("destination_icao", serde_json::json!("KJFK")),
            ("aircraft_id", serde_json::json!(99)),
            ("aircraft_id", serde_json::json!(2)),
        ] {
            let mut bad = valid.clone();
            bad["state"]["routes"]["routes"][0][field] = value;
            fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(load(&path).is_err(), "accepted invalid route {field}");
        }
        for value in [0, 1, 2] {
            let mut bad = valid.clone();
            bad["state"]["routes"]["next_route_id"] = serde_json::json!(value);
            fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(load(&path).is_err());
        }
        let mut old = valid.clone();
        old["version"] = serde_json::json!(3);
        old["state"].as_object_mut().unwrap().remove("routes");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let migrated = load(&path).unwrap();
        assert_eq!(migrated.fleet, game.fleet);
        assert_eq!(migrated.airline, game.airline);
        assert!(migrated.routes.is_empty());
        assert_eq!(migrated.next_route_id, 1);
        let mut bad = valid.clone();
        bad["state"].as_object_mut().unwrap().remove("routes");
        fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(load(&path).is_err());
        save(&path, &game).unwrap();
        let previous = fs::read(&path).unwrap();
        game.routes[0].aircraft_id = Some(2);
        assert!(save(&path, &game).is_err());
        assert_eq!(fs::read(&path).unwrap(), previous);
    }

    #[test]
    fn exact_roundtrip_and_overwrite_for_all_clock_states() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Spielstände 日本語").join("savegame.json");
        for speed in Clock::SPEEDS {
            for running in [false, true] {
                let mut original = Game::default();
                original.found("Neustart Air", "TR", "LTFM").unwrap();
                original.competitors.clear(); // This fixture directly replaces the clock.
                // Loading must retain saved capital, never reapply today's config.
                original.airline.as_mut().unwrap().capital_eur = 1_234_567;
                original.history.as_mut().unwrap().opening_balance = 1_234_567;
                original.clock =
                    Clock::from_state(Duration::new(987654, 123456789), running, speed).unwrap();
                save(&path, &original).unwrap();
                let restored = load(&path).unwrap();
                assert_eq!(restored.clock.elapsed(), original.clock.elapsed());
                assert_eq!(restored.clock.is_running(), running);
                assert_eq!(restored.clock.speed(), speed);
                assert_eq!(restored.airline, original.airline);
            }
        }
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn rejects_missing_corrupt_truncated_invalid_and_future_saves() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("save.json");
        assert!(load(&path).is_err());
        for bytes in ["", "not json", "{", "{}", "null", "{\"version\":1}"] {
            fs::write(&path, bytes).unwrap();
            assert!(load(&path).is_err());
        }
        save(&path, &Game::default()).unwrap();
        let valid: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for (field, value) in [
            ("speed", serde_json::json!(0)),
            ("speed", serde_json::json!(-1)),
            ("running", serde_json::json!("true")),
            ("elapsed_seconds", serde_json::json!(-1)),
            ("elapsed_seconds", serde_json::json!(1e30)),
            ("elapsed_nanoseconds", serde_json::json!(1_000_000_000)),
        ] {
            let mut bad = valid.clone();
            bad["state"]["clock"][field] = value;
            fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(load(&path).is_err(), "accepted invalid {field}");
        }
        let mut future = valid;
        future["version"] = serde_json::json!(VERSION + 1);
        fs::write(&path, serde_json::to_vec(&future).unwrap()).unwrap();
        assert!(load(&path).err().unwrap().contains("Version"));
    }

    #[test]
    fn rejects_oversized_saves_and_write_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.json");
        fs::write(&path, vec![b' '; MAX_BYTES as usize + 1]).unwrap();
        assert!(load(&path).is_err());
        // A directory cannot be overwritten as a save file.
        assert!(save(dir.path(), &Game::default()).is_err());
        assert!(dir.path().is_dir());
    }

    #[test]
    fn migrates_clock_only_v1_without_fabricating_a_company() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        fs::write(&path, r#"{"version":1,"state":{"clock":{"elapsed_seconds":42,"elapsed_nanoseconds":123,"running":true,"speed":4}}}"#).unwrap();
        let game = load(&path).unwrap();
        assert!(game.airline.is_none());
        assert_eq!(game.clock.elapsed(), Duration::new(42, 123));
        assert!(game.clock.is_running());
        assert_eq!(game.clock.speed(), 4);
        save(&path, &game).unwrap();
        assert!(load(&path).unwrap().airline.is_none());
    }

    #[test]
    fn rejects_invalid_company_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("company.json");
        let mut game = Game::default();
        game.found("Valid Air", "DE", "EDDF").unwrap();
        save(&path, &game).unwrap();
        let valid: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for (field, value) in [
            ("name", serde_json::json!("   ")),
            ("country_code", serde_json::json!("TR")),
            ("home_airport_icao", serde_json::json!("XXXX")),
        ] {
            let mut bad = valid.clone();
            bad["state"]["airline"][field] = value;
            fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(load(&path).is_err());
        }
    }

    #[test]
    fn fleet_and_balance_roundtrip_and_identifiers_continue_after_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.json");
        let mut game = Game::default();
        game.found("Fleet Save Air", "TR", "LTFJ").unwrap();
        let price = crate::aircraft::model("atr72_600").unwrap().price_eur;
        game.airline.as_mut().unwrap().capital_eur = price * 3;
        game.history.as_mut().unwrap().opening_balance = price * 3;
        game.buy_aircraft("atr72_600").unwrap();
        game.buy_aircraft("atr72_600").unwrap();
        // A historical purchase price need not equal the current catalog price.
        game.fleet[0].purchase_price_eur = 123;
        save(&path, &game).unwrap();
        let mut restored = load(&path).unwrap();
        assert_eq!(restored.fleet, game.fleet);
        assert_eq!(restored.airline, game.airline);
        assert_eq!(restored.next_aircraft_id, 3);
        restored.buy_aircraft("atr72_600").unwrap();
        assert_eq!(restored.fleet[2].identifier(), "AM-000003");
        assert_eq!(restored.airline.as_ref().unwrap().capital_eur, 0);
    }

    #[test]
    fn v2_company_migrates_without_buying_aircraft_or_changing_balance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old_company.json");
        let mut game = Game::default();
        game.found("Old Air", "DE", "EDDF").unwrap();
        save(&path, &game).unwrap();
        let mut old: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        old["version"] = serde_json::json!(2);
        old["state"].as_object_mut().unwrap().remove("fleet");
        old["state"].as_object_mut().unwrap().remove("routes");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(restored.airline, game.airline);
        assert!(restored.fleet.is_empty());
        assert_eq!(restored.next_aircraft_id, 1);
    }

    #[test]
    fn rejects_corrupt_fleet_instead_of_silently_losing_aircraft() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad_fleet.json");
        let mut game = Game::default();
        game.found("Fleet Air", "DE", "EDDF").unwrap();
        let price = crate::aircraft::model("atr72_600").unwrap().price_eur;
        game.airline.as_mut().unwrap().capital_eur = price * 2;
        game.history.as_mut().unwrap().opening_balance = price * 2;
        game.buy_aircraft("atr72_600").unwrap();
        game.buy_aircraft("atr72_600").unwrap();
        save(&path, &game).unwrap();
        let valid: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for (field, value) in [
            ("id", serde_json::json!(0)),
            ("id", serde_json::json!(2)),
            ("model_id", serde_json::json!("unknown")),
            ("location_icao", serde_json::json!("XXXX")),
            ("status", serde_json::json!("flying")),
            ("purchase_price_eur", serde_json::json!(-1)),
        ] {
            let mut bad = valid.clone();
            bad["state"]["fleet"]["aircraft"][0][field] = value;
            fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(load(&path).is_err(), "accepted invalid {field}");
        }
        for field in ["fleet", "airline"] {
            let mut bad = valid.clone();
            bad["state"][field] = serde_json::Value::Null;
            fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(load(&path).is_err());
        }
        for next_id in [0, 1, 2] {
            let mut bad = valid.clone();
            bad["state"]["fleet"]["next_aircraft_id"] = serde_json::json!(next_id);
            fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(load(&path).is_err());
        }
    }

    #[test]
    fn oversized_archive_cannot_overwrite_a_loadable_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet_limit.json");
        let mut game = Game::default();
        game.found("Large Fleet Air", "DE", "EDDF").unwrap();
        save(&path, &game).unwrap();
        let previous = fs::read(&path).unwrap();
        let count = MAX_BYTES as usize / 10;
        game.history.as_mut().unwrap().purchases =
            vec![crate::history::Purchase { time: 0, amount: 1 }; count];
        game.airline.as_mut().unwrap().capital_eur -= count as i64;
        game.validate_history().unwrap();
        assert!(save(&path, &game).is_err());
        assert_eq!(fs::read(&path).unwrap(), previous);
        assert!(load(&path).unwrap().fleet.is_empty());
    }
}
