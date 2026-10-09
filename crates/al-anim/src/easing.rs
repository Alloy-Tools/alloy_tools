#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Easing {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Step,
}

impl std::fmt::Display for Easing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Easing::Linear => write!(f, "Linear"),
            Easing::EaseIn => write!(f, "EaseIn"),
            Easing::EaseOut => write!(f, "EaseOut"),
            Easing::EaseInOut => write!(f, "EaseInOut"),
            Easing::Step => write!(f, "Step"),
        }
    }
}

impl Easing {
    pub const All: [Easing; 5] = [
        Self::Linear,
        Self::EaseIn,
        Self::EaseOut,
        Self::EaseInOut,
        Self::Step,
    ];

    pub fn next(self) -> Self {
        let i = Self::All.iter().position(|e| self == *e).unwrap_or(0);
        Self::All[(i + 1) % Self::All.len()]
    }

    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0., 1.);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1. - (1. - t) * (1. - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2. * t * t
                } else {
                    1. - (-2. * t + 2.) * (-2. * t + 2.) * 0.5
                }
            }
            Easing::Step => {
                if t >= 1. {
                    1.
                } else {
                    0.
                }
            }
        }
    }
}
