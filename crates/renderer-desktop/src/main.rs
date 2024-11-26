use std::sync::{mpsc, Arc};

use renderer::{
    asset::loader::AssetLoadParams,
    egui::{Align2, Context, Window},
    gui::{GuiAction, ModelLoaderGui},
    winit::{App, NoOpAppcallCallback},
};
use rfd::FileDialog;
use std::thread;

#[derive(Default)]
struct DesktopModelLoaderGui {}

impl ModelLoaderGui for DesktopModelLoaderGui {
    fn ui(
        &self,
        ctx: &Context,
        param: &mut AssetLoadParams,
        gui_actions_tx: &mut mpsc::Sender<GuiAction>,
    ) {
        Window::new("Load Model")
            .resizable([false, false])
            .pivot(Align2::RIGHT_TOP)
            .show(ctx, |ui| {
                ui.checkbox(&mut param.disable_unlit, "Disable unlit");
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
                if ui.button("Load GLTF / VRM").clicked() {
                    let tx = gui_actions_tx.clone();
                    thread::spawn(move || {
                        if let Some(file) = FileDialog::new()
                            .add_filter("GLTF json file", &["gltf"])
                            .add_filter("GLTF binary file", &["glb"])
                            .add_filter("VRM file", &["vrm"])
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
}

fn main() {
    env_logger::init();

    App::<NoOpAppcallCallback>::run(
        NoOpAppcallCallback::default(),
        Arc::new(DesktopModelLoaderGui::default()),
    );
}
