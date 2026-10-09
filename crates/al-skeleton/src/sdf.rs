use al_math::vec::Vec2;

#[inline]
pub fn circle_sdf(radius: f32, point: Vec2, origin: Vec2) -> f32 {
    (point - origin).length() - radius
}

#[inline]
pub fn capsule_sdf(radius: f32, point: Vec2, origin: Vec2, tip: Vec2) -> f32 {
    let po = point - origin;
    let to = tip - origin;
    let h = (po.dot(to) / to.dot(to).max(1e-8)).clamp(0., 1.);
    (po - to * h).length() - radius
}
