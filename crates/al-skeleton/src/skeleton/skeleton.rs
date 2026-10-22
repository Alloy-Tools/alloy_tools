use crate::{
    collider::Collider2d,
    collider_kind::ColliderKind,
    skeleton::{Bone2d, Joint2d},
};
use al_math::{
    sdf::{capsule_sdf_2d, circle_sdf_2d, rect_sdf_2d, Sdf2d},
    transform::Transform2d,
    vec::Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActiveCollider {
    pub target: ColliderRef,
    pub start_frame: u32,
    pub end_frame: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColliderRef {
    Joint(usize),
    Bone(usize),
    Extra(usize),
}

impl std::fmt::Display for ColliderRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColliderRef::Joint(i) => write!(f, "Joint `{i}`"),
            ColliderRef::Bone(i) => write!(f, "Bone `{i}`"),
            ColliderRef::Extra(i) => write!(f, "Extra `{i}`"),
        }
    }
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Skeleton2d {
    joints: Vec<Joint2d>,
    bones: Vec<Bone2d>,
    extras: Vec<Collider2d>,
}

impl Skeleton2d {
    pub(super) fn new() -> Self {
        Self {
            joints: vec![],
            bones: vec![],
            extras: vec![],
        }
    }

    pub fn joint_index(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name() == name)
    }

    pub fn joints(&self) -> &[Joint2d] {
        &self.joints
    }

    pub fn joints_mut(&mut self) -> &mut Vec<Joint2d> {
        &mut self.joints
    }

    pub fn bones(&self) -> &[Bone2d] {
        &self.bones
    }

    pub fn bones_mut(&mut self) -> &mut Vec<Bone2d> {
        &mut self.bones
    }

    pub fn extras(&self) -> &[Collider2d] {
        &self.extras
    }

    pub fn extras_mut(&mut self) -> &mut Vec<Collider2d> {
        &mut self.extras
    }

    pub fn validate(&self) -> Result<(), String> {
        for (i, joint) in self.joints.iter().enumerate() {
            if let Some(p) = joint.parent() {
                if p >= i {
                    return Err(format!(
                        "joint {i} ({}) has parent {p} >= {i} itself",
                        joint.name()
                    ));
                }
            }
        }
        for bone in &self.bones {
            if bone.origin_index() >= self.joints.len() || bone.tip_index() >= self.joints.len() {
                return Err(format!(
                    "bone {} references an out-of-range joint",
                    bone.name()
                ));
            }
        }
        Ok(())
    }

    pub fn world_transforms(&self, pose: &crate::pose::Pose2d) -> Vec<Transform2d> {
        let mut world = vec![Transform2d::IDENTITY; self.joints.len()];
        let root_override =
            Transform2d::new(pose.root_translation, pose.root_rotation, pose.root_scale);
        for (i, joint) in self.joints.iter().enumerate() {
            let delta = pose.rotations.get(i).copied().unwrap_or(0.);
            let local = Transform2d::from_translation(joint.center()).with_rotation(delta);
            world[i] = match joint.parent() {
                None => root_override.compose(&local),
                Some(p) => world[p].compose(&local),
            };
        }

        world
    }

    /// Emit (position, radius) for each joint that has radius > 0.
    pub fn joint_circles<'a>(
        &'a self,
        world: &'a [Transform2d],
    ) -> impl Iterator<Item = (usize, Vec2, f32)> + 'a {
        self.joints
            .iter()
            .enumerate()
            .filter_map(move |(i, joint)| {
                let rad = joint.radius();
                (rad > 0.).then(|| (i, world[i].transform_point(Vec2::ZERO), rad))
            })
    }

    /// Emit (origin, tip, radius) for each bone.
    pub fn bone_capsules<'a>(
        &'a self,
        world: &'a [Transform2d],
    ) -> impl Iterator<Item = (Vec2, Vec2, f32)> + 'a {
        self.bones.iter().map(move |bone| {
            (
                world[bone.origin_index()].transform_point(Vec2::ZERO),
                world[bone.tip_index()].transform_point(Vec2::ZERO),
                bone.radius(),
            )
        })
    }

    /// Signed distance from `p` (world space) to the union of all colliders.
    pub fn body_sdf(&self, world: &[Transform2d], p: Vec2) -> f32 {
        let mut d = f32::INFINITY;

        for (i, joint) in self.joints.iter().enumerate() {
            let rad = joint.radius();
            if rad <= 0. {
                continue;
            }
            d = d.min(circle_sdf_2d(p, rad, world[i].transform_point(Vec2::ZERO)));
        }

        for bone in &self.bones {
            let a = world[bone.origin_index()].transform_point(Vec2::ZERO);
            let b = world[bone.tip_index()].transform_point(Vec2::ZERO);
            d = d.min(capsule_sdf_2d(p, bone.radius(), a, b));
        }

        d
    }

    /// Signed distance from `p` (world space) to the union of all colliders types in `kinds`.
    /// # Example
    /// - let defender_hurt = skeleton.sdf_of_group(&world, p, ColliderGroup::HURT);
    /// - let attacker_hit  = skeleton.sdf_of_group(&world, p, ColliderGroup::HIT);
    pub fn body_sdf_of_group(&self, world: &[Transform2d], p: Vec2, mask: ColliderKind) -> f32 {
        let mut d = f32::INFINITY;
        for (i, joint) in self.joints.iter().enumerate() {
            let rad = joint.radius();
            if rad <= 0. || !joint.kind().intersects(mask) {
                continue;
            }
            d = d.min(circle_sdf_2d(p, rad, world[i].transform_point(Vec2::ZERO)));
        }

        for bone in &self.bones {
            if !bone.kind().intersects(mask) {
                continue;
            }
            let a = world[bone.origin_index()].transform_point(Vec2::ZERO);
            let e = world[bone.tip_index()].transform_point(Vec2::ZERO);
            d = d.min(capsule_sdf_2d(p, bone.radius(), a, e));
        }

        d
    }

    pub fn extra_sdf(&self, world: &[Transform2d], p: Vec2, active: &[bool]) -> f32 {
        let mut d = f32::INFINITY;
        for (i, extra) in self.extras.iter().enumerate() {
            if !active.get(i).copied().unwrap_or(false) {
                continue;
            }
            d = d.min(active_extra(extra, world, p))
        }
        d
    }

    pub fn extra_sdf_of_group(
        &self,
        world: &[Transform2d],
        p: Vec2,
        mask: ColliderKind,
        active: &[bool],
    ) -> f32 {
        let mut d = f32::INFINITY;
        for (i, extra) in self.extras.iter().enumerate() {
            if !active.get(i).copied().unwrap_or(false) || !extra.kind().intersects(mask) {
                continue;
            }
            d = d.min(active_extra(extra, world, p))
        }
        d
    }
}

pub fn active_extra(extra: &Collider2d, world: &[Transform2d], p: Vec2) -> f32 {
    match extra.shape() {
        Sdf2d::Circle {
            origin: center,
            radius,
        } => {
            let c = if let Some(idx) = extra.parent() {
                world[idx].transform_point(center)
            } else {
                center
            };
            circle_sdf_2d(p, radius, c)
        }
        Sdf2d::Capsule {
            origin,
            tip,
            radius,
        } => {
            let (o, t) = if let Some(idx) = extra.parent() {
                (
                    world[idx].transform_point(origin),
                    world[idx].transform_point(tip),
                )
            } else {
                (origin, tip)
            };
            capsule_sdf_2d(p, radius, o, t)
        }
        Sdf2d::Rect {
            origin: center,
            half_size,
            corner_radius,
        } => {
            let c = if let Some(idx) = extra.parent() {
                world[idx].transform_point(center)
            } else {
                center
            };
            rect_sdf_2d(p, c, half_size, corner_radius)
        }
    }
}
