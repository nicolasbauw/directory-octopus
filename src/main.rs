mod app;
mod button_bar;
mod panel;
mod status_bar;
mod theme;

use app::DirectoryOctopusApp;

fn main() -> eframe::Result<()> {
    let min_size = [theme::logical_width(), theme::logical_height()];
    let default_size = [min_size[0] * 1.15, min_size[1] * 1.4];

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Directory Octopus")
            .with_inner_size(default_size)
            .with_min_inner_size(min_size)
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
