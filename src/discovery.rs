use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredGame {
    pub launcher: String,
    pub launcher_id: String,
    pub name: String,
    pub install_dir: Option<PathBuf>,
}

pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        if let Ok(pf) = std::env::var("ProgramFiles(x86)") {
            roots.push(PathBuf::from(pf).join("Steam"));
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            roots.push(PathBuf::from(pf).join("Steam"));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".steam").join("steam"));
        roots.push(home.join(".local").join("share").join("Steam"));
        roots.push(
            home.join(".var")
                .join("app")
                .join("com.valvesoftware.Steam")
                .join(".local")
                .join("share")
                .join("Steam"),
        );
        roots.push(home.join(".steam"));
    }
    roots.into_iter().filter(|p| p.exists()).collect()
}

fn extract_vdf_string(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let mut search_from = 0;
    while let Some(pos) = content[search_from..].find(&needle) {
        let after = &content[search_from + pos + needle.len()..];
        let after = after.trim_start();
        if let Some(rest) = after.strip_prefix('"') {
            let end = rest.find('"')?;
            return Some(rest[..end].to_string());
        }
        search_from += pos + needle.len();
    }
    None
}

fn parse_libraryfolders(content: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut search_from = 0;
    let needle = "\"path\"";
    while let Some(pos) = content[search_from..].find(needle) {
        let after = &content[search_from + pos + needle.len()..];
        let after = after.trim_start();
        if let Some(rest) = after.strip_prefix('"') {
            if let Some(end) = rest.find('"') {
                let raw = &rest[..end];
                let unescaped = raw.replace("\\\\", "\\");
                paths.push(PathBuf::from(unescaped));
            }
        }
        search_from += pos + needle.len();
    }
    paths
}

pub fn discover_steam() -> Vec<DiscoveredGame> {
    let mut games = Vec::new();
    for root in steam_roots() {
        let mut libraries = vec![root.clone()];
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(content) = std::fs::read_to_string(&vdf) {
            for p in parse_libraryfolders(&content) {
                libraries.push(p);
            }
        }
        for lib in libraries {
            let steamapps = lib.join("steamapps");
            let entries = match std::fs::read_dir(&steamapps) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
                    continue;
                }
                let content = match std::fs::read_to_string(entry.path()) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let appid = extract_vdf_string(&content, "appid").unwrap_or_default();
                let game_name = extract_vdf_string(&content, "name")
                    .unwrap_or_else(|| format!("Steam app {}", appid));
                let installdir = extract_vdf_string(&content, "installdir").unwrap_or_default();
                let install_dir = if installdir.is_empty() {
                    None
                } else {
                    Some(steamapps.join("common").join(installdir))
                };
                games.push(DiscoveredGame {
                    launcher: "Steam".to_string(),
                    launcher_id: appid,
                    name: game_name,
                    install_dir,
                });
            }
        }
    }
    games.sort_by(|a, b| a.name.cmp(&b.name));
    games.dedup_by(|a, b| a.launcher_id == b.launcher_id && a.launcher == b.launcher);
    games
}

#[derive(Debug, Deserialize)]
struct EpicManifest {
    #[serde(rename = "DisplayName")]
    display_name: Option<String>,
    #[serde(rename = "InstallLocation")]
    install_location: Option<String>,
    #[serde(rename = "AppName")]
    app_name: Option<String>,
}

pub fn epic_manifest_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(windows)]
    {
        if let Ok(pd) = std::env::var("ProgramData") {
            dirs.push(
                PathBuf::from(pd)
                    .join("Epic")
                    .join("EpicGamesLauncher")
                    .join("Data")
                    .join("Manifests"),
            );
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(
            PathBuf::from(home)
                .join(".config")
                .join("Epic")
                .join("Manifests"),
        );
    }
    dirs
}

pub fn discover_epic() -> Vec<DiscoveredGame> {
    let mut games = Vec::new();
    for dir in epic_manifest_dirs() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("item") {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let manifest: EpicManifest = match serde_json::from_str(&raw) {
                Ok(m) => m,
                Err(_) => continue,
            };
            games.push(DiscoveredGame {
                launcher: "Epic".to_string(),
                launcher_id: manifest.app_name.unwrap_or_default(),
                name: manifest
                    .display_name
                    .unwrap_or_else(|| "Unknown Epic game".to_string()),
                install_dir: manifest.install_location.map(PathBuf::from),
            });
        }
    }
    games.sort_by(|a, b| a.name.cmp(&b.name));
    games
}

pub fn discover_gog() -> Vec<DiscoveredGame> {
    let mut games = Vec::new();
    let mut db_paths: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        if let Ok(pd) = std::env::var("ProgramData") {
            db_paths.push(
                PathBuf::from(pd)
                    .join("GOG.com")
                    .join("Galaxy")
                    .join("storage")
                    .join("galaxy-2.0.db"),
            );
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        db_paths.push(
            PathBuf::from(home)
                .join(".config")
                .join("GOG.com")
                .join("Galaxy")
                .join("storage")
                .join("galaxy-2.0.db"),
        );
    }
    for db_path in db_paths {
        if !db_path.exists() {
            continue;
        }
        let conn = match rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut stmt = match conn.prepare(
            "SELECT productId, title FROM Products WHERE productId IN (SELECT productId FROM InstalledBaseProducts)",
        ) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let rows = match stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for row in rows.flatten() {
            games.push(DiscoveredGame {
                launcher: "GOG".to_string(),
                launcher_id: row.0.to_string(),
                name: row.1,
                install_dir: None,
            });
        }
    }
    games.sort_by(|a, b| a.name.cmp(&b.name));
    games
}

pub fn discover_all() -> Vec<DiscoveredGame> {
    let mut games = discover_steam();
    games.extend(discover_epic());
    games.extend(discover_gog());
    games.sort_by(|a, b| a.name.cmp(&b.name));
    games
}
