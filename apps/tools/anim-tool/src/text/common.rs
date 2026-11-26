use std::{collections::HashMap, path::PathBuf};

pub const EFFECT_UNDERLINE: u32 = 1;
pub const EFFECT_STRIKETHROUGH: u32 = 1 << 1;
pub const EFFECT_HIGHLIGHT: u32 = 1 << 2;
pub const EFFECT_FILL: u32 = 1 << 3;
pub const EFFECT_OUTLINE: u32 = 1 << 4;
pub const EFFECT_GLOW: u32 = 1 << 5;
pub const EFFECT_WAVE_GLYPH: u32 = 1 << 6;
pub const EFFECT_WAVE_STYLE: u32 = 1 << 7;

/// Everything the renderer needs to draw one glyph.
#[derive(Clone, Copy, Debug)]
pub struct GlyphInfo {
    /// Atlas UVs, already normalized to [0,1].
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    /// Quad size in pixels at the reference em size (before scaling).
    pub quad_size: [f32; 2],
    /// Offset from pen origin to the top-left of the quad, in pixels.
    pub bearing: [f32; 2],
    /// How far to advance the pen after this glyph, in pixels.
    pub advance: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextGlobals {
    atlas_size: [f32; 2],
    msdf_edge: f32,
    time: f32,
}

impl TextGlobals {
    pub fn new(atlas_size: [f32; 2], msdf_edge: f32, time: f32) -> Self {
        Self {
            atlas_size,
            msdf_edge,
            time,
        }
    }

    pub fn atlas_size(&self) -> [f32; 2] {
        self.atlas_size
    }

    pub fn msdf_edge(&self) -> f32 {
        self.msdf_edge
    }

    pub fn time(&self) -> f32 {
        self.time
    }
}

/// A single text rendering instance with its own text, font size, position, and color.
#[derive(Clone, Debug)]
pub struct TextInstance {
    text: String,
    position: [f32; 2],
    font_handle: FontHandle,
    shape_ops: ShapeOptions,
    style: TextStyle,
}

impl TextInstance {
    pub fn new(
        text: impl Into<String>,
        position: [f32; 2],
        font_handle: FontHandle,
        shape_ops: ShapeOptions,
        style: TextStyle,
    ) -> Self {
        Self {
            text: text.into(),
            position,
            font_handle,
            shape_ops,
            style,
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn position(&self) -> [f32; 2] {
        self.position
    }

    pub fn font_handle(&self) -> FontHandle {
        self.font_handle
    }

    pub fn shape_ops(&self) -> ShapeOptions {
        self.shape_ops
    }

    pub fn style(&self) -> TextStyle {
        self.style
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextQuad {
    min: [f32; 2],
    max: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    //TODO: pack into RGB8?
    color: [f32; 4],
    px_range: f32,
    baseline_y: f32,
    midline_y: f32,
    effects: u32,
}

impl TextQuad {
    pub fn new(
        min: [f32; 2],
        max: [f32; 2],
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        color: [f32; 4],
        px_range: f32,
        baseline_y: f32,
        midline_y: f32,
        effects: u32,
    ) -> Self {
        Self {
            min,
            max,
            uv_min,
            uv_max,
            color,
            px_range,
            baseline_y,
            midline_y,
            effects,
        }
    }

    pub fn min(&self) -> [f32; 2] {
        self.min
    }

    pub fn min_mut<F: Fn(&mut [f32; 2])>(&mut self, f: F) {
        f(&mut self.min)
    }

    pub fn max(&self) -> [f32; 2] {
        self.max
    }

    pub fn max_mut<F: Fn(&mut [f32; 2])>(&mut self, f: F) {
        f(&mut self.max)
    }

    pub fn uv_min(&self) -> [f32; 2] {
        self.uv_min
    }

    pub fn uv_min_mut<F: Fn(&mut [f32; 2])>(&mut self, f: F) {
        f(&mut self.uv_min)
    }

    pub fn uv_max(&self) -> [f32; 2] {
        self.uv_max
    }

    pub fn uv_max_mut<F: Fn(&mut [f32; 2])>(&mut self, f: F) {
        f(&mut self.uv_max)
    }

    pub fn color(&self) -> [f32; 4] {
        self.color
    }

    pub fn color_mut<F: Fn(&mut [f32; 4])>(&mut self, f: F) {
        f(&mut self.color)
    }

    pub fn px_range(&self) -> f32 {
        self.px_range
    }

    pub fn px_range_mut<F: Fn(f32) -> f32>(&mut self, f: F) {
        self.px_range = f(self.px_range)
    }

    pub fn baseline_y(&self) -> f32 {
        self.baseline_y
    }

    pub fn baseline_y_mut<F: Fn(f32) -> f32>(&mut self, f: F) {
        self.baseline_y = f(self.baseline_y)
    }

    pub fn midline_y(&self) -> f32 {
        self.midline_y
    }

    pub fn midline_y_mut<F: Fn(f32) -> f32>(&mut self, f: F) {
        self.midline_y = f(self.midline_y)
    }

    pub fn effects(&self) -> u32 {
        self.effects
    }

    pub fn effects_mut<F: Fn(u32) -> u32>(&mut self, f: F) {
        self.effects = f(self.effects)
    }
}

// ----- Font Registration -----
#[derive(Debug, Clone)]
/// A font atlas with glyph metadata for rendering.
pub struct FontAtlas {
    size: u32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    fonts: Vec<FontEntry>,
    globals_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl FontAtlas {
    pub fn new(
        size: u32,
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        sampler: wgpu::Sampler,
        fonts: Vec<FontEntry>,
        globals_buffer: wgpu::Buffer,
        bind_group: wgpu::BindGroup,
    ) -> Self {
        Self {
            size,
            texture,
            view,
            sampler,
            fonts,
            globals_buffer,
            bind_group,
        }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn fonts(&self) -> &[FontEntry] {
        &self.fonts
    }

    pub fn globals_buffer(&self) -> &wgpu::Buffer {
        &self.globals_buffer
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug, Clone)]
pub struct FontEntry {
    spec: FontSpec,
    metrics: FontMetrics,
    glyphs: HashMap<char, GlyphInfo>,
}

impl FontEntry {
    pub fn new(spec: FontSpec, metrics: FontMetrics, glyphs: HashMap<char, GlyphInfo>) -> Self {
        Self {
            spec,
            metrics,
            glyphs,
        }
    }

    pub fn spec(&self) -> &FontSpec {
        &self.spec
    }

    pub fn metrics(&self) -> FontMetrics {
        self.metrics
    }

    pub fn glyphs(&self) -> &HashMap<char, GlyphInfo> {
        &self.glyphs
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtlasHandle(u32);

impl AtlasHandle {
    pub fn new(atlas: u32) -> Self {
        Self(atlas)
    }

    pub fn id(&self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontHandle(AtlasHandle, u32);

impl FontHandle {
    pub fn new(atlas: AtlasHandle, font: u32) -> Self {
        Self(atlas, font)
    }

    pub fn atlas(&self) -> AtlasHandle {
        self.0
    }

    pub fn font(&self) -> u32 {
        self.1
    }
}

#[derive(Clone, Debug)]
pub struct FontSpec {
    name: String,
    path: PathBuf,
    charset: Charset,
    em_size: f32,
}

impl FontSpec {
    pub fn new(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        charset: impl Into<Charset>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            charset: charset.into(),
            em_size: 48.,
        }
    }

    pub fn with_em_size(mut self, em_size: f32) -> Self {
        self.em_size = em_size;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn charset(&self) -> &Charset {
        &self.charset
    }

    pub fn em_size(&self) -> f32 {
        self.em_size
    }
}

pub const ASCII_PRINTABLE_CHARS: [char; 95] = {
    let mut arr = [' '; 95];
    let mut i = 0;
    while i < arr.len() {
        arr[i] = (0x20 + i as u8) as char;
        i += 1;
    }
    arr
};

pub const LATIN1_PRINTABLE_CHARS: [char; 191] = {
    let mut arr = [' '; 191];
    let mut i = 0;
    while i < 95 {
        arr[i] = (0x20 + i as u8) as char;
        i += 1;
    }
    i = 0;
    while i < 96 {
        arr[95 + i] = (0xA0 + i as u8) as char;
        i += 1;
    }
    arr
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Charset {
    #[default]
    Ascii,
    Latin1,
    Custom(Vec<char>),
}

impl From<Vec<char>> for Charset {
    fn from(value: Vec<char>) -> Self {
        Self::Custom(value)
    }
}

impl Charset {
    pub fn iter(&self) -> CharsetIter<'_> {
        match self {
            Charset::Ascii => CharsetIter(ASCII_PRINTABLE_CHARS.iter()),
            Charset::Latin1 => CharsetIter(LATIN1_PRINTABLE_CHARS.iter()),
            Charset::Custom(items) => CharsetIter(items.iter()),
        }
    }
}

impl<'a> IntoIterator for &'a Charset {
    type Item = &'a char;

    type IntoIter = CharsetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct CharsetIter<'a>(std::slice::Iter<'a, char>);

impl<'a> Iterator for CharsetIter<'a> {
    type Item = &'a char;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> DoubleEndedIterator for CharsetIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl ExactSizeIterator for CharsetIter<'_> {}
impl std::iter::FusedIterator for CharsetIter<'_> {}

#[derive(Debug)]
pub enum FontError {
    Io(std::io::Error),
    Parse(String),
    AtlasFull { missing: Vec<char> },
    DuplicateName(String),
    InvalidHandle,
}

#[derive(Clone, Copy, Debug)]
pub struct FontMetrics {
    /// bake size
    em_size: f32,
    ascent_n: f32,
    descent_n: f32,
    line_gap_n: f32,
    cap_height_n: f32,
    x_height_n: f32,
    underline_offset_n: f32,
    underline_size_n: f32,
    strikethrough_offset_n: f32,
    strikethrough_size_n: f32,
}

impl FontMetrics {
    pub fn new(
        em_size: f32,
        ascent_n: f32,
        descent_n: f32,
        line_gap_n: f32,
        cap_height_n: f32,
        x_height_n: f32,
        underline_offset_n: f32,
        underline_size_n: f32,
        strikethrough_offset_n: f32,
        strikethrough_size_n: f32,
    ) -> Self {
        Self {
            em_size,
            ascent_n,
            descent_n,
            line_gap_n,
            cap_height_n,
            x_height_n,
            underline_offset_n,
            underline_size_n,
            strikethrough_offset_n,
            strikethrough_size_n,
        }
    }

    pub fn em_size(&self) -> f32 {
        self.em_size
    }

    pub fn ascent_n(&self) -> f32 {
        self.ascent_n
    }

    pub fn descent_n(&self) -> f32 {
        self.descent_n
    }

    pub fn line_gap_n(&self) -> f32 {
        self.line_gap_n
    }

    pub fn cap_height_n(&self) -> f32 {
        self.cap_height_n
    }

    pub fn x_height_n(&self) -> f32 {
        self.x_height_n
    }

    pub fn underline_offset_n(&self) -> f32 {
        self.underline_offset_n
    }

    pub fn underline_size_n(&self) -> f32 {
        self.underline_size_n
    }

    pub fn strikethrough_offset_n(&self) -> f32 {
        self.strikethrough_offset_n
    }

    pub fn strikethrough_size_n(&self) -> f32 {
        self.strikethrough_size_n
    }
}

// ----- Text Shaping -----
/// Wrapping mode for text layout.
#[derive(Clone, Copy, Debug, Default)]
pub enum WrapMode {
    /// Wrap at character boundaries (breaks mid-word if needed).
    #[default]
    PerChar,
    /// Wrap at word boundaries (keeps whole words together).
    PerWord,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum LineHeight {
    #[default]
    Metrics, // (ascent_n + descent_n + line_gap_n) * size_px
    Multiplier(f32), // size_px * n
    Fixed(f32),      // absolute NDC
}

pub fn px_to_ndc(px: f32, screen_height: f32) -> f32 {
    px / screen_height * 2.0
}

#[derive(Debug, Clone, Copy)]
pub struct ShapeOptions {
    size_ndc: f32,          // em_size in NDC units
    max_width: Option<f32>, // NDC wrap width; None = no wrap
    wrap: WrapMode,
    line_height: LineHeight,
    align: Align,
    baseline_offset: f32,
    midline_offset: f32,
}

impl ShapeOptions {
    pub fn new(
        size_ndc: f32,
        max_width: Option<f32>,
        wrap: WrapMode,
        line_height: LineHeight,
        align: Align,
        baseline_offset: f32,
        midline_offset: f32,
    ) -> Self {
        Self {
            size_ndc,
            max_width,
            wrap,
            line_height,
            align,
            baseline_offset,
            midline_offset,
        }
    }

    pub fn size_ndc(&self) -> f32 {
        self.size_ndc
    }

    pub fn size_ndc_mut<F: Fn(f32) -> f32>(&mut self, f: F) {
        self.size_ndc = f(self.size_ndc);
    }

    pub fn max_width(&self) -> Option<f32> {
        self.max_width
    }

    pub fn max_width_mut<F: Fn(&mut Option<f32>)>(&mut self, f: F) {
        f(&mut self.max_width);
    }

    pub fn wrap(&self) -> WrapMode {
        self.wrap
    }

    pub fn wrap_mode_mut<F: Fn(WrapMode) -> WrapMode>(&mut self, f: F) {
        self.wrap = f(self.wrap);
    }

    pub fn line_height(&self) -> LineHeight {
        self.line_height
    }

    pub fn line_height_mut<F: Fn(LineHeight) -> LineHeight>(&mut self, f: F) {
        self.line_height = f(self.line_height);
    }

    pub fn align(&self) -> Align {
        self.align
    }

    pub fn align_mut<F: Fn(Align) -> Align>(&mut self, f: F) {
        self.align = f(self.align);
    }

    pub fn baseline_offset(&self) -> f32 {
        self.baseline_offset
    }

    pub fn baseline_offset_mut<F: Fn(f32) -> f32>(&mut self, f: F) {
        self.baseline_offset = f(self.baseline_offset);
    }

    pub fn midline_offset(&self) -> f32 {
        self.midline_offset
    }

    pub fn midline_offset_mut<F: Fn(f32) -> f32>(&mut self, f: F) {
        self.midline_offset = f(self.midline_offset);
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// A shaped, laid-out text — reusable across frames.
#[derive(Clone, Debug)]
pub struct ShapedText {
    quads: Vec<TextQuad>,
    effects: Vec<TextEffect>,
    bounds: [f32; 4],
    line_count: usize,
    atlas: AtlasHandle,
}

impl ShapedText {
    pub fn new(
        quads: Vec<TextQuad>,
        effects: Vec<TextEffect>,
        bounds: [f32; 4],
        line_count: usize,
        atlas: AtlasHandle,
    ) -> Self {
        Self {
            quads,
            effects,
            bounds,
            line_count,
            atlas,
        }
    }

    pub fn quads(&self) -> &[TextQuad] {
        &self.quads
    }

    pub fn effects(&self) -> &[TextEffect] {
        &self.effects
    }

    pub fn bounds(&self) -> [f32; 4] {
        self.bounds
    }

    pub fn line_count(&self) -> usize {
        self.line_count
    }

    pub fn atlas(&self) -> AtlasHandle {
        self.atlas
    }
}

// ----- Text Style -----
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextEffect {
    outline_color: [f32; 4],
    glow_color: [f32; 4],
    outline_width: f32,
    glow_width: f32,
    underline_thickness: f32,
    strike_thickness: f32,
    highlight_pad: f32,
    glyph_wave_amp: f32,
    glyph_wave_freq: f32,
    glyph_wave_speed: f32,
    style_wave_amp: f32,
    style_wave_freq: f32,
    style_wave_speed: f32,
    _pad: f32,
}

impl TextEffect {
    pub fn new(underline_thickness: f32, strike_thickness: f32) -> Self {
        Self {
            underline_thickness,
            strike_thickness,
            ..Default::default()
        }
    }

    pub fn from_style(mut self, style: TextStyle) -> Self {
        style.outline.map(|spec| self.with_outline(spec));
        style.glow.map(|spec| self.with_glow(spec));
        style.glyph_wave.map(|spec| self.with_glyph_wave(spec));
        style.style_wave.map(|spec| self.with_style_wave(spec));
        self
    }

    pub fn with_outline(&mut self, spec: OutlineSpec) {
        self.outline_color = spec.color;
        self.outline_width = spec.width_px;
    }

    pub fn with_glow(&mut self, spec: GlowSpec) {
        self.glow_color = spec.color;
        self.glow_width = spec.width_px;
    }

    pub fn with_glyph_wave(&mut self, spec: WaveSpec) {
        self.glyph_wave_amp = spec.amplitude_n;
        self.glyph_wave_freq = spec.frequency;
        self.glyph_wave_speed = spec.speed;
    }

    pub fn with_style_wave(&mut self, spec: WaveSpec) {
        self.style_wave_amp = spec.amplitude_n;
        self.style_wave_freq = spec.frequency;
        self.style_wave_speed = spec.speed;
    }

    pub fn with_highlight_pad(&mut self, pad: f32) {
        self.highlight_pad = pad;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextStyle {
    color: [f32; 4],
    effects: u32,
    outline: Option<OutlineSpec>,
    glow: Option<GlowSpec>,
    glyph_wave: Option<WaveSpec>,
    style_wave: Option<WaveSpec>,
    highlight_pad: Option<f32>,
    // underline, strikethough
    // ... future: bold, italic, gradient
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_fill(mut self, color: Option<[f32; 4]>) -> Self {
        if color.is_some() {
            self.effects |= EFFECT_FILL;
        } else {
            self.effects &= !EFFECT_FILL;
        }
        self.color = color.unwrap_or_default();
        self
    }

    pub fn with_outline(mut self, outline: Option<OutlineSpec>) -> Self {
        if outline.is_some() {
            self.effects |= EFFECT_OUTLINE;
        } else {
            self.effects &= !EFFECT_OUTLINE;
        }
        self.outline = outline;
        self
    }

    pub fn with_glow(mut self, glow: Option<GlowSpec>) -> Self {
        if glow.is_some() {
            self.effects |= EFFECT_GLOW;
        } else {
            self.effects &= !EFFECT_GLOW;
        }
        self.glow = glow;
        self
    }

    pub fn with_glyph_wave(mut self, wave: Option<WaveSpec>) -> Self {
        if wave.is_some() {
            self.effects |= EFFECT_WAVE_GLYPH;
        } else {
            self.effects &= !EFFECT_WAVE_GLYPH;
        }
        self.glyph_wave = wave;
        self
    }

    pub fn with_style_wave(mut self, wave: Option<WaveSpec>) -> Self {
        if wave.is_some() {
            self.effects |= EFFECT_WAVE_STYLE;
        } else {
            self.effects &= !EFFECT_WAVE_STYLE;
        }
        self.style_wave = wave;
        self
    }

    pub fn with_highlight(mut self, pad: Option<f32>) -> Self {
        if pad.is_some() {
            self.effects |= EFFECT_HIGHLIGHT;
        } else {
            self.effects &= !EFFECT_HIGHLIGHT;
        }
        self.highlight_pad = pad;
        self
    }

    pub fn color(&self) -> [f32; 4] {
        self.color
    }

    pub fn effects(&self) -> u32 {
        self.effects
    }

    pub fn outline(&self) -> &Option<OutlineSpec> {
        &self.outline
    }

    pub fn glow(&self) -> &Option<GlowSpec> {
        &self.glow
    }

    pub fn glyph_wave(&self) -> &Option<WaveSpec> {
        &self.glyph_wave
    }

    pub fn style_wave(&self) -> &Option<WaveSpec> {
        &self.style_wave
    }

    pub fn highlight_pad(&self) -> Option<f32> {
        self.highlight_pad
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OutlineSpec {
    color: [f32; 4],
    width_px: f32,
}

impl OutlineSpec {
    pub fn new(color: [f32; 4], width_px: f32) -> Self {
        Self { color, width_px }
    }

    pub fn color(&self) -> [f32; 4] {
        self.color
    }

    pub fn width_px(&self) -> f32 {
        self.width_px
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GlowSpec {
    color: [f32; 4],
    width_px: f32,
}

impl GlowSpec {
    pub fn new(color: [f32; 4], width_px: f32) -> Self {
        Self { color, width_px }
    }

    pub fn color(&self) -> [f32; 4] {
        self.color
    }

    pub fn width_px(&self) -> f32 {
        self.width_px
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WaveSpec {
    /// in NDC units
    amplitude_n: f32,
    /// cycles per NDC unit
    frequency: f32,
    speed: f32,
}

impl WaveSpec {
    pub fn new(amplitude_n: f32, frequency: f32, speed: f32) -> Self {
        Self {
            amplitude_n,
            frequency,
            speed,
        }
    }

    pub fn amplitude_n(&self) -> f32 {
        self.amplitude_n
    }

    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }
}
