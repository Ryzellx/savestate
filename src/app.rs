use crate::backup::{create_backup, diff_snapshots, verify_snapshot, BackupOptions};
use crate::discovery::discover_all;
use crate::profile::{load_profiles, profile_path, resolve_save_paths};
use crate::restore::{execute_restore, plan_restore, process_running, ConflictChoice, RestoreMode};
use crate::settings::Settings;
use crate::storage::{Game, Store};
use crate::ui::{self, ToastKind};
use crate::{format_bytes, format_time, now_unix};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Dashboard,
    Library,
    Details,
    Snapshots,
    Restore,
    Storage,
    Automation,
    Settings,
}

impl Page {
    fn label(&self) -> &'static str {
        match self {
            Page::Dashboard => "Dashboard",
            Page::Library => "Game Library",
            Page::Details => "Game Details",
            Page::Snapshots => "Snapshots",
            Page::Restore => "Restore Wizard",
            Page::Storage => "Storage",
            Page::Automation => "Automation",
            Page::Settings => "Settings",
        }
    }
}

const PAGES: [Page; 8] = [
    Page::Dashboard,
    Page::Library,
    Page::Details,
    Page::Snapshots,
    Page::Restore,
    Page::Storage,
    Page::Automation,
    Page::Settings,
];

pub struct SaveStateApp {
    store: Arc<Store>,
    settings: Settings,
    page: Page,
    sidebar_collapsed: bool,
    selected_game: Option<String>,
    selected_snapshot: Option<String>,
    restore_target: String,
    restore_dry_run: bool,
    conflict_choice: ConflictChoice,
    toast: Option<(String, ToastKind, std::time::Instant)>,
    scan_filter: String,
    new_game_id: String,
    new_game_name: String,
    new_save_path: String,
    wizard_step: usize,
    confirm_delete_snapshot: Option<String>,
    page_started: std::time::Instant,
}

impl SaveStateApp {
    pub fn new(cc: &eframe::CreationContext<'_>, store: Arc<Store>, settings: Settings) -> Self {
        ui::apply_theme(&cc.egui_ctx);
        Self {
            store,
            settings,
            page: Page::Dashboard,
            sidebar_collapsed: false,
            selected_game: None,
            selected_snapshot: None,
            restore_target: String::new(),
            restore_dry_run: false,
            conflict_choice: ConflictChoice::Replace,
            toast: None,
            scan_filter: String::new(),
            new_game_id: String::new(),
            new_game_name: String::new(),
            new_save_path: String::new(),
            wizard_step: 0,
            confirm_delete_snapshot: None,
            page_started: std::time::Instant::now(),
        }
    }

    fn set_toast(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toast = Some((text.into(), kind, std::time::Instant::now()));
    }

    /// Opacity of the current page's content: fades in over 180ms after a
    /// page switch, instant with reduced motion.
    fn page_fade(&self) -> f32 {
        crate::animation::page_fade(
            self.page_started.elapsed().as_secs_f32(),
            0.18,
            self.settings.reduced_motion,
        )
    }

    fn nav_icon(page: Page) -> &'static str {
        match page {
            Page::Dashboard => "▦",
            Page::Library => "▤",
            Page::Details => "ℹ",
            Page::Snapshots => "◈",
            Page::Restore => "↺",
            Page::Storage => "▣",
            Page::Automation => "◷",
            Page::Settings => "⚙",
        }
    }

    fn games(&self) -> Vec<Game> {
        self.store.list_games().unwrap_or_default()
    }

    fn do_scan(&mut self) {
        let found = discover_all();
        let mut added = 0;
        for g in &found {
            let id = format!("{}:{}", g.launcher.to_lowercase(), g.launcher_id);
            let game = Game {
                id,
                name: g.name.clone(),
                launcher: g.launcher.clone(),
                launcher_id: g.launcher_id.clone(),
                install_dir: g.install_dir.as_ref().map(|p| p.display().to_string()),
                discovered_at: now_unix(),
            };
            if self.store.upsert_game(&game).is_ok() {
                added += 1;
            }
        }
        let profiles = load_profiles(&profile_path(&self.store.data_dir)).unwrap_or_default();
        for profile in &profiles {
            let _ = self.store.store_profile(
                &profile.id,
                &profile.name,
                &serde_json::to_string(profile).unwrap_or_default(),
            );
        }
        self.set_toast(
            format!(
                "Scan complete: {} games found, {} profiles loaded",
                added,
                profiles.len()
            ),
            ToastKind::Ok,
        );
    }

    fn save_root_for(&self, game_id: &str) -> Option<PathBuf> {
        let locations = self.store.list_save_locations(game_id).unwrap_or_default();
        for loc in locations {
            let path = PathBuf::from(crate::profile::expand_env_vars(&loc.path));
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    fn do_backup(&mut self, game_id: &str) {
        let Some(root) = self.save_root_for(game_id) else {
            self.set_toast(
                "No existing save location known for this game. Add one in Game Details.",
                ToastKind::Error,
            );
            return;
        };
        match create_backup(&self.store, game_id, &root, BackupOptions::default()) {
            Ok(snap) => self.set_toast(
                format!(
                    "Backup {} complete: {} files, verified: {}",
                    snap.id, snap.file_count, snap.verified
                ),
                ToastKind::Ok,
            ),
            Err(e) => self.set_toast(e, ToastKind::Error),
        }
    }
}

impl eframe::App for SaveStateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let p = ui::palette();
        let reduced = self.settings.reduced_motion;
        let sidebar_target = if self.sidebar_collapsed { 60.0 } else { 216.0 };
        let sidebar_width =
            crate::animation::animate_to(ctx, "sidebar-width", sidebar_target, 0.22, reduced);
        let fade = self.page_fade();

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(sidebar_width)
            .show(ctx, |ui| {
                ui::paint_gradient_background(ui);
                ui.add_space(14.0);
                if self.sidebar_collapsed {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("SS")
                                .font(ui::heading_font(20.0))
                                .color(p.accent),
                        );
                    });
                } else {
                    ui.label(egui::RichText::new("SaveState").font(ui::heading_font(22.0)));
                    ui.label(
                        egui::RichText::new("Game saves, backed up and restorable.")
                            .small()
                            .color(p.text_dim),
                    );
                }
                ui.add_space(10.0);
                if ui
                    .button(if self.sidebar_collapsed {
                        "›"
                    } else {
                        "‹ Collapse"
                    })
                    .clicked()
                {
                    self.sidebar_collapsed = !self.sidebar_collapsed;
                }
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(6.0);
                for page in PAGES {
                    if ui::nav_button(
                        ui,
                        Self::nav_icon(page),
                        page.label(),
                        self.page == page,
                        self.sidebar_collapsed,
                    )
                    .clicked()
                    {
                        self.page = page;
                        self.page_started = std::time::Instant::now();
                    }
                    ui.add_space(2.0);
                }
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                if ui.button("Scan for games").clicked() {
                    self.do_scan();
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui::paint_gradient_background(ui);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("SaveState").font(ui::heading_font(16.0)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.monospace(format!("v{}", env!("CARGO_PKG_VERSION")));
                });
            });
            ui.add_space(4.0);
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.scope(|ui| {
                    ui.set_opacity(fade);
                    ui.add_space(8.0);
                    match self.page {
                        Page::Dashboard => self.show_dashboard(ui),
                        Page::Library => self.show_library(ui),
                        Page::Details => self.show_details(ui),
                        Page::Snapshots => self.show_snapshots(ui),
                        Page::Restore => self.show_restore(ui),
                        Page::Storage => self.show_storage(ui),
                        Page::Automation => self.show_automation(ui),
                        Page::Settings => self.show_settings(ui),
                    }
                });
            });
        });

        // Animated toast, bottom-right: slides in, fades out after 4s.
        if let Some((text, kind, since)) = &self.toast {
            let age = since.elapsed().as_secs_f32();
            if age > 4.0 {
                self.toast = None;
            } else {
                let alpha = if age < 0.2 {
                    crate::animation::ease_out_cubic(age / 0.2)
                } else if age > 3.5 {
                    1.0 - (age - 3.5) / 0.5
                } else {
                    1.0
                };
                let slide = if reduced {
                    0.0
                } else {
                    (1.0 - crate::animation::ease_out_cubic(age / 0.25)) * 20.0
                };
                let text = text.clone();
                let kind = *kind;
                egui::Area::new("toast".into())
                    .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0 + slide))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        ui.set_opacity(alpha.clamp(0.0, 1.0));
                        ui::toast(ui, &text, kind);
                    });
            }
        }

        // Repaint only while something is still moving.
        let sidebar_moving = (sidebar_width - sidebar_target).abs() > 0.5;
        let animating = sidebar_moving || fade < 1.0 || self.toast.is_some();
        if animating {
            ctx.request_repaint();
        }
    }
}

impl SaveStateApp {
    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Dashboard");
        let stats = self
            .store
            .storage_stats()
            .unwrap_or(crate::storage::StorageStats {
                object_count: 0,
                stored_bytes: 0,
                logical_bytes: 0,
                snapshot_count: 0,
                game_count: 0,
            });
        let games = self.games();
        let last_backup = self
            .store
            .list_all_snapshots()
            .ok()
            .and_then(|s| s.into_iter().next());
        ui.horizontal(|ui| {
            ui::stat_card(ui, "Games", &stats.game_count.to_string());
            ui::stat_card(ui, "Snapshots", &stats.snapshot_count.to_string());
            ui::stat_card(ui, "Storage used", &format_bytes(stats.stored_bytes));
            ui::stat_card(
                ui,
                "Dedup + compression saved",
                &format_bytes(stats.logical_bytes.saturating_sub(stats.stored_bytes)),
            );
        });
        ui.add_space(8.0);
        match last_backup {
            Some(s) => {
                ui.label(format!(
                    "Last backup: {} ({})",
                    s.name,
                    format_time(s.created_at)
                ));
            }
            None => {
                ui.label("Last backup: none yet. Scan for games, then back up a save.");
            }
        }
        ui.add_space(16.0);
        ui.label(egui::RichText::new("Your games").font(ui::heading_font(18.0)));
        ui.add_space(6.0);
        if games.is_empty() {
            ui.label("No games detected yet. Press Scan for games in the sidebar.");
        }
        let fade = self.page_fade();
        for (i, game) in games.iter().take(8).enumerate() {
            let snaps = self
                .store
                .list_snapshots(&game.id)
                .map(|s| s.len())
                .unwrap_or(0);
            let alpha =
                crate::animation::ease_out_cubic((fade * 1.4 - i as f32 * 0.08).clamp(0.0, 1.0));
            let p = ui::palette();
            ui.scope(|ui| {
                ui.set_opacity(alpha);
                ui::card_frame(&p).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&game.name).font(ui::heading_font(16.0)));
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} · {} snapshot(s)",
                                    game.launcher, snaps
                                ))
                                .small()
                                .color(p.text_dim),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let id = game.id.clone();
                            if ui::primary_button(ui, "Back up").clicked() {
                                self.do_backup(&id);
                            }
                        });
                    });
                });
            });
            ui.add_space(8.0);
        }
    }

    fn game_picker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.scan_filter);
        });
        let games = self.games();
        for game in games {
            if !self.scan_filter.is_empty()
                && !game
                    .name
                    .to_lowercase()
                    .contains(&self.scan_filter.to_lowercase())
            {
                continue;
            }
            let selected = self.selected_game.as_deref() == Some(game.id.as_str());
            if ui
                .selectable_label(selected, format!("{} [{}]", game.name, game.launcher))
                .clicked()
            {
                self.selected_game = Some(game.id.clone());
            }
        }
    }

    fn show_library(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Game Library");
        self.game_picker(ui);
        ui.add_space(12.0);
        ui.heading("Register a game manually");
        ui.label("Game id (lowercase, no spaces):");
        ui.text_edit_singleline(&mut self.new_game_id);
        ui.label("Display name:");
        ui.text_edit_singleline(&mut self.new_game_name);
        ui.label("Save folder path:");
        ui.text_edit_singleline(&mut self.new_save_path);
        if ui.button("Register game").clicked() {
            if self.new_game_id.trim().is_empty()
                || self.new_game_name.trim().is_empty()
                || self.new_save_path.trim().is_empty()
            {
                self.set_toast(
                    "Fill in the game id, name, and save path.",
                    ToastKind::Error,
                );
            } else {
                let game = Game {
                    id: self.new_game_id.trim().to_string(),
                    name: self.new_game_name.trim().to_string(),
                    launcher: "manual".to_string(),
                    launcher_id: self.new_game_id.trim().to_string(),
                    install_dir: None,
                    discovered_at: now_unix(),
                };
                match self.store.upsert_game(&game) {
                    Ok(()) => {
                        let _ =
                            self.store
                                .add_save_location(&game.id, self.new_save_path.trim(), 1.0);
                        self.set_toast(format!("Registered {}", game.name), ToastKind::Ok);
                        self.new_game_id.clear();
                        self.new_game_name.clear();
                        self.new_save_path.clear();
                    }
                    Err(e) => self.set_toast(e, ToastKind::Error),
                }
            }
        }
    }

    fn show_details(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Game Details");
        self.game_picker(ui);
        let Some(game_id) = self.selected_game.clone() else {
            ui.label("Select a game above to see its details.");
            return;
        };
        let Some(game) = self.store.get_game(&game_id).ok().flatten() else {
            ui.label("Selected game is no longer in the database.");
            return;
        };
        ui.add_space(8.0);
        ui.heading(&game.name);
        ui.label(format!(
            "Launcher: {} (id {})",
            game.launcher, game.launcher_id
        ));
        ui.label(format!(
            "Install dir: {}",
            game.install_dir.as_deref().unwrap_or("(unknown)")
        ));
        ui.add_space(8.0);
        ui.heading("Save locations");
        let locations = self.store.list_save_locations(&game_id).unwrap_or_default();
        if locations.is_empty() {
            ui.label("No save location known. SaveState can only back up a game once it knows where the saves live.");
            let profiles = load_profiles(&profile_path(&self.store.data_dir)).unwrap_or_default();
            for profile in profiles
                .iter()
                .filter(|p| p.launcher_ids.values().any(|v| v == &game.launcher_id))
            {
                for (path, confidence, confirmed) in resolve_save_paths(profile) {
                    ui.label(format!(
                        "Profile suggests: {} (confidence {:.0}%)",
                        path.display(),
                        confidence * 100.0
                    ));
                    let _ = confirmed;
                }
            }
        }
        for loc in &locations {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} (confidence {:.0}%, confirmed: {})",
                    loc.path,
                    loc.confidence * 100.0,
                    loc.confirmed
                ));
                if !loc.confirmed && ui.button("Confirm").clicked() {
                    let _ = self.store.confirm_save_location(loc.id);
                }
            });
        }
        ui.add_space(8.0);
        if ui.button("Back up now").clicked() {
            self.do_backup(&game_id);
        }
        ui.add_space(8.0);
        ui.heading("Snapshots");
        let snaps = self.store.list_snapshots(&game_id).unwrap_or_default();
        for snap in snaps.iter().take(10) {
            ui.label(format!(
                "{}  {}  {} files  verified: {}",
                snap.name,
                format_time(snap.created_at),
                snap.file_count,
                snap.verified
            ));
        }
        if snaps.is_empty() {
            ui.label("No snapshots yet.");
        }
    }

    fn show_snapshots(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Snapshot Manager");
        self.game_picker(ui);
        let Some(game_id) = self.selected_game.clone() else {
            ui.label("Select a game to list its snapshots.");
            return;
        };
        let snaps = self.store.list_snapshots(&game_id).unwrap_or_default();
        if snaps.is_empty() {
            ui.label("No snapshots for this game yet. Back it up from Game Details.");
            return;
        }
        let fade = self.page_fade();
        for (i, snap) in snaps.iter().enumerate() {
            let p = ui::palette();
            let alpha =
                crate::animation::ease_out_cubic((fade * 1.4 - i as f32 * 0.08).clamp(0.0, 1.0));
            ui.scope(|ui| {
                ui.set_opacity(alpha);
                ui::card_frame(&p).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.monospace(&snap.name);
                        ui.label(format_time(snap.created_at));
                        if snap.verified {
                            ui.colored_label(p.accent, "● Verified");
                        } else {
                            ui.colored_label(p.danger, "Not verified");
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!(
                                "{} files, {}",
                                snap.file_count,
                                format_bytes(snap.total_bytes.max(0) as u64)
                            ));
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(&snap.description);
                    ui.horizontal(|ui| {
                        let snap_id = snap.id.clone();
                        if ui.button("Verify").clicked() {
                            match verify_snapshot(&self.store, &snap_id) {
                                Ok(ok) => {
                                    let _ = self.store.mark_verified(&snap_id, ok);
                                    self.set_toast(
                                        format!(
                                            "Verification: {}",
                                            if ok { "passed" } else { "FAILED" }
                                        ),
                                        if ok { ToastKind::Ok } else { ToastKind::Error },
                                    );
                                }
                                Err(e) => self.set_toast(e, ToastKind::Error),
                            }
                        }
                        if ui::primary_button(ui, "Restore this").clicked() {
                            self.selected_snapshot = Some(snap_id.clone());
                            self.page = Page::Restore;
                            self.wizard_step = 0;
                        }
                        if ui.button("Export").clicked() {
                            let export_dir = self.store.data_dir.join("exports").join(&snap_id);
                            let _ = std::fs::create_dir_all(&export_dir);
                            let plan = plan_restore(&self.store, &snap_id, &export_dir, None);
                            match plan {
                                Ok(plan) => match execute_restore(
                                    &self.store,
                                    &plan,
                                    RestoreMode::Export,
                                    ConflictChoice::Replace,
                                    Some(&export_dir),
                                ) {
                                    Ok(out) => self.set_toast(
                                        format!(
                                            "Exported {} files to {}",
                                            out.restored,
                                            export_dir.display()
                                        ),
                                        ToastKind::Ok,
                                    ),
                                    Err(e) => self.set_toast(e, ToastKind::Error),
                                },
                                Err(e) => self.set_toast(e, ToastKind::Error),
                            }
                        }
                        if self.confirm_delete_snapshot.as_deref() == Some(snap_id.as_str()) {
                            if ui.button("Confirm delete").clicked() {
                                let _ = self.store.delete_snapshot(&snap_id);
                                self.confirm_delete_snapshot = None;
                                self.set_toast("Snapshot deleted.", ToastKind::Info);
                            }
                        } else if ui.button("Delete").clicked() {
                            self.confirm_delete_snapshot = Some(snap_id.clone());
                        }
                    });
                });
            });
            ui.add_space(8.0);
        }
        if snaps.len() >= 2 {
            ui.add_space(8.0);
            if ui.button("Compare newest two snapshots").clicked() {
                match diff_snapshots(&self.store, &snaps[0].id, &snaps[1].id) {
                    Ok(lines) => {
                        if lines.is_empty() {
                            self.set_toast(
                                "The two newest snapshots are identical.",
                                ToastKind::Info,
                            );
                        } else {
                            self.set_toast(lines.join(" | "), ToastKind::Info);
                        }
                    }
                    Err(e) => self.set_toast(e, ToastKind::Error),
                }
            }
        }
    }

    fn show_restore(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Restore Wizard");
        let steps = ["Choose snapshot", "Check target", "Preview and confirm"];
        let p = ui::palette();
        ui.horizontal(|ui| {
            for (i, step) in steps.iter().enumerate() {
                let done = i < self.wizard_step;
                let current = i == self.wizard_step;
                let marker = if done {
                    "✓".to_string()
                } else {
                    format!("{}", i + 1)
                };
                ui.colored_label(
                    if current || done {
                        p.accent
                    } else {
                        p.text_dim
                    },
                    marker,
                );
                ui.label(if current {
                    egui::RichText::new(*step).color(p.accent)
                } else {
                    egui::RichText::new(*step).color(p.text_dim)
                });
                if i + 1 < steps.len() {
                    ui.colored_label(p.text_dim, "─");
                }
            }
        });
        ui.add_space(12.0);
        match self.wizard_step {
            0 => {
                self.game_picker(ui);
                if let Some(game_id) = self.selected_game.clone() {
                    let snaps = self.store.list_snapshots(&game_id).unwrap_or_default();
                    for snap in snaps {
                        let selected = self.selected_snapshot.as_deref() == Some(snap.id.as_str());
                        if ui
                            .selectable_label(
                                selected,
                                format!("{}  {}", snap.name, format_time(snap.created_at)),
                            )
                            .clicked()
                        {
                            self.selected_snapshot = Some(snap.id.clone());
                        }
                    }
                }
                ui.add_space(8.0);
                if ui.button("Next: check target").clicked() {
                    if self.selected_snapshot.is_some() {
                        self.wizard_step = 1;
                    } else {
                        self.set_toast("Choose a snapshot first.", ToastKind::Error);
                    }
                }
            }
            1 => {
                let Some(snap_id) = self.selected_snapshot.clone() else {
                    ui.label("No snapshot selected.");
                    return;
                };
                let Some(game_id) = self.selected_game.clone() else {
                    ui.label("No game selected.");
                    return;
                };
                ui.label("Restore target folder (leave empty to use the known save location):");
                ui.text_edit_singleline(&mut self.restore_target);
                let target = if self.restore_target.trim().is_empty() {
                    self.save_root_for(&game_id).unwrap_or_default()
                } else {
                    PathBuf::from(self.restore_target.trim())
                };
                ui.label(format!("Target: {}", target.display()));
                let profiles =
                    load_profiles(&profile_path(&self.store.data_dir)).unwrap_or_default();
                let process_names: Vec<String> = profiles
                    .iter()
                    .filter(|p| p.id == game_id)
                    .flat_map(|p| p.process_names.clone())
                    .collect();
                if process_running(&process_names) {
                    ui.colored_label(
                        ui::palette().danger,
                        "The game appears to be running. Close it before restoring.",
                    );
                } else {
                    ui.label("Game process check: not running (or unknown process name).");
                }
                match plan_restore(&self.store, &snap_id, &target, None) {
                    Ok(plan) => {
                        ui.label(format!(
                            "{} file(s), {} to restore.",
                            plan.files.len(),
                            format_bytes(plan.needed_bytes)
                        ));
                        if plan.conflicts.is_empty() {
                            ui.label("No conflicts: no existing file differs from the snapshot.");
                        } else {
                            ui.colored_label(
                                ui::palette().warn,
                                format!(
                                    "{} conflict(s): existing files differ from the snapshot.",
                                    plan.conflicts.len()
                                ),
                            );
                            for c in &plan.conflicts {
                                ui.label(format!(
                                    "  {} (current {} bytes, snapshot {} bytes)",
                                    c.rel_path, c.current_size, c.snapshot_size
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        ui.colored_label(ui::palette().danger, e);
                    }
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        self.wizard_step = 0;
                    }
                    if ui.button("Next: preview and confirm").clicked() {
                        self.wizard_step = 2;
                    }
                });
            }
            _ => {
                let Some(snap_id) = self.selected_snapshot.clone() else {
                    ui.label("No snapshot selected.");
                    return;
                };
                let Some(game_id) = self.selected_game.clone() else {
                    ui.label("No game selected.");
                    return;
                };
                let target = if self.restore_target.trim().is_empty() {
                    self.save_root_for(&game_id).unwrap_or_default()
                } else {
                    PathBuf::from(self.restore_target.trim())
                };
                ui.label(format!("Snapshot: {}", snap_id));
                ui.label(format!("Target: {}", target.display()));
                ui.checkbox(
                    &mut self.restore_dry_run,
                    "Dry run (preview only, write nothing)",
                );
                ui.label("If a file already exists and differs, what should happen?");
                ui.horizontal(|ui| {
                    for (choice, label) in [
                        (ConflictChoice::Replace, "Replace"),
                        (ConflictChoice::KeepCurrent, "Keep current"),
                        (ConflictChoice::PreserveBoth, "Preserve both"),
                        (ConflictChoice::Skip, "Skip"),
                        (ConflictChoice::Abort, "Abort"),
                    ] {
                        if ui
                            .selectable_label(self.conflict_choice == choice, label)
                            .clicked()
                        {
                            self.conflict_choice = choice;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Back").clicked() {
                        self.wizard_step = 1;
                    }
                    if ui.button("Run restore").clicked() {
                        match plan_restore(&self.store, &snap_id, &target, None) {
                            Ok(plan) => {
                                let mode = if self.restore_dry_run {
                                    RestoreMode::DryRun
                                } else {
                                    RestoreMode::Full
                                };
                                match execute_restore(
                                    &self.store,
                                    &plan,
                                    mode,
                                    self.conflict_choice,
                                    None,
                                ) {
                                    Ok(out) => {
                                        if self.restore_dry_run {
                                            self.set_toast(
                                                format!(
                                                    "Dry run: {} file(s) would be restored",
                                                    plan.files.len()
                                                ),
                                                ToastKind::Info,
                                            );
                                        } else {
                                            self.set_toast(
                                                format!(
                                                    "Restore complete: {} restored, {} skipped",
                                                    out.restored, out.skipped
                                                ),
                                                ToastKind::Ok,
                                            );
                                        }
                                        self.wizard_step = 0;
                                    }
                                    Err(e) => self.set_toast(e, ToastKind::Error),
                                }
                            }
                            Err(e) => self.set_toast(e, ToastKind::Error),
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label("A safety backup of the current saves is taken automatically before any restore writes files.");
            }
        }
    }

    fn show_storage(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Storage Manager");
        let stats = self
            .store
            .storage_stats()
            .unwrap_or(crate::storage::StorageStats {
                object_count: 0,
                stored_bytes: 0,
                logical_bytes: 0,
                snapshot_count: 0,
                game_count: 0,
            });
        ui.label(format!("Data folder: {}", self.store.data_dir.display()));
        ui.label(format!("Stored objects: {}", stats.object_count));
        ui.label(format!(
            "Logical size: {}",
            format_bytes(stats.logical_bytes)
        ));
        ui.label(format!("Stored size: {}", format_bytes(stats.stored_bytes)));
        ui.label(format!(
            "Saved by dedup + compression: {}",
            format_bytes(stats.logical_bytes.saturating_sub(stats.stored_bytes))
        ));
        ui.label(format!("Snapshots: {}", stats.snapshot_count));
        ui.add_space(12.0);
        if ui.button("Run garbage collection (dry run)").clicked() {
            match self.store.gc(true) {
                Ok((count, bytes)) => self.set_toast(
                    format!(
                        "GC dry run: {} unreferenced object(s), {} would be freed",
                        count,
                        format_bytes(bytes)
                    ),
                    ToastKind::Info,
                ),
                Err(e) => self.set_toast(e, ToastKind::Error),
            }
        }
        if ui
            .button("Run garbage collection (delete unreferenced objects)")
            .clicked()
        {
            match self.store.gc(false) {
                Ok((count, bytes)) => self.set_toast(
                    format!(
                        "GC removed {} object(s), freed {}",
                        count,
                        format_bytes(bytes)
                    ),
                    ToastKind::Ok,
                ),
                Err(e) => self.set_toast(e, ToastKind::Error),
            }
        }
        ui.add_space(8.0);
        ui.label(
            "Garbage collection never deletes an object still referenced by a retained snapshot.",
        );
    }

    fn show_automation(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Automation");
        self.game_picker(ui);
        let Some(game_id) = self.selected_game.clone() else {
            ui.label("Select a game to edit its backup policy.");
            return;
        };
        let policy = self.store.get_policy(&game_id).ok().flatten();
        let (mut mode, mut interval, mut on_launch, mut on_exit, mut on_change, mut retention) =
            policy.unwrap_or_else(|| ("manual".to_string(), 0, false, false, false, 20));
        ui.label("Mode:");
        ui.text_edit_singleline(&mut mode);
        ui.horizontal(|ui| {
            ui.label("Interval (minutes, 0 = off):");
            ui.add(egui::DragValue::new(&mut interval).range(0..=10080));
        });
        ui.checkbox(&mut on_launch, "Back up before launch");
        ui.checkbox(&mut on_exit, "Back up after exit");
        ui.checkbox(&mut on_change, "Back up when saves change (watcher)");
        ui.horizontal(|ui| {
            ui.label("Keep at most (snapshots):");
            ui.add(egui::DragValue::new(&mut retention).range(1..=200));
        });
        if ui.button("Save policy").clicked() {
            let update = crate::storage::PolicyUpdate {
                mode,
                interval_minutes: interval,
                on_launch,
                on_exit,
                on_change,
                retention,
            };
            match self.store.set_policy(&game_id, update) {
                Ok(()) => self.set_toast("Policy saved.", ToastKind::Ok),
                Err(e) => self.set_toast(e, ToastKind::Error),
            }
        }
        ui.add_space(12.0);
        ui.heading("Recent operations");
        for (kind, status, detail, at) in self.store.list_operations(20).unwrap_or_default() {
            ui.label(format!(
                "{}  {}  {}  {}",
                format_time(at),
                kind,
                status,
                detail
            ));
        }
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Settings");
        ui.checkbox(
            &mut self.settings.compress_by_default,
            "Compress stored objects with Zstandard",
        );
        ui.checkbox(&mut self.settings.reduced_motion, "Reduce motion");
        ui.checkbox(&mut self.settings.notifications, "Desktop notifications");
        ui.checkbox(
            &mut self.settings.auto_scan_on_start,
            "Scan for games on start",
        );
        ui.horizontal(|ui| {
            ui.label("File stability wait (ms):");
            ui.add(egui::DragValue::new(&mut self.settings.stability_wait_ms).range(0..=5000));
        });
        ui.horizontal(|ui| {
            ui.label("Default snapshot retention:");
            ui.add(egui::DragValue::new(&mut self.settings.retention_count).range(1..=200));
        });
        if ui.button("Save settings").clicked() {
            match self.settings.save(&self.store) {
                Ok(()) => self.set_toast("Settings saved.", ToastKind::Ok),
                Err(e) => self.set_toast(e, ToastKind::Error),
            }
        }
        ui.add_space(12.0);
        ui.label(format!("Data folder: {}", self.store.data_dir.display()));
        ui.label(format!(
            "Profiles folder: {}",
            profile_path(&self.store.data_dir).display()
        ));
        if ui.button("Check database integrity").clicked() {
            match self.store.integrity_check() {
                Ok(result) => self.set_toast(format!("Integrity check: {}", result), ToastKind::Ok),
                Err(e) => self.set_toast(e, ToastKind::Error),
            }
        }
        ui.add_space(8.0);
        ui.heading("Known limitations");
        for limit in crate::platform::known_limitations() {
            ui.label(format!("- {}", limit));
        }
    }
}
