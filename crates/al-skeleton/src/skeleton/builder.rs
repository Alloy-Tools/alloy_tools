use crate::{
    collider::ColliderGroup,
    skeleton::{bone::Bone2d, joint::Joint2d, Skeleton2d},
};
use al_math::transform::Transform2D;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Skeleton2DBuilder {
    skeleton: Skeleton2d,
}

impl Skeleton2DBuilder {
    pub fn new() -> Self {
        Self {
            skeleton: Skeleton2d::new(),
        }
    }

    pub fn joint(
        &mut self,
        name: String,
        parent: Option<usize>,
        local: Transform2D,
        radius: f32,
        group: ColliderGroup,
        depth: f32,
    ) -> usize {
        let joints = self.skeleton.joints_mut();
        let idx = joints.len();
        if let Some(p) = parent {
            assert!(p < idx);
        }
        joints.push(Joint2d::new(name, parent, local, radius, group, depth));
        idx
    }

    pub fn bone(
        &mut self,
        name: String,
        origin_index: usize,
        tip_index: usize,
        radius: f32,
        group: ColliderGroup,
        depth: f32,
    ) {
        self.skeleton.bones_mut().push(Bone2d::new(
            name,
            origin_index,
            tip_index,
            radius,
            group,
            depth,
        ));
    }

    pub fn build(self) -> Skeleton2d {
        self.skeleton
    }
}
