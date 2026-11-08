# Architecture

## Layout

```
savestate/
  src/
    main.rs        entry point: CLI dispatch, or GUI + auto-scan
    lib.rs         data-dir resolution, time/byte formatting helpers
    app.rs         egui application: 8 pages, restore wizard, toasts
    ui.rs          theme palette and small shared components
    profile.rs     versioned JSON game profiles, env-var expansion
    discovery.rs   Steam/Epic/GOG parsers
    backup.rs      object store, snapshot creation, verify, diff
    restore.rs     staged restore, conflicts, disk-space and process checks
    storage.rs     SQLite store (all 12 PRD tables), GC, policies
    watcher.rs     notify-based on-change watcher with settle delay
    settings.rs    settings load/save
    platform/      platform notes and known limitations
    cli.rs         clap commands, all backed by the same engine
  profiles/        bundled game profiles (Stardew Valley, Minecraft, Terraria)
  tests/           integration tests against temp directories
```

## Data flow

1. Discovery parses launcher metadata into the `games` table. Profiles
   in `profiles/` add known save locations with confidence levels.
2. A backup walks a save folder (symlinks skipped), waits for each file
   to be stable, hashes it with BLAKE3, and stores the bytes once under
   `objects/<first-2-hex-chars>/<hash>` (zstd-compressed when smaller).
   A snapshot row plus per-file rows plus a JSON manifest describe the
   backup. Verification re-reads every object and re-hashes it before
   the snapshot is marked verified.
3. A restore plans first (conflicts, disk space, game-running check),
   stages all files under `staging/`, verifies their hashes, takes a
   safety backup of the current saves, then commits file by file and
   verifies the destination. Every step writes to the `operations`
   journal, so an interrupted restore can be diagnosed and rolled back
   from the safety backup.
4. GC lists objects with no referencing `snapshot_files` row and only
   deletes those.

## Storage

```
savestate-data/
  database/savestate.db   games, installations, save_locations, profiles,
                          snapshots, snapshot_files, objects, operations,
                          backup_policies, exclusions, settings, schema_metadata
  objects/                content-addressed blobs, sharded by hash prefix
  manifests/              JSON manifest per snapshot
  staging/                restore staging area (cleaned after each restore)
  logs/                   reserved for log output
  profiles/               versioned JSON game profiles
```

SQLite runs in WAL mode with foreign keys on. Snapshot inserts are
transactional. Object writes go through a temp file plus rename.

## Threading

- The GUI runs on the main thread via eframe.
- The on-change watcher runs on a plain std thread with a 5-second
  settle delay before triggering an automatic backup.
- Tokio is present for future async scheduling; the backup and restore
  engines are synchronous by design because they are disk-bound and
  must be easy to reason about step by step.
