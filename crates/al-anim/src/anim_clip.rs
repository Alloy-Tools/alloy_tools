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
    const EPS: f32 = 1. / 240.;

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            duration: 0.,
            looping: false,
            keyframes: Vec::new(),
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

    pub fn insert(&mut self, time: f32, pose: Pose2d) {
        let pos = self
            .keyframes
            .partition_point(|kf| kf.time < time - Self::EPS);
        if pos < self.keyframes.len() && (self.keyframes[pos].time - time).abs() < Self::EPS {
            self.keyframes[pos].pose = pose;
            return;
        }
        self.keyframes.insert(pos, KeyFrame2d { time, pose });

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
        a.pose.blend(&b.pose, u)
    }
}
