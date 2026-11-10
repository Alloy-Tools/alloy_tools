// MSDF/MTSDF Text Rendering Shader
// Renders text from a pre-baked MSDF or MTSDF atlas.
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
    effects: u32,
    baseline_y: f32,
    midline_y: f32,
    underline_thickness: f32,
    strike_thickness: f32,
    highlight_pad: f32,
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
    msdf_edge: f32,
    px_range: f32,
}

fn msdf_median(sample: vec3<f32>) -> f32 {
    return max(min(sample.r, sample.g), min(max(sample.r, sample.g), sample.b));
}

struct TextVsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @interpolate(flat)
    @location(2) effects: u32,
    @interpolate(flat)
    @location(3) baseline_y: f32,
    @interpolate(flat)
    @location(4) midline_y: f32,
    @interpolate(flat)
    @location(5) underline_thickness: f32,
    @interpolate(flat)
    @location(6) strike_thickness: f32,
    @interpolate(flat)
    @location(7) highlight_pad: f32,
    @location(8) ndc_y: f32,
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
    out.effects = quad.effects;
    out.baseline_y = quad.baseline_y;
    out.midline_y = quad.midline_y;
    out.underline_thickness = quad.underline_thickness;
    out.strike_thickness = quad.strike_thickness;
    out.highlight_pad = quad.highlight_pad;
    out.ndc_y = pos_ndc.y;
    return out;
}

@fragment
fn fs_text(in: TextVsOut) -> @location(0) vec4<f32> {
    let s = textureSampleLevel(text_atlas, text_atlas_sampler, in.uv, 0.0);

    let msd = msdf_median(s.rgb);
    let disagree = (msd > text_globals.msdf_edge) != (s.a > text_globals.msdf_edge);
    let sd = select(msd, s.a, disagree);

    let dx_texels = dpdx(in.uv) * text_globals.atlas_size;
    let dy_texels = dpdy(in.uv) * text_globals.atlas_size;
    let texels_per_px = sqrt(dot(dx_texels, dy_texels) + dot(dy_texels, dy_texels));

    let screen_px_range = text_globals.px_range / max(texels_per_px, 1e-6);
    let alpha = clamp(screen_px_range * (sd - text_globals.msdf_edge) + 0.5, 0.0, 1.0);

    let color = in.color;
    return vec4<f32>(color.rgb, color.a * alpha);
}

const EFFECT_UNDERLINE: u32 = 1u;
const EFFECT_STRIKETHROUGH: u32 = 1u << 1u;
const EFFECT_HIGHLIGHT: u32 = 1u << 2u;
const EFFECT_FILL: u32 = 1u << 3u;
const EFFECT_OUTLINE: u32 = 1u << 4u;
const EFFECT_GLOW: u32 = 1u << 5u;

// Hardcoded for now.
const UNDERLINE_COLOR: vec4<f32> = vec4(0.0, 0.0, 1.0, 1.0);
const STRIKE_COLOR: vec4<f32> = vec4(1.0, 0.0, 0.0, 1.0);
const HIGHLIGHT_COLOR: vec4<f32> = vec4(1.0, 1.0, 0.0, 0.5);
const OUTLINE_COLOR: vec4<f32> = vec4(0.0, 0.0, 0.0, 0.5);
const GLOW_COLOR: vec4<f32> = vec4(1., 1., 0., 0.8);
//REQUIRED: atlas padding ≥ max(OUTLINE_WIDTH, GLOW_WIDTH)
const OUTLINE_WIDTH: f32 = 2.0; // screen px
const GLOW_WIDTH: f32 = 10.0; // screen px

@fragment
fn fs_effect_text(in: TextVsOut) -> @location(0) vec4<f32> {
    let effects = in.effects;
    let edge = text_globals.msdf_edge;

    // Decorations (no texture sample)
    if (effects & EFFECT_UNDERLINE) != 0u && in_underline_band(in) {
        return UNDERLINE_COLOR;
    }
    if (effects & EFFECT_STRIKETHROUGH) != 0u && in_strike_band(in) {
        return STRIKE_COLOR;
    }

    var color = vec4<f32>(0.0);
    if (effects & EFFECT_HIGHLIGHT) != 0u {
        color = HIGHLIGHT_COLOR;
    }

    // Shared sample
    let s = textureSampleLevel(text_atlas, text_atlas_sampler, in.uv, 0.0);
    let msd = msdf_median(s.rgb);
    let disagree = (msd > edge) != (s.a > edge);
    let sd = select(msd, s.a, disagree);

    let dx_texels = dpdx(in.uv) * text_globals.atlas_size;
    let dy_texels = dpdy(in.uv) * text_globals.atlas_size;
    let texels_per_px = sqrt(dot(dx_texels, dy_texels) + dot(dy_texels, dy_texels));
    let texel_scale = texels_per_px / text_globals.px_range;
    let hw = 0.5 * texel_scale;

    if (effects & EFFECT_GLOW) != 0u {
        let w = GLOW_WIDTH * texel_scale;
        let t = clamp((sd - (edge - w)) / max(w, 1e-6), 0.0, 1.0);
        let a = GLOW_COLOR.a * t;
        if a > 0.001 {
            color = mix(color, vec4(GLOW_COLOR.rgb, 1.), a);
        }
    }
    if (effects & EFFECT_OUTLINE) != 0u {
        let w = OUTLINE_WIDTH * texel_scale;
        let outer = smoothstep(edge - w - hw, edge - w + hw, sd);
        let inner = smoothstep(edge - hw, edge + hw, sd);
        let a = outer * (1.0 - inner) * OUTLINE_COLOR.a;
        if a > 0.001 {
            color = mix(color, vec4(OUTLINE_COLOR.rgb, 1.), a);
        }
    }
    if (effects & EFFECT_FILL) != 0u {
        let a = smoothstep(edge - hw, edge + hw, sd) * in.color.a;
        if a > 0.001 {
            color = mix(color, vec4(in.color.rgb, 1.), a);
        }
    }

    return vec4<f32>(color);
}

fn in_underline_band(in: TextVsOut) -> bool {
    let y = in.ndc_y;
    return y >= in.baseline_y && y <= in.baseline_y + in.underline_thickness;
}

fn in_strike_band(in: TextVsOut) -> bool {
    let y = in.ndc_y;
    return y >= in.midline_y - in.strike_thickness * 0.5
        && y <= in.midline_y + in.strike_thickness * 0.5;
}
