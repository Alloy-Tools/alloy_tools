use crate::{
    collider::Collider2d,
    collider_kind::ColliderKind,
    skeleton::{Bone2d, Joint2d},
};
use al_math::{
    sdf::{capsule_tip_sdf_2d, circle_sdf_2d, rect_sdf_2d, Sdf2d},
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
            let w = world[bone.origin_index()];
            let a = w.transform_point(Vec2::ZERO);
            let b = world[bone.tip_index()].transform_point(Vec2::ZERO);
            let world_r = world_bone_radius(w.linear, bone.radius(), a, b);
            d = d.min(capsule_tip_sdf_2d(p, world_r, a, b));
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
            let w = world[bone.origin_index()];
            let a = w.transform_point(Vec2::ZERO);
            let b = world[bone.tip_index()].transform_point(Vec2::ZERO);
            let world_r = world_bone_radius(w.linear, bone.radius(), a, b);
            d = d.min(capsule_tip_sdf_2d(p, world_r, a, b));
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

pub fn world_bone_radius(
    linear: al_math::matrix::Mat2,
    local_radius: f32,
    wo: Vec2,
    wt: Vec2,
) -> f32 {
    let dir = wt - wo;
    let length = dir.length();
    if length < 1e-6 {
        return local_radius * linear.determinant().abs().sqrt();
    }
    let inv = linear
        .try_inverse()
        .unwrap_or(al_math::matrix::Mat2::IDENTITY);
    let u_local = (inv * (dir / length)).normalize();
    let perp_local = Vec2::new(-u_local.y, u_local.x);
    (linear * perp_local).length() * local_radius
}

pub fn local_point_and_scale(p: Vec2, parent: Option<usize>, world: &[Transform2d]) -> (Vec2, f32) {
    match parent {
        Some(idx) => {
            let w = &world[idx];
            (
                w.inverse_transform_point(p).unwrap_or(p),
                w.linear.determinant().abs().sqrt(),
            )
        }
        None => (p, 1.0),
    }
}

pub fn active_extra(extra: &Collider2d, world: &[Transform2d], p: Vec2) -> f32 {
    let parent = extra.parent();
    match extra.shape() {
        Sdf2d::Circle { origin, radius } => {
            let (local_p, scale) = local_point_and_scale(p, parent, world);
            circle_sdf_2d(local_p, radius, origin) * scale
        }
        Sdf2d::Capsule {
            origin,
            length,
            radius,
        } => match parent {
            Some(idx) => {
                let w = &world[idx];
                let wo = w.transform_point(origin);
                let wt = w.transform_point(origin + Vec2::new(length, 0.));
                let world_r = world_bone_radius(w.linear, radius, wo, wt);
                capsule_tip_sdf_2d(p, world_r, wo, wt)
            }
            None => capsule_tip_sdf_2d(p, radius, origin, origin + Vec2::new(length, 0.)),
        },
        Sdf2d::Rect { origin, half_size } => {
            let (local_p, scale) = local_point_and_scale(p, parent, world);
            rect_sdf_2d(local_p, origin, half_size) * scale
        }
        _ => todo!(),
    }
}
