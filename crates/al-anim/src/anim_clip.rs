use al_skeleton::pose::Pose2d;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KeyFrame2d {
    pub time: f32,
    pub pose: Pose2d,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AnimationClip2d {
    name: String,
    duration: f32,
    looping: bool,
    keyframes: Vec<KeyFrame2d>,
}

impl AnimationClip2d {
    /// Sample the clip at `t` seconds. Loops if `self.looping`.
    pub fn sample(&self, t: f32) -> Pose2d {
        debug_assert!(!self.keyframes.is_empty(), "empty clip `{}`", self.name);

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
        a.pose.blend(&b.pose, u)
    }
}
