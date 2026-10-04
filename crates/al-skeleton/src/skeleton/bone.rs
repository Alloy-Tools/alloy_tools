use crate::collider::ColliderGroup;
use al_math::vec::Vec2;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bone2D {
    name: String,
    origin_index: usize,
    tip_index: usize,
    radius: f32,
    group: ColliderGroup,
}

impl Bone2D {
    pub fn new(
        name: String,
        origin_index: usize,
        tip_index: usize,
        radius: f32,
        group: ColliderGroup,
    ) -> Self {
        Self {
            name,
            origin_index,
            tip_index,
            radius,
            group,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn origin_index(&self) -> usize {
        self.origin_index
    }

    pub fn tip_index(&self) -> usize {
        self.tip_index
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn group(&self) -> ColliderGroup {
        self.group
    }

    #[inline]
    pub fn capsule_sdf(&self, point: Vec2, origin: Vec2, tip: Vec2) -> f32 {
        let po = point - origin;
        let to = tip - origin;
        let h = (po.dot(to) / to.dot(to).max(1e-8)).clamp(0., 1.);
        (po - to * h).length() - self.radius
    }
}
