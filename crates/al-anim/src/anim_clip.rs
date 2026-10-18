use crate::{vfx::Vfx2d, Easing};
use al_math::{
    sdf::Sdf2dPrim,
    vec::{Vec2, Vec4},
};
use al_skeleton::pose::Pose2d;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KeyFrame2d {
    pub time: f32,
    pub pose: Pose2d,
    pub easing: Easing,
}

// TODO: Wire up camera insert/delete keybinds
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CameraKeyFrame2d {
    pub time: f32,
    pub pos: [f32; 2],
    pub view_size: f32,
    pub easing: Easing,
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttachType {
    /// Follows `joint` every frame
    Follow { joint: usize, offset: Vec2 },
    /// World space, starting from `joint`
    From { joint: usize, offset: Vec2 },
    /// World space
    World { pos: Vec2 },
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AnimationClip2d {
    name: String,
    duration: f32,
    looping: bool,
    keyframes: Vec<KeyFrame2d>,
    camera_keys: Vec<CameraKeyFrame2d>,
    vfx: Vec<Vfx2d>,
}

impl AnimationClip2d {
    const EPS: f32 = 1. / 240.;

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            duration: 0.,
            looping: false,
            keyframes: Vec::new(),
            camera_keys: Vec::new(),
            vfx: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    pub fn looping(&self) -> bool {
        self.looping
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping
    }

    pub fn keyframes(&self) -> &[KeyFrame2d] {
        &self.keyframes
    }

    pub fn camera_keyframes(&self) -> &[CameraKeyFrame2d] {
        &self.camera_keys
    }

    pub fn vfx(&self) -> &[Vfx2d] {
        &self.vfx
    }

    pub fn vfx_mut(&mut self) -> &mut Vec<Vfx2d> {
        &mut self.vfx
    }

    pub fn insert(&mut self, time: f32, pose: Pose2d, easing: Easing) {
        let pos = self
            .keyframes
            .partition_point(|kf| kf.time < time - Self::EPS);
        if pos < self.keyframes.len() && (self.keyframes[pos].time - time).abs() < Self::EPS {
            self.keyframes[pos].pose = pose;
            return;
        }
        self.keyframes
            .insert(pos, KeyFrame2d { time, pose, easing });

        if time > self.duration {
            self.duration = time;
        }
    }

    pub fn remove_at(&mut self, time: f32, tolerance: f32) -> bool {
        if let Some(pos) = self
            .keyframes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (a.time - time)
                    .abs()
                    .partial_cmp(&(b.time - time).abs())
                    .unwrap()
            })
            .filter(|(_, kf)| (kf.time - time).abs() <= tolerance)
            .map(|(i, _)| i)
        {
            self.keyframes.remove(pos);
            if let Some(last) = self.keyframes.last() {
                self.duration = self.duration.max(last.time);
            }
            true
        } else {
            false
        }
    }

    pub fn set_easing_at(&mut self, time: f32, easing: Easing, tolerance: f32) -> bool {
        let Some(pos) = self
            .keyframes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (a.time - time)
                    .abs()
                    .partial_cmp(&(b.time - time).abs())
                    .unwrap()
            })
            .filter(|(_, kf)| (kf.time - time).abs() <= tolerance)
            .map(|(i, _)| i)
        else {
            return false;
        };
        self.keyframes[pos].easing = easing;
        true
    }

    pub fn easing_at(&self, time: f32, tolerance: f32) -> Option<Easing> {
        self.keyframes
            .iter()
            .min_by(|a, b| {
                (a.time - time)
                    .abs()
                    .partial_cmp(&(b.time - time).abs())
                    .unwrap()
            })
            .filter(|kf| (kf.time - time).abs() <= tolerance)
            .map(|kf| kf.easing)
    }

    pub fn nearest_keyframe_time(&self, t: f32) -> Option<f32> {
        self.keyframes
            .iter()
            .min_by(|a, b| (a.time - t).abs().partial_cmp(&(b.time - t).abs()).unwrap())
            .map(|kf| kf.time)
    }

    /// Sample the clip at `t` seconds. Loops if `self.looping`.
    pub fn sample(&self, t: f32) -> Pose2d {
        if self.keyframes.is_empty() {
            return Pose2d::default();
        } else if self.keyframes.len() == 1 {
            return self.keyframes[0].pose.clone();
        }

        let t = if self.looping && self.duration > 0. {
            t.rem_euclid(self.duration)
        } else {
            t.clamp(0., self.duration)
        };

        let i = self.keyframes.partition_point(|kf| kf.time <= t);

        if i == 0 {
            return self.keyframes[0].pose.clone();
        } else if i == self.keyframes.len() {
            return self.keyframes.last().unwrap().pose.clone();
        }

        let a = &self.keyframes[i - 1];
        let b = &self.keyframes[i];
        let span = (b.time - a.time).max(f32::EPSILON);
        let u = ((t - a.time) / span).clamp(0., 1.);
        let e = a.easing.apply(u);
        a.pose.blend(&b.pose, e)
    }

    pub fn vfx_instances(
        &self,
        t: f32,
        skeleton: &al_skeleton::skeleton::Skeleton2d,
    ) -> Vec<Sdf2dPrim> {
        if self.keyframes.is_empty() {
            return Vec::new();
        }
        let t = if self.looping && self.duration > 0. {
            t.rem_euclid(self.duration)
        } else {
            t.clamp(0., self.duration)
        };

        let mut out = Vec::new();
        //REVIEW: Should this use easing as well?
        let curr_world = skeleton.world_transforms(&self.sample(t));

        for vfx in &self.vfx {
            let st = vfx.start_time();
            let dur = vfx.duration();
            if t < st || t > st + dur {
                continue;
            }

            let u = ((t - st) / dur.max(1e-6)).clamp(0., 1.);
            let e = vfx.easing().apply(u);
            let scale = lerp(vfx.start_scale(), vfx.end_scale(), e);
            let alpha = lerp(vfx.start_alpha(), vfx.end_alpha(), e);
            let rotation = lerp(vfx.start_rotation(), vfx.end_rotation(), e);
            //REVIEW: should this ease?
            let motion = vfx.motion() * u;

            let (frame_tf, frame_offset) = match vfx.attach_type() {
                AttachType::Follow { joint, offset } => (curr_world[joint], offset),
                AttachType::From { joint, offset } => {
                    (skeleton.world_transforms(&self.sample(st))[joint], offset)
                }
                AttachType::World { pos } => (al_math::transform::Transform2d::IDENTITY, pos),
            };

            let col = vfx.color();
            out.push(vfx.shape().to_prim(
                |p: Vec2| {
                    let r = p.rotate(rotation) * scale + frame_offset;
                    frame_tf.transform_point(r) + motion
                },
                Vec4::new(col.x, col.y, col.z, col.w * alpha),
            ));
        }

        out
    }
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
