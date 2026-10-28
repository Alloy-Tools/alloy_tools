use al_math::vec::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub pos: Vec2,
    pub view_size: f32,
    pub follow_speed: f32,
}

impl Camera {
    pub fn new(pos: impl Into<Vec2>, view_size: f32) -> Self {
        Self {
            pos: pos.into(),
            view_size,
            follow_speed: 8.,
        }
    }

    pub fn update(&mut self, target: impl Into<Vec2>, dt: f32) {
        let target = target.into();
        let factor = 1.0 - (-self.follow_speed * dt).exp();
        self.pos.x += (target.x - self.pos.x) * factor;
        self.pos.y += (target.y - self.pos.y) * factor;
    }
}
