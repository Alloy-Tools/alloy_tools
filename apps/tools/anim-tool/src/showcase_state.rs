use crate::{
    camera::Camera,
    common_state::{CameraState, InputState, Timeline},
};

pub struct ShowcaseState {
    timeline: Timeline,
    camera_state: CameraState,
    keys: InputState,
}

impl ShowcaseState {
    pub fn new() -> Self {
        Self {
            timeline: Timeline::new(0.),
            camera_state: CameraState::new([800., 600.], Camera::new([0., 0.], 1.4)),
            keys: InputState::default(),
        }
    }

    pub fn hud_string(&self) -> String {
        format!(
            "{} {:.3}/{:.3} loop {}",
            if self.timeline.playing() {
                "▶"
            } else {
                "▮▮"
            },
            self.timeline.time(),
            self.timeline.duration(),
            if self.timeline.looping() { "on" } else { "off" }
        )
    }

    pub fn update(&mut self) {}

    pub fn window_event(&mut self, event: winit::event::WindowEvent) {}
}
