use crate::now_unix;
use crate::storage::{Snapshot, SnapshotFile, Store};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn object_bytes(data: &[u8], compress: bool) -> (Vec<u8>, bool) {
    if compress {
        match zstd::bulk::compress(data, 3) {
            Ok(c) if (c.len() as u64) < data.len() as u64 => (c, true),
            _ => (data.to_vec(), false),
        }
    } else {
        (data.to_vec(), false)
    }
}

pub fn store_object(
    store: &Arc<Store>,
    data: &[u8],
    compress: bool,
) -> Result<(String, u64, bool), String> {
    let hash = blake3::hash(data).to_hex().to_string();
    if store.object_exists(&hash)? {
        let path = store.object_path(&hash);
        let stored = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let compressed = stored < data.len() as u64;
        return Ok((hash, stored, compressed));
    }
    let (bytes, compressed) = object_bytes(data, compress);
    let path = store.object_path(&hash);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    store.register_object(&hash, data.len() as u64, bytes.len() as u64, compressed)?;
    Ok((hash, bytes.len() as u64, compressed))
}

pub fn read_object(store: &Arc<Store>, hash: &str, compressed: bool) -> Result<Vec<u8>, String> {
    let path = store.object_path(hash);
    let bytes = std::fs::read(&path).map_err(|e| format!("object {} unreadable: {}", hash, e))?;
    if compressed {
        zstd::bulk::decompress(&bytes, 512 * 1024 * 1024).map_err(|e| e.to_string())
    } else {
        Ok(bytes)
    }
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn collect_files(root: &Path, exclusions: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let walker = walkdir::WalkDir::new(root).follow_links(false).into_iter();
    for entry in walker.flatten() {
        let path = entry.path().to_path_buf();
        if entry.file_type().is_dir() {
            continue;
        }
        if is_symlink(&path) {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if exclusions
            .iter()
            .any(|pat| rel.contains(pat.as_str()) || path.to_string_lossy().contains(pat.as_str()))
        {
            continue;
        }
        out.push(path);
    }
    out.sort();
    out
}

pub fn file_stable(path: &Path, wait_ms: u64) -> bool {
    let first = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let size_first = std::fs::metadata(path).map(|m| m.len()).ok();
    std::thread::sleep(std::time::Duration::from_millis(wait_ms));
    let second = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let size_second = std::fs::metadata(path).map(|m| m.len()).ok();
    first == second && size_first == size_second
}

pub struct BackupOptions {
    pub name: Option<String>,
    pub description: String,
    pub compress: bool,
    pub stability_wait_ms: u64,
}

impl Default for BackupOptions {
    fn default() -> Self {
        Self {
            name: None,
            description: String::new(),
            compress: true,
            stability_wait_ms: 250,
        }
    }
}

pub fn create_backup(
    store: &Arc<Store>,
    game_id: &str,
    save_root: &Path,
    options: BackupOptions,
) -> Result<Snapshot, String> {
    let game = store
        .get_game(game_id)?
        .ok_or_else(|| format!("unknown game id: {}", game_id))?;
    if !save_root.exists() {
        return Err(format!(
            "save location does not exist: {}",
            save_root.display()
        ));
    }
    let exclusions: Vec<String> = Vec::new();
    let files = collect_files(save_root, &exclusions);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let snapshot_id = format!("snap_{}_{}", game_id, nanos);
    let mut snap_files: Vec<SnapshotFile> = Vec::new();
    let mut total_bytes: i64 = 0;
    let mut stored_bytes: i64 = 0;
    for path in &files {
        if !file_stable(path, options.stability_wait_ms) {
            store.log_operation(
                "backup",
                game_id,
                Some(&snapshot_id),
                &format!("skipped unstable file {}", path.display()),
                "warning",
            );
            continue;
        }
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                store.log_operation(
                    "backup",
                    game_id,
                    Some(&snapshot_id),
                    &format!("could not read {}: {}", path.display(), e),
                    "warning",
                );
                continue;
            }
        };
        let rel = path
            .strip_prefix(save_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());
        let (hash, stored, is_compressed) = store_object(store, &data, options.compress)?;
        total_bytes += data.len() as i64;
        stored_bytes += stored as i64;
        snap_files.push(SnapshotFile {
            rel_path: rel,
            hash,
            size: data.len() as u64,
            compressed: is_compressed,
            stored_bytes: stored,
        });
    }
    let snapshot = Snapshot {
        id: snapshot_id.clone(),
        game_id: game_id.to_string(),
        name: options
            .name
            .unwrap_or_else(|| format!("Backup {}", crate::format_time(now_unix()))),
        description: options.description,
        created_at: now_unix(),
        file_count: snap_files.len() as i64,
        total_bytes,
        stored_bytes,
        verified: false,
        favorite: false,
        tags: Vec::new(),
    };
    store.insert_snapshot(&snapshot, &snap_files)?;
    let manifest = serde_json::to_string_pretty(&snap_files).map_err(|e| e.to_string())?;
    std::fs::write(
        store
            .data_dir
            .join("manifests")
            .join(format!("{}.json", snapshot_id)),
        manifest,
    )
    .map_err(|e| e.to_string())?;
    let verified = verify_snapshot(store, &snapshot_id)?;
    store.mark_verified(&snapshot_id, verified)?;
    store.log_operation(
        "backup",
        game_id,
        Some(&snapshot_id),
        &format!(
            "{} files, {} stored",
            snap_files.len(),
            crate::format_bytes(stored_bytes.max(0) as u64)
        ),
        if verified { "ok" } else { "failed" },
    );
    let _ = game;
    let mut snapshot = snapshot;
    snapshot.verified = verified;
    Ok(snapshot)
}

pub fn verify_snapshot(store: &Arc<Store>, snapshot_id: &str) -> Result<bool, String> {
    let files = store.snapshot_files(snapshot_id)?;
    for f in &files {
        let data = read_object(store, &f.hash, f.compressed)?;
        if data.len() as u64 != f.size {
            return Ok(false);
        }
        let hash = blake3::hash(&data).to_hex().to_string();
        if hash != f.hash {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn diff_snapshots(store: &Arc<Store>, a: &str, b: &str) -> Result<Vec<String>, String> {
    let fa = store.snapshot_files(a)?;
    let fb = store.snapshot_files(b)?;
    let map_a: std::collections::HashMap<&str, &SnapshotFile> =
        fa.iter().map(|f| (f.rel_path.as_str(), f)).collect();
    let map_b: std::collections::HashMap<&str, &SnapshotFile> =
        fb.iter().map(|f| (f.rel_path.as_str(), f)).collect();
    let mut lines = Vec::new();
    for (path, f) in &map_a {
        match map_b.get(path) {
            None => lines.push(format!("removed: {} ({} bytes)", path, f.size)),
            Some(other) if other.hash != f.hash => {
                lines.push(format!(
                    "changed: {} ({} -> {} bytes)",
                    path, f.size, other.size
                ));
            }
            _ => {}
        }
    }
    for (path, f) in &map_b {
        if !map_a.contains_key(path) {
            lines.push(format!("added: {} ({} bytes)", path, f.size));
        }
    }
    lines.sort();
    Ok(lines)
}
