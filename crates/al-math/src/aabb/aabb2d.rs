use crate::vec::Vec2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Aabb2d {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb2d {
    #[inline]
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y
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

    #[inline]
    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    #[inline]
    pub fn size(self) -> Vec2 {
        self.max - self.min
    }

    #[inline]
    pub fn half_size(self) -> Vec2 {
        (self.max - self.min) * 0.5
    }

    #[inline]
    pub fn expand(self, amount: f32) -> Self {
        let d = Vec2::new(amount, amount);
        Self {
            min: self.min - d,
            max: self.max + d,
        }
    }

    #[inline]
    pub fn merge(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[inline]
    pub fn intersection(self, other: Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }
        Some(Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        })
    }

    #[inline]
    pub fn corners(self) -> [Vec2; 4] {
        [
            Vec2::new(self.min.x, self.min.y),
            Vec2::new(self.max.x, self.min.y),
            Vec2::new(self.max.x, self.max.y),
            Vec2::new(self.min.x, self.max.y),
        ]
    }

    /// Transform this AABB by `t` and return the enclosing AABB in world space.
    ///
    /// Returns `self` unchanged if empty (min > max).
    #[inline]
    pub fn transformed_by(self, t: crate::transform::Transform2D) -> Self {
        if !self.is_valid() {
            return self;
        }
        let mut out = Self::inverted();
        for c in self.corners() {
            out.grow(t.transform_point(c));
        }
        out
    }
}
