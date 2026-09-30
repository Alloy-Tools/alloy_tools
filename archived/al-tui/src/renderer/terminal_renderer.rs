use crate::Capabilities;
use al_ui::{Color, FlushMode, Modifier, Point, Renderer};
use crossterm::execute;

pub struct TerminalRenderer {
    dimensions: (u16, u16),
    flush_mode: FlushMode,
    cursor_visible: bool,
    cursor_pos: Point<<TerminalRenderer as Renderer>::Coord, 2>,
    capabilities: Capabilities,
    active_modifiers: Modifier,
    active_fg: Color,
    active_bg: Color,
}

impl Drop for TerminalRenderer {
    fn drop(&mut self) {
        // Restore terminal state
        let _ = execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

impl TerminalRenderer {
    pub fn new() -> Result<Self, std::io::Error> {
        let dimensions = crossterm::terminal::size()?;

        // init crossterm
        crossterm::terminal::enable_raw_mode()?;
        execute!(
            std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::cursor::Hide,
        )?;

        Ok(Self {
            dimensions,
            flush_mode: FlushMode::DirtyRectOnly,
            cursor_visible: false,
            cursor_pos: Point::new_2d(0, 0),
            capabilities: Capabilities::detect(),
            active_modifiers: Modifier::new(),
            active_fg: Color::Default,
            active_bg: Color::Default,
        })
    }

    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    pub fn color_to_crossterm(color: Color) -> crossterm::style::Color {
        use crossterm::style::Color as CtColor;
        match color {
            Color::Default => CtColor::Reset,
            Color::Rgb(r, g, b) => CtColor::Rgb { r, g, b },
            Color::Index256(idx) => CtColor::AnsiValue(idx),
            Color::Ansi(ansi_color) => match ansi_color {
                al_ui::AnsiColor::Black => CtColor::Black,
                al_ui::AnsiColor::Red => CtColor::DarkRed,
                al_ui::AnsiColor::Green => CtColor::DarkGreen,
                al_ui::AnsiColor::Yellow => CtColor::DarkYellow,
                al_ui::AnsiColor::Blue => CtColor::DarkBlue,
                al_ui::AnsiColor::Magenta => CtColor::DarkMagenta,
                al_ui::AnsiColor::Cyan => CtColor::DarkCyan,
                al_ui::AnsiColor::White => CtColor::White,
                al_ui::AnsiColor::Gray => CtColor::Grey,
                al_ui::AnsiColor::BrightRed => CtColor::Red,
                al_ui::AnsiColor::BrightGreen => CtColor::Green,
                al_ui::AnsiColor::BrightYellow => CtColor::Yellow,
                al_ui::AnsiColor::BrightBlue => CtColor::Blue,
                al_ui::AnsiColor::BrightMagenta => CtColor::Magenta,
                al_ui::AnsiColor::BrightCyan => CtColor::Cyan,
                al_ui::AnsiColor::BrightWhite => CtColor::White,
            },
        }
    }

    pub fn style_to_crossterm(
        &self,
        style: &al_ui::Style,
    ) -> (crossterm::style::Color, crossterm::style::Color) {
        (
            Self::color_to_crossterm(style.fg),
            Self::color_to_crossterm(style.bg),
        )
    }

    pub fn wait_for_input(&mut self) -> Result<(), std::io::Error> {
        use crossterm::event::{self, Event};

        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(_) = event::read()? {
                    break;
                }
            }
        }
        Ok(())
    }
}

impl Renderer for TerminalRenderer {
    type Coord = u16;

    fn draw_glyph(
        &mut self,
        pos: al_ui::Point<Self::Coord, 2>,
        clip: al_ui::Rect<Self::Coord>,
        glyph_id: al_ui::GlyphId,
        style: al_ui::Style,
    ) -> Result<(), al_ui::RendererError> {
        if let Some(ch) = char::from_u32(glyph_id) {
            let mut stdout = std::io::stdout();
            // move cursor to (x, y)
            execute!(stdout, crossterm::cursor::MoveTo(pos.x(), pos.y()))
                .map_err(|e| e.to_string())?;

            // apply style
            if style.modifiers.bold {
                if !self.active_modifiers.bold {
                    execute!(
                        stdout,
                        crossterm::style::SetAttribute(crossterm::style::Attribute::Bold)
                    )
                    .map_err(|e| e.to_string())?;
                    self.active_modifiers.set_bold(true);
                }
            } else {
                if self.active_modifiers.bold {
                    execute!(
                        stdout,
                        crossterm::style::SetAttribute(crossterm::style::Attribute::NoBold)
                    )
                    .map_err(|e| e.to_string())?;
                    self.active_modifiers.set_bold(false);
                }
            }
            if style.modifiers.underline {
                if !self.active_modifiers.underline {
                    execute!(
                        stdout,
                        crossterm::style::SetAttribute(crossterm::style::Attribute::Underlined)
                    )
                    .map_err(|e| e.to_string())?;
                    self.active_modifiers.set_underline(true);
                }
            } else {
                if self.active_modifiers.underline {
                    execute!(
                        stdout,
                        crossterm::style::SetAttribute(crossterm::style::Attribute::NoUnderline)
                    )
                    .map_err(|e| e.to_string())?;
                    self.active_modifiers.set_underline(false);
                }
            }
            if style.modifiers.inverse {
                if !self.active_modifiers.inverse {
                    execute!(
                        stdout,
                        crossterm::style::SetAttribute(crossterm::style::Attribute::Reverse)
                    )
                    .map_err(|e| e.to_string())?;
                    self.active_modifiers.set_inverse(true);
                }
            } else {
                if self.active_modifiers.inverse {
                    execute!(
                        stdout,
                        crossterm::style::SetAttribute(crossterm::style::Attribute::NoReverse)
                    )
                    .map_err(|e| e.to_string())?;
                    self.active_modifiers.set_inverse(false);
                }
            }
            if style.modifiers.italic {
                if !self.active_modifiers.italic {
                    execute!(
                        stdout,
                        crossterm::style::SetAttribute(crossterm::style::Attribute::Italic)
                    )
                    .map_err(|e| e.to_string())?;
                    self.active_modifiers.set_italic(true);
                }
            } else {
                if self.active_modifiers.italic {
                    execute!(
                        stdout,
                        crossterm::style::SetAttribute(crossterm::style::Attribute::NoItalic)
                    )
                    .map_err(|e| e.to_string())?;
                    self.active_modifiers.set_italic(false);
                }
            }

            // set color

            // write

            // flush ?

            Ok(())
        } else {
            // Custom glyphs could be drawn as '?', ignored, or handled via escape codes.
            todo!()
        }
    }

    fn draw_glyphs(
        &mut self,
        pos: al_ui::Point<Self::Coord, 2>,
        clip: al_ui::Rect<Self::Coord>,
        glyph_ids: &[al_ui::GlyphId],
        style: al_ui::Style,
    ) -> Result<(), al_ui::RendererError> {
        todo!()
    }

    fn draw_text(
        &mut self,
        pos: al_ui::Point<Self::Coord, 2>,
        text: &str,
        style: al_ui::Style,
        clip: al_ui::Rect<Self::Coord>,
    ) -> Result<(), al_ui::RendererError> {
        todo!()
    }

    fn get_glyph_metrics(
        &self,
        glyph_id: al_ui::GlyphId,
    ) -> Option<al_ui::GlyphMetrics<Self::Coord>> {
        todo!()
    }

    fn draw_rect(
        &mut self,
        rect: al_ui::Rect<Self::Coord>,
        clip: al_ui::Rect<Self::Coord>,
        color: al_ui::Color,
        border_color: Option<al_ui::Color>,
    ) -> Result<(), al_ui::RendererError> {
        todo!()
    }

    fn dimensions(&self) -> Result<(Self::Coord, Self::Coord), al_ui::RendererError> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), al_ui::RendererError> {
        todo!()
    }

    fn set_flush_mode(&mut self, mode: al_ui::FlushMode) {
        todo!()
    }

    fn get_flush_mode(&self) -> al_ui::FlushMode {
        todo!()
    }

    fn show_cursor(&mut self, x: Self::Coord, y: Self::Coord) -> Result<(), al_ui::RendererError> {
        todo!()
    }

    fn hide_cursor(&mut self) -> Result<(), al_ui::RendererError> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), al_ui::RendererError> {
        todo!()
    }

    fn mark_all_dirty(&mut self) {
        todo!()
    }
}
