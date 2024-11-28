#![allow(unused)]
mod asset;
mod client;
pub mod gui;
pub mod renderer;
pub mod state;
pub mod transport;

#[cfg(feature = "winit")]
pub mod winit;

pub use egui;
pub use egui_wgpu;

pub use renderer_protocol as protocol;

use wgpu::{
    rwh::{HasDisplayHandle, HasWindowHandle},
    WasmNotSendSync,
};

pub trait RenderTarget: HasWindowHandle + HasDisplayHandle + WasmNotSendSync + 'static {
    fn native_pixels_per_point(&self) -> f32;
    fn pre_present_notify(&self);
    fn request_redraw(&self);
}
