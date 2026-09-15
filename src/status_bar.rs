use egui::{RichText, Ui, Vec2};

use crate::theme;

/// Barre de statut du bas, façon "CHIP/FAST/TOTAL" original mais adaptée à Linux.
pub fn show_status_bar(ui: &mut Ui, disk_free_label: &str, date_time_label: &str) {
    let size = Vec2::new(ui.available_width(), theme::STATUS_BAR_HEIGHT - 4.0);
    ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.label(
            RichText::new(disk_free_label)
                .color(egui::Color32::BLACK)
                .size(theme::SMALL_TEXT_SIZE)
                .monospace(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for letter in ["Q", "I", "C", "F", "E", "?"] {
                ui.add(
                    egui::Button::new(
                        RichText::new(letter)
                            .color(egui::Color32::BLACK)
                            .size(theme::SMALL_TEXT_SIZE)
                            .monospace(),
                    )
                    .min_size(Vec2::new(16.0, theme::STATUS_BAR_HEIGHT - 4.0)),
                );
            }
            ui.label(
                RichText::new(date_time_label)
                    .color(egui::Color32::BLACK)
                    .size(theme::SMALL_TEXT_SIZE)
                    .monospace(),
            );
        });
    });
}
