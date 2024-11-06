#![cfg(target_family = "wasm")]

use std::{ffi::c_void, ptr::NonNull, sync::Arc};

use log::{info, Level};
use renderer::{RenderTarget, State};
use wasm_bindgen::prelude::*;
use web_sys::{js_sys::Function, window, HtmlCanvasElement};
use wgpu::rwh::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
    WebCanvasWindowHandle, WindowHandle,
};

#[cfg(feature = "gui")]
mod gui;

struct CanvasRenderTarget {
    canvas: HtmlCanvasElement,
    redraw_handler: Function,
}

// SAFETY: Threads are NEVER used in the web, so it's safe to implement these trait
unsafe impl Send for CanvasRenderTarget {}
unsafe impl Sync for CanvasRenderTarget {}

impl CanvasRenderTarget {
    pub fn new(canvas: HtmlCanvasElement, redraw_handler: Function) -> Self {
        Self {
            canvas,
            redraw_handler,
        }
    }
}

impl HasWindowHandle for CanvasRenderTarget {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let canvas: NonNull<c_void> = NonNull::from(&self.canvas).cast();
        let raw_handle: RawWindowHandle = WebCanvasWindowHandle::new(canvas).into();
        // SAFETY: as the HtmlCanvasElement will always be available, it is safe to create a static handle to it
        Ok(unsafe { WindowHandle::borrow_raw(raw_handle) })
    }
}

impl HasDisplayHandle for CanvasRenderTarget {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::web())
    }
}

impl RenderTarget for CanvasRenderTarget {
    fn native_pixels_per_point(&self) -> f32 {
        // TODO
        1.0
    }

    fn pre_present_notify(&self) {
        // TODO
    }

    fn request_redraw(&self) {
        self.redraw_handler
            .call0(&JsValue::null())
            .expect("Failed to call redraw");
    }
}

#[cfg(not(feature = "gui"))]
type RendererState = State<'static>;

#[cfg(feature = "gui")]
type RendererState = State<'static, WebEventHandler>;

#[wasm_bindgen]
pub struct StateHolder {
    state: RendererState,
    render_target: Arc<CanvasRenderTarget>,
    #[cfg(feature = "gui")]
    event_handler: Arc<std::sync::Mutex<WebEventHandler>>,
}

#[wasm_bindgen]
impl StateHolder {
    fn new(
        state: RendererState,
        render_target: Arc<CanvasRenderTarget>,
        #[cfg(feature = "gui")] event_handler: Arc<std::sync::Mutex<WebEventHandler>>,
    ) -> Self {
        Self {
            state,
            render_target,
            #[cfg(feature = "gui")]
            event_handler,
        }
    }

    pub fn render(&mut self) {
        self.state.render(self.render_target.as_ref());
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let new_size = (width, height);
        #[cfg(feature = "gui")]
        {
            let mut event_handler = self.event_handler.lock().unwrap();
            event_handler.resize(new_size);
        }
        self.state.resize(new_size);
    }

    pub fn update_fov(&mut self, inc: bool) {
        self.state.update_fov(inc);
    }

    pub fn set_forward(&mut self, forward: f32) {
        self.state.position_controller.forward = forward;
    }

    pub fn set_backward(&mut self, backward: f32) {
        self.state.position_controller.backward = backward;
    }

    pub fn set_left(&mut self, left: f32) {
        self.state.position_controller.left = left;
    }

    pub fn set_right(&mut self, right: f32) {
        self.state.position_controller.right = right;
    }

    pub fn set_up(&mut self, up: f32) {
        self.state.position_controller.up = up;
    }

    pub fn set_down(&mut self, down: f32) {
        self.state.position_controller.down = down;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.state.position_controller.speed = speed;
    }

    pub fn update_rotation(&mut self, delta_x: f32, delta_y: f32) {
        self.state.update_rotation((delta_x, delta_y));
    }

    pub fn egui_active(&self) -> bool {
        #[cfg(not(feature = "gui"))]
        return false;

        #[cfg(feature = "gui")]
        return self.state.egui_active();
    }

    pub fn set_egui_active(&mut self, active: bool) {
        #[cfg(feature = "gui")]
        self.state.set_egui_active(active);
    }

    pub fn focused(&mut self) {}

    pub fn click(&mut self) {}
}

#[cfg(feature = "gui")]
struct WebEventHandler {
    egui_context: renderer::egui::Context,
    raw_input: renderer::egui::RawInput,
}

#[cfg(feature = "gui")]
impl WebEventHandler {
    fn new(size: (u32, u32)) -> Self {
        use renderer::egui::{Pos2, RawInput, Rect};

        let raw_input = RawInput {
            screen_rect: Some(Rect {
                min: Pos2::ZERO,
                max: Pos2 {
                    x: size.0 as f32,
                    y: size.1 as f32,
                },
            }),
            ..Default::default()
        };

        Self {
            egui_context: Default::default(),
            raw_input,
        }
    }

    pub fn update_max_texture_side(&mut self, max_texture_side: usize) {
        self.raw_input.max_texture_side = Some(max_texture_side);
    }

    pub fn resize(&mut self, new_size: (u32, u32)) {
        use renderer::egui::{Pos2, Rect};

        self.raw_input.screen_rect = Some(Rect {
            min: Pos2::ZERO,
            max: Pos2 {
                x: new_size.0 as f32,
                y: new_size.1 as f32,
            },
        });
    }
}

#[cfg(feature = "gui")]
impl renderer::gui::event::GuiEventHandler for WebEventHandler {
    fn egui_context(&self) -> &renderer::egui::Context {
        &self.egui_context
    }

    fn take_egui_input(&mut self) -> renderer::egui::RawInput {
        self.raw_input.take()
    }

    fn handle_platform_output(&mut self, _platform_output: renderer::egui::PlatformOutput) {
        // TODO
    }
}

#[wasm_bindgen]
#[allow(clippy::arc_with_non_send_sync)]
pub fn run(redraw_handler: Function, create_handler: Function) {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Warn).expect("Failed to setup logger");
    info!("Renderer loading");

    let window = window().expect("Unable to get window");
    let document = window.document().expect("Unable to get document");
    let canvas: HtmlCanvasElement = document
        .get_element_by_id("renderer-canvas")
        .expect("Unable to get canvas")
        .dyn_into()
        .expect("Target is not a canvas");

    let size = (canvas.client_width() as u32, canvas.client_height() as u32);
    let target = CanvasRenderTarget::new(canvas, redraw_handler);
    let target = Arc::new(target);

    #[cfg(not(feature = "gui"))]
    let state = State::new(target.clone(), size);

    #[cfg(feature = "gui")]
    let (state, event_handler) = {
        let event_handler = Arc::new(std::sync::Mutex::new(WebEventHandler::new(size)));
        let state = State::new(
            target.clone(),
            size,
            event_handler.clone(),
            Arc::new(renderer::gui::NotSupportedModelLoaderGui),
        );
        (state, event_handler)
    };

    wasm_bindgen_futures::spawn_local(async move {
        let mut state = state.await;
        state.setup_scene();

        {
            let mut event_handler = event_handler.lock().unwrap();
            event_handler.update_max_texture_side(state.limits().max_texture_dimension_2d as usize);
        }

        #[cfg(not(feature = "gui"))]
        let state_holder = StateHolder::new(state, target);
        #[cfg(feature = "gui")]
        let state_holder = StateHolder::new(state, target, event_handler);

        create_handler
            .call1(&JsValue::null(), &JsValue::from(state_holder))
            .expect("Unable to call create handler");
    });
}
