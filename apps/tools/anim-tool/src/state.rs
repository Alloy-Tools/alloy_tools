use al_skeleton::{pose::Pose2d, skeleton::Skeleton2D};

use crate::Camera;

#[derive(Clone, Copy, PartialEq)]
pub enum DragMode {
    MoveJoint,
    RotateJoint,
    Radius,
}

#[derive(Clone)]
pub struct Snapshot {
    pub skeleton: Skeleton2D,
    pub pose: Pose2d,
}

pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn record(&mut self, skeleton: Skeleton2D, pose: Pose2d) {
        self.undo.push(Snapshot { skeleton, pose });
    }

    pub fn undo(&mut self, skeleton: &Skeleton2D, pose: &Pose2d) -> Option<Snapshot> {
        let Some(prev) = self.undo.pop() else {
            return None;
        };
        self.redo.push(Snapshot {
            skeleton: skeleton.clone(),
            pose: pose.clone(),
        });
        Some(prev)
    }

    pub fn redo(&mut self, skeleton: &Skeleton2D, pose: &Pose2d) -> Option<Snapshot> {
        let Some(next) = self.redo.pop() else {
            return None;
        };
        self.undo.push(Snapshot {
            skeleton: skeleton.clone(),
            pose: pose.clone(),
        });
        Some(next)
    }
}

pub struct DesignerState {
    pub skeleton: Skeleton2D,
    pub pose: Pose2d,

    pub selected: Option<usize>,
    pub drag: Option<DragMode>,
    pub edit_pose: bool, // true = edit Pose, false = edit rest transforms

    pub camera: crate::Camera,
    pub following: bool,
    pub panning: bool,
    pub pan_last: [f32; 2],
    pub cursor_px: [f32; 2],
    pub cursor_world: al_math::vec::Vec2,
    pub resolution: [f32; 2],

    pub history: History,
    pub drag_dirty: bool,
    pub ctrl_held: bool,
    pub shift_held: bool,

    pub status: String,
}

impl DesignerState {
    pub fn new(skeleton: Skeleton2D) -> Self {
        let pose = Pose2d::rest(skeleton.joints().len());
        Self {
            skeleton,
            pose,
            selected: None,
            drag: None,
            edit_pose: false,
            camera: Camera::new([0., 0.], 1.4),
            following: true,
            panning: false,
            pan_last: [0., 0.],
            cursor_px: [0., 0.],
            cursor_world: al_math::vec::Vec2::ZERO,
            resolution: [800., 600.],
            history: History::new(),
            drag_dirty: false,
            ctrl_held: false,
            shift_held: false,
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

    pub fn record(&mut self) {
        self.history
            .record(self.skeleton.clone(), self.pose.clone());
    }

    pub fn undo(&mut self) -> bool {
        if let Some(Snapshot { skeleton, pose }) = self.history.undo(&self.skeleton, &self.pose) {
            self.skeleton = skeleton;
            self.pose = pose;
            return true;
        }
        false
    }

    pub fn redo(&mut self) -> bool {
        if let Some(Snapshot { skeleton, pose }) = self.history.redo(&self.skeleton, &self.pose) {
            self.skeleton = skeleton;
            self.pose = pose;
            return true;
        }
        false
    }
}
