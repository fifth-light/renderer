use egui::{Grid, Ui};
use glam::Mat4;

use std::hash::Hash;

pub fn matrix_label(ui: &mut Ui, id: impl Hash, matrix: &Mat4) {
    Grid::new(id).show(ui, |ui| {
        for row in 0..4 {
            for col in 0..4 {
                ui.label(format!("{:#.2}", matrix.row(row)[col]));
            }
            ui.end_row()
        }
    });
}
