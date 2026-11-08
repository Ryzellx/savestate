use crate::backup::read_object;
use crate::storage::{SnapshotFile, Store};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    Replace,
    KeepCurrent,
    PreserveBoth,
    Skip,
    Abort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreMode {
    Full,
    Selective,
    Folder,
    Export,
    DryRun,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub rel_path: String,
    pub current_size: u64,
    pub snapshot_size: u64,
}

#[derive(Debug, Clone)]
pub struct RestorePlan {
    pub snapshot_id: String,
    pub target: PathBuf,
    pub files: Vec<SnapshotFile>,
    pub conflicts: Vec<Conflict>,
    pub needed_bytes: u64,
}

pub fn plan_restore(
    store: &Arc<Store>,
    snapshot_id: &str,
    target: &Path,
    only_files: Option<&[String]>,
) -> Result<RestorePlan, String> {
    let snapshot = store
        .get_snapshot(snapshot_id)?
        .ok_or_else(|| format!("unknown snapshot: {}", snapshot_id))?;
    let _ = snapshot;
    let mut files = store.snapshot_files(snapshot_id)?;
    if let Some(only) = only_files {
        files.retain(|f| {
            only.iter()
                .any(|o| f.rel_path == *o || f.rel_path.starts_with(&format!("{}/", o)))
        });
    }
    let mut conflicts = Vec::new();
    let mut needed: u64 = 0;
    for f in &files {
        needed += f.size;
        let dest = target.join(&f.rel_path);
        if dest.exists() {
            let current = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
            let current_data = std::fs::read(&dest).unwrap_or_default();
            let current_hash = blake3::hash(&current_data).to_hex().to_string();
            if current_hash != f.hash {
                conflicts.push(Conflict {
                    rel_path: f.rel_path.clone(),
                    current_size: current,
                    snapshot_size: f.size,
                });
            }
        }
    }
    Ok(RestorePlan {
        snapshot_id: snapshot_id.to_string(),
        target: target.to_path_buf(),
        files,
        conflicts,
        needed_bytes: needed,
    })
}

pub fn check_disk_space(target: &Path, needed: u64) -> Result<(), String> {
    let mut path = target.to_path_buf();
    while !path.exists() {
        match path.parent() {
            Some(p) => path = p.to_path_buf(),
            None => return Err("cannot determine disk space for target".to_string()),
        }
    }
    let available = fs_available_bytes(&path);
    if available < needed + 64 * 1024 * 1024 {
        return Err(format!(
            "not enough disk space: need {} but only {} is free on the target drive",
            crate::format_bytes(needed),
            crate::format_bytes(available)
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn fs_available_bytes(path: &Path) -> u64 {
    use std::os::unix::ffi::OsStrExt;
    let cstr = std::ffi::CString::new(path.as_os_str().as_bytes())
        .unwrap_or_else(|_| std::ffi::CString::new("/").unwrap());
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(cstr.as_ptr(), &mut stat) == 0 {
            stat.f_bavail as u64 * stat.f_bsize as u64
        } else {
            u64::MAX / 2
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn fs_available_bytes(_path: &Path) -> u64 {
    u64::MAX / 2
}

pub fn process_running(process_names: &[String]) -> bool {
    #[cfg(target_os = "linux")]
    {
        let entries = match std::fs::read_dir("/proc") {
            Ok(e) => e,
            Err(_) => return false,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(comm) = std::fs::read_to_string(entry.path().join("comm")) {
                    let comm = comm.trim().to_string();
                    if process_names.iter().any(|p| {
                        comm.eq_ignore_ascii_case(p)
                            || comm.to_lowercase().contains(&p.to_lowercase())
                    }) {
                        return true;
                    }
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = process_names;
        false
    }
}

pub struct RestoreOutcome {
    pub restored: usize,
    pub skipped: usize,
    pub preserved: Vec<String>,
    pub safety_backup_id: Option<String>,
}

pub fn execute_restore(
    store: &Arc<Store>,
    plan: &RestorePlan,
    mode: RestoreMode,
    choice: ConflictChoice,
    export_dir: Option<&Path>,
) -> Result<RestoreOutcome, String> {
    let snapshot = store
        .get_snapshot(&plan.snapshot_id)?
        .ok_or_else(|| "snapshot vanished".to_string())?;
    let game_id = snapshot.game_id.clone();
    store.log_operation(
        "restore",
        &game_id,
        Some(&plan.snapshot_id),
        "restore started",
        "started",
    );

    if mode == RestoreMode::DryRun {
        let mut skipped = 0;
        for f in &plan.files {
            let dest = plan.target.join(&f.rel_path);
            if dest.exists() {
                skipped += 1;
            }
        }
        store.log_operation(
            "restore",
            &game_id,
            Some(&plan.snapshot_id),
            "dry run finished, nothing written",
            "ok",
        );
        return Ok(RestoreOutcome {
            restored: 0,
            skipped,
            preserved: Vec::new(),
            safety_backup_id: None,
        });
    }

    let dest_root: PathBuf = match mode {
        RestoreMode::Export => export_dir
            .ok_or_else(|| "export mode needs a destination folder".to_string())?
            .to_path_buf(),
        _ => plan.target.clone(),
    };
    if mode != RestoreMode::Export {
        check_disk_space(&dest_root, plan.needed_bytes)?;
    }

    let staging = store
        .data_dir
        .join("staging")
        .join(format!("restore_{}", plan.snapshot_id));
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let mut staged: Vec<(SnapshotFile, PathBuf)> = Vec::new();
    for f in &plan.files {
        let data = read_object(store, &f.hash, f.compressed)?;
        let hash = blake3::hash(&data).to_hex().to_string();
        if hash != f.hash {
            let _ = std::fs::remove_dir_all(&staging);
            store.log_operation(
                "restore",
                &game_id,
                Some(&plan.snapshot_id),
                &format!("hash mismatch for {}", f.rel_path),
                "failed",
            );
            return Err(format!("stored object for {} failed hash verification, restore aborted before writing anything", f.rel_path));
        }
        let staged_path = staging.join(&f.rel_path);
        if let Some(parent) = staged_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&staged_path, &data).map_err(|e| e.to_string())?;
        staged.push((f.clone(), staged_path));
    }

    let mut safety_backup_id: Option<String> = None;
    if mode != RestoreMode::Export && dest_root.exists() {
        let safety = crate::backup::create_backup(
            store,
            &game_id,
            &dest_root,
            crate::backup::BackupOptions {
                name: Some(format!(
                    "Safety backup before restore {}",
                    crate::format_time(crate::now_unix())
                )),
                description: "Automatic safety backup taken before a restore".to_string(),
                compress: true,
                stability_wait_ms: 0,
            },
        );
        match safety {
            Ok(snap) => safety_backup_id = Some(snap.id),
            Err(e) => {
                store.log_operation(
                    "restore",
                    &game_id,
                    Some(&plan.snapshot_id),
                    &format!("safety backup failed: {}", e),
                    "warning",
                );
            }
        }
    }

    let mut restored = 0usize;
    let mut skipped = 0usize;
    let mut preserved: Vec<String> = Vec::new();
    let conflict_set: std::collections::HashSet<&str> =
        plan.conflicts.iter().map(|c| c.rel_path.as_str()).collect();
    for (f, staged_path) in &staged {
        let dest = dest_root.join(&f.rel_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if dest.exists() && conflict_set.contains(f.rel_path.as_str()) {
            match choice {
                ConflictChoice::Abort => {
                    let _ = std::fs::remove_dir_all(&staging);
                    store.log_operation(
                        "restore",
                        &game_id,
                        Some(&plan.snapshot_id),
                        "restore aborted by conflict choice",
                        "aborted",
                    );
                    return Err("restore aborted on conflict".to_string());
                }
                ConflictChoice::Skip | ConflictChoice::KeepCurrent => {
                    skipped += 1;
                    continue;
                }
                ConflictChoice::PreserveBoth => {
                    let backup_name = format!("{}.before-restore", dest.to_string_lossy());
                    std::fs::rename(&dest, &backup_name).map_err(|e| e.to_string())?;
                    preserved.push(backup_name);
                }
                ConflictChoice::Replace => {}
            }
        }
        if dest.exists() {
            let _ = std::fs::remove_file(&dest);
        }
        match std::fs::rename(staged_path, &dest) {
            Ok(()) => restored += 1,
            Err(_) => {
                std::fs::copy(staged_path, &dest).map_err(|e| e.to_string())?;
                let _ = std::fs::remove_file(staged_path);
                restored += 1;
            }
        }
    }
    let _ = std::fs::remove_dir_all(&staging);

    let mut ok = true;
    for f in &plan.files {
        if (choice == ConflictChoice::Skip || choice == ConflictChoice::KeepCurrent)
            && conflict_set.contains(f.rel_path.as_str())
        {
            continue;
        }
        let dest = dest_root.join(&f.rel_path);
        let data = match std::fs::read(&dest) {
            Ok(d) => d,
            Err(_) => {
                ok = false;
                break;
            }
        };
        if blake3::hash(&data).to_hex().to_string() != f.hash {
            ok = false;
            break;
        }
    }
    store.log_operation(
        "restore",
        &game_id,
        Some(&plan.snapshot_id),
        &format!(
            "restored {} files, skipped {}, destination verification {}",
            restored,
            skipped,
            if ok { "passed" } else { "FAILED" }
        ),
        if ok { "ok" } else { "failed" },
    );
    if !ok {
        return Err("destination verification failed after restore; safety backup is available to roll back".to_string());
    }
    Ok(RestoreOutcome {
        restored,
        skipped,
        preserved,
        safety_backup_id,
    })
}
