// MSDF/MTSDF Text Rendering Shader
// Renders text from a pre-baked MSDF or MTSDF atlas.
// Each glyph quad is drawn as two triangles directly in clip space (NDC [-1, 1]).
// No camera transform - text position is completely static and independent of mouse/camera.
//
// Atlas type is selected via text_globals.atlas_type:
//   0 = MSDF (SDF in RGB median)
//   1 = MTSDF (SDF in alpha channel)

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
    atlas_type: f32, // 0 = MSDF, 1 = MTSDF
    _pad: f32,
}

// MSDF edge threshold - controls how sharp the text edges are.
const MSDF_EDGE: f32 = 0.5;

fn msdf_median(sample: vec3<f32>) -> f32 {
    return max(min(sample.r, sample.g), min(max(sample.r, sample.g), sample.b));
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
    // Force LOD 0 — never sample mips (see note below).
    let s = textureSampleLevel(text_atlas, text_atlas_sampler, in.uv, 0.0);

    // Select SDF source based on atlas type.
    let sd = select(msdf_median(s.rgb), s.a, text_globals.atlas_type > 0.5);
    let w = fwidth(sd);

    // ~1 screen-pixel-wide transition, scale-independent.
    let alpha = clamp((sd - MSDF_EDGE) / max(w, 1e-6) + 0.5, 0.0, 1.0);

    let color = in.color;
    return vec4<f32>(color.rgb, color.a * alpha);
}
