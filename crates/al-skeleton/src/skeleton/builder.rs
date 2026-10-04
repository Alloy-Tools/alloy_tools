use crate::{
    collider::ColliderGroup,
    skeleton::{bone::Bone2D, joint::Joint2D, Skeleton2D},
};
use al_math::transform::Transform2D;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Skeleton2DBuilder {
    skeleton: Skeleton2D,
}

impl Skeleton2DBuilder {
    pub fn new() -> Self {
        Self {
            skeleton: Skeleton2D::new(),
        }
    }

    pub fn joint(
        &mut self,
        name: String,
        parent: Option<usize>,
        local: Transform2D,
        radius: f32,
        group: ColliderGroup,
    ) -> usize {
        let joints = self.skeleton.joints_mut();
        let idx = joints.len();
        if let Some(p) = parent {
            assert!(p < idx);
        }
        joints.push(Joint2D::new(name, parent, local, radius, group));
        idx
    }

    pub fn bone(
        &mut self,
        name: String,
        origin_index: usize,
        tip_index: usize,
        radius: f32,
        group: ColliderGroup,
    ) {
        self.skeleton
            .bones_mut()
            .push(Bone2D::new(name, origin_index, tip_index, radius, group));
    }

    pub fn build(self) -> Skeleton2D {
        self.skeleton
    }
}
