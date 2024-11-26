use egui::{Align2, Context, Window};
use renderer_perf_tracker::PerformanceTracker;

pub fn perf_info(ctx: &Context, perf_tracker: &PerformanceTracker) {
    Window::new("Performance Info")
        .resizable([false, false])
        .pivot(Align2::RIGHT_BOTTOM)
        .show(ctx, |ui| {
            match perf_tracker.last_frame_time() {
                Some(time) => {
                    ui.label(format!("Frame time: {}ms", time.as_millis()));
                }
                None => {
                    ui.label("Frame time: unknown");
                }
            };

            match perf_tracker.avg_frame_time() {
                Some(time) => {
                    ui.label(format!("Avg frame time: {}ms", time.as_millis()));
                }
                None => {
                    ui.label("Avg frame time: unknown");
                }
            };

            match perf_tracker.fps() {
                Some(fps) => {
                    ui.label(format!("FPS: {:#.2}", fps));
                }
                None => {
                    ui.label("FPS: unknown");
                }
            };
        });
}
