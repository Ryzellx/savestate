use crate::now_unix;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub launcher: String,
    pub launcher_id: String,
    pub install_dir: Option<String>,
    pub discovered_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveLocationRow {
    pub id: i64,
    pub game_id: String,
    pub path: String,
    pub confidence: f32,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub description: String,
    pub created_at: i64,
    pub file_count: i64,
    pub total_bytes: i64,
    pub stored_bytes: i64,
    pub verified: bool,
    pub favorite: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub rel_path: String,
    pub hash: String,
    pub size: u64,
    pub compressed: bool,
    pub stored_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub object_count: i64,
    pub stored_bytes: u64,
    pub logical_bytes: u64,
    pub snapshot_count: i64,
    pub game_count: i64,
}

pub type Policy = (String, i64, bool, bool, bool, i64);

pub struct PolicyUpdate {
    pub mode: String,
    pub interval_minutes: i64,
    pub on_launch: bool,
    pub on_exit: bool,
    pub on_change: bool,
    pub retention: i64,
}

pub struct Store {
    conn: Mutex<Connection>,
    pub data_dir: PathBuf,
}

impl Store {
    pub fn open(data_dir: &Path) -> Result<Self, String> {
        for sub in ["database", "objects", "manifests", "staging", "logs"] {
            std::fs::create_dir_all(data_dir.join(sub)).map_err(|e| e.to_string())?;
        }
        std::fs::create_dir_all(data_dir.join("profiles")).map_err(|e| e.to_string())?;
        let db_path = data_dir.join("database").join("savestate.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(|e| e.to_string())?;
        let store = Self {
            conn: Mutex::new(conn),
            data_dir: data_dir.to_path_buf(),
        };
        store.migrate()?;
        crate::ensure_bundled_profiles(data_dir);
        Ok(store)
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS games (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               launcher TEXT NOT NULL DEFAULT '',
               launcher_id TEXT NOT NULL DEFAULT '',
               install_dir TEXT,
               discovered_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS game_installations (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
               launcher TEXT NOT NULL,
               install_dir TEXT NOT NULL,
               detected_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS save_locations (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
               path TEXT NOT NULL,
               confidence REAL NOT NULL DEFAULT 0.5,
               confirmed INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS game_profiles (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               profile_json TEXT NOT NULL,
               updated_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS snapshots (
               id TEXT PRIMARY KEY,
               game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
               name TEXT NOT NULL,
               description TEXT NOT NULL DEFAULT '',
               created_at INTEGER NOT NULL,
               file_count INTEGER NOT NULL DEFAULT 0,
               total_bytes INTEGER NOT NULL DEFAULT 0,
               stored_bytes INTEGER NOT NULL DEFAULT 0,
               verified INTEGER NOT NULL DEFAULT 0,
               favorite INTEGER NOT NULL DEFAULT 0,
               tags TEXT NOT NULL DEFAULT '[]'
             );
             CREATE TABLE IF NOT EXISTS snapshot_files (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               snapshot_id TEXT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
               rel_path TEXT NOT NULL,
               hash TEXT NOT NULL,
               size INTEGER NOT NULL,
               compressed INTEGER NOT NULL DEFAULT 0,
               stored_bytes INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_snapshot_files_snapshot ON snapshot_files(snapshot_id);
             CREATE INDEX IF NOT EXISTS idx_snapshot_files_hash ON snapshot_files(hash);
             CREATE TABLE IF NOT EXISTS objects (
               hash TEXT PRIMARY KEY,
               size INTEGER NOT NULL,
               stored_bytes INTEGER NOT NULL,
               compressed INTEGER NOT NULL DEFAULT 0,
               created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS operations (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               kind TEXT NOT NULL,
               game_id TEXT NOT NULL,
               snapshot_id TEXT,
               detail TEXT NOT NULL DEFAULT '',
               status TEXT NOT NULL,
               created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_operations_game ON operations(game_id);
             CREATE TABLE IF NOT EXISTS backup_policies (
               game_id TEXT PRIMARY KEY REFERENCES games(id) ON DELETE CASCADE,
               mode TEXT NOT NULL DEFAULT 'manual',
               interval_minutes INTEGER NOT NULL DEFAULT 0,
               on_launch INTEGER NOT NULL DEFAULT 0,
               on_exit INTEGER NOT NULL DEFAULT 0,
               on_change INTEGER NOT NULL DEFAULT 0,
               retention_count INTEGER NOT NULL DEFAULT 20,
               compress INTEGER NOT NULL DEFAULT 1
             );
             CREATE TABLE IF NOT EXISTS exclusions (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               game_id TEXT NOT NULL,
               pattern TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT OR IGNORE INTO schema_metadata(key, value) VALUES('version', '1');",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn integrity_check(&self) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn log_operation(
        &self,
        kind: &str,
        game_id: &str,
        snapshot_id: Option<&str>,
        detail: &str,
        status: &str,
    ) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = conn.execute(
            "INSERT INTO operations(kind, game_id, snapshot_id, detail, status, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![kind, game_id, snapshot_id, detail, status, now_unix()],
        );
    }

    pub fn upsert_game(&self, game: &Game) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO games(id, name, launcher, launcher_id, install_dir, discovered_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, launcher = excluded.launcher, launcher_id = excluded.launcher_id, install_dir = excluded.install_dir",
            params![game.id, game.name, game.launcher, game.launcher_id, game.install_dir, game.discovered_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_games(&self) -> Result<Vec<Game>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, name, launcher, launcher_id, install_dir, discovered_at FROM games ORDER BY name").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Game {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    launcher: r.get(2)?,
                    launcher_id: r.get(3)?,
                    install_dir: r.get(4)?,
                    discovered_at: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn get_game(&self, id: &str) -> Result<Option<Game>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT id, name, launcher, launcher_id, install_dir, discovered_at FROM games WHERE id = ?1",
            params![id],
            |r| Ok(Game { id: r.get(0)?, name: r.get(1)?, launcher: r.get(2)?, launcher_id: r.get(3)?, install_dir: r.get(4)?, discovered_at: r.get(5)? }),
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn add_save_location(
        &self,
        game_id: &str,
        path: &str,
        confidence: f32,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO save_locations(game_id, path, confidence, confirmed) VALUES(?1, ?2, ?3, 0)",
            params![game_id, path, confidence],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_save_locations(&self, game_id: &str) -> Result<Vec<SaveLocationRow>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, game_id, path, confidence, confirmed FROM save_locations WHERE game_id = ?1 ORDER BY confidence DESC").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![game_id], |r| {
                Ok(SaveLocationRow {
                    id: r.get(0)?,
                    game_id: r.get(1)?,
                    path: r.get(2)?,
                    confidence: r.get(3)?,
                    confirmed: r.get::<_, i64>(4)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn confirm_save_location(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE save_locations SET confirmed = 1 WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn store_profile(&self, id: &str, name: &str, json: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO game_profiles(id, name, profile_json, updated_at) VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, profile_json = excluded.profile_json, updated_at = excluded.updated_at",
            params![id, name, json, now_unix()],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_snapshot(&self, snap: &Snapshot, files: &[SnapshotFile]) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO snapshots(id, game_id, name, description, created_at, file_count, total_bytes, stored_bytes, verified, favorite, tags) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![snap.id, snap.game_id, snap.name, snap.description, snap.created_at, snap.file_count, snap.total_bytes, snap.stored_bytes, snap.verified as i32, snap.favorite as i32, serde_json::to_string(&snap.tags).unwrap_or_else(|_| "[]".to_string())],
        )
        .map_err(|e| e.to_string())?;
        for f in files {
            tx.execute(
                "INSERT INTO snapshot_files(snapshot_id, rel_path, hash, size, compressed, stored_bytes) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![snap.id, f.rel_path, f.hash, f.size as i64, f.compressed as i32, f.stored_bytes as i64],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_snapshots(&self, game_id: &str) -> Result<Vec<Snapshot>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, game_id, name, description, created_at, file_count, total_bytes, stored_bytes, verified, favorite, tags FROM snapshots WHERE game_id = ?1 ORDER BY created_at DESC").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![game_id], row_to_snapshot)
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn list_all_snapshots(&self) -> Result<Vec<Snapshot>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, game_id, name, description, created_at, file_count, total_bytes, stored_bytes, verified, favorite, tags FROM snapshots ORDER BY created_at DESC").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], row_to_snapshot)
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn get_snapshot(&self, id: &str) -> Result<Option<Snapshot>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT id, game_id, name, description, created_at, file_count, total_bytes, stored_bytes, verified, favorite, tags FROM snapshots WHERE id = ?1",
            params![id],
            row_to_snapshot,
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn snapshot_files(&self, snapshot_id: &str) -> Result<Vec<SnapshotFile>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT rel_path, hash, size, compressed, stored_bytes FROM snapshot_files WHERE snapshot_id = ?1 ORDER BY rel_path").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![snapshot_id], |r| {
                Ok(SnapshotFile {
                    rel_path: r.get(0)?,
                    hash: r.get(1)?,
                    size: r.get::<_, i64>(2)? as u64,
                    compressed: r.get::<_, i64>(3)? != 0,
                    stored_bytes: r.get::<_, i64>(4)? as u64,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn mark_verified(&self, snapshot_id: &str, verified: bool) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE snapshots SET verified = ?1 WHERE id = ?2",
            params![verified as i32, snapshot_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM snapshots WHERE id = ?1", params![snapshot_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn latest_snapshot(&self, game_id: &str) -> Result<Option<Snapshot>, String> {
        Ok(self.list_snapshots(game_id)?.into_iter().next())
    }

    pub fn register_object(
        &self,
        hash: &str,
        size: u64,
        stored_bytes: u64,
        compressed: bool,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO objects(hash, size, stored_bytes, compressed, created_at) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![hash, size as i64, stored_bytes as i64, compressed as i32, now_unix()],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn object_exists(&self, hash: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM objects WHERE hash = ?1",
                params![hash],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub fn storage_stats(&self) -> Result<StorageStats, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let object_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM objects", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let stored_bytes: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(stored_bytes), 0) FROM objects",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let logical_bytes: i64 = conn
            .query_row("SELECT COALESCE(SUM(size), 0) FROM objects", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        let snapshot_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let game_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM games", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(StorageStats {
            object_count,
            stored_bytes: stored_bytes.max(0) as u64,
            logical_bytes: logical_bytes.max(0) as u64,
            snapshot_count,
            game_count,
        })
    }

    pub fn gc(&self, dry_run: bool) -> Result<(u64, u64), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT hash, stored_bytes FROM objects WHERE hash NOT IN (SELECT DISTINCT hash FROM snapshot_files)").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map_err(|e| e.to_string())?;
        let mut orphaned: Vec<(String, i64)> = Vec::new();
        for r in rows {
            orphaned.push(r.map_err(|e| e.to_string())?);
        }
        drop(stmt);
        let count = orphaned.len() as u64;
        let bytes: u64 = orphaned.iter().map(|(_, b)| (*b).max(0) as u64).sum();
        if !dry_run {
            for (hash, _) in &orphaned {
                conn.execute("DELETE FROM objects WHERE hash = ?1", params![hash])
                    .map_err(|e| e.to_string())?;
                let path = self.object_path(hash);
                let _ = std::fs::remove_file(path);
            }
        }
        Ok((count, bytes))
    }

    pub fn object_path(&self, hash: &str) -> PathBuf {
        let prefix = &hash[..2.min(hash.len())];
        self.data_dir.join("objects").join(prefix).join(hash)
    }

    pub fn list_operations(
        &self,
        limit: u32,
    ) -> Result<Vec<(String, String, String, i64)>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT kind, status, detail, created_at FROM operations ORDER BY created_at DESC LIMIT ?1").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn get_policy(&self, game_id: &str) -> Result<Option<Policy>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT mode, interval_minutes, on_launch, on_exit, on_change, retention_count FROM backup_policies WHERE game_id = ?1",
            params![game_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get::<_, i64>(3)? != 0, r.get::<_, i64>(4)? != 0, r.get(5)?)),
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn set_policy(&self, game_id: &str, policy: PolicyUpdate) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO backup_policies(game_id, mode, interval_minutes, on_launch, on_exit, on_change, retention_count, compress) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)
             ON CONFLICT(game_id) DO UPDATE SET mode = excluded.mode, interval_minutes = excluded.interval_minutes, on_launch = excluded.on_launch, on_exit = excluded.on_exit, on_change = excluded.on_change, retention_count = excluded.retention_count",
            params![game_id, policy.mode, policy.interval_minutes, policy.on_launch as i32, policy.on_exit as i32, policy.on_change as i32, policy.retention],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn row_to_snapshot(r: &rusqlite::Row) -> rusqlite::Result<Snapshot> {
    let tags_raw: String = r.get(10)?;
    let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
    Ok(Snapshot {
        id: r.get(0)?,
        game_id: r.get(1)?,
        name: r.get(2)?,
        description: r.get(3)?,
        created_at: r.get(4)?,
        file_count: r.get(5)?,
        total_bytes: r.get(6)?,
        stored_bytes: r.get(7)?,
        verified: r.get::<_, i64>(8)? != 0,
        favorite: r.get::<_, i64>(9)? != 0,
        tags,
    })
}
