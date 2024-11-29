use renderer::{gui::connect::tokio::TokioConnectParam, winit::App};

fn main() {
    env_logger::init();

    App::<TokioConnectParam>::run();
}
