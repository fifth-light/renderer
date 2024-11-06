use std::{collections::HashMap, sync::mpsc::Sender, time::Duration};

use egui::{Align2, CollapsingHeader, Context, ScrollArea, Ui, Window};
use web_time::Instant;

use crate::{
    asset::animation::{AnimationKeyFrames, AnimationSampler},
    renderer::animation::{AnimationGroupNode, AnimationNode, AnimationState},
};

use super::GuiAction;

fn duration_format(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let millis = duration.as_millis();
    format!("{}.{:#03}s", seconds, millis % 1000)
}

fn animation_item(ui: &mut Ui, animation: &AnimationNode) {
    let label = format!("Animation #{}", animation.id());
    CollapsingHeader::new(label)
        .id_salt(animation.id())
        .show(ui, |ui| {
            ui.label(format!(
                "Length: {:#.03}s",
                duration_format(*animation.length())
            ));
            ui.label(format!("Target node: #{}", animation.target_node()));
            match animation.sampler() {
                AnimationSampler::Rotation(AnimationKeyFrames::Linear(keyframes)) => {
                    ui.label(format!("Rotation, Linear (keyframes: {})", keyframes.len()));
                }
                AnimationSampler::Rotation(AnimationKeyFrames::Step(keyframes)) => {
                    ui.label(format!("Rotation, Step (keyframes: {})", keyframes.len()));
                }
                AnimationSampler::Rotation(AnimationKeyFrames::CubicSpline(keyframes)) => {
                    ui.label(format!(
                        "Rotation, CubicSpline (keyframes: {})",
                        keyframes.len()
                    ));
                }
                AnimationSampler::Translation(AnimationKeyFrames::Linear(keyframes)) => {
                    ui.label(format!(
                        "Translation, Linear (keyframes: {})",
                        keyframes.len()
                    ));
                }
                AnimationSampler::Translation(AnimationKeyFrames::Step(keyframes)) => {
                    ui.label(format!(
                        "Translation, Step (keyframes: {})",
                        keyframes.len()
                    ));
                }
                AnimationSampler::Translation(AnimationKeyFrames::CubicSpline(keyframes)) => {
                    ui.label(format!(
                        "Translation, CubicSpline (keyframes: {})",
                        keyframes.len()
                    ));
                }
                AnimationSampler::Scale(AnimationKeyFrames::Linear(keyframes)) => {
                    ui.label(format!("Scale, Linear (keyframes: {})", keyframes.len()));
                }
                AnimationSampler::Scale(AnimationKeyFrames::Step(keyframes)) => {
                    ui.label(format!("Scale, Step (keyframes: {})", keyframes.len()));
                }
                AnimationSampler::Scale(AnimationKeyFrames::CubicSpline(keyframes)) => {
                    ui.label(format!(
                        "Scale, CubicSpline (keyframes: {})",
                        keyframes.len()
                    ));
                }
            };
        });
}

fn animation_group(
    ui: &mut Ui,
    time: &Instant,
    animation: &AnimationGroupNode,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    let label = match &animation.label() {
        Some(label) => format!("Animation Group \"{}\"", label),
        None => format!("Animation Group #{}", animation.id()),
    };
    CollapsingHeader::new(label)
        .id_salt(animation.id())
        .show(ui, |ui| {
            ui.label(format!(
                "Length: {:#.03}s",
                duration_format(animation.length())
            ));
            match animation.state() {
                AnimationState::Stopped => {
                    ui.label("Stopped");
                    if ui.button("Play once").clicked() {
                        let _ = gui_actions_tx.send(GuiAction::StartAnimationOnce(animation.id()));
                    }
                    if ui.button("Play repeatedly").clicked() {
                        let _ =
                            gui_actions_tx.send(GuiAction::StartAnimationRepeat(animation.id()));
                    }
                    if ui.button("Play loop").clicked() {
                        let _ = gui_actions_tx.send(GuiAction::StartAnimationLoop(animation.id()));
                    }
                }
                AnimationState::Once(start_time) => {
                    let duration: Duration = *time - *start_time;
                    ui.label("Once");
                    ui.label(format!("Duration: {}", duration_format(duration)));
                    if ui.button("Stop").clicked() {
                        let _ = gui_actions_tx.send(GuiAction::StopAnimation(animation.id()));
                    }
                }
                AnimationState::Repeat(start_time) => {
                    let duration: Duration = *time - *start_time;
                    ui.label("Repeat");
                    ui.label(format!("Duration: {}", duration_format(duration)));
                    if ui.button("Stop").clicked() {
                        let _ = gui_actions_tx.send(GuiAction::StopAnimation(animation.id()));
                    }
                }
                AnimationState::Loop(start_time) => {
                    let duration: Duration = *time - *start_time;
                    ui.label("Loop");
                    ui.label(format!("Duration: {}", duration_format(duration)));
                    if ui.button("Stop").clicked() {
                        let _ = gui_actions_tx.send(GuiAction::StopAnimation(animation.id()));
                    }
                }
            };
            for node in animation.nodes().iter() {
                animation_item(ui, node);
            }
        });
}

pub fn animation_items(
    ctx: &Context,
    time: &Instant,
    animations: &HashMap<usize, AnimationGroupNode>,
    gui_actions_tx: &mut Sender<GuiAction>,
) {
    Window::new("Animation")
        .pivot(Align2::LEFT_TOP)
        .show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                for animation in animations.values() {
                    animation_group(ui, time, animation, gui_actions_tx);
                }
            });
        });
}
