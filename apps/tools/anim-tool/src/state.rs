use std::time::Instant;

use al_anim::anim_clip::AnimationClip2d;
use al_skeleton::{pose::Pose2d, skeleton::Skeleton2d};

use crate::Camera;

#[derive(Clone, Copy, PartialEq)]
pub enum EditMode {
    Pose,
    Rest,
}

impl std::fmt::Display for EditMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditMode::Pose => write!(f, "Pose"),
            EditMode::Rest => write!(f, "Rest"),
        }
    }
}

impl EditMode {
    pub fn cycle(&mut self) {
        *self = match self {
            EditMode::Pose => EditMode::Rest,
            EditMode::Rest => EditMode::Pose,
        }
    }

    pub fn status(&self) -> String {
        match self {
            EditMode::Pose => "Editing 'Pose' (rotations)".into(),
            EditMode::Rest => "Editing 'Rest' (translations)".into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum DragMode {
    MoveJoint,
    RotateJoint,
}

#[derive(Clone)]
pub enum Snapshot {
    Skeleton(Skeleton2d),
    Pose(Pose2d),
    Animation(Vec<AnimationClip2d>, usize),
}

pub struct History {
    pub undo: Vec<Snapshot>,
    pub redo: Vec<Snapshot>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn record(&mut self, snapshot: Snapshot) {
        self.undo.push(snapshot);
    }
}

#[derive(Default, Clone, Copy)]
pub struct InputState {
    pub ctrl_held: bool,
    pub shift_held: bool,
    pub alt_held: bool,
}

pub struct DesignerState {
    pub skeleton: Skeleton2d,
    pub pose: Pose2d,

    pub selected: Option<usize>,
    pub drag: Option<DragMode>,
    pub edit_mode: EditMode,

    pub clips: Vec<AnimationClip2d>,
    pub current_clip: usize,
    pub time: f32,
    pub playing: bool,
    pub last_tick: Instant,

    pub camera: crate::Camera,
    pub following: bool,
    pub panning: bool,
    pub pan_last: [f32; 2],
    pub cursor_px: [f32; 2],
    pub cursor_world: al_math::vec::Vec2,
    pub resolution: [f32; 2],

    pub history: History,
    pub last_scroll: Instant,
    pub scroll_dirty: bool,
    pub drag_dirty: bool,
    pub keys: InputState,

    pub status: String,
}

impl DesignerState {
    pub fn new(skeleton: Skeleton2d) -> Self {
        let pose = Pose2d::rest(skeleton.joints().len());
        Self {
            skeleton,
            pose,
            selected: None,
            drag: None,
            edit_mode: EditMode::Pose,
            clips: vec![AnimationClip2d::new("default")],
            current_clip: 0,
            time: 0.,
            playing: false,
            last_tick: Instant::now(),
            camera: Camera::new([0., 0.], 1.4),
            following: true,
            panning: false,
            pan_last: [0., 0.],
            cursor_px: [0., 0.],
            cursor_world: al_math::vec::Vec2::ZERO,
            resolution: [800., 600.],
            history: History::new(),
            last_scroll: Instant::now(),
            scroll_dirty: false,
            drag_dirty: false,
            keys: InputState::default(),
            status: "Ready.".into(),
        }
    }

    /// Nearest joint origin within ~2% of view height.
    pub fn pick(&self) -> Option<usize> {
        let world = self.skeleton.world_transforms(&self.pose);
        let pick_r = 0.02 * self.camera.view_size; // ~2% of view height
        let mut best: Option<(usize, f32)> = None;
        for (i, _) in self.skeleton.joints().iter().enumerate() {
            let c = world[i].transform_point(al_math::vec::Vec2::ZERO);
            let d = (c - self.cursor_world).length();
            if d <= pick_r && best.map_or(true, |(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    pub fn record(&mut self, snapshot: Snapshot) {
        self.history.record(snapshot);
    }

    pub fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.history.undo.pop() {
            match snapshot {
                Snapshot::Skeleton(skeleton) => {
                    self.history
                        .redo
                        .push(Snapshot::Skeleton(std::mem::take(&mut self.skeleton)));
                    self.skeleton = skeleton;
                }
                Snapshot::Pose(pose) => {
                    self.history
                        .redo
                        .push(Snapshot::Pose(std::mem::take(&mut self.pose)));
                    self.pose = pose;
                }
                Snapshot::Animation(clips, current_clip) => {
                    self.history.redo.push(Snapshot::Animation(
                        std::mem::take(&mut self.clips),
                        self.current_clip,
                    ));
                    self.clips = clips;
                    self.current_clip = current_clip;
                }
            }
            return true;
        }
        false
    }

    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.history.redo.pop() {
            match snapshot {
                Snapshot::Skeleton(skeleton) => {
                    self.history
                        .undo
                        .push(Snapshot::Skeleton(std::mem::take(&mut self.skeleton)));
                    self.skeleton = skeleton;
                }
                Snapshot::Pose(pose) => {
                    self.history
                        .undo
                        .push(Snapshot::Pose(std::mem::take(&mut self.pose)));
                    self.pose = pose;
                }
                Snapshot::Animation(clips, current_clip) => {
                    self.history.undo.push(Snapshot::Animation(
                        std::mem::take(&mut self.clips),
                        self.current_clip,
                    ));
                    self.clips = clips;
                    self.current_clip = current_clip;
                }
            }
            return true;
        }
        false
    }

    pub fn clip(&self) -> &AnimationClip2d {
        &self.clips[self.current_clip]
    }

    pub fn clip_mut(&mut self) -> &mut AnimationClip2d {
        &mut self.clips[self.current_clip]
    }

    pub fn with_clip<R>(&mut self, f: impl FnOnce(&mut AnimationClip2d) -> R) -> R {
        f(&mut self.clips[self.current_clip])
    }

    pub fn cycle_clip(&mut self, delta: isize) {
        if self.clips.is_empty() {
            return;
        }
        let n = self.clips.len() as isize;
        self.current_clip = (self.current_clip as isize + delta).rem_euclid(n) as usize;
        self.time = 0.;
        self.playing = false;
        if !self.clip().keyframes().is_empty() {
            self.pose = self.clip().sample(0.);
        }
    }

    pub fn hud_string(&self) -> String {
        let clip = self.clip();
        format!(
        "designer — {} joints, {} bones | [Tab] cycle: {}\n[T] pose mode: {} | [F] following: {} | clips: [{}]\nclip {}: {} {:.3}/{:.3} loop: {} | keys: {}\n[Space] play | [K/⇧K] key [←/→] step [G] snap\n undo: {} redo: {}\n{}",
        self.skeleton.joints().len(),
        self.skeleton.bones().len(),
        self.selected
            .map(|i| self.skeleton.joints()[i].name().to_string())
            .unwrap_or_else(|| "none".into()),
        self.edit_mode,
        self.following,
        self.clips.iter().map(|c| c.name()).collect::<Vec<_>>().join(" | "),
        self.clip().name(),
        if self.playing { "▶" } else { "▮▮" },
        self.time,
        clip.duration(),
        if clip.looping() { "on" } else { "off" },
        clip.keyframes().len(),
        self.history.undo.len(),
        self.history.redo.len(),
        self.status,
    )
    }
}
