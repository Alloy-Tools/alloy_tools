use crate::{
    collider::ColliderGroup,
    skeleton::{Bone2d, Joint2d},
};
use al_math::{transform::Transform2D, vec::Vec2};

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Skeleton2d {
    joints: Vec<Joint2d>,
    bones: Vec<Bone2d>,
}

impl Skeleton2d {
    pub(super) fn new() -> Self {
        Self {
            joints: vec![],
            bones: vec![],
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

    pub fn world_transforms(&self, pose: &crate::pose::Pose2d) -> Vec<Transform2D> {
        let mut world = vec![Transform2D::IDENTITY; self.joints.len()];
        let root_override =
            Transform2D::new(pose.root_translation, pose.root_rotation, pose.root_scale);
        for (i, joint) in self.joints.iter().enumerate() {
            let delta = pose.rotations.get(i).copied().unwrap_or(0.);
            let local = joint
                .rest_transform()
                .compose(&Transform2D::from_rotation(delta));
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
        world: &'a [Transform2D],
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
        world: &'a [Transform2D],
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
    pub fn sdf(&self, world: &[Transform2D], p: Vec2) -> f32 {
        let mut d = f32::INFINITY;

        for (i, joint) in self.joints.iter().enumerate() {
            if joint.radius() <= 0. {
                continue;
            }
            let c = world[i].transform_point(Vec2::ZERO);
            d = d.min(joint.circle_sdf(p, c));
        }

        for bone in &self.bones {
            let a = world[bone.origin_index()].transform_point(Vec2::ZERO);
            let b = world[bone.tip_index()].transform_point(Vec2::ZERO);
            d = d.min(bone.capsule_sdf(p, a, b));
        }

        d
    }

    /// Signed distance from `p` (world space) to the union of all colliders types in `kinds`.
    /// # Example
    /// - let defender_hurt = skeleton.sdf_of_group(&world, p, ColliderGroup::HURT);
    /// - let attacker_hit  = skeleton.sdf_of_group(&world, p, ColliderGroup::HIT);
    pub fn sdf_of_group(&self, world: &[Transform2D], p: Vec2, mask: ColliderGroup) -> f32 {
        let mut d = f32::INFINITY;
        for (i, joint) in self.joints.iter().enumerate() {
            if joint.radius() <= 0. || !joint.group().intersects(mask) {
                continue;
            }
            let c = world[i].transform_point(Vec2::ZERO);
            d = d.min(joint.circle_sdf(p, c));
        }

        for bone in &self.bones {
            if !bone.group().intersects(mask) {
                continue;
            }
            let a = world[bone.origin_index()].transform_point(Vec2::ZERO);
            let e = world[bone.tip_index()].transform_point(Vec2::ZERO);
            d = d.min(bone.capsule_sdf(p, a, e));
        }

        d
    }
}
