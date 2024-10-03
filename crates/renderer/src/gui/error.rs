use egui::{Align2, Context, Id, Window};

pub fn error_dialog(ctx: &Context, id: usize, message: &str, on_dismiss: impl FnOnce()) {
    Window::new("Error")
        .id(Id::new(id).with(message))
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(message);
            if ui.button("Dismiss").clicked() {
                on_dismiss()
            }
        });
}
