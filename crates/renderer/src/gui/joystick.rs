use egui::{Align2, Context, Order, Pos2, Rect, Rgba, Sense, Stroke, Vec2, Widget};

use crate::renderer::camera::PositionController;

struct Joystick<'a> {
    sense: Sense,
    outer_size: f32,
    inner_size: f32,
    outside_fill_color: Rgba,
    outside_stroke_color: Rgba,
    outside_stroke_width: f32,
    inner_fill_color: Rgba,
    inner_stroke_color: Rgba,
    inner_stroke_width: f32,
    value_x: &'a mut f32,
    value_y: &'a mut f32,
}

impl<'a> Joystick<'a> {
    pub fn new(
        value_x: &'a mut f32,
        value_y: &'a mut f32,
        outer_size: f32,
        inner_size: f32,
    ) -> Self {
        Self {
            sense: Sense::drag(),
            outer_size,
            inner_size,
            outside_fill_color: Rgba::from_rgba_unmultiplied(0.0, 0.0, 0.0, 0.1),
            outside_stroke_color: Rgba::from_rgba_unmultiplied(0.0, 0.0, 0.0, 0.3),
            outside_stroke_width: 1.0,
            inner_fill_color: Rgba::from_rgba_unmultiplied(0.0, 0.0, 0.0, 0.3),
            inner_stroke_color: Rgba::from_rgba_unmultiplied(0.0, 0.0, 0.0, 0.5),
            inner_stroke_width: 1.0,
            value_x,
            value_y,
        }
    }
}

impl<'a> Widget for Joystick<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (_id, response) =
            ui.allocate_exact_size(Vec2::new(self.outer_size, self.outer_size), self.sense);

        fn normalize_pos(x: f32, y: f32) -> (f32, f32) {
            let length = (x * x + y * y).sqrt();
            if length > 1.0 {
                (x / length, y / length)
            } else {
                (x, y)
            }
        }
        fn translate_position(pos: Pos2, rect: &Rect, outer_size: f32) -> (f32, f32) {
            let relative_pos = pos - rect.min;
            let normalized_pos = relative_pos / outer_size;
            let new_x = normalized_pos.x * 2.0 - 1.0;
            let new_y = normalized_pos.y * 2.0 - 1.0;
            normalize_pos(new_x, new_y)
        }
        fn translate_delta(delta: Vec2, orig: Vec2, outer_size: f32) -> (f32, f32) {
            let new_x = orig.x + delta.x / outer_size * 2.0;
            let new_y = orig.y + delta.y / outer_size * 2.0;
            (new_x, new_y)
        }
        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                let (new_x, new_y) = translate_position(pos, &response.rect, self.outer_size);
                *self.value_x = new_x;
                *self.value_y = new_y;
            }
        }
        if response.dragged() {
            let (new_x, new_y) = translate_delta(
                response.drag_motion(),
                Vec2::new(*self.value_x, *self.value_y),
                self.outer_size,
            );
            *self.value_x = new_x;
            *self.value_y = new_y;
        }
        if response.drag_stopped() {
            *self.value_x = 0.0;
            *self.value_y = 0.0;
        }

        let painter = ui.painter();

        let outside_radius = self.outer_size / 2.0;
        let outsize_center = response.rect.center();
        let outside_stroke = Stroke::new(self.outside_stroke_width, self.outside_stroke_color);
        painter.circle(
            outsize_center,
            outside_radius,
            self.outside_fill_color,
            outside_stroke,
        );

        let inner_radius = self.inner_size / 2.0;
        let inner_offset = Vec2::new(
            outside_radius * *self.value_x,
            outside_radius * *self.value_y,
        );
        let inner_center = outsize_center + inner_offset;
        let inner_strode = Stroke::new(self.inner_stroke_width, self.inner_stroke_color);
        painter.circle(
            inner_center,
            inner_radius,
            self.inner_fill_color,
            inner_strode,
        );

        response
    }
}

pub fn joystick(ctx: &Context, position_controller: &mut PositionController) {
    egui::Area::new(egui::Id::new("Joystick"))
        .anchor(Align2::LEFT_BOTTOM, (32.0, -32.0))
        .order(Order::Background)
        .show(ctx, |ui| {
            let orig_x = position_controller.right - position_controller.left;
            let orig_y = position_controller.backward - position_controller.forward;
            let mut x = orig_x;
            let mut y = orig_y;
            Joystick::new(&mut x, &mut y, 192.0, 48.0).ui(ui);
            if x != orig_x {
                position_controller.left = -(x.max(0.0));
                position_controller.right = x.min(0.0);
            }
            if y != orig_y {
                position_controller.forward = -(y.max(0.0));
                position_controller.backward = y.min(0.0);
            }
        });
}
