use al_math::vec::Vec2;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pose2d {
    /// One local rotation per joint.
    pub rotations: Vec<f32>,
    /// Whole-rig offset, applied at the root.
    pub root_translation: Vec2,
    pub root_rotation: f32,
    pub root_scale: Vec2,
}

impl Default for Pose2d {
    fn default() -> Self {
        Self {
            rotations: Vec::new(),
            root_translation: Vec2::ZERO,
            root_rotation: 0.,
            root_scale: Vec2::ONE,
        }
    }
}

impl Pose2d {
    pub fn rest(num_joints: usize) -> Self {
        Self {
            rotations: vec![0.0; num_joints],
            root_translation: Vec2::ZERO,
            root_rotation: 0.,
            root_scale: Vec2::ONE,
        }
    }

    pub fn blend(&self, b: &Pose2d, t: f32) -> Pose2d {
        Pose2d::lerp(self, b, t)
    }

    pub fn lerp(a: &Pose2d, b: &Pose2d, t: f32) -> Pose2d {
        debug_assert_eq!(a.rotations.len(), b.rotations.len());
        Pose2d {
            rotations: a
                .rotations
                .iter()
                .zip(&b.rotations)
                .map(|(x, y)| lerp_angle(*x, *y, t))
                .collect(),
            root_translation: a.root_translation.lerp(b.root_translation, t),
            root_rotation: lerp_angle(a.root_rotation, b.root_rotation, t),
            root_scale: a.root_scale.lerp(b.root_scale, t),
        }
    }

    /// Additive layering: `other` is applied on top of `self`.
    pub fn add(&self, other: &Pose2d) -> Pose2d {
        debug_assert_eq!(self.rotations.len(), other.rotations.len());
        Pose2d {
            rotations: self
                .rotations
                .iter()
                .zip(&other.rotations)
                .map(|(a, b)| a + b)
                .collect(),
            root_translation: self.root_translation + other.root_translation,
            root_rotation: self.root_rotation + other.root_rotation,
            root_scale: Vec2::new(
                self.root_scale.x * other.root_scale.x,
                self.root_scale.y * other.root_scale.y,
            ),
        }
    }
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    use std::f32::consts::PI;
    let mut d = (b - a).rem_euclid(2.0 * PI);
    if d > PI {
        d -= 2.0 * PI;
    }
    a + d * t
}
