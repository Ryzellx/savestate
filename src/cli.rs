use crate::backup::{create_backup, diff_snapshots, verify_snapshot, BackupOptions};
use crate::discovery::discover_all;
use crate::profile::{load_profiles, profile_path};
use crate::restore::{
    check_disk_space, execute_restore, plan_restore, process_running, ConflictChoice, RestoreMode,
};
use crate::storage::{Game, Store};
use crate::{format_bytes, format_time, now_unix};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(
    name = "savestate",
    version,
    about = "Offline-first universal game save backup and restoration manager"
)]
pub struct Cli {
    #[arg(long, global = true, help = "Data directory override")]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Scan,
    Games,
    Game {
        #[command(subcommand)]
        cmd: GameCmd,
    },
    Backup {
        game_id: Option<String>,
        #[arg(long)]
        all: bool,
    },
    Snapshots {
        game_id: String,
    },
    Diff {
        snapshot_a: String,
        snapshot_b: String,
    },
    Verify {
        #[arg(long)]
        all: bool,
        snapshot: Option<String>,
    },
    Restore {
        game_id: String,
        #[arg(long)]
        latest: bool,
        #[arg(long)]
        snapshot: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        target: Option<PathBuf>,
        #[arg(long, default_value = "abort", value_parser = ["replace", "keep", "both", "skip", "abort"])]
        on_conflict: String,
    },
    Storage {
        #[command(subcommand)]
        cmd: StorageCmd,
    },
    Gc {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        execute: bool,
    },
    Automation {
        #[command(subcommand)]
        cmd: AutomationCmd,
    },
    Doctor,
}

#[derive(Subcommand)]
pub enum GameCmd {
    Info {
        game_id: String,
    },
    Register {
        game_id: String,
        name: String,
        save_path: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum StorageCmd {
    Info,
}

#[derive(Subcommand)]
pub enum AutomationCmd {
    Status,
}

pub fn game_id_for(store: &Arc<Store>, query: &str) -> Result<String, String> {
    if store.get_game(query)?.is_some() {
        return Ok(query.to_string());
    }
    let games = store.list_games()?;
    let lowered = query.to_lowercase();
    let matches: Vec<_> = games
        .iter()
        .filter(|g| g.name.to_lowercase().contains(&lowered))
        .collect();
    match matches.len() {
        1 => Ok(matches[0].id.clone()),
        0 => Err(format!("no game matches \"{}\"", query)),
        _ => Err(format!(
            "\"{}\" matches {} games, use the exact game id",
            query,
            matches.len()
        )),
    }
}

pub fn first_save_root(store: &Arc<Store>, game_id: &str) -> Result<PathBuf, String> {
    let locations = store.list_save_locations(game_id)?;
    for loc in locations {
        let path = PathBuf::from(crate::profile::expand_env_vars(&loc.path));
        if path.exists() {
            return Ok(path);
        }
    }
    Err(format!("no existing save location known for game {}. Register one with: savestate game register {} <name> <save-path>", game_id, game_id))
}

pub fn run(cli: Cli) -> Result<i32, String> {
    let data_dir = cli.data_dir.clone().unwrap_or_else(crate::default_data_dir);
    let store = Arc::new(Store::open(&data_dir)?);
    match cli.command {
        None => Ok(2),
        Some(Commands::Scan) => {
            let found = discover_all();
            let mut added = 0;
            for g in &found {
                let id = format!("{}:{}", g.launcher.to_lowercase(), g.launcher_id);
                let game = Game {
                    id: id.clone(),
                    name: g.name.clone(),
                    launcher: g.launcher.clone(),
                    launcher_id: g.launcher_id.clone(),
                    install_dir: g.install_dir.as_ref().map(|p| p.display().to_string()),
                    discovered_at: now_unix(),
                };
                store.upsert_game(&game)?;
                added += 1;
            }
            let profiles = load_profiles(&profile_path(&data_dir))?;
            for profile in &profiles {
                store.store_profile(
                    &profile.id,
                    &profile.name,
                    &serde_json::to_string(profile).unwrap_or_default(),
                )?;
            }
            println!(
                "Scan complete: {} games found across launchers, {} profiles loaded",
                added,
                profiles.len()
            );
            for g in &found {
                println!("  [{}] {} ({})", g.launcher, g.name, g.launcher_id);
            }
            if found.is_empty() {
                println!(
                    "  No games detected. Steam/Epic/GOG metadata was not found on this machine."
                );
            }
            Ok(0)
        }
        Some(Commands::Games) => {
            let games = store.list_games()?;
            if games.is_empty() {
                println!("No games registered yet. Run: savestate scan");
            }
            for g in games {
                let snaps = store.list_snapshots(&g.id).map(|s| s.len()).unwrap_or(0);
                println!(
                    "{}  {}  [{}]  snapshots: {}",
                    g.id, g.name, g.launcher, snaps
                );
            }
            Ok(0)
        }
        Some(Commands::Game { cmd }) => match cmd {
            GameCmd::Info { game_id } => {
                let id = game_id_for(&store, &game_id)?;
                let game = store.get_game(&id)?.unwrap();
                println!("{} ({})", game.name, game.id);
                println!("  launcher: {}", game.launcher);
                println!(
                    "  install dir: {}",
                    game.install_dir.as_deref().unwrap_or("(unknown)")
                );
                println!("  discovered: {}", format_time(game.discovered_at));
                let locations = store.list_save_locations(&id)?;
                if locations.is_empty() {
                    println!("  save locations: none known");
                }
                for loc in locations {
                    println!(
                        "  save: {} (confidence {:.0}%, confirmed: {})",
                        loc.path,
                        loc.confidence * 100.0,
                        loc.confirmed
                    );
                }
                let snaps = store.list_snapshots(&id)?;
                println!("  snapshots: {}", snaps.len());
                for s in snaps.iter().take(5) {
                    println!(
                        "    {}  {}  {} files  verified: {}",
                        s.id, s.name, s.file_count, s.verified
                    );
                }
                Ok(0)
            }
            GameCmd::Register {
                game_id,
                name,
                save_path,
            } => {
                let game = Game {
                    id: game_id.clone(),
                    name: name.clone(),
                    launcher: "manual".to_string(),
                    launcher_id: game_id.clone(),
                    install_dir: None,
                    discovered_at: now_unix(),
                };
                store.upsert_game(&game)?;
                store.add_save_location(&game_id, &save_path.display().to_string(), 1.0)?;
                store.confirm_save_location(
                    store
                        .list_save_locations(&game_id)?
                        .last()
                        .map(|l| l.id)
                        .unwrap_or(0),
                )?;
                println!(
                    "Registered game \"{}\" ({}) with save path {}",
                    name,
                    game_id,
                    save_path.display()
                );
                Ok(0)
            }
        },
        Some(Commands::Backup { game_id, all }) => {
            if all {
                let games = store.list_games()?;
                let mut done = 0;
                for g in games {
                    match first_save_root(&store, &g.id).and_then(|root| {
                        create_backup(&store, &g.id, &root, BackupOptions::default())
                    }) {
                        Ok(snap) => {
                            println!(
                                "Backed up {} -> {} ({} files, verified: {})",
                                g.name, snap.id, snap.file_count, snap.verified
                            );
                            done += 1;
                        }
                        Err(e) => println!("Skipped {}: {}", g.name, e),
                    }
                }
                println!("Backup finished for {} game(s)", done);
                return Ok(0);
            }
            let query = game_id.ok_or_else(|| "specify a game id or use --all".to_string())?;
            let id = game_id_for(&store, &query)?;
            let root = first_save_root(&store, &id)?;
            let snap = create_backup(&store, &id, &root, BackupOptions::default())?;
            println!(
                "Backup complete: {} ({} files, {}, verified: {})",
                snap.id,
                snap.file_count,
                format_bytes(snap.total_bytes.max(0) as u64),
                snap.verified
            );
            Ok(0)
        }
        Some(Commands::Snapshots { game_id }) => {
            let id = game_id_for(&store, &game_id)?;
            let snaps = store.list_snapshots(&id)?;
            if snaps.is_empty() {
                println!("No snapshots for {}", id);
            }
            for s in snaps {
                println!(
                    "{}  {}  {}  {} files  {}  verified: {}",
                    s.id,
                    s.name,
                    format_time(s.created_at),
                    s.file_count,
                    format_bytes(s.total_bytes.max(0) as u64),
                    s.verified
                );
            }
            Ok(0)
        }
        Some(Commands::Diff {
            snapshot_a,
            snapshot_b,
        }) => {
            let lines = diff_snapshots(&store, &snapshot_a, &snapshot_b)?;
            if lines.is_empty() {
                println!("Snapshots are identical.");
            }
            for line in lines {
                println!("{}", line);
            }
            Ok(0)
        }
        Some(Commands::Verify { all, snapshot }) => {
            if all {
                let snaps = store.list_all_snapshots()?;
                let mut bad = 0;
                for s in &snaps {
                    let ok = verify_snapshot(&store, &s.id)?;
                    store.mark_verified(&s.id, ok)?;
                    if !ok {
                        bad += 1;
                        println!("FAILED: {}", s.id);
                    }
                }
                println!("Verified {} snapshot(s), {} failed", snaps.len(), bad);
                return Ok(if bad == 0 { 0 } else { 1 });
            }
            let id = snapshot.ok_or_else(|| "specify a snapshot id or use --all".to_string())?;
            let ok = verify_snapshot(&store, &id)?;
            store.mark_verified(&id, ok)?;
            println!(
                "Snapshot {} verification: {}",
                id,
                if ok { "passed" } else { "FAILED" }
            );
            Ok(if ok { 0 } else { 1 })
        }
        Some(Commands::Restore {
            game_id,
            latest,
            snapshot,
            dry_run,
            target,
            on_conflict,
        }) => {
            let id = game_id_for(&store, &game_id)?;
            let snap_id = if latest {
                store
                    .latest_snapshot(&id)?
                    .ok_or_else(|| format!("no snapshots for {}", id))?
                    .id
            } else {
                snapshot.ok_or_else(|| "specify --snapshot <id> or --latest".to_string())?
            };
            let root = match target {
                Some(t) => t,
                None => first_save_root(&store, &id)?,
            };
            let plan = plan_restore(&store, &snap_id, &root, None)?;
            if !plan.conflicts.is_empty() {
                println!("Conflicts (existing files differ from the snapshot):");
                for c in &plan.conflicts {
                    println!(
                        "  {} (current {} bytes, snapshot {} bytes)",
                        c.rel_path, c.current_size, c.snapshot_size
                    );
                }
            }
            if dry_run {
                println!(
                    "Dry run: {} file(s) would be restored to {}",
                    plan.files.len(),
                    root.display()
                );
                return Ok(0);
            }
            let choice = match on_conflict.as_str() {
                "replace" => ConflictChoice::Replace,
                "keep" => ConflictChoice::KeepCurrent,
                "both" => ConflictChoice::PreserveBoth,
                "skip" => ConflictChoice::Skip,
                _ => {
                    if !plan.conflicts.is_empty() {
                        println!("Refusing to overwrite without an explicit choice. Re-run with --on-conflict replace|keep|both|skip, or use the GUI restore wizard.");
                        return Ok(1);
                    }
                    ConflictChoice::Replace
                }
            };
            check_disk_space(&root, plan.needed_bytes)?;
            let outcome = execute_restore(&store, &plan, RestoreMode::Full, choice, None)?;
            println!(
                "Restore complete: {} file(s) restored, {} skipped",
                outcome.restored, outcome.skipped
            );
            if let Some(safety) = outcome.safety_backup_id {
                println!("Safety backup before restore: {}", safety);
            }
            Ok(0)
        }
        Some(Commands::Storage { cmd }) => match cmd {
            StorageCmd::Info => {
                let stats = store.storage_stats()?;
                println!("SaveState storage");
                println!("  data dir: {}", data_dir.display());
                println!("  games: {}", stats.game_count);
                println!("  snapshots: {}", stats.snapshot_count);
                println!("  stored objects: {}", stats.object_count);
                println!("  logical size: {}", format_bytes(stats.logical_bytes));
                println!("  stored size: {}", format_bytes(stats.stored_bytes));
                let saved = stats.logical_bytes.saturating_sub(stats.stored_bytes);
                println!(
                    "  deduplication + compression saved: {}",
                    format_bytes(saved)
                );
                Ok(0)
            }
        },
        Some(Commands::Gc { dry_run, execute }) => {
            if !dry_run && !execute {
                return Err("specify --dry-run or --execute".to_string());
            }
            let (count, bytes) = store.gc(!execute)?;
            if dry_run {
                println!(
                    "GC dry run: {} unreferenced object(s), {} would be freed",
                    count,
                    format_bytes(bytes)
                );
            } else {
                println!(
                    "GC complete: removed {} unreferenced object(s), freed {}",
                    count,
                    format_bytes(bytes)
                );
            }
            Ok(0)
        }
        Some(Commands::Automation { cmd }) => match cmd {
            AutomationCmd::Status => {
                let games = store.list_games()?;
                println!("Automation status");
                for g in games {
                    match store.get_policy(&g.id)? {
                        Some((mode, interval, on_launch, on_exit, on_change, retention)) => {
                            println!("  {}: mode={} interval={}m on_launch={} on_exit={} on_change={} retention={}", g.name, mode, interval, on_launch, on_exit, on_change, retention);
                        }
                        None => println!("  {}: no policy (manual only)", g.name),
                    }
                }
                Ok(0)
            }
        },
        Some(Commands::Doctor) => {
            println!("SaveState doctor");
            println!("  data dir exists: {}", data_dir.exists());
            println!("  database integrity: {}", store.integrity_check()?);
            let stats = store.storage_stats()?;
            println!("  games: {}", stats.game_count);
            println!("  snapshots: {}", stats.snapshot_count);
            println!("  objects: {}", stats.object_count);
            let found = discover_all();
            println!("  launchers detected now: {} game(s)", found.len());
            for limit in crate::platform::known_limitations() {
                println!("  note: {}", limit);
            }
            Ok(0)
        }
    }
}

pub fn resolve_game_process_names(_store: &Arc<Store>, _game_id: &str) -> Vec<String> {
    Vec::new()
}

pub fn game_is_running(process_names: &[String]) -> bool {
    process_running(process_names)
}
