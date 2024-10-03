#![cfg(any(target_arch = "wasm32", target_arch = "wasm64"))]

use log::Level;
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
pub async fn run() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Warn).expect("Failed to setup logger");
    web_sys::window()
        .and_then(|win| win.document())
        .map(|doc| {
            let canvas: HtmlCanvasElement = doc
                .get_element_by_id("renderer-canvas")
                .expect("Canvas not found")
                .dyn_into()
                .expect("#renderer-canvas is not a <canvas>");
            canvas.set_attribute("width", "720").unwrap_throw();
            canvas.set_attribute("height", "480").unwrap_throw();
            App::<WebAppCallback, NotSupportedModelLoaderGui>::run(
                WebAppCallback { canvas },
                NotSupportedModelLoaderGui::default(),
            );
        })
        .expect("Failed to get document object");
}
