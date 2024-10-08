pub mod asset;
pub mod gui;
pub mod perf;
pub mod renderer;

use asset::{
    loader::{self, obj::ObjLoader, pmx::load_pmx, AssetLoadParams},
    node::DecomposedTransform,
};
use egui::{Context, ViewportId};
use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use glam::{EulerRot, Quat, Vec3};
use gui::{gui_main, GuiAction, GuiParam, GuiState, ModelLoaderGui};
use log::{debug, info, warn};
use perf::PerformanceTracker;
use renderer::{
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
};
use std::{
    cmp::Ordering,
    f32::consts::PI,
    path::PathBuf,
    sync::{mpsc, Arc},
    time::Instant,
};
use wgpu::{
    util::{initialize_adapter_from_env, power_preference_from_env},
    Adapter, Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, PowerPreference,
    PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceError,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowAttributes, WindowId},
};

pub use egui;
pub use egui_plot;
pub use egui_wgpu;
pub use winit;

struct State<'a, ModelLoader: ModelLoaderGui> {
    instance: Instance,
    adapter: Adapter,
    surface: Option<Surface<'a>>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,

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

    model_loader: Arc<ModelLoader>,
}

impl<'a, ModelLoader: ModelLoaderGui> State<'a, ModelLoader> {
    fn create_config(
        surface: &Surface,
        adapter: &Adapter,
        size: PhysicalSize<u32>,
    ) -> SurfaceConfiguration {
        if cfg!(target_family = "wasm") {
            surface
                .get_default_config(adapter, size.width, size.height)
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
                width: size.width,
                height: size.height,
                present_mode: PresentMode::AutoVsync,
                alpha_mode: surface_caps.alpha_modes[0],
                desired_maximum_frame_latency: 2,
                view_formats: vec![],
            }
        }
    }

    async fn new(window: Arc<Window>, model_loader: Arc<ModelLoader>) -> Self {
        let size = window.inner_size();

        let backends = if cfg!(target_family = "wasm") {
            Backends::GL | Backends::BROWSER_WEBGPU
        } else {
            wgpu::util::backend_bits_from_env().unwrap_or(Backends::all())
        };
        let instance = Instance::new(InstanceDescriptor {
            backends,
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
                    compatible_surface: Some(&surface),
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
                    required_limits: if cfg!(target_family = "wasm") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Failed to acquire a device");

        let config = Self::create_config(&surface, &adapter, size);

        #[cfg(not(target_family = "wasm"))]
        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, &queue, size);
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
            instance,
            adapter,
            surface: Some(surface),
            device,
            queue,
            config,
            size,
            perf_tracker: PerformanceTracker::default(),
            renderer,
            pipelines,
            position_controller: PositionController::default(),
            last_render_time: None,
            rotation_speed: 0.3,
            egui_active: cfg!(target_os = "android"),
            egui_renderer,
            egui_state,
            gui_state: GuiState::default(),
            gui_actions_tx,
            gui_actions_rx,
            model_loader,
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
        let mesh_asset = match obj_loader.load(base_dir, &path) {
            Ok(asset) => asset,
            Err(err) => {
                self.gui_state
                    .add_error(format!("Load OBJ failed: {}", err));
                return;
            }
        };
        let mesh_node = asset_loader.load_mesh(&self.device, &self.queue, mesh_asset);
        self.renderer.add_node(mesh_node);
    }

    fn load_gltf(&mut self, path: PathBuf, params: &AssetLoadParams) {
        let mut asset_loader =
            RendererAssetLoader::new(self.renderer.state.bind_group_layout(), &mut self.pipelines);
        let (scenes, animations) = match loader::gltf::load_from_path(&path, params) {
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

    fn load_pmx(&mut self, path: PathBuf) {
        let mut asset_loader =
            RendererAssetLoader::new(self.renderer.state.bind_group_layout(), &mut self.pipelines);
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
        let scene_asset = match load_pmx(base_dir, &path) {
            Ok(asset) => asset,
            Err(err) => {
                self.gui_state
                    .add_error(format!("Load pmx failed: {}", err));
                return;
            }
        };
        let node = asset_loader.load_scene(&self.device, &self.queue, scene_asset);
        self.renderer.add_node(node);
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

        let point_light = LightNode::new(
            &self.device,
            self.renderer.state.bind_group_layout(),
            &mut pipelines,
            LightParam::Parallel {
                color: Vec3::new(1.0, 1.0, 0.9),
                direction: Vec3::new(0.0, 1.0, 0.0),
                strength: 0.5,
            },
            true,
        );
        let light_transform = TransformNode::from_decomposed_transform(
            DecomposedTransform {
                translation: Vec3::new(2.0, 2.0, 2.0),
                rotation: Quat::from_euler(EulerRot::XYZ, 0.0, PI * 0.75, -PI * 0.25),
                scale: Vec3::ONE,
            },
            RenderNodeItem::Light(Box::new(point_light)),
        );

        self.renderer
            .add_node(RenderNodeItem::Transform(Box::new(crosshair_transform)));
        self.renderer
            .add_node(RenderNodeItem::Transform(Box::new(light_transform)));
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        if new_size.height != 0 && new_size.width != 0 {
            let surface = match &self.surface {
                Some(surface) => surface,
                None => return,
            };
            surface.configure(&self.device, &self.config);
            self.renderer.state.resize(&self.device, new_size);
        }
    }

    fn recreate_surface(&mut self, window: Arc<Window>) {
        if self.size.height != 0 && self.size.width != 0 {
            let surface = self
                .instance
                .create_surface(window.clone())
                .expect("Failed to create surface");
            self.config = Self::create_config(&surface, &self.adapter, self.size);
            surface.configure(&self.device, &self.config);
            self.surface = Some(surface);
        }
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

    fn destroy_surface(&mut self) {
        self.surface = None;
    }

    fn render(&mut self, window: &Window) {
        while let Ok(action) = self.gui_actions_rx.try_recv() {
            match action {
                GuiAction::LoadObj(path) => self.load_obj(path),
                GuiAction::LoadGltf(path) => {
                    let param = self.gui_state.asset_load_params().clone();
                    self.load_gltf(path, &param);
                }
                GuiAction::LoadPmx(path) => self.load_pmx(path),
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
                GuiAction::SetLightParam(param) => {
                    self.renderer.state.set_global_light_param(param);
                }
                GuiAction::SetBackgroundColor(color) => {
                    self.renderer.state.set_background_color(color);
                }
            }
        }

        let surface = match &self.surface {
            Some(surface) => surface,
            None => return,
        };

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
            match surface.get_current_texture() {
                Ok(output) => break output,
                Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                    warn!("Surface is lost or outdated, drop the frame and resize.");
                    self.resize(window.inner_size());
                    return;
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
            OngoingRenderState::new(&self.device, &texture_view, &self.renderer.state);

        self.renderer.render(&mut ongoing_state);

        // Egui
        if self.egui_active {
            let pixels_per_point =
                self.egui_state.egui_ctx().zoom_factor() * window.scale_factor() as f32;
            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [self.size.width, self.size.height],
                pixels_per_point,
            };
            let input = self.egui_state.take_egui_input(window);
            let full_output = self.egui_state.egui_ctx().run(input, |ctx| {
                gui_main(
                    ctx,
                    GuiParam {
                        time: &start_time,
                        renderer: &self.renderer,
                        model_loader: &*self.model_loader,
                        perf_tracker: &self.perf_tracker,
                        position_controller: &mut self.position_controller,
                        gui_actions_tx: &mut self.gui_actions_tx,
                    },
                    &mut self.gui_state,
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

pub trait AppCallback {
    fn event_loop_building<T: 'static>(&mut self, _event_loop_builder: &mut EventLoopBuilder<T>) {}
    fn window_creating(&mut self, param: WindowAttributes) -> WindowAttributes {
        param.with_inner_size(PhysicalSize::new(720, 480))
    }
    fn window_created(&mut self, _window: &Window) {}
}

#[derive(Default)]
pub struct NoOpAppcallCallback {}

impl AppCallback for NoOpAppcallCallback {}

#[allow(unused)]
enum MaybeState<ModelLoader: ModelLoaderGui + 'static> {
    None,
    Building,
    State(State<'static, ModelLoader>),
}

#[allow(unused)]
pub struct App<Callback, ModelLoader>
where
    Callback: AppCallback,
    ModelLoader: ModelLoaderGui + 'static,
{
    state: MaybeState<ModelLoader>,
    window: Option<Arc<Window>>,
    window_size: Option<PhysicalSize<u32>>,
    model_loader: Arc<ModelLoader>,
    event_loop_proxy: EventLoopProxy<State<'static, ModelLoader>>,
    callback: Callback,
}

impl<Callback, ModelLoader> App<Callback, ModelLoader>
where
    Callback: AppCallback + 'static,
    ModelLoader: ModelLoaderGui + 'static,
{
    pub fn run(mut callback: Callback, model_loader: ModelLoader) {
        let mut event_loop_builder = EventLoop::with_user_event();
        callback.event_loop_building(&mut event_loop_builder);
        let event_loop = event_loop_builder
            .build()
            .expect("Failed to create event loop");

        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = Self {
            state: MaybeState::None,
            window: Default::default(),
            window_size: Default::default(),
            model_loader: Arc::new(model_loader),
            event_loop_proxy: event_loop.create_proxy(),
            callback,
        };

        #[cfg(not(target_family = "wasm"))]
        event_loop
            .run_app(&mut app)
            .expect("Failed to run the application");

        #[cfg(target_family = "wasm")]
        {
            use wasm_bindgen_futures::{
                js_sys,
                wasm_bindgen::{self, prelude::*},
            };
            use wgpu::web_sys;

            wasm_bindgen_futures::spawn_local(async move {
                let run_closure = Closure::once_into_js(move || {
                    event_loop
                        .run_app(&mut app)
                        .expect("Failed to run the application");
                });

                if let Err(error) = call_catch(&run_closure) {
                    let is_control_flow_exception =
                        error.dyn_ref::<js_sys::Error>().map_or(false, |e| {
                            e.message().includes("Using exceptions for control flow", 0)
                        });

                    if !is_control_flow_exception {
                        web_sys::console::error_1(&error);
                    }
                }

                #[wasm_bindgen]
                extern "C" {
                    #[wasm_bindgen(catch, js_namespace = Function, js_name = "prototype.call.call")]
                    fn call_catch(this: &JsValue) -> Result<(), JsValue>;
                }
            });
        }
    }
}

impl<Callback, ModelLoader> App<Callback, ModelLoader>
where
    Callback: AppCallback,
    ModelLoader: ModelLoaderGui,
{
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

    fn create_state(&mut self) {
        debug!("Create state requested");
        assert!(matches!(self.state, MaybeState::None));
        let Some(window) = self.window.as_ref() else {
            debug!("Window is none, don't create state");
            return;
        };
        let Some(size) = self.window_size.as_ref() else {
            debug!("Window size is none, don't create state");
            return;
        };
        if size.width == 0 || size.height == 0 {
            debug!("Size is zero, don't create state");
            return;
        }
        debug!("Creating state");
        #[cfg(not(target_family = "wasm"))]
        {
            use pollster::FutureExt;

            let mut state = State::new(window.clone(), self.model_loader.clone()).block_on();
            state.setup_scene();
            self.state = MaybeState::State(state);
        }
        #[cfg(target_family = "wasm")]
        {
            self.state = MaybeState::Building;
            let event_loop_proxy = self.event_loop_proxy.clone();
            let state = State::new(window.clone(), self.model_loader.clone());
            wasm_bindgen_futures::spawn_local(async move {
                let mut state = state.await;
                debug!("Created state, send to event loop");
                state.setup_scene();
                if event_loop_proxy.send_event(state).is_err() {
                    warn!("Event loop is closed");
                }
            });
        }
    }
}

impl<Callback, ModelLoader> ApplicationHandler<State<'static, ModelLoader>>
    for App<Callback, ModelLoader>
where
    Callback: AppCallback,
    ModelLoader: ModelLoaderGui,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        debug!("Resumed");
        let window = self
            .window
            .get_or_insert_with(|| {
                let param = WindowAttributes::default();
                let param = self.callback.window_creating(param);
                let window = Arc::new(
                    event_loop
                        .create_window(param)
                        .expect("Failed to create window"),
                );
                self.callback.window_created(&window);
                debug!("Window created, reported size: {:?}", window.inner_size());
                window
            })
            .clone();
        Self::update_cursor_grab(&window, true);
        match &mut self.state {
            MaybeState::None => self.create_state(),
            MaybeState::Building => {
                debug!("State is already building");
            }
            MaybeState::State(state) => {
                debug!("Recreating state");
                state.recreate_surface(window.clone());
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        debug!("Suspended");
        match &mut self.state {
            MaybeState::State(state) => {
                state.destroy_surface();
            }
            MaybeState::Building => {
                self.state = MaybeState::None;
            }
            MaybeState::None => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let state = match &mut self.state {
            MaybeState::State(state) => state,
            _ => return,
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
        let window = match self.window.as_ref() {
            Some(window) => window,
            None => {
                debug!("Event received when window is none, event: {:?}", event);
                return;
            }
        };
        let state = match &mut self.state {
            MaybeState::Building => return,
            MaybeState::State(state) => state,
            MaybeState::None => {
                if let WindowEvent::Resized(new_size) = event {
                    debug!(
                        "Resized event received when state is none, new size: {:?}",
                        new_size
                    );
                    self.window_size = Some(new_size);
                    self.create_state();
                } else {
                    debug!("Event received when state is none: {:?}", event);
                }
                return;
            }
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
            WindowEvent::Resized(new_size) => {
                self.window_size = Some(*new_size);
                state.resize(*new_size);
            }
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    event_loop.exit();
                }
                PhysicalKey::Code(KeyCode::F2) => {
                    if !event.repeat && event.state == ElementState::Released {
                        let image = state.renderer.state.dump_depth(&state.device, &state.queue);
                        if let Err(err) = image.save("depth.png") {
                            warn!("Failed to write depth image: {}", err);
                        }
                    }
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
                        state.position_controller.forward =
                            if event.state.is_pressed() { 1.0 } else { 0.0 };
                    }
                    PhysicalKey::Code(KeyCode::KeyA) => {
                        state.position_controller.left =
                            if event.state.is_pressed() { 1.0 } else { 0.0 };
                    }
                    PhysicalKey::Code(KeyCode::KeyS) => {
                        state.position_controller.backward =
                            if event.state.is_pressed() { 1.0 } else { 0.0 };
                    }
                    PhysicalKey::Code(KeyCode::KeyD) => {
                        state.position_controller.right =
                            if event.state.is_pressed() { 1.0 } else { 0.0 };
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        state.position_controller.up =
                            if event.state.is_pressed() { 1.0 } else { 0.0 };
                    }
                    PhysicalKey::Code(KeyCode::ShiftLeft) => {
                        state.position_controller.down =
                            if event.state.is_pressed() { 1.0 } else { 0.0 };
                    }
                    _ => (),
                },
                _ => (),
            }
        }
    }

    #[cfg(target_family = "wasm")]
    fn user_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        mut state: State<'static, ModelLoader>,
    ) {
        debug!("Received created state");
        match self.state {
            MaybeState::Building => (),
            MaybeState::State(_) => {
                warn!("State is created when state is created");
                return;
            }
            MaybeState::None => {
                warn!("State is created when state is none");
                return;
            }
        }
        if let Some(window_size) = self.window_size.as_ref() {
            state.resize(*window_size);
        }
        self.state = MaybeState::State(state);
    }
}
