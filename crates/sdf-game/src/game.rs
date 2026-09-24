use al_events::EventHelpers;
use bytemuck::Zeroable;
use std::time::{Duration, Instant};

use crate::{
    block_on,
    net::{ConnId, NetworkState},
};

pub const MAX_PLAYERS: usize = 64;

#[derive(Default, Clone, Copy)]
pub struct InputState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Clone, Copy)]
pub struct Player {
    pos: [f32; 2],
    radius: f32,
    target_pos: [f32; 2],
    is_local: bool,
    conn_id: Option<ConnId>,
}

impl Player {
    pub fn new(pos: [f32; 2], radius: f32, conn_id: ConnId) -> Self {
        Self {
            pos,
            radius,
            target_pos: pos,
            is_local: false,
            conn_id: Some(conn_id),
        }
    }

    pub fn new_local(pos: [f32; 2], radius: f32) -> Self {
        Self {
            pos,
            radius,
            target_pos: pos,
            is_local: true,
            conn_id: None,
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

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PlayerData {
    count: u32,
    _pad: [u32; 3],
    players: [[f32; 4]; crate::game::MAX_PLAYERS],
}

impl PlayerData {
    pub fn new(game: &GameState) -> Self {
        let mut data = PlayerData::zeroed();
        let count = game.players.len().min(MAX_PLAYERS);
        data.count = count as u32;
        for (i, p) in game.players.iter().take(MAX_PLAYERS).enumerate() {
            let is_local = if p.is_local { 1.0 } else { 0.0 };
            data.players[i] = [p.pos[0], p.pos[1], p.radius, is_local];
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
    pub camera: Camera,
    pub players: Vec<Player>,
    pub local_index: usize,
    pub keys: InputState,
    pub last_frame: Instant,
    pub last_send: Instant,
    pub send_interval: Duration,
    pub net_state: NetworkState,
}

impl GameState {
    pub fn new(send_interval: Duration, net_state: NetworkState) -> Self {
        let players = vec![Player::new_local([0.0, 0.0], 0.05)];
        Self {
            camera: Camera::new([0., 0.], 1.4),
            players,
            local_index: 0,
            keys: InputState::default(),
            last_frame: Instant::now(),
            last_send: Instant::now(),
            send_interval,
            net_state,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        // Local player movement
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
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
        }

        self.sync_remote_players(dt);

        self.camera.update(self.players[self.local_index].pos, dt);
    }

    fn sync_remote_players(&mut self, dt: f32) {
        use std::collections::HashSet;

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

        // Interpolate remote toward targets
        let factor = 1. - (-12. * dt).exp();
        for p in self.players.iter_mut().filter(|p| !p.is_local) {
            p.pos[0] += (p.target_pos[0] - p.pos[0]) * factor;
            p.pos[1] += (p.target_pos[1] - p.pos[1]) * factor;
        }
    }

    pub fn maybe_broadcast_update(&mut self) {
        if self.last_send.elapsed() < self.send_interval {
            return;
        }
        let Some(server_conn) = block_on(self.net_state.local_conn_id()) else {
            return;
        };
        self.last_send = Instant::now();

        let p = &self.players[self.local_index];
        if let Err(e) = self
            .net_state
            .handle()
            .send_to(server_conn, PlayerUpdate::new(0, p.pos, p.radius).to_msg())
        {
            eprintln!("Failed to send player update: {e}");
        }
    }
}
