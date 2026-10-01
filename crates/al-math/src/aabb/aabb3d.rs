use crate::vec::Vec3;

//#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AABB3d {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB3d {
    #[inline]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
    #[inline]
    pub fn inverted() -> Self {
        Self {
            min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }
    /// Returns an AABB that encloses the points.
    pub fn from_points(points: &[Vec3]) -> Self {
        let mut aabb = Self::inverted();
        for &point in points {
            aabb.grow(point);
        }
        aabb
    }
    /// Expands the bounding box to enclose the given point.
    #[inline]
    pub fn grow(&mut self, point: Vec3) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }
    /// Returns `true` if self overlaps other.
    #[inline]
    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
    /// Returns `true` if the point is inside the AABB.
    #[inline]
    pub fn contains(self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }
}
