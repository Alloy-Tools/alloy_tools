use std::sync::Arc;

use crate::{
    block_on,
    render::RenderState,
    state::{DesignerState, DragMode},
};
use al_math::{transform::Transform2D, vec::Vec2};
use al_skeleton::{
    collider::{ColliderGroup, ColliderKind},
    skeleton::Skeleton2DBuilder,
};
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
        let t = |x, y| Transform2D::from_translation(Vec2::new(x, y));
        let hurt = ColliderGroup::One(ColliderKind::Hurt);
        let none = ColliderGroup::None;
        let mut builder = Skeleton2DBuilder::new();
        // ----- spine -----
        let hips = builder.joint("hips".into(), None, t(0., -0.05), 0.075, hurt);
        let chest = builder.joint("chest".into(), Some(hips), t(0.02, 0.20), 0.085, hurt);

        let neck = builder.joint("neck".into(), Some(chest), t(0.01, 0.14), 0., none);
        let head = builder.joint("head".into(), Some(neck), t(0.02, 0.05), 0.09, hurt);

        builder.bone("torso".into(), hips, chest, 0.09, hurt);
        builder.bone("neck".into(), chest, head, 0.045, hurt);

        // ----- arms -----
        let sh_f = builder.joint("shoulder_f".into(), Some(chest), t(0.07, 0.03), 0., none);
        let el_f = builder.joint("elbow_f".into(), Some(sh_f), t(0.1, -0.12), 0., none);
        let hd_f = builder.joint("hand_f".into(), Some(el_f), t(0.1, -0.1), 0.045, hurt);

        builder.bone("upper_arm_f".into(), sh_f, el_f, 0.045, hurt);
        builder.bone("forearm_f".into(), el_f, hd_f, 0.04, hurt);

        let sh_b = builder.joint("shoulder_b".into(), Some(chest), t(-0.05, 0.03), 0., none);
        let el_b = builder.joint("elbow_b".into(), Some(sh_b), t(-0.1, -0.14), 0., none);
        let hd_b = builder.joint("hand_b".into(), Some(el_b), t(0.07, -0.08), 0.045, hurt);

        builder.bone("upper_arm_b".into(), sh_b, el_b, 0.045, hurt);
        builder.bone("forearm_b".into(), el_b, hd_b, 0.04, hurt);

        // ----- legs -----
        let kn_f = builder.joint("knee_f".into(), Some(hips), t(0.08, -0.22), 0., hurt);
        let ft_f = builder.joint("foot_f".into(), Some(kn_f), t(0.04, -0.22), 0.055, hurt);

        builder.bone("thigh_f".into(), hips, kn_f, 0.06, hurt);
        builder.bone("shin_f".into(), kn_f, ft_f, 0.05, hurt);

        let kn_b = builder.joint("knee_b".into(), Some(hips), t(-0.06, -0.22), 0., hurt);
        let ft_b = builder.joint("foot_b".into(), Some(kn_b), t(-0.06, -0.22), 0.055, hurt);

        builder.bone("thigh_b".into(), hips, kn_b, 0.06, hurt);
        builder.bone("shin_b".into(), kn_b, ft_b, 0.05, hurt);

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
        let designer = &mut self.state;
        if designer.following {
            let world = designer.skeleton.world_transforms(&designer.pose);
            if !world.is_empty() {
                let root = world[0].transform_point(al_math::vec::Vec2::ZERO);
                let target = [root.x, root.y];
                let dt = 1.0 / 60.0;
                designer.camera.update(target, dt);
            }
        }
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
                designer.cursor_world = al_math::vec::Vec2::new(
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
                designer.ctrl_held = state.control_key();
                designer.shift_held = state.shift_key();
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

                        KeyCode::KeyF => designer.following = !designer.following,

                        KeyCode::KeyT => {
                            designer.edit_pose = !designer.edit_pose;
                            designer.status = if designer.edit_pose {
                                "Editing POSE (rotations)".into()
                            } else {
                                "Editing REST (translations)".into()
                            };
                        }

                        // Cycle drag mode
                        KeyCode::KeyM => designer.drag = Some(DragMode::MoveJoint),
                        KeyCode::KeyR => designer.drag = Some(DragMode::RotateJoint),

                        // Reset root transforms (nudge squash/stretch by hand via keys)
                        KeyCode::KeyQ => {
                            designer.record();
                            designer.pose.root_scale = al_math::vec::Vec2::new(
                                (designer.pose.root_scale.x - 0.05).max(0.2),
                                (designer.pose.root_scale.y + 0.05).min(3.0),
                            );
                        }
                        KeyCode::KeyE => {
                            designer.record();
                            designer.pose.root_scale = al_math::vec::Vec2::new(
                                (designer.pose.root_scale.x + 0.05).min(3.0),
                                (designer.pose.root_scale.y - 0.05).max(0.2),
                            );
                        }
                        KeyCode::KeyZ => {
                            if designer.ctrl_held {
                                if designer.shift_held {
                                    if !designer.redo() {
                                        designer.status = "Nothing to redo.".into();
                                    }
                                } else {
                                    if !designer.undo() {
                                        designer.status = "Nothing to undo.".into();
                                    }
                                }
                            } else {
                                designer.record();
                                designer.pose.root_scale = al_math::vec::Vec2::ONE;
                                designer.pose.root_rotation = 0.0;
                                designer.pose.root_translation = al_math::vec::Vec2::ZERO;
                                designer.status = "Root transform reset.".into();
                            }
                        }

                        KeyCode::F5 => match save_skeleton(designer) {
                            Ok(()) => designer.status = "Saved skeleton.sk".into(),
                            Err(e) => designer.status = format!("Save failed: {e}"),
                        },
                        KeyCode::F9 => match load_skeleton(designer) {
                            Ok(()) => designer.status = "Loaded skeleton.sk".into(),
                            Err(e) => designer.status = format!("Load failed: {e}"),
                        },

                        _ => {}
                    }
                }
            }

            _ => {}
        }
    }
}

fn apply_drag(designer: &mut DesignerState, i: usize, mode: DragMode) {
    if !designer.drag_dirty {
        designer.record();
        designer.drag_dirty = true;
    }

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
            if designer.edit_pose {
                // No per-joint translation in Pose; ignore (or add one later).
                designer.status =
                    "Move in pose mode is a no-op (add per-joint offset to Pose).".into();
            } else {
                let j = &mut designer.skeleton.joints_mut()[i];
                let mut t = j.rest_transform();
                t.translation = target_local;
                *j.rest_transform_mut() = t;
            }
        }
        DragMode::RotateJoint => {
            // Rotation applied in the parent's local frame.
            let origin = designer.skeleton.joints()[i].rest_transform().translation;
            let v = target_local - origin;
            if v.length() < 1e-5 {
                return;
            }
            let angle = v.y.atan2(v.x);

            if designer.edit_pose {
                // Pose rotation is added on top of rest_rotation.
                let rest_rot = designer.skeleton.joints()[i]
                    .rest_transform()
                    .to_components()
                    .1;
                designer.pose.rotations[i] = angle - rest_rot;
            } else {
                let (tr, _, sc) = designer.skeleton.joints_mut()[i]
                    .rest_transform()
                    .to_components();
                *designer.skeleton.joints_mut()[i].rest_transform_mut() =
                    al_math::transform::Transform2D::new(tr, angle, sc);
            }
        }
        DragMode::Radius => { /* handled by mouse wheel */ }
    }
}

fn save_skeleton(designer: &DesignerState) -> Result<(), String> {
    #[cfg(feature = "serde")]
    {
        let s = serde_json::to_string_pretty(&designer.skeleton).map_err(|e| e.to_string())?;
        std::fs::write("skeleton.sk", s).map_err(|e| e.to_string())
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
        let sk: al_skeleton::skeleton::Skeleton2D =
            serde_json::from_str(&s).map_err(|e| e.to_string())?;
        designer.pose = al_skeleton::pose::Pose2d::rest(sk.joints().len());
        designer.skeleton = sk;
        designer.selected = None;
        designer.status = "Loaded.".into();
        Ok(())
    }
    #[cfg(not(feature = "serde"))]
    {
        Err("serde feature off".into())
    }
}
