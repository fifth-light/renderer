use renderer::{
    egui::{Context, Pos2, RawInput, Rect},
    gui::event::GuiEventHandler,
};

use crate::MouseButton;

pub mod connect;

impl From<MouseButton> for renderer::egui::PointerButton {
    fn from(mouse_button: MouseButton) -> Self {
        use renderer::egui::PointerButton;
        match mouse_button {
            MouseButton::Primary => PointerButton::Primary,
            MouseButton::Secondary => PointerButton::Secondary,
            MouseButton::Middle => PointerButton::Middle,
            MouseButton::Extra1 => PointerButton::Extra1,
            MouseButton::Extra2 => PointerButton::Extra2,
        }
    }
}

pub struct WebEventHandler {
    egui_context: Context,
    raw_input: RawInput,
}

impl WebEventHandler {
    pub fn new(size: (u32, u32)) -> Self {
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

    pub fn set_max_texture_side(&mut self, max_texture_side: usize) {
        self.raw_input.max_texture_side = Some(max_texture_side);
    }

    pub fn set_native_pixels_per_point(&mut self, native_pixels_per_point: f32) {
        let Some(viewport) = self
            .raw_input
            .viewports
            .get_mut(&self.raw_input.viewport_id)
        else {
            return;
        };
        viewport.native_pixels_per_point = Some(native_pixels_per_point);
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

    pub fn set_theme(&mut self, is_dark: Option<bool>) {
        use renderer::egui::Theme;

        self.raw_input.system_theme =
            is_dark.map(|dark| if dark { Theme::Dark } else { Theme::Light });
    }

    pub fn set_focused(&mut self, focused: bool) {
        use renderer::egui::Event;
        self.raw_input.focused = focused;
        self.raw_input.events.push(Event::WindowFocused(focused));
    }

    pub fn mouse_moved(&mut self, pos: (f32, f32)) {
        use renderer::egui::{Event, Pos2};
        self.raw_input
            .events
            .push(Event::PointerMoved(Pos2::new(pos.0, pos.1)));
    }

    pub fn mouse_button(&mut self, pos: (f32, f32), button: MouseButton, pressed: bool) {
        use renderer::egui::{Event, Modifiers, Pos2};
        self.raw_input.events.push(Event::PointerButton {
            pos: Pos2::new(pos.0, pos.1),
            button: button.into(),
            pressed,
            modifiers: Modifiers::default(),
        });
    }
}

impl GuiEventHandler for WebEventHandler {
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
