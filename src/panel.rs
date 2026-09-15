use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use egui::{Color32, RichText, ScrollArea, Sense, Ui, Vec2};

use crate::theme;

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub size: Option<u64>,
    pub is_dir: bool,
}

pub struct PanelState {
    /// Identifiant stable ("left"/"right") distinguant les deux volets pour
    /// egui, indépendamment du répertoire affiché (qui peut être le même des
    /// deux côtés).
    pub id: &'static str,
    pub title: String,
    pub size_label: String,
    pub path: String,
    pub entries: Vec<FileEntry>,
    /// Indices des éléments sélectionnés (fichiers et/ou dossiers mélangés).
    /// Sélectionner dans l'autre panneau vide cette liste (cf. app.rs).
    pub selected: HashSet<usize>,
    /// Point de départ d'une future sélection par plage (Maj+clic) : le
    /// dernier élément cliqué en clic simple ou Ctrl+clic.
    pub range_anchor: Option<usize>,
    pub active: bool,
}

impl PanelState {
    pub fn at_path(id: &'static str, path: PathBuf) -> Self {
        let mut panel = Self {
            id,
            title: String::new(),
            size_label: "--".to_owned(),
            path: String::new(),
            entries: Vec::new(),
            selected: HashSet::new(),
            range_anchor: None,
            active: false,
        };
        panel.navigate_to(path);
        panel
    }

    /// Change le répertoire affiché par ce panneau et relit son contenu réel
    /// sur le disque.
    pub fn navigate_to(&mut self, path: PathBuf) {
        self.title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| path.display().to_string());
        self.path = path.display().to_string();
        self.entries = read_directory(&path);
        self.selected.clear();
        self.range_anchor = None;
    }

    /// Les entrées (fichiers et dossiers confondus) actuellement sélectionnées.
    pub fn selected_entries(&self) -> impl Iterator<Item = &FileEntry> {
        self.selected.iter().filter_map(|&idx| self.entries.get(idx))
    }

    pub fn select_all(&mut self) {
        self.selected = (0..self.entries.len()).collect();
    }

    pub fn select_none(&mut self) {
        self.selected.clear();
        self.range_anchor = None;
    }

    /// Sélectionne la plage continue entre le point d'ancrage courant (ou
    /// `idx` lui-même si aucun) et `idx`, comme pour un Maj+clic.
    pub fn select_range_to(&mut self, idx: usize) {
        let anchor = self.range_anchor.unwrap_or(idx);
        let (start, end) = if anchor <= idx { (anchor, idx) } else { (idx, anchor) };
        self.selected = (start..=end).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_panel() -> PanelState {
        PanelState {
            id: "left",
            title: "test".into(),
            size_label: "--".into(),
            path: "/tmp".into(),
            entries: vec![
                FileEntry { name: "docs".into(), size: Some(100), is_dir: true },
                FileEntry { name: "readme.txt".into(), size: Some(20), is_dir: false },
                FileEntry { name: "src".into(), size: Some(40), is_dir: true },
            ],
            selected: HashSet::new(),
            range_anchor: None,
            active: false,
        }
    }

    #[test]
    fn select_all_and_none_cover_every_entry_regardless_of_type() {
        let mut panel = sample_panel();
        panel.select_all();
        assert_eq!(panel.selected.len(), 3);
        assert_eq!(panel.selected_entries().count(), 3);

        panel.select_none();
        assert!(panel.selected.is_empty());
    }

    #[test]
    fn selected_entries_sums_sizes_across_dirs_and_files() {
        let mut panel = sample_panel();
        panel.selected.insert(0); // dossier "docs"
        panel.selected.insert(1); // fichier "readme.txt"

        let total: u64 = panel.selected_entries().filter_map(|e| e.size).sum();
        assert_eq!(total, 120);

        let dirs_selected = panel.selected_entries().filter(|e| e.is_dir).count();
        let files_selected = panel.selected_entries().filter(|e| !e.is_dir).count();
        assert_eq!(dirs_selected, 1);
        assert_eq!(files_selected, 1);
    }

    #[test]
    fn shift_click_selects_continuous_range_from_anchor() {
        let mut panel = sample_panel();
        panel.entries.push(FileEntry { name: "z.txt".into(), size: Some(1), is_dir: false });

        panel.range_anchor = Some(1);
        panel.select_range_to(3);
        assert_eq!(panel.selected, HashSet::from([1, 2, 3]));

        // La plage fonctionne aussi en partant d'un index plus grand vers un
        // plus petit (ordre de clic inversé).
        panel.range_anchor = Some(3);
        panel.select_range_to(0);
        assert_eq!(panel.selected, HashSet::from([0, 1, 2, 3]));
    }

    #[test]
    fn shift_click_without_prior_anchor_selects_only_clicked_item() {
        let mut panel = sample_panel();
        assert_eq!(panel.range_anchor, None);
        panel.select_range_to(2);
        assert_eq!(panel.selected, HashSet::from([2]));
    }
}

/// Lit le contenu d'un répertoire : dossiers d'abord, puis fichiers, triés par
/// nom. Un répertoire illisible (permissions, périphérique absent...) donne
/// simplement une liste vide plutôt que de planter.
fn read_directory(path: &Path) -> Vec<FileEntry> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    if let Ok(read_dir) = fs::read_dir(path) {
        for entry in read_dir.flatten() {
            let Ok(file_type) = entry.file_type() else { continue };
            let name = entry.file_name().to_string_lossy().into_owned();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let file_entry = FileEntry { name, size: Some(size), is_dir: file_type.is_dir() };
            if file_type.is_dir() {
                dirs.push(file_entry);
            } else {
                files.push(file_entry);
            }
        }
    }

    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    dirs.into_iter().chain(files).collect()
}

/// Dessine un panneau fichiers (bandeau titre + liste), au style Directory Opus 4.
/// Retourne `true` si l'utilisateur vient de cliquer dans ce panneau (pour que
/// l'appelant le rende actif).
pub fn show_panel(ui: &mut Ui, panel: &mut PanelState) -> bool {
    let mut activated = false;
    let header_bg = if panel.active {
        theme::HEADER_ACTIVE_BG
    } else {
        theme::HEADER_INACTIVE_BG
    };

    // Tant qu'une sélection est active, l'en-tête affiche son décompte à la
    // place de la taille du répertoire (retour visuel direct de la sélection
    // multiple, fichiers et dossiers confondus).
    let header_right_text = if panel.selected.is_empty() {
        panel.size_label.clone()
    } else {
        let total_size: u64 = panel.selected_entries().filter_map(|e| e.size).sum();
        format!("{} sel. ({total_size})", panel.selected.len())
    };

    let header_frame = egui::Frame::new().fill(header_bg).inner_margin(2.0).show(ui, |ui| {
        let size = Vec2::new(ui.available_width(), theme::PANEL_HEADER_HEIGHT);
        ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(&panel.title)
                    .color(theme::HEADER_TEXT)
                    .size(theme::SMALL_TEXT_SIZE)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(&header_right_text)
                        .color(theme::HEADER_TEXT)
                        .size(theme::SMALL_TEXT_SIZE),
                );
            });
        });
    });
    if ui
        .interact(header_frame.response.rect, ui.id().with((panel.id, "header")), Sense::click())
        .clicked()
    {
        activated = true;
    }

    let available_height = (ui.available_height() - (theme::PATH_BAR_HEIGHT + 4.0)).max(0.0);
    // On ne montre que des lignes entières : sans ça, la dernière ligne
    // visible est tranchée en plein milieu dès que la hauteur disponible
    // n'est pas un multiple exact de LIST_ROW_HEIGHT. Le reliquat devient un
    // peu de marge grise en bas, invisible car de la même couleur que le fond.
    let visible_rows = (available_height / theme::LIST_ROW_HEIGHT).floor();
    let scroll_height = visible_rows * theme::LIST_ROW_HEIGHT;
    let list_frame = egui::Frame::new().fill(theme::BG_GREY).show(ui, |ui| {
        ui.set_min_size(Vec2::new(ui.available_width(), available_height));
        ScrollArea::vertical()
            .id_salt(panel.id)
            .max_height(scroll_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                // Sans ça, l'espacement vertical par défaut d'egui entre
                // chaque ligne fausserait le calcul de `scroll_height`
                // ci-dessus (qui suppose des lignes flush les unes aux
                // autres, exactement LIST_ROW_HEIGHT chacune).
                ui.spacing_mut().item_spacing.y = 0.0;
                for idx in 0..panel.entries.len() {
                    let entry = &panel.entries[idx];
                    let is_selected = panel.selected.contains(&idx);
                    // Le fond du panneau reste gris en permanence : seul
                    // l'élément sélectionné reçoit un fond coloré (bleu pour
                    // un dossier, noir pour un fichier), jamais tout le volet.
                    let (row_bg, fg) = match (is_selected, entry.is_dir) {
                        (true, true) => (theme::LIST_BLUE, theme::LIST_SELECTED_TEXT),
                        (true, false) => (theme::LIST_SELECTED_FILE_BG, theme::LIST_SELECTED_TEXT),
                        (false, true) => (Color32::TRANSPARENT, theme::LIST_BLUE),
                        (false, false) => (Color32::TRANSPARENT, theme::LIST_FILE_TEXT),
                    };

                    let (row_response, painter) = ui.allocate_painter(
                        Vec2::new(ui.available_width(), theme::LIST_ROW_HEIGHT),
                        Sense::click(),
                    );
                    let rect = row_response.rect;
                    painter.rect_filled(rect, 0.0, row_bg);

                    painter.text(
                        rect.left_top() + Vec2::new(4.0, theme::LIST_TEXT_TOP_PADDING),
                        egui::Align2::LEFT_TOP,
                        &entry.name,
                        egui::FontId::monospace(theme::LIST_TEXT_SIZE),
                        fg,
                    );
                    if let Some(size) = entry.size {
                        painter.text(
                            rect.right_top() + Vec2::new(-4.0, theme::LIST_TEXT_TOP_PADDING),
                            egui::Align2::RIGHT_TOP,
                            size.to_string(),
                            egui::FontId::monospace(theme::LIST_TEXT_SIZE),
                            fg,
                        );
                    }

                    if row_response.clicked() {
                        let (ctrl_held, shift_held) =
                            ui.input(|i| (i.modifiers.ctrl || i.modifiers.command, i.modifiers.shift));
                        if shift_held {
                            // Maj+clic : sélectionne la plage continue entre le
                            // dernier point d'ancrage et cet élément.
                            panel.select_range_to(idx);
                        } else if ctrl_held {
                            // Ctrl+clic : ajoute/retire cet élément de la sélection
                            // multiple sans toucher aux autres.
                            if !panel.selected.remove(&idx) {
                                panel.selected.insert(idx);
                            }
                            panel.range_anchor = Some(idx);
                        } else {
                            // Clic simple : ne garde que cet élément sélectionné,
                            // et devient le nouveau point d'ancrage pour Maj+clic.
                            panel.selected.clear();
                            panel.selected.insert(idx);
                            panel.range_anchor = Some(idx);
                        }
                        activated = true;
                    }
                }
            });
    });
    // Remarque : on n'ajoute pas de zone cliquable couvrant tout le fond du
    // panneau ici — elle recouvrirait les lignes de fichiers et intercepterait
    // leurs clics avant elles (egui donne la priorité au widget du dessus).
    // L'activation du panneau se fait donc via le bandeau titre ou une ligne.
    theme::draw_bevel(&ui.painter().with_clip_rect(list_frame.response.rect), list_frame.response.rect, false);

    let path_frame = egui::Frame::new()
        .fill(theme::BG_GREY)
        .inner_margin(2.0)
        .show(ui, |ui| {
            let size = Vec2::new(ui.available_width(), theme::PATH_BAR_HEIGHT);
            ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // Sans ça, la zone "utilisée" par cette ui se réduit à la
                // largeur du texte : le liseré 3D dessiné plus bas (basé sur
                // path_frame.response.rect) n'entourerait alors que le texte
                // au lieu de toute la largeur de la barre de chemin.
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new(&panel.path).color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
            });
        });
    theme::draw_bevel(&ui.painter().with_clip_rect(path_frame.response.rect), path_frame.response.rect, false);

    activated
}
