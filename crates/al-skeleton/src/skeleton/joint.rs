use crate::{
    collider::Collider2d,
    collider_kind::ColliderKind,
};
use al_math::{sdf::{Sdf2d, Sdf2dKind}, transform::Transform2d, vec::Vec2};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Joint2d {
    collider: Collider2d,
    rotation: f32,
    depth: f32,
}

impl Joint2d {
    pub fn new(
        name: String,
        parent: Option<usize>,
        center: Vec2,
        radius: f32,
        rotation: f32,
        kind: ColliderKind,
        depth: f32,
    ) -> Self {
        Self {
            collider: Collider2d::new(name, parent, Sdf2d::new(Transform2d::from_translation(center), Sdf2dKind::circle(radius)), kind),
            rotation,
            depth,
        }
    }

    pub fn name(&self) -> &str {
        &self.collider.name()
    }

    pub fn parent(&self) -> Option<usize> {
        self.collider.parent()
    }

    pub fn center(&self) -> Vec2 {
        self.collider.shape().transform().translation
    }

    pub fn set_center(&mut self, center: Vec2) {
        let trans = self.collider.shape_mut().transform_mut();
        *trans = trans.with_translation(center);
    }

    pub fn radius(&self) -> f32 {
        let Sdf2dKind::Circle { radius } = self.collider.shape().shape() else {
            panic!("Joint has shape `{}` instead of `Circle`", self.collider.shape().shape().name())
        };
        radius
    }

    pub fn set_radius(&mut self, radius: f32) {
        let Sdf2dKind::Circle { radius: rad } = self.collider.shape_mut().shape_mut() else {
            panic!("Joint has shape `{}` instead of `Circle`", self.collider.shape().shape().name())
        };
        *rad = radius;
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    pub fn set_rotation(&mut self, rotation: f32) {
        self.rotation = rotation;
    }

    pub fn kind(&self) -> ColliderKind {
        self.collider.kind()
    }

    pub fn set_kind(&mut self, kind: ColliderKind) {
        self.collider.set_kind(kind);
    }

    pub fn depth(&self) -> f32 {
        self.depth
    }
}
