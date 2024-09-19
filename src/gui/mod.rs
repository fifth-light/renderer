use std::{path::PathBuf, sync::mpsc::Sender, time::Instant};

use animation::animation_items;
use egui::Context;
use error::error_dialog;
use load::model_load;
use node_tree::node_tree;
use perf::perf_info;

use crate::{perf::PerformanceTracker, renderer::Renderer};

mod animation;
mod context;
mod error;
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
    StopAnimation(usize),
    StartAnimationOnce(usize),
    StartAnimationRepeat(usize),
    StartAnimationLoop(usize),
    EnableCamera(Option<usize>),
}

pub fn gui_main(
    ctx: &Context,
    time: &Instant,
    renderer: &Renderer,
    perf_tracker: &PerformanceTracker,
    state: &mut GuiState,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    node_tree(ctx, renderer, gui_actions_tx);
    perf_info(ctx, perf_tracker);
    model_load(ctx, gui_actions_tx);
    animation_items(ctx, time, renderer.animation_groups(), gui_actions_tx);

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
