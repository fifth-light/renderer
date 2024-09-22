use std::sync::mpsc::Sender;

use egui::{Align2, Context, Slider, Window};

use crate::renderer::{uniform::light::LightParam, RendererState};

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
            let param = renderer.light_param();
            ui.add(
                Slider::from_get_set(0.0..=1.0, |value| {
                    if let Some(new_value) = value {
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(LightParam {
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
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(LightParam {
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
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(LightParam {
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
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(LightParam {
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
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(LightParam {
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
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(LightParam {
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
                        let _ = gui_actions_tx.send(GuiAction::SetLightParam(LightParam {
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
