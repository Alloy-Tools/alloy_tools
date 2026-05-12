use serde::{Deserialize, Serialize};

/// Text styling (bold, italic, underline, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Modifier {
    pub bold: bool,
    pub italic: bool,
    pub inverse: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl Modifier {
    /// Create a new modifier with all flags off
    pub fn new() -> Self {
        Self {
            bold: false,
            italic: false,
            inverse: false,
            underline: false,
            strikethrough: false,
        }
    }

    /// Builder: set bold
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    pub fn set_bold(&mut self, value: bool) {
        self.bold = value;
    }
    
    pub fn toggle_bold(&mut self) {
        self.bold = !self.bold;
    }

    /// Builder: set italic
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    pub fn set_italic(&mut self, value: bool) {
        self.italic = value;
    }
    
    pub fn toggle_italic(&mut self) {
        self.italic = !self.italic;
    }

    /// Builder: set underline
    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    pub fn set_underline(&mut self, value: bool) {
        self.underline = value;
    }
    
    pub fn toggle_underline(&mut self) {
        self.underline = !self.underline;
    }

    /// Builder: set inverse (swap foreground and background)
    pub fn inverse(mut self, inverse: bool) -> Self {
        self.inverse = inverse;
        self
    }

    pub fn set_inverse(&mut self, value: bool) {
        self.inverse = value;
    }
    
    pub fn toggle_inverse(&mut self) {
        self.inverse = !self.inverse;
    }

    /// Builder: set strikethrough
    pub fn strikethrough(mut self, strikethrough: bool) -> Self {
        self.strikethrough = strikethrough;
        self
    }

    pub fn set_strikethrough(&mut self, value: bool) {
        self.strikethrough = value;
    }
    
    pub fn toggle_strikethrough(&mut self) {
        self.strikethrough = !self.strikethrough;
    }
}

impl Default for Modifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Modifier;

    #[test]
    fn default() {
        let m = Modifier::default();
        assert!(!m.bold);
        assert!(!m.italic);
        assert!(!m.inverse);
        assert!(!m.underline);
        assert!(!m.strikethrough);
        assert_eq!(m, Modifier::new());
    }

    #[test]
    fn toggle() {
        let mut m = Modifier::new();
        assert!(!m.bold);
        assert!(!m.italic);
        assert!(!m.inverse);
        assert!(!m.underline);
        assert!(!m.strikethrough);

        m.toggle_bold();
        m.toggle_italic();
        m.toggle_inverse();
        m.toggle_underline();
        m.toggle_strikethrough();

        assert!(m.bold);
        assert!(m.italic);
        assert!(m.inverse);
        assert!(m.underline);
        assert!(m.strikethrough);
    }

    #[test]
    fn equality() {
        assert_eq!(Modifier::new().bold(true).italic(true), Modifier::new().bold(true).italic(true));
        assert_ne!(Modifier::new().bold(true).italic(true), Modifier::new().bold(false).italic(true));
    }
}
