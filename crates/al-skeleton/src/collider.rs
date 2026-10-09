use crate::collider_kind::ColliderKind;
use al_math::transform::Transform2d;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Shape2d {
    Circle {
        origin: Transform2d,
        radius: f32,
    },
    Capsule {
        origin: Transform2d,
        tip: Transform2d,
        radius: f32,
    },
}

impl Shape2d {
    pub fn circle(origin: Transform2d, radius: f32) -> Self {
        Self::Circle { origin, radius }
    }

    pub fn capsule(origin: Transform2d, tip: Transform2d, radius: f32) -> Self {
        Self::Capsule {
            origin,
            tip,
            radius,
        }
    }

    pub fn radius(&self) -> f32 {
        match self {
            Shape2d::Circle { radius, .. } => *radius,
            Shape2d::Capsule { radius, .. } => *radius,
        }
    }

    pub fn set_radius(&mut self, radius: f32) {
        *match self {
            Shape2d::Circle { radius, .. } => radius,
            Shape2d::Capsule { radius, .. } => radius,
        } = radius;
    }

    pub fn origin(&self) -> Transform2d {
        match self {
            Shape2d::Circle { origin, .. } => *origin,
            Shape2d::Capsule { origin, .. } => *origin,
        }
    }

    pub fn set_origin(&mut self, origin: Transform2d) {
        *match self {
            Shape2d::Circle { origin, .. } => origin,
            Shape2d::Capsule { origin, .. } => origin,
        } = origin;
    }

    pub fn origin_mut(&mut self) -> &mut Transform2d {
        match self {
            Shape2d::Circle { origin, .. } => origin,
            Shape2d::Capsule { origin, .. } => origin,
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Collider2d {
    name: String,
    parent: Option<usize>,
    shape: Shape2d,
    kind: ColliderKind,
}

impl Collider2d {
    pub fn new(name: String, parent: Option<usize>, shape: Shape2d, kind: ColliderKind) -> Self {
        Self {
            name,
            parent,
            shape,
            kind,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn parent(&self) -> Option<usize> {
        self.parent
    }

    pub fn shape(&self) -> Shape2d {
        self.shape
    }

    pub fn shape_mut(&mut self) -> &mut Shape2d {
        &mut self.shape
    }

    pub fn kind(&self) -> ColliderKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: ColliderKind) {
        self.kind = kind
    }
}
