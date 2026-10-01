struct Slot {
    // center.xy, half_size.xy
    rect: vec4<f32>,
    // radius, thickness, selected (0/1), unused
    params: vec4<f32>,
    // fill rgba
    fill: vec4<f32>,
};

struct MenuUI {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    slots: array<Slot, 16>,
};

@group(0) @binding(0) var<uniform> menu_ui: MenuUI;

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

fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - (half_size - vec2<f32>(r, r));
    let outside = length(max(q, vec2<f32>(0.0)));
    let inside = min(max(q.x, q.y), 0.0);
    return outside + inside - r;
}

fn overlay(top: vec4<f32>, bottom: vec4<f32>) -> vec4<f32> {
    let a = top.a + bottom.a * (1.0 - top.a);
    if a < 1e-5 { return vec4<f32>(0.0); }
    let rgb = (top.rgb * top.a + bottom.rgb * bottom.a * (1.0 - top.a)) / a;
    return vec4<f32>(rgb, a);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let p = frag_coord.xy;
    var out_color = vec4<f32>(0.0);

    for (var i: u32 = 0u; i < menu_ui.count; i = i + 1u) {
        let slot = menu_ui.slots[i];
        let center = slot.rect.xy;
        let half_size = slot.rect.zw;
        let radius = slot.params.x;
        let thickness = slot.params.y;
        let selected = slot.params.z;

        let d = rounded_rect_sdf(p - center, half_size, radius);

        // Dim fill under every option
        let fill_a = (1.0 - smoothstep(-0.5, 0.5, d)) * slot.fill.a;

        // Bright outline around the selected option
        let ring_d = abs(d) - thickness;
        let ring_a = (1.0 - smoothstep(-0.5, 0.5, ring_d)) * selected;

        // Fill under, ring overlay for this slot
        let slot_color = overlay(
            vec4<f32>(vec3<f32>(1.0), ring_a),
            vec4<f32>(slot.fill.rgb, fill_a),
        );
        // Then onto the accumulated result
        out_color = overlay(slot_color, out_color);
    }

    return out_color;
}