mod app;
mod button_bar;
mod config;
mod disk_info;
mod file_ops;
mod grep;
mod panel;
mod popups;
mod search;
mod status_bar;
mod theme;

use app::DirectoryOctopusApp;

fn main() -> eframe::Result<()> {
    let min_size = [theme::logical_width(), theme::logical_height()];
    let default_size = [min_size[0] * 1.15, min_size[1] * 1.4];

    // Icône unique (bureau et fenêtre) : dérivée de assets/icon.png (qui
    // reste la référence, inchangée), sans le texte.
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-window.png"))
        .expect("l'icône embarquée doit être un PNG valide");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Directory Octopus")
            .with_inner_size(default_size)
            .with_min_inner_size(min_size)
            .with_icon(icon)
            // Doit correspondre au nom de directory-octopus.desktop pour que
            // l'environnement de bureau (Wayland/xdg-shell, X11 WM_CLASS)
            // associe correctement l'icône à la fenêtre.
            .with_app_id("directory-octopus")
            // Démarrer maximisé donne le plus d'espace possible aux panneaux
            // de fichiers ; le niveau de zoom lui-même est un preset fixe
            // (X1/X2/X3 via F1/F2/F3, cf. app.rs), pas une échelle recalculée
            // automatiquement à partir de la taille d'écran.
            .with_maximized(true),
        ..Default::default()
    };

    eframe::run_native(
        "Directory Octopus",
        native_options,
        Box::new(|cc| {
            theme::install_fonts(&cc.egui_ctx);
            Ok(Box::new(DirectoryOctopusApp::default()))
        }),
    )
}
