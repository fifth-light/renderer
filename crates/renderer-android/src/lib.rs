#![cfg(target_os = "android")]
use android_logger::Config;
use log::LevelFilter;
use renderer::{gui::NotSupportedModelLoaderGui, App, AppCallback};
use winit::{
    event_loop::EventLoopBuilder,
    platform::android::{activity::AndroidApp, EventLoopBuilderExtAndroid},
};

struct AndroidAppCallback {
    app: AndroidApp,
}

impl AppCallback for AndroidAppCallback {
    fn event_loop_building<T: 'static>(&mut self, event_loop_builder: &mut EventLoopBuilder<T>) {
        event_loop_builder.with_android_app(self.app.clone());
    }
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        Config::default()
            .with_tag("renderer")
            .with_max_level(LevelFilter::Info),
    );
    #[cfg(feature = "panics-log")]
    log_panics::init();

    App::<AndroidAppCallback, NotSupportedModelLoaderGui>::run(
        AndroidAppCallback { app },
        NotSupportedModelLoaderGui::default(),
    );
}
