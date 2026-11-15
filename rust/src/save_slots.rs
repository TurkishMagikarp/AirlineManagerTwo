//! Slot storage is independent of simulation serialization. A single manifest
//! replacement commits both current and previous generation together.
use crate::{game::Game, save};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
    time::UNIX_EPOCH,
};

#[derive(Deserialize)]
pub struct Rules {
    pub slot_count: u32,
    pub name_length: usize,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let rules: Rules =
        serde_json::from_str(include_str!("../../config/saves.json")).expect("save configuration");
    assert!((1..=100).contains(&rules.slot_count) && (1..=128).contains(&rules.name_length));
    rules
});
const HEADER_LIMIT: u64 = 4096;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Header {
    pub format_version: u32,
    pub save_version: u32,
    pub name: String,
    pub game_time: String,
    pub cash: i64,
    pub fleet: usize,
    pub written_at: u64,
    bytes: u64,
    modified_ns: u128,
}
#[derive(Serialize, Deserialize)]
struct Manifest {
    main: String,
    backup: Option<String>,
}
pub struct Slot {
    pub id: u32,
    pub occupied: bool,
    pub header: Option<Header>,
    pub backup: Option<Header>,
    pub error: String,
}
fn error(e: impl std::fmt::Display) -> String {
    format!("Spielstand: {e}")
}
pub fn validate_name(name: &str) -> Result<(), String> {
    let reserved = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if name.trim() != name
        || name.is_empty()
        || name.chars().count() > RULES.name_length
        || name == "."
        || name == ".."
        || name.ends_with('.')
        || name
            .chars()
            .any(|c| c.is_control() || "/\\<>:\"|?*".contains(c))
        || ["CON", "PRN", "AUX", "NUL"].contains(&reserved.as_str())
        || (reserved.len() == 4
            && (reserved.starts_with("COM") || reserved.starts_with("LPT"))
            && matches!(reserved.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(format!(
            "Name: 1–{} Zeichen, keine Pfadzeichen oder reservierten Dateinamen.",
            RULES.name_length
        ));
    }
    Ok(())
}
fn slot_path(root: &Path, id: u32) -> Result<PathBuf, String> {
    if id >= RULES.slot_count {
        return Err("Ungültiger Speicherplatz.".into());
    }
    let path = root.join(format!("slot-{id:02}"));
    reject_link(root)?;
    reject_link(&path)?;
    Ok(path)
}
fn reject_link(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => {
            Err("Verknüpfungen sind keine Speicherplätze.".into())
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(error(e)),
    }
}
fn generation(slot: &Path, id: &str) -> Result<PathBuf, String> {
    if !id.starts_with("gen-") || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
        return Err("Beschädigtes Speicherverzeichnis.".into());
    }
    let path = slot.join(id);
    reject_link(&path)?;
    Ok(path)
}
fn read_small<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    reject_link(path)?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(error)?
        .take(HEADER_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() as u64 > HEADER_LIMIT {
        return Err("Beschädigter Spielstandkopf.".into());
    }
    serde_json::from_slice(&bytes).map_err(error)
}
fn modified_ns(m: &fs::Metadata) -> Result<u128, String> {
    Ok(m.modified()
        .map_err(error)?
        .duration_since(UNIX_EPOCH)
        .map_err(error)?
        .as_nanos())
}
fn read_header_metadata(slot: &Path, id: &str) -> Result<Header, String> {
    let dir = generation(slot, id)?;
    let h: Header = read_small(&dir.join("header.json"))?;
    validate_name(&h.name)?;
    if h.format_version != 1
        || !(1..=save::VERSION).contains(&h.save_version)
        || h.game_time.is_empty()
        || h.game_time.len() > 128
        || h.bytes == 0
        || h.bytes > save::MAX_BYTES
    {
        return Err("Beschädigter Spielstandkopf.".into());
    }
    Ok(h)
}
fn read_header(slot: &Path, id: &str) -> Result<Header, String> {
    let h = read_header_metadata(slot, id)?;
    let path = generation(slot, id)?.join("save.json");
    reject_link(&path)?;
    // Opening without reading also detects permission errors cheaply.
    let m = fs::File::open(path)
        .map_err(error)?
        .metadata()
        .map_err(error)?;
    // Cheap integrity indication without parsing/reading the simulation archive.
    // Full semantic validation still runs transactionally on explicit load.
    if m.len() != h.bytes || modified_ns(&m)? != h.modified_ns {
        return Err("Beschädigt: Speicherdatei wurde verändert.".into());
    }
    Ok(h)
}
#[cfg(test)]
thread_local! { pub(crate) static FAIL_COMMIT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap()).map_err(error)?;
    serde_json::to_writer(&mut file, value).map_err(error)?;
    file.flush()
        .and_then(|()| file.as_file().sync_all())
        .map_err(error)?;
    #[cfg(test)]
    if path.file_name().is_some_and(|n| n == "manifest.json")
        && FAIL_COMMIT.with(|f| f.replace(false))
    {
        return Err("Simulierter Schreibfehler.".into());
    }
    file.persist(path).map_err(|e| error(e.error))?;
    Ok(())
}
fn write_generation(
    slot: &Path,
    name: &str,
    game: &Game,
    legacy: Option<&Path>,
) -> Result<String, String> {
    let dir = tempfile::Builder::new()
        .prefix("gen-")
        .tempdir_in(slot)
        .map_err(error)?;
    let path = dir.path().join("save.json");
    if let Some(old) = legacy {
        reject_link(old)?;
        fs::copy(old, &path).map_err(error)?;
        fs::File::open(&path)
            .map_err(error)?
            .sync_all()
            .map_err(error)?;
    } else {
        save::save(&path, game)?;
    }
    // Never replace a good save with bytes that cannot be loaded again.
    let (_, save_version) = save::load_versioned(&path)?;
    let meta = fs::metadata(&path).map_err(error)?;
    let h = Header {
        format_version: 1,
        save_version,
        name: name.into(),
        game_time: game.clock.display_time(),
        cash: game.cash(),
        fleet: game.fleet.len(),
        written_at: legacy
            .map(fs::metadata)
            .transpose()
            .map_err(error)?
            .as_ref()
            .unwrap_or(&meta)
            .modified()
            .map_err(error)?
            .duration_since(UNIX_EPOCH)
            .map_err(error)?
            .as_secs(),
        bytes: meta.len(),
        modified_ns: modified_ns(&meta)?,
    };
    atomic_json(&dir.path().join("header.json"), &h)?;
    let id = dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let _ = dir.keep();
    Ok(id)
}
/// Import once into a dedicated slot, retaining the legacy file byte-for-byte.
/// The marker is committed with the slot, so deleting it cannot reimport it.
fn import_legacy(root: &Path, legacy: &Path) -> Result<(), String> {
    if root.join("legacy-imported").exists() || root.join("slot-00").exists() || !legacy.exists() {
        return Ok(());
    }
    fs::create_dir_all(root).map_err(error)?;
    let staging = tempfile::tempdir_in(root).map_err(error)?;
    // A damaged legacy save is still visible and never prevents listing.
    let manifest = match save::load(legacy) {
        Ok(g) => Manifest {
            main: write_generation(staging.path(), "Bisheriger Spielstand", &g, Some(legacy))?,
            backup: None,
        },
        Err(_) => Manifest {
            main: "gen-damaged-legacy".into(),
            backup: None,
        },
    };
    atomic_json(&staging.path().join("manifest.json"), &manifest)?;
    fs::rename(staging.path(), slot_path(root, 0)?).map_err(error)?;
    atomic_json(&root.join("legacy-imported"), &true)
}
pub fn list(root: &Path, legacy: &Path) -> Result<Vec<Slot>, String> {
    reject_link(root)?;
    import_legacy(root, legacy)?;
    Ok((0..RULES.slot_count)
        .map(|id| {
            let mut s = Slot {
                id,
                occupied: false,
                header: None,
                backup: None,
                error: String::new(),
            };
            let result = (|| {
                let path = slot_path(root, id)?;
                s.occupied = path.exists();
                if !s.occupied {
                    return Ok(());
                }
                let manifest: Manifest = read_small(&path.join("manifest.json"))?;
                s.backup = manifest
                    .backup
                    .as_ref()
                    .and_then(|b| read_header(&path, b).ok());
                s.header = Some(read_header_metadata(&path, &manifest.main)?);
                read_header(&path, &manifest.main)?;
                Ok(())
            })();
            if let Err(e) = result {
                s.occupied = true;
                s.error = e;
            }
            s
        })
        .collect())
}
pub fn store(root: &Path, id: u32, name: &str, game: &Game, overwrite: bool) -> Result<(), String> {
    validate_name(name)?;
    let path = slot_path(root, id)?;
    fs::create_dir_all(root).map_err(error)?;
    if !path.exists() {
        let staging = tempfile::tempdir_in(root).map_err(error)?;
        let main = write_generation(staging.path(), name, game, None)?;
        atomic_json(
            &staging.path().join("manifest.json"),
            &Manifest { main, backup: None },
        )?;
        fs::rename(staging.path(), path).map_err(error)?;
        return Ok(());
    }
    if !overwrite {
        return Err("Speicherplatz ist belegt. Überschreiben bestätigen.".into());
    }
    let previous: Manifest = read_small(&path.join("manifest.json"))?;
    // Validate every persisted identifier before any mutation, including cleanup.
    generation(&path, &previous.main)?;
    if let Some(id) = &previous.backup {
        generation(&path, id)?;
    }
    // Keep a healthy backup when overwriting a damaged main.
    let backup = if read_header(&path, &previous.main).is_ok()
        && save::load(&generation(&path, &previous.main)?.join("save.json")).is_ok()
    {
        Some(previous.main.clone())
    } else {
        previous.backup.clone()
    };
    let main = write_generation(&path, name, game, None)?;
    let next = Manifest {
        main: main.clone(),
        backup,
    };
    if let Err(e) = atomic_json(&path.join("manifest.json"), &next) {
        let _ = fs::remove_dir_all(generation(&path, &main)?);
        return Err(e);
    }
    for old in [Some(previous.main), previous.backup].into_iter().flatten() {
        if Some(&old) != next.backup.as_ref()
            && let Ok(dir) = generation(&path, &old)
        {
            let _ = fs::remove_dir_all(dir);
        }
    }
    Ok(())
}
pub fn load(root: &Path, id: u32, backup: bool) -> Result<Game, String> {
    let path = slot_path(root, id)?;
    let manifest: Manifest = read_small(&path.join("manifest.json"))?;
    let gen_id = if backup {
        manifest
            .backup
            .as_ref()
            .ok_or("Keine Sicherung vorhanden.")?
    } else {
        &manifest.main
    };
    read_header(&path, gen_id)?;
    save::load(&generation(&path, gen_id)?.join("save.json"))
}
pub fn delete(root: &Path, id: u32) -> Result<(), String> {
    let path = slot_path(root, id)?;
    if !path.exists() {
        return Err("Speicherplatz ist leer.".into());
    }
    // Commit the import marker before deleting slot zero; never resurrect legacy.
    if id == 0 {
        atomic_json(&root.join("legacy-imported"), &true)?;
    }
    let tomb = tempfile::tempdir_in(root).map_err(error)?;
    fs::rename(path, tomb.path().join("deleted")).map_err(error)?;
    // Once renamed, the slot and backup are gone together. TempDir owns cleanup.
    Ok(())
}
