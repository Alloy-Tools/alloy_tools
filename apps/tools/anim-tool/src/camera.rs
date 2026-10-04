pub struct Camera {
    pub pos: [f32; 2],
    pub view_size: f32,
    pub follow_speed: f32,
}

impl Camera {
    pub fn new(pos: [f32; 2], view_size: f32) -> Self {
        Self {
            pos,
            view_size,
            follow_speed: 8.,
        }
    }

    pub fn update(&mut self, target: [f32; 2], dt: f32) {
        let factor = 1.0 - (-self.follow_speed * dt).exp();
        self.pos[0] += (target[0] - self.pos[0]) * factor;
        self.pos[1] += (target[1] - self.pos[1]) * factor;
    }
}