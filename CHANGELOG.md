# Changelog

All notable changes to SaveState will be documented here.

## [0.1.1] - 2026-11-09

- Windows: double-clicking the release exe no longer opens a console
  window. Running the exe from a terminal with a command still prints
  output (the app re-attaches to the parent console).
- New interface: Space Grotesk headings, Inter body text, JetBrains Mono
  for IDs and timestamps; subtle dark vertical gradient in the sidebar
  and main area with a faint accent tint; rounded cards with soft
  shadows; animated sidebar collapse, page fade, staggered card
  entrance, and slide-in toast notifications. Reduced-motion setting
  makes every transition instant. The app only repaints while an
  animation is active.
- Header bar with the app name and version; dashboard stat cards;
  Restore Wizard step indicator with numbered steps.

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
