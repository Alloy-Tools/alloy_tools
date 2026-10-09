use std::{sync::Arc, time::Instant};

use crate::{
    block_on,
    render::RenderState,
    state::{DesignerState, DragMode, Snapshot},
};
use al_math::{transform::Transform2d, vec::Vec2};
use al_skeleton::{collider_kind::ColliderKind, skeleton::Skeleton2DBuilder};
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

const TITLE: &str = "Skeleton2D Designer Tool";
const START_SIZE: winit::dpi::LogicalSize<i32> = winit::dpi::LogicalSize::new(800, 600);

pub struct App {
    window: Option<Arc<Window>>,
    render_state: Option<RenderState>,
    state: DesignerState,
}

impl App {
    pub fn new() -> Self {
        let t = |x, y| Transform2d::from_translation(Vec2::new(x, y));
        let hurt = ColliderKind::HURT;
        let none = ColliderKind::NONE;
        let mut builder = Skeleton2DBuilder::new();
        // ----- spine -----
        let hips = builder.joint("hips", None, t(0., -0.05), 0.075, hurt, 0.);
        let chest = builder.joint("chest", Some(hips), t(0.02, 0.20), 0.085, hurt, 0.);

        let neck = builder.joint("neck", Some(chest), t(0.01, 0.14), 0., none, 0.);
        let head = builder.joint("head", Some(neck), t(0.02, 0.05), 0.09, hurt, 0.);

        builder.bone("torso", hips, chest, 0.09, hurt, 0.);
        builder.bone("neck", chest, head, 0.045, hurt, 0.);

        // ----- arms -----
        let sh_f = builder.joint(
            "shoulder_f",
            Some(chest),
            t(0.07, 0.03),
            0.,
            none,
            -0.6,
        );
        let el_f = builder.joint("elbow_f", Some(sh_f), t(0.1, -0.12), 0., none, -0.6);
        let hd_f = builder.joint(
            "hand_f",
            Some(el_f),
            t(0.1, -0.1),
            0.045,
            hurt,
            -0.6,
        );

        builder.bone("upper_arm_f", sh_f, el_f, 0.045, hurt, -0.6);
        builder.bone("forearm_f", el_f, hd_f, 0.04, hurt, -0.6);

        let sh_b = builder.joint(
            "shoulder_b",
            Some(chest),
            t(-0.05, 0.03),
            0.,
            none,
            0.6,
        );
        let el_b = builder.joint("elbow_b", Some(sh_b), t(-0.1, -0.14), 0., none, 0.6);
        let hd_b = builder.joint(
            "hand_b",
            Some(el_b),
            t(0.07, -0.08),
            0.045,
            hurt,
            0.6,
        );

        builder.bone("upper_arm_b", sh_b, el_b, 0.045, hurt, 0.6);
        builder.bone("forearm_b", el_b, hd_b, 0.04, hurt, 0.6);

        // ----- legs -----
        let kn_f = builder.joint("knee_f", Some(hips), t(0.08, -0.22), 0., hurt, -0.6);
        let ft_f = builder.joint(
            "foot_f",
            Some(kn_f),
            t(0.04, -0.22),
            0.055,
            hurt,
            -0.6,
        );

        builder.bone("thigh_f", hips, kn_f, 0.06, hurt, -0.6);
        builder.bone("shin_f", kn_f, ft_f, 0.05, hurt, -0.6);

        let kn_b = builder.joint("knee_b", Some(hips), t(-0.06, -0.22), 0., hurt, 0.6);
        let ft_b = builder.joint(
            "foot_b",
            Some(kn_b),
            t(-0.06, -0.22),
            0.055,
            hurt,
            0.6,
        );

        builder.bone("thigh_b", hips, kn_b, 0.06, hurt, 0.6);
        builder.bone("shin_b", kn_b, ft_b, 0.05, hurt, 0.6);

        let skeleton = builder.build();
        if let Err(e) = skeleton.validate() {
            panic!("Skeleton validation failed: {e}")
        }
        Self {
            window: None,
            render_state: None,
            state: DesignerState::new(skeleton),
        }
    }

    pub fn run(&mut self) -> Result<(), winit::error::EventLoopError> {
        winit::event_loop::EventLoop::new().unwrap().run_app(self)
    }

    pub fn update(&mut self) {
        let designer = &mut self.state;
        if designer.following {
            let world = designer.skeleton.world_transforms(&designer.pose);
            if !world.is_empty() {
                let root = world[0].transform_point(Vec2::ZERO);
                let target = [root.x, root.y];
                let dt = 1.0 / 60.0;
                designer.camera.update(target, dt);
            }
        }
        if designer.playing {
            let now = Instant::now();
            let dt = (now - designer.last_tick).as_secs_f32().min(0.1);
            designer.last_tick = now;
            designer.time += dt;

            let (looping, dur) = {
                let clip = designer.clip();
                (clip.looping(), clip.duration())
            };
            if looping && dur > 0. {
                designer.time = designer.time.rem_euclid(dur);
            } else if designer.time >= dur {
                designer.time = dur;
                designer.playing = false;
            }

            if !designer.clip().keyframes().is_empty() {
                designer.pose = designer.clip().sample(designer.time);
                //TODO: Remove
                if let Some(selected) = designer.selected {
                    designer.status = format!(
                        "t={:.3} dur={:.3} n={} rot[{}]={:.3} rscale=({:.2},{:.2})",
                        designer.time,
                        designer.clip().duration(),
                        designer.clip().keyframes().len(),
                        selected,
                        designer
                            .pose
                            .rotations
                            .get(selected)
                            .copied()
                            .unwrap_or(f32::NAN),
                        designer.pose.root_scale.x,
                        designer.pose.root_scale.y,
                    );
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Create window once
        if self.window.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title(TITLE)
                            .with_inner_size(START_SIZE),
                    )
                    .expect("Failed to create window"),
            );
            self.window = Some(window.clone());
            // Init wgpu using new window
            self.render_state = Some(block_on(RenderState::new(window)));
        }
    }

    fn about_to_wait(&mut self, _: &winit::event_loop::ActiveEventLoop) {
        self.update();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let designer = &mut self.state;
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(render_state) = &mut self.render_state {
                    if size.width > 0 && size.height > 0 {
                        render_state.config.width = size.width;
                        render_state.config.height = size.height;
                        render_state
                            .surface
                            .configure(&render_state.device, &render_state.config);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.render_state {
                    state.render_designer(designer);
                }
            }
            WindowEvent::Focused(focused) => {
                if let Some(window) = &self.window {
                    window.set_cursor_visible(!focused);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                designer.cursor_px = [position.x as f32, position.y as f32];

                if designer.panning {
                    let dx = designer.cursor_px[0] - designer.pan_last[0];
                    let dy = designer.cursor_px[1] - designer.pan_last[1];
                    let h = designer.resolution[1].max(1.0);
                    let vs = designer.camera.view_size;
                    designer.camera.pos[0] -= (dx / h) * vs;
                    designer.camera.pos[1] += (dy / h) * vs;
                    designer.pan_last = designer.cursor_px;
                }

                // Recompute world before applying drag
                let cx = designer.cursor_px[0] - 0.5 * designer.resolution[0];
                let cy = designer.cursor_px[1] - 0.5 * designer.resolution[1];
                designer.cursor_world = Vec2::new(
                    designer.camera.pos[0]
                        + (cx / designer.resolution[1]) * designer.camera.view_size,
                    designer.camera.pos[1]
                        + (-cy / designer.resolution[1]) * designer.camera.view_size,
                );

                if let (Some(i), Some(mode)) = (designer.selected, designer.drag) {
                    apply_drag(designer, i, mode);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == winit::event::ElementState::Pressed;
                if button == winit::event::MouseButton::Left {
                    if pressed {
                        designer.selected = designer.pick();
                        designer.drag = designer.selected.map(|_| DragMode::MoveJoint);
                        designer.drag_dirty = false;
                    } else {
                        designer.drag = None;
                        designer.drag_dirty = false;
                    }
                } else if button == winit::event::MouseButton::Right {
                    if pressed {
                        designer.panning = true;
                        designer.pan_last = designer.cursor_px;
                        designer.following = false;
                    } else {
                        designer.panning = false;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // Scroll = adjust radius of selected joint/bone or zoom if none selected.
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => (p.y as f32) / 40.0,
                };
                if let Some(i) = designer.selected {
                    if designer.scroll_dirty && designer.last_scroll.elapsed().as_millis() >= 250 {
                        designer.record(Snapshot::Skeleton(designer.skeleton.clone()));
                    }
                    if !designer.scroll_dirty {
                        designer.scroll_dirty = true;
                        designer.record(Snapshot::Skeleton(designer.skeleton.clone()));
                    }
                    designer.last_scroll = Instant::now();
                    let step = 0.002 * scroll;
                    let joints = designer.skeleton.joints_mut();
                    let r = joints[i].radius() + step;
                    joints[i].set_radius(r);
                } else {
                    designer.camera.view_size =
                        (designer.camera.view_size * (1. - 0.1 * scroll)).clamp(0.2, 20.);
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                let state = mods.state();
                designer.keys.ctrl_held = state.control_key();
                designer.keys.shift_held = state.shift_key();
                designer.keys.alt_held = state.alt_key();
            }

            WindowEvent::KeyboardInput { event, .. }
                if event.state == winit::event::ElementState::Released =>
            {
                use winit::keyboard::KeyCode;
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        KeyCode::Escape => event_loop.exit(),

                        KeyCode::Tab => {
                            let n = designer.skeleton.joints().len();
                            if n > 0 {
                                designer.selected = Some(match designer.selected {
                                    None => 0,
                                    Some(i) => (i + 1) % n,
                                });
                            }
                        }

                        KeyCode::Space => {
                            designer.playing = !designer.playing;
                            if designer.playing {
                                designer.last_tick = Instant::now();
                            }
                            designer.status = if designer.playing {
                                "Playing".into()
                            } else {
                                "Paused".into()
                            };
                        }

                        KeyCode::KeyK => {
                            if designer.keys.shift_held {
                                designer.record(Snapshot::Animation(
                                    designer.clips.clone(),
                                    designer.current_clip,
                                ));
                                let tol = 1. / 30.;
                                let time = designer.time;
                                if designer.clip_mut().remove_at(time, tol) {
                                    designer.status = format!(
                                        "Key removed ({} left)",
                                        designer.clip_mut().keyframes().len()
                                    );
                                } else {
                                    designer.status = "No keyframe near playhead.".into();
                                }
                            } else {
                                designer.record(Snapshot::Animation(
                                    designer.clips.clone(),
                                    designer.current_clip,
                                ));
                                let time = designer.time;
                                let pose = designer.pose.clone();
                                designer.clip_mut().insert(time, pose);
                                designer.status = format!(
                                    "Key inserted at t={:.3} ({} total)",
                                    designer.time,
                                    designer.clip().keyframes().len()
                                );
                            }
                        }

                        KeyCode::BracketLeft => designer.cycle_clip(-1),
                        KeyCode::BracketRight => designer.cycle_clip(1),

                        KeyCode::KeyN if designer.keys.ctrl_held => {
                            designer.history.record(Snapshot::Animation(
                                designer.clips.clone(),
                                designer.current_clip,
                            ));
                            designer
                                .clips
                                .push(al_anim::anim_clip::AnimationClip2d::new(format!(
                                    "clip_{}",
                                    designer.clips.len()
                                )));
                            designer.current_clip = designer.clips.len() - 1;
                            designer.time = 0.;
                            designer.playing = false;
                        }

                        KeyCode::KeyD if designer.keys.ctrl_held => {
                            designer.history.record(Snapshot::Animation(
                                designer.clips.clone(),
                                designer.current_clip,
                            ));
                            let mut c = designer.clip().clone();
                            c.set_name(format!("{}.copy", c.name()));
                            designer.clips.push(c);
                            designer.current_clip = designer.clips.len() - 1;
                        }

                        KeyCode::Delete if designer.keys.ctrl_held => {
                            if designer.clips.is_empty() {
                                return;
                            }
                            designer.history.record(Snapshot::Animation(
                                designer.clips.clone(),
                                designer.current_clip,
                            ));
                            designer.clips.remove(designer.current_clip);
                            designer.current_clip =
                                designer.current_clip.min(designer.clips.len() - 1);
                            designer.time = 0.;
                        }

                        KeyCode::ArrowLeft => {
                            let step = if designer.keys.alt_held {
                                1. / 60.
                            } else {
                                5. / 60.
                            };
                            //REIVEW: Cycle to clip duration instead of clamping?
                            designer.time = (designer.time - step).max(0.);
                            if !designer.clip().keyframes().is_empty() {
                                designer.pose = designer.clip().sample(designer.time);
                            }
                        }

                        KeyCode::ArrowRight => {
                            let step = if designer.keys.alt_held {
                                1. / 60.
                            } else {
                                5. / 60.
                            };
                            designer.time = designer.time + step;
                            if !designer.clip().keyframes().is_empty() {
                                designer.pose = designer.clip().sample(designer.time);
                            }
                        }

                        KeyCode::KeyG => {
                            if let Some(t) = designer.clip().nearest_keyframe_time(designer.time) {
                                designer.time = t;
                                designer.pose = designer.clip().sample(t);
                            }
                        }

                        KeyCode::Home => {
                            designer.time = 0.;
                            if !designer.clip().keyframes().is_empty() {
                                designer.pose = designer.clip().sample(0.);
                            }
                        }

                        KeyCode::KeyL => {
                            let looping = !designer.clip().looping();
                            designer.clip_mut().set_looping(looping);
                            designer.status =
                                format!("Loop: {}", if looping { "on" } else { "off" });
                        }

                        KeyCode::KeyF => designer.following = !designer.following,

                        KeyCode::KeyT => {
                            designer.edit_mode.cycle();
                            designer.status = designer.edit_mode.status();
                        }

                        // Cycle drag mode
                        KeyCode::KeyM => designer.drag = Some(DragMode::MoveJoint),
                        KeyCode::KeyR => designer.drag = Some(DragMode::RotateJoint),

                        // Reset root transforms (nudge squash/stretch by hand via keys)
                        KeyCode::KeyQ => {
                            designer.record(Snapshot::Pose(designer.pose.clone()));
                            designer.pose.root_scale = Vec2::new(
                                (designer.pose.root_scale.x - 0.05).max(0.2),
                                (designer.pose.root_scale.y + 0.05).min(3.0),
                            );
                        }
                        KeyCode::KeyE => {
                            designer.record(Snapshot::Pose(designer.pose.clone()));
                            designer.pose.root_scale = Vec2::new(
                                (designer.pose.root_scale.x + 0.05).min(3.0),
                                (designer.pose.root_scale.y - 0.05).max(0.2),
                            );
                        }
                        KeyCode::KeyZ => {
                            if designer.keys.ctrl_held {
                                if designer.keys.shift_held {
                                    if !designer.redo() {
                                        designer.status = "Nothing to redo.".into();
                                    }
                                } else {
                                    if !designer.undo() {
                                        designer.status = "Nothing to undo.".into();
                                    }
                                }
                            } else {
                                designer.record(Snapshot::Pose(designer.pose.clone()));
                                designer.pose.root_scale = Vec2::ONE;
                                designer.pose.root_rotation = 0.0;
                                designer.pose.root_translation = Vec2::ZERO;
                                designer.status = "Root transform reset.".into();
                            }
                        }

                        KeyCode::F5 => {
                            if let Err(e) = save_skeleton(designer) {
                                designer.status = format!("Skeleton save failed: {e}");
                            }
                        }
                        KeyCode::F9 => {
                            if let Err(e) = load_skeleton(designer) {
                                designer.status = format!("Skeleton load failed: {e}");
                            }
                        }

                        KeyCode::F6 => {
                            if let Err(e) = save_clips(designer) {
                                designer.status = format!("Animations save failed: {e}");
                            }
                        }
                        KeyCode::F10 => {
                            if let Err(e) = load_clips(designer) {
                                designer.status = format!("Animations load failed: {e}");
                            }
                        }

                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }
}

fn apply_drag(designer: &mut DesignerState, i: usize, mode: DragMode) {
    let world = designer.skeleton.world_transforms(&designer.pose);

    // Cursor in the parent's local frame (or world if no parent).
    let target_local = match designer.skeleton.joints()[i].parent() {
        None => designer.cursor_world,
        Some(p) => match world[p].try_inverse() {
            Some(inv) => inv.transform_point(designer.cursor_world),
            None => return,
        },
    };

    match mode {
        DragMode::MoveJoint => {
            match designer.edit_mode {
                crate::state::EditMode::Pose => {
                    // No per-joint translation in Pose; ignore (or add one later).
                    designer.status =
                        "Move in pose mode is a no-op (add per-joint offset to Pose).".into()
                }
                crate::state::EditMode::Rest => {
                    if !designer.drag_dirty {
                        designer.record(Snapshot::Skeleton(designer.skeleton.clone()));
                        designer.drag_dirty = true;
                    }
                    let j = &mut designer.skeleton.joints_mut()[i];
                    let mut t = j.transform();
                    t.translation = target_local;
                    *j.transform_mut() = t;
                }
            }
        }
        DragMode::RotateJoint => {
            // Rotation applied in the parent's local frame.
            let origin = designer.skeleton.joints()[i].transform().to_components().0;
            let v = target_local - origin;
            if v.length() < 1e-5 {
                return;
            }
            let angle = v.y.atan2(v.x);

            match designer.edit_mode {
                crate::state::EditMode::Pose => {
                    if !designer.drag_dirty {
                        designer.record(Snapshot::Pose(designer.pose.clone()));
                        designer.drag_dirty = true;
                    }
                    // Pose rotation is added on top of rest_rotation.
                    let rest_rot = designer.skeleton.joints()[i].transform().to_components().1;
                    designer.pose.rotations[i] = angle - rest_rot;
                }
                crate::state::EditMode::Rest => {
                    if !designer.drag_dirty {
                        designer.record(Snapshot::Skeleton(designer.skeleton.clone()));
                        designer.drag_dirty = true;
                    }
                    let (tr, _, sc) = designer.skeleton.joints_mut()[i]
                        .transform()
                        .to_components();
                    *designer.skeleton.joints_mut()[i].transform_mut() =
                        al_math::transform::Transform2d::new(tr, angle, sc);
                }
            }
        }
    }
}

fn save_skeleton(designer: &mut DesignerState) -> Result<(), String> {
    #[cfg(feature = "serde")]
    {
        let s = serde_json::to_string_pretty(&designer.skeleton).map_err(|e| e.to_string())?;
        std::fs::write("skeleton.sk", s).map_err(|e| e.to_string())?;
        designer.status = "Skeleton Saved.".into();
        Ok(())
    }
    #[cfg(not(feature = "serde"))]
    {
        Err("serde feature off".into())
    }
}

fn load_skeleton(designer: &mut DesignerState) -> Result<(), String> {
    #[cfg(feature = "serde")]
    {
        let s = std::fs::read_to_string("skeleton.sk").map_err(|e| e.to_string())?;
        let sk: al_skeleton::skeleton::Skeleton2d =
            serde_json::from_str(&s).map_err(|e| e.to_string())?;
        designer.pose = al_skeleton::pose::Pose2d::rest(sk.joints().len());
        designer.skeleton = sk;
        designer.selected = None;
        designer.status = "Skeleton Loaded.".into();
        Ok(())
    }
    #[cfg(not(feature = "serde"))]
    {
        Err("serde feature off".into())
    }
}

fn save_clips(designer: &mut DesignerState) -> Result<(), String> {
    #[cfg(feature = "serde")]
    {
        let s = serde_json::to_string_pretty(&designer.clips).map_err(|e| e.to_string())?;
        std::fs::write("clips.am", s).map_err(|e| e.to_string())?;
        designer.status = "Clips Saved.".into();
        Ok(())
    }
    #[cfg(not(feature = "serde"))]
    {
        Err("serde feature off".into())
    }
}

fn load_clips(designer: &mut DesignerState) -> Result<(), String> {
    #[cfg(feature = "serde")]
    {
        let s = std::fs::read_to_string("clips.am").map_err(|e| e.to_string())?;
        let clips: Vec<al_anim::anim_clip::AnimationClip2d> =
            serde_json::from_str(&s).map_err(|e| e.to_string())?;
        designer.clips = clips;
        designer.status = "Clips Loaded.".into();
        Ok(())
    }
    #[cfg(not(feature = "serde"))]
    {
        Err("serde feature off".into())
    }
}
