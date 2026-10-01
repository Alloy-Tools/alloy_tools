use crate::vec::Vec2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AABB2d {
    pub min: Vec2,
    pub max: Vec2,
}

impl AABB2d {
    #[inline]
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }
    #[inline]
    pub fn inverted() -> Self {
        Self {
            min: Vec2::new(f32::INFINITY, f32::INFINITY),
            max: Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }
    /// Returns an AABB that encloses the points.
    pub fn from_points(points: &[Vec2]) -> Self {
        let mut aabb = Self::inverted();
        for &point in points {
            aabb.grow(point);
        }
        aabb
    }
    /// Expands the bounding box to enclose the given point.
    #[inline]
    pub fn grow(&mut self, point: Vec2) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
    }
    /// Returns `true` if self overlaps other.
    #[inline]
    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
    /// Returns `true` if the point is inside the AABB.
    #[inline]
    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }
}
