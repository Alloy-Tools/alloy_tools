use crate::{
    camera::Camera,
    common_state::{CameraState, Globals, InputState, RenderData, Timeline},
};
use al_math::{sdf::Sdf2dPrim, vec::Vec2};

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

    pub fn update(&mut self) {
        self.camera_state
            .update(self.timeline.tick(), !self.keys.shift_held);
    }

    pub fn window_event(&mut self, event: winit::event::WindowEvent) {
        
    }

    pub fn to_render(&mut self, resolution: impl Into<Vec2>) -> RenderData {
        let resolution = resolution.into();
        self.camera_state.set_resolution(resolution);

        let cam = self.camera_state.camera();
        let c_px = self.camera_state.cursor_px();
        let cx = c_px.x - 0.5 * resolution.x;
        let cy = c_px.y - 0.5 * resolution.y;
        let c_world = cam.pos
            + [
                cx / resolution.y * cam.view_size,
                -cy / resolution.y * cam.view_size,
            ];
        self.camera_state.set_cursor_world(c_world);

        let pos = Vec2::ZERO;
        let radius = 0.5;
        let m_inv = al_math::matrix::Mat2::IDENTITY.to_cols_array();
        let color = [0.9, 0.9, 0.9, 1.];
        let mut sdfs = Vec::new();

        sdfs.push(Sdf2dPrim::circle(pos, radius, m_inv, color));

        // Cursor
        sdfs.push(Sdf2dPrim::circle(c_world, 0.006, m_inv, [0.4, 1., 0.6, 1.]));

        RenderData::new(
            Globals::new(resolution, cam.pos, c_world, cam.view_size),
            sdfs,
            self.hud_string(),
        )
    }
}
