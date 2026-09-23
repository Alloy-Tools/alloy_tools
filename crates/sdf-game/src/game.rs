use std::time::Instant;
use bytemuck::Zeroable;

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
}

impl Player {
    pub fn new(pos: [f32; 2], radius: f32) -> Self {
        Self { pos, radius }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PlayerData {
    count: u32,
    _pad: [u32; 3],
    players: [[f32; 4]; crate::game::MAX_PLAYERS]
}

impl PlayerData {
    pub fn new(game: &GameState) -> Self {
        let mut data = PlayerData::zeroed();
        let count = game.players.len().min(MAX_PLAYERS);
        data.count = count as u32;
        for (i, p) in game.players.iter().take(MAX_PLAYERS).enumerate() {
            let is_local = if i == game.local_index { 1.0 } else { 0.0 };
            data.players[i] = [p.pos[0], p.pos[1], p.radius, is_local];
        }
        data
    }
}

pub struct GameState {
    pub players: Vec<Player>,
    pub local_index: usize,
    pub keys: InputState,
    pub last_frame: Instant,
}

impl GameState {
    pub fn new() -> Self {
        let players = vec![
            Player::new([0.0, 0.0], 0.05),
            Player::new([-0.30, 0.20], 0.04),
            Player::new([0.25, -0.15], 0.06),
        ];
        Self {
            players,
            local_index: 0,
            keys: InputState::default(),
            last_frame: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

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
        let p = &mut self.players[self.local_index];
        p.pos[0] += dx * speed * dt;
        p.pos[1] += dy * speed * dt;
    }
}
