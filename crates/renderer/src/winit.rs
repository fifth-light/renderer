#![cfg(not(target_family = "wasm"))]
use std::{cmp::Ordering, sync::Arc};

use log::{debug, info, warn};
use wgpu::rwh::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowAttributes, WindowId},
};

pub use winit;

use crate::{
    state::{RenderResult, State},
    RenderTarget,
};

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

struct WindowRenderTarget {
    window: Arc<Window>,
}

impl WindowRenderTarget {
    fn new(window: Window) -> Self {
        Self {
            window: Arc::new(window),
        }
    }
}

impl HasWindowHandle for WindowRenderTarget {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.window.window_handle()
    }
}

impl HasDisplayHandle for WindowRenderTarget {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.window.display_handle()
    }
}

impl RenderTarget for WindowRenderTarget {
    fn native_pixels_per_point(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

struct WindowEventHandler {
    window: Arc<Window>,
    egui_state: egui_winit::State,
}

impl WindowEventHandler {
    fn new(window: Arc<Window>) -> Self {
        use egui::{Context, ViewportId};
        use egui_winit::State;

        let egui_state = State::new(
            Context::default(),
            ViewportId::default(),
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        Self { window, egui_state }
    }
}

impl crate::gui::event::GuiEventHandler for WindowEventHandler {
    fn egui_context(&self) -> &egui::Context {
        self.egui_state.egui_ctx()
    }

    fn take_egui_input(&mut self) -> egui::RawInput {
        self.egui_state.take_egui_input(&self.window)
    }

    fn handle_platform_output(&mut self, platform_output: egui::PlatformOutput) {
        self.egui_state
            .handle_platform_output(&self.window, platform_output)
    }
}

pub struct App<Callback: AppCallback> {
    state: Option<State<'static, WindowEventHandler>>,
    render_target: Option<Arc<WindowRenderTarget>>,
    window_size: Option<PhysicalSize<u32>>,

    event_handler: Option<Arc<std::sync::Mutex<WindowEventHandler>>>,

    model_loader: Arc<dyn crate::gui::ModelLoaderGui>,
    callback: Callback,
}

impl<Callback: AppCallback> App<Callback> {
    pub fn run(mut callback: Callback, model_loader: Arc<dyn crate::gui::ModelLoaderGui>) {
        let mut event_loop_builder = EventLoop::with_user_event();
        callback.event_loop_building(&mut event_loop_builder);
        let event_loop = event_loop_builder
            .build()
            .expect("Failed to create event loop");

        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = Self {
            state: None,
            render_target: Default::default(),
            window_size: Default::default(),
            event_handler: None,
            model_loader,
            callback,
        };

        event_loop
            .run_app(&mut app)
            .expect("Failed to run the application");
    }
}

impl<Callback: AppCallback> App<Callback> {
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
        if self.state.is_some() {
            warn!("Request to create state when already exists");
            return;
        }
        let Some(ref mut render_target) = self.render_target else {
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

        use pollster::FutureExt;

        let event_handler = Arc::new(std::sync::Mutex::new(WindowEventHandler::new(
            render_target.window.clone(),
        )));
        self.event_handler = Some(event_handler.clone());

        let size = render_target.window.inner_size();
        let size = (size.width, size.height);
        let mut state = State::new(
            render_target.clone(),
            size,
            event_handler,
            self.model_loader.clone(),
        )
        .block_on();
        state.setup_scene();
        self.state = Some(state);
    }
}

type AppState = (Arc<Window>, State<'static, WindowEventHandler>);

impl<Callback: AppCallback> ApplicationHandler<(Arc<Window>, AppState)> for App<Callback> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        debug!("Resumed");
        let render_target = self.render_target.get_or_insert_with(|| {
            let param = WindowAttributes::default();
            let param = self.callback.window_creating(param);
            let window = event_loop
                .create_window(param)
                .expect("Failed to create window");
            self.callback.window_created(&window);
            debug!("Window created, reported size: {:?}", window.inner_size());
            Arc::new(WindowRenderTarget::new(window))
        });
        Self::update_cursor_grab(&render_target.window, true);
        match &mut self.state {
            Some(state) => {
                debug!("Recreating state");
                state.recreate_surface(render_target.clone());
            }
            None => self.create_state(),
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        debug!("Suspended");
        if let Some(state) = &mut self.state {
            state.destroy_surface();
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let Some(state) = &mut self.state else { return };

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
        let Some(render_target) = self.render_target.as_mut() else {
            debug!("Event received when window is none, event: {:?}", event);
            return;
        };
        let state = match &mut self.state {
            Some(state) => state,
            None => {
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
                match state.render(render_target.as_ref()) {
                    RenderResult::Succeed => (),
                    RenderResult::NoSurface | RenderResult::SurfaceLost => {
                        state.recreate_surface(render_target.clone());
                        render_target.request_redraw();
                    }
                }
                return;
            }

            WindowEvent::Focused(focused) => {
                let should_grab = *focused && !state.egui_active();
                Self::update_cursor_grab(&render_target.window, should_grab);
                return;
            }
            WindowEvent::Resized(new_size) => {
                if new_size.width == 0 || new_size.height == 0 {
                    debug!("Resize to zero size: {:?}", new_size);
                    return;
                }
                self.window_size = Some(*new_size);
                state.resize((new_size.width, new_size.height));
            }
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                PhysicalKey::Code(KeyCode::Escape) => {
                    event_loop.exit();
                }
                PhysicalKey::Code(KeyCode::F2) => {
                    if !event.repeat && event.state == ElementState::Released {
                        let image = state.dump_depth();
                        if let Err(err) = image.save("depth.png") {
                            warn!("Failed to write depth image: {}", err);
                        }
                    }
                }
                PhysicalKey::Code(KeyCode::F11) => {
                    if !event.repeat && event.state == ElementState::Released {
                        if render_target.window.fullscreen().is_some() {
                            render_target.window.set_fullscreen(None)
                        } else {
                            render_target
                                .window
                                .set_fullscreen(Some(Fullscreen::Borderless(None)))
                        }
                    }
                }

                PhysicalKey::Code(KeyCode::F10) => {
                    if !event.repeat && event.state == ElementState::Released {
                        let active = !state.egui_active();
                        state.set_egui_active(active);

                        let should_grab = render_target.window.has_focus() && !state.egui_active();
                        Self::update_cursor_grab(&render_target.window, should_grab);
                    }
                    return;
                }
                _ => (),
            },
            _ => (),
        }

        if state.egui_active() {
            // Since we always redraw, we can ignore the result
            let Some(event_handler) = self.event_handler.as_ref() else {
                return;
            };
            let mut event_handler = event_handler.lock().unwrap();
            let _ = event_handler
                .egui_state
                .on_window_event(&render_target.window, &event);
            return;
        }

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
                    state.position_controller.up = if event.state.is_pressed() { 1.0 } else { 0.0 };
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
