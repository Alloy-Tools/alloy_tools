use serde::{Deserialize, Serialize};

/// Color representation supporting multiple backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    /// Use renderers default color
    Default,
    /// RGB true color (24-bit). Falls back to 256-color or 16-color if unsupported.
    Rgb(u8, u8, u8),
    /// 256-color palette index
    Index256(u8),
    /// 16 ANSI colors
    Ansi(AnsiColor),
}

impl From<(u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8)) -> Self {
        Color::Rgb(value.0, value.1, value.2)
    }
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        Color::Index256(value)
    }
}

impl From<AnsiColor> for Color {
    fn from(value: AnsiColor) -> Self {
        Color::Ansi(value)
    }
}

/// 16 ANSI color codes
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    #[default]
    White,
    Gray,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl AnsiColor {
    /// Get the ANSI index (0-15)
    pub fn index(self) -> u8 {
        match self {
            Self::Black => 0,
            Self::Red => 1,
            Self::Green => 2,
            Self::Yellow => 3,
            Self::Blue => 4,
            Self::Magenta => 5,
            Self::Cyan => 6,
            Self::White => 7,
            Self::Gray => 8,
            Self::BrightRed => 9,
            Self::BrightGreen => 10,
            Self::BrightYellow => 11,
            Self::BrightBlue => 12,
            Self::BrightMagenta => 13,
            Self::BrightCyan => 14,
            Self::BrightWhite => 15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from() {
        assert_eq!(Color::Rgb(100, 150, 200), (100, 150, 200).into());
        assert_eq!(Color::Rgb(0, 0, 0), (0, 0, 0).into());
        assert_eq!(Color::Rgb(255, 255, 255), (255, 255, 255).into());
        assert_eq!(Color::Index256(127), 127.into());
        assert_eq!(Color::Ansi(AnsiColor::Green), AnsiColor::Green.into());
    }

    #[test]
    fn equality() {
        assert_eq!(Color::Rgb(100, 100, 100), Color::Rgb(100, 100, 100));
        assert_ne!(Color::Rgb(100, 100, 100), Color::Rgb(100, 100, 101));
        assert_ne!(Color::Rgb(255, 0, 0), Color::Index256(1));
        assert_eq!(AnsiColor::Yellow, AnsiColor::Yellow);
        assert_ne!(AnsiColor::Yellow, AnsiColor::Cyan);
    }

    #[test]
    fn ansi_color() {
        assert_eq!(AnsiColor::Black.index(), 0);
        assert_eq!(AnsiColor::Red.index(), 1);
        assert_eq!(AnsiColor::Green.index(), 2);
        assert_eq!(AnsiColor::Yellow.index(), 3);
        assert_eq!(AnsiColor::Blue.index(), 4);
        assert_eq!(AnsiColor::Magenta.index(), 5);
        assert_eq!(AnsiColor::Cyan.index(), 6);
        assert_eq!(AnsiColor::White.index(), 7);
        assert_eq!(AnsiColor::Gray.index(), 8);
        assert_eq!(AnsiColor::BrightRed.index(), 9);
        assert_eq!(AnsiColor::BrightGreen.index(), 10);
        assert_eq!(AnsiColor::BrightYellow.index(), 11);
        assert_eq!(AnsiColor::BrightBlue.index(), 12);
        assert_eq!(AnsiColor::BrightMagenta.index(), 13);
        assert_eq!(AnsiColor::BrightCyan.index(), 14);
        assert_eq!(AnsiColor::BrightWhite.index(), 15);

        let color = AnsiColor::default();
        assert_eq!(color, AnsiColor::White);
    }
}
