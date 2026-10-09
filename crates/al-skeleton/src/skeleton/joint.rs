use crate::{
    collider::{Collider2d, Shape2d},
    collider_kind::ColliderKind,
};
use al_math::transform::Transform2d;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Joint2d {
    collider: Collider2d,
    depth: f32,
}

impl Joint2d {
    pub fn new(
        name: String,
        parent: Option<usize>,
        transform: Transform2d,
        radius: f32,
        kind: ColliderKind,
        depth: f32,
    ) -> Self {
        Self {
            collider: Collider2d::new(name, parent, Shape2d::circle(transform, radius), kind),
            depth,
        }
    }

    pub fn name(&self) -> &str {
        &self.collider.name()
    }

    pub fn parent(&self) -> Option<usize> {
        self.collider.parent()
    }

    pub fn transform(&self) -> Transform2d {
        self.collider.shape().origin()
    }

    pub fn transform_mut(&mut self) -> &mut Transform2d {
        self.collider.shape_mut().origin_mut()
    }

    pub fn radius(&self) -> f32 {
        self.collider.shape().radius()
    }

    pub fn set_radius(&mut self, r: f32) {
        self.collider.shape().set_radius(r)
    }

    pub fn kind(&self) -> ColliderKind {
        self.collider.kind()
    }

    pub fn set_kind(&mut self, k: ColliderKind) {
        self.collider.set_kind(k)
    }

    pub fn depth(&self) -> f32 {
        self.depth
    }
}
