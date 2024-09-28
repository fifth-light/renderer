#[cfg(not(target_os = "android"))]
fn main() {
    use renderer::App;
    use winit::event_loop::{ControlFlow, EventLoop};

    env_logger::init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop
        .run_app(&mut app)
        .expect("Failed to run the application");
}

#[cfg(target_os = "android")]
fn main() {
    panic!("Android don't use main")
}
