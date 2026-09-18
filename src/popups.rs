use std::path::PathBuf;

use egui::{Color32, Context, RichText, ScrollArea, TextEdit, Vec2};

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

/// Création d'un nouveau dossier dans un panneau.
pub struct MakeDirState {
    /// `true` si le dossier doit être créé dans le panneau gauche.
    pub for_left: bool,
    pub name: String,
}

/// Confirmation avant suppression (un ou plusieurs dossiers non vides sont
/// concernés, cf. `file_ops::is_non_empty_dir`).
pub struct ConfirmDeleteState {
    pub for_left: bool,
    pub paths: Vec<PathBuf>,
    pub message: String,
}

/// Saisie du motif de recherche (`*`/`?` supportés) pour "Find".
pub struct FindState {
    pub for_left: bool,
    pub pattern: String,
}

/// Résultats d'une recherche ayant trouvé plusieurs correspondances.
pub struct FindResultsState {
    pub for_left: bool,
    pub matches: Vec<PathBuf>,
    /// `true` si la recherche a atteint son plafond de résultats : il pourrait
    /// donc y en avoir davantage sur le disque que ce qui est listé ici.
    pub truncated: bool,
}

/// Montage manuel d'un périphérique (Linux/`mount` uniquement).
#[derive(Default)]
pub struct MountState {
    pub device: String,
    pub mount_point: String,
}

/// Exécution du fichier sélectionné, avec d'éventuels arguments.
pub struct RunState {
    pub path: PathBuf,
    pub args: String,
}

/// Édition des droits d'accès des éléments sélectionnés ("Permissions").
/// Sous Unix : lecture/écriture/exécution pour propriétaire/groupe/autres.
/// Ailleurs (Windows) : seul l'attribut lecture seule a un équivalent direct.
#[derive(Clone)]
pub struct PermissionsState {
    pub paths: Vec<PathBuf>,
    #[cfg(unix)]
    pub owner: [bool; 3],
    #[cfg(unix)]
    pub group: [bool; 3],
    #[cfg(unix)]
    pub other: [bool; 3],
    #[cfg(not(unix))]
    pub read_only: bool,
}

/// Saisie du motif (regex) pour "Search", recherché dans les fichiers
/// sélectionnés au moment de l'ouverture du pop-up.
pub struct SearchState {
    pub paths: Vec<PathBuf>,
    pub pattern: String,
}

/// Une ligne trouvée par "Search", affichée dans le pop-up de résultats.
pub struct SearchResultLine {
    pub path: PathBuf,
    pub line_number: usize,
    pub line: String,
}

pub struct SearchResultsState {
    pub matches: Vec<SearchResultLine>,
    pub truncated: bool,
}

/// Visualisateur plein écran d'un fichier texte (numéros de ligne, défilement
/// horizontal/vertical), ouvert par "Read" ou "View file" depuis un résultat
/// de recherche.
pub struct ViewFileState {
    pub path: PathBuf,
    pub lines: Vec<String>,
    /// `true` si le fichier a été coupé (trop de lignes) pour rester réactif.
    pub truncated: bool,
    /// Ligne à amener à l'écran à l'ouverture (ex : depuis un résultat Search).
    pub highlight_line: Option<usize>,
    /// `true` une fois le défilement initial vers `highlight_line` effectué,
    /// pour ne pas re-scroller à chaque frame par-dessus le défilement manuel
    /// de l'utilisateur.
    pub scrolled: bool,
}

/// Le pop-up actuellement affiché par-dessus le reste de l'interface, le cas
/// échéant (un seul à la fois).
pub enum Modal {
    EditDriveSlot(DriveSlotEdit),
    Rename(RenameState),
    MakeDir(MakeDirState),
    Mount(MountState),
    Run(RunState),
    Permissions(PermissionsState),
    Find(FindState),
    FindResults(FindResultsState),
    Search(SearchState),
    SearchResults(SearchResultsState),
    ViewFile(ViewFileState),
    ConfirmDelete(ConfirmDeleteState),
    /// Message d'erreur simple (ex : échec de `mount`), avec un bouton OK.
    Error(String),
}

/// Ce que l'utilisateur a décidé de faire du pop-up affiché cette frame.
pub enum ModalAction {
    None,
    Close,
    SaveDriveSlot { row_idx: usize, label: String, path: Option<PathBuf> },
    ApplyRename { for_left: bool, old_name: String, new_name: String },
    CreateDir { for_left: bool, name: String },
    Mount { device: String, mount_point: String },
    RunProgram { path: PathBuf, args: String },
    ApplyPermissions(PermissionsState),
    RunFind { for_left: bool, pattern: String },
    RunSearch { paths: Vec<PathBuf>, pattern: String },
    JumpTo { for_left: bool, path: PathBuf },
    ViewFile { path: PathBuf, line: Option<usize> },
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

/// Largeur (en points) d'un texte avec cette police, pour la troncature
/// pixel-perfect ci-dessous (indépendante du niveau de zoom X1/X2/X3, car
/// mesurée avec la même police/taille que celle réellement dessinée).
fn text_width(ui: &egui::Ui, text: &str, font_id: &egui::FontId) -> f32 {
    ui.ctx().fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font_id.clone(), Color32::BLACK).rect.width())
}

/// Coupe `text` avec "..." (ASCII — garanti présent dans notre police bitmap,
/// contrairement à une ellipse Unicode) pour qu'il tienne dans `max_width`
/// points. Sans ça, les lignes de code longues débordent silencieusement du
/// pop-up au lieu de rester lisibles.
fn truncate_to_fit(ui: &egui::Ui, text: &str, font_id: &egui::FontId, max_width: f32) -> String {
    if text_width(ui, text, font_id) <= max_width {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = lo + (hi - lo + 1) / 2;
        let candidate = format!("{}...", chars[..mid].iter().collect::<String>());
        if text_width(ui, &candidate, font_id) <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    format!("{}...", chars[..lo].iter().collect::<String>())
}

/// Ligne cliquable pour un résultat de recherche : même langage visuel que
/// les lignes de fichiers des panneaux (surbrillance bleue au survol).
fn popup_result_row(ui: &mut egui::Ui, text: &str) -> bool {
    let size = Vec2::new(ui.available_width(), theme::LIST_ROW_HEIGHT);
    let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
    let (bg, fg) = if resp.hovered() { (theme::LIST_BLUE, Color32::WHITE) } else { (Color32::TRANSPARENT, Color32::BLACK) };
    painter.rect_filled(resp.rect, 0.0, bg);
    let font_id = egui::FontId::monospace(theme::LIST_TEXT_SIZE);
    let max_width = (resp.rect.width() - 8.0).max(0.0);
    let display_text = truncate_to_fit(ui, text, &font_id, max_width);
    painter.text(
        resp.rect.left_top() + Vec2::new(4.0, theme::LIST_TEXT_TOP_PADDING),
        egui::Align2::LEFT_TOP,
        display_text,
        font_id,
        fg,
    );
    resp.clicked()
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
fn popup_frame(ctx: &Context, min_width: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Area::new(egui::Id::new("app_modal_popup"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            let frame_response = egui::Frame::new()
                .fill(theme::BG_GREY)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_min_width(min_width);
                    ui.vertical(add_contents);
                });
            theme::draw_bevel(ui.painter(), frame_response.response.rect, true);
        });
}

const DEFAULT_POPUP_WIDTH: f32 = 260.0;
const ERROR_POPUP_WIDTH: f32 = 420.0;
const RESULTS_POPUP_WIDTH: f32 = 760.0;
/// Marge laissée visible autour du visualisateur de fichier "plein écran".
const FULLSCREEN_MARGIN: f32 = 24.0;

/// Comme `popup_frame`, mais occupe (presque) toute la fenêtre plutôt qu'une
/// taille minimale liée au contenu — pour le visualisateur de fichier.
fn show_fullscreen_frame(ctx: &Context, window_rect: egui::Rect, add_contents: impl FnOnce(&mut egui::Ui)) {
    let size = (window_rect.size() - Vec2::splat(FULLSCREEN_MARGIN * 2.0)).max(Vec2::splat(100.0));
    egui::Area::new(egui::Id::new("app_modal_popup"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            let frame_response = egui::Frame::new()
                .fill(theme::BG_GREY)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.set_min_size(size);
                    ui.set_max_size(size);
                    ui.vertical(add_contents);
                });
            theme::draw_bevel(ui.painter(), frame_response.response.rect, true);
        });
}

fn show_edit_drive_slot_popup(ctx: &Context, edit: &mut DriveSlotEdit) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, DEFAULT_POPUP_WIDTH, |ui| {
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
    popup_frame(ctx, DEFAULT_POPUP_WIDTH, |ui| {
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

fn show_makedir_popup(ctx: &Context, state: &mut MakeDirState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, DEFAULT_POPUP_WIDTH, |ui| {
        ui.label(RichText::new("Folder name").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        styled_text_edit(ui, &mut state.name);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if popup_button(ui, "OK") {
                action = ModalAction::CreateDir { for_left: state.for_left, name: state.name.clone() };
            }
            if popup_button(ui, "Cancel") {
                action = ModalAction::Close;
            }
        });
    });
    action
}

fn show_mount_popup(ctx: &Context, state: &mut MountState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, DEFAULT_POPUP_WIDTH, |ui| {
        ui.label(RichText::new("Device (ex: /dev/sdb1)").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        styled_text_edit(ui, &mut state.device);
        ui.add_space(8.0);
        ui.label(RichText::new("Mount point").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        styled_text_edit(ui, &mut state.mount_point);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if popup_button(ui, "Mount") {
                action = ModalAction::Mount { device: state.device.clone(), mount_point: state.mount_point.clone() };
            }
            if popup_button(ui, "Cancel") {
                action = ModalAction::Close;
            }
        });
    });
    action
}

fn show_run_popup(ctx: &Context, state: &mut RunState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, DEFAULT_POPUP_WIDTH, |ui| {
        ui.label(
            RichText::new(state.path.display().to_string()).color(Color32::BLACK).strong().size(theme::SMALL_TEXT_SIZE),
        );
        ui.add_space(6.0);
        ui.label(RichText::new("Arguments (optional)").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        styled_text_edit(ui, &mut state.args);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if popup_button(ui, "Run") {
                action = ModalAction::RunProgram { path: state.path.clone(), args: state.args.clone() };
            }
            if popup_button(ui, "Cancel") {
                action = ModalAction::Close;
            }
        });
    });
    action
}

fn show_permissions_popup(ctx: &Context, state: &mut PermissionsState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, DEFAULT_POPUP_WIDTH, |ui| {
        let label = if state.paths.len() == 1 {
            state.paths[0].display().to_string()
        } else {
            format!("{} items selected", state.paths.len())
        };
        ui.label(RichText::new(label).color(Color32::BLACK).strong().size(theme::SMALL_TEXT_SIZE));
        ui.add_space(8.0);

        #[cfg(unix)]
        {
            for (row_label, bits) in
                [("Owner", &mut state.owner), ("Group", &mut state.group), ("Other", &mut state.other)]
            {
                ui.label(RichText::new(row_label).color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut bits[0], RichText::new("Read").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
                    ui.checkbox(&mut bits[1], RichText::new("Write").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
                    ui.checkbox(&mut bits[2], RichText::new("Execute").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
                });
                ui.add_space(6.0);
            }
        }
        #[cfg(not(unix))]
        {
            ui.checkbox(
                &mut state.read_only,
                RichText::new("Read-only").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE),
            );
            ui.add_space(6.0);
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if popup_button(ui, "OK") {
                action = ModalAction::ApplyPermissions(state.clone());
            }
            if popup_button(ui, "Cancel") {
                action = ModalAction::Close;
            }
        });
    });
    action
}

fn show_find_popup(ctx: &Context, state: &mut FindState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, DEFAULT_POPUP_WIDTH, |ui| {
        ui.label(RichText::new("Find (e.g. *.txt, photo?.jpg)").color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        styled_text_edit(ui, &mut state.pattern);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if popup_button(ui, "Find") {
                action = ModalAction::RunFind { for_left: state.for_left, pattern: state.pattern.clone() };
            }
            if popup_button(ui, "Cancel") {
                action = ModalAction::Close;
            }
        });
    });
    action
}

fn show_find_results_popup(ctx: &Context, state: &FindResultsState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, RESULTS_POPUP_WIDTH, |ui| {
        let heading = if state.matches.is_empty() {
            "No matches found.".to_owned()
        } else if state.truncated {
            format!(
                "{}+ match(es) found (showing the first {}) — refine your search. Click one to jump there:",
                state.matches.len(),
                state.matches.len()
            )
        } else {
            format!("{} match(es) — click one to jump there:", state.matches.len())
        };
        ui.label(RichText::new(heading).color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        ui.add_space(6.0);
        ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            for path in &state.matches {
                if popup_result_row(ui, &path.display().to_string()) {
                    action = ModalAction::JumpTo { for_left: state.for_left, path: path.clone() };
                }
            }
        });
        ui.add_space(10.0);
        if popup_button(ui, "Close") {
            action = ModalAction::Close;
        }
    });
    action
}

fn show_search_popup(ctx: &Context, state: &mut SearchState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, DEFAULT_POPUP_WIDTH, |ui| {
        let label = format!(
            "Search in {} selected file(s) (regex)",
            state.paths.len()
        );
        ui.label(RichText::new(label).color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        styled_text_edit(ui, &mut state.pattern);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if popup_button(ui, "Search") {
                action = ModalAction::RunSearch { paths: state.paths.clone(), pattern: state.pattern.clone() };
            }
            if popup_button(ui, "Cancel") {
                action = ModalAction::Close;
            }
        });
    });
    action
}

fn show_search_results_popup(ctx: &Context, state: &SearchResultsState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, RESULTS_POPUP_WIDTH, |ui| {
        let heading = if state.matches.is_empty() {
            "No matches found.".to_owned()
        } else if state.truncated {
            format!(
                "{}+ line(s) found (showing the first {}) — refine your search. Click one to view its file:",
                state.matches.len(),
                state.matches.len()
            )
        } else {
            format!("{} line(s) — click one to view its file:", state.matches.len())
        };
        ui.label(RichText::new(heading).color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE));
        ui.add_space(6.0);
        ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            for hit in &state.matches {
                let text = format!("{}:{}: {}", hit.path.display(), hit.line_number, hit.line);
                if popup_result_row(ui, &text) {
                    action = ModalAction::ViewFile { path: hit.path.clone(), line: Some(hit.line_number) };
                }
            }
        });
        ui.add_space(10.0);
        if popup_button(ui, "Close") {
            action = ModalAction::Close;
        }
    });
    action
}

fn show_view_file_popup(ctx: &Context, state: &mut ViewFileState, window_rect: egui::Rect) -> ModalAction {
    let mut action = ModalAction::None;
    show_fullscreen_frame(ctx, window_rect, |ui| {
        ui.label(
            RichText::new(state.path.display().to_string()).color(Color32::BLACK).strong().size(theme::SMALL_TEXT_SIZE),
        );
        ui.add_space(6.0);

        let close_row_height = theme::BUTTON_ROW_HEIGHT + 10.0;
        let content_height = (ui.available_height() - close_row_height).max(0.0);
        let number_width = state.lines.len().max(1).to_string().len();

        ScrollArea::both().max_height(content_height).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            for (idx, line) in state.lines.iter().enumerate() {
                let line_no = idx + 1;
                let text = format!("{line_no:>number_width$} | {line}");
                let resp = ui.add(
                    egui::Label::new(RichText::new(text).color(Color32::BLACK).monospace().size(theme::LIST_TEXT_SIZE))
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
                if !state.scrolled && state.highlight_line == Some(line_no) {
                    resp.scroll_to_me(Some(egui::Align::Center));
                    state.scrolled = true;
                }
            }
            if state.truncated {
                ui.label(
                    RichText::new("(file truncated — showing the first lines only)")
                        .color(Color32::BLACK)
                        .italics()
                        .size(theme::SMALL_TEXT_SIZE),
                );
            }
        });

        ui.add_space(8.0);
        if popup_button(ui, "Close") {
            action = ModalAction::Close;
        }
    });
    action
}

fn show_confirm_delete_popup(ctx: &Context, state: &mut ConfirmDeleteState) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, DEFAULT_POPUP_WIDTH, |ui| {
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

fn show_error_popup(ctx: &Context, message: &str) -> ModalAction {
    let mut action = ModalAction::None;
    popup_frame(ctx, ERROR_POPUP_WIDTH, |ui| {
        ui.label(RichText::new("Error").color(Color32::BLACK).strong().size(theme::SMALL_TEXT_SIZE));
        ui.add_space(4.0);
        ui.add(
            egui::Label::new(RichText::new(message).color(Color32::BLACK).size(theme::SMALL_TEXT_SIZE)).wrap(),
        );
        ui.add_space(10.0);
        if popup_button(ui, "OK") {
            action = ModalAction::Close;
        }
    });
    action
}

/// Affiche le pop-up correspondant à l'état courant du modal. `window_rect`
/// n'est utilisé que par le visualisateur de fichier ("plein écran").
pub fn show_modal(ctx: &Context, modal: &mut Modal, window_rect: egui::Rect) -> ModalAction {
    match modal {
        Modal::EditDriveSlot(edit) => show_edit_drive_slot_popup(ctx, edit),
        Modal::Rename(state) => show_rename_popup(ctx, state),
        Modal::MakeDir(state) => show_makedir_popup(ctx, state),
        Modal::Mount(state) => show_mount_popup(ctx, state),
        Modal::Run(state) => show_run_popup(ctx, state),
        Modal::Permissions(state) => show_permissions_popup(ctx, state),
        Modal::Find(state) => show_find_popup(ctx, state),
        Modal::FindResults(state) => show_find_results_popup(ctx, state),
        Modal::Search(state) => show_search_popup(ctx, state),
        Modal::SearchResults(state) => show_search_results_popup(ctx, state),
        Modal::ViewFile(state) => show_view_file_popup(ctx, state, window_rect),
        Modal::ConfirmDelete(state) => show_confirm_delete_popup(ctx, state),
        Modal::Error(message) => show_error_popup(ctx, message),
    }
}
