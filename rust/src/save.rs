//! Versioned disk format, deliberately separate from the live clock and Godot.
use crate::clock::Clock;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

const VERSION: u32 = 1;
const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct SaveGame {
    version: u32,
    state: GameState,
}

// Future simulation data belongs here, not in the UI or a serialized Godot object.
#[derive(Serialize, Deserialize)]
struct GameState {
    clock: SavedClock,
}

#[derive(Serialize, Deserialize)]
struct SavedClock {
    elapsed_seconds: u64,
    elapsed_nanoseconds: u32,
    running: bool,
    speed: u32,
}

pub fn save(path: &Path, clock: &Clock) -> Result<(), String> {
    let data = SaveGame {
        version: VERSION,
        state: GameState {
            clock: SavedClock {
                elapsed_seconds: clock.elapsed().as_secs(),
                elapsed_nanoseconds: clock.elapsed().subsec_nanos(),
                running: clock.is_running(),
                speed: clock.speed(),
            },
        },
    };
    let bytes = serde_json::to_vec_pretty(&data)
        .map_err(|error| format!("Spielstand konnte nicht erzeugt werden: {error}"))?;
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

pub fn load(path: &Path) -> Result<Clock, String> {
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
    if data.version != VERSION {
        return Err(format!(
            "Nicht unterstützte Spielstand-Version: {}.",
            data.version
        ));
    }
    let clock = data.state.clock;
    if clock.elapsed_nanoseconds >= 1_000_000_000 {
        return Err("Ungültige Spielzeit im Spielstand.".into());
    }
    Clock::from_state(
        Duration::new(clock.elapsed_seconds, clock.elapsed_nanoseconds),
        clock.running,
        clock.speed,
    )
    .ok_or_else(|| "Ungültige Geschwindigkeit im Spielstand.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_roundtrip_and_overwrite_for_all_clock_states() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Spielstände 日本語").join("savegame.json");
        for speed in Clock::SPEEDS {
            for running in [false, true] {
                let original =
                    Clock::from_state(Duration::new(987654, 123456789), running, speed).unwrap();
                save(&path, &original).unwrap();
                let restored = load(&path).unwrap();
                assert_eq!(restored.elapsed(), original.elapsed());
                assert_eq!(restored.is_running(), running);
                assert_eq!(restored.speed(), speed);
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
        save(&path, &Clock::default()).unwrap();
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
        assert!(save(dir.path(), &Clock::default()).is_err());
        assert!(dir.path().is_dir());
    }
}
