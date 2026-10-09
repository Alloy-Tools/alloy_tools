use crate::{
    collider::{Collider2d, Shape2d},
    collider_kind::ColliderKind,
    skeleton::{bone::Bone2d, joint::Joint2d, Skeleton2d},
};

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
        name: impl Into<String>,
        parent: Option<usize>,
        transform: al_math::transform::Transform2d,
        radius: f32,
        kind: ColliderKind,
        depth: f32,
    ) -> usize {
        let joints = self.skeleton.joints_mut();
        let idx = joints.len();
        if let Some(p) = parent {
            assert!(p < idx);
        }
        joints.push(Joint2d::new(
            name.into(),
            parent,
            transform,
            radius,
            kind,
            depth,
        ));
        idx
    }

    pub fn bone(
        &mut self,
        name: impl Into<String>,
        origin_index: usize,
        tip_index: usize,
        radius: f32,
        kind: ColliderKind,
        depth: f32,
    ) {
        self.skeleton.bones_mut().push(Bone2d::new(
            name.into(),
            origin_index,
            tip_index,
            radius,
            kind,
            depth,
        ));
    }

    pub fn extra(
        &mut self,
        name: impl Into<String>,
        parent: Option<usize>,
        shape: Shape2d,
        kind: ColliderKind,
    ) -> usize {
        let extras = self.skeleton.extras_mut();
        let idx = extras.len();
        extras.push(Collider2d::new(name.into(), parent, shape, kind));
        idx
    }

    pub fn build(self) -> Skeleton2d {
        self.skeleton
    }
}
