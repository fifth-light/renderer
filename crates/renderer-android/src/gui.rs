use std::cmp::Ordering;

use android_activity::{
    input::{Axis, KeyAction, KeyEvent, MotionAction, MotionEvent, TextInputState, TextSpan},
    AndroidApp, InputStatus,
};
use jni::{
    objects::{JObject, JValue},
    JavaVM,
};
use log::info;
use ndk::configuration::UiModeNight;
use renderer::{
    egui::{
        Context as EguiContext, Event, Key, Modifiers, MouseWheelUnit, PlatformOutput,
        PointerButton, Pos2, RawInput, Rect, Theme, TouchDeviceId, TouchId, TouchPhase, Vec2,
    },
    gui::event::GuiEventHandler,
};

use crate::{
    app_density,
    keycodes::{keycode_to_key, keycode_to_text},
    AndroidRenderTarget,
};

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

#[derive(Debug, Clone, Default)]
struct Insets {
    top: u32,
    bottom: u32,
    left: u32,
    right: u32,
}

pub struct AndroidEventHandler {
    app: AndroidApp,
    egui_context: EguiContext,
    raw_input: RawInput,
    pointer_captured: bool,
    keyboard_shown: bool,
    render_target_size: (u32, u32),
}

impl AndroidEventHandler {
    pub fn new(app: &AndroidApp, target: &AndroidRenderTarget) -> Self {
        let mut handler = Self {
            app: app.clone(),
            egui_context: EguiContext::default(),
            raw_input: RawInput::default(),
            pointer_captured: true,
            keyboard_shown: false,
            render_target_size: target.size(),
        };
        app.set_text_input_state(TextInputState {
            text: String::from("  "),
            selection: TextSpan { start: 1, end: 1 },
            compose_region: None,
        });
        handler.update_config(target);
        handler
    }

    pub fn set_max_texture_side(&mut self, max_texture_side: usize) {
        self.raw_input.max_texture_side = Some(max_texture_side);
    }

    fn get_window_insets(&self) -> Insets {
        let java_vm = self.app.java_vm();
        let activity = self.app.activity_object();
        let mut jni_env = java_vm.get_env().unwrap();

        let insets = jni_env
            .get_field(activity, "imeInsets", "Landroidx/core/graphics/Insets;")
            .expect("Unable to get insets");
        let inset = insets.l().expect("Bad insets object");
        if inset.is_null() {
            return Insets::default();
        }

        let top = jni_env
            .get_field(&inset, "top", "I")
            .expect("Failed to get top inset")
            .i()
            .expect("Bad top inset");
        let bottom = jni_env
            .get_field(&inset, "bottom", "I")
            .expect("Failed to get bottom inset")
            .i()
            .expect("Bad bottom inset");
        let left = jni_env
            .get_field(&inset, "left", "I")
            .expect("Failed to get left inset")
            .i()
            .expect("Bad left inset");
        let right = jni_env
            .get_field(&inset, "right", "I")
            .expect("Failed to get right inset")
            .i()
            .expect("Bad right inset");
        Insets {
            top: top.max(0) as u32,
            bottom: bottom.max(0) as u32,
            left: left.max(0) as u32,
            right: right.max(0) as u32,
        }
    }

    fn update_screen_rect(&mut self) {
        let density = app_density(&self.app);
        let inset = self.get_window_insets();
        self.raw_input.screen_rect = Some(Rect::from_min_size(
            Pos2 {
                x: inset.top as f32,
                y: inset.left as f32,
            },
            Vec2 {
                x: (self.render_target_size.0 as f32 - inset.right as f32).max(0.0),
                y: (self.render_target_size.1 as f32 - inset.bottom as f32).max(0.0),
            } / density,
        ));
    }

    pub fn update_config(&mut self, target: &AndroidRenderTarget) {
        let density = app_density(&self.app);
        if let Some(viewport) = self
            .raw_input
            .viewports
            .get_mut(&self.raw_input.viewport_id)
        {
            viewport.native_pixels_per_point = Some(density);
        }

        self.render_target_size = target.size();
        self.update_screen_rect();

        self.raw_input.system_theme = match self.app.config().ui_mode_night() {
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

    pub fn on_inset_changed(&mut self) {
        info!("on_inset_changed()");
        self.update_screen_rect();
    }

    #[must_use]
    pub fn on_key_event(&mut self, key_event: &KeyEvent) -> InputStatus {
        let keycode = key_event.key_code();

        let state = match key_event.action() {
            KeyAction::Down => true,
            KeyAction::Up => false,
            _ => return InputStatus::Unhandled,
        };

        if state {
            if let Some(text) = keycode_to_text(keycode) {
                self.raw_input.events.push(Event::Text(String::from(text)));
            }
        }

        let Some(key) = keycode_to_key(keycode) else {
            return InputStatus::Unhandled;
        };

        self.raw_input.events.push(Event::Key {
            key,
            physical_key: None,
            pressed: state,
            repeat: false,
            modifiers: Modifiers::default(),
        });

        InputStatus::Handled
    }

    pub fn on_text_event(&mut self, text_event: &TextInputState) -> InputStatus {
        // HACK: Android game input model don't work very well with egui's text
        // input model, so there is a huge hack around it.
        if !text_event.text.is_empty() {
            fn fake_key_press(events: &mut Vec<Event>, key: Key) {
                events.push(Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::default(),
                });
                events.push(Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: Modifiers::default(),
                });
            }

            // First, set text to '  ', and put cursor to the center. Second,
            // when the ime want to move the cursor, send a left key event or
            // right key event instead, and reset the cursor position.
            // Third, if user delete spaces, send a backspace event, and reset
            // the text. If user press enter, send a enter event, and do reset.
            // In other cases we send a text event with the text between two
            // spaces. This basically recreated text input events, but doesn't
            // handle text composing very well.
            let text = text_event.text.as_str();
            match text.len().cmp(&2) {
                Ordering::Less => {
                    fake_key_press(&mut self.raw_input.events, Key::Backspace);
                }
                Ordering::Greater => {
                    let input_text = &text[1..text.len() - 1];
                    self.raw_input
                        .events
                        .push(Event::Text(String::from(input_text)));
                }
                Ordering::Equal => match text_event.selection.start.cmp(&1) {
                    Ordering::Less => {
                        fake_key_press(&mut self.raw_input.events, Key::ArrowLeft);
                    }
                    Ordering::Greater => {
                        fake_key_press(&mut self.raw_input.events, Key::ArrowRight);
                    }
                    Ordering::Equal => {}
                },
            }
        }
        self.app.set_text_input_state(TextInputState {
            text: String::from("  "),
            selection: TextSpan { start: 1, end: 1 },
            compose_region: None,
        });
        InputStatus::Handled
    }

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
        self.update_screen_rect();

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

        if platform_output.ime.is_some() {
            if !self.keyboard_shown {
                self.keyboard_shown = true;
                self.app.show_soft_input(true);
            }
        } else if self.keyboard_shown {
            self.keyboard_shown = false;
            self.app.hide_soft_input(true);
        }
    }
}
