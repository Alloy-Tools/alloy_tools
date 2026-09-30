use num_traits::zero;

use crate::CoordType;

/// Position and size for a component or window
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(bound = "Coord: for<'des> serde::Deserialize<'des>")]
pub struct Rect<Coord: CoordType> {
    /// X coordinate (column)
    pub x: Coord,
    /// Y coordinate (row)
    pub y: Coord,
    /// Width in columns
    pub width: Coord,
    /// Height in rows
    pub height: Coord,
}

impl<Coord: CoordType> Default for Rect<Coord> {
    fn default() -> Self {
        Self::new(Coord::zero(), Coord::zero(), Coord::zero(), Coord::zero())
    }
}

impl<Coord: CoordType> Rect<Coord> {
    /// Create a new absolute rectangle
    pub fn new(x: Coord, y: Coord, width: Coord, height: Coord) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Get the area of the `Rect` as f64
    pub fn area(&self) -> f64 {
        Coord::to_f64(&self.width).unwrap_or(0.) * Coord::to_f64(&self.height).unwrap_or(0.)
    }

    /// Get right edge (x + width - 1)
    pub fn right(&self) -> Coord {
        self.x
            .saturating_add((self.width.saturating_sub(Coord::one())).max(Coord::zero()))
    }

    /// Get bottom edge (y + height - 1)
    pub fn bottom(&self) -> Coord {
        self.y
            .saturating_add((self.height.saturating_sub(Coord::one())).max(Coord::zero()))
    }

    /// Check if point (px, py) is inside this rectangle
    pub fn contains(&self, pos: impl Into<crate::Point<Coord, 2>>) -> bool {
        if self.width == Coord::zero() || self.height == Coord::zero() {
            return false;
        }
        let pos = pos.into();
        let x = pos.x();
        let y = pos.y();
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }

    /// Remove parent offset
    pub fn relative(&self, parent: Rect<Coord>) -> Rect<Coord> {
        Rect::new(
            self.x - parent.x,
            self.y - parent.y,
            self.width,
            self.height,
        )
    }

    /// Apply parent offset
    pub fn absolute(&self, parent: Rect<Coord>) -> Rect<Coord> {
        Rect::new(
            parent.x + self.x,
            parent.y + self.y,
            self.width,
            self.height,
        )
    }

    /// Check if this rect intersects with another, assumes both rects share a coordinate space
    pub fn intersects(&self, other: Rect<Coord>) -> bool {
        if self.width == Coord::zero()
            || self.height == Coord::zero()
            || other.width == Coord::zero()
            || other.height == Coord::zero()
        {
            return false;
        }
        self.x <= other.right()
            && self.right() >= other.x
            && self.y <= other.bottom()
            && self.bottom() >= other.y
    }

    /// Calculate intersection of two rectangles, assumes both rects share the same coordinate space
    pub fn intersection(&self, other: Rect<Coord>) -> Rect<Coord> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if right > x && bottom > y {
            Rect::new(x, y, right - x + Coord::one(), bottom - y + Coord::one())
        } else {
            Rect::new(zero(), zero(), zero(), zero())
        }
    }

    /// Calculate union of two rectangles, assumes both rects share the same coordinate space
    pub fn union(&self, other: Rect<Coord>) -> Rect<Coord> {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(x, y, right - x + Coord::one(), bottom - y + Coord::one())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_dimensions() {
        let rect = Rect::new(0, 0, 0, 0);
        assert_eq!(rect.width, 0);
        assert_eq!(rect.height, 0);
        assert_eq!(rect.area(), 0.);
        assert!(!rect.contains((0, 0)));
    }

    #[test]
    fn max_coordinates() {
        let rect = Rect::new(u16::MAX, u16::MAX, u16::MAX, u16::MAX);
        assert_eq!(rect.x, u16::MAX);
        assert_eq!(rect.y, u16::MAX);
        assert_eq!(rect.width, u16::MAX);
        assert_eq!(rect.height, u16::MAX);
    }

    #[test]
    fn edges() {
        // right edge
        let rect = Rect::new(5, 10, 20, 30);
        assert_eq!(rect.right(), 24);
        assert!(rect.contains((24, 10)));

        // bottom edge
        assert_eq!(rect.bottom(), 39);
        assert!(rect.contains((5, 39)));
    }

    #[test]
    fn contains_point() {
        // inside
        let rect = Rect::new(5, 5, 10, 10);
        assert!(rect.contains((8, 8)));
        assert!(rect.contains((10, 10)));

        // left edge
        assert!(!rect.contains((4, 8)));
        assert!(rect.contains((5, 8)));

        // top left
        assert!(!rect.contains((5, 4)));
        assert!(!rect.contains((4, 5)));
        assert!(rect.contains((5, 5)));

        // right edge
        assert!(rect.contains((14, 8)));
        assert!(!rect.contains((15, 8)));

        // top right
        assert!(!rect.contains((15, 5)));
        assert!(!rect.contains((14, 4)));
        assert!(rect.contains((14, 5)));

        // top edge
        assert!(!rect.contains((8, 4)));
        assert!(rect.contains((8, 5)));

        // bottom edge
        assert!(rect.contains((8, 14)));
        assert!(!rect.contains((8, 15)));

        // bottom left
        assert!(!rect.contains((4, 14)));
        assert!(!rect.contains((5, 15)));
        assert!(rect.contains((5, 14)));

        // bottom right
        assert!(!rect.contains((15, 14)));
        assert!(!rect.contains((14, 15)));
        assert!(rect.contains((14, 14)));
    }

    #[test]
    fn intersects() {
        // overlapping
        let rect_a = Rect::new(0, 0, 10, 10);
        let rect_b = Rect::new(5, 5, 10, 10);
        assert!(rect_a.intersects(rect_b));
        assert!(rect_b.intersects(rect_a));

        // edge to edge
        let rect_b = Rect::new(10, 0, 10, 10);
        assert!(!rect_a.intersects(rect_b));
        assert!(!rect_b.intersects(rect_a));

        // corner to corner
        let rect_b = Rect::new(10, 10, 10, 10);
        assert!(!rect_a.intersects(rect_b));
        assert!(!rect_b.intersects(rect_a));

        // inside
        let rect_b = Rect::new(5, 5, 2, 2);
        assert!(rect_a.intersects(rect_b));
        assert!(rect_b.intersects(rect_a));

        // horizontal overlap
        let rect_a = Rect::new(10, 10, 20, 20);
        let rect_b = Rect::new(0, 10, 15, 20);
        assert!(rect_a.intersects(rect_b));
        assert!(rect_b.intersects(rect_a));

        // vertical overlap
        let rect_b = Rect::new(10, 0, 20, 15);
        assert!(rect_a.intersects(rect_b));
        assert!(rect_b.intersects(rect_a));

        // identical
        assert!(Rect::new(5, 5, 10, 10).intersects(Rect::new(5, 5, 10, 10)));

        // zero area
        let rect_a = Rect::new(5, 5, 0, 0);
        let rect_b = Rect::new(5, 5, 10, 10);
        assert!(!rect_a.intersects(rect_b));
        assert!(!rect_b.intersects(rect_a));
    }

    #[test]
    fn intersection_commutative() {
        let rect_a = Rect::new(0, 0, 10, 10);
        let rect_b = Rect::new(5, 5, 10, 10);
        assert_eq!(rect_a.intersection(rect_b), rect_b.intersection(rect_a));
    }

    #[test]
    fn intersection() {
        // overlapping
        let rect_a = Rect::new(0, 0, 10, 10);
        let rect_b = Rect::new(5, 5, 10, 10);
        let inter = rect_a.intersection(rect_b);
        assert_eq!(inter.x, 5);
        assert_eq!(inter.y, 5);
        assert_eq!(inter.width, 5);
        assert_eq!(inter.height, 5);
        assert_eq!(inter.area(), 25.);

        // not overlapping
        let rect_b = Rect::new(15, 15, 10, 10);
        let inter = rect_a.intersection(rect_b);
        assert_eq!(inter.x, 0);
        assert_eq!(inter.y, 0);
        assert_eq!(inter.width, 0);
        assert_eq!(inter.height, 0);
        assert_eq!(inter.area(), 0.);

        // inside
        let rect_b = Rect::new(5, 5, 2, 2);
        let inter = rect_a.intersection(rect_b);
        assert_eq!(inter.x, 5);
        assert_eq!(inter.y, 5);
        assert_eq!(inter.width, 2);
        assert_eq!(inter.height, 2);
        assert_eq!(inter.area(), 4.);

        // identical
        assert_eq!(
            Rect::new(5, 5, 10, 10).intersection(Rect::new(5, 5, 10, 10)),
            Rect::new(5, 5, 10, 10)
        );

        // edge to edge
        let rect_b = Rect::new(10, 0, 10, 10);
        let inter = rect_a.intersection(rect_b);
        assert_eq!(inter.width, 0);
        assert_eq!(inter.height, 0);

        // zero width
        let rect_b = Rect::new(5, 5, 0, 10);
        let inter = rect_a.intersection(rect_b);
        assert_eq!(inter.width, 0);
        assert_eq!(inter.height, 0);

        // zero height
        let rect_b = Rect::new(5, 5, 10, 0);
        let inter = rect_a.intersection(rect_b);
        assert_eq!(inter.width, 0);
        assert_eq!(inter.height, 0);
    }

    #[test]
    fn union_commutative() {
        let rect_a = Rect::new(0, 0, 10, 10);
        let rect_b = Rect::new(5, 5, 10, 10);
        let union_a_b = rect_a.union(rect_b);
        let union_b_a = rect_b.union(rect_a);
        assert_eq!(union_a_b, union_b_a);
    }

    #[test]
    fn union() {
        // overlapping
        let rect_a = Rect::new(0, 0, 10, 10);
        let rect_b = Rect::new(5, 5, 10, 10);
        let union = rect_a.union(rect_b);
        assert_eq!(union.x, 0);
        assert_eq!(union.y, 0);
        assert_eq!(union.width, 15);
        assert_eq!(union.height, 15);

        // non overlapping
        let rect_b = Rect::new(20, 20, 10, 10);
        let union = rect_a.union(rect_b);
        assert_eq!(union.x, 0);
        assert_eq!(union.y, 0);
        assert_eq!(union.width, 30);
        assert_eq!(union.height, 30);
        assert_eq!(union.right(), 29);
        assert_eq!(union.bottom(), 29);
        assert!(union.intersects(rect_a));
        assert!(union.intersects(rect_b));

        // inside
        let rect_b = Rect::new(5, 5, 2, 2);
        assert_eq!(rect_a.union(rect_b), rect_a);

        // identical
        assert_eq!(Rect::new(5, 5, 10, 10).union(Rect::new(5, 5, 10, 10)), Rect::new(5, 5, 10, 10));

        // zero area (acts as a single point)
        let rect_b = Rect::new(10, 10, 0, 0);
        let union = rect_a.union(rect_b);
        assert!(union.contains((10, 10)));
        assert_eq!(union.x, 0);
        assert_eq!(union.y, 0);
        assert_eq!(union.width, 11);
        assert_eq!(union.height, 11);
    }

    #[test]
    fn relative() {
        // inside parent
        let parent = Rect::new(10, 10, 40, 40);
        let abs_child = Rect::new(20, 20, 10, 10);
        let rel_child = abs_child.relative(parent);
        assert_eq!(rel_child.x, 10);
        assert_eq!(rel_child.y, 10);
        assert_eq!(rel_child.width, 10);
        assert_eq!(rel_child.height, 10);

        // at parent origin
        let parent = Rect::new(10, 10, 30, 20);
        let abs_child = Rect::new(10, 10, 10, 10);
        let rel_child = abs_child.relative(parent);
        assert_eq!(rel_child.x, 0);
        assert_eq!(rel_child.y, 0);

        // beyond parent
        let parent = Rect::new(10, 10, 20, 20);
        let abs_child = Rect::new(35, 35, 10, 10);
        let rel_child = abs_child.relative(parent);
        assert_eq!(rel_child.x, 25);
        assert_eq!(rel_child.y, 25);
    }

    #[test]
    fn absolute() {
        // inside parent
        let parent = Rect::new(10, 10, 20, 20);
        let abs_child = Rect::new(5, 5, 10, 10).absolute(parent);
        assert_eq!(abs_child.x, 15);
        assert_eq!(abs_child.y, 15);
        assert_eq!(abs_child.width, 10);
        assert_eq!(abs_child.height, 10);

        // at parent origin
        let rel_child = Rect::new(0, 0, 10, 10);
        let abs_child = rel_child.absolute(parent);
        assert_eq!(abs_child.x, 10);
        assert_eq!(abs_child.y, 10);

        // beyond parent
        let rel_child = Rect::new(25, 25, 10, 10);
        let abs_child = rel_child.absolute(parent);
        assert_eq!(abs_child.x, 35);
        assert_eq!(abs_child.y, 35);
    }

    #[test]
    fn default() {
        let rect = Rect::<u16>::default();
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 0);
        assert_eq!(rect.height, 0);
    }

    #[test]
    fn equality() {
        assert_eq!(Rect::new(5, 5, 10, 10), Rect::new(5, 5, 10, 10));
        assert_ne!(Rect::new(5, 5, 10, 10), Rect::new(5, 5, 10, 11));
    }
}
