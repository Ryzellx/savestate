use crate::storage::Store;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub data_dir_display: String,
    pub compress_by_default: bool,
    pub stability_wait_ms: u64,
    pub retention_count: u32,
    pub reduced_motion: bool,
    pub notifications: bool,
    pub auto_scan_on_start: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            data_dir_display: String::new(),
            compress_by_default: true,
            stability_wait_ms: 250,
            retention_count: 20,
            reduced_motion: false,
            notifications: true,
            auto_scan_on_start: true,
        }
    }
}

impl Settings {
    pub fn load(store: &Store) -> Self {
        let mut settings = Self::default();
        if let Ok(Some(raw)) = store.get_setting("settings_json") {
            if let Ok(parsed) = serde_json::from_str::<Settings>(&raw) {
                settings = parsed;
            }
        }
        settings
    }

    pub fn save(&self, store: &Store) -> Result<(), String> {
        let raw = serde_json::to_string(self).map_err(|e| e.to_string())?;
        store
            .set_setting("settings_json", &raw)
            .map_err(|e| e.to_string())
    }
}
