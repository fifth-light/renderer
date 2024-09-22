use egui::{Align2, Context, Window};
use egui_plot::{Line, Plot, PlotPoints};

use crate::perf::PerformanceTracker;

pub fn perf_info(ctx: &Context, perf_tracker: &PerformanceTracker) {
    Window::new("Performance Info")
        .resizable([false, false])
        .pivot(Align2::RIGHT_BOTTOM)
        .show(ctx, |ui| {
            Plot::new("Frame time")
                .height(120.0)
                .x_axis_label("Samples")
                .y_axis_label("Frame Time")
                .show(ui, |ui| {
                    let points = perf_tracker
                        .frame_time()
                        .iter()
                        .enumerate()
                        .map(|(index, time)| [index as f64, time.as_nanos() as f64 / 1_000_000.0])
                        .collect();
                    let points = PlotPoints::new(points);
                    let line = Line::new(points);
                    ui.line(line);
                });
            match perf_tracker.last_frame_time() {
                Some(time) => {
                    ui.label(format!("Frame time: {}ms", time.as_millis()));
                }
                None => {
                    ui.label("Frame time: unknown");
                }
            };
            match perf_tracker.fps() {
                Some(fps) => {
                    ui.label(format!("FPS: {:#.2}", fps));
                }
                None => {
                    ui.label("FPS: unknown");
                }
            }
        });
}
