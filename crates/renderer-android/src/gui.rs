use std::sync::mpsc;

use android_activity::{
    input::{Axis, KeyEvent, MotionAction, MotionEvent},
    AndroidApp,
};
use jni::{
    objects::{JObject, JValue},
    sys::jlong,
    JavaVM,
};
use log::info;
use ndk::configuration::UiModeNight;
use renderer::{
    asset::loader::AssetLoadParams,
    egui::{
        Context as EguiContext, Event, Modifiers, MouseWheelUnit, PlatformOutput, PointerButton,
        Pos2, RawInput, Rect, Theme, TouchDeviceId, TouchId, TouchPhase, Vec2,
    },
    gui::{event::GuiEventHandler, GuiAction, ModelLoaderGui},
};

use crate::{app_density, AndroidRenderTarget};

trait AndroidAppExt {
    fn activity_object(&self) -> JObject<'_>;
    fn java_vm(&self) -> JavaVM;
}

impl AndroidAppExt for AndroidApp {
    fn java_vm(&self) -> JavaVM {
        unsafe { JavaVM::from_raw(self.vm_as_ptr() as _).unwrap() }
    }

    fn activity_object(&self) -> JObject<'_> {
        unsafe { JObject::from_raw(self.activity_as_ptr() as _) }
    }
}

pub struct AndroidEventHandler {
    app: AndroidApp,
    egui_context: EguiContext,
    raw_input: RawInput,
    pointer_captured: bool,
}

impl AndroidEventHandler {
    pub fn new(app: &AndroidApp, target: &AndroidRenderTarget) -> Self {
        let mut handler = Self {
            app: app.clone(),
            egui_context: EguiContext::default(),
            raw_input: RawInput::default(),
            pointer_captured: true,
        };
        handler.update_config(app, target);
        handler
    }

    pub fn set_max_texture_side(&mut self, max_texture_side: usize) {
        self.raw_input.max_texture_side = Some(max_texture_side);
    }

    pub fn update_config(&mut self, app: &AndroidApp, target: &AndroidRenderTarget) {
        let density = app_density(app);
        if let Some(viewport) = self
            .raw_input
            .viewports
            .get_mut(&self.raw_input.viewport_id)
        {
            viewport.native_pixels_per_point = Some(density);
        }

        let size = target.size();
        self.raw_input.screen_rect = Some(Rect::from_min_size(
            Default::default(),
            Vec2 {
                x: size.0 as f32,
                y: size.1 as f32,
            } / density,
        ));

        self.raw_input.system_theme = match app.config().ui_mode_night() {
            UiModeNight::No => Some(Theme::Light),
            UiModeNight::Yes => Some(Theme::Dark),
            _ => None,
        };
    }

    fn update_pointer_captured(&mut self, captured: bool) {
        let java_vm = self.app.java_vm();
        let activity = self.app.activity_object();
        let mut jni_env = java_vm.get_env().unwrap();

        if captured {
            jni_env
                .call_method(activity, "enablePointerLock", "()V", &[])
                .expect("Call enablePointerLock failed");
        } else {
            jni_env
                .call_method(activity, "disablePointerLock", "()V", &[])
                .expect("Call disablePointerLock failed");
        }
    }

    pub fn set_pointer_captured(&mut self, captured: bool) {
        self.pointer_captured = captured;
        self.update_pointer_captured(captured);
    }

    pub fn on_resume(&mut self) {
        info!("on_resume()");
        self.update_pointer_captured(self.pointer_captured);
    }

    pub fn on_pause(&mut self) {
        info!("on_pause()");
        self.update_pointer_captured(false);
    }

    pub fn on_key_event(&mut self, _key_event: &KeyEvent) {}

    pub fn on_motion_event(&mut self, motion_event: &MotionEvent) {
        let density = app_density(&self.app);

        match motion_event.action() {
            MotionAction::Down => {
                for pointer in motion_event.pointers() {
                    let pos = Pos2::new(pointer.x(), pointer.y()) / density;
                    self.raw_input.events.push(Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::default(),
                    });
                    self.raw_input.events.push(Event::Touch {
                        device_id: TouchDeviceId(0),
                        id: TouchId(pointer.pointer_id() as u64),
                        phase: TouchPhase::Start,
                        pos,
                        force: Some(pointer.pressure()),
                    });
                }
            }
            MotionAction::Up => {
                for pointer in motion_event.pointers() {
                    let pos = Pos2::new(pointer.x(), pointer.y()) / density;
                    self.raw_input.events.push(Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed: false,
                        modifiers: Modifiers::default(),
                    });
                    self.raw_input.events.push(Event::Touch {
                        device_id: TouchDeviceId(0),
                        id: TouchId(pointer.pointer_id() as u64),
                        phase: TouchPhase::End,
                        pos,
                        force: Some(pointer.pressure()),
                    });
                    self.raw_input.events.push(Event::PointerGone);
                }
            }
            MotionAction::Move => {
                for pointer in motion_event.pointers() {
                    let pos = Pos2::new(pointer.x(), pointer.y()) / density;
                    self.raw_input.events.push(Event::PointerMoved(pos));
                    self.raw_input.events.push(Event::Touch {
                        device_id: TouchDeviceId(0),
                        id: TouchId(pointer.pointer_id() as u64),
                        phase: TouchPhase::Move,
                        pos,
                        force: Some(pointer.pressure()),
                    });
                }
            }
            MotionAction::Cancel => {
                for pointer in motion_event.pointers() {
                    let pos = Pos2::new(pointer.x(), pointer.y()) / density;
                    self.raw_input.events.push(Event::Touch {
                        device_id: TouchDeviceId(0),
                        id: TouchId(pointer.pointer_id() as u64),
                        phase: TouchPhase::Cancel,
                        pos,
                        force: Some(pointer.pressure()),
                    });
                }
            }
            MotionAction::HoverEnter | MotionAction::HoverMove => {
                for pointer in motion_event.pointers() {
                    let pos = Pos2::new(pointer.x(), pointer.y()) / density;
                    self.raw_input.events.push(Event::PointerMoved(pos));
                }
            }
            MotionAction::HoverExit => {
                for _pointer in motion_event.pointers() {
                    self.raw_input.events.push(Event::PointerGone);
                }
            }
            MotionAction::Outside => {
                for pointer in motion_event.pointers() {
                    let pos = Pos2::new(pointer.x(), pointer.y()) / density;
                    self.raw_input.events.push(Event::PointerMoved(pos));
                }
            }
            MotionAction::Scroll => {
                for pointer in motion_event.pointers() {
                    let v_scroll = pointer.axis_value(Axis::Vscroll);
                    let h_scroll = pointer.axis_value(Axis::Hscroll);
                    self.raw_input.events.push(Event::MouseWheel {
                        unit: MouseWheelUnit::Page,
                        delta: Vec2::new(h_scroll, v_scroll),
                        modifiers: Modifiers::default(),
                    });
                }
            }
            _ => (),
        }
    }
}

impl GuiEventHandler for AndroidEventHandler {
    fn egui_context(&self) -> &EguiContext {
        &self.egui_context
    }

    fn take_egui_input(&mut self) -> RawInput {
        self.raw_input.take()
    }

    fn handle_platform_output(&mut self, platform_output: PlatformOutput) {
        let java_vm = self.app.java_vm();
        let activity = self.app.activity_object();
        let mut jni_env = java_vm.get_env().unwrap();

        if let Some(open_url) = platform_output.open_url {
            let url = jni_env
                .new_string(open_url.url)
                .expect("Failed to create url string");
            jni_env
                .call_method(
                    activity,
                    "openUrl",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(&url)],
                )
                .expect("Call openUrl failed");
        }
    }
}

pub struct AndroidModelLoaderGui {
    app: AndroidApp,
}

impl AndroidModelLoaderGui {
    pub fn new(app: AndroidApp) -> Self {
        Self { app }
    }

    fn open_file(&self, tx: mpsc::Sender<GuiAction>, filters: &[&str]) {
        let tx = Box::new(tx);
        let tx = Box::into_raw(tx);
        let tx = tx as jlong;

        let java_vm = self.app.java_vm();
        let activity = self.app.activity_object();
        let mut jni_env = java_vm.get_env().unwrap();

        let string_class = jni_env
            .find_class("java/lang/String")
            .expect("Failed to fetch class of String");
        let mut filters_array = jni_env
            .new_object_array(filters.len() as i32, string_class, JObject::null())
            .expect("Failed to create file filters");
        for (index, filter) in filters.iter().enumerate() {
            let filter = jni_env
                .new_string(filter)
                .expect("Failed to create url string");
            jni_env
                .set_object_array_element(&mut filters_array, index as i32, filter)
                .expect("Failed to set filter array item");
        }
        jni_env
            .call_method(
                activity,
                "openFile",
                "(J[Ljava/lang/String;)V",
                &[JValue::Long(tx), JValue::Object(&filters_array)],
            )
            .expect("Call openFile failed");
    }
}

impl ModelLoaderGui for AndroidModelLoaderGui {
    fn ui(
        &self,
        ctx: &EguiContext,
        param: &mut AssetLoadParams,
        gui_actions_tx: &mut mpsc::Sender<GuiAction>,
    ) {
        use renderer::egui::{Align2, Window};

        Window::new("Load Model")
            .resizable([false, false])
            .pivot(Align2::RIGHT_TOP)
            .show(ctx, |ui| {
                ui.checkbox(&mut param.disable_unlit, "Disable unlit");
                if ui.button("Load GLTF / VRM").clicked() {
                    let tx = gui_actions_tx.clone();
                    // Android don't recognize gltf files
                    self.open_file(tx, &["*/*"]);
                }
            });
    }
}
