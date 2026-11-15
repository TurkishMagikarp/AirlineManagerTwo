use crate::{game::Game, save, save_slots as slots};
use std::{fs, path::Path};
fn game() -> Game {
    let mut g = Game::default();
    g.found("Slot Air", "DE", "EDDF").unwrap();
    g.lease_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn files(path: &Path) -> Vec<(String, Vec<u8>)> {
    fn visit(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(base, &path, out);
            } else {
                out.push((
                    path.strip_prefix(base).unwrap().to_string_lossy().into(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut out = vec![];
    visit(path, path, &mut out);
    out.sort();
    out
}
fn main_file(root: &Path, id: u32) -> std::path::PathBuf {
    let slot = root.join(format!("slot-{id:02}"));
    let m: serde_json::Value =
        serde_json::from_slice(&fs::read(slot.join("manifest.json")).unwrap()).unwrap();
    slot.join(m["main"].as_str().unwrap()).join("save.json")
}
#[test]
fn save_slots_roundtrip_backup_corruption_delete_and_jump() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Spielstände 日本語");
    let mut a = game();
    let first = state(&a);
    slots::store(&root, 0, "Vor dem Sprung 日本語", &a, false).unwrap();
    a.jump(86400.0).unwrap();
    slots::store(&root, 1, "Nach dem Sprung", &a, false).unwrap();
    let original = slots::load(&root, 0, false).unwrap();
    assert_eq!(state(&original), first);
    assert_ne!(state(&original), state(&a));
    assert_eq!(state(&slots::load(&root, 1, false).unwrap()), state(&a));
    slots::store(&root, 0, "Umbenannt", &a, true).unwrap();
    assert_eq!(
        state(&slots::load(&root, 0, true).unwrap()),
        state(&original)
    );
    let mut loaded = slots::load(&root, 0, false).unwrap();
    let mut uninterrupted = a.clone();
    loaded.jump(7.0 * 86400.0).unwrap();
    uninterrupted.jump(7.0 * 86400.0).unwrap();
    assert_eq!(state(&loaded), state(&uninterrupted)); // Includes all AI owners.
    let old_bytes = fs::read(main_file(&root, 0)).unwrap();
    for corrupt in [b"{".to_vec(), vec![b'x'; old_bytes.len()]] {
        fs::write(main_file(&root, 0), corrupt).unwrap();
        let list = slots::list(&root, &temp.path().join("missing")).unwrap();
        assert!(!list[0].error.is_empty());
        assert!(list[0].backup.is_some());
        assert!(slots::load(&root, 0, false).is_err());
        assert_eq!(
            state(&slots::load(&root, 0, true).unwrap()),
            state(&original)
        );
    }
    // Overwriting a broken main keeps the last healthy backup.
    slots::store(&root, 0, "Repariert", &a, true).unwrap();
    assert_eq!(
        state(&slots::load(&root, 0, true).unwrap()),
        state(&original)
    );
    let other = fs::read(main_file(&root, 1)).unwrap();
    fs::write(root.join("unrelated.txt"), b"keep").unwrap();
    slots::delete(&root, 0).unwrap();
    assert!(!root.join("slot-00").exists());
    assert_eq!(fs::read(main_file(&root, 1)).unwrap(), other);
    assert_eq!(fs::read(root.join("unrelated.txt")).unwrap(), b"keep");
}
#[test]
fn save_slots_names_boundaries_and_failed_writes_are_transactional() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("saves");
    let g = game();
    slots::store(&root, 0, "Eins", &g, false).unwrap();
    slots::store(&root, 0, "Zwei", &g, true).unwrap();
    let before = files(&root);
    for bad in [
        "..",
        ".",
        "/",
        "a/b",
        "a\\b",
        "",
        " ",
        "end.",
        "CON",
        "con.txt",
        "NUL",
        "COM1",
        "LPT9",
        "a\n",
        "x:y",
        &"長".repeat(slots::RULES.name_length + 1),
    ] {
        assert!(slots::store(&root, 0, bad, &g, true).is_err(), "{bad:?}");
    }
    assert!(slots::store(&root, 0, "Taken", &g, false).is_err());
    assert!(slots::store(&root, slots::RULES.slot_count, "Outside", &g, true).is_err());
    let mut invalid = g.clone();
    invalid.fleet[0].id = 0;
    assert!(slots::store(&root, 0, "Invalid", &invalid, true).is_err());
    assert_eq!(files(&root), before);
    slots::FAIL_COMMIT.with(|f| f.set(true));
    assert!(slots::store(&root, 0, "Disk failure", &g, true).is_err());
    assert_eq!(files(&root), before);
    assert_eq!(state(&slots::load(&root, 0, false).unwrap()), state(&g));
    assert_eq!(state(&slots::load(&root, 0, true).unwrap()), state(&g));
}
#[test]
fn save_slots_legacy_import_is_once_and_byte_preserving() {
    let temp = tempfile::tempdir().unwrap();
    let legacy = temp.path().join("savegame.json");
    let root = temp.path().join("saves");
    let g = game();
    save::save(&legacy, &g).unwrap();
    let original = fs::read(&legacy).unwrap();
    let rows = slots::list(&root, &legacy).unwrap();
    assert_eq!(
        rows[0].header.as_ref().unwrap().name,
        "Bisheriger Spielstand"
    );
    assert_eq!(state(&slots::load(&root, 0, false).unwrap()), state(&g));
    assert_eq!(fs::read(main_file(&root, 0)).unwrap(), original);
    assert_eq!(fs::read(&legacy).unwrap(), original);
    slots::delete(&root, 0).unwrap();
    assert!(!slots::list(&root, &legacy).unwrap()[0].occupied);
    assert_eq!(fs::read(&legacy).unwrap(), original);
}
#[test]
fn save_slots_listing_is_bounded_header_only_and_tolerates_bad_headers() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("saves");
    let g = game();
    for id in 0..slots::RULES.slot_count {
        slots::store(&root, id, "Slot", &g, false).unwrap();
    }
    // Sparse 200 MiB unreadable JSON bodies: listing succeeds without deserializing them.
    // Refresh file metadata in headers to isolate the no-body-read contract.
    for id in 0..slots::RULES.slot_count {
        let path = main_file(&root, id);
        fs::File::create(&path)
            .unwrap()
            .set_len(200 * 1024 * 1024)
            .unwrap();
        let header_path = path.parent().unwrap().join("header.json");
        let mut h: serde_json::Value =
            serde_json::from_slice(&fs::read(&header_path).unwrap()).unwrap();
        h["bytes"] = (200_u64 * 1024 * 1024).into();
        h["modified_ns"] = serde_json::to_value(
            fs::metadata(&path)
                .unwrap()
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
        .unwrap();
        fs::write(header_path, serde_json::to_vec(&h).unwrap()).unwrap();
    }
    let start = std::time::Instant::now();
    let rows = slots::list(&root, &temp.path().join("absent")).unwrap();
    assert!(
        rows.iter()
            .all(|r| r.header.is_some() && r.error.is_empty())
    );
    assert!(start.elapsed().as_secs() < 2);
    fs::remove_file(main_file(&root, 0).parent().unwrap().join("header.json")).unwrap();
    fs::write(
        main_file(&root, 1).parent().unwrap().join("header.json"),
        b"{",
    )
    .unwrap();
    let rows = slots::list(&root, &temp.path().join("absent")).unwrap();
    assert!(!rows[0].error.is_empty() && !rows[1].error.is_empty());
    assert!(rows[2].header.is_some());
}
#[cfg(unix)]
#[test]
fn save_slots_reject_symlinks_and_manifest_path_traversal() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("saves");
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::create_dir(&root).unwrap();
    symlink(&outside, root.join("slot-00")).unwrap();
    assert!(slots::store(&root, 0, "Escape", &game(), false).is_err());
    assert!(slots::delete(&root, 0).is_err());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
    slots::store(&root, 1, "Safe", &game(), false).unwrap();
    fs::write(
        root.join("slot-01/manifest.json"),
        br#"{"main":"../../outside","backup":null}"#,
    )
    .unwrap();
    assert!(slots::load(&root, 1, false).is_err());
}

#[cfg(unix)]
#[test]
fn save_slots_unreadable_main_is_reported_without_reading_the_archive() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("saves");
    let g = game();
    slots::store(&root, 0, "Before", &g, false).unwrap();
    slots::store(&root, 0, "After", &g, true).unwrap();
    let path = main_file(&root, 0);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    let rows = slots::list(&root, &temp.path().join("missing")).unwrap();
    assert!(!rows[0].error.is_empty());
    assert!(rows[0].header.is_some() && rows[0].backup.is_some());
    assert_eq!(state(&slots::load(&root, 0, true).unwrap()), state(&g));
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    let before = files(&root);
    assert!(slots::delete(&root, 1).is_err());
    assert_eq!(files(&root), before);
}
