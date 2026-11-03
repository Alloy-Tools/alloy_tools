// MSDF Text Rendering Shader
// Renders text from a pre-baked MSDF (Multi-channel Signed Distance Field) atlas.
// Each glyph quad is drawn as two triangles directly in clip space (NDC [-1, 1]).
// No camera transform - text position is completely static and independent of mouse/camera.

struct TextQuad {
    // Clip-space corners of the quad (in NDC [-1, 1]).
    min: vec2<f32>,
    max: vec2<f32>,
    // Atlas UV coordinates for the quad corners.
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0)
var<storage, read> text_quads: array<TextQuad>;

@group(0) @binding(1)
var text_atlas: texture_2d<f32>;
@group(0) @binding(2)
var text_atlas_sampler: sampler;

@group(0) @binding(3)
var<uniform> text_globals: TextGlobals;

struct TextGlobals {
    atlas_size: vec2<f32>,
    _pad: vec2<f32>,
}

// MSDF edge threshold - controls how sharp the text edges are.
const MSDF_EDGE: f32 = 0.5;

// Sample the MSDF atlas and compute a combined signed distance.
// MSDF encodes distance in all three channels; take the median per Inigo's algorithm.
fn msdf_distance(uv: vec2<f32>, atlas_size: vec2<f32>) -> f32 {
    let s = textureSample(text_atlas, text_atlas_sampler, uv);
    // Median of three: max(min(r,g), min(max(r,g), b))
    let med = max(min(s.r, s.g), min(max(s.r, s.g), s.b));
    // Signed distance: positive inside, negative outside.
    return med - 0.5;
}

struct TextVsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_text(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> TextVsOut {
    let quad = text_quads[ii];

    // Six vertices per quad (two triangles spanning [0,1]^2 in local quad space).
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    let c = corners[vi];

    // Interpolate quad corner positions in NDC and UVs in atlas space.
    let pos_ndc = mix(quad.min, quad.max, c);
    let uv_atlas = mix(quad.uv_min, quad.uv_max, c);

    // Direct clip space - no camera transform at all.
    var out: TextVsOut;
    out.clip = vec4<f32>(pos_ndc.x, -pos_ndc.y, 0.0, 1.0);
    out.uv = uv_atlas;
    out.color = quad.color;
    return out;
}

@fragment
fn fs_text(in: TextVsOut) -> @location(0) vec4<f32> {
    // Sample MSDF distance. Atlas size from uniform.
    let atlas_size = text_globals.atlas_size;
    let d = msdf_distance(in.uv, atlas_size);

    // Convert signed distance to alpha using edge threshold.
    // Inside the glyph: d > 0 (sd > 0.5), Outside: d < 0 (sd < 0.5).
    let edge = MSDF_EDGE / atlas_size.x; // one texel
    let alpha = smoothstep(-edge, edge, d);

    // Premultiply alpha for correct blending.
    let color = in.color;
    return vec4<f32>(color.rgb, color.a * alpha);
}
