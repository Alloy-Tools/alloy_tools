use crate::collider_kind::ColliderKind;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bone2d {
    name: String,
    origin_index: usize,
    tip_index: usize,
    radius: f32,
    kind: ColliderKind,
    depth: f32,
}

impl Bone2d {
    pub fn new(
        name: String,
        origin_index: usize,
        tip_index: usize,
        radius: f32,
        kind: ColliderKind,
        depth: f32,
    ) -> Self {
        Self {
            name,
            origin_index,
            tip_index,
            radius,
            kind,
            depth,
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

    pub fn kind(&self) -> ColliderKind {
        self.kind
    }

    pub fn depth(&self) -> f32 {
        self.depth
    }
}
