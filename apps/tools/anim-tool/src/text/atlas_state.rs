use crate::text::{
    atlas_generator::{build_atlas, save_atlas_png, AtlasGenConfig},
    common::{
        Align, AtlasHandle, FontAtlas, FontEntry, FontError, FontHandle, FontSpec, LineHeight,
        ShapeOptions, ShapedText, TextEffect, TextQuad, WrapMode,
    },
};

pub struct AtlasState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    atlases: Vec<Option<FontAtlas>>,
}

impl AtlasState {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            device,
            queue,
            atlases: Vec::new(),
        }
    }

    pub fn build_atlas(
        &mut self,
        atlas_gen_config: AtlasGenConfig,
        specs: Vec<FontSpec>,
        quads_buffer: &wgpu::Buffer,
        effects_buffer: &wgpu::Buffer,
        text_bgl: &wgpu::BindGroupLayout,
    ) -> Result<(AtlasHandle, Vec<FontHandle>), FontError> {
        let atlas = build_atlas(
            &self.device,
            &self.queue,
            atlas_gen_config,
            specs,
            quads_buffer,
            effects_buffer,
            text_bgl,
        )?;
        let atlas_handle = AtlasHandle::new(self.atlases.len() as u32);
        let font_handles = (0..atlas.fonts().len())
            .map(|i| FontHandle::new(atlas_handle, i as u32))
            .collect::<Vec<_>>();
        self.atlases.push(Some(atlas));
        Ok((atlas_handle, font_handles))
    }

    //REVIEW: Should save the atlas data as well, then add loading with `png + data`
    pub fn save_atlas(&self, atlas: AtlasHandle) -> Result<(), Box<dyn std::error::Error>> {
        let (size, texture) = self
            .atlases
            .get(atlas.id() as usize)
            .and_then(|atlas| {
                atlas
                    .as_ref()
                    .and_then(|atlas| Some((atlas.size(), atlas.texture())))
            })
            .ok_or("No atlas for the passed `AtlasHandle`")?;
        let atlas_name = format!(
            "atlas_{}_{}.png",
            atlas.id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        save_atlas_png(
            &self.device,
            &self.queue,
            texture,
            size,
            size,
            std::path::Path::new(&atlas_name),
        )?;
        eprintln!("Saved atlas PNG as {atlas_name}");
        Ok(())
    }

    pub fn erase_atlas(&mut self, handle: AtlasHandle) {
        if let Some(slot) = self.atlases.get_mut(handle.id() as usize) {
            *slot = None;
        }
    }

    pub fn get(&self, handle: AtlasHandle) -> Option<&FontAtlas> {
        self.atlases
            .get(handle.id() as usize)
            .and_then(|s| s.as_ref())
    }

    pub fn font_handle(&self, name: &str) -> Option<FontHandle> {
        self.atlases
            .iter()
            .enumerate()
            .find_map(|(atlas_pos, atlas_op)| {
                atlas_op.as_ref().and_then(|atlas| {
                    atlas
                        .fonts()
                        .iter()
                        .position(|font| font.spec().name() == name)
                        .map(|font_pos| {
                            FontHandle::new(AtlasHandle::new(atlas_pos as u32), font_pos as u32)
                        })
                })
            })
    }

    pub fn font(&self, handle: FontHandle) -> Result<&FontEntry, FontError> {
        self.get(handle.atlas())
            .and_then(|atlas| atlas.fonts().get(handle.font() as usize))
            .ok_or(FontError::InvalidHandle)
    }

    pub fn set_atlas_time(&self, time: f32) {
        for slot in &self.atlases {
            let Some(atlas) = slot.as_ref() else { continue };
            self.queue
                .write_buffer(atlas.globals_buffer(), 12, bytemuck::bytes_of(&time));
        }
    }

    pub fn shape(
        &self,
        text: &str,
        font: FontHandle,
        options: ShapeOptions,
    ) -> Result<ShapedText, FontError> {
        match options.wrap() {
            WrapMode::PerChar => self.shape_wrap_char(text, font, options),
            WrapMode::PerWord => self.shape_wrap_word(text, font, options),
        }
    }

    fn shape_wrap_char(
        &self,
        text: &str,
        font: FontHandle,
        options: ShapeOptions,
    ) -> Result<ShapedText, FontError> {
        let entry = self.font(font)?;
        let glyphs = entry.glyphs();
        let metrics = &entry.metrics();
        let scale = options.size_ndc();
        let ascent_n = metrics.ascent_n();
        let line_h = match options.line_height() {
            LineHeight::Metrics => (ascent_n + metrics.descent_n() + metrics.line_gap_n()) * scale,
            LineHeight::Multiplier(k) => scale * k,
            LineHeight::Fixed(v) => v,
        };
        let em_size = metrics.em_size();
        let effect = TextEffect::new(
            metrics.underline_size_n() * scale,
            metrics.strikethrough_size_n() * scale,
        );
        let mw = options.max_width().unwrap_or(f32::INFINITY);
        let baseline_offset = (ascent_n * scale) + options.baseline_offset();
        let midline_offset =
            (baseline_offset - metrics.cap_height_n() * scale * 0.5) + options.midline_offset();

        let mut quads = Vec::new();
        let mut effects = Vec::new();
        let mut cursor_x = 0.;
        let mut cursor_y = 0.;
        let mut max_x = 0.0f32;
        let mut line_count = 1;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = 0.;
                cursor_y += line_h;
                line_count += 1;
                continue;
            }

            let Some(info) = glyphs.get(&ch) else {
                continue;
            };
            let advance_ndc = (info.advance / em_size) * scale;

            if cursor_x + advance_ndc > mw && cursor_x > 0. {
                cursor_x = 0.;
                cursor_y += line_h;
                line_count += 1;
            }

            if info.quad_size[0] > 0. && info.quad_size[1] > 0. {
                let bx = (info.bearing[0] / em_size) * scale;
                let by = (info.bearing[1] / em_size) * scale;
                let qw = (info.quad_size[0] / em_size) * scale;
                let qh = (info.quad_size[1] / em_size) * scale;

                let min_x = cursor_x + bx;
                let min_y = cursor_y + by;

                quads.push(TextQuad::new(
                    [min_x, min_y],
                    [min_x + qw, min_y + qh],
                    info.uv_min,
                    info.uv_max,
                    [1.; 4],
                    em_size,
                    cursor_y + baseline_offset,
                    cursor_y + midline_offset,
                    0,
                ));
                effects.push(effect);
            }

            cursor_x += advance_ndc;
            max_x = max_x.max(cursor_x);
        }

        match options.align() {
            Align::Left => {}
            Align::Center => {
                let dx = max_x * 0.5;
                for q in &mut quads {
                    q.min_mut(|min| min[0] -= dx);
                    q.max_mut(|max| max[0] -= dx);
                }
            }
            Align::Right => {
                for q in &mut quads {
                    q.min_mut(|min| min[0] -= max_x);
                    q.max_mut(|max| max[0] -= max_x);
                }
            }
        }

        Ok(ShapedText::new(
            quads,
            effects,
            [0., 0., max_x, line_count as f32 * line_h],
            line_count,
            font.atlas(),
        ))
    }

    fn shape_wrap_word(
        &self,
        text: &str,
        font: FontHandle,
        options: ShapeOptions,
    ) -> Result<ShapedText, FontError> {
        let entry = self.font(font)?;
        let glyphs = entry.glyphs();
        let metrics = &entry.metrics();
        let scale = options.size_ndc();
        let ascent_n = metrics.ascent_n();
        let line_h = match options.line_height() {
            LineHeight::Metrics => (ascent_n + metrics.descent_n() + metrics.line_gap_n()) * scale,
            LineHeight::Multiplier(k) => scale * k,
            LineHeight::Fixed(v) => v,
        };
        let em_size = metrics.em_size();
        let effect = TextEffect::new(
            metrics.underline_size_n() * scale,
            metrics.strikethrough_size_n() * scale,
        );
        let mw = options.max_width().unwrap_or(f32::INFINITY);
        let baseline_offset = (ascent_n * scale) + options.baseline_offset();
        let midline_offset =
            (baseline_offset - metrics.cap_height_n() * scale * 0.5) + options.midline_offset();

        let mut quads = Vec::new();
        let mut effects = Vec::new();
        let mut cursor_x = 0.;
        let mut cursor_y = 0.;
        let mut max_x = 0.0f32;
        let mut line_count = 1;

        let mut words: Vec<&str> = Vec::new();
        let mut start = 0;
        // TODO: Should this use `chars()` rather than `as_bytes()`?
        let bytes = text.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b' ' || b == b'\n' {
                words.push(&text[start..=i]);
                start = i + 1;
            }
        }
        if start < text.len() {
            words.push(&text[start..]);
        }

        for word in words {
            let mut word_w = 0.;
            for ch in word.chars() {
                if ch == '\n' {
                    continue;
                }
                if let Some(info) = glyphs.get(&ch) {
                    word_w += (info.advance / em_size) * scale;
                }
            }

            let has_newline = word.ends_with('\n');
            if cursor_x + word_w > mw && cursor_x > 0. {
                cursor_x = 0.;
                cursor_y += line_h;
                line_count += 1;
            }

            for ch in word.chars() {
                if ch == '\n' {
                    continue;
                }
                let Some(info) = glyphs.get(&ch) else {
                    continue;
                };
                let advance_ndc = (info.advance / em_size) * scale;

                if info.quad_size[0] > 0. && info.quad_size[1] > 0. {
                    let bx = (info.bearing[0] / em_size) * scale;
                    let by = (info.bearing[1] / em_size) * scale;
                    let qw = (info.quad_size[0] / em_size) * scale;
                    let qh = (info.quad_size[1] / em_size) * scale;

                    let min_x = cursor_x + bx;
                    let min_y = cursor_y + by;

                    let q = TextQuad::new(
                        [min_x, min_y],
                        [min_x + qw, min_y + qh],
                        info.uv_min,
                        info.uv_max,
                        [1.; 4],
                        em_size,
                        cursor_y + baseline_offset,
                        cursor_y + midline_offset,
                        0,
                    );
                    quads.push(q);
                    effects.push(effect);
                }
                cursor_x += advance_ndc;
                max_x = max_x.max(cursor_x);
            }

            if has_newline {
                cursor_x = 0.;
                cursor_y += line_h;
                line_count += 1;
            }
        }

        match options.align() {
            Align::Left => {}
            Align::Center => {
                let dx = max_x * 0.5;
                for q in &mut quads {
                    q.min_mut(|min| min[0] -= dx);
                    q.max_mut(|max| max[0] -= dx);
                }
            }
            Align::Right => {
                for q in &mut quads {
                    q.min_mut(|min| min[0] -= max_x);
                    q.max_mut(|max| max[0] -= max_x);
                }
            }
        }

        Ok(ShapedText::new(
            quads,
            effects,
            [0., 0., max_x, line_count as f32 * line_h],
            line_count,
            font.atlas(),
        ))
    }
}
