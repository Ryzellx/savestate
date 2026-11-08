# SaveState CLI

Run `savestate --help` for the full list. All commands operate on the same
local data directory as the GUI (`~/.local/share/savestate-data` on Linux,
`%LOCALAPPDATA%\SaveState` on Windows, or `--data-dir <path>`).

| Command | What it does |
| --- | --- |
| `savestate scan` | Detect installed games from Steam, Epic, GOG; load profiles |
| `savestate games` | List registered games with snapshot counts |
| `savestate game info <id-or-name>` | Game, install dir, save locations, recent snapshots |
| `savestate game register <id> <name> <save-path>` | Register a game manually |
| `savestate backup <id>` | Back up one game's saves |
| `savestate backup --all` | Back up every registered game with a known save location |
| `savestate snapshots <id>` | List snapshots for a game |
| `savestate diff <snap-a> <snap-b>` | Added, removed, and changed files between snapshots |
| `savestate verify <snapshot>` | Re-read and re-hash every stored object |
| `savestate verify --all` | Verify every snapshot |
| `savestate restore <id> --latest` | Restore the newest snapshot (refuses on conflicts unless `--on-conflict` is given) |
| `savestate restore <id> --snapshot <id> [--dry-run] [--on-conflict replace\|keep\|both\|skip\|abort]` | Restore a snapshot, or preview only |
| `savestate storage info` | Games, snapshots, objects, dedup savings |
| `savestate gc --dry-run` / `--execute` | Report or remove unreferenced objects |
| `savestate automation status` | Per-game backup policies |
| `savestate doctor` | Data dir, database integrity, launcher detection |

With no subcommand, `savestate` opens the desktop GUI.

## Restore safety notes

- The CLI refuses to restore when existing files differ from the
  snapshot. Use the GUI restore wizard to choose a conflict policy
  (replace, keep current, preserve both, skip, abort).
- Every restore takes a safety backup of the current saves first and
  verifies the destination afterwards.
