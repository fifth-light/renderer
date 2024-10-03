use std::sync::mpsc::Sender;

use egui::{color_picker::color_edit_button_rgb, Align2, Context, Slider, Window};
use glam::Vec3;

use crate::renderer::{uniform::light::GlobalLightParam, RendererState};

use super::GuiAction;

pub fn light_param(
    ctx: &Context,
    renderer: &RendererState,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    Window::new("Light Params")
        .resizable([false, false])
        .pivot(Align2::RIGHT_BOTTOM)
        .show(ctx, |ui| {
            ui.label("Background color:");
            let background_color = renderer.background_color().to_array();
            let mut edit_background_color = background_color;
            color_edit_button_rgb(ui, &mut edit_background_color);
            if edit_background_color != background_color {
                let _ = gui_actions_tx.send(GuiAction::SetBackgroundColor(Vec3::from_array(
                    edit_background_color,
                )));
            }
            let param = renderer.global_light_param();
            ui.add(
                Slider::from_get_set(0.0..=1.0, |value| {
                    if let Some(new_value) = value {
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(GlobalLightParam {
                            start_strength: (new_value as f32).min(param.stop_strength),
                            ..*param
                        }));
                        new_value
                    } else {
                        param.start_strength as f64
                    }
                })
                .text("Start Strength"),
            );
            ui.add(
                Slider::from_get_set(0.0..=1.0, |value| {
                    if let Some(new_value) = value {
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(GlobalLightParam {
                            stop_strength: (new_value as f32).max(param.start_strength),
                            ..*param
                        }));
                        new_value
                    } else {
                        param.stop_strength as f64
                    }
                })
                .text("Stop Strength"),
            );
            ui.add(
                Slider::from_get_set(0.0..=1.0, |value| {
                    if let Some(new_value) = value {
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(GlobalLightParam {
                            max_strength: new_value as f32,
                            ..*param
                        }));
                        new_value
                    } else {
                        param.max_strength as f64
                    }
                })
                .text("Max Strength"),
            );
            ui.add(
                Slider::from_get_set(0.0..=1.0, |value| {
                    if let Some(new_value) = value {
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(GlobalLightParam {
                            border_start_strength: (new_value as f32)
                                .min(param.border_stop_strength),
                            ..*param
                        }));
                        new_value
                    } else {
                        param.border_start_strength as f64
                    }
                })
                .text("Border Start Strength"),
            );
            ui.add(
                Slider::from_get_set(0.0..=1.0, |value| {
                    if let Some(new_value) = value {
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(GlobalLightParam {
                            border_stop_strength: (new_value as f32)
                                .max(param.border_start_strength),
                            ..*param
                        }));
                        new_value
                    } else {
                        param.border_stop_strength as f64
                    }
                })
                .text("Border Stop Strength"),
            );
            ui.add(
                Slider::from_get_set(0.0..=1.0, |value| {
                    if let Some(new_value) = value {
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(GlobalLightParam {
                            border_max_strength: new_value as f32,
                            ..*param
                        }));
                        new_value
                    } else {
                        param.border_max_strength as f64
                    }
                })
                .text("Border Max Strength"),
            );
            ui.add(
                Slider::from_get_set(0.0..=1.0, |value| {
                    if let Some(new_value) = value {
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(GlobalLightParam {
                            ambient_strength: new_value as f32,
                            ..*param
                        }));
                        new_value
                    } else {
                        param.ambient_strength as f64
                    }
                })
                .text("Ambient Strength"),
            );
        });
}
