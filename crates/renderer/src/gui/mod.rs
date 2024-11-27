use std::sync::mpsc::Sender;

use egui::Context;
use error::error_dialog;
use glam::Vec3;
use light::light_param;
use perf::perf_info;
use renderer_perf_tracker::PerformanceTracker;
use web_time::Instant;

use crate::renderer::{camera::PositionController, uniform::light::GlobalLightParam, Renderer};

mod error;
pub mod event;
mod light;
mod perf;
pub(crate) mod state;

#[derive(Default)]
pub struct GuiState {
    errors: Vec<String>,
}

impl GuiState {
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error)
    }
}

#[derive(Debug, Clone)]
pub enum GuiAction {
    SetLightParam(GlobalLightParam),
    SetBackgroundColor(Vec3),
}

pub struct GuiParam<'a> {
    pub time: &'a Instant,
    pub renderer: &'a Renderer,
    pub perf_tracker: &'a PerformanceTracker,
    pub position_controller: &'a mut PositionController,
    pub gui_actions_tx: &'a mut Sender<GuiAction>,
}

pub fn gui_main(ctx: &Context, param: GuiParam, state: &mut GuiState) {
    perf_info(ctx, param.perf_tracker);
    light_param(ctx, param.renderer, param.gui_actions_tx);

    let mut remove_index = Vec::new();
    for (index, error) in state.errors.iter().enumerate() {
        error_dialog(ctx, index, error, || {
            remove_index.push(index);
        });
    }
    for index in remove_index.into_iter().rev() {
        state.errors.remove(index);
    }
}
