use std::path::PathBuf;

use egui::{Ui, Vec2};

use crate::theme::{self, ButtonStyle};

/// Un raccourci fixe de la colonne de gauche (HOME:, ROOT:, ...), cliquable
/// pour naviguer directement vers le répertoire/périphérique associé.
#[derive(Clone)]
pub struct DriveSlot {
    pub label: String,
    pub path: Option<PathBuf>,
}

impl DriveSlot {
    pub fn new(label: &str, path: impl Into<PathBuf>) -> Self {
        Self { label: label.to_owned(), path: Some(path.into()) }
    }

    /// Un emplacement pas encore relié à un vrai chemin (ex: BOOKMARKS, en
    /// attendant la fonctionnalité de marque-pages personnalisés).
    pub fn placeholder(label: &str) -> Self {
        Self { label: label.to_owned(), path: None }
    }
}

/// Un bouton de la grille du bas. `action` est un identifiant libre, destiné plus
/// tard à être relié à une fonctionnalité built-in ou à un script utilisateur.
#[derive(Clone)]
pub struct ButtonSlot {
    pub label: String,
    pub style: ButtonStyle,
    pub action: Option<String>,
}

impl ButtonSlot {
    pub fn new(label: &str, style: ButtonStyle, action: &str) -> Self {
        Self { label: label.to_owned(), style, action: Some(action.to_owned()) }
    }
}

/// Grille de boutons paramétrable : `rows` lignes, chaque ligne étant une liste de
/// slots optionnels (une case vide = cellule grise inactive, comme sur l'original).
pub struct ButtonBarConfig {
    pub drive_slots: Vec<DriveSlot>,
    pub rows: Vec<Vec<Option<ButtonSlot>>>,
}

impl ButtonBarConfig {
    /// Configuration par défaut reprenant la disposition de la capture Directory
    /// Opus 4, avec des libellés de lecteurs Linux à la place de DF0/DF1/RAM/C.
    pub fn default_layout() -> Self {
        use ButtonStyle::*;
        let b = ButtonSlot::new;
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_owned());
        Self {
            drive_slots: vec![
                DriveSlot::new("HOME:", &home),
                DriveSlot::new("ROOT:", "/"),
                DriveSlot::new("MEDIA:", "/run/media"),
                DriveSlot::new("DEVICES:", "/dev"),
                DriveSlot::new("TRASH:", format!("{home}/.local/share/Trash/files")),
                DriveSlot::placeholder("BOOKMARKS:"),
            ],
            rows: vec![
                vec![
                    Some(b("All", Blue, "select_all")),
                    Some(b("Copy", Purple, "copy")),
                    Some(b("Makedir", Orange, "makedir")),
                    Some(b("Hunt", Black, "hunt")),
                    Some(b("Run", Orange, "run")),
                    Some(b("Comment", Grey, "comment")),
                    Some(b("Read", Red, "read")),
                ],
                vec![
                    Some(b("None", Blue, "select_none")),
                    Some(b("Move", Purple, "move")),
                    Some(b("Assign", Orange, "assign")),
                    Some(b("Search", Black, "search")),
                    None,
                    Some(b("Datestamp", Grey, "datestamp")),
                    Some(b("Hex Read", Red, "hex_read")),
                ],
                vec![
                    Some(b("Parent", Blue, "parent")),
                    Some(b("Rename", Purple, "rename")),
                    Some(b("Check Fit", Orange, "check_fit")),
                    None,
                    None,
                    Some(b("Protect", Grey, "protect")),
                    Some(b("Show", Red, "show")),
                ],
                vec![
                    Some(b("Root", Blue, "root")),
                    None,
                    Some(b("GetSizes", Orange, "get_sizes")),
                    None,
                    None,
                    Some(b("Icon Info", Grey, "icon_info")),
                    Some(b("Play", Red, "play")),
                ],
                vec![
                    None,
                    None,
                    None,
                    None,
                    None,
                    Some(b("Arc Ext", Grey, "arc_ext")),
                    Some(b("Edit", Red, "edit")),
                ],
                vec![
                    None,
                    Some(b("DELETE", Red, "delete")),
                    None,
                    None,
                    None,
                    Some(b("Encrypt", Grey, "encrypt")),
                    Some(b("Print", Red, "print")),
                ],
            ],
        }
    }
}

/// Affiche la grille de boutons en étirant les colonnes pour occuper toute la
/// largeur disponible (comme sur l'original, où la grille épouse exactement
/// la largeur des deux volets fichiers au-dessus).
pub fn show_button_bar(ui: &mut Ui, config: &ButtonBarConfig, on_action: &mut dyn FnMut(&str)) {
    let row_height = theme::BUTTON_ROW_HEIGHT;
    let spacing = theme::BUTTON_ROW_SPACING;
    let columns = config.rows[0].len() as f32;

    ui.spacing_mut().item_spacing = Vec2::new(spacing, spacing);

    for (row_idx, row) in config.rows.iter().enumerate() {
        ui.horizontal(|ui| {
            let drive_slot = config.drive_slots.get(row_idx);
            let drive_size = Vec2::new(theme::DRIVE_LABEL_WIDTH, row_height);
            let clickable = drive_slot.is_some_and(|s| s.path.is_some());
            let sense = if clickable { egui::Sense::click() } else { egui::Sense::hover() };
            let (resp, painter) = ui.allocate_painter(drive_size, sense);
            let pressed = clickable && resp.is_pointer_button_down_on();
            painter.rect_filled(resp.rect, 0.0, theme::BG_GREY);
            if clickable {
                theme::draw_bevel(&painter, resp.rect, !pressed);
            } else {
                painter.rect_stroke(
                    resp.rect,
                    0.0,
                    egui::Stroke::new(theme::BEVEL_THICKNESS, theme::BEVEL_DARK),
                    egui::StrokeKind::Inside,
                );
            }
            if let Some(slot) = drive_slot {
                painter.text(
                    resp.rect.left_top() + Vec2::new(3.0, theme::BUTTON_TEXT_TOP_PADDING),
                    egui::Align2::LEFT_TOP,
                    &slot.label,
                    egui::FontId::monospace(10.0),
                    egui::Color32::BLACK,
                );
                if resp.clicked() {
                    if let Some(path) = &slot.path {
                        on_action(&format!("goto:{}", path.display()));
                    }
                }
            }

            let remaining = ui.available_width();
            let button_width = ((remaining - spacing * (columns - 1.0)) / columns).max(1.0);
            let size = Vec2::new(button_width, row_height);

            for slot in row.iter() {
                match slot {
                    Some(slot) => {
                        let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
                        let (bg, fg) = slot.style.colors();
                        let pressed = resp.is_pointer_button_down_on();
                        painter.rect_filled(resp.rect, 0.0, bg);
                        theme::draw_bevel(&painter, resp.rect, !pressed);
                        painter.text(
                            egui::pos2(resp.rect.center().x, resp.rect.top() + theme::BUTTON_TEXT_TOP_PADDING),
                            egui::Align2::CENTER_TOP,
                            &slot.label,
                            egui::FontId::monospace(10.0),
                            fg,
                        );
                        if resp.clicked() {
                            if let Some(action) = &slot.action {
                                on_action(action);
                            }
                        }
                    }
                    None => {
                        let (resp, painter) = ui.allocate_painter(size, egui::Sense::hover());
                        painter.rect_filled(resp.rect, 0.0, theme::BG_GREY);
                        painter.rect_stroke(
                            resp.rect,
                            0.0,
                            egui::Stroke::new(theme::BEVEL_THICKNESS, theme::BEVEL_DARK),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }
        });
    }
}
