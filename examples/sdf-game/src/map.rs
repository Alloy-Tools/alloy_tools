use half::f16;

pub const MAP_RES: u32 = 512;
pub const WORLD_MIN: f32 = -1.0;
pub const WORLD_MAX: f32 = 1.0;
pub const WORLD_SIZE: f32 = WORLD_MAX - WORLD_MIN;

pub struct MapData {
    sdf_map: Vec<f16>,
}

impl MapData {
    pub fn new(map_sdf: impl Fn(f32, f32) -> f32) -> Self {
        let res = MAP_RES as usize;
        let mut map = vec![f16::ZERO; res * res];

        for j in 0..res {
            for i in 0..res {
                let world_x = WORLD_MIN + WORLD_SIZE * (i as f32 + 0.5) / res as f32;
                let world_y = WORLD_MAX - WORLD_SIZE * (j as f32 + 0.5) / res as f32;
                map[j * res + i] = f16::from_f32(map_sdf(world_x, world_y));
            }
        }

        Self { sdf_map: map }
    }

    pub fn sdf_map(&self) -> &Vec<f16> {
        &self.sdf_map
    }
}

pub fn map_sdf(x: f32, y: f32) -> f32 {
    let boundary = -rounded_rect_sdf(x, y, 0.9, 0.9, 0.15);

    let c1 = circle_sdf(x, y, 0.4, 0.4, 0.15);
    let c2 = circle_sdf(x, y, -0.4, 0.3, 0.1);
    let c3 = circle_sdf(x, y, 0.0, 0.55, 0.08);
    let cap = capsule_sdf(x, y, -0.3, -0.4, 0.3, -0.4, 0.06);

    boundary.min(c1).min(c2).min(c3).min(cap)
}

pub fn rounded_rect_sdf(x: f32, y: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let qx = x.abs() - (hx - r);
    let qy = y.abs() - (hy - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let inside = qx.max(qy).min(0.0);
    outside + inside - r
}

pub fn circle_sdf(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> f32 {
    let dx = x - cx;
    let dy = y - cy;
    (dx * dx + dy * dy).sqrt() - r
}

pub fn capsule_sdf(x: f32, y: f32, ax: f32, ay: f32, bx: f32, by: f32, r: f32) -> f32 {
    let pax = x - ax;
    let pay = y - ay;
    let bax = bx - ax;
    let bay = by - ay;
    let dot = pax * bax + pay * bay;
    let len2 = bax * bax + bay * bay;
    let h = (dot / len2).clamp(0.0, 1.0);
    let dx = pax - bax * h;
    let dy = pay - bay * h;
    (dx * dx + dy * dy).sqrt() - r
}
