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

fn sdf_rounded_rect(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    var cr = r.xy;
    if (p.x <= 0.0) {
        cr = r.zw;
    }
    if (p.y <= 0.0) {
        cr.x = cr.y;
    }
    let q = abs(p) - b + vec2<f32>(cr.x);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - cr.x;
}

fn dot2(v: vec2<f32>) -> f32 {
    return dot(v, v);
}

fn sdf_color(prim: SdfPrim) -> vec4<f32> {
    return unpack4x8unorm(prim.color);
}

// GLSL style mod (always non-negative for a positive modulus).
fn mod_(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

fn sdf_ellipse(p_in: vec2<f32>, ab_in: vec2<f32>) -> f32 {
    var p = abs(p_in);
    var ab = ab_in;
    if p.x > p.y {
        p = p.yx;
        ab = ab.yx;
    }

    let l = ab.y * ab.y - ab.x * ab.x;
    if abs(l) < 1e-7 {
        return length(p) - max(ab.x, ab.y);
    }

    let m = ab.x * p.x / l;
    let m2 = m * m;
    let n = ab.y * p.y / l;
    let n2 = n * n;
    let c = (m2 + n2 - 1.0) / 3.0;
    let c3 = c * c * c;
    let q = c3 + m2 * n2 * 2.0;
    let d = c3 + m2 * n2;
    let g = m + m * n2;

    var co: f32;
    if d < 0.0 {
        let h = acos(q / c3) / 3.0;
        let s = cos(h);
        let t = sin(h) * sqrt(3.0);
        let rx = sqrt(-c * (s + t + 2.0) + m2);
        let ry = sqrt(-c * (s - t + 2.0) + m2);
        co = (ry + sign(l) * rx + abs(g) / (rx * ry) - m) / 2.0;
    } else {
        let h = 2.0 * m * n * sqrt(d);
        let s = sign(q + h) * pow(abs(q + h), 1.0 / 3.0);
        let u = sign(q - h) * pow(abs(q - h), 1.0 / 3.0);
        let rx = -s - u - c * 4.0 + 2.0 * m2;
        let ry = (s - u) * sqrt(3.0);
        let rm = sqrt(rx * rx + ry * ry);
        co = (ry / sqrt(rm - rx) + 2.0 * g / rm - m) / 2.0;
    }

    let r = vec2<f32>(ab.x * co, ab.y * sqrt(max(1.0 - co * co, 0.0)));
    return length(r - p) * sign(p.y - r.y);
}

fn sdf_uneven_capsule(p_in: vec2<f32>, r1: f32, r2: f32, h: f32) -> f32 {
    var p = p_in;
    p.x = abs(p.x);
    let b = (r1 - r2) / h;
    let a = sqrt(max(1.0 - b * b, 0.0));
    let k = dot(p, vec2<f32>(-b, a));
    if k < 0.0 {
        return length(p) - r1;
    }
    if k > a * h {
        return length(p - vec2<f32>(0.0, h)) - r2;
    }
    return dot(p, vec2<f32>(a, b)) - r1;
}

// Corner values are ordered like the rounded box: xy for x > 0, zw otherwise,
// and within each pair x is for y > 0.
fn sdf_chamfer_rect(p_in: vec2<f32>, b: vec2<f32>, chamfers: vec4<f32>) -> f32 {
    var chamfer = chamfers.x;
    if p_in.x <= 0.0 {
        chamfer = select(chamfers.w, chamfers.z, p_in.y > 0.0);
    } else {
        chamfer = select(chamfers.y, chamfers.x, p_in.y > 0.0);
    }

    var p = abs(p_in) - b;
    p = select(p, p.yx, p.y > p.x);
    p.y += chamfer;

    let k = 1.0 - sqrt(2.0);
    if p.y < 0.0 && p.y + p.x * k < 0.0 {
        return p.x;
    }
    if p.x < p.y {
        return (p.x + p.y) * sqrt(0.5);
    }
    return length(p);
}

fn sdf_rhombus(p_in: vec2<f32>, b_in: vec2<f32>) -> f32 {
    let b = vec2<f32>(b_in.x, -b_in.y);
    var p = abs(p_in);
    let h = clamp((dot(b, p) + b.y * b.y) / dot(b, b), 0.0, 1.0);
    p -= b * vec2<f32>(h, h - 1.0);
    return length(p) * sign(p.x);
}

fn sdf_trapezoid(p_in: vec2<f32>, r1: f32, r2: f32, he: f32) -> f32 {
    let k1 = vec2<f32>(r2, he);
    let k2 = vec2<f32>(r2 - r1, 2.0 * he);

    var p = p_in;
    p.x = abs(p.x);
    let nearest = select(r2, r1, p.y < 0.0);
    let ca = vec2<f32>(p.x - min(p.x, nearest), abs(p.y) - he);
    let cb = p - k1 + k2 * clamp(dot(k1 - p, k2) / dot2(k2), 0.0, 1.0);
    let s = select(1.0, -1.0, cb.x < 0.0 && ca.y < 0.0);
    return s * sqrt(min(dot2(ca), dot2(cb)));
}

fn sdf_parallelogram(p_in: vec2<f32>, wi: f32, he: f32, sk: f32) -> f32 {
    let e = vec2<f32>(sk, he);
    var p = select(p_in, -p_in, p_in.y < 0.0);

    var w = p - e;
    w.x -= clamp(w.x, -wi, wi);
    var d = vec2<f32>(dot2(w), -w.y);

    let s = p.x * e.y - p.y * e.x;
    p = select(p, -p, s < 0.0);
    var v = p - vec2<f32>(wi, 0.0);
    v -= e * clamp(dot(v, e) / dot2(e), -1.0, 1.0);
    d = min(d, vec2<f32>(dot2(v), wi * he - abs(s)));

    return sqrt(d.x) * sign(-d.y);
}

fn sdf_triangle(p: vec2<f32>, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>) -> f32 {
    let e0 = p1 - p0;
    let e1 = p2 - p1;
    let e2 = p0 - p2;
    let v0 = p - p0;
    let v1 = p - p1;
    let v2 = p - p2;

    let pq0 = v0 - e0 * clamp(dot(v0, e0) / dot2(e0), 0.0, 1.0);
    let pq1 = v1 - e1 * clamp(dot(v1, e1) / dot2(e1), 0.0, 1.0);
    let pq2 = v2 - e2 * clamp(dot(v2, e2) / dot2(e2), 0.0, 1.0);

    let s = sign(e0.x * e2.y - e0.y * e2.x);
    let d = min(
            min(
                vec2<f32>(dot2(pq0), s * (v0.x * e0.y - v0.y * e0.x)),
                vec2<f32>(dot2(pq1), s * (v1.x * e1.y - v1.y * e1.x)),
            ),
            vec2<f32>(dot2(pq2), s * (v2.x * e2.y - v2.y * e2.x)),
        );

    return -sqrt(d.x) * sign(d.y);
}

fn sdf_equ_triangle(p_in: vec2<f32>, r: f32) -> f32 {
    let k = sqrt(3.0);
    var p = p_in;
    p.x = abs(p.x) - r;
    p.y += r / k;
    if p.x + k * p.y > 0.0 {
        let nx = p.x - k * p.y;
        let ny = -k * p.x - p.y;
        p = vec2<f32>(nx, ny) / 2.0;
    }
    p.x -= clamp(p.x, -2.0 * r, 0.0);
    return -length(p) * sign(p.y);
}

fn sdf_iso_triangle(p_in: vec2<f32>, height: f32, half_base: f32) -> f32 {
    let q = vec2<f32>(half_base, height);
    var p = p_in;
    p.x = abs(p.x);

    let a = p - q * clamp(dot(p, q) / dot2(q), 0.0, 1.0);
    let b = p - q * vec2<f32>(clamp(p.x / q.x, 0.0, 1.0), 1.0);
    let s = -sign(q.y);
    let d = min(
            vec2<f32>(dot2(a), s * (p.x * q.y - p.y * q.x)),
            vec2<f32>(dot2(b), s * (p.y - q.y)),
        );

    return -sqrt(d.x) * sign(d.y);
}

fn sdf_pentagon(p_in: vec2<f32>, r: f32) -> f32 {
    let k = vec2<f32>(0.809016994, 0.587785252);
    let kz = 0.726542528;

    var p = p_in;
    p.x = abs(p.x);
    p -= 2.0 * min(dot(vec2<f32>(-k.x, k.y), p), 0.0) * vec2<f32>(-k.x, k.y);
    p -= 2.0 * min(dot(vec2<f32>(k.x, k.y), p), 0.0) * vec2<f32>(k.x, k.y);
    p -= vec2<f32>(clamp(p.x, -r * kz, r * kz), r);
    return length(p) * sign(p.y);
}

fn sdf_hexagon(p_in: vec2<f32>, r: f32) -> f32 {
    let k = vec2<f32>(-0.866025404, 0.5);
    let kz = 0.577350269;

    var p = abs(p_in);
    p -= 2.0 * min(dot(k, p), 0.0) * k;
    p -= vec2<f32>(clamp(p.x, -r * kz, r * kz), r);
    return length(p) * sign(p.y);
}

fn sdf_octogon(p_in: vec2<f32>, r: f32) -> f32 {
    let k = vec2<f32>(-0.9238795325, 0.3826834323);
    let kz = 0.4142135623;

    var p = abs(p_in);
    p -= 2.0 * min(dot(k, p), 0.0) * k;
    p -= 2.0 * min(dot(vec2<f32>(-k.x, k.y), p), 0.0) * vec2<f32>(-k.x, k.y);
    p -= vec2<f32>(clamp(p.x, -r * kz, r * kz), r);
    return length(p) * sign(p.y);
}

fn sdf_pentagram(p_in: vec2<f32>, r: f32) -> f32 {
    let k1x = 0.809016994;
    let k2x = 0.309016994;
    let k1y = 0.587785252;
    let k2y = 0.951056516;
    let k1z = 0.726542528;

    let v1 = vec2<f32>(k1x, -k1y);
    let v2 = vec2<f32>(-k1x, -k1y);
    let v3 = vec2<f32>(k2x, -k2y);

    var p = p_in;
    p.x = abs(p.x);
    p -= 2.0 * max(dot(v1, p), 0.0) * v1;
    p -= 2.0 * max(dot(v2, p), 0.0) * v2;
    p.x = abs(p.x);
    p.y -= r;
    return length(p - v3 * clamp(dot(p, v3), 0.0, k1z * r)) * sign(p.y * v3.x - p.x * v3.y);
}

fn sdf_hexagram(p_in: vec2<f32>, r: f32) -> f32 {
    let k = vec4<f32>(-0.5, 0.8660254038, 0.5773502692, 1.7320508076);

    var p = abs(p_in);
    p -= 2.0 * min(dot(k.xy, p), 0.0) * k.xy;
    p -= 2.0 * min(dot(k.yx, p), 0.0) * k.yx;
    p -= vec2<f32>(clamp(p.x, r * k.z, r * k.w), r);
    return length(p) * sign(p.y);
}

fn sdf_star(p_in: vec2<f32>, r: f32, n: u32, m: f32) -> f32 {
    let an = 3.1415927 / f32(n);
    let en = 3.1415927 / m;
    let acs = vec2<f32>(cos(an), sin(an));
    let ecs = vec2<f32>(cos(en), sin(en));

    let bn = mod_(atan2(p_in.x, p_in.y), 2.0 * an) - an;
    var p = length(p_in) * vec2<f32>(cos(bn), abs(sin(bn)));
    p -= r * acs;
    p += ecs * clamp(-dot(p, ecs), 0.0, r * acs.y / ecs.y);
    return length(p) * sign(p.x);
}

fn sdf_pie(p_in: vec2<f32>, c: vec2<f32>, r: f32) -> f32 {
    var p = p_in;
    p.x = abs(p.x);
    let l = length(p) - r;
    let m = length(p - c * clamp(dot(p, c), 0.0, r));
    return max(l, m * sign(c.y * p.x - c.x * p.y));
}

fn sdf_cut_disk(p_in: vec2<f32>, r: f32, h: f32) -> f32 {
    let w = sqrt(max(r * r - h * h, 0.0));
    var p = p_in;
    p.x = abs(p.x);
    let s = max((h - r) * p.x * p.x + w * w * (h + r - 2.0 * p.y), h * p.x - w * p.y);
    if s < 0.0 {
        return length(p) - r;
    }
    if p.x < w {
        return h - p.y;
    }
    return length(p - vec2<f32>(w, h));
}

fn sdf_arc(p_in: vec2<f32>, sc: vec2<f32>, ra: f32, rb: f32) -> f32 {
    var p = p_in;
    p.x = abs(p.x);
    return select(abs(length(p) - ra), length(p - sc * ra), sc.y * p.x > sc.x * p.y) - rb;
}

fn sdf_ring(p_in: vec2<f32>, n: vec2<f32>, r: f32, th: f32) -> f32 {
    var p = p_in;
    p.x = abs(p.x);
    p = mat2x2<f32>(
            vec2<f32>(n.x, n.y),
            vec2<f32>(-n.y, n.x),
        ) * p;

    let half_th = th * 0.5;
    return max(
            abs(length(p) - r) - half_th,
            length(vec2<f32>(p.x, max(0.0, abs(r - p.y) - half_th))) * sign(p.x),
        );
}

fn sdf_horseshoe(p_in: vec2<f32>, c: vec2<f32>, r: f32, w: vec2<f32>) -> f32 {
    var p = p_in;
    p.x = abs(p.x);
    let l = length(p);
    p = mat2x2<f32>(
            vec2<f32>(-c.x, c.y),
            vec2<f32>(c.y, c.x),
        ) * p;

    let px = select(l * sign(-c.x), p.x, p.y > 0.0 || p.x > 0.0);
    let py = select(l, p.y, p.x > 0.0);
    p = vec2<f32>(px, abs(py - r)) - w;
    return length(max(p, vec2<f32>(0.0))) + min(0.0, max(p.x, p.y));
}

fn sdf_vesica(p_in: vec2<f32>, w: f32, h: f32) -> f32 {
    let d = 0.5 * (w * w - h * h) / h;
    let p = abs(p_in);
    let use_arc = w * p.y < d * (p.x - w);
    let center = select(vec2<f32>(0.0, -d), vec2<f32>(w, 0.0), use_arc);
    let rad = select(d + h, 0.0, use_arc);
    return length(p - center) - rad;
}

fn sdf_moon(p_in: vec2<f32>, d: f32, ra: f32, rb: f32) -> f32 {
    var p = p_in;
    p.y = abs(p.y);

    let a = (ra * ra - rb * rb + d * d) / (2.0 * d);
    let b = sqrt(max(ra * ra - a * a, 0.0));
    if d * (p.x * b - p.y * a) > d * d * max(b - p.y, 0.0) {
        return length(p - vec2<f32>(a, b));
    }
    return max(length(p) - ra, -(length(p - vec2<f32>(d, 0.0)) - rb));
}

// ra is the bottom radius, rb the top radius.
fn sdf_egg(p_in: vec2<f32>, he: f32, ra: f32, rb: f32, bu: f32) -> f32 {
    let r = 0.5 * (he + ra + rb) / bu;
    let da = r - ra;
    let db = r - rb;
    let y = (db * db - da * da - he * he) / (2.0 * he);
    let x = sqrt(max(da * da - y * y, 0.0));

    var p = p_in;
    p.x = abs(p.x);
    let k = p.y * x - p.x * y;
    if k > 0.0 && k < he * (p.x + x) {
        return length(p + vec2<f32>(x, y)) - r;
    }
    return min(length(p) - ra, length(vec2<f32>(p.x, p.y - he)) - rb);
}

fn sdf_heart(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    p.x = abs(p.x);
    if p.y + p.x > 1.0 {
        return length(p - vec2<f32>(0.25, 0.75)) - sqrt(2.0) / 4.0;
    }
    let center = vec2<f32>(0.5 * max(p.x + p.y, 0.0));
    return sqrt(min(dot2(p - vec2<f32>(0.0, 1.0)), dot2(p - center))) * sign(p.x - p.y);
}

fn sdf_rounded_x(p_in: vec2<f32>, w: f32, r: f32) -> f32 {
    let p = abs(p_in);
    return length(p - vec2<f32>(min(p.x + p.y, w) * 0.5)) - r;
}

fn sdf_cross(p_in: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    var p = abs(p_in);
    p = select(p, p.yx, p.y > p.x);
    let q = p - b;
    let k = max(q.y, q.x);
    let w = select(vec2<f32>(b.y - p.x, -k), q, k > 0.0);
    return sign(k) * length(max(w, vec2<f32>(0.0))) + r;
}

fn sdf_circle_cross(p_in: vec2<f32>, h: f32) -> f32 {
    let k = 0.5 * (h + 1.0 / h);
    let p = abs(p_in);
    if p.x < 1.0 && p.y < p.x * (k - h) + h {
        return k - sqrt(dot2(p - vec2<f32>(1.0, k)));
    }
    return sqrt(min(dot2(p - vec2<f32>(0.0, h)), dot2(p - vec2<f32>(1.0, 0.0))));
}

fn sdf_blobby_cross(p_in: vec2<f32>, he: f32) -> f32 {
    var p = abs(p_in);
    p = vec2<f32>(abs(p.x - p.y), 1.0 - p.x - p.y) / sqrt(2.0);

    let q = (he - p.y - 0.25 / he) / (6.0 * he);
    let r = p.x / (he * he * 16.0);
    let h = r * r - q * q * q;

    var x: f32;
    if h > 0.0 {
        let s = sqrt(h);
        x = pow(r + s, 1.0 / 3.0) - pow(abs(r - s), 1.0 / 3.0) * sign(s - r);
    } else {
        let s = sqrt(q);
        x = 2.0 * s * cos(acos(r / (q * s)) / 3.0);
    }
    x = min(x, sqrt(2.0) / 2.0);

    let z = vec2<f32>(x, he * (1.0 - 2.0 * x * x)) - p;
    return length(z) * sign(z.y);
}

fn sdf_parabola(p_in: vec2<f32>, k: f32) -> f32 {
    var p = p_in;
    p.x = abs(p.x);

    let ik = 1.0 / k;
    let pp = ik * (p.y - 0.5 * ik) / 3.0;
    let q = 0.25 * ik * ik * p.x;
    let h = q * q - pp * pp * pp;

    var x: f32;
    if h > 0.0 {
        let r = pow(q + sqrt(h), 1.0 / 3.0);
        x = r + pp / r;
    } else {
        let r = sqrt(pp);
        x = 2.0 * r * cos(acos(q / (pp * r)) / 3.0);
    }

    return length(p - vec2<f32>(x, k * x * x)) * sign(p.x - x);
}

fn sdf_parabola_segment(p_in: vec2<f32>, wi: f32, he: f32) -> f32 {
    var p = p_in;
    p.x = abs(p.x);

    let ik = wi * wi / he;
    let pp = ik * (he - p.y - 0.5 * ik) / 3.0;
    let q = p.x * ik * ik / 4.0;
    let h = q * q - pp * pp * pp;

    var x: f32;
    if h > 0.0 {
        let r = pow(q + sqrt(h), 1.0 / 3.0);
        x = r + pp / r;
    } else {
        let r = sqrt(pp);
        x = 2.0 * r * cos(acos(q / (pp * r)) / 3.0);
    }
    x = min(x, wi);

    return length(p - vec2<f32>(x, he - x * x / ik)) * sign(ik * (p.y - he) + p.x * p.x);
}

fn sdf_hyperbola(p_in: vec2<f32>, k: f32, he: f32) -> f32 {
    let p0 = abs(p_in);
    let p = vec2<f32>(p0.x - p0.y, p0.x + p0.y) / sqrt(2.0);

    let x2 = p.x * p.x / 16.0;
    let y2 = p.y * p.y / 16.0;
    let r = k * (4.0 * k - p.x * p.y) / 12.0;
    let q = (x2 - y2) * k * k;
    let h = q * q + r * r * r;

    var u: f32;
    if h < 0.0 {
        let m = sqrt(-r);
        u = m * cos(acos(q / (r * m)) / 3.0);
    } else {
        let m = pow(sqrt(h) - q, 1.0 / 3.0);
        u = (m - r / m) / 2.0;
    }

    let w = sqrt(u + x2);
    let b = k * p.y - x2 * p.x * 2.0;
    var t = p.x / 4.0 - w + sqrt(2.0 * x2 - u + b / w / 4.0);
    t = max(t, sqrt(he * he * 0.5 + k) - he / sqrt(2.0));
    let d = length(p - vec2<f32>(t, k / t));
    return select(-d, d, p.x * p.y < k);
}

fn sdf_quadratic_bezier(pos: vec2<f32>, pa: vec2<f32>, pb: vec2<f32>, pc: vec2<f32>) -> f32 {
    let a = pb - pa;
    let b = pa - 2.0 * pb + pc;
    let c = a * 2.0;
    let d = pa - pos;

    let kk = 1.0 / dot2(b);
    let kx = kk * dot(a, b);
    let ky = kk * (2.0 * dot2(a) + dot(d, b)) / 3.0;
    let kz = kk * dot(d, a);

    let p = ky - kx * kx;
    let p3 = p * p * p;
    let q = kx * (2.0 * kx * kx - 3.0 * ky) + kz;
    let h = q * q + 4.0 * p3;

    var res: f32;
    if h >= 0.0 {
        let hh = sqrt(h);
        let x = (vec2<f32>(hh, -hh) - vec2<f32>(q)) / 2.0;
        let uv = sign(x) * pow(abs(x), vec2<f32>(1.0 / 3.0));
        let t = clamp(uv.x + uv.y - kx, 0.0, 1.0);
        res = dot2(d + (c + b * t) * t);
    } else {
        let z = sqrt(-p);
        let v = acos(q / (p * z * 2.0)) / 3.0;
        let m = cos(v);
        let n = sin(v) * 1.732050808;
        let t = clamp(
                vec3<f32>(m + m, -n - m, n - m) * z - vec3<f32>(kx),
                vec3<f32>(0.0),
                vec3<f32>(1.0),
            );
        res = min(dot2(d + (c + b * t.x) * t.x), dot2(d + (c + b * t.y) * t.y));
    }

    return sqrt(res);
}

fn sdf_quadratic_circle(p_in: vec2<f32>) -> f32 {
    var p = abs(p_in);
    p = select(p, p.yx, p.y > p.x);

    let a = p.x - p.y;
    let b = p.x + p.y;
    let c = (2.0 * b - 1.0) / 3.0;
    let h = a * a + c * c * c;

    var t: f32;
    if h >= 0.0 {
        let hh = sqrt(h);
        t = sign(hh - a) * pow(abs(hh - a), 1.0 / 3.0) - pow(hh + a, 1.0 / 3.0);
    } else {
        let z = sqrt(-c);
        let v = acos(a / (c * z)) / 3.0;
        t = -z * (cos(v) + sin(v) * 1.732050808);
    }
    t *= 0.5;

    let w = vec2<f32>(-t, t) + vec2<f32>(0.75) - vec2<f32>(t * t) - p;
    return length(w) * sign(a * a * 0.5 + b - 1.5);
}

fn sdf_tunnel(p_in: vec2<f32>, wh: vec2<f32>) -> f32 {
    var p = p_in;
    p.x = abs(p.x);
    p.y = -p.y;

    var q = p - wh;
    let d1 = dot2(vec2<f32>(max(q.x, 0.0), q.y));
    q.x = select(length(p) - wh.x, q.x, p.y > 0.0);
    let d2 = dot2(vec2<f32>(q.x, max(q.y, 0.0)));
    let d = sqrt(min(d1, d2));

    return select(d, -d, max(q.x, q.y) < 0.0);
}

fn sdf_stairs(p_in: vec2<f32>, wh: vec2<f32>, n: f32) -> f32 {
    let ba = wh * n;
    var p = p_in;

    var d = min(
            dot2(p - vec2<f32>(clamp(p.x, 0.0, ba.x), 0.0)),
            dot2(p - vec2<f32>(ba.x, clamp(p.y, 0.0, ba.y))),
        );
    var s = sign(max(-p.y, p.x - ba.x));

    let dia = length(wh);
    p = mat2x2<f32>(
            vec2<f32>(wh.x, -wh.y),
            vec2<f32>(wh.y, wh.x),
        ) * p / dia;
    let id = clamp(round(p.x / dia), 0.0, n - 1.0);
    p.x = p.x - id * dia;
    p = mat2x2<f32>(
            vec2<f32>(wh.x, wh.y),
            vec2<f32>(-wh.y, wh.x),
        ) * p / dia;

    let hh = wh.y / 2.0;
    p.y -= hh;
    if p.y > hh * sign(p.x) {
        s = 1.0;
    }
    p = select(-p, p, id < 0.5 || p.x > 0.0);

    d = min(d, dot2(p - vec2<f32>(0.0, clamp(p.y, -hh, hh))));
    d = min(d, dot2(p - vec2<f32>(clamp(p.x, 0.0, wh.x), hh)));

    return sqrt(d) * s;
}

fn sdf_circle_wave(p_in: vec2<f32>, tb_in: f32, ra: f32) -> f32 {
    let tb = 3.1415927 * 5.0 / 6.0 * max(tb_in, 0.0001);
    let co = ra * vec2<f32>(sin(tb), cos(tb));

    var p = p_in;
    p.x = abs(mod_(p.x, co.x * 4.0) - co.x * 2.0);

    let p1 = p;
    let p2 = vec2<f32>(abs(p.x - 2.0 * co.x), -p.y + 2.0 * co.y);
    let d1 = select(abs(length(p1) - ra), length(p1 - co), co.y * p1.x > co.x * p1.y);
    let d2 = select(abs(length(p2) - ra), length(p2 - co), co.y * p2.x > co.x * p2.y);
    return min(d1, d2);
}

fn rotate(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

fn eval_shape(local: vec2<f32>, prim: SdfPrim) -> f32 {
    var d_local: f32;
    switch prim.kind {
        case 0u { d_local = sdf_circle(local, prim.radius); }
        case 1u { d_local = sdf_capsule(local, prim.b, prim.radius); }
        case 2u { d_local = sdf_rect(local, prim.b); }
        case 3u { d_local = sdf_ellipse(local, prim.b); }
        case 4u { d_local = sdf_uneven_capsule(local, prim.radius, prim.aux, prim.b.x); }
        case 5u {
            d_local = sdf_rounded_rect(
                    local,
                    prim.b,
                    vec4<f32>(prim.radius, prim.aux, prim.c.x, prim.c.y),
                );
        }
        case 6u {
            d_local = sdf_chamfer_rect(
                    local,
                    prim.b,
                    vec4<f32>(prim.radius, prim.aux, prim.c.x, prim.c.y),
                );
        }
        case 7u { d_local = sdf_rhombus(local, prim.b); }
        case 8u { d_local = sdf_trapezoid(local, prim.radius, prim.aux, prim.b.x); }
        case 9u { d_local = sdf_parallelogram(local, prim.b.x, prim.b.y, prim.radius); }
        case 10u {
            d_local = sdf_triangle(local, prim.b, prim.c, vec2<f32>(prim.radius, prim.aux));
        }
        case 11u { d_local = sdf_equ_triangle(local, prim.radius); }
        case 12u { d_local = sdf_iso_triangle(local, prim.b.y, prim.b.x); }
        case 13u { d_local = sdf_pentagon(local, prim.radius); }
        case 14u { d_local = sdf_hexagon(local, prim.radius); }
        case 15u { d_local = sdf_octogon(local, prim.radius); }
        case 16u { d_local = sdf_pentagram(local, prim.radius); }
        case 17u { d_local = sdf_hexagram(local, prim.radius); }
        case 18u { d_local = sdf_star(local, prim.radius, prim.aux_2, prim.aux); }
        case 19u { d_local = sdf_pie(local, prim.b, prim.radius); }
        case 20u { d_local = sdf_cut_disk(local, prim.radius, prim.b.x); }
        case 21u { d_local = sdf_arc(local, prim.b, prim.radius, prim.aux); }
        case 22u { d_local = sdf_ring(local, prim.b, prim.radius, prim.aux); }
        case 23u { d_local = sdf_horseshoe(local, prim.b, prim.radius, prim.c); }
        case 24u { d_local = sdf_vesica(local, prim.b.x, prim.b.y); }
        case 25u { d_local = sdf_moon(local, prim.b.x, prim.radius, prim.aux); }
        case 26u { d_local = sdf_egg(local, prim.b.x, prim.radius, prim.aux, prim.b.y); }
        case 27u { d_local = sdf_heart(local); }
        case 28u { d_local = sdf_rounded_x(local, prim.aux, prim.radius); }
        case 29u { d_local = sdf_cross(local, prim.b, prim.radius); }
        case 30u { d_local = sdf_circle_cross(local, prim.radius); }
        case 31u { d_local = sdf_blobby_cross(local, prim.radius); }
        case 32u { d_local = sdf_parabola(local, prim.radius); }
        case 33u { d_local = sdf_parabola_segment(local, prim.b.x, prim.b.y); }
        case 34u { d_local = sdf_hyperbola(local, prim.b.x, prim.b.y); }
        case 35u {
            d_local = sdf_quadratic_bezier(local, prim.b, prim.c, vec2<f32>(prim.radius, prim.aux));
        }
        case 36u { d_local = sdf_quadratic_circle(local); }
        case 37u { d_local = sdf_tunnel(local, prim.b); }
        case 38u { d_local = sdf_stairs(local, prim.b, prim.radius); }
        case 39u { d_local = sdf_circle_wave(local, prim.aux, prim.radius); }
        default { d_local = 1e10; }
    }
    return d_local;
}

fn inverse_mat(prim: SdfPrim) -> mat2x2<f32> {
    return mat2x2(
            vec2<f32>(prim.m_inv[0], prim.m_inv[1]),
            vec2<f32>(prim.m_inv[2], prim.m_inv[3]),
        );
}

fn inv_scale(mi: mat2x2<f32>) -> f32 {
    let abs_det_inv = abs(mi[0].x * mi[1].y - mi[1].x * mi[0].y);
    return select(1., 1. / sqrt(abs_det_inv), abs_det_inv > 1e-8);
}

fn eval_sdf(p: vec2<f32>, prim: SdfPrim) -> f32 {
    let mi = inverse_mat(prim);
    let local = mi * (p - prim.a);
    return eval_shape(local, prim) * inv_scale(mi);
}

fn eval_sdf_ops(p: vec2<f32>, prim: SdfPrim, op: SdfOp) -> f32 {
    let mi = inverse_mat(prim);
    let local = mi * (p - prim.a);
    let q = local - clamp(local, -op.elongate, op.elongate);
    var d = eval_shape(q, prim);
    if (op.onion > 0.0) {
        d = abs(d) - op.onion;
    }
    d = d - op.round;
    return d * inv_scale(mi);
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
            local_half = abs(prim.b) + vec2<f32>(prim.radius);
            center_local = vec2<f32>(0.);
        }
        case 3u {
            local_half = abs(prim.b);
            center_local = vec2<f32>(0.);
        }
        case 4u {
            let h = abs(prim.b.x);
            let r = max(abs(prim.radius), abs(prim.aux));
            local_half = vec2<f32>(r, h * 0.5 + r);
            center_local = vec2<f32>(0.0, h * 0.5);
        }
        case 5u {
            let cr = max(max(prim.radius, prim.aux), max(prim.c.x, prim.c.y));
            local_half = abs(prim.b) + vec2<f32>(max(cr, 0.0));
            center_local = vec2<f32>(0.);
        }
        case 6u {
            local_half = abs(prim.b);
            center_local = vec2<f32>(0.);
        }
        case 7u {
            local_half = abs(prim.b);
            center_local = vec2<f32>(0.);
        }
        case 8u {
            local_half = vec2<f32>(max(abs(prim.radius), abs(prim.aux)), abs(prim.b.x));
            center_local = vec2<f32>(0.);
        }
        case 9u {
            local_half = vec2<f32>(abs(prim.b.x) + abs(prim.radius), abs(prim.b.y));
            center_local = vec2<f32>(0.);
        }
        case 10u {
            local_half = max(abs(prim.b), max(abs(prim.c), abs(vec2<f32>(prim.radius, prim.aux))));
            center_local = vec2<f32>(0.);
        }
        case 11u {
            // Vertices are at (+-r, -r/sqrt(3)) and (0, 2r/sqrt(3)).
            let r = abs(prim.radius);
            local_half = vec2<f32>(r, r * 1.1547005);
            center_local = vec2<f32>(0.);
        }
        case 12u {
            let h = abs(prim.b.y);
            local_half = vec2<f32>(abs(prim.b.x), h * 0.5);
            center_local = vec2<f32>(0.0, h * 0.5);
        }
        case 13u {
            // Flat top edge at y = r, circumradius r/sin(54deg).
            let r = abs(prim.radius);
            let circum = r * 1.2360679;
            local_half = vec2<f32>(r * 1.1755705, (r + circum) * 0.5);
            center_local = vec2<f32>(0.0, (r - circum) * 0.5);
        }
        case 14u {
            // Flat top/bottom edges at y = +-r, vertices at x = +-2r/sqrt(3).
            let r = abs(prim.radius);
            local_half = vec2<f32>(r * 1.1547005, r);
            center_local = vec2<f32>(0.);
        }
        case 15u, 16u, 18u {
            local_half = vec2<f32>(abs(prim.radius));
            center_local = vec2<f32>(0.);
        }
        case 17u {
            // Outer points are at (+-sqrt(3)r, 0) and (+-sqrt(3)r/2, +-2r).
            let r = abs(prim.radius);
            local_half = vec2<f32>(r * 1.7320508, r * 2.0);
            center_local = vec2<f32>(0.);
        }
        case 19u {
            local_half = vec2<f32>(abs(prim.radius));
            center_local = vec2<f32>(0.);
        }
        case 20u {
            local_half = vec2<f32>(abs(prim.radius));
            center_local = vec2<f32>(0.);
        }
        case 21u {
            local_half = vec2<f32>(abs(prim.radius) + abs(prim.aux));
            center_local = vec2<f32>(0.);
        }
        case 22u {
            local_half = vec2<f32>(abs(prim.radius) + abs(prim.aux) * 0.5);
            center_local = vec2<f32>(0.);
        }
        case 23u {
            local_half = vec2<f32>(abs(prim.radius) + max(abs(prim.c.x), abs(prim.c.y)));
            center_local = vec2<f32>(0.);
        }
        case 24u {
            local_half = abs(prim.b);
            center_local = vec2<f32>(0.);
        }
        case 25u {
            let r = max(abs(prim.radius), abs(prim.aux));
            local_half = vec2<f32>(max(abs(prim.radius), abs(prim.b.x) + abs(prim.aux)), r);
            center_local = vec2<f32>(0.);
        }
        case 26u {
            let he = abs(prim.b.x);
            let ra = abs(prim.radius);
            let rb = abs(prim.aux);
            let bu = max(abs(prim.b.y), 1e-6);
            let r = 0.5 * (he + ra + rb) / bu;
            let da = r - ra;
            let db = r - rb;
            let y0 = (db * db - da * da - he * he) / (2.0 * max(he, 1e-6));
            let x0 = sqrt(max(da * da - y0 * y0, 0.0));
            // The bulge arc can be wider than either endpoint circle.
            local_half = vec2<f32>(
                max(max(ra, rb), max(r - x0, 0.0)),
                max(ra, he + rb),
            );
            center_local = vec2<f32>(0.);
        }
        case 27u {
            // Heart occupies [-(1/4 + sqrt(2)/4), 1/4 + sqrt(2)/4] x [0, 3/4 + sqrt(2)/4].
            let lobe = 0.35355339;
            let top = 0.75 + lobe;
            local_half = vec2<f32>(0.25 + lobe, top * 0.5);
            center_local = vec2<f32>(0.0, top * 0.5);
        }
        case 28u {
            local_half = vec2<f32>(abs(prim.aux) + abs(prim.radius));
            center_local = vec2<f32>(0.);
        }
        case 29u {
            // The cross is symmetric across x = y; this covers both arms.
            let e = max(abs(prim.b.x), abs(prim.b.y)) + abs(prim.radius);
            local_half = vec2<f32>(e);
            center_local = vec2<f32>(0.);
        }
        case 30u {
            local_half = vec2<f32>(1.0 + abs(prim.radius));
            center_local = vec2<f32>(0.);
        }
        case 31u {
            local_half = vec2<f32>(1.0 + abs(prim.radius));
            center_local = vec2<f32>(0.);
        }
        case 32u, 33u, 34u, 39u {
            // Unbounded or periodic shapes: cover the visible view.
            local_half = vec2<f32>(max(globals.view_size * 4.0, 1.0));
            center_local = vec2<f32>(0.);
        }
        case 35u {
            local_half = max(abs(prim.b), max(abs(prim.c), abs(vec2<f32>(prim.radius, prim.aux))));
            center_local = vec2<f32>(0.);
        }
        case 36u {
            local_half = vec2<f32>(1.0);
            center_local = vec2<f32>(0.);
        }
        case 37u {
            local_half = abs(prim.b) + vec2<f32>(1.0);
            center_local = vec2<f32>(0.);
        }
        case 38u {
            let n = max(abs(prim.radius), 1.0);
            local_half = abs(prim.b) * n;
            center_local = vec2<f32>(0.);
        }
        default {
            local_half = vec2<f32>(abs(prim.radius));
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

fn eval_aabb_ops(prim: SdfPrim, op: SdfOp) -> Aabb {
    var out = eval_aabb(prim);

    let m = mat2_from_inv(
            vec2<f32>(prim.m_inv[0], prim.m_inv[1]),
            vec2<f32>(prim.m_inv[2], prim.m_inv[3]),
        );
    let abs_m = mat2x2<f32>(abs(m[0]), abs(m[1]));
    let grow_local = abs(op.elongate)
        + vec2<f32>(max(op.onion, 0.0) + abs(op.round));
    let grow_world = abs_m * grow_local;
    out.min -= grow_world;
    out.max += grow_world;
    return out;
}

@group(0) @binding(0)
var<uniform> globals: Globals;
@group(0) @binding(1)
var<storage, read> sdf: SdfData;
@group(0) @binding(2)
var<storage, read> ops: array<SdfOp>;

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
    out.color = sdf_color(prim);
    out.prim_idx = ii;
    return out;
}

@vertex
fn vs_main_ops(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VsOut {
    let prim = sdf.prims[ii];
    let op = ops[ii];
    let aabb = eval_aabb_ops(prim, op);

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
