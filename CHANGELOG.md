# Changelog

All notable changes to SaveState will be documented here.

## [0.1.0] - 2026-11-08

- Initial release.
- Game discovery for Steam, Epic, and GOG; manual registration; bundled
  profiles for Stardew Valley, Minecraft: Java Edition, and Terraria.
- Backup engine: BLAKE3 content-addressed objects, file-level dedup,
  incremental snapshots, optional Zstandard compression, verification
  against stored objects.
- Restore engine: staged restore with safety backup, hash verification,
  conflict resolution, dry run, selective and export modes, operation
  journal.
- Storage: SQLite metadata, safe garbage collection, storage statistics.
- Automation: per-game policies, on-change watcher.
- Desktop GUI (egui) with Dashboard, Game Library, Game Details,
  Snapshot Manager, Restore Wizard, Storage Manager, Automation, and
  Settings pages.
- CLI: scan, games, game info/register, backup, snapshots, diff, verify,
  restore, storage info, gc, automation status, doctor.
