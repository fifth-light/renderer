use renderer::winit::{App, NoOpAppcallCallback};

fn main() {
    env_logger::init();

    App::run(NoOpAppcallCallback::default());
}
