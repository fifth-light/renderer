use std::sync::mpsc::Sender;

use egui::{Align2, Context, Window};

use super::GuiAction;

pub trait ModelLoaderGui {
    fn ui(&self, ctx: &Context, gui_actions_tx: &mut Sender<GuiAction>);
}

#[derive(Default)]
pub struct NotSupportedModelLoaderGui {}

impl ModelLoaderGui for NotSupportedModelLoaderGui {
    fn ui(&self, ctx: &Context, _gui_actions_tx: &mut Sender<GuiAction>) {
        Window::new("Load Model")
            .resizable([false, false])
            .pivot(Align2::RIGHT_TOP)
            .show(ctx, |ui| {
                ui.label("Loading model is not supported for now.");
            });
    }
}