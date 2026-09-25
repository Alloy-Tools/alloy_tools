use al_events::EventHelpers;
use bytemuck::Zeroable;
use std::time::{Duration, Instant};

use crate::{
    block_on,
    net::{ConnId, NetworkState},
};

pub const EPS: f32 = 0.002;
pub const FIXED_DT: f32 = 1. / 60.;
pub const MAX_FRAME_DT: f32 = 0.25;

pub const PROJECTILE_RADIUS: f32 = 0.012;
pub const PROJECTILE_SPEED: f32 = 1.8;
pub const PROJECTILE_LIFETIME: f32 = 1.5;
pub const MAX_PROJECTILES: usize = 64;
pub const FIRE_COOLDOWN: Duration = Duration::from_millis(180);

pub const MAX_PLAYERS: usize = 64;
pub const MAX_HP: f32 = 100.;
pub const DAMAGE_PER_HIT: f32 = 25.;
pub const RESPAWN_DELAY: Duration = Duration::from_secs(3);

#[derive(Clone, Copy)]
pub struct Projectile {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub life: f32,
    pub owner: Option<ConnId>,
}

#[al_events::event(Hash)]
pub struct ProjectileSpawn {
    owner: ConnId,
    pos: [f32; 2],
    dir: [f32; 2],
}

impl ProjectileSpawn {
    pub fn new(owner: ConnId, pos: [f32; 2], dir: [f32; 2]) -> Self {
        Self { owner, pos, dir }
    }
    pub fn owner(&self) -> ConnId {
        self.owner
    }
    pub fn pos(&self) -> [f32; 2] {
        self.pos
    }
    pub fn dir(&self) -> [f32; 2] {
        self.dir
    }
}

impl std::hash::Hash for ProjectileSpawn {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
        self.pos[0].to_bits().hash(state);
        self.pos[1].to_bits().hash(state);
        self.dir[0].to_bits().hash(state);
        self.dir[1].to_bits().hash(state);
    }
}

#[derive(Default, Clone, Copy)]
pub struct InputState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub space: bool,
}

#[derive(Clone, Copy)]
pub struct Player {
    pos: [f32; 2],
    radius: f32,
    target_pos: [f32; 2],
    is_local: bool,
    conn_id: Option<ConnId>,
    pub hp: f32,
    pub alive: bool,
}

impl Player {
    pub fn new(pos: [f32; 2], radius: f32, conn_id: ConnId) -> Self {
        Self {
            pos,
            radius,
            target_pos: pos,
            is_local: false,
            conn_id: Some(conn_id),
            hp: MAX_HP,
            alive: true,
        }
    }

    pub fn new_local(pos: [f32; 2], radius: f32) -> Self {
        Self {
            pos,
            radius,
            target_pos: pos,
            is_local: true,
            conn_id: None,
            hp: MAX_HP,
            alive: true,
        }
    }
}

#[al_events::event(Hash)]
pub struct PlayerUpdate {
    conn_id: ConnId,
    pos: [f32; 2],
    radius: f32,
}

impl std::hash::Hash for PlayerUpdate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.conn_id.hash(state);
        self.pos[0].to_bits().hash(state);
        self.pos[1].to_bits().hash(state);
        self.radius.to_bits().hash(state);
    }
}

impl PlayerUpdate {
    pub fn new(conn_id: ConnId, pos: [f32; 2], radius: f32) -> Self {
        Self {
            conn_id,
            pos,
            radius,
        }
    }

    pub fn conn_id(&self) -> ConnId {
        self.conn_id
    }
    pub fn pos(&self) -> [f32; 2] {
        self.pos
    }
    pub fn radius(&self) -> f32 {
        self.radius
    }
}

#[al_events::event(Hash)]
pub struct HealthUpdate {
    conn_id: ConnId,
    hp: f32,
}
impl std::hash::Hash for HealthUpdate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.conn_id.hash(state);
        self.hp.to_bits().hash(state);
    }
}
impl HealthUpdate {
    pub fn new(conn_id: ConnId, hp: f32) -> Self {
        Self { conn_id, hp }
    }
    pub fn conn_id(&self) -> ConnId {
        self.conn_id
    }
    pub fn hp(&self) -> f32 {
        self.hp
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ScoreEntry {
    pub conn_id: ConnId,
    pub kills: u32,
    pub deaths: u32,
}

#[al_events::event]
pub struct Scoreboard {
    entries: Vec<ScoreEntry>,
}
impl Scoreboard {
    pub fn new(entries: Vec<ScoreEntry>) -> Self {
        Self { entries }
    }
    pub fn entries(&self) -> &[ScoreEntry] {
        &self.entries
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PlayerData {
    count: u32,
    _pad: [u32; 3],
    players: [[f32; 8]; crate::game::MAX_PLAYERS],
}

impl PlayerData {
    pub fn new(game: &GameState) -> Self {
        let mut data = PlayerData::zeroed();
        let count = game.players.len().min(MAX_PLAYERS);
        data.count = count as u32;
        for (i, p) in game.players.iter().take(MAX_PLAYERS).enumerate() {
            data.players[i] = [
                p.pos[0],
                p.pos[1],
                p.radius,
                if p.is_local { 1. } else { 0. },
                (p.hp / MAX_HP).clamp(0., 1.),
                if p.alive { 1. } else { 0. },
                0.,
                0.,
            ];
        }
        data
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ProjectileData {
    count: u32,
    _pad: [u32; 3],
    projectiles: [[f32; 4]; MAX_PROJECTILES],
}

impl ProjectileData {
    pub fn new(game: &GameState) -> Self {
        let mut data = Self::zeroed();
        let count = game.projectiles.len().max(MAX_PLAYERS);
        data.count = count as u32;
        for (i, p) in game.projectiles.iter().take(MAX_PROJECTILES).enumerate() {
            let is_local = if p.owner.is_none() { 1. } else { 0. };
            data.projectiles[i] = [p.pos[0], p.pos[1], PROJECTILE_RADIUS, is_local];
        }
        data
    }
}

pub struct Camera {
    pub pos: [f32; 2],
    pub view_size: f32,
    pub follow_speed: f32,
}

impl Camera {
    pub fn new(pos: [f32; 2], view_size: f32) -> Self {
        Self {
            pos,
            view_size,
            follow_speed: 8.,
        }
    }

    pub fn update(&mut self, target: [f32; 2], dt: f32) {
        let factor = 1.0 - (-self.follow_speed * dt).exp();
        self.pos[0] += (target[0] - self.pos[0]) * factor;
        self.pos[1] += (target[1] - self.pos[1]) * factor;
    }
}

pub struct GameState {
    pub last_resolution: [f32; 2],
    pub camera: Camera,
    pub cursor_px: [f32; 2],
    pub cursor_world: [f32; 2],
    pub players: Vec<Player>,
    pub scoreboard: Scoreboard,
    pub projectiles: Vec<Projectile>,
    pub local_index: usize,
    pub local_conn_id: Option<ConnId>,
    pub server_id: Option<ConnId>,
    pub keys: InputState,
    pub last_fire: Instant,
    pub last_frame: Instant,
    pub accumulator: f32,
    pub last_send: Instant,
    pub send_interval: Duration,
    pub net_state: NetworkState,
}

impl GameState {
    pub fn new(send_interval: Duration, net_state: NetworkState) -> Self {
        Self {
            last_resolution: [0., 0.],
            camera: Camera::new([0., 0.], 1.4),
            cursor_px: [400., 300.],
            cursor_world: [0., 0.],
            players: vec![Player::new_local([0., 0.], 0.05)],
            scoreboard: Scoreboard::new(Vec::new()),
            projectiles: Vec::new(),
            local_index: 0,
            local_conn_id: None,
            server_id: None,
            keys: InputState::default(),
            last_fire: Instant::now() - FIRE_COOLDOWN,
            last_frame: Instant::now(),
            accumulator: 0.,
            last_send: Instant::now(),
            send_interval,
            net_state,
        }
    }

    pub fn set_resolution(&mut self, resolution: [f32; 2]) {
        self.last_resolution = resolution;
    }

    pub fn update(&mut self, map: impl Fn(f32, f32) -> f32) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(MAX_FRAME_DT);
        self.last_frame = now;

        // Pull in net state
        self.sync_remote();

        let res = self.last_resolution;
        let centered = [
            self.cursor_px[0] - 0.5 * res[0],
            self.cursor_px[1] - 0.5 * res[1],
        ];
        let ndc = [centered[0] / res[1], -centered[1] / res[1]];
        self.cursor_world = [
            self.camera.pos[0] + ndc[0] * self.camera.view_size,
            self.camera.pos[1] + ndc[1] * self.camera.view_size,
        ];

        self.accumulator += dt;
        while self.accumulator >= FIXED_DT {
            self.fixed_update(&map, FIXED_DT);
            self.accumulator -= FIXED_DT;
        }

        self.interpolate_remotes(dt);

        self.camera.update(self.players[self.local_index].pos, dt);
    }

    fn fixed_update(&mut self, map: impl Fn(f32, f32) -> f32, dt: f32) {
        let local_alive = self.players[self.local_index].alive;
        // Local player movement
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        if local_alive {
            if self.keys.up {
                dy += 1.;
            }
            if self.keys.down {
                dy -= 1.;
            }
            if self.keys.right {
                dx += 1.;
            }
            if self.keys.left {
                dx -= 1.;
            }
        }

        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 {
            dx /= len;
            dy /= len;
        }

        let speed = 0.5;
        {
            let p = &mut self.players[self.local_index];
            p.pos[0] += dx * speed * dt;
            p.pos[1] += dy * speed * dt;

            // Resolve collisions on the local player after movement.
            resolve_collision(&mut p.pos, p.radius, crate::map::map_sdf);
        }

        if local_alive && self.keys.space && self.last_fire.elapsed() >= FIRE_COOLDOWN {
            self.last_fire = Instant::now();

            let p = self.players[self.local_index];
            let dx = self.cursor_world[0] - p.pos[0];
            let dy = self.cursor_world[1] - p.pos[1];
            let len = (dx * dx + dy * dy).sqrt();
            if len > 1e-4 {
                let dir = [dx / len, dy / len];
                let offset = p.radius + PROJECTILE_RADIUS + 0.005;
                let pos = [p.pos[0] + dir[0] * offset, p.pos[1] + dir[1] * offset];

                self.projectiles.push(Projectile {
                    pos,
                    vel: [dir[0] * PROJECTILE_SPEED, dir[1] * PROJECTILE_SPEED],
                    life: PROJECTILE_LIFETIME,
                    owner: None,
                });

                let msg = ProjectileSpawn::new(self.local_conn_id.unwrap_or(0), pos, dir).to_msg();
                if let Some(server) = block_on(self.net_state.local_conn_id()) {
                    if let Err(e) = self.net_state.handle().send_to(server, msg) {
                        eprintln!("Failed to send `ProjectileSpawn`: {e}");
                    }
                }
            }
        }

        self.resolve_player_collisions();

        // Pushing may have shoved the local player into a wall; resolve again.
        {
            let p = &mut self.players[self.local_index];
            resolve_collision(&mut p.pos, p.radius, crate::map::map_sdf);
        }

        let mut alive = Vec::with_capacity(self.projectiles.len());
        for mut proj in std::mem::take(&mut self.projectiles) {
            proj.life -= dt;
            if proj.life <= 0. {
                continue;
            }

            let step_dist = (proj.vel[0] * proj.vel[0] + proj.vel[1] * proj.vel[1]).sqrt() * dt;
            let steps = (step_dist / (PROJECTILE_RADIUS * 0.9)).ceil().max(1.) as u32;
            let step_dt = dt / steps as f32;

            let mut hit = false;
            'outer: for _ in 0..steps {
                proj.pos[0] += proj.vel[0] * step_dt;
                proj.pos[1] += proj.vel[1] * step_dt;

                // Map hit
                if map(proj.pos[0], proj.pos[1]) < PROJECTILE_RADIUS {
                    hit = true;
                    break;
                }

                // Player hit
                for (i, p) in self.players.iter().enumerate() {
                    if !p.alive || i == self.local_index {
                        continue;
                    }
                    let dx = proj.pos[0] - p.pos[0];
                    let dy = proj.pos[1] - p.pos[1];
                    let r = PROJECTILE_RADIUS + p.radius;
                    if dx * dx + dy * dy < r * r {
                        hit = true;
                        break 'outer;
                    }
                }
            }
            if !hit {
                alive.push(proj);
            }
        }
        self.projectiles = alive;
    }

    fn resolve_player_collisions(&mut self) {
        let local_idx = self.local_index;
        let local_pos = self.players[local_idx].pos;
        let local_r = self.players[local_idx].radius;

        let mut push_x = 0.0f32;
        let mut push_y = 0.0f32;

        for (i, other) in self.players.iter().enumerate() {
            if i == local_idx {
                continue;
            }

            let dx = local_pos[0] - other.pos[0];
            let dy = local_pos[1] - other.pos[1];
            let dist_sq = dx * dx + dy * dy;
            let min_dist = local_r + other.radius;

            if dist_sq < min_dist * min_dist && dist_sq > 1e-9 {
                let dist = dist_sq.sqrt();
                let overlap = min_dist - dist;
                push_x += (dx / dist) * overlap;
                push_y += (dy / dist) * overlap;
            }
        }

        if push_x != 0.0 || push_y != 0.0 {
            self.players[local_idx].pos[0] += push_x;
            self.players[local_idx].pos[1] += push_y;
        }
    }

    fn sync_remote(&mut self) {
        use std::collections::HashSet;

        if self.local_conn_id.is_none() {
            self.local_conn_id = block_on(self.net_state.local_conn_id());
        }
        if self.server_id.is_none() {
            self.server_id = block_on(self.net_state.server_id());
        }

        // Sync players
        let snapshot = {
            let state = block_on(self.net_state.state().read());
            state
                .players()
                .map(|p| (p.conn_id, p.pos, p.radius))
                .collect::<Vec<_>>()
        };

        let valid = snapshot
            .iter()
            .map(|(id, _, _)| *id)
            .collect::<HashSet<_>>();

        // Drop disconnected players
        self.players
            .retain(|p| p.is_local || p.conn_id.map_or(false, |id| valid.contains(&id)));

        // Add or Update
        for (id, pos, radius) in snapshot {
            if let Some(existing) = self.players.iter_mut().find(|p| p.conn_id == Some(id)) {
                existing.target_pos = pos;
                existing.radius = radius;
            } else {
                self.players.push(Player::new(pos, radius, id));
            }
        }

        // Sync Projectiles
        let incoming = { block_on(self.net_state.state().write()).drain_projectiles() };
        for (owner, pos, dir) in incoming {
            self.spawn_remote_projectile(owner, pos, dir);
        }

        // Sync health
        let incoming = { block_on(self.net_state.state().write()).drain_health() };
        for (conn_id, hp) in incoming {
            self.set_health(conn_id, hp);
        }
        let sb = { block_on(self.net_state.state().write()).take_scoreboard() };
        if let Some(sb) = sb {
            self.scoreboard = sb;
        }
    }

    fn interpolate_remotes(&mut self, dt: f32) {
        let factor = 1.0 - (-12.0 * dt).exp();
        for p in self.players.iter_mut().filter(|p| !p.is_local) {
            p.pos[0] += (p.target_pos[0] - p.pos[0]) * factor;
            p.pos[1] += (p.target_pos[1] - p.pos[1]) * factor;
            // Lerp path can clip a corner; keep visual pos out of walls.
            resolve_collision(&mut p.pos, p.radius, crate::map::map_sdf);
        }
    }

    pub fn maybe_broadcast_update(&mut self) {
        if self.last_send.elapsed() < self.send_interval || !self.players[self.local_index].alive {
            return;
        }
        let Some(server_conn) = block_on(self.net_state.local_conn_id()) else {
            return;
        };
        self.last_send = Instant::now();

        let p = &self.players[self.local_index];
        if let Err(e) = self.net_state.handle().send_to(
            server_conn,
            PlayerUpdate::new(self.local_conn_id.unwrap_or(0), p.pos, p.radius).to_msg(),
        ) {
            eprintln!("Failed to send player update: {e}");
        }
    }

    pub fn spawn_remote_projectile(&mut self, owner: ConnId, pos: [f32; 2], dir: [f32; 2]) {
        self.projectiles.push(Projectile {
            pos,
            vel: [dir[0] * PROJECTILE_SPEED, dir[1] * PROJECTILE_SPEED],
            life: PROJECTILE_LIFETIME,
            owner: Some(owner),
        });
    }

    pub fn set_health(&mut self, conn_id: ConnId, hp: f32) {
        let was_alive = self.players[self.local_index].alive;
        for p in self.players.iter_mut() {
            if (p.is_local && self.server_id == Some(conn_id))
                || (!p.is_local && p.conn_id == Some(conn_id))
            {
                p.hp = hp;
                p.alive = hp > 0.;
            }
        }
        if self.local_conn_id == Some(conn_id) && was_alive && hp <= 0. {
            // local just died
        }
    }
}

/// Push `pos` out of any wall by up to `radius`.
/// Uses the map SDF plus central-difference gradient.
fn resolve_collision(pos: &mut [f32; 2], radius: f32, map: impl Fn(f32, f32) -> f32) {
    let d = map(pos[0], pos[1]);
    let penetration = radius - d;
    if penetration <= 0.0 {
        return;
    }

    let gx = (map(pos[0] + EPS, pos[1]) - map(pos[0] - EPS, pos[1])) / (2.0 * EPS);
    let gy = (map(pos[0], pos[1] + EPS) - map(pos[0], pos[1] - EPS)) / (2.0 * EPS);

    let len = (gx * gx + gy * gy).sqrt();
    if len < 1e-6 {
        // Inside a wall with no gradient.
        // nudge toward origin?
        return;
    }

    let nx = gx / len;
    let ny = gy / len;

    pos[0] += nx * penetration;
    pos[1] += ny * penetration;
}
