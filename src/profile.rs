use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveLocation {
    pub path: String,
    pub kind: String,
    pub confidence: f32,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub platforms: Vec<String>,
    pub launcher_ids: HashMap<String, String>,
    pub process_names: Vec<String>,
    pub install_hints: Vec<String>,
    pub save_locations: Vec<SaveLocation>,
    pub exclusions: Vec<String>,
    pub confidence: f32,
}

impl GameProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("profile id is empty".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("profile name is empty".to_string());
        }
        if !(0.0..=1.0).contains(&self.confidence) {
            return Err("profile confidence must be between 0 and 1".to_string());
        }
        for loc in &self.save_locations {
            if loc.path.trim().is_empty() {
                return Err("save location path is empty".to_string());
            }
            if !(0.0..=1.0).contains(&loc.confidence) {
                return Err("save location confidence must be between 0 and 1".to_string());
            }
        }
        Ok(())
    }
}

pub fn expand_env_vars(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let mut name = String::new();
            let mut closed = false;
            for c2 in chars.by_ref() {
                if c2 == '%' {
                    closed = true;
                    break;
                }
                name.push(c2);
            }
            if closed {
                if let Ok(v) = std::env::var(&name) {
                    out.push_str(&v);
                } else {
                    out.push('%');
                    out.push_str(&name);
                    out.push('%');
                }
            } else {
                out.push('%');
                out.push_str(&name);
            }
        } else if c == '$' {
            let mut name = String::new();
            while let Some(&c2) = chars.peek() {
                if c2.is_alphanumeric() || c2 == '_' {
                    name.push(c2);
                    chars.next();
                } else {
                    break;
                }
            }
            if name.is_empty() {
                out.push('$');
            } else if name == "HOME" || name == "USERPROFILE" {
                out.push_str(&std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
            } else if let Ok(v) = std::env::var(&name) {
                out.push_str(&v);
            } else {
                out.push('$');
                out.push_str(&name);
            }
        } else if c == '~' && out.is_empty() {
            out.push_str(&std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
        } else {
            out.push(c);
        }
    }
    out
}

pub fn resolve_save_paths(profile: &GameProfile) -> Vec<(PathBuf, f32, bool)> {
    profile
        .save_locations
        .iter()
        .map(|loc| {
            let expanded = expand_env_vars(&loc.path);
            (PathBuf::from(expanded), loc.confidence, loc.confirmed)
        })
        .collect()
}

pub fn load_profiles(dir: &Path) -> Result<Vec<GameProfile>, String> {
    let mut profiles = Vec::new();
    if !dir.exists() {
        return Ok(profiles);
    }
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
        let profile: GameProfile =
            serde_json::from_str(&raw).map_err(|e| format!("{}: {}", path.display(), e))?;
        profile
            .validate()
            .map_err(|e| format!("{}: {}", path.display(), e))?;
        profiles.push(profile);
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

pub fn profile_path(data_dir: &Path) -> PathBuf {
    data_dir.join("profiles")
}
