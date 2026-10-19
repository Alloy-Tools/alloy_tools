struct Globals {
    resolution: vec2<f32>,
    camera_pos: vec2<f32>,
    cursor_world: vec2<f32>,
    view_size: f32,
    _pad: f32,
}

struct SdfPrim {
    a: vec2<f32>,
    b: vec2<f32>,
    radius: f32,
    rotation: f32,
    kind: u32,
    op: u32,
    color: vec4<f32>,
}

struct SdfData {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    prims: array<SdfPrim, 256>,
}

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(1)
var<uniform> sdf: SdfData;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec2<f32>,
    @location(1) color: vec4<f32>,
    @interpolate(flat)
    @location(2) prim_idx: u32,
}

struct BgVsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec2<f32>,
}

@vertex
fn vs_bg(@builtin(vertex_index) vi: u32) -> BgVsOut {
    var pos = array<vec2<f32>, 3>(vec2<f32>(-1., -3.), vec2<f32>(-1., 1.), vec2<f32>(3., 1.));
    let p = pos[vi];
    let aspect = globals.resolution.x / globals.resolution.y;
    let world = globals.camera_pos + p * 0.5 * globals.view_size * vec2<f32>(aspect, 1.);
    var out: BgVsOut;
    out.clip = vec4<f32>(p, 0., 1.);
    out.world = world;
    return out;
}

fn grid_line(world: vec2<f32>, cell: f32, thickness: f32) -> f32 {
    let g = world / cell;
    let d = abs(fract(g - 0.5) - 0.5);
    let px = max(fwidth(g.x), fwidth(g.y));
    return 1. - smoothstep(0., px * thickness, min(d.x, d.y));
}

@fragment
fn fs_bg(in: BgVsOut) -> @location(0) vec4<f32> {
    let fine = grid_line(in.world, 0.05, 1.);
    let coarse = grid_line(in.world, 0.25, 1.5);

    let px = fwidth(in.world);
    let ax = 1. - smoothstep(0., px.x * 1.5, abs(in.world.x));
    let ay = 1. - smoothstep(0., px.y * 1.5, abs(in.world.y));
    let axis = max(ax, ay);

    let base = vec3<f32>(0.06, 0.06, 0.08);
    let fine_c = vec3<f32>(0.11, 0.11, 0.14);
    let coarse_c = vec3<f32>(0.20, 0.20, 0.26);
    let axis_c = vec3<f32>(0.55, 0.25, 0.35);

    var col = base;
    col = mix(col, fine_c, fine);
    col = mix(col, coarse_c, coarse);
    col = mix(col, axis_c, axis);
    return vec4<f32>(col, 1.);
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VsOut {
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

    var mn: vec2<f32>;
    var mx: vec2<f32>;

    switch prim.kind {
        case 1u {
            let tip = prim.a + prim.b;
            mn = min(prim.a, tip);
            mx = max(prim.a, tip);
        }
        case 2u {
            let c = cos(prim.rotation);
            let s = sin(prim.rotation);
            let hx = prim.b.x;
            let hy = prim.b.y;
            let rx = abs(hx * c) + abs(hy * s);
            let ry = abs(hx * s) + abs(hy * c);
            mn = prim.a - vec2<f32>(rx, ry);
            mx = prim.a + vec2<f32>(rx, ry);
        }
        default {
            mn = prim.a;
            mx = prim.a;
        }
    }
    
    // One pixel worth of world-space padding for AA.
    let px_world = globals.view_size / globals.resolution.y;
    let pad = vec2<f32>(prim.radius + px_world * 1.5);
    mn = mn - pad;
    mx = mx + pad;
    let world = mix(mn, mx, c);

    // World -> clip. view_size is the total visible height in world units.
    let aspect = globals.resolution.x / globals.resolution.y;
    let ndc = 2. * (world - globals.camera_pos) / (globals.view_size * vec2<f32>(aspect, 1.));

    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0., 1.);
    out.world = world;
    out.color = prim.color;
    out.prim_idx = ii;
    return out;
}

// Gathered from `Inigo Quilez` at `https://iquilezles.org/articles/distfunctions2d/`
fn sdf_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdf_capsule(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let h = clamp(dot(p, b) / max(dot(b, b), 1e-8), 0., 1.);
    return length(p - b * h) - r;
}

fn sdf_rect(p: vec2<f32>, h: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - h + r;
    return length(max(q, vec2<f32>(0.))) + min(max(q.x, q.y), 0.) - r;
}

fn rotate(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

fn eval_sdf(p: vec2<f32>, prim: SdfPrim) -> f32 {
    let local = rotate(p - prim.a, -prim.rotation);
    switch prim.kind {
        case 0u { return sdf_circle(local, prim.radius); }
        case 1u { return sdf_capsule(local, prim.b, prim.radius); }
        case 2u { return sdf_rect(local, prim.b, prim.radius); }
        default { return 1e10; }
    }
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let prim = sdf.prims[in.prim_idx];
    let d = eval_sdf(in.world, prim);

    let aa = max(fwidth(d), 1e-5);
    let stroke_w = aa * 1.5;
    let saa = smoothstep(-aa, aa, d);
    let fill = 1. - saa;
    let stroke = saa * (1. - smoothstep(stroke_w - aa, stroke_w + aa, d));
    let alpha = fill + stroke;
    if alpha <= 0. {
        discard;
    }
    let col = mix(prim.color.rgb, prim.color.rgb * 0.3, stroke / max(alpha, 1e-5));

    return vec4<f32>(col, prim.color.a * alpha);
}
