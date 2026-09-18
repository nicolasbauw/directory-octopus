use std::path::PathBuf;

use egui::{Ui, Vec2};

use crate::config::{AppConfig, CustomButtonConfig, DriveSlotConfig};
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
/// `command` n'est renseigné que pour les boutons ajoutés par l'utilisateur
/// (clic droit sur une case grise) : il marque le bouton comme "personnalisé"
/// (liseré distinctif, ré-éditable) et porte la commande shell à exécuter.
#[derive(Clone)]
pub struct ButtonSlot {
    pub label: String,
    pub style: ButtonStyle,
    pub action: Option<String>,
    pub command: Option<String>,
}

impl ButtonSlot {
    pub fn new(label: &str, style: ButtonStyle, action: &str) -> Self {
        Self { label: label.to_owned(), style, action: Some(action.to_owned()), command: None }
    }

    /// Bouton ajouté par l'utilisateur : la couleur reprend celle de la
    /// colonne, l'action exécute directement `command` via un shell.
    pub fn custom(label: String, style: ButtonStyle, command: String) -> Self {
        let action = format!("custom:{command}");
        Self { label, style, action: Some(action), command: Some(command) }
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
        // "mount" s'appuie sur l'utilitaire `mount` (Device / Mount point) :
        // un concept propre à Linux (comme "Assign" l'était à AmigaDOS),
        // donc ce bouton n'existe que sur cette plateforme.
        #[cfg(target_os = "linux")]
        let mount_button = Some(b("Mount", Orange, "mount"));
        #[cfg(not(target_os = "linux"))]
        let mount_button = None;
        Self {
            drive_slots: vec![
                DriveSlot::new("HOME:", &home),
                DriveSlot::new("ROOT:", "/"),
                DriveSlot::new("MEDIA:", "/run/media"),
                DriveSlot::new("TRASH:", format!("{home}/.local/share/Trash/files")),
                DriveSlot::placeholder("CUSTOMIZE:"),
                DriveSlot::placeholder("CUSTOMIZE:"),
            ],
            rows: vec![
                vec![
                    Some(b("All", Blue, "select_all")),
                    Some(b("Copy", Purple, "copy")),
                    Some(b("Makedir", Orange, "makedir")),
                    Some(b("Find", Black, "find")),
                    Some(b("Run", Orange, "run")),
                    Some(b("Datestamp", Grey, "datestamp")),
                    Some(b("Read", Red, "read")),
                ],
                vec![
                    Some(b("None", Blue, "select_none")),
                    Some(b("Move", Purple, "move")),
                    mount_button,
                    Some(b("Search", Black, "search")),
                    None,
                    Some(b("Permissions", Grey, "permissions")),
                    Some(b("Hex Read", Red, "hex_read")),
                ],
                vec![
                    Some(b("Parent", Blue, "parent")),
                    Some(b("Rename", Purple, "rename")),
                    None,
                    None,
                    None,
                    Some(b("Extract", Grey, "extract")),
                    Some(b("Edit", Red, "edit")),
                ],
                vec![
                    Some(b("Root", Blue, "root")),
                    None,
                    None,
                    None,
                    None,
                    Some(b("UPX", Grey, "upx")),
                    None,
                ],
                vec![
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ],
                vec![
                    None,
                    Some(b("DELETE", Red, "delete")),
                    None,
                    None,
                    None,
                    None,
                    None,
                ],
            ],
        }
    }

    /// Applique les nouvelles valeurs (libellé + répertoire, saisies dans le
    /// pop-up d'édition) à un raccourci de la colonne de gauche.
    pub fn set_drive_slot(&mut self, row_idx: usize, label: String, path: Option<PathBuf>) {
        if let Some(slot) = self.drive_slots.get_mut(row_idx) {
            slot.label = label;
            slot.path = path;
        }
    }

    /// Couleur "de la colonne" `col_idx` : celle du premier bouton non-vide
    /// qui s'y trouve (built-in ou personnalisé), pour que les boutons
    /// ajoutés par l'utilisateur reprennent le code couleur existant. Repli
    /// sur `Grey` si la colonne est entièrement vide.
    pub fn column_style(&self, col_idx: usize) -> ButtonStyle {
        self.rows
            .iter()
            .filter_map(|row| row.get(col_idx).and_then(|slot| slot.as_ref()))
            .map(|slot| slot.style)
            .next()
            .unwrap_or(ButtonStyle::Grey)
    }

    /// Ajoute (ou remplace) un bouton personnalisé à l'emplacement donné.
    pub fn set_custom_button(&mut self, row_idx: usize, col_idx: usize, label: String, command: String) {
        let style = self.column_style(col_idx);
        if let Some(slot) = self.rows.get_mut(row_idx).and_then(|row| row.get_mut(col_idx)) {
            *slot = Some(ButtonSlot::custom(label, style, command));
        }
    }

    /// Retire un bouton personnalisé, redonnant sa case grise vide.
    pub fn remove_custom_button(&mut self, row_idx: usize, col_idx: usize) {
        if let Some(slot) = self.rows.get_mut(row_idx).and_then(|row| row.get_mut(col_idx)) {
            *slot = None;
        }
    }

    /// Charge les raccourcis et boutons personnalisés depuis
    /// `~/.config/directory-octopus/` s'ils existent ; sinon repart de la
    /// disposition par défaut.
    pub fn load_or_default() -> Self {
        let mut config = Self::default_layout();
        if let Some(saved) = AppConfig::load() {
            if !saved.drive_slots.is_empty() {
                config.drive_slots = saved
                    .drive_slots
                    .into_iter()
                    .map(|s| DriveSlot { label: s.label, path: s.path })
                    .collect();
            }
            for button in saved.custom_buttons {
                config.set_custom_button(button.row, button.col, button.label, button.command);
            }
        }
        config
    }

    /// Sauvegarde les raccourcis de gauche et les boutons personnalisés sur
    /// disque.
    pub fn save(&self) {
        let drive_slots = self
            .drive_slots
            .iter()
            .map(|s| DriveSlotConfig { label: s.label.clone(), path: s.path.clone() })
            .collect();
        let custom_buttons = self
            .rows
            .iter()
            .enumerate()
            .flat_map(|(row_idx, row)| {
                row.iter().enumerate().filter_map(move |(col_idx, slot)| {
                    let slot = slot.as_ref()?;
                    let command = slot.command.as_ref()?;
                    Some(CustomButtonConfig {
                        row: row_idx,
                        col: col_idx,
                        label: slot.label.clone(),
                        command: command.clone(),
                    })
                })
            })
            .collect();
        AppConfig { drive_slots, custom_buttons }.save();
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
            // Tous les raccourcis de gauche sont personnalisables : clic
            // gauche pour y naviguer (s'il pointe déjà vers un chemin), clic
            // droit pour éditer son libellé et son répertoire.
            let (resp, painter) = ui.allocate_painter(drive_size, egui::Sense::click());
            let pressed = resp.is_pointer_button_down_on();
            painter.rect_filled(resp.rect, 0.0, theme::BG_GREY);
            theme::draw_bevel(&painter, resp.rect, !pressed);
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
                if resp.secondary_clicked() {
                    on_action(&format!("edit_drive:{row_idx}"));
                }
            }

            let remaining = ui.available_width();
            let button_width = ((remaining - spacing * (columns - 1.0)) / columns).max(1.0);
            let size = Vec2::new(button_width, row_height);

            for (col_idx, slot) in row.iter().enumerate() {
                match slot {
                    Some(slot) => {
                        let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
                        let (bg, fg) = slot.style.colors();
                        let pressed = resp.is_pointer_button_down_on();
                        painter.rect_filled(resp.rect, 0.0, bg);
                        theme::draw_bevel(&painter, resp.rect, !pressed);
                        if slot.command.is_some() {
                            theme::draw_custom_marker(&painter, resp.rect);
                        }
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
                        if resp.secondary_clicked() && slot.command.is_some() {
                            on_action(&format!("edit_button:{row_idx}:{col_idx}"));
                        }
                    }
                    None => {
                        let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
                        painter.rect_filled(resp.rect, 0.0, theme::BG_GREY);
                        painter.rect_stroke(
                            resp.rect,
                            0.0,
                            egui::Stroke::new(theme::BEVEL_THICKNESS, theme::BEVEL_DARK),
                            egui::StrokeKind::Inside,
                        );
                        if resp.secondary_clicked() {
                            on_action(&format!("add_button:{row_idx}:{col_idx}"));
                        }
                    }
                }
            }
        });
    }
}
