use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::button_bar::{show_button_bar, ButtonBarConfig};
use crate::disk_info;
use crate::file_ops;
use crate::panel::{show_panel, PanelState};
use crate::popups::{show_modal, ConfirmDeleteState, DriveSlotEdit, Modal, ModalAction, RenameState};
use crate::status_bar::show_status_bar;
use crate::theme;

/// Espacée d'une seconde : suffisante pour une horloge lisible sans relancer
/// `df`/`date` à chaque frame.
const STATUS_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

pub struct DirectoryOctopusApp {
    left: PanelState,
    right: PanelState,
    buttons: ButtonBarConfig,
    zoom: f32,
    /// Dernier niveau de zoom pour lequel la taille minimale de fenêtre a été
    /// envoyée au gestionnaire de fenêtres ; `0.0` force l'envoi initial.
    min_size_synced_for: f32,
    /// Pop-up actuellement affiché (édition de raccourci, renommage,
    /// confirmation de suppression), le cas échéant.
    modal: Option<Modal>,
    /// "DISK: 12.3G  FREE: 4.5G" du système de fichiers du panneau actif.
    disk_status_line: String,
    /// Date/heure courante, au format de l'utilitaire `date`.
    clock_line: String,
    last_status_refresh: Instant,
    /// Chemin du panneau actif au dernier calcul de `disk_status_line` :
    /// permet de détecter un changement (nouveau support de stockage choisi)
    /// et de rafraîchir immédiatement plutôt que d'attendre la minuterie.
    last_status_path: String,
}

impl Default for DirectoryOctopusApp {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_owned());

        let mut left = PanelState::at_path("left", PathBuf::from(&home));
        left.active = true;

        let start_right = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(&home));
        let right = PanelState::at_path("right", start_right);

        let mut app = Self {
            left,
            right,
            buttons: ButtonBarConfig::load_or_default(),
            zoom: theme::DEFAULT_ZOOM,
            min_size_synced_for: 0.0,
            modal: None,
            disk_status_line: String::new(),
            clock_line: String::new(),
            // Dans le passé pour forcer un premier calcul dès la première frame.
            last_status_refresh: Instant::now() - STATUS_REFRESH_INTERVAL,
            last_status_path: String::new(),
        };
        app.refresh_status_bar();
        app
    }
}

impl DirectoryOctopusApp {
    /// Le panneau actuellement actif (celui dont le bandeau titre est en rouge),
    /// destinataire des raccourcis de navigation et futures actions de bouton.
    fn active_panel_mut(&mut self) -> &mut PanelState {
        if self.left.active { &mut self.left } else { &mut self.right }
    }

    fn active_panel(&self) -> &PanelState {
        if self.left.active { &self.left } else { &self.right }
    }

    fn panel_mut(&mut self, for_left: bool) -> &mut PanelState {
        if for_left { &mut self.left } else { &mut self.right }
    }

    /// Le panneau actif (source) et l'autre (destination), pour Copy/Move.
    fn source_and_dest_mut(&mut self) -> (&mut PanelState, &mut PanelState) {
        if self.left.active { (&mut self.left, &mut self.right) } else { (&mut self.right, &mut self.left) }
    }

    /// Copie ou déplace les éléments sélectionnés du panneau actif vers
    /// l'autre panneau, puis relit le contenu des dossiers concernés.
    fn copy_or_move(&mut self, is_move: bool) {
        let (source, dest) = self.source_and_dest_mut();
        let src_dir = PathBuf::from(&source.path);
        let dest_dir = PathBuf::from(&dest.path);
        if src_dir == dest_dir {
            return;
        }
        for entry in source.selected_entries() {
            let src_path = src_dir.join(&entry.name);
            let result =
                if is_move { file_ops::move_into(&src_path, &dest_dir) } else { file_ops::copy_into(&src_path, &dest_dir) };
            if let Err(err) = result {
                eprintln!("{} de {src_path:?} échoué : {err}", if is_move { "Déplacement" } else { "Copie" });
            }
        }
        dest.navigate_to(dest_dir);
        if is_move {
            source.navigate_to(src_dir);
        }
    }

    /// Recalcule l'espace disque (du panneau actif) et l'horloge affichés
    /// dans la barre de statut.
    fn refresh_status_bar(&mut self) {
        self.disk_status_line = match disk_info::disk_space(Path::new(&self.active_panel().path)) {
            Some((total, free)) => {
                format!("DISK: {}  FREE: {}", disk_info::human_size(total), disk_info::human_size(free))
            }
            None => "DISK: --  FREE: --".to_owned(),
        };
        self.clock_line = disk_info::current_date_time();
        self.last_status_refresh = Instant::now();
        self.last_status_path = self.active_panel().path.clone();
    }

    /// À appeler à chaque frame : rafraîchit la barre de statut immédiatement
    /// si le panneau actif a changé de répertoire (nouveau support de
    /// stockage choisi), sinon au plus toutes les `STATUS_REFRESH_INTERVAL`
    /// (pour que l'horloge et l'espace libre restant continuent d'avancer).
    fn maybe_refresh_status_bar(&mut self) {
        if self.active_panel().path != self.last_status_path
            || self.last_status_refresh.elapsed() >= STATUS_REFRESH_INTERVAL
        {
            self.refresh_status_bar();
        }
    }

    fn handle_action(&mut self, action: &str) {
        if let Some(target) = action.strip_prefix("goto:") {
            self.active_panel_mut().navigate_to(PathBuf::from(target));
            return;
        }
        if let Some(row_idx) = action.strip_prefix("edit_drive:").and_then(|s| s.parse::<usize>().ok()) {
            if let Some(slot) = self.buttons.drive_slots.get(row_idx) {
                self.modal = Some(Modal::EditDriveSlot(DriveSlotEdit::new(row_idx, &slot.label, slot.path.as_ref())));
            }
            return;
        }
        match action {
            "select_all" => self.active_panel_mut().select_all(),
            "select_none" => self.active_panel_mut().select_none(),
            "parent" => self.active_panel_mut().navigate_to_parent(),
            "root" => {
                let panel = self.active_panel_mut();
                if let Some(root) = disk_info::device_root(Path::new(&panel.path)) {
                    panel.navigate_to(root);
                }
            }
            "copy" => self.copy_or_move(false),
            "move" => self.copy_or_move(true),
            "rename" => {
                let for_left = self.left.active;
                let panel = self.active_panel();
                if panel.selected.len() == 1 {
                    if let Some(&idx) = panel.selected.iter().next() {
                        if let Some(entry) = panel.entries.get(idx) {
                            self.modal = Some(Modal::Rename(RenameState {
                                for_left,
                                old_name: entry.name.clone(),
                                new_name: entry.name.clone(),
                            }));
                        }
                    }
                }
            }
            "delete" => {
                let for_left = self.left.active;
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let paths: Vec<PathBuf> = panel.selected_entries().map(|e| dir.join(&e.name)).collect();
                if paths.is_empty() {
                    return;
                }
                if paths.iter().any(|p| file_ops::is_non_empty_dir(p)) {
                    let message = format!("Delete {} item(s), including non-empty folder(s)?", paths.len());
                    self.modal = Some(Modal::ConfirmDelete(ConfirmDeleteState { for_left, paths, message }));
                } else {
                    for path in &paths {
                        if let Err(err) = file_ops::delete(path) {
                            eprintln!("Suppression de {path:?} échouée : {err}");
                        }
                    }
                    self.panel_mut(for_left).navigate_to(dir);
                }
            }
            _ => {}
        }
    }
}

impl eframe::App for DirectoryOctopusApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let window_rect = ui.max_rect();

        // Presets de zoom X1/X2/X3 façon bytebox, au lieu d'une échelle
        // recalculée automatiquement à partir de la taille de fenêtre.
        for (key, zoom) in [
            (egui::Key::F1, theme::ZOOM_PRESETS[0]),
            (egui::Key::F2, theme::ZOOM_PRESETS[1]),
            (egui::Key::F3, theme::ZOOM_PRESETS[2]),
        ] {
            if ctx.input(|i| i.key_pressed(key)) {
                self.zoom = zoom;
            }
        }
        if (ctx.pixels_per_point() - self.zoom).abs() > f32::EPSILON {
            ctx.set_pixels_per_point(self.zoom);
        }
        if self.zoom != self.min_size_synced_for {
            self.min_size_synced_for = self.zoom;
            // La fenêtre ne doit jamais pouvoir descendre sous la taille dont
            // ce niveau de zoom a besoin, sous peine de chevauchements. Ces
            // valeurs restent en "points" : c'est egui qui les multiplie par
            // le pixels_per_point courant (donc notre zoom) pour obtenir la
            // taille physique minimale réelle.
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                theme::logical_width(),
                theme::logical_height(),
            )));
        }

        // Vérifié à chaque frame (donc dès la frame suivant un changement de
        // répertoire du panneau actif, quasi instantané) mais ne relance
        // réellement `df`/`date` que si le chemin a changé ou qu'une seconde
        // s'est écoulée.
        self.maybe_refresh_status_bar();
        // Garantit qu'une frame sera bien redessinée dans une seconde même
        // sans interaction utilisateur, pour que l'horloge continue d'avancer.
        ctx.request_repaint_after(STATUS_REFRESH_INTERVAL);

        egui::Panel::bottom("status_bar")
            .exact_size(theme::STATUS_BAR_HEIGHT)
            .frame(egui::Frame::new().fill(theme::BG_GREY).inner_margin(2.0))
            .show(ui, |ui| {
                show_status_bar(ui, &self.disk_status_line, &self.clock_line);
            });

        egui::Panel::bottom("button_bar")
            .exact_size(theme::button_bar_height())
            .frame(egui::Frame::new().fill(theme::BG_GREY).inner_margin(2.0))
            .show(ui, |ui| {
                let mut clicked_action = None;
                show_button_bar(ui, &self.buttons, &mut |action| {
                    clicked_action = Some(action.to_owned());
                });
                if let Some(action) = clicked_action {
                    self.handle_action(&action);
                }
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG_GREY))
            .show(ui, |ui| {
                ui.columns(2, |columns| {
                    let left_clicked = show_panel(&mut columns[0], &mut self.left);
                    let right_clicked = show_panel(&mut columns[1], &mut self.right);
                    // Sélectionner dans un panneau annule la sélection de l'autre :
                    // une seule sélection peut exister à la fois entre les deux volets.
                    if left_clicked {
                        self.left.active = true;
                        self.right.active = false;
                        self.right.select_none();
                    } else if right_clicked {
                        self.right.active = true;
                        self.left.active = false;
                        self.left.select_none();
                    }
                });
            });

        // Liseré 3D en relief tout autour de la fenêtre, façon écran Amiga.
        theme::draw_bevel(ui.painter(), window_rect, true);

        if let Some(modal) = &mut self.modal {
            match show_modal(&ctx, modal) {
                ModalAction::None => {}
                ModalAction::Close => self.modal = None,
                ModalAction::SaveDriveSlot { row_idx, label, path } => {
                    self.buttons.set_drive_slot(row_idx, label, path);
                    self.buttons.save_drive_slots();
                    self.modal = None;
                }
                ModalAction::ApplyRename { for_left, old_name, new_name } => {
                    let panel = self.panel_mut(for_left);
                    let dir = PathBuf::from(&panel.path);
                    let new_name = new_name.trim();
                    if !new_name.is_empty() && new_name != old_name {
                        if let Err(err) = std::fs::rename(dir.join(&old_name), dir.join(new_name)) {
                            eprintln!("Renommage de {old_name:?} échoué : {err}");
                        }
                    }
                    panel.navigate_to(dir);
                    self.modal = None;
                }
                ModalAction::ConfirmDelete { for_left, paths } => {
                    for path in &paths {
                        if let Err(err) = file_ops::delete(path) {
                            eprintln!("Suppression de {path:?} échouée : {err}");
                        }
                    }
                    let panel = self.panel_mut(for_left);
                    let dir = PathBuf::from(&panel.path);
                    panel.navigate_to(dir);
                    self.modal = None;
                }
            }
        }
    }
}
