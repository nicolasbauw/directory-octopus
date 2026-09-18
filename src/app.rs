use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

use crate::button_bar::{show_button_bar, ButtonBarConfig};
use crate::disk_info;
use crate::file_ops;
use crate::grep;
use crate::panel::{show_panel, PanelState};
use crate::popups::{
    show_modal, ConfirmDeleteState, DriveSlotEdit, FindResultsState, FindState, MakeDirState, Modal, ModalAction,
    MountState, PermissionsState, RenameState, RunState, SearchResultLine, SearchResultsState, SearchState,
    ViewFileState,
};
use crate::search;
use crate::status_bar::show_status_bar;
use crate::theme;

/// Espacée d'une seconde : suffisante pour une horloge lisible sans relancer
/// `df`/`date` à chaque frame.
const STATUS_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

/// Plafond de lignes affichées par le visualisateur de fichier, pour rester
/// réactif même sur un très gros fichier (log, dump...).
const MAX_VIEW_LINES: usize = 20_000;

/// Découpe `content` en lignes pour le visualisateur plein écran, en coupant
/// au-delà de `MAX_VIEW_LINES` pour rester réactif.
fn lines_from_content(content: &str) -> (Vec<String>, bool) {
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    let truncated = lines.len() > MAX_VIEW_LINES;
    lines.truncate(MAX_VIEW_LINES);
    (lines, truncated)
}

/// Charge `path` pour le visualisateur plein écran, ou renvoie un pop-up
/// d'erreur si le fichier n'est pas du texte lisible.
fn load_view_file(path: PathBuf, highlight_line: Option<usize>) -> Modal {
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let (lines, truncated) = lines_from_content(&content);
            Modal::ViewFile(ViewFileState { path, lines, truncated, highlight_line, scrolled: false })
        }
        Err(err) => Modal::Error(format!("Cannot display {}:\n{err}", path.display())),
    }
}

/// Formate `bytes` façon `hexdump -C` : décalage, 16 octets en hexa (avec un
/// espace supplémentaire après le 8e), puis leur rendu ASCII entre `|...|`.
fn hex_dump_lines(bytes: &[u8]) -> Vec<String> {
    const HEX_COLUMN_WIDTH: usize = 16 * 3 + 1;
    bytes
        .chunks(16)
        .enumerate()
        .map(|(i, chunk)| {
            let mut hex = String::new();
            for (j, byte) in chunk.iter().enumerate() {
                if j == 8 {
                    hex.push(' ');
                }
                hex.push_str(&format!("{byte:02x} "));
            }
            let ascii: String =
                chunk.iter().map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' }).collect();
            format!("{:08x}  {hex:<HEX_COLUMN_WIDTH$} |{ascii}|", i * 16)
        })
        .collect()
}

/// Heuristique simple (façon `git`) : un fichier est considéré texte si ses
/// premiers octets ne contiennent aucun octet nul.
fn is_text_file(path: &Path) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => !bytes[..bytes.len().min(8192)].contains(&0),
        Err(_) => false,
    }
}

/// Extensions d'archives reconnues pour "Extract" (y compris les doubles
/// extensions comme `.tar.gz`, dont `Path::extension()` ne renvoie que `gz`).
const ARCHIVE_EXTENSIONS: &[&str] =
    &["zip", "tar", "gz", "tgz", "bz2", "tbz2", "xz", "txz", "7z", "rar", "zst", "lz", "lzma", "cab", "iso"];

fn is_archive_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ARCHIVE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

/// Charge `path` en hexadécimal pour le visualisateur plein écran ("Hex
/// Read"), ou renvoie un pop-up d'erreur si le fichier est illisible.
fn load_hex_view(path: PathBuf) -> Modal {
    match std::fs::read(&path) {
        Ok(bytes) => {
            let max_bytes = MAX_VIEW_LINES * 16;
            let truncated = bytes.len() > max_bytes;
            let lines = hex_dump_lines(&bytes[..bytes.len().min(max_bytes)]);
            Modal::ViewFile(ViewFileState { path, lines, truncated, highlight_line: None, scrolled: false })
        }
        Err(err) => Modal::Error(format!("Cannot read {}:\n{err}", path.display())),
    }
}

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

    /// Active le panneau `for_left`/`for_right` et le fait naviguer vers le
    /// dossier de `path`, en y sélectionnant `path` lui-même (utilisé par
    /// Find pour "sauter" à un résultat).
    fn jump_to(&mut self, for_left: bool, path: &Path) {
        self.left.active = for_left;
        self.right.active = !for_left;
        let Some(parent) = path.parent() else { return };
        let panel = self.panel_mut(for_left);
        panel.navigate_to(parent.to_path_buf());
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if let Some(idx) = panel.entries.iter().position(|e| e.name == name) {
                panel.selected.clear();
                panel.selected.insert(idx);
                panel.range_anchor = Some(idx);
            }
        }
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
            "makedir" => {
                self.modal = Some(Modal::MakeDir(MakeDirState { for_left: self.left.active, name: String::new() }));
            }
            "mount" => self.modal = Some(Modal::Mount(MountState::default())),
            "find" => {
                self.modal = Some(Modal::Find(FindState { for_left: self.left.active, pattern: String::new() }));
            }
            "search" => {
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let paths: Vec<PathBuf> =
                    panel.selected_entries().map(|e| dir.join(&e.name)).filter(|p| grep::is_searchable_file(p)).collect();
                self.modal = if paths.is_empty() {
                    Some(Modal::Error("Select at least one file to search in.".to_owned()))
                } else {
                    Some(Modal::Search(SearchState { paths, pattern: String::new() }))
                };
            }
            "run" => {
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let first_file = panel.selected_entries().map(|e| dir.join(&e.name)).find(|p| p.is_file());
                self.modal = match first_file {
                    Some(path) => Some(Modal::Run(RunState { path, args: String::new() })),
                    None => Some(Modal::Error("Select a file to run.".to_owned())),
                };
            }
            "read" => {
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let first_file = panel.selected_entries().map(|e| dir.join(&e.name)).find(|p| p.is_file());
                self.modal = match first_file {
                    Some(path) => Some(load_view_file(path, None)),
                    None => Some(Modal::Error("Select a text file to view.".to_owned())),
                };
            }
            "datestamp" => {
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let paths: Vec<PathBuf> = panel.selected_entries().map(|e| dir.join(&e.name)).collect();
                let now = filetime::FileTime::now();
                for path in &paths {
                    if let Err(err) = filetime::set_file_times(path, now, now) {
                        eprintln!("Datestamp de {path:?} échoué : {err}");
                    }
                }
            }
            "permissions" => {
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let paths: Vec<PathBuf> = panel.selected_entries().map(|e| dir.join(&e.name)).collect();
                self.modal = if paths.is_empty() {
                    Some(Modal::Error("Select at least one file or folder.".to_owned()))
                } else {
                    match std::fs::metadata(&paths[0]) {
                        Ok(meta) => {
                            #[cfg(unix)]
                            {
                                let mode = meta.permissions().mode();
                                let bit = |shift: u32| (mode >> shift) & 1 != 0;
                                Some(Modal::Permissions(PermissionsState {
                                    paths,
                                    owner: [bit(8), bit(7), bit(6)],
                                    group: [bit(5), bit(4), bit(3)],
                                    other: [bit(2), bit(1), bit(0)],
                                }))
                            }
                            #[cfg(not(unix))]
                            {
                                Some(Modal::Permissions(PermissionsState {
                                    paths,
                                    read_only: meta.permissions().readonly(),
                                }))
                            }
                        }
                        Err(err) => {
                            Some(Modal::Error(format!("Cannot read permissions of {}:\n{err}", paths[0].display())))
                        }
                    }
                };
            }
            "edit" => {
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let first_file = panel.selected_entries().map(|e| dir.join(&e.name)).find(|p| p.is_file());
                if let Some(path) = first_file {
                    if is_text_file(&path) {
                        if let Err(err) = open::that(&path) {
                            self.modal = Some(Modal::Error(format!("Could not open {}:\n{err}", path.display())));
                        }
                    }
                }
            }
            "upx" => {
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let first_file = panel.selected_entries().map(|e| dir.join(&e.name)).find(|p| p.is_file());
                if let Some(path) = first_file {
                    self.modal = match std::process::Command::new("upx").arg("--best").arg(&path).output() {
                        Ok(output) if output.status.success() => None,
                        Ok(output) => {
                            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                            Some(Modal::Error(format!("upx --best {} failed:\n{stderr}", path.display())))
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                            Some(Modal::Info("UPX is not installed.".to_owned()))
                        }
                        Err(err) => Some(Modal::Error(format!("Could not run upx:\n{err}"))),
                    };
                }
            }
            "extract" => {
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let first_file = panel.selected_entries().map(|e| dir.join(&e.name)).find(|p| p.is_file());
                if let Some(path) = first_file {
                    if is_archive_file(&path) {
                        if let Err(err) = open::that(&path) {
                            self.modal = Some(Modal::Error(format!("Could not open {}:\n{err}", path.display())));
                        }
                    }
                }
            }
            "hex_read" => {
                let panel = self.active_panel();
                let dir = PathBuf::from(&panel.path);
                let first_file = panel.selected_entries().map(|e| dir.join(&e.name)).find(|p| p.is_file());
                self.modal = match first_file {
                    Some(path) => Some(load_hex_view(path)),
                    None => Some(Modal::Error("Select a file to view in hex.".to_owned())),
                };
            }
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
            match show_modal(&ctx, modal, window_rect) {
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
                ModalAction::CreateDir { for_left, name } => {
                    let panel = self.panel_mut(for_left);
                    let dir = PathBuf::from(&panel.path);
                    let name = name.trim();
                    if !name.is_empty() {
                        if let Err(err) = std::fs::create_dir(dir.join(name)) {
                            eprintln!("Création du dossier {name:?} échouée : {err}");
                        }
                    }
                    panel.navigate_to(dir);
                    self.modal = None;
                }
                ModalAction::Mount { device, mount_point } => {
                    let device = device.trim();
                    let mount_point = mount_point.trim();
                    self.modal = if device.is_empty() || mount_point.is_empty() {
                        None
                    } else {
                        match std::process::Command::new("mount").arg(device).arg(mount_point).output() {
                            Ok(output) if output.status.success() => None,
                            Ok(output) => {
                                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                                let message = if stderr.is_empty() {
                                    format!("Mount of {device} on {mount_point} failed.")
                                } else {
                                    format!("Mount of {device} on {mount_point} failed:\n{stderr}")
                                };
                                Some(Modal::Error(message))
                            }
                            Err(err) => Some(Modal::Error(format!("Could not run `mount`: {err}"))),
                        }
                    };
                }
                ModalAction::RunProgram { path, args } => {
                    let dir = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
                    let mut command = std::process::Command::new(&path);
                    command.args(args.split_whitespace()).current_dir(&dir);
                    let label = if args.trim().is_empty() {
                        path.display().to_string()
                    } else {
                        format!("{} {}", path.display(), args.trim())
                    };
                    self.modal = match command.output() {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            if !stdout.trim().is_empty() {
                                let (lines, truncated) = lines_from_content(&stdout);
                                Some(Modal::ViewFile(ViewFileState {
                                    path: PathBuf::from(format!("{label} (output)")),
                                    lines,
                                    truncated,
                                    highlight_line: None,
                                    scrolled: false,
                                }))
                            } else if !output.status.success() {
                                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                                let message = if stderr.is_empty() {
                                    format!("{label} exited with an error.")
                                } else {
                                    format!("{label} failed:\n{stderr}")
                                };
                                Some(Modal::Error(message))
                            } else {
                                None
                            }
                        }
                        Err(err) => Some(Modal::Error(format!("Could not run {label}:\n{err}"))),
                    };
                }
                ModalAction::ApplyPermissions(state) => {
                    let mut errors = Vec::new();
                    for path in &state.paths {
                        #[cfg(unix)]
                        let result = {
                            let bits = |flags: [bool; 3], shift: u32| -> u32 {
                                (u32::from(flags[0]) << (shift + 2))
                                    | (u32::from(flags[1]) << (shift + 1))
                                    | (u32::from(flags[2]) << shift)
                            };
                            let mode = bits(state.owner, 6) | bits(state.group, 3) | bits(state.other, 0);
                            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
                        };
                        #[cfg(not(unix))]
                        let result = std::fs::metadata(path).and_then(|meta| {
                            let mut perms = meta.permissions();
                            perms.set_readonly(state.read_only);
                            std::fs::set_permissions(path, perms)
                        });
                        if let Err(err) = result {
                            errors.push(format!("{}: {err}", path.display()));
                        }
                    }
                    self.modal = if errors.is_empty() {
                        None
                    } else {
                        Some(Modal::Error(format!("Could not change permissions:\n{}", errors.join("\n"))))
                    };
                }
                ModalAction::RunFind { for_left, pattern } => {
                    let root = PathBuf::from(&self.panel_mut(for_left).path);
                    let pattern = pattern.trim();
                    self.modal = if pattern.is_empty() {
                        None
                    } else {
                        let outcome = search::find_matches(&root, pattern);
                        if outcome.matches.len() == 1 && !outcome.truncated {
                            self.jump_to(for_left, &outcome.matches[0]);
                            None
                        } else {
                            Some(Modal::FindResults(FindResultsState {
                                for_left,
                                matches: outcome.matches,
                                truncated: outcome.truncated,
                            }))
                        }
                    };
                }
                ModalAction::RunSearch { paths, pattern } => {
                    let pattern = pattern.trim();
                    self.modal = if pattern.is_empty() {
                        None
                    } else {
                        match grep::search_in_files(&paths, pattern) {
                            Ok(outcome) => Some(Modal::SearchResults(SearchResultsState {
                                truncated: outcome.truncated,
                                matches: outcome
                                    .matches
                                    .into_iter()
                                    .map(|m| SearchResultLine { path: m.path, line_number: m.line_number, line: m.line })
                                    .collect(),
                            })),
                            Err(err) => Some(Modal::Error(format!("Invalid search pattern: {err}"))),
                        }
                    };
                }
                ModalAction::JumpTo { for_left, path } => {
                    self.jump_to(for_left, &path);
                    self.modal = None;
                }
                ModalAction::ViewFile { path, line } => {
                    self.modal = Some(load_view_file(path, line));
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
