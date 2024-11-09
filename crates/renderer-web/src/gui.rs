use crate::MouseButton;

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
    egui_context: renderer::egui::Context,
    raw_input: renderer::egui::RawInput,
}

impl WebEventHandler {
    pub fn new(size: (u32, u32)) -> Self {
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

#[derive(Default)]
pub struct WebModelLoaderGui;

impl renderer::gui::ModelLoaderGui for WebModelLoaderGui {
    fn ui(
        &self,
        ctx: &renderer::egui::Context,
        param: &mut renderer::asset::loader::AssetLoadParams,
        gui_actions_tx: &mut std::sync::mpsc::Sender<renderer::gui::GuiAction>,
    ) {
        use renderer::{
            egui::{Align2, Window},
            gui::GuiAction,
        };
        use rfd::AsyncFileDialog;

        Window::new("Load Model")
            .resizable([false, false])
            .pivot(Align2::RIGHT_TOP)
            .show(ctx, |ui| {
                ui.checkbox(&mut param.disable_unlit, "Disable unlit");
                if ui.button("Load GLTF / VRM").clicked() {
                    let tx = gui_actions_tx.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some(file) = AsyncFileDialog::new()
                            .add_filter("GLTF json file", &["gltf"])
                            .add_filter("GLTF binary file", &["glb"])
                            .add_filter("VRM file", &["vrm"])
                            .pick_file()
                            .await
                        {
                            let file_name = file.file_name();
                            let model = file.read().await;
                            let _ = tx.send(GuiAction::LoadGltfData(Some(file_name), model));
                        }
                    });
                }
            });
    }
}
