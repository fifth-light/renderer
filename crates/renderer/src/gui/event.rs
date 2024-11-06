use egui::{Context, PlatformOutput, RawInput};

pub trait GuiEventHandler: Send + Sync {
    fn egui_context(&self) -> &Context;
    fn take_egui_input(&mut self) -> RawInput;
    fn handle_platform_output(&mut self, platform_output: PlatformOutput);
}
