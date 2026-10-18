use crate::collider_kind::ColliderKind;
use al_math::sdf::Sdf2d;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Collider2d {
    name: String,
    parent: Option<usize>,
    shape: Sdf2d,
    kind: ColliderKind,
}

impl Collider2d {
    pub fn new(name: String, parent: Option<usize>, shape: Sdf2d, kind: ColliderKind) -> Self {
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

    pub fn shape(&self) -> Sdf2d {
        self.shape
    }

    pub fn shape_mut(&mut self) -> &mut Sdf2d {
        &mut self.shape
    }

    pub fn kind(&self) -> ColliderKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: ColliderKind) {
        self.kind = kind
    }
}
