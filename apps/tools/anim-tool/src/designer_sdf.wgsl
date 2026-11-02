struct Aabb {
    min: vec2<f32>,
    max: vec2<f32>,
}

struct SdfPrim {
    a: vec2<f32>,
    b: vec2<f32>,
    c: vec2<f32>,
    radius: f32,
    aux: f32,
    aux_2: u32,
    kind: u32,
    color: u32,
    _pad: u32,
    m_inv: vec4<f32>,
}

struct SdfOp {
    round: f32,
    onion: f32,
    elongate: vec2<f32>,
    boolean: u32,
    smoothing: f32,
    _pad0: u32,
    _pad1: u32,
}

fn inverse_mat(_prim: SdfPrim) -> mat2x2<f32> {
    return mat2x2<f32>(vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0));
}

fn sdf_color(_prim: SdfPrim) -> vec4<f32> {
    return vec4<f32>(0.0);
}

fn eval_shape(_local: vec2<f32>, _prim: SdfPrim) -> f32 {
    return 0.0;
}

fn eval_sdf(_p: vec2<f32>, _prim: SdfPrim) -> f32 {
    return 0.0;
}

fn eval_sdf_ops(_p: vec2<f32>, _prim: SdfPrim, _op: SdfOp) -> f32 {
    return 0.0;
}

fn eval_aabb(_prim: SdfPrim, _pad: f32, _view_size: f32) -> Aabb {
    var out: Aabb;
    return out;
}

fn eval_aabb_ops(_prim: SdfPrim, _op: SdfOp, _pad: f32, _view_size: f32) -> Aabb {
    var out: Aabb;
    return out;
}

fn op_union(_a: f32, _b: f32) -> f32 {
    return 0.0;
}

fn op_subtraction(_a: f32, _b: f32) -> f32 {
    return 0.0;
}

fn op_smooth_union(_a: f32, _b: f32, _k: f32) -> f32 {
    return 0.0;
}

fn op_smooth_subtraction(_a: f32, _b: f32, _k: f32) -> f32 {
    return 0.0;
}
// ----- Header End -----
struct Globals {
    resolution: vec2<f32>,
    camera_pos: vec2<f32>,
    cursor_world: vec2<f32>,
    view_size: f32,
    aux: f32,
}

struct SdfData {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    prims: array<SdfPrim>,
}

struct SdfGroup {
    start: u32,
    len: u32,
}

fn grid_line(world: vec2<f32>, cell: f32, thickness: f32) -> f32 {
    let g = world / cell;
    let d = abs(fract(g - 0.5) - 0.5);
    let px = max(fwidth(g.x), fwidth(g.y));
    return 1. - smoothstep(0., px * thickness, min(d.x, d.y));
}

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(1)
var<storage, read> sdf: SdfData;
@group(0) @binding(2)
var<storage, read> ops: array<SdfOp>;
@group(0) @binding(3)
var<storage, read> groups: array<SdfGroup>;

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

struct GroupVsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec2<f32>,
    @interpolate(flat)
    @location(1) group_idx: u32,
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
fn fs_bg_solid() -> @location(0) vec4<f32> {
    return vec4<f32>(0.10, 0.11, 0.14, 1.);
}

@fragment
fn fs_debug_bg(in: BgVsOut) -> @location(0) vec4<f32> {
    if sdf.count == 0 {
        return vec4<f32>(0.05, 0.05, 0.08, 1.);
    }

    var d = 1e10;
    var prim_col = vec3<f32>(1.);
    var inner = false;
    for(var i = 0u; i < sdf.count; i++) {
        let prim = sdf.prims[i];
        let op = ops[i];
        let dist = eval_sdf_ops(in.world, prim, op);
        if dist < d {
            prim_col = sdf_color(prim).rgb * 0.4;
            d = dist;
            let local = inverse_mat(prim) * (in.world - prim.a);
            let q = local - clamp(local, -op.elongate, op.elongate);
            let base = eval_shape(q, prim);
            inner = op.onion > 0.0 && base < 0.0 && dist > 0.0;
        }
    }

    let bands = fract(d * globals.aux);
    var c = select(vec3<f32>(0.85, 0.8, 0.7), vec3<f32>(0.55, 0.45, 0.3), bands > 0.5);
    c = select(c, prim_col, d < 0);
    let inner_c = select(vec3<f32>(0.35, 0.55, 0.95), vec3<f32>(0.10, 0.22, 0.55), bands > 0.5);
    c = select(c, inner_c, inner);

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
    let pad = globals.view_size / globals.resolution.y * 6.;
    let aabb = eval_aabb(prim, pad, globals.view_size);

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
    out.color = sdf_color(prim);
    out.prim_idx = ii;
    return out;
}

@vertex
fn vs_main_ops(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VsOut {
    let prim = sdf.prims[ii];
    let op = ops[ii];
    let pad = globals.view_size / globals.resolution.y * 6.;
    let aabb = eval_aabb_ops(prim, op, pad, globals.view_size);

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
    out.color = sdf_color(prim);
    out.prim_idx = ii;
    return out;
}

fn group_aabb(group: SdfGroup, pad: f32, view_size: f32) -> Aabb {
    let first = group.start;
    var bounds = eval_aabb_ops(sdf.prims[first], ops[first], pad, view_size);
    for (var j = 1u; j < group.len; j++) {
        let i = group.start + j;
        let b = eval_aabb_ops(sdf.prims[i], ops[i], pad, view_size);
        bounds.min = min(bounds.min, b.min);
        bounds.max = max(bounds.max, b.max);
    }
    return bounds;
}

@vertex
fn vs_groups(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> GroupVsOut {
    let group = groups[ii];
    let pad = globals.view_size / globals.resolution.y * 6.;
    let aabb = group_aabb(group, pad, globals.view_size);

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

    var out: GroupVsOut;
    out.clip = vec4<f32>(ndc, 0., 1.);
    out.world = world;
    out.group_idx = ii;
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
    let color = sdf_color(prim);
    let col = mix(color.rgb, stroke_color, stroke / max(alpha, 1e-5));

    return vec4<f32>(col, color.a * alpha);
}

@fragment
fn fs_main_ops(in: VsOut) -> @location(0) vec4<f32> {
    let prim = sdf.prims[in.prim_idx];
    let op = ops[in.prim_idx];
    let d = eval_sdf_ops(in.world, prim, op);

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
    let color = sdf_color(prim);
    let col = mix(color.rgb, stroke_color, stroke / max(alpha, 1e-5));

    return vec4<f32>(col, color.a * alpha);
}

fn shade_sdf(d: f32, color: vec4<f32>) -> vec4<f32> {
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
    let col = mix(color.rgb, stroke_color, stroke / max(alpha, 1e-5));
    return vec4<f32>(col, color.a * alpha);
}

@fragment
fn fs_groups(in: GroupVsOut) -> @location(0) vec4<f32> {
    let group = groups[in.group_idx];

    var d = 1e10;
    var color = vec4<f32>(1.0);
    for (var j = 0u; j < group.len; j++) {
        let i = group.start + j;
        let prim = sdf.prims[i];
        let op = ops[i];
        let dist = eval_sdf_ops(in.world, prim, op);

        if j == 0u {
            d = dist;
            color = sdf_color(prim);
        } else {
            switch op.boolean {
                case 1u {
                    d = op_subtraction(d, dist);
                }
                case 2u {
                    if dist < d {
                        color = sdf_color(prim);
                    }
                    d = op_smooth_union(d, dist, op.smoothing);
                }
                case 3u {
                    d = op_smooth_subtraction(d, dist, op.smoothing);
                }
                default {
                    if dist < d {
                        color = sdf_color(prim);
                    }
                    d = op_union(d, dist);
                }
            }
        }
    }

    return shade_sdf(d, color);
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
        out.color = sdf_color(prim);
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
