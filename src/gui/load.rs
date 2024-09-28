use std::{sync::mpsc::Sender, thread};

use egui::{Align2, Context, Window};

use super::GuiAction;

#[cfg(not(target_os = "android"))]
pub fn model_load(ctx: &Context, gui_actions_tx: &mut Sender<GuiAction>) {
    use rfd::FileDialog;

    Window::new("Load Model")
        .resizable([false, false])
        .pivot(Align2::RIGHT_TOP)
        .show(ctx, |ui| {
            if ui.button("Load OBJ").clicked() {
                let tx = gui_actions_tx.clone();
                thread::spawn(move || {
                    if let Some(file) = FileDialog::new()
                        .add_filter("Wavefront OBJ file", &["obj"])
                        .pick_file()
                    {
                        let _ = tx.send(GuiAction::LoadObj(file));
                    }
                });
            }
            if ui.button("Load GLTF").clicked() {
                let tx = gui_actions_tx.clone();
                thread::spawn(move || {
                    if let Some(file) = FileDialog::new()
                        .add_filter("GLTF json file", &["gltf"])
                        .add_filter("GLTF binary file", &["glb"])
                        .pick_file()
                    {
                        let _ = tx.send(GuiAction::LoadGltf(file));
                    }
                });
            }
            if ui.button("Load PMX 2.0").clicked() {
                let tx = gui_actions_tx.clone();
                thread::spawn(move || {
                    if let Some(file) = FileDialog::new()
                        .add_filter("PMX 2.0 file", &["pmx"])
                        .pick_file()
                    {
                        let _ = tx.send(GuiAction::LoadPmx(file));
                    }
                });
            }
        });
}

#[cfg(target_os = "android")]
pub fn model_load(ctx: &Context, _gui_actions_tx: &mut Sender<GuiAction>) {
    Window::new("Load Model")
        .resizable([false, false])
        .pivot(Align2::RIGHT_TOP)
        .show(ctx, |ui| {
            ui.label("Loading model is not supported on Android.");
        });
}
