pub mod animation;
pub mod app;
pub mod backup;
pub mod cli;
pub mod discovery;
pub mod platform;
pub mod profile;
pub mod restore;
pub mod settings;
pub mod storage;
pub mod ui;
pub mod watcher;

use std::path::PathBuf;

pub fn default_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SAVESTATE_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("SaveState");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("savestate-data")
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn format_time(unix: i64) -> String {
    chrono::DateTime::from_timestamp(unix, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "unknown time".to_string())
}

pub fn format_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = n as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", n, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

const BUNDLED_PROFILES: [(&str, &str); 3] = [
    (
        "stardew-valley.json",
        include_str!("../profiles/stardew-valley.json"),
    ),
    (
        "minecraft-java.json",
        include_str!("../profiles/minecraft-java.json"),
    ),
    ("terraria.json", include_str!("../profiles/terraria.json")),
];

pub fn ensure_bundled_profiles(data_dir: &std::path::Path) -> usize {
    let dir = data_dir.join("profiles");
    if std::fs::create_dir_all(&dir).is_err() {
        return 0;
    }
    let mut written = 0;
    for (name, content) in BUNDLED_PROFILES {
        let path = dir.join(name);
        if !path.exists() && std::fs::write(&path, content).is_ok() {
            written += 1;
        }
    }
    written
}
