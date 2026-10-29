struct Globals {
    resolution: vec2<f32>,
    camera_pos: vec2<f32>,
    cursor_world: vec2<f32>,
    view_size: f32,
    aux: f32,
}

struct Aabb {
    min: vec2<f32>,
    max: vec2<f32>,
}

struct SdfPrim {
    a: vec2<f32>,
    b: vec2<f32>,
    radius: f32,
    aux: f32,
    aux_2: u32,
    kind: u32,
    m_inv: vec4<f32>,
    color: vec4<f32>,
}

struct SdfData {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    prims: array<SdfPrim>,
}

// elongation

// rounding

// onion

// union

// subtraction

// smooth union

// smooth subtraction

fn grid_line(world: vec2<f32>, cell: f32, thickness: f32) -> f32 {
    let g = world / cell;
    let d = abs(fract(g - 0.5) - 0.5);
    let px = max(fwidth(g.x), fwidth(g.y));
    return 1. - smoothstep(0., px * thickness, min(d.x, d.y));
}

fn mat2_from_inv(mi0: vec2<f32>, mi1: vec2<f32>) -> mat2x2<f32> {
    let det = mi0.x * mi1.y - mi1.x * mi0.y;
    let s = select(0., 1. / det, abs(det) > 1e-8);
    return mat2x2<f32>(
            vec2<f32>(mi1.y, -mi0.y),
            vec2<f32>(-mi1.x, mi0.x),
        ) * s;
}

// Gathered from `Inigo Quilez` at `https://iquilezles.org/articles/distfunctions2d/`
fn sdf_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdf_capsule(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let h = clamp(dot(p, b) / max(dot(b, b), 1e-8), 0., 1.);
    return length(p - b * h) - r;
}

fn sdf_rect(p: vec2<f32>, h: vec2<f32>) -> f32 {
    let q = abs(p) - h;
    return length(max(q, vec2<f32>(0.))) + min(max(q.x, q.y), 0.);
}

fn rotate(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

fn eval_sdf(p: vec2<f32>, prim: SdfPrim) -> f32 {
    let mi = mat2x2(
            vec2<f32>(prim.m_inv[0], prim.m_inv[1]),
            vec2<f32>(prim.m_inv[2], prim.m_inv[3]),
        );
    let c = p - prim.a;
    let local = mi * c;
    // Apply distortions to local (eg elongation)
    var d_local: f32;
    switch prim.kind {
        case 0u { d_local = sdf_circle(local, prim.radius); }
        case 1u { d_local = sdf_capsule(local, prim.b, prim.radius); }
        case 2u { d_local = sdf_rect(local, prim.b); }
        default { d_local = 1e10; }
    }
    // Apply distance modifiers to switch return (eg rounding, onion)

    let abs_det_inv = abs(mi[0].x * mi[1].y - mi[1].x * mi[0].y);

    let scale = select(1., 1. / sqrt(abs_det_inv), abs_det_inv > 1e-8);
    return d_local * scale;
}

fn eval_aabb(prim: SdfPrim) -> Aabb {
    let m = mat2_from_inv(
            vec2<f32>(prim.m_inv[0], prim.m_inv[1]),
            vec2<f32>(prim.m_inv[2], prim.m_inv[3]),
        );

    var local_half: vec2<f32>;
    var center_local: vec2<f32>;
    switch prim.kind {
        case 1u {
            local_half = abs(prim.b) * 0.5 + vec2<f32>(prim.radius);
            center_local = prim.b * 0.5;
        }
        case 2u {
            local_half = prim.b + vec2<f32>(prim.radius);
            center_local = vec2<f32>(0.);
        }
        default {
            local_half = vec2<f32>(prim.radius);
            center_local = vec2<f32>(0.);
        }
    }

    let world_center = prim.a + m * center_local;
    let abs_m = mat2x2<f32>(
            abs(m[0]),
            abs(m[1]),
        );
    let world_half = abs_m * local_half;
    let px_world = globals.view_size / globals.resolution.y;
    let pad = px_world * 6.;
    let mn = world_center - world_half - pad;
    let mx = world_center + world_half + pad;

    var out: Aabb;
    out.min = mn;
    out.max = mx;
    return out;
}

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(1)
var<storage, read> sdf: SdfData;

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

@fragment
fn fs_debug_bg(in: BgVsOut) -> @location(0) vec4<f32> {
    if sdf.count == 0 {
        return vec4<f32>(0.05, 0.05, 0.08, 1.);
    }

    var d = 1e10;
    var prim_col = vec3<f32>(1.);
    for(var i = 0u; i < sdf.count; i++) {
        let prim = sdf.prims[i];
        let dist = eval_sdf(in.world, prim);
        if dist < d {
            prim_col = prim.color.rgb * 0.4;
            d = dist;
        }
    }

    let bands = fract(d * globals.aux);
    var c = select(vec3<f32>(0.85, 0.8, 0.7), vec3<f32>(0.55, 0.45, 0.3), bands > 0.5);
    c = select(c, prim_col, d < 0);

    let px = fwidth(in.world);
    let ax = 1. - smoothstep(0., px.x * 1.5, abs(in.world.x));
    let ay = 1. - smoothstep(0., px.y * 1.5, abs(in.world.y));
    let axis = max(ax, ay);
    c = mix(c, vec3<f32>(0.55, 0.25, 0.35), axis);

    return vec4<f32>(c, 1.);
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VsOut {
    let prim = sdf.prims[ii];
    let aabb = eval_aabb(prim);

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
    let world = mix(aabb.min, aabb.max, c);
    let aspect = globals.resolution.x / globals.resolution.y;
    let ndc = 2. * (world - globals.camera_pos) / (globals.view_size * vec2<f32>(aspect, 1.));

    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0., 1.);
    out.world = world;
    out.color = prim.color;
    out.prim_idx = ii;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let prim = sdf.prims[in.prim_idx];
    let d = eval_sdf(in.world, prim);

    let aa = max(fwidth(d), 1e-5);
    let stroke_w = aa * 3.5;
    let saa = smoothstep(-aa, aa, d);
    let fill = 1. - saa;
    let stroke = saa * (1. - smoothstep(stroke_w - aa, stroke_w + aa, d));
    let alpha = fill + stroke;
    if alpha <= 0. {
        discard;
    }
    let stroke_color = vec3<f32>(0.16, 0.4, 0.24);
    let col = mix(prim.color.rgb, stroke_color, stroke / max(alpha, 1e-5));

    return vec4<f32>(col, prim.color.a * alpha);
}

// ----------------------------------------
// Boolean pass
// ----------------------------------------
struct BooleanUnionOut {
    @location(0) color: vec4<f32>,
    @location(1) d: f32,
}

@fragment
fn fs_boolean_union(in: VsOut) -> BooleanUnionOut {
    let prim = sdf.prims[in.prim_idx];
    let d = eval_sdf(in.world, prim);
    let aa = globals.view_size / globals.resolution.y;

    if d > aa * 5. {
        discard;
    }

    var out: BooleanUnionOut;
    if d < 0. {
        out.color = prim.color;
    } else {
        out.color = vec4<f32>(0.);
    }
    out.d = d;
    return out;
}

struct BooleanSubtractOut {
    @location(0) d: f32,
}

@fragment
fn fs_boolean_subtract(in: VsOut) -> BooleanSubtractOut {
    let prim = sdf.prims[in.prim_idx];
    let d = eval_sdf(in.world, prim);

    var out: BooleanSubtractOut;
    out.d = -d;
    return out;
}

struct CompositeOut {
    @builtin(position) clip: vec4<f32>,
}

@vertex
fn vs_composite(@builtin(vertex_index) vi: u32) -> CompositeOut {
    var pos = array<vec2<f32>, 3>(vec2<f32>(-1., -3.), vec2<f32>(-1., 1.), vec2<f32>(3., 1.));
    let p = pos[vi];
    var out: CompositeOut;
    out.clip = vec4<f32>(p, 0., 1.);
    return out;
}

@group(0) @binding(0)
var union_tex: texture_2d<f32>;
@group(0) @binding(1)
var subtract_tex: texture_2d<f32>;
@group(0) @binding(2)
var color_tex: texture_2d<f32>;
@group(0) @binding(3)
var<uniform> composite_globals: Globals;

@fragment
fn fs_composite(in: CompositeOut) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(in.clip.xy);
    let d_union = textureLoad(union_tex, coords, 0).r;
    let neg_d_sub = textureLoad(subtract_tex, coords, 0).r;
    let color = textureLoad(color_tex, coords, 0);

    let aa = composite_globals.view_size / composite_globals.resolution.y;
    let d = max(d_union, neg_d_sub);
    let stroke_w = aa * 3.5;

    if d > stroke_w + aa {
        discard;
    }

    let saa = smoothstep(-aa, aa, d);
    let fill = 1. - saa;
    let stroke = saa * (1. - smoothstep(stroke_w - aa, stroke_w + aa, d));
    let alpha = min(fill + stroke, 1.);
    if alpha <= 0. {
        discard;
    }

    let stroke_color = vec3<f32>(0.18, 0.32, 0.55);
    let rgb = mix(color.rgb, stroke_color, stroke / max(alpha, 1e-5));
    return vec4<f32>(rgb, alpha);
}
