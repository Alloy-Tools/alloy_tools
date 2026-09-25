struct Globals {
    resolution: vec2<f32>,
    camera_pos: vec2<f32>,
    cursor_world: vec2<f32>,
    view_size: f32,
    _pad0: f32,
};

struct PlayerData {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    // 64 players * 2 vec4s each
    players: array<vec4<f32>, 128>,
};

struct ProjectileData {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    projectiles: array<vec4<f32>, 64>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> player_data: PlayerData;
@group(0) @binding(2) var map_texture: texture_2d<f32>;
@group(0) @binding(3) var map_sampler: sampler;
@group(0) @binding(4) var<uniform> projectile_data: ProjectileData;

const WORLD_MIN: f32 = -1.0;
const WORLD_MAX: f32 = 1.0;
const WORLD_SIZE: f32 = WORLD_MAX - WORLD_MIN;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    return vec4<f32>(positions[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    // map with shorter screen dimension as [-0.5, 0.5].
    let centered = frag_coord.xy - 0.5 * globals.resolution;
    let ndc = vec2<f32>(centered.x, -centered.y) / globals.resolution.y;

    let world_p = globals.camera_pos + ndc * globals.view_size;

    // ----- Map Dist -----
    let map_uv = vec2<f32>(
        (world_p.x - WORLD_MIN) / WORLD_SIZE,
        (WORLD_MAX - world_p.y) / WORLD_SIZE
    );
    let map_dist = textureSampleLevel(map_texture, map_sampler, map_uv, 0.0).r;

    // ----- Player Dist -----
    var min_player_dist: f32 = 1e10;
    var is_local: f32 = 0.;
    var player_hp: f32 = 1.;
    for (var i: u32 = 0u; i < player_data.count; i = i + 1u) {
        let base = i * 2u;
        let da = player_data.players[base];
        let db = player_data.players[base + 1u];
        // da = [x, y, radius, is_local]
        // db = [hp_norm, alive, 0, 0]
        if db.y <= 0.5 { continue; }

        let d = length(world_p - da.xy) - da.z;
        if d < min_player_dist {
            min_player_dist = d;
            is_local = da.w;
            player_hp = db.x;
        }
    }

    // ----- Map Colors -----
    // wall for `<0` else near or floor
    let wall_color = vec3<f32>(0.1, 0.11, 0.14);
    let floor_near_wall = vec3<f32>(0.22, 0.24, 0.28);
    let floor = vec3<f32>(0.34, 0.37, 0.44);

    let map_t = smoothstep(-0.003, 0.003, map_dist);
    let floor_blend = smoothstep(0.0, 0.15, map_dist);
    let floor_color = mix(floor_near_wall, floor, floor_blend);
    let bg = mix(wall_color, floor_color, map_t);

    // ----- Player drawn on top -----
    let remote_color = vec3<f32>(0.95, 0.55, 0.20);
    let local_color = vec3<f32>(0.30, 0.80, 0.50);
    let base_player_color = mix(remote_color, local_color, is_local);
    let hurt_tint = vec3<f32>(0.9, 0.15, 0.15);
    let player_color = mix(base_player_color, hurt_tint, (1. - player_hp) * 0.7);

    let player_t = smoothstep(-0.003, 0.003, min_player_dist);
    let color_0 = mix(player_color, bg, player_t);

    // ----- Projectiles -----
    var min_proj_dist: f32 = 1e10;
    var proj_is_local: f32 = 0.;
    for (var i: u32 = 0u; i < projectile_data.count; i = i + 1u) {
        let pd = projectile_data.projectiles[i];
        let d = length(world_p - pd.xy) - pd.z;
        if d < min_proj_dist {
            min_proj_dist = d;
            proj_is_local = pd.w;
        }
    }
    let proj_color = mix(
        vec3<f32>(1., 0.85, 0.25),
        vec3<f32>(1., 0.95, 0.55),
        proj_is_local
    );
    let proj_t = smoothstep(-0.0015, 0.0015, min_proj_dist);
    let color_1 = mix(proj_color, color_0, proj_t);

    let cpos = world_p - globals.cursor_world;
    let d_ring = abs(length(cpos) - 0.012) - 0.0018;
    let d_dot = length(cpos) - 0.0025;
    let d_cross = min(d_ring, d_dot);
    let cross_alpha = 1. - smoothstep(-0.0008, 0.0008, d_cross);
    let color_2 = mix(color_1, vec3<f32>(0.95, 0.95, 0.98), cross_alpha);

    return vec4<f32>(color_2, 1.0);
}