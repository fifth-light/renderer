use renderer::winit::{App, NoOpAppcallCallback};

#[cfg(feature = "gui")]
#[derive(Default)]
struct DesktopModelLoaderGui {}

#[cfg(feature = "gui")]
impl renderer::gui::ModelLoaderGui for DesktopModelLoaderGui {
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
        use rfd::FileDialog;
        use std::thread;

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
        #[cfg(feature = "gui")]
        std::sync::Arc::new(DesktopModelLoaderGui::default()),
    );
}
