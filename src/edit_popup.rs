use std::path::PathBuf;

use egui::{Color32, Context, RichText, TextEdit, Vec2};

use crate::theme;

/// État d'édition d'un raccourci de la colonne de gauche (HOME:, ROOT:, ...),
/// ouvert par un clic droit dessus.
pub struct DriveSlotEdit {
    pub row_idx: usize,
    pub label: String,
    pub path: String,
}

impl DriveSlotEdit {
    pub fn new(row_idx: usize, label: &str, path: Option<&PathBuf>) -> Self {
        Self {
            row_idx,
            label: label.trim_end_matches(':').to_owned(),
            path: path.map(|p| p.display().to_string()).unwrap_or_default(),
        }
    }
}

/// Ce que l'utilisateur a décidé de faire du pop-up cette frame.
pub enum EditPopupAction {
    None,
    Save { row_idx: usize, label: String, path: Option<PathBuf> },
    Cancel,
}

/// Champ de saisie stylé comme la barre de chemin des panneaux fichiers :
/// fond gris, texte noir en Topaz, relief "en creux" (zone de saisie).
fn styled_text_edit(ui: &mut egui::Ui, text: &mut String) {
    let frame_response = egui::Frame::new()
        .fill(theme::BG_GREY)
        .inner_margin(2.0)
        .show(ui, |ui| {
            let size = Vec2::new(ui.available_width(), theme::PATH_BAR_HEIGHT);
            ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_min_width(ui.available_width());
                ui.add(
                    TextEdit::singleline(text)
                        .frame(egui::Frame::NONE)
                        .text_color(Color32::BLACK)
                        .font(egui::FontId::monospace(theme::SMALL_TEXT_SIZE))
                        .desired_width(f32::INFINITY),
                );
            });
        });
    theme::draw_bevel(ui.painter(), frame_response.response.rect, false);
}

/// Bouton gris avec relief, comme les raccourcis de la colonne de gauche,
/// pour les actions du pop-up (OK/Annuler).
fn popup_button(ui: &mut egui::Ui, label: &str) -> bool {
    let size = Vec2::new(90.0, theme::BUTTON_ROW_HEIGHT);
    let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    painter.rect_filled(resp.rect, 0.0, theme::BG_GREY);
    theme::draw_bevel(&painter, resp.rect, !pressed);
    painter.text(
        egui::pos2(resp.rect.center().x, resp.rect.top() + theme::BUTTON_TEXT_TOP_PADDING),
        egui::Align2::CENTER_TOP,
        label,
        egui::FontId::monospace(10.0),
        Color32::BLACK,
    );
    resp.clicked()
}

/// Affiche le pop-up d'édition d'un raccourci, centré, avec le même code
/// esthétique que le reste de l'appli (fond gris, relief 3D, police Topaz).
pub fn show_edit_popup(ctx: &Context, edit: &mut DriveSlotEdit) -> EditPopupAction {
    let mut action = EditPopupAction::None;

    egui::Area::new(egui::Id::new("drive_slot_edit_popup"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            let frame_response = egui::Frame::new()
                .fill(theme::BG_GREY)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_min_width(260.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Shortcut name")
                                .color(Color32::BLACK)
                                .size(theme::SMALL_TEXT_SIZE),
                        );
                        styled_text_edit(ui, &mut edit.label);
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Directory")
                                .color(Color32::BLACK)
                                .size(theme::SMALL_TEXT_SIZE),
                        );
                        styled_text_edit(ui, &mut edit.path);
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if popup_button(ui, "OK") {
                                let label = if edit.label.trim().is_empty() {
                                    "UNNAMED".to_owned()
                                } else {
                                    format!("{}:", edit.label.trim())
                                };
                                let path = if edit.path.trim().is_empty() {
                                    None
                                } else {
                                    Some(PathBuf::from(edit.path.trim()))
                                };
                                action = EditPopupAction::Save { row_idx: edit.row_idx, label, path };
                            }
                            if popup_button(ui, "Cancel") {
                                action = EditPopupAction::Cancel;
                            }
                        });
                    });
                });
            theme::draw_bevel(ui.painter(), frame_response.response.rect, true);
        });

    action
}
