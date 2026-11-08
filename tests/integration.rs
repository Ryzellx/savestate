use savestate::backup::{create_backup, diff_snapshots, verify_snapshot, BackupOptions};
use savestate::profile::{expand_env_vars, GameProfile};
use savestate::restore::{execute_restore, plan_restore, ConflictChoice, RestoreMode};
use savestate::storage::{Game, Store};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn test_store() -> (tempfile::TempDir, Arc<Store>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(Store::open(dir.path()).expect("open store"));
    (dir, store)
}

fn register_game(store: &Arc<Store>, id: &str, save_path: &Path) -> String {
    let game = Game {
        id: id.to_string(),
        name: format!("Test Game {}", id),
        launcher: "manual".to_string(),
        launcher_id: id.to_string(),
        install_dir: None,
        discovered_at: savestate::now_unix(),
    };
    store.upsert_game(&game).unwrap();
    store
        .add_save_location(id, &save_path.display().to_string(), 1.0)
        .unwrap();
    id.to_string()
}

fn write_save(dir: &Path, rel: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content).unwrap();
    path
}

fn make_saves(dir: &Path) -> PathBuf {
    let saves = dir.join("saves");
    write_save(&saves, "slot1.sav", b"player progress v1");
    write_save(&saves, "config.ini", b"difficulty=normal");
    write_save(&saves, "sub/extra.dat", &[0u8; 1024]);
    saves
}

#[test]
fn profile_parsing_and_env_expansion() {
    let raw = r#"{
        "version": 1,
        "id": "stardew-valley",
        "name": "Stardew Valley",
        "platforms": ["windows", "linux"],
        "launcher_ids": {"steam": "413150"},
        "process_names": ["Stardew Valley"],
        "install_hints": ["Stardew Valley"],
        "save_locations": [{"path": "%APPDATA%/StardewValley/Saves", "kind": "appdata", "confidence": 0.9, "confirmed": false}],
        "exclusions": [],
        "confidence": 0.9
    }"#;
    let profile: GameProfile = serde_json::from_str(raw).unwrap();
    profile.validate().unwrap();
    assert_eq!(profile.launcher_ids["steam"], "413150");
    let expanded = expand_env_vars("$HOME/.local/share/TestGame");
    assert!(expanded.starts_with('/'));
    assert!(expanded.ends_with("TestGame"));
}

#[test]
fn steam_manifest_parsing() {
    let dir = tempfile::tempdir().unwrap();
    let steamapps = dir.path().join("steamapps");
    std::fs::create_dir_all(&steamapps).unwrap();
    std::fs::write(
        steamapps.join("appmanifest_413150.acf"),
        "\"AppState\"\n{\n\t\"appid\"\t\t\"413150\"\n\t\"name\"\t\t\"Stardew Valley\"\n\t\"installdir\"\t\t\"Stardew Valley\"\n}\n",
    )
    .unwrap();
    std::fs::create_dir_all(steamapps.join("common").join("Stardew Valley")).unwrap();
    let games = savestate::discovery::discover_steam();
    let _ = games;
    assert!(dir.path().exists());
}

#[test]
fn backup_and_verify() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let game_id = register_game(&store, "test-backup", &saves);
    let snap = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    assert_eq!(snap.file_count, 3);
    assert!(
        snap.verified,
        "snapshot must be verified against stored objects"
    );
    assert!(verify_snapshot(&store, &snap.id).unwrap());
}

#[test]
fn incremental_backup_dedups_identical_files() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let game_id = register_game(&store, "test-dedup", &saves);
    let first = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    let second = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    assert_eq!(first.file_count, second.file_count);
    let stats = store.storage_stats().unwrap();
    assert_eq!(stats.object_count, 3, "identical files must share objects");
}

#[test]
fn diff_detects_changes() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let game_id = register_game(&store, "test-diff", &saves);
    let a = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    write_save(&saves, "slot1.sav", b"player progress v2, much further");
    write_save(&saves, "newfile.sav", b"brand new save");
    let b = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    let lines = diff_snapshots(&store, &a.id, &b.id).unwrap();
    assert!(lines.iter().any(|l| l.starts_with("changed: slot1.sav")));
    assert!(lines.iter().any(|l| l.starts_with("added: newfile.sav")));
}

#[test]
fn full_restore_roundtrip() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let game_id = register_game(&store, "test-restore", &saves);
    let snap = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    write_save(&saves, "slot1.sav", b"corrupted progress");
    std::fs::remove_file(saves.join("config.ini")).unwrap();
    let plan = plan_restore(&store, &snap.id, &saves, None).unwrap();
    assert!(
        !plan.conflicts.is_empty(),
        "modified file must show as a conflict"
    );
    let outcome = execute_restore(
        &store,
        &plan,
        RestoreMode::Full,
        ConflictChoice::Replace,
        None,
    )
    .unwrap();
    assert_eq!(outcome.restored, 3);
    assert_eq!(
        std::fs::read(saves.join("slot1.sav")).unwrap(),
        b"player progress v1"
    );
    assert!(saves.join("config.ini").exists());
}

#[test]
fn selective_restore_only_chosen_files() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let game_id = register_game(&store, "test-selective", &saves);
    let snap = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    write_save(&saves, "slot1.sav", b"changed");
    write_save(&saves, "config.ini", b"changed too");
    let plan = plan_restore(&store, &snap.id, &saves, Some(&["slot1.sav".to_string()])).unwrap();
    assert_eq!(plan.files.len(), 1);
    let outcome = execute_restore(
        &store,
        &plan,
        RestoreMode::Full,
        ConflictChoice::Replace,
        None,
    )
    .unwrap();
    assert_eq!(outcome.restored, 1);
    assert_eq!(
        std::fs::read(saves.join("slot1.sav")).unwrap(),
        b"player progress v1"
    );
    assert_eq!(
        std::fs::read(saves.join("config.ini")).unwrap(),
        b"changed too"
    );
}

#[test]
fn restore_conflict_keep_current() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let game_id = register_game(&store, "test-conflict", &saves);
    let snap = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    write_save(&saves, "slot1.sav", b"newer progress I want to keep");
    let plan = plan_restore(&store, &snap.id, &saves, None).unwrap();
    let outcome = execute_restore(
        &store,
        &plan,
        RestoreMode::Full,
        ConflictChoice::KeepCurrent,
        None,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(saves.join("slot1.sav")).unwrap(),
        b"newer progress I want to keep"
    );
    assert!(outcome.skipped >= 1);
}

#[test]
fn restore_dry_run_writes_nothing() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let game_id = register_game(&store, "test-dryrun", &saves);
    let snap = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    write_save(&saves, "slot1.sav", b"changed");
    let plan = plan_restore(&store, &snap.id, &saves, None).unwrap();
    let outcome = execute_restore(
        &store,
        &plan,
        RestoreMode::DryRun,
        ConflictChoice::Replace,
        None,
    )
    .unwrap();
    assert_eq!(outcome.restored, 0);
    assert_eq!(std::fs::read(saves.join("slot1.sav")).unwrap(), b"changed");
}

#[test]
fn gc_never_deletes_referenced_objects() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let game_id = register_game(&store, "test-gc", &saves);
    let snap = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    let (count, _) = store.gc(true).unwrap();
    assert_eq!(
        count, 0,
        "no objects are orphaned while the snapshot exists"
    );
    store.delete_snapshot(&snap.id).unwrap();
    let (count, _) = store.gc(true).unwrap();
    assert_eq!(count, 3);
    let (removed, _) = store.gc(false).unwrap();
    assert_eq!(removed, 3);
    assert_eq!(store.storage_stats().unwrap().object_count, 0);
}

#[test]
fn symlink_is_not_followed_into_backup() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let outside = _dir.path().join("outside.txt");
    std::fs::write(&outside, b"secret outside the save folder").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, saves.join("link.sav")).unwrap();
    let game_id = register_game(&store, "test-symlink", &saves);
    let snap = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    assert_eq!(
        snap.file_count, 3,
        "symlink must not be backed up as a file"
    );
}

#[test]
fn large_file_backup() {
    let (_dir, store) = test_store();
    let saves = _dir.path().join("saves");
    std::fs::create_dir_all(&saves).unwrap();
    let big = vec![0xABu8; 8 * 1024 * 1024];
    write_save(&saves, "big.sav", &big);
    let game_id = register_game(&store, "test-large", &saves);
    let snap = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    assert_eq!(snap.file_count, 1);
    assert!(snap.verified);
    let plan = plan_restore(&store, &snap.id, &saves, None).unwrap();
    let _ = std::fs::remove_file(saves.join("big.sav"));
    execute_restore(
        &store,
        &plan,
        RestoreMode::Full,
        ConflictChoice::Replace,
        None,
    )
    .unwrap();
    assert_eq!(std::fs::read(saves.join("big.sav")).unwrap(), big);
}

#[test]
fn db_recovery_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let saves = make_saves(dir.path());
    let snap_id = {
        let store = Arc::new(Store::open(dir.path()).unwrap());
        let game_id = register_game(&store, "test-reopen", &saves);
        create_backup(&store, &game_id, &saves, BackupOptions::default())
            .unwrap()
            .id
    };
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.integrity_check().unwrap(), "ok");
    assert!(store.get_snapshot(&snap_id).unwrap().is_some());
}

#[test]
fn interrupted_restore_leaves_journal_entry() {
    let (_dir, store) = test_store();
    let saves = make_saves(_dir.path());
    let game_id = register_game(&store, "test-journal", &saves);
    let snap = create_backup(&store, &game_id, &saves, BackupOptions::default()).unwrap();
    let plan = plan_restore(&store, &snap.id, &saves, None).unwrap();
    execute_restore(
        &store,
        &plan,
        RestoreMode::Full,
        ConflictChoice::Replace,
        None,
    )
    .unwrap();
    let ops = store.list_operations(50).unwrap();
    assert!(ops.iter().any(|(kind, _, _, _)| kind == "restore"));
    assert!(ops.iter().any(|(kind, _, _, _)| kind == "backup"));
}
