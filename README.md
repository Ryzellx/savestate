<p align="center">
  <img src="assets/logo.svg" width="120" alt="SaveState logo">
</p>

<h1 align="center">SaveState</h1>

<p align="center">
  <b>Offline-first game save backup &amp; restore for Windows and Linux.</b><br>
  Built in Rust with egui. No account, no cloud, no telemetry.
</p>

<p align="center">
  <a href="https://github.com/Ryzellx/savestate/actions/workflows/ci.yml"><img src="https://github.com/Ryzellx/savestate/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/Ryzellx/savestate/releases/latest"><img src="https://img.shields.io/github/v/release/Ryzellx/savestate" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License: MIT">
  <img src="https://img.shields.io/badge/rust-stable-orange" alt="Rust stable">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-blue" alt="Windows and Linux">
</p>

<p align="center">
  <a href="https://github.com/Ryzellx/savestate/releases/latest">⬇ Download</a>
  ·
  <a href="docs/CLI.md">CLI reference</a>
  ·
  <a href="docs/ARCHITECTURE.md">Architecture</a>
  ·
  <a href="CHANGELOG.md">Changelog</a>
</p>

---

<p align="center">
  SaveState finds your installed games, discovers where their saves live,<br>
  takes verified snapshots of those saves, and restores them safely.<br>
  Every snapshot is independently restorable, every restore takes a safety<br>
  backup first, and nothing is ever overwritten silently.
</p>

## ✨ Features

<div align="center">

| Feature | What it does |
|---|---|
| 🎮 Game discovery | Steam (`libraryfolders.vdf` + `appmanifest_*.acf`), Epic manifests, GOG database, custom folders, manual registration |
| 🗂️ Versioned game profiles | JSON profiles in `profiles/` with save locations, confidence levels, and exclusions. Profile content is data, never code. |
| 💾 Backup engine | BLAKE3 content-addressed objects, file-level deduplication, incremental snapshots, optional Zstandard compression, file-stability checks, verification that reads back stored objects |
| ↩️ Restore engine | 13-step staged restore with safety backup, staging, hash verification, change preview, conflict resolution (replace, keep current, preserve both, skip, abort), dry run, selective and export modes, operation journal for recovery |
| 🗄️ Storage | SQLite metadata plus object store under `savestate-data/`, safe garbage collection that never deletes referenced objects |
| 🤖 Automation | Per-game policies for launch, exit, change-triggered, and scheduled backups (change watcher included) |
| ⌨️ CLI | `scan`, `games`, `game info`, `backup`, `snapshots`, `diff`, `verify`, `restore`, `storage info`, `gc`, `automation status`, `doctor` |

</div>

## 📸 Screenshots

<p align="center">
  <i>No screenshots yet — real screenshots of the running app will be added to <code>docs/screenshots/</code> once captured. None are faked here.</i>
</p>

## 📥 Install

<div align="center">

| From a release | From source |
|---|---|
| Download the archive for your platform from the [latest release](https://github.com/Ryzellx/savestate/releases/latest) and run the binary. On Windows, double-clicking the exe opens just the app window (no console window); running it from a terminal with a command still prints output. | Requires a stable Rust toolchain (1.85+) and, on Linux, X11/GL dev libraries (`libxcb`, `libxkbcommon`, `libGL`). |

</div>

```sh
git clone https://github.com/Ryzellx/savestate
cd savestate
cargo build --release
./target/release/savestate
```

<p align="center">
  SaveState stores its data in <code>~/.local/share/savestate-data/</code> on Linux<br>
  and <code>%LOCALAPPDATA%\SaveState\</code> on Windows.
</p>

## 🚀 Quick start

1. Run `savestate`. It scans for installed games on first start.
2. Open a game in Game Details, confirm its save location (uncertain ones ask you first), and press **Back up now**.
3. To go back in time, open the Restore Wizard, pick a snapshot, review the conflicts, and run the restore. A safety backup of your current saves is taken first.

## ⌨️ CLI

See the full [CLI reference](docs/CLI.md). Quick examples:

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

## 🏗️ Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). In short: discovery parsers feed a SQLite store; the backup engine writes BLAKE3-addressed objects and manifests; the restore engine stages, verifies, and commits through an operation journal; the egui app and the CLI share the same engine code.

## 🛡️ Safety model

<p align="center">
  Restore is the highest-priority component: safety backup, staging,<br>
  hash verification before commit, destination verification after commit,<br>
  journal entries for every step.
</p>

- Backups never follow symlinks out of the save folder.
- Garbage collection only removes objects no retained snapshot references.
- Tests use synthetic saves in temporary directories only, never real user saves.

## ⚠️ Known limitations

- **Xbox / Microsoft Store:** save locations are sandboxed and not reliably discoverable from metadata; register these games manually with a custom save path.
- **EA App and Ubisoft Connect:** no public local manifest format is parsed in this release; games from those launchers can be added manually.
- **Cloud-only saves:** Steam Cloud saves with no local files have nothing local to back up until the game writes files to disk.
- **Game running check:** on Linux the check reads `/proc`; it only works when the game's process name is known from its profile.

## 🤝 Contributing

Issues and pull requests are welcome. Run `cargo fmt`, `cargo clippy --all-targets`, and `cargo test` before submitting.

---

<p align="center">
  MIT License — see <a href="LICENSE">LICENSE</a>
</p>
