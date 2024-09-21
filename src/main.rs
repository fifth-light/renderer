use egui::{Context, ViewportId};
use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use glam::Vec3;
use log::{error, info};
use pollster::FutureExt;
use renderer::{
    asset::loader::{self, obj::ObjLoader},
    gui::{gui_main, GuiAction, GuiState},
    perf::PerformanceTracker,
    renderer::{
        animation::AnimationState,
        camera::{CameraProjection, PositionController},
        loader::RendererAssetLoader,
        node::{
            crosshair::CrosshairNode,
            light::{LightNode, LightParam},
            transform::TransformNode,
            RenderNodeItem,
        },
        pipeline::Pipelines,
        OngoingRenderState, Renderer, DEPTH_TEXTURE_FORMAT,
    },
};
use std::{
    cmp::Ordering,
    path::PathBuf,
    sync::{mpsc, Arc},
    time::Instant,
};
use wgpu::{
    util::{backend_bits_from_env, initialize_adapter_from_env, power_preference_from_env},
    Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, PowerPreference, PresentMode,
    Queue, RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceError, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{DeviceEvent, DeviceId, ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowAttributes, WindowId},
};

struct State<'a> {
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,

    perf_tracker: PerformanceTracker,

    renderer: Renderer,
    pipelines: Pipelines,
    pub position_controller: PositionController,
    last_render_time: Option<Instant>,
    rotation_speed: f32,

    egui_active: bool,
    egui_renderer: EguiRenderer,
    egui_state: EguiWinitState,
    gui_state: GuiState,
    gui_actions_tx: mpsc::Sender<GuiAction>,
    gui_actions_rx: mpsc::Receiver<GuiAction>,
}

impl<'a> State<'a> {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = Instance::new(InstanceDescriptor {
            backends: backend_bits_from_env().unwrap_or(Backends::all()),
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");
        let adapter = initialize_adapter_from_env(&instance, Some(&surface));
        let adapter = match adapter {
            Some(adapter) => adapter,
            None => instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: power_preference_from_env().unwrap_or(PowerPreference::None),
                    ..Default::default()
                })
                .await
                .expect("Failed to acquire a graphic adapter"),
        };
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Device"),
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Failed to acquire a device");
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| surface_caps.formats.first().copied())
            .unwrap_or(TextureFormat::Bgra8UnormSrgb);
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, &queue, window.inner_size());
        let pipelines = Pipelines::new(&device, config.format);

        let egui_renderer =
            EguiRenderer::new(&device, config.format, Some(DEPTH_TEXTURE_FORMAT), 1, false);
        let egui_state = EguiWinitState::new(
            Context::default(),
            ViewportId::default(),
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let (gui_actions_tx, gui_actions_rx) = mpsc::channel();

        Self {
            surface,
            device,
            queue,
            config,
            perf_tracker: PerformanceTracker::default(),
            renderer,
            pipelines,
            position_controller: PositionController::default(),
            last_render_time: None,
            rotation_speed: 0.3,
            egui_active: false,
            egui_renderer,
            egui_state,
            gui_state: GuiState::default(),
            gui_actions_tx,
            gui_actions_rx,
        }
    }

    fn load_obj(&mut self, path: PathBuf) {
        let mut asset_loader =
            RendererAssetLoader::new(self.renderer.state.bind_group_layout(), &mut self.pipelines);
        let mut obj_loader = ObjLoader::default();
        let base_dir = match path.parent() {
            Some(base_dir) => base_dir,
            None => {
                self.gui_state.add_error(format!(
                    "Failed to find a base path for \"{}\"",
                    path.to_string_lossy()
                ));
                return;
            }
        };
        let mesh_asset = match obj_loader.load_obj(base_dir, &path) {
            Ok(asset) => asset,
            Err(err) => {
                self.gui_state
                    .add_error(format!("Load OBJ failed: {}", err));
                return;
            }
        };
        let ufo_group = asset_loader.load_mesh(&self.device, &self.queue, mesh_asset);
        self.renderer.add_node(ufo_group);
    }

    fn load_gltf(&mut self, path: PathBuf) {
        let mut asset_loader =
            RendererAssetLoader::new(self.renderer.state.bind_group_layout(), &mut self.pipelines);
        let (scenes, animations) = match loader::gltf::load_from_path(&path) {
            Ok(scenes) => scenes,
            Err(err) => {
                self.gui_state
                    .add_error(format!("Load GLTF failed: {}", err));
                return;
            }
        };
        let scene_group = asset_loader.load_scenes(
            &self.device,
            &self.queue,
            scenes,
            Some(path.to_string_lossy().to_string()),
        );
        let animations = asset_loader.load_animations(animations);
        self.renderer.add_node(scene_group);
        for animation in animations {
            self.renderer.add_animation_group(animation);
        }
    }

    fn setup_scene(&mut self) {
        let mut pipelines = Pipelines::new(&self.device, self.config.format);

        let crosshair = CrosshairNode::new(
            &self.device,
            self.renderer.state.bind_group_layout(),
            &mut pipelines,
        );
        let crosshair_transform = TransformNode::from_scale(
            Vec3::splat(200.0),
            RenderNodeItem::Crosshair(Box::new(crosshair)),
        );

        let global_light = LightNode::new(
            &self.device,
            self.renderer.state.bind_group_layout(),
            &mut pipelines,
            LightParam::Parallel {
                direction: Vec3::new(2.0, 3.0, 2.0),
                color: Vec3::new(1.0, 1.0, 0.8),
                strength: 1.3,
            },
            false,
        );

        self.renderer
            .add_node(RenderNodeItem::Transform(Box::new(crosshair_transform)));
        self.renderer
            .add_node(RenderNodeItem::Light(Box::new(global_light)));
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.renderer.state.resize(&self.device, new_size);
    }

    fn update_fov(&mut self, inc: bool) {
        self.renderer.state.update_camera(|camera| {
            let delta = if inc { 10.0 } else { -10.0 };
            if let CameraProjection::Perspective { yfov, .. } = &mut camera.projection {
                *yfov += delta;
                *yfov = yfov.clamp(30.0, 120.0);
            };
        })
    }

    fn update_rotation(&mut self, delta: (f32, f32)) {
        self.renderer.state.update_camera(|camera| {
            let x_delta = delta.0 * self.rotation_speed;
            let y_delta = delta.1 * self.rotation_speed;
            camera.view.yaw += x_delta;
            camera.view.pitch -= y_delta;
            camera.view.pitch = camera.view.pitch.clamp(-89.0, 89.0);
        })
    }

    fn render(&mut self, window: &Window) {
        while let Ok(action) = self.gui_actions_rx.try_recv() {
            match action {
                GuiAction::LoadObj(path) => self.load_obj(path),
                GuiAction::LoadGltf(path) => self.load_gltf(path),
                GuiAction::StopAnimation(id) => {
                    self.renderer
                        .set_animation_state(id, AnimationState::Stopped);
                }
                GuiAction::StartAnimationOnce(id) => {
                    self.renderer
                        .set_animation_state(id, AnimationState::Once(Instant::now()));
                }
                GuiAction::StartAnimationRepeat(id) => {
                    self.renderer
                        .set_animation_state(id, AnimationState::Repeat(Instant::now()));
                }
                GuiAction::StartAnimationLoop(id) => {
                    self.renderer
                        .set_animation_state(id, AnimationState::Loop(Instant::now()));
                }
                GuiAction::EnableCamera(id) => {
                    self.renderer.state.set_enabled_camera(id);
                }
            }
        }

        let start_time = Instant::now();
        if let Some(last_renderer_time) = self.last_render_time {
            let duration = start_time - last_renderer_time;
            self.renderer
                .state
                .update_camera(|camera| self.position_controller.update(duration, camera));
        }
        self.last_render_time = Some(start_time);
        self.renderer
            .prepare(&self.device, &self.queue, &start_time);

        let output = loop {
            match self.surface.get_current_texture() {
                Ok(output) => break output,
                Err(SurfaceError::Lost) => todo!(),
                Err(SurfaceError::OutOfMemory) => {
                    error!("Out of memory when allocating a frame");
                    return;
                }
                Err(SurfaceError::Timeout) => todo!(),
                Err(SurfaceError::Outdated) => todo!(),
            }
        };
        let texture_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());
        let mut ongoing_state =
            OngoingRenderState::new(&self.device, texture_view, &self.renderer.state);

        self.renderer.render(&mut ongoing_state);

        // Egui
        if self.egui_active {
            let size = window.inner_size();
            let pixels_per_point =
                self.egui_state.egui_ctx().zoom_factor() * window.scale_factor() as f32;
            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [size.width, size.height],
                pixels_per_point,
            };
            let input = self.egui_state.take_egui_input(window);
            let full_output = self.egui_state.egui_ctx().run(input, |ctx| {
                gui_main(
                    ctx,
                    &start_time,
                    &self.renderer,
                    &self.perf_tracker,
                    &mut self.gui_state,
                    &mut self.gui_actions_tx,
                );
            });
            self.egui_state
                .handle_platform_output(window, full_output.platform_output);
            let paint_jobs = self
                .egui_state
                .egui_ctx()
                .tessellate(full_output.shapes, full_output.pixels_per_point);
            for (id, image_delta) in &full_output.textures_delta.set {
                self.egui_renderer
                    .update_texture(&self.device, &self.queue, *id, image_delta);
            }
            self.egui_renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut ongoing_state.encoder,
                &paint_jobs,
                &screen_descriptor,
            );
            self.egui_renderer.render(
                &mut ongoing_state.render_pass,
                &paint_jobs,
                &screen_descriptor,
            );
            ongoing_state.finish(&self.queue);
        } else {
            ongoing_state.finish(&self.queue);
        }

        window.pre_present_notify();
        output.present();
        window.request_redraw();

        let end_time = Instant::now();
        let frame_time = end_time - start_time;
        self.perf_tracker.add_sample(frame_time, end_time);
    }

    fn egui_active(&self) -> bool {
        self.egui_active
    }

    fn set_egui_active(&mut self, active: bool) {
        self.egui_active = active;
    }
}

#[derive(Default)]
struct App {
    state: Option<State<'static>>,
    window: Option<Arc<Window>>,
}

impl App {
    fn update_cursor_grab(window: &Window, grab: bool) {
        window.set_cursor_visible(!grab);
        if grab {
            window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                .unwrap_or_else(|err| {
                    info!("Failed to grab mouse: {:?}", err);
                });
        } else if let Err(err) = window.set_cursor_grab(CursorGrabMode::None) {
            info!("Failed to cancel grab mouse: {:?}", err);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = self.window.get_or_insert_with(|| {
            Arc::new(
                event_loop
                    .create_window(
                        WindowAttributes::default().with_inner_size(LogicalSize::new(720, 480)),
                    )
                    .expect("Failed to create window"),
            )
        });
        Self::update_cursor_grab(window, true);
        if self.state.is_none() {
            let mut state = State::new(window.clone()).block_on();
            state.setup_scene();
            self.state = Some(state);
        }
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.state = None;
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };

        if state.egui_active() {
            return;
        }

        if let DeviceEvent::MouseMotion { delta } = event {
            let (delta_y, delta_z) = delta;
            state.update_rotation((delta_y as f32, delta_z as f32));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let (window, state) = match self.window.as_ref().zip(self.state.as_mut()) {
            Some((window, state)) => (window, state),
            None => return,
        };

        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::RedrawRequested => {
                state.render(window);
                return;
            }
            WindowEvent::Focused(focused) => {
                let should_grab = *focused && !state.egui_active();
                Self::update_cursor_grab(window, should_grab);
                return;
            }
            WindowEvent::Resized(physical_size) => state.resize(*physical_size),
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    event_loop.exit();
                }
                PhysicalKey::Code(KeyCode::F11) => {
                    if !event.repeat && event.state == ElementState::Released {
                        if window.fullscreen().is_some() {
                            window.set_fullscreen(None)
                        } else {
                            window.set_fullscreen(Some(Fullscreen::Borderless(None)))
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::F10) => {
                    if !event.repeat && event.state == ElementState::Released {
                        let active = !state.egui_active();
                        state.set_egui_active(active);

                        let should_grab = window.has_focus() && !state.egui_active();
                        Self::update_cursor_grab(window, should_grab);
                    }
                    return;
                }
                _ => (),
            },
            _ => (),
        }

        if state.egui_active() {
            // Since we always redraw, we can ignore the result
            let _ = state.egui_state.on_window_event(window, &event);
        } else {
            match event {
                WindowEvent::MouseWheel { delta, .. } => {
                    let delta = match delta {
                        MouseScrollDelta::LineDelta(_, y_delta) => y_delta,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    match delta.total_cmp(&0.0) {
                        Ordering::Less => state.update_fov(true),
                        Ordering::Greater => state.update_fov(false),
                        Ordering::Equal => (),
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyW) => {
                        state.position_controller.forward = event.state.is_pressed();
                    }
                    PhysicalKey::Code(KeyCode::KeyA) => {
                        state.position_controller.left = event.state.is_pressed();
                    }
                    PhysicalKey::Code(KeyCode::KeyS) => {
                        state.position_controller.backward = event.state.is_pressed();
                    }
                    PhysicalKey::Code(KeyCode::KeyD) => {
                        state.position_controller.right = event.state.is_pressed();
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        state.position_controller.up = event.state.is_pressed();
                    }
                    PhysicalKey::Code(KeyCode::ShiftLeft) => {
                        state.position_controller.down = event.state.is_pressed();
                    }
                    _ => (),
                },
                _ => (),
            }
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop
        .run_app(&mut app)
        .expect("Failed to run the application");
}
