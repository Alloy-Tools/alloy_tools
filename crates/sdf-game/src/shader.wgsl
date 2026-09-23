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

// Triangle 1: (-1,-1) (1,-1) (1,1)
// Triangle 2: (-1,-1) (1,1) (-1,1)
@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    return vec4<f32>(positions[idx], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    // map with shorter screen dimension as [-0.5, 0.5].
    let centered = frag_coord.xy - 0.5 * globals.resolution;
    let p = vec2<f32>(centered.x, -centered.y) / globals.resolution.y;

    var min_dist: f32 = 1e10;
    var is_local: f32 = 0.0;

    for (var i: u32 = 0u; i < player_data.count; i = i + 1u) {
        let pd = player_data.players[i];
        let d = length(p - pd.xy) - pd.z;
        if (d < min_dist) {
            min_dist = d;
            is_local = pd.w;
        }
    }

    // Colors
    let bg = vec3<f32>(0.08, 0.10, 0.14);
    let remote_color = vec3<f32>(0.95, 0.55, 0.20);
    let local_color = vec3<f32>(0.30, 0.80, 0.50);
    let fg = mix(remote_color, local_color, is_local);

    let t = smoothstep(-0.005, 0.005, min_dist);
    let color = mix(fg, bg, t);

    /*// Hardcoded circle at origin, radius 0.3
    let d = length(p) - 0.3;

    // Anti-aliased edge with a small smoothstep band
    let t = smoothstep(-0.005, 0.005, d);
    let inside_color  = vec3<f32>(0.95, 0.55, 0.20);
    let outside_color = vec3<f32>(0.08, 0.10, 0.14);
    let color = mix(inside_color, outside_color, t);*/

    return vec4<f32>(color, 1.0);
}