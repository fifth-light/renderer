use std::{path::PathBuf, sync::mpsc::Sender, time::Instant};

use animation::animation_items;
use egui::Context;
use error::error_dialog;
use glam::Vec3;
use joystick::joystick;
use light::light_param;
pub use load::{ModelLoaderGui, NotSupportedModelLoaderGui};
use node_tree::node_tree;
use perf::perf_info;

use crate::{
    perf::PerformanceTracker,
    renderer::{camera::PositionController, uniform::light::GlobalLightParam, Renderer},
};

mod animation;
mod context;
mod error;
mod joystick;
mod light;
mod load;
mod matrix;
mod node_tree;
mod perf;

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
    LoadObj(PathBuf),
    LoadGltf(PathBuf),
    LoadPmx(PathBuf),
    StopAnimation(usize),
    StartAnimationOnce(usize),
    StartAnimationRepeat(usize),
    StartAnimationLoop(usize),
    EnableCamera(Option<usize>),
    SetLightParam(GlobalLightParam),
    SetBackgroundColor(Vec3),
}

pub struct GuiParam<'a, ModelLoader: ModelLoaderGui> {
    pub time: &'a Instant,
    pub renderer: &'a Renderer,
    pub model_loader: &'a ModelLoader,
    pub perf_tracker: &'a PerformanceTracker,
    pub position_controller: &'a mut PositionController,
    pub gui_actions_tx: &'a mut Sender<GuiAction>,
}

pub fn gui_main<ModelLoader: ModelLoaderGui>(
    ctx: &Context,
    param: GuiParam<ModelLoader>,
    state: &mut GuiState,
) {
    node_tree(ctx, param.renderer, param.gui_actions_tx);
    perf_info(ctx, param.perf_tracker);
    param.model_loader.ui(ctx, param.gui_actions_tx);
    animation_items(
        ctx,
        param.time,
        param.renderer.animation_groups(),
        param.gui_actions_tx,
    );
    light_param(ctx, &param.renderer.state, param.gui_actions_tx);
    joystick(ctx, param.position_controller);

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