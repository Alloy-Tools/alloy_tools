struct Globals {
    resolution: vec2<f32>,
    camera_pos: vec2<f32>,
    cursor_world: vec2<f32>,
    view_size: f32,
    _pad: f32,
};

//REVIEW: Could remove kind branching and always draw capsules
struct SdfPrim {
    origin: vec2<f32>,  // circle center, or capsule origin
    tip: vec2<f32>,     // capsule tip
    radius: f32,
    kind: u32,          // 0 = circle, 1 = capsule
    _pad: vec2<f32>,    // keeps color 16-byte aligned
    color: vec4<f32>,
};

struct SdfData {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    prims: array<SdfPrim, 256>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> sdf: SdfData;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) prim_idx: u32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32,
) -> VsOut {
    let prim = sdf.prims[ii];

    // Two triangles spanning [0,1]^2.
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );
    let c = corners[vi];

    // One pixel worth of world-space padding for AA.
    let px_world = globals.view_size / globals.resolution.y;
    let pad = vec2<f32>(prim.radius + px_world * 1.5);

    let mn = min(prim.origin, prim.tip) - pad;
    let mx = max(prim.origin, prim.tip) + pad;
    let world = mix(mn, mx, c);

    // World -> clip. view_size is the total visible height in world units.
    let aspect = globals.resolution.x / globals.resolution.y;
    let ndc = 2.0 * (world - globals.camera_pos) / (globals.view_size * vec2<f32>(aspect, 1.0));

    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.world = world;
    out.color = prim.color;
    out.prim_idx = ii;
    return out;
}

fn sd_circle(p: vec2<f32>, c: vec2<f32>, r: f32) -> f32 {
    return length(p - c) - r;
}

fn sd_capsule(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-8), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let prim = sdf.prims[in.prim_idx];

    var d: f32;
    if prim.kind == 0u {
        d = sd_circle(in.world, prim.origin, prim.radius);
    } else {
        d = sd_capsule(in.world, prim.origin, prim.tip, prim.radius);
    }

    // Screen-space AA. fwidth gives ~1px in SDF units.
    let aa = max(fwidth(d), 1e-5);
    let alpha = 1.0 - smoothstep(-aa, aa, d);
    if alpha <= 0.0 { discard; }

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}