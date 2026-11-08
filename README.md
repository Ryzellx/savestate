# SaveState

Offline-first universal game save backup and restoration manager for
Windows and Linux. Built in Rust with egui. No account, no cloud, no
telemetry.

![License: MIT](https://img.shields.io/badge/license-MIT-green)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-blue)

## What it is

SaveState finds your installed games, discovers where their saves live,
takes verified snapshots of those saves, and restores them safely. Every
snapshot is independently restorable, every restore takes a safety backup
first, and nothing is ever overwritten silently.

## Screenshots

No screenshots yet. Real screenshots of the running app will be added to
`docs/screenshots/` once captured; none are faked here.

## Features

- Game discovery: Steam (`libraryfolders.vdf` + `appmanifest_*.acf`),
  Epic manifests, GOG database, custom folders, manual registration
- Versioned JSON game profiles (`profiles/`) with save locations,
  confidence levels, and exclusions. Profile content is data, never code.
- Backup engine: BLAKE3 content-addressed objects, file-level
  deduplication, incremental snapshots, optional Zstandard compression,
  file-stability checks, verification that reads back stored objects
- Restore engine: 13-step staged restore with safety backup, staging,
  hash verification, change preview, conflict resolution (replace, keep
  current, preserve both, skip, abort), dry run, selective and export
  modes, operation journal for recovery
- Storage: SQLite metadata plus object store under `savestate-data/`,
  safe garbage collection that never deletes referenced objects
- Automation: per-game policies for launch, exit, change-triggered, and
  scheduled backups (change watcher included)
- CLI with the same engine: `scan`, `games`, `game info`, `backup`,
  `snapshots`, `diff`, `verify`, `restore`, `storage info`, `gc`,
  `automation status`, `doctor`

## Install

### From a release

Download the release for your platform from the Releases page and run the
binary. SaveState stores its data in:

- Linux: `~/.local/share/savestate-data/`
- Windows: `%LOCALAPPDATA%\SaveState\`

### From source

Requires a stable Rust toolchain (1.85 or newer) and, on Linux, X11/GL
development libraries (`libxcb`, `libxkbcommon`, `libGL`).

```sh
git clone https://github.com/ryzellx/savestate
cd savestate
cargo build --release
./target/release/savestate
```

## Quick start

1. Run `savestate`. It scans for installed games on first start.
2. Open a game in Game Details, confirm its save location (uncertain
   ones ask you first), and press **Back up now**.
3. To go back in time, open the Restore Wizard, pick a snapshot, review
   the conflicts, and run the restore. A safety backup of your current
   saves is taken first.

## CLI reference

See [docs/CLI.md](docs/CLI.md). Quick examples:

```sh
savestate scan
savestate games
savestate game info stardew-valley
savestate backup stardew-valley
savestate backup --all
savestate snapshots stardew-valley
savestate diff snap_a snap_b
savestate verify --all
savestate restore stardew-valley --latest
savestate restore stardew-valley --snapshot snap_001 --dry-run
savestate storage info
savestate gc --dry-run
savestate automation status
savestate doctor
```

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). In short: discovery
parsers feed a SQLite store; the backup engine writes BLAKE3-addressed
objects and manifests; the restore engine stages, verifies, and commits
through an operation journal; the egui app and the CLI share the same
engine code.

## Safety model

- Restore is the highest-priority component: safety backup, staging,
  hash verification before commit, destination verification after commit,
  journal entries for every step.
- Backups never follow symlinks out of the save folder.
- Garbage collection only removes objects no retained snapshot references.
- Tests use synthetic saves in temporary directories only, never real
  user saves.

## Known limitations

- **Xbox / Microsoft Store:** save locations are sandboxed and not
  reliably discoverable from metadata; register these games manually
  with a custom save path.
- **EA App and Ubisoft Connect:** no public local manifest format is
  parsed in this release; games from those launchers can be added
  manually.
- **Cloud-only saves:** Steam Cloud saves with no local files have
  nothing local to back up until the game writes files to disk.
- **Game running check:** on Linux the check reads `/proc`; it only
  works when the game's process name is known from its profile.

## Contributing

Issues and pull requests are welcome. Run `cargo fmt`, `cargo clippy
--all-targets`, and `cargo test` before submitting.

## License

MIT. See [LICENSE](LICENSE).
