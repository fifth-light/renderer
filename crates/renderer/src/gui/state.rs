use std::{
    sync::{
        mpsc::{self, TryRecvError},
        Arc, Mutex,
    },
    time::Instant,
};

use egui::FullOutput;
use egui_wgpu::Renderer as EguiRenderer;
use renderer_perf_tracker::PerformanceTracker;
use wgpu::{Device, SurfaceConfiguration};

use crate::{
    client::Client,
    renderer::{camera::PositionController, Renderer, DEPTH_TEXTURE_FORMAT},
};

use super::{
    connect::ConnectParam, event::GuiEventHandler, gui_main, GuiAction, GuiParam, GuiState,
};

pub(crate) struct EguiState<CP: ConnectParam> {
    pub renderer: EguiRenderer,
    pub event_handler: Arc<Mutex<dyn GuiEventHandler>>,
    pub state: GuiState<CP>,
    pub gui_actions_tx: mpsc::Sender<GuiAction>,
    pub gui_actions_rx: mpsc::Receiver<GuiAction>,
}

impl<CP: ConnectParam> EguiState<CP> {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        event_handler: Arc<Mutex<dyn GuiEventHandler>>,
    ) -> Self {
        let egui_renderer =
            EguiRenderer::new(device, config.format, Some(DEPTH_TEXTURE_FORMAT), 1, false);

        let (gui_actions_tx, gui_actions_rx) = mpsc::channel();
        Self {
            renderer: egui_renderer,
            event_handler,
            state: GuiState::default(),
            gui_actions_tx,
            gui_actions_rx,
        }
    }

    pub fn recv_events(&mut self) -> Result<GuiAction, TryRecvError> {
        self.gui_actions_rx.try_recv()
    }

    pub fn run<'a>(
        &'a mut self,
        renderer: &'a Renderer,
        perf_tracker: &'a PerformanceTracker,
        client: Option<&'a Client>,
        position_controller: &'a mut PositionController,
        start_time: &'a Instant,
    ) -> FullOutput {
        let mut event_handler = self.event_handler.lock().unwrap();
        let input = event_handler.take_egui_input();
        event_handler.egui_context().run(input, |ctx| {
            let connection_status = client.map(Client::connection_status);
            gui_main(
                ctx,
                GuiParam {
                    time: start_time,
                    renderer,
                    perf_tracker,
                    position_controller,
                    connection_status,
                    entities: client.and_then(Client::world).map(|world| &world.entities),
                    gui_actions_tx: &mut self.gui_actions_tx,
                },
                &mut self.state,
            );
        })
    }
}
