#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use clap::Parser;
use savestate::app::SaveStateApp;
use savestate::cli::{run, Cli};
use savestate::settings::Settings;
use savestate::storage::Store;
use std::sync::Arc;

fn main() {
    // Release GUI builds on Windows run without a console window. When the
    // user does pass a command, re-attach to the terminal that launched us
    // so CLI output is still visible.
    #[cfg(windows)]
    if std::env::args().len() > 1 {
        savestate::platform::attach_parent_console();
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    if cli.command.is_some() {
        match run(cli) {
            Ok(code) => std::process::exit(code),
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }
    let data_dir = cli
        .data_dir
        .clone()
        .unwrap_or_else(savestate::default_data_dir);
    let store = match Store::open(&data_dir) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!(
                "error: could not open SaveState data at {}: {}",
                data_dir.display(),
                e
            );
            std::process::exit(1);
        }
    };
    let settings = Settings::load(&store);
    if settings.auto_scan_on_start {
        let found = savestate::discovery::discover_all();
        for g in &found {
            let id = format!("{}:{}", g.launcher.to_lowercase(), g.launcher_id);
            let game = savestate::storage::Game {
                id,
                name: g.name.clone(),
                launcher: g.launcher.clone(),
                launcher_id: g.launcher_id.clone(),
                install_dir: g.install_dir.as_ref().map(|p| p.display().to_string()),
                discovered_at: savestate::now_unix(),
            };
            let _ = store.upsert_game(&game);
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_title("SaveState"),
        ..Default::default()
    };
    let result = eframe::run_native(
        "SaveState",
        options,
        Box::new(move |cc| Ok(Box::new(SaveStateApp::new(cc, store, settings)))),
    );
    if let Err(e) = result {
        eprintln!(
            "error: SaveState window could not start (no display available?): {}",
            e
        );
        std::process::exit(1);
    }
}
