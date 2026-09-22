use crate::{Color, Modifier};
use serde::{Deserialize, Serialize};

/// Complete text styling (colors + modifiers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub modifiers: Modifier,
}

impl Style {
    /// Create a new style with default colors and no modifiers
    pub fn new() -> Self {
        Self {
            fg: Color::Default,
            bg: Color::Default,
            modifiers: Modifier::new(),
        }
    }

    /// set foreground color
    pub fn fg(mut self, color: impl Into<Color>) -> Self {
        self.fg = color.into();
        self
    }

    /// set background color
    pub fn bg(mut self, color: impl Into<Color>) -> Self {
        self.bg = color.into();
        self
    }

    /// set modifiers
    pub fn modifiers(mut self, modifiers: Modifier) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// add bold
    pub fn bold(mut self, bold: bool) -> Self {
        self.modifiers = self.modifiers.bold(bold);
        self
    }

    /// set underline
    pub fn underline(mut self, underline: bool) -> Self {
        self.modifiers = self.modifiers.underline(underline);
        self
    }

    /// set inverse (swap fg/bg)
    pub fn inverse(mut self, inverse: bool) -> Self {
        self.modifiers = self.modifiers.inverse(inverse);
        self
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{AnsiColor, Color, Modifier, Style};

    #[test]
    fn default() {
        let style = Style::new();
        assert_eq!(style, Style::default());
        assert_eq!(style.fg, Color::Default);
        assert_eq!(style.bg, Color::Default);
        assert_eq!(style.modifiers, Modifier::new());
    }

    #[test]
    fn color() {
        // fg
        let style = Style::new().fg(AnsiColor::Red);
        assert_eq!(style.fg, Color::Ansi(AnsiColor::Red));

        let style = Style::new().fg((255u8, 128u8, 64u8));
        assert_eq!(style.fg, Color::Rgb(255, 128, 64));

        let style = Style::new().fg(200u8);
        assert_eq!(style.fg, Color::Index256(200));

        let style = Style::new().fg(AnsiColor::Red).fg(AnsiColor::Blue);
        assert_eq!(style.fg, Color::Ansi(AnsiColor::Blue));

        // bg
        let style = Style::new().bg(AnsiColor::Green);
        assert_eq!(style.bg, Color::Ansi(AnsiColor::Green));

        let style = Style::new().bg((100u8, 150u8, 200u8));
        assert_eq!(style.bg, Color::Rgb(100, 150, 200));

        let style = Style::new().bg(50u8);
        assert_eq!(style.bg, Color::Index256(50));

        let style = Style::new().bg(AnsiColor::Yellow).bg(AnsiColor::Cyan);
        assert_eq!(style.bg, Color::Ansi(AnsiColor::Cyan));
    }

    #[test]
    fn complex_builder() {
        let style = Style::new()
            .fg(AnsiColor::Blue)
            .bg((100u8, 100u8, 100u8))
            .bold(true)
            .underline(true)
            .fg(AnsiColor::White);
        assert_eq!(style.fg, Color::Ansi(AnsiColor::White));
        assert_eq!(style.bg, Color::Rgb(100, 100, 100));
        assert!(style.modifiers.bold);
        assert!(style.modifiers.underline);
        assert!(!style.modifiers.inverse);
    }

    #[test]
    fn set_modifiers() {
        let modifiers = Modifier::new().bold(true).underline(true);
        let style = Style::new().modifiers(modifiers);
        assert!(style.modifiers.bold);
        assert!(style.modifiers.underline);
    }

    #[test]
    fn equality() {
        assert_eq!(
            Style::new().fg(AnsiColor::Red).bold(true),
            Style::new().fg(AnsiColor::Red).bold(true)
        );
        assert_ne!(
            Style::new().fg(AnsiColor::Red).bold(true),
            Style::new().fg(AnsiColor::Red)
        );
    }
}
