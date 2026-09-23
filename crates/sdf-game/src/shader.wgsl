struct Globals {
    resolution: vec2<f32>,
    _pad: vec2<f32>,
};

struct PlayerData {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    // x,y = center in world space, z = radius, w = 1.0 if local, else 0.0
    players: array<vec4<f32>, 64>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> player_data: PlayerData;
@group(0) @binding(2) var map_texture: texture_2d<f32>;
@group(0) @binding(3) var map_sampler: sampler;

const WORLD_MIN: f32 = -1.0;
const WORLD_MAX: f32 = 1.0;
const WORLD_SIZE: f32 = WORLD_MAX - WORLD_MIN;

const WORLD_VIEW_SIZE: f32 = 2.4;

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

    let world_p = ndc * WORLD_VIEW_SIZE;

    // ----- Map Dist -----
    let map_uv = vec2<f32>(
        (world_p.x - WORLD_MIN) / WORLD_SIZE,
        (WORLD_MAX - world_p.y) / WORLD_SIZE
    );
    let map_dist = textureSampleLevel(map_texture, map_sampler, map_uv, 0.0).r;

    // ----- Player Dist -----
    var min_player_dist: f32 = 1e10;
    var is_local: f32 = 0.0;
    for (var i: u32 = 0u; i < player_data.count; i = i + 1u) {
        let pd = player_data.players[i];
        let d = length(world_p - pd.xy) - pd.z;
        if d < min_player_dist {
            min_player_dist = d;
            is_local = pd.w;
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
    let player_color = mix(remote_color, local_color, is_local);
    
    let player_t = smoothstep(-0.003, 0.003, min_player_dist);
    let color = mix(player_color, bg, player_t);

    return vec4<f32>(color, 1.0);
}