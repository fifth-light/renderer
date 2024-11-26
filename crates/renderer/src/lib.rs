pub mod asset;
pub mod renderer;
pub mod state;

#[cfg(feature = "winit")]
pub mod winit;

#[cfg(feature = "gui")]
pub mod gui;
#[cfg(feature = "gui")]
pub use egui;
#[cfg(feature = "gui")]
pub use egui_wgpu;

use wgpu::{
    rwh::{HasDisplayHandle, HasWindowHandle},
    WasmNotSendSync,
};

pub trait RenderTarget: HasWindowHandle + HasDisplayHandle + WasmNotSendSync + 'static {
    fn native_pixels_per_point(&self) -> f32;
    fn pre_present_notify(&self);
    fn request_redraw(&self);
}
