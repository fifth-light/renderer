use crate::{
    client::Client,
    gui::{
        connect::{ConnectParam, ConnectionStatus},
        event::GuiEventHandler,
        state::EguiState,
        GuiAction,
    },
    renderer::{
        camera::{CameraProjection, PositionController},
        pipeline::Pipelines,
        OngoingRenderState, Renderer,
    },
    RenderTarget,
};
use image::GrayImage;
use log::warn;
use renderer_perf_tracker::PerformanceTracker;
use std::sync::{Arc, Mutex};
use web_time::Instant;
use wgpu::{
    util::{backend_bits_from_env, initialize_adapter_from_env, power_preference_from_env},
    Adapter, Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, Limits,
    PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration,
    SurfaceError, TextureFormat, TextureUsages, TextureViewDescriptor,
};

#[must_use]
#[derive(Debug, Clone, Copy)]
pub enum RenderResult {
    Succeed,
    NoSurface,
    SurfaceLost,
}

pub struct State<'a, CP: ConnectParam> {
    instance: Instance,
    adapter: Adapter,
    surface: Option<Surface<'a>>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    limits: Limits,
    size: (u32, u32),

    perf_tracker: PerformanceTracker,

    renderer: Renderer,
    _pipelines: Pipelines,
    pub position_controller: PositionController,
    last_render_time: Option<Instant>,
    rotation_speed: f32,

    client: Option<Client>,
    gui_active: bool,
    gui_state: EguiState<CP>,
}

impl<'a, CP: ConnectParam> State<'a, CP> {
    fn create_config(
        surface: &Surface,
        adapter: &Adapter,
        size: (u32, u32),
    ) -> SurfaceConfiguration {
        if cfg!(target_family = "wasm") {
            surface
                .get_default_config(adapter, size.0, size.1)
                .expect("The surface is not supported by adapter")
        } else {
            let surface_caps = surface.get_capabilities(adapter);
            let surface_format = surface_caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .or_else(|| surface_caps.formats.first().copied())
                .unwrap_or(TextureFormat::Bgra8UnormSrgb);
            SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: size.0,
                height: size.1,
                present_mode: PresentMode::AutoVsync,
                alpha_mode: surface_caps.alpha_modes[0],
                desired_maximum_frame_latency: 2,
                view_formats: vec![],
            }
        }
    }

    pub async fn new(
        render_target: Arc<dyn RenderTarget>,
        size: (u32, u32),
        event_handler: Arc<Mutex<dyn GuiEventHandler>>,
    ) -> Self {
        let backends = if cfg!(target_family = "wasm") {
            Backends::GL | Backends::BROWSER_WEBGPU
        } else {
            backend_bits_from_env().unwrap_or(Backends::all())
        };
        let instance = Instance::new(InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let surface = instance
            .create_surface(render_target.clone())
            .expect("Failed to create surface");
        let adapter = initialize_adapter_from_env(&instance, Some(&surface));
        let adapter = match adapter {
            Some(adapter) => adapter,
            None => instance
                .request_adapter(&RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    power_preference: power_preference_from_env().unwrap_or(PowerPreference::None),
                    ..Default::default()
                })
                .await
                .expect("Failed to acquire a graphic adapter"),
        };
        let limits = if cfg!(target_family = "wasm") {
            Limits::downlevel_webgl2_defaults()
        } else {
            Limits::default()
        }
        .using_resolution(adapter.limits());
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Device"),
                    required_limits: limits.clone(),
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Failed to acquire a device");

        let config = Self::create_config(&surface, &adapter, size);
        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, &queue, size);
        let pipelines = Pipelines::new(&device, config.format);

        let gui_state = EguiState::new(&device, &config, event_handler);

        Self {
            instance,
            adapter,
            surface: Some(surface),
            device,
            queue,
            config,
            limits,
            size,
            perf_tracker: PerformanceTracker::new(60),
            renderer,
            _pipelines: pipelines,
            position_controller: PositionController::default(),
            last_render_time: None,
            rotation_speed: 0.3,
            client: None,
            gui_active: true,
            gui_state,
        }
    }

    fn _show_error(&mut self, error: String) {
        self.gui_state.state.add_error(error);
    }

    pub fn resize(&mut self, new_size: (u32, u32)) {
        self.size = new_size;
        self.config.width = new_size.0;
        self.config.height = new_size.1;
        if new_size.0 != 0 && new_size.1 != 0 {
            let surface = match &self.surface {
                Some(surface) => surface,
                None => return,
            };
            surface.configure(&self.device, &self.config);
            self.renderer.resize(&self.device, new_size);
        }
    }

    pub fn recreate_surface(&mut self, display_target: Arc<impl RenderTarget>) {
        if self.size.0 != 0 && self.size.1 != 0 {
            let surface = self
                .instance
                .create_surface(display_target)
                .expect("Failed to create surface");
            self.config = Self::create_config(&surface, &self.adapter, self.size);
            surface.configure(&self.device, &self.config);
            self.surface = Some(surface);
        }
    }

    pub fn update_fov(&mut self, inc: bool) {
        self.renderer.update_camera(|camera| {
            let delta = if inc { 10.0 } else { -10.0 };
            if let CameraProjection::Perspective { yfov, .. } = &mut camera.projection {
                *yfov += delta;
                *yfov = yfov.clamp(30.0, 120.0);
            };
        })
    }

    pub fn update_rotation(&mut self, delta: (f32, f32)) {
        self.renderer.update_camera(|camera| {
            let x_delta = delta.0 * self.rotation_speed;
            let y_delta = delta.1 * self.rotation_speed;
            camera.view.yaw += x_delta;
            camera.view.pitch -= y_delta;
            camera.view.pitch = camera.view.pitch.clamp(-89.0, 89.0);
        })
    }

    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    pub fn destroy_surface(&mut self) {
        self.surface = None;
    }

    pub fn dump_depth(&self) -> GrayImage {
        self.renderer.dump_depth(&self.device, &self.queue)
    }

    fn handle_gui_events(&mut self) {
        while let Ok(action) = self.gui_state.recv_events() {
            match action {
                GuiAction::SetLightParam(param) => {
                    self.renderer.set_global_light_param(param);
                }
                GuiAction::SetBackgroundColor(color) => {
                    self.renderer.set_background_color(color);
                }
                GuiAction::Connect(param) => {
                    self.client = Some(Client::new(param.connect()));
                }
            }
        }
    }

    pub fn render(&mut self, display_target: &impl RenderTarget) -> RenderResult {
        self.handle_gui_events();
        if let Some(client) = self.client.as_mut() {
            if !client.tick(&mut self.renderer, &mut self.gui_state.state) {
                self.client = None;
            }
        }

        let surface = match &self.surface {
            Some(surface) => surface,
            None => return RenderResult::NoSurface,
        };

        let start_time = Instant::now();
        if let Some(last_renderer_time) = self.last_render_time {
            let duration = start_time - last_renderer_time;
            self.renderer
                .update_camera(|camera| self.position_controller.update(duration, camera));
        }
        self.last_render_time = Some(start_time);
        self.renderer.prepare(&self.queue);

        let output = loop {
            match surface.get_current_texture() {
                Ok(output) => break output,
                Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                    warn!("Surface is lost or outdated, drop this frame.");
                    return RenderResult::SurfaceLost;
                }
                Err(SurfaceError::OutOfMemory) => {
                    panic!("Out of memory when allocating a frame.");
                }
                Err(SurfaceError::Timeout) => {
                    warn!("Timed out when allocating a frame");
                }
            }
        };
        let texture_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        let mut ongoing_state =
            OngoingRenderState::new(&self.device, &texture_view, &self.renderer);

        if let Some(world) = self.client.as_ref().and_then(Client::world) {
            self.renderer.render(&mut ongoing_state, world);
        }

        // Egui
        let full_output = self.gui_state.run(
            &self.renderer,
            &self.perf_tracker,
            self.client.as_ref(),
            &mut self.position_controller,
            &start_time,
        );
        let mut event_handler = self.gui_state.event_handler.lock().unwrap();
        let pixels_per_point =
            event_handler.egui_context().zoom_factor() * display_target.native_pixels_per_point();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.size.0, self.size.1],
            pixels_per_point,
        };
        event_handler.handle_platform_output(full_output.platform_output);
        let paint_jobs = event_handler
            .egui_context()
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.gui_state
                .renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }
        self.gui_state.renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut ongoing_state.encoder,
            &paint_jobs,
            &screen_descriptor,
        );
        self.gui_state.renderer.render(
            &mut ongoing_state.render_pass,
            &paint_jobs,
            &screen_descriptor,
        );

        ongoing_state.finish(&self.queue);

        display_target.pre_present_notify();
        output.present();
        display_target.request_redraw();

        let end_time = Instant::now();
        let frame_time = end_time - start_time;
        self.perf_tracker.add_sample(frame_time, end_time);
        RenderResult::Succeed
    }

    pub fn gui_active(&self) -> bool {
        match self.client.as_ref() {
            Some(client) => match client.connection_status() {
                ConnectionStatus::Connected => self.gui_active,
                _ => true,
            },
            None => true,
        }
    }

    pub fn toggle_gui_active(&mut self) {
        if let Some(client) = self.client.as_ref() {
            if let ConnectionStatus::Connected = client.connection_status() {
                self.gui_active = !self.gui_active
            }
        }
    }
}
