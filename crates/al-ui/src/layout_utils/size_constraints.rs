use crate::CoordType;

/// Some(max) if constrained, None if unbounded
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeConstraints<Coord: CoordType> {
    pub width: Option<Coord>,
    pub height: Option<Coord>,
}

impl<Coord: CoordType> SizeConstraints<Coord> {
    pub fn new(width: Coord, height: Coord) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
        }
    }

    pub fn unbounded() -> Self {
        Self {
            width: None,
            height: None,
        }
    }

    pub fn width(width: Coord) -> Self {
        Self {
            width: Some(width),
            height: None,
        }
    }

    pub fn height(height: Coord) -> Self {
        Self {
            width: None,
            height: Some(height),
        }
    }

    pub fn clip(&self, width: Coord, height: Coord) -> (Coord, Coord) {
        let w = self.width.unwrap_or(width);
        let h = self.height.unwrap_or(height);
        (
            if width <= w { width } else { w },
            if height <= h { height } else { h },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let constraints = SizeConstraints::new(100, 50);
        assert_eq!(constraints.width, Some(100));
        assert_eq!(constraints.height, Some(50));
        
        let constraints = SizeConstraints::<u8>::unbounded();
        assert_eq!(constraints.width, None);
        assert_eq!(constraints.height, None);

        let constraints = SizeConstraints::width(80);
        assert_eq!(constraints.width, Some(80));
        assert_eq!(constraints.height, None);

        let constraints = SizeConstraints::height(24);
        assert_eq!(constraints.width, None);
        assert_eq!(constraints.height, Some(24));
    }

    #[test]
    fn clipping() {
        let (w, h) = SizeConstraints::new(100, 50).clip(100, 50);
        assert_eq!(w, 100);
        assert_eq!(h, 50);

        let (w, h) = SizeConstraints::new(100, 50).clip(50, 25);
        assert_eq!(w, 50);
        assert_eq!(h, 25);

        let (w, h) = SizeConstraints::new(100, 50).clip(150, 75);
        assert_eq!(w, 100);
        assert_eq!(h, 50);

        let (w, h) = SizeConstraints::width(80).clip(100, 24);
        assert_eq!(w, 80);
        assert_eq!(h, 24);

        let (w, h) = SizeConstraints::height(30).clip(100, 50);
        assert_eq!(w, 100);
        assert_eq!(h, 30);

        let (w, h) = SizeConstraints::unbounded().clip(100, 50);
        assert_eq!(w, 100);
        assert_eq!(h, 50);
    }

    #[test]
    fn equality() {
        assert_eq!(SizeConstraints::new(100, 50), SizeConstraints::new(100, 50));
        assert_ne!(SizeConstraints::new(100, 50), SizeConstraints::new(100, 51));
        assert_ne!(SizeConstraints::new(100, 50), SizeConstraints::new(101, 50));
    }
}
