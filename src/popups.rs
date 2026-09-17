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

/// Renommage d'un fichier/dossier sélectionné dans un panneau.
pub struct RenameState {
    /// `true` si l'élément à renommer est dans le panneau gauche.
    pub for_left: bool,
    pub old_name: String,
    pub new_name: String,
}

/// Confirmation avant suppression (un ou plusieurs dossiers non vides sont
/// concernés, cf. `file_ops::is_non_empty_dir`).
pub struct ConfirmDeleteState {
    pub for_left: bool,
    pub paths: Vec<PathBuf>,
    pub message: String,
}

/// Le pop-up actuellement affiché par-dessus le reste de l'interface, le cas
/// échéant (un seul à la fois).
pub enum Modal {
    EditDriveSlot(DriveSlotEdit),
    Rename(RenameState),
    ConfirmDelete(ConfirmDeleteState),
}

/// Ce que l'utilisateur a décidé de faire du pop-up affiché cette frame.
pub enum ModalAction {
    None,
    Close,
    SaveDriveSlot { row_idx: usize, label: String, path: Option<PathBuf> },
    ApplyRename { for_left: bool, old_name: String, new_name: String },
    ConfirmDelete { for_left: bool, paths: Vec<PathBuf> },
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
/// pour les actions des pop-ups (OK/Annuler/Delete...).
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

/// Enrobe le contenu d'un pop-up dans le même code esthétique partout : fond
/// gris, relief 3D en bosse, centré à l'écran, au-dessus de tout le reste.
fn popup_frame(ctx: &Context, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Area::new(egui::Id::new("app_modal_popup"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            let frame_response = egui::Frame::new()
                .fill(theme::BG_GREY)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_min_width(260.0);
                    ui.vertical(add_contents);
                });
            theme::draw_bevel(ui.painter(), frame_response.response.rect, true);
        });
}

fn show_edit_drive_slot_popup(ctx: &Context, edit: &mut DriveSlotEdit) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, |ui| {
        ui.label(RichText::new("Shortcut name").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        styled_text_edit(ui, &mut edit.label);
        ui.add_space(8.0);
        ui.label(RichText::new("Directory").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        styled_text_edit(ui, &mut edit.path);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if popup_button(ui, "OK") {
                let label = if edit.label.trim().is_empty() {
                    "UNNAMED".to_owned()
                } else {
                    format!("{}:", edit.label.trim())
                };
                let path = if edit.path.trim().is_empty() { None } else { Some(PathBuf::from(edit.path.trim())) };
                action = ModalAction::SaveDriveSlot { row_idx: edit.row_idx, label, path };
            }
            if popup_button(ui, "Cancel") {
                action = ModalAction::Close;
            }
        });
    });
    action
}

fn show_rename_popup(ctx: &Context, state: &mut RenameState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, |ui| {
        ui.label(RichText::new("New name").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        styled_text_edit(ui, &mut state.new_name);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if popup_button(ui, "OK") {
                action = ModalAction::ApplyRename {
                    for_left: state.for_left,
                    old_name: state.old_name.clone(),
                    new_name: state.new_name.clone(),
                };
            }
            if popup_button(ui, "Cancel") {
                action = ModalAction::Close;
            }
        });
    });
    action
}

fn show_confirm_delete_popup(ctx: &Context, state: &mut ConfirmDeleteState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, |ui| {
        ui.label(RichText::new(&state.message).color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if popup_button(ui, "Delete") {
                action = ModalAction::ConfirmDelete { for_left: state.for_left, paths: state.paths.clone() };
            }
            if popup_button(ui, "Cancel") {
                action = ModalAction::Close;
            }
        });
    });
    action
}

/// Affiche le pop-up correspondant à l'état courant du modal.
pub fn show_modal(ctx: &Context, modal: &mut Modal) -> ModalAction {
    match modal {
        Modal::EditDriveSlot(edit) => show_edit_drive_slot_popup(ctx, edit),
        Modal::Rename(state) => show_rename_popup(ctx, state),
        Modal::ConfirmDelete(state) => show_confirm_delete_popup(ctx, state),
    }
}
