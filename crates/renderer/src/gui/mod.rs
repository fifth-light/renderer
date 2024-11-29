use std::sync::mpsc::Sender;

use connect::{connect, connecting, ConnectParam, ConnectionStatus};
use egui::Context;
use entity::entities;
use error::error_dialog;
use glam::Vec3;
use light::light_param;
use perf::perf_info;
use renderer_perf_tracker::PerformanceTracker;
use web_time::Instant;

use crate::{
    client::world::Entities,
    renderer::{camera::PositionController, uniform::light::GlobalLightParam, Renderer},
    transport::TransportParam,
};

pub mod connect;
mod entity;
mod error;
pub mod event;
mod light;
mod perf;
pub(crate) mod state;

pub struct GuiState<CP: ConnectParam> {
    errors: Vec<String>,
    selected_param: usize,
    connect_params: Vec<CP>,
}

impl<CP: ConnectParam> Default for GuiState<CP> {
    fn default() -> Self {
        Self {
            errors: Vec::default(),
            selected_param: 0,
            connect_params: Vec::default(),
        }
    }
}

impl<CP: ConnectParam> GuiState<CP> {
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error)
    }
}

pub enum GuiAction {
    SetLightParam(GlobalLightParam),
    SetBackgroundColor(Vec3),
    Connect(Box<dyn TransportParam>),
}

pub struct GuiParam<'a> {
    pub time: &'a Instant,
    pub renderer: &'a Renderer,
    pub perf_tracker: &'a PerformanceTracker,
    pub position_controller: &'a mut PositionController,
    pub connection_status: Option<ConnectionStatus>,
    pub entities: Option<&'a Entities>,
    pub gui_actions_tx: &'a mut Sender<GuiAction>,
}

pub fn gui_main<CP: ConnectParam>(ctx: &Context, param: GuiParam, state: &mut GuiState<CP>) {
    perf_info(ctx, param.perf_tracker);
    light_param(ctx, param.renderer, param.gui_actions_tx);
    if let Some(connection_status) = param.connection_status {
        match connection_status {
            ConnectionStatus::Connecting
            | ConnectionStatus::Handshaking
            | ConnectionStatus::SyncingWorld { .. } => {
                connecting(ctx, connection_status);
            }
            ConnectionStatus::Connected | ConnectionStatus::Closed => {}
        }
    } else {
        connect(
            ctx,
            &mut state.selected_param,
            &mut state.connect_params,
            param.gui_actions_tx,
        );
    }
    if let Some(current_entities) = param.entities {
        entities(ctx, current_entities);
    }

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
