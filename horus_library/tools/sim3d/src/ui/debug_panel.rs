#[cfg(feature = "visual")]
use bevy::prelude::*;
#[cfg(feature = "visual")]
use bevy_egui::{egui, EguiContexts};

#[cfg(feature = "visual")]
pub fn debug_panel_system(mut contexts: EguiContexts, time: Res<Time>) {
    egui::Window::new("Debug Panel").show(contexts.ctx_mut(), |ui| {
        ui.heading("sim3d - HORUS 3D Simulator");
        ui.separator();

        ui.label(format!("FPS: {:.1}", 1.0 / time.delta_secs()));
        ui.label(format!("Delta: {:.3}ms", time.delta_secs() * 1000.0));

        ui.separator();
        ui.label("Controls:");
        ui.label("  Right Mouse: Rotate camera");
        ui.label("  Middle Mouse: Pan camera");
        ui.label("  Mouse Wheel: Zoom");
        ui.label("  WASD/Arrows: Move camera focus");
        ui.label("  Q/E: Move focus up/down");
    });
}

#[cfg(not(feature = "visual"))]
pub fn debug_panel_system() {}
