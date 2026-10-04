use crate::collider::ColliderGroup;
use al_math::{transform::Transform2D, vec::Vec2};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Joint2D {
    name: String,
    parent: Option<usize>,
    rest_transform: Transform2D,
    radius: f32,
    group: ColliderGroup,
}

impl Joint2D {
    pub fn new(
        name: String,
        parent: Option<usize>,
        rest_transform: Transform2D,
        radius: f32,
        group: ColliderGroup,
    ) -> Self {
        Self {
            name,
            parent,
            rest_transform,
            radius,
            group,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn parent(&self) -> Option<usize> {
        self.parent
    }

    pub fn rest_transform(&self) -> Transform2D {
        self.rest_transform
    }

    pub fn rest_transform_mut(&mut self) -> &mut Transform2D {
        &mut self.rest_transform
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn set_radius(&mut self, r: f32) {
        self.radius = r;
    }

    pub fn group(&self) -> ColliderGroup {
        self.group
    }

    pub fn set_group(&mut self, g: ColliderGroup) {
        self.group = g;
    }

    #[inline]
    pub fn circle_sdf(&self, point: Vec2, origin: Vec2) -> f32 {
        (point - origin).length() - self.radius
    }
}
