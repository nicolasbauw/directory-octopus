//! Palette et constantes visuelles extraites de la capture Directory Opus 4
//! (assets/dopus4.webp) par échantillonnage de pixels.

use egui::Color32;

// -- Dimensions logiques de la grille de boutons du bas --
// Ces constantes pilotent à la fois le rendu de la grille (button_bar.rs) et le
// calcul de la taille de fenêtre logique ci-dessous : elles ne doivent jamais
// diverger, sous peine de voir la grille déborder de son panneau.
pub const PANEL_MARGIN: f32 = 2.0;
// Notre police a un interligne réel de 1.6x sa taille nominale (ascendant
// 1.4em + descendant 0.2em). À taille 10, ça fait 16px : 22px de ligne laisse
// 4px de marge en haut (BUTTON_TEXT_TOP_PADDING) et 2px en bas, assez pour ne
// rogner ni le haut des majuscules ni le bas des descendantes (Copy, Play...).
pub const BUTTON_ROW_HEIGHT: f32 = 22.0;
pub const BUTTON_ROW_SPACING: f32 = 2.0;
pub const BUTTON_ROWS: usize = 6;
pub const BUTTON_WIDTH: f32 = 76.0;
pub const BUTTON_COLUMNS: usize = 7;
pub const DRIVE_LABEL_WIDTH: f32 = 78.0;

pub fn button_bar_height() -> f32 {
    PANEL_MARGIN * 2.0
        + BUTTON_ROWS as f32 * BUTTON_ROW_HEIGHT
        + (BUTTON_ROWS as f32 - 1.0) * BUTTON_ROW_SPACING
}

pub fn button_bar_width() -> f32 {
    PANEL_MARGIN * 2.0
        + DRIVE_LABEL_WIDTH
        + BUTTON_COLUMNS as f32 * BUTTON_WIDTH
        + BUTTON_COLUMNS as f32 * BUTTON_ROW_SPACING
}

// Ces trois hauteurs ont de la marge par rapport à la taille de police (11px)
// car la police Topaz Kickstart 3.0 a un interligne réel plus grand que sa
// taille nominale ; sans cette marge, le haut/bas du texte est rogné.
pub const STATUS_BAR_HEIGHT: f32 = 26.0;
pub const PANEL_HEADER_HEIGHT: f32 = 24.0;
pub const PATH_BAR_HEIGHT: f32 = 24.0;
pub const MIN_LIST_HEIGHT: f32 = 140.0;

/// Taille de police utilisée pour les libellés compacts (chemin, statut...),
/// cohérente avec le reste de la grille de boutons.
pub const SMALL_TEXT_SIZE: f32 = 11.0;

/// Hauteur d'une ligne de fichier/dossier dans les panneaux, et taille de
/// police associée. À taille 11, l'interligne réel fait 17.6px : 22px de
/// ligne laisse 2px de marge en haut et ~2.4px en bas.
pub const LIST_ROW_HEIGHT: f32 = 22.0;
pub const LIST_TEXT_SIZE: f32 = 11.0;

// Notre police a des métriques (ascendant/descendant) qui déséquilibrent le
// centrage vertical automatique d'egui pour le texte dessiné au `Painter`
// (boutons, lignes de fichiers) : le texte colle en haut sans marge. On
// l'ancre donc en haut avec un padding fixe plutôt que de le centrer — mais
// ce padding dépend de la taille de police (donc du contexte), d'où deux
// constantes distinctes plutôt qu'une seule partagée.
pub const BUTTON_TEXT_TOP_PADDING: f32 = 4.0;
pub const LIST_TEXT_TOP_PADDING: f32 = 2.0;

/// Largeur/hauteur logiques de référence (grille de dessin avant mise à l'échelle entière).
pub fn logical_width() -> f32 {
    button_bar_width()
}

pub fn logical_height() -> f32 {
    PANEL_HEADER_HEIGHT
        + MIN_LIST_HEIGHT
        + PATH_BAR_HEIGHT
        + button_bar_height()
        + STATUS_BAR_HEIGHT
}

/// Fond général des fenêtres/panneaux (gris Workbench).
pub const BG_GREY: Color32 = Color32::from_rgb(170, 170, 170);
/// Liseré clair du relief 3D (bord "en haut/à gauche" d'un élément en relief).
pub const BEVEL_LIGHT: Color32 = Color32::from_rgb(230, 230, 230);
/// Liseré sombre du relief 3D (bord "en bas/à droite" d'un élément en relief).
pub const BEVEL_DARK: Color32 = Color32::from_rgb(70, 70, 70);
pub const BEVEL_THICKNESS: f32 = 1.0;

/// Dessine un liseré 3D façon Amiga Workbench autour de `rect` : clair en
/// haut/gauche et sombre en bas/droite pour un effet "en relief" (`raised`),
/// ou l'inverse pour un effet "en creux" (zones de saisie, listes...).
pub fn draw_bevel(painter: &egui::Painter, rect: egui::Rect, raised: bool) {
    let (top_left, bottom_right) = if raised {
        (BEVEL_LIGHT, BEVEL_DARK)
    } else {
        (BEVEL_DARK, BEVEL_LIGHT)
    };
    let t = BEVEL_THICKNESS;
    // Haut
    painter.rect_filled(
        egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.right(), rect.top() + t)),
        0.0,
        top_left,
    );
    // Gauche
    painter.rect_filled(
        egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.left() + t, rect.bottom())),
        0.0,
        top_left,
    );
    // Bas
    painter.rect_filled(
        egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - t), rect.right_bottom()),
        0.0,
        bottom_right,
    );
    // Droite
    painter.rect_filled(
        egui::Rect::from_min_max(egui::pos2(rect.right() - t, rect.top()), rect.right_bottom()),
        0.0,
        bottom_right,
    );
}

/// Point blanc en haut à droite distinguant un bouton personnalisé (ajouté
/// par l'utilisateur) d'un bouton built-in.
pub fn draw_custom_marker(painter: &egui::Painter, rect: egui::Rect) {
    let center = rect.right_top() + egui::Vec2::new(-5.0, 5.0);
    painter.circle_filled(center, 2.5, Color32::WHITE);
}

/// Bandeau de titre du panneau actif.
pub const HEADER_ACTIVE_BG: Color32 = Color32::from_rgb(172, 51, 23);
/// Bandeau de titre du panneau inactif (reprend le gris de fond).
pub const HEADER_INACTIVE_BG: Color32 = BG_GREY;
pub const HEADER_TEXT: Color32 = Color32::WHITE;

// Liste de fichiers : le fond du panneau reste toujours gris (BG_GREY), qu'il
// soit actif ou non — seule la couleur de CHAQUE élément change selon qu'il
// est sélectionné ou non (jamais tout le panneau d'un coup).
/// Dossier non sélectionné : texte bleu sur le gris de fond.
/// Dossier sélectionné : fond bleu, texte blanc — même bleu dans les deux cas.
pub const LIST_BLUE: Color32 = Color32::from_rgb(0, 85, 186);
/// Fichier non sélectionné : texte noir sur le gris de fond.
pub const LIST_FILE_TEXT: Color32 = Color32::BLACK;
/// Fichier sélectionné : fond noir, texte blanc.
pub const LIST_SELECTED_FILE_BG: Color32 = Color32::BLACK;
pub const LIST_SELECTED_TEXT: Color32 = Color32::WHITE;

/// Couleurs des boutons de la grille du bas, façon Directory Opus 4.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonStyle {
    Blue,
    Purple,
    Orange,
    Black,
    Grey,
    Red,
}

impl ButtonStyle {
    pub fn colors(self) -> (Color32, Color32) {
        match self {
            ButtonStyle::Blue => (Color32::from_rgb(0, 83, 184), YELLOW_TEXT),
            ButtonStyle::Purple => (Color32::from_rgb(116, 0, 114), YELLOW_TEXT),
            ButtonStyle::Orange => (Color32::from_rgb(238, 170, 67), PURPLE_TEXT),
            ButtonStyle::Black => (Color32::BLACK, Color32::from_rgb(238, 170, 67)),
            ButtonStyle::Grey => (BG_GREY, PURPLE_TEXT),
            ButtonStyle::Red => (Color32::from_rgb(204, 35, 1), YELLOW_TEXT),
        }
    }
}

pub const YELLOW_TEXT: Color32 = Color32::from_rgb(255, 232, 42);
pub const PURPLE_TEXT: Color32 = Color32::from_rgb(91, 17, 94);

pub const FONT_REGULAR: &str = "topaz-regular";
pub const FONT_BOLD: &str = "topaz-bold";

/// Installe la police Topaz façon Kickstart 3.0 (style AmigaOS 2.0+/3.x,
/// plus arrondi que la version 1.3) comme police par défaut. Voir
/// assets/fonts/TOPAZ-LICENSE.md pour la provenance et la licence.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        FONT_REGULAR.to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/fonts/topaz_ks30_regular.ttf"
        ))
        .into(),
    );
    fonts.font_data.insert(
        FONT_BOLD.to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/fonts/topaz_ks30_bold.ttf"
        ))
        .into(),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, FONT_REGULAR.to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, FONT_REGULAR.to_owned());
    fonts
        .families
        .insert(egui::FontFamily::Name(FONT_BOLD.into()), vec![FONT_BOLD.to_owned()]);

    ctx.set_fonts(fonts);
}

/// Presets de zoom façon bytebox : facteurs d'échelle entiers sélectionnables
/// via F1/F2/F3, plutôt qu'un facteur recalculé automatiquement à partir de
/// la taille de fenêtre (qui grossissait indéfiniment la grille de boutons
/// sur les grands écrans, au détriment du nombre de fichiers visibles).
pub const ZOOM_PRESETS: [f32; 3] = [1.0, 2.0, 3.0];
pub const DEFAULT_ZOOM: f32 = ZOOM_PRESETS[1];
