#![cfg(target_family = "wasm")]

use log::{debug, Level};
use renderer::{
    gui::NotSupportedModelLoaderGui,
    winit::{platform::web::WindowAttributesExtWebSys, window::WindowAttributes},
    App, AppCallback,
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

struct WebAppCallback {
    canvas: HtmlCanvasElement,
}

impl AppCallback for WebAppCallback {
    fn window_creating(&mut self, param: WindowAttributes) -> WindowAttributes {
        param.with_canvas(Some(self.canvas.clone()))
    }
}

#[wasm_bindgen]
pub extern "C" fn run() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Debug).expect("Failed to setup logger");
    debug!("Renderer loading");
    web_sys::window()
        .and_then(|win| win.document())
        .map(|doc| {
            let canvas: HtmlCanvasElement = doc
                .get_element_by_id("renderer-canvas")
                .expect("Canvas not found")
                .dyn_into()
                .expect("#renderer-canvas is not a <canvas>");
            App::<WebAppCallback, NotSupportedModelLoaderGui>::run(
                WebAppCallback { canvas },
                NotSupportedModelLoaderGui::default(),
            );
        })
        .expect("Failed to get document object");
}
