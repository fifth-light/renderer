pub mod asset;
pub mod perf;
pub mod renderer;
#[cfg(feature = "winit")]
pub mod winit;

use asset::{
    loader::{self, obj::ObjLoader, pmx::load_pmx, AssetLoadParams},
    node::DecomposedTransform,
};
use glam::{EulerRot, Quat, Vec3};
use log::warn;
use perf::PerformanceTracker;
use renderer::{
    camera::{CameraProjection, PositionController},
    loader::RendererAssetLoader,
    node::{
        crosshair::CrosshairNode,
        light::{LightNode, LightParam},
        transform::TransformNode,
        RenderNodeItem,
    },
    pipeline::Pipelines,
    OngoingRenderState, Renderer,
};
use std::{f32::consts::PI, path::PathBuf, sync::Arc};
use web_time::Instant;
use wgpu::{
    rwh::{HasDisplayHandle, HasWindowHandle},
    util::{backend_bits_from_env, initialize_adapter_from_env, power_preference_from_env},
    Adapter, Backends, Device, DeviceDescriptor, Instance, InstanceDescriptor, Limits,
    PowerPreference, PresentMode, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration,
    SurfaceError, TextureFormat, TextureUsages, TextureViewDescriptor, WasmNotSendSync,
};

#[cfg(feature = "gui")]
#[path = ""]
mod reexport_egui {
    pub mod gui;
    pub use egui;
    pub use egui_wgpu;
}
#[cfg(feature = "gui")]
pub use reexport_egui::*;

#[cfg(feature = "gui")]
struct EguiState<EventHandler: gui::event::GuiEventHandler> {
    active: bool,
    renderer: egui_wgpu::Renderer,
    event_handler: Arc<std::sync::Mutex<EventHandler>>,
    state: gui::GuiState,
    gui_actions_tx: std::sync::mpsc::Sender<gui::GuiAction>,
    gui_actions_rx: std::sync::mpsc::Receiver<gui::GuiAction>,
    model_loader: Arc<dyn gui::ModelLoaderGui>,
}

#[cfg(feature = "gui")]
impl<EventHandler: gui::event::GuiEventHandler> EguiState<EventHandler> {
    fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        event_handler: Arc<std::sync::Mutex<EventHandler>>,
        model_loader: Arc<dyn gui::ModelLoaderGui>,
    ) -> Self {
        use egui_wgpu::Renderer;
        use gui::GuiState;
        use renderer::DEPTH_TEXTURE_FORMAT;
        use std::sync::mpsc;

        let egui_renderer =
            Renderer::new(device, config.format, Some(DEPTH_TEXTURE_FORMAT), 1, false);

        let (gui_actions_tx, gui_actions_rx) = mpsc::channel();
        Self {
            model_loader,
            active: cfg!(target_os = "android"),
            renderer: egui_renderer,
            event_handler,
            state: GuiState::default(),
            gui_actions_tx,
            gui_actions_rx,
        }
    }
}

pub trait TimeSource {
    fn now(&self) -> Instant;
}

pub trait RenderTarget: HasWindowHandle + HasDisplayHandle + WasmNotSendSync + 'static {
    fn native_pixels_per_point(&self) -> f32;
    fn pre_present_notify(&self);
    fn request_redraw(&self);
}

#[derive(Debug, Clone, Copy)]
pub enum RenderResult {
    Succeed,
    NoSurface,
    SurfaceLost,
}

pub struct State<'a, #[cfg(feature = "gui")] EventHandler: gui::event::GuiEventHandler> {
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
    pipelines: Pipelines,
    pub position_controller: PositionController,
    last_render_time: Option<Instant>,
    rotation_speed: f32,
    #[cfg(feature = "gui")]
    gui_state: EguiState<EventHandler>,
}

macro_rules! impl_State {
    { impl State $implementations:tt } => {
        #[cfg(not(feature = "gui"))]
        impl<'a> State<'a> $implementations

        #[cfg(feature = "gui")]
        impl<'a, EventHandler: gui::event::GuiEventHandler> State<'a, EventHandler>
        $implementations
    }
}

impl_State! {
    impl State {
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
            #[cfg(feature = "gui")] event_handler: Arc<std::sync::Mutex<EventHandler>>,
            #[cfg(feature = "gui")] model_loader: Arc<dyn gui::ModelLoaderGui>,
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
            };
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

            #[cfg(feature = "gui")]
            let gui_state = EguiState::new(&device, &config, event_handler, model_loader);

            Self {
                instance,
                adapter,
                surface: Some(surface),
                device,
                queue,
                config,
                limits,
                size,
                perf_tracker: PerformanceTracker::default(),
                renderer,
                pipelines,
                position_controller: PositionController::default(),
                last_render_time: None,
                rotation_speed: 0.3,
                #[cfg(feature = "gui")]
                gui_state,
            }
        }

        pub fn load_obj(&mut self, path: PathBuf) {
            let mut asset_loader = RendererAssetLoader::new(
                self.renderer.state.bind_group_layout(),
                self.renderer.state.global_defaults(),
                &mut self.pipelines,
            );
            let mut obj_loader = ObjLoader::default();
            let base_dir = match path.parent() {
                Some(base_dir) => base_dir,
                None => {
                    #[cfg(feature = "gui")]
                    self.gui_state.state.add_error(format!(
                        "Failed to find a base path for \"{}\"",
                        path.to_string_lossy()
                    ));
                    return;
                }
            };
            let mesh_asset = match obj_loader.load(base_dir, &path) {
                Ok(asset) => asset,
                #[allow(unused)]
                Err(err) => {
                    #[cfg(feature = "gui")]
                    self.gui_state
                        .state
                        .add_error(format!("Load OBJ failed: {}", err));
                    return;
                }
            };
            let mesh_node = asset_loader.load_mesh(&self.device, &self.queue, mesh_asset);
            self.renderer.add_node(mesh_node);
        }

        pub fn load_gltf(&mut self, path: PathBuf, params: &AssetLoadParams) {
            let mut asset_loader = RendererAssetLoader::new(
                self.renderer.state.bind_group_layout(),
                self.renderer.state.global_defaults(),
                &mut self.pipelines,
            );
            let (scenes, animations) = match loader::gltf::load_from_path(&path, params) {
                Ok(scenes) => scenes,
                #[allow(unused)]
                Err(err) => {
                    #[cfg(feature = "gui")]
                    self.gui_state
                        .state
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

        pub fn load_pmx(&mut self, path: PathBuf) {
            let mut asset_loader = RendererAssetLoader::new(
                self.renderer.state.bind_group_layout(),
                self.renderer.state.global_defaults(),
                &mut self.pipelines,
            );
            let base_dir = match path.parent() {
                Some(base_dir) => base_dir,
                None => {
                    #[cfg(feature = "gui")]
                    self.gui_state.state.add_error(format!(
                        "Failed to find a base path for \"{}\"",
                        path.to_string_lossy()
                    ));
                    return;
                }
            };
            let scene_asset = match load_pmx(base_dir, &path) {
                Ok(asset) => asset,
                #[allow(unused)]
                Err(err) => {
                    #[cfg(feature = "gui")]
                    self.gui_state
                        .state
                        .add_error(format!("Load pmx failed: {}", err));
                    return;
                }
            };
            let node = asset_loader.load_scene(&self.device, &self.queue, scene_asset);
            self.renderer.add_node(node);
        }

        pub fn setup_scene(&mut self) {
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
                self.renderer.state.resize(&self.device, new_size);
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
            self.renderer.state.update_camera(|camera| {
                let delta = if inc { 10.0 } else { -10.0 };
                if let CameraProjection::Perspective { yfov, .. } = &mut camera.projection {
                    *yfov += delta;
                    *yfov = yfov.clamp(30.0, 120.0);
                };
            })
        }

        pub fn update_rotation(&mut self, delta: (f32, f32)) {
            self.renderer.state.update_camera(|camera| {
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

        pub fn render(&mut self, display_target: &impl RenderTarget) -> RenderResult {
            #[cfg(feature = "gui")]
            while let Ok(action) = self.gui_state.gui_actions_rx.try_recv() {
                let gui_time = Instant::now();
                use gui::GuiAction;
                use renderer::animation::AnimationState;
                match action {
                    GuiAction::LoadObj(path) => self.load_obj(path),
                    GuiAction::LoadGltf(path) => {
                        let param = self.gui_state.state.asset_load_params().clone();
                        self.load_gltf(path, &param);
                    }
                    GuiAction::LoadPmx(path) => self.load_pmx(path),
                    GuiAction::StopAnimation(id) => {
                        self.renderer
                            .set_animation_state(id, AnimationState::Stopped);
                    }
                    GuiAction::StartAnimationOnce(id) => {
                        self.renderer
                            .set_animation_state(id, AnimationState::Once(gui_time));
                    }
                    GuiAction::StartAnimationRepeat(id) => {
                        self.renderer
                            .set_animation_state(id, AnimationState::Repeat(gui_time));
                    }
                    GuiAction::StartAnimationLoop(id) => {
                        self.renderer
                            .set_animation_state(id, AnimationState::Loop(gui_time));
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
                None => return RenderResult::NoSurface,
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
                OngoingRenderState::new(&self.device, &texture_view, &self.renderer.state);

            self.renderer.render(&mut ongoing_state);

            // Egui
            #[cfg(feature = "gui")]
            {
                use gui::{gui_main, GuiParam};
                if self.gui_state.active {
                    let mut event_handler = self.gui_state.event_handler.lock().unwrap();
                    let pixels_per_point = event_handler.egui_context().zoom_factor()
                        * display_target.native_pixels_per_point();
                    let screen_descriptor = egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [self.size.0, self.size.1],
                        pixels_per_point,
                    };
                    let input = event_handler.take_egui_input();
                    let full_output = event_handler.egui_context().run(input, |ctx| {
                        gui_main(
                            ctx,
                            GuiParam {
                                time: &start_time,
                                renderer: &self.renderer,
                                model_loader: self.gui_state.model_loader.clone(),
                                perf_tracker: &self.perf_tracker,
                                position_controller: &mut self.position_controller,
                                gui_actions_tx: &mut self.gui_state.gui_actions_tx,
                            },
                            &mut self.gui_state.state,
                        );
                    });
                    event_handler.handle_platform_output(full_output.platform_output);
                    let paint_jobs = event_handler
                        .egui_context()
                        .tessellate(full_output.shapes, full_output.pixels_per_point);
                    for (id, image_delta) in &full_output.textures_delta.set {
                        self.gui_state.renderer.update_texture(
                            &self.device,
                            &self.queue,
                            *id,
                            image_delta,
                        );
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
                }
            }
            ongoing_state.finish(&self.queue);

            display_target.pre_present_notify();
            output.present();
            display_target.request_redraw();

            let end_time = Instant::now();
            let frame_time = end_time - start_time;
            self.perf_tracker.add_sample(frame_time, end_time);
            RenderResult::Succeed
        }

        #[cfg(feature = "gui")]
        pub fn egui_active(&self) -> bool {
            self.gui_state.active
        }

        #[cfg(feature = "gui")]
        pub fn set_egui_active(&mut self, active: bool) {
            self.gui_state.active = active;
        }
    }
}
