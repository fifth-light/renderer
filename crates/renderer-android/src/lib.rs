#![cfg(target_os = "android")]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use android_activity::{
    input::{Axis, InputEvent, KeyAction, Keycode, MotionAction, Source},
    AndroidApp, InputStatus, MainEvent, PollEvent,
};
use android_logger::{Config, FilterBuilder};
use log::{info, warn, LevelFilter};
use ndk::native_window::NativeWindow;
use pollster::FutureExt;
use renderer::{
    egui_wgpu::wgpu::rwh::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
    },
    state::{RenderResult, State},
    RenderTarget,
};

mod gui;
mod jni;

type RendererState = State<'static, gui::AndroidEventHandler>;

struct AndroidRenderTarget {
    android_app: AndroidApp,
    native_window: NativeWindow,
}

impl AndroidRenderTarget {
    pub fn new(android_app: AndroidApp, native_window: NativeWindow) -> Self {
        Self {
            android_app,
            native_window,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        (
            self.native_window.width() as u32,
            self.native_window.height() as u32,
        )
    }
}

impl HasWindowHandle for AndroidRenderTarget {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.native_window.window_handle()
    }
}

impl HasDisplayHandle for AndroidRenderTarget {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::android())
    }
}

pub fn app_density(app: &AndroidApp) -> f32 {
    app.config()
        .density()
        .map(|dpi| dpi as f32 / 160.0)
        .unwrap_or(1.0)
}

impl RenderTarget for AndroidRenderTarget {
    fn native_pixels_per_point(&self) -> f32 {
        app_density(&self.android_app)
    }

    fn pre_present_notify(&self) {
        // TODO
    }

    fn request_redraw(&self) {}
}

fn create_state(
    app: &AndroidApp,
    size: (u32, u32),
    render_target: Arc<AndroidRenderTarget>,
    event_handler: Arc<Mutex<gui::AndroidEventHandler>>,
) -> RendererState {
    info!("Create new state");
    let mut new_state = RendererState::new(
        render_target,
        size,
        event_handler.clone(),
        Arc::new(gui::AndroidModelLoaderGui::new(app.clone())),
    )
    .block_on();
    new_state.setup_scene();

    new_state
}

#[derive(Default)]
struct PointerState {
    last_hover_position: HashMap<i32, (f32, f32)>,
    last_press_position: HashMap<i32, (f32, f32)>,
}

fn handle_event(
    state: &mut RendererState,
    pointer_state: &mut PointerState,
    event: &InputEvent,
    event_handler: Option<&Arc<Mutex<gui::AndroidEventHandler>>>,
) -> InputStatus {
    match event {
        InputEvent::MotionEvent(motion_event) => {
            if state.egui_active() {
                {
                    let Some(event_handler) = event_handler else {
                        return InputStatus::Handled;
                    };
                    let mut handler = event_handler.lock().unwrap();
                    handler.on_motion_event(motion_event);
                }
            } else {
                match motion_event.action() {
                    MotionAction::Down => {
                        for pointer in motion_event.pointers() {
                            pointer_state
                                .last_press_position
                                .insert(pointer.pointer_id(), (pointer.x(), pointer.y()));
                        }
                    }
                    MotionAction::Up => {
                        for pointer in motion_event.pointers() {
                            pointer_state
                                .last_press_position
                                .remove(&pointer.pointer_id());
                        }
                    }
                    MotionAction::Move => match motion_event.source() {
                        Source::MouseRelative => {
                            for pointer in motion_event.pointers() {
                                let x = pointer.x();
                                let y = pointer.y();
                                state.update_rotation((x, y));
                            }
                        }
                        Source::Touchpad => {
                            for pointer in motion_event.pointers() {
                                let x = pointer.axis_value(Axis::RelativeX);
                                let y = pointer.axis_value(Axis::RelativeY);
                                state.update_rotation((x, y));
                            }
                        }
                        _ => {
                            for pointer in motion_event.pointers() {
                                let x = pointer.x();
                                let y = pointer.y();

                                if let Some((last_x, last_y)) =
                                    pointer_state.last_press_position.get(&pointer.pointer_id())
                                {
                                    let delta_x = x - last_x;
                                    let delta_y = y - last_y;
                                    state.update_rotation((delta_x, delta_y));
                                }

                                pointer_state
                                    .last_press_position
                                    .insert(pointer.pointer_id(), (x, y));
                            }
                        }
                    },
                    MotionAction::HoverMove => {
                        for pointer in motion_event.pointers() {
                            let x = pointer.x();
                            let y = pointer.y();

                            if let Some((last_x, last_y)) =
                                pointer_state.last_hover_position.get(&pointer.pointer_id())
                            {
                                let delta_x = x - last_x;
                                let delta_y = y - last_y;
                                state.update_rotation((delta_x, delta_y));
                            }

                            pointer_state
                                .last_hover_position
                                .insert(pointer.pointer_id(), (x, y));
                        }
                    }
                    MotionAction::HoverEnter => {
                        for pointer in motion_event.pointers() {
                            pointer_state
                                .last_hover_position
                                .insert(pointer.pointer_id(), (pointer.x(), pointer.y()));
                        }
                    }
                    MotionAction::HoverExit => {
                        for pointer in motion_event.pointers() {
                            pointer_state
                                .last_hover_position
                                .remove(&pointer.pointer_id());
                        }
                    }
                    MotionAction::Scroll => {
                        for pointer in motion_event.pointers() {
                            let offset = pointer.axis_value(Axis::Vscroll);
                            if offset > 0.0 {
                                state.update_fov(true);
                            } else if offset < 0.0 {
                                state.update_fov(false);
                            }
                        }
                    }
                    _ => (),
                }
            }
            InputStatus::Handled
        }
        InputEvent::KeyEvent(key_event) => {
            if let KeyAction::Down = key_event.action() {
                if let Keycode::F10 = key_event.key_code() {
                    state.set_egui_active(!state.egui_active());

                    let Some(event_handler) = event_handler else {
                        return InputStatus::Handled;
                    };
                    let mut handler = event_handler.lock().unwrap();
                    handler.set_pointer_captured(!state.egui_active());

                    return InputStatus::Handled;
                }
            }
            if state.egui_active() {
                {
                    let Some(event_handler) = event_handler else {
                        return InputStatus::Handled;
                    };
                    let mut handler = event_handler.lock().unwrap();
                    handler.on_key_event(key_event);
                }
            } else {
                match key_event.action() {
                    KeyAction::Down => match key_event.key_code() {
                        Keycode::W => {
                            state.position_controller.forward = 1.0;
                        }
                        Keycode::A => {
                            state.position_controller.left = 1.0;
                        }
                        Keycode::S => {
                            state.position_controller.backward = 1.0;
                        }
                        Keycode::D => {
                            state.position_controller.right = 1.0;
                        }
                        Keycode::ShiftLeft => {
                            state.position_controller.down = 1.0;
                        }
                        Keycode::Space => {
                            state.position_controller.up = 1.0;
                        }
                        _ => {}
                    },
                    KeyAction::Up => match key_event.key_code() {
                        Keycode::W => {
                            state.position_controller.forward = 0.0;
                        }
                        Keycode::A => {
                            state.position_controller.left = 0.0;
                        }
                        Keycode::S => {
                            state.position_controller.backward = 0.0;
                        }
                        Keycode::D => {
                            state.position_controller.right = 0.0;
                        }
                        Keycode::ShiftLeft => {
                            state.position_controller.down = 0.0;
                        }
                        Keycode::Space => {
                            state.position_controller.up = 0.0;
                        }
                        _ => {}
                    },
                    _ => (),
                }
            }
            InputStatus::Handled
        }
        InputEvent::TextEvent(_text_input_state) => {
            // TODO
            InputStatus::Handled
        }
        _ => InputStatus::Unhandled,
    }
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        Config::default()
            .with_tag("renderer")
            .with_max_level(LevelFilter::Info)
            .with_filter(
                FilterBuilder::new()
                    .filter_module("wgpu", LevelFilter::Warn)
                    .filter_module("wgpu_core", LevelFilter::Warn)
                    .filter_level(LevelFilter::Info)
                    .build(),
            ),
    );

    #[cfg(feature = "log-panics")]
    log_panics::init();

    let mut event_handler: Option<Arc<Mutex<gui::AndroidEventHandler>>> = None;
    let mut pointer_state = PointerState::default();
    let mut state: Option<RendererState> = None;
    let mut render_target: Option<Arc<AndroidRenderTarget>> = None;

    info!("Initializing");

    let mut running = true;
    while running {
        app.poll_events(Some(Duration::ZERO), |event| {
            match event {
                PollEvent::Wake => {}
                PollEvent::Timeout => {}
                PollEvent::Main(main_event) => match main_event {
                    MainEvent::InputAvailable => {
                        match app.input_events_iter() {
                            Ok(mut events_iter) => loop {
                                let handled = events_iter.next(|event| {
                                    let Some(state) = state.as_mut() else {
                                        return InputStatus::Handled;
                                    };
                                    handle_event(
                                        state,
                                        &mut pointer_state,
                                        event,
                                        event_handler.as_ref(),
                                    )
                                });
                                if !handled {
                                    break;
                                }
                            },
                            Err(err) => {
                                warn!("Failed to get input events iterator: {:?}", err);
                            }
                        };
                    }
                    MainEvent::InitWindow { .. } => {
                        info!("Init window");

                        let Some(window) = app.native_window() else {
                            return;
                        };

                        let new_render_target =
                            Arc::new(AndroidRenderTarget::new(app.clone(), window));
                        render_target = Some(new_render_target.clone());

                        if let Some(state) = state.as_mut() {
                            info!("Recreate surface");
                            state.recreate_surface(new_render_target);
                            return;
                        }

                        let size = new_render_target.size();
                        if size.0 == 0 || size.1 == 0 {
                            return;
                        }

                        let event_handler = if let Some(event_handler) = event_handler.as_mut() {
                            let mut handler = event_handler.lock().unwrap();
                            handler.update_config(&app, &new_render_target);
                            event_handler.clone()
                        } else {
                            let new_event_handler = Arc::new(Mutex::new(
                                gui::AndroidEventHandler::new(&app, &new_render_target),
                            ));
                            event_handler = Some(new_event_handler.clone());
                            new_event_handler
                        };

                        let new_state =
                            create_state(&app, size, new_render_target, event_handler.clone());

                        let limits = new_state.limits();
                        let mut handler = event_handler.lock().unwrap();
                        handler.set_pointer_captured(!new_state.egui_active());
                        handler.set_max_texture_side(limits.max_texture_dimension_2d as usize);

                        state = Some(new_state);
                    }
                    MainEvent::TerminateWindow { .. } => {
                        info!("Terminate window");
                        if let Some(state) = state.as_mut() {
                            info!("Destroy surface");
                            state.destroy_surface();
                        }
                        render_target = None;
                    }
                    MainEvent::RedrawNeeded { .. } => {}
                    MainEvent::ContentRectChanged { .. } => {}
                    MainEvent::WindowResized { .. } => {
                        let Some(event_handler) = event_handler.as_ref() else {
                            return;
                        };
                        let Some(render_target) = render_target.as_ref() else {
                            return;
                        };
                        let size = render_target.size();
                        if size.0 == 0 || size.1 == 0 {
                            return;
                        }

                        if let Some(state) = state.as_mut() {
                            info!("Resize state");
                            state.resize(size);
                        } else {
                            let new_state = create_state(
                                &app,
                                size,
                                render_target.clone(),
                                event_handler.clone(),
                            );
                            state = Some(new_state);
                        }
                    }
                    MainEvent::GainedFocus => {}
                    MainEvent::LostFocus => {}
                    MainEvent::ConfigChanged { .. } => {
                        info!("Config changed");
                        let Some(render_target) = render_target.as_ref() else {
                            return;
                        };
                        if let Some(event_handler) = event_handler.as_mut() {
                            let mut handler = event_handler.lock().unwrap();
                            info!("Update config");
                            handler.update_config(&app, render_target);
                        }
                    }
                    MainEvent::LowMemory => {}
                    MainEvent::Start => {}
                    MainEvent::Resume { loader: _, .. } => {
                        let Some(event_handler) = event_handler.as_ref() else {
                            return;
                        };
                        let mut handler = event_handler.lock().unwrap();
                        handler.on_resume();
                    }
                    MainEvent::SaveState { saver: _, .. } => {}
                    MainEvent::Pause => {
                        let Some(event_handler) = event_handler.as_ref() else {
                            return;
                        };
                        let mut handler = event_handler.lock().unwrap();
                        handler.on_pause();
                    }
                    MainEvent::Stop => {}
                    MainEvent::Destroy => running = false,
                    MainEvent::InsetsChanged { .. } => {}
                    _ => {}
                },
                _ => (),
            };

            let Some(state) = state.as_mut() else {
                return;
            };
            let Some(render_target) = render_target.as_ref() else {
                return;
            };

            let size = render_target.size();
            if size.0 == 0 || size.1 == 0 {
                return;
            }

            match state.render(render_target.as_ref()) {
                RenderResult::Succeed => {}
                RenderResult::NoSurface | RenderResult::SurfaceLost => {
                    state.recreate_surface(render_target.clone());
                }
            }
        });
    }
}
