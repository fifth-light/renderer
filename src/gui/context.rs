use egui::{Id, Ui};

use crate::renderer::context::Context;

use super::matrix::matrix_label;

pub fn context_label(ui: &mut Ui, root_id: usize, context: &Context) {
    matrix_label(
        ui,
        Id::new(root_id).with("Context Matrix"),
        context.transform(),
    );
}
