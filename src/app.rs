use std::path::PathBuf;

use crate::button_bar::{show_button_bar, ButtonBarConfig};
use crate::edit_popup::{show_edit_popup, DriveSlotEdit, EditPopupAction};
use crate::panel::{show_panel, PanelState};
use crate::status_bar::show_status_bar;
use crate::theme;

pub struct DirectoryOctopusApp {
    left: PanelState,
    right: PanelState,
    buttons: ButtonBarConfig,
    zoom: f32,
    /// Dernier niveau de zoom pour lequel la taille minimale de fenêtre a été
    /// envoyée au gestionnaire de fenêtres ; `0.0` force l'envoi initial.
    min_size_synced_for: f32,
    /// Raccourci de la colonne de gauche en cours d'édition (pop-up ouvert
    /// par un clic droit dessus), le cas échéant.
    editing_drive_slot: Option<DriveSlotEdit>,
}

impl Default for DirectoryOctopusApp {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_owned());

        let mut left = PanelState::at_path("left", PathBuf::from(&home));
        left.active = true;

        let start_right = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(&home));
        let right = PanelState::at_path("right", start_right);

        Self {
            left,
            right,
            buttons: ButtonBarConfig::load_or_default(),
            zoom: theme::DEFAULT_ZOOM,
            min_size_synced_for: 0.0,
            editing_drive_slot: None,
        }
    }
}

impl DirectoryOctopusApp {
    /// Le panneau actuellement actif (celui dont le bandeau titre est en rouge),
    /// destinataire des raccourcis de navigation et futures actions de bouton.
    fn active_panel_mut(&mut self) -> &mut PanelState {
        if self.left.active { &mut self.left } else { &mut self.right }
    }

    fn handle_action(&mut self, action: &str) {
        if let Some(target) = action.strip_prefix("goto:") {
            self.active_panel_mut().navigate_to(PathBuf::from(target));
            return;
        }
        if let Some(row_idx) = action.strip_prefix("edit_drive:").and_then(|s| s.parse::<usize>().ok()) {
            if let Some(slot) = self.buttons.drive_slots.get(row_idx) {
                self.editing_drive_slot = Some(DriveSlotEdit::new(row_idx, &slot.label, slot.path.as_ref()));
            }
            return;
        }
        match action {
            "select_all" => self.active_panel_mut().select_all(),
            "select_none" => self.active_panel_mut().select_none(),
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

        egui::Panel::bottom("status_bar")
            .exact_size(theme::STATUS_BAR_HEIGHT)
            .frame(egui::Frame::new().fill(theme::BG_GREY).inner_margin(2.0))
            .show(ui, |ui| {
                show_status_bar(ui, "DISK: -- FREE: --", "today  --:--");
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

        if let Some(edit) = &mut self.editing_drive_slot {
            match show_edit_popup(&ctx, edit) {
                EditPopupAction::Save { row_idx, label, path } => {
                    self.buttons.set_drive_slot(row_idx, label, path);
                    self.buttons.save_drive_slots();
                    self.editing_drive_slot = None;
                }
                EditPopupAction::Cancel => self.editing_drive_slot = None,
                EditPopupAction::None => {}
            }
        }
    }
}
