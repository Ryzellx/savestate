use crate::backup::{create_backup, BackupOptions};
use crate::storage::Store;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub fn start_watcher(store: Arc<Store>, game_id: String, save_root: PathBuf) {
    std::thread::Builder::new()
        .name("save-watcher".to_string())
        .spawn(move || {
            let (tx, rx) = channel();
            let mut watcher: RecommendedWatcher =
                match RecommendedWatcher::new(tx, Config::default()) {
                    Ok(w) => w,
                    Err(e) => {
                        tracing::warn!("save watcher could not start: {}", e);
                        return;
                    }
                };
            if watcher.watch(&save_root, RecursiveMode::Recursive).is_err() {
                return;
            }
            let mut last_event = Instant::now();
            let mut pending = false;
            loop {
                match rx.recv_timeout(Duration::from_millis(500)) {
                    Ok(_) => {
                        pending = true;
                        last_event = Instant::now();
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if pending && last_event.elapsed() > Duration::from_secs(5) {
                            pending = false;
                            let result = create_backup(
                                &store,
                                &game_id,
                                &save_root,
                                BackupOptions {
                                    name: Some("Automatic backup on save change".to_string()),
                                    description:
                                        "Created by the on-change watcher after files settled"
                                            .to_string(),
                                    compress: true,
                                    stability_wait_ms: 250,
                                },
                            );
                            match result {
                                Ok(snap) => {
                                    tracing::info!("auto backup {} for {}", snap.id, game_id)
                                }
                                Err(e) => {
                                    tracing::warn!("auto backup failed for {}: {}", game_id, e)
                                }
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        })
        .ok();
}
