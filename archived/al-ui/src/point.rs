use num_traits::zero;

use crate::{CoordType, NumUtils};
use std::ops::Neg;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<T: CoordType, const N: usize>([T; N]);

// 1D point
impl<T: CoordType> Point<T, 1> {
    pub fn new_1d(x: T) -> Self {
        Point([x])
    }

    pub fn x(&self) -> T {
        self.0[0]
    }

    pub fn set_x(&mut self, x: T) {
        self.0[0] = x;
    }
}

// 2D point
impl<T: CoordType> Point<T, 2> {
    pub fn new_2d(x: T, y: T) -> Self {
        Point([x, y])
    }

    pub fn x(&self) -> T {
        self.0[0]
    }

    pub fn y(&self) -> T {
        self.0[1]
    }

    pub fn set_x(&mut self, x: T) {
        self.0[0] = x;
    }

    pub fn set_y(&mut self, y: T) {
        self.0[1] = y;
    }
}

// 3D point
impl<T: CoordType> Point<T, 3> {
    pub fn new_3d(x: T, y: T, z: T) -> Self {
        Point([x, y, z])
    }

    pub fn x(&self) -> T {
        self.0[0]
    }

    pub fn y(&self) -> T {
        self.0[1]
    }

    pub fn z(&self) -> T {
        self.0[2]
    }

    pub fn set_x(&mut self, x: T) {
        self.0[0] = x;
    }

    pub fn set_y(&mut self, y: T) {
        self.0[1] = y;
    }

    pub fn set_z(&mut self, z: T) {
        self.0[2] = z;
    }
}

// 4D point
impl<T: CoordType> Point<T, 4> {
    pub fn new_4d(x: T, y: T, z: T, w: T) -> Self {
        Point([x, y, z, w])
    }

    pub fn x(&self) -> T {
        self.0[0]
    }

    pub fn y(&self) -> T {
        self.0[1]
    }

    pub fn z(&self) -> T {
        self.0[2]
    }

    pub fn w(&self) -> T {
        self.0[3]
    }

    pub fn set_x(&mut self, x: T) {
        self.0[0] = x;
    }

    pub fn set_y(&mut self, y: T) {
        self.0[1] = y;
    }

    pub fn set_z(&mut self, z: T) {
        self.0[2] = z;
    }

    pub fn set_w(&mut self, w: T) {
        self.0[3] = w;
    }
}

impl<T: CoordType, const N: usize> NumUtils for Point<T, N> {
    fn sqrt(self) -> f64 {
        self.magnitude()
    }

    fn abs_value(mut self) -> Self {
        *self.map(T::abs_value)
    }

    fn is_nan(&self) -> bool {
        self.any(T::is_nan)
    }

    fn min(mut self, other: Self) -> Self {
        *self.index_map(|i, t| t.min(other.0[i]))
    }

    fn max(mut self, other: Self) -> Self {
        *self.index_map(|i, t| t.max(other.0[i]))
    }

    fn saturating_sub(mut self, other: Self) -> Self {
        *self.index_map(|i, t| t.saturating_sub(other.0[i]))
    }

    fn saturating_add(mut self, other: Self) -> Self {
        *self.index_map(|i, t| t.saturating_add(other.0[i]))
    }
}

impl<T: CoordType, const N: usize> Point<T, N> {
    /// Consumes the point and returns its array representation
    pub fn into_array(self) -> [T; N] {
        self.0
    }

    /// Consumes the point and returns a new point of type `U` by applying `f` to each element.
    pub fn map_into<U: CoordType, F: Fn(T) -> U>(self, f: F) -> Point<U, N> {
        Point(self.0.map(f))
    }

    /// Borrows the point and returns a new point of type `U` by applying `f` to each element by reference.
    pub fn map_ref<U: CoordType, F: Fn(&T) -> U>(&self, f: F) -> Point<U, N> {
        Point(std::array::from_fn(|i| f(&self.0[i])))
    }

    /// Modifies each element in‑place using a closure that receives a mutable reference.
    pub fn map_mut<F: FnMut(&mut T)>(&mut self, mut f: F) -> &mut Self {
        for elem in &mut self.0 {
            f(elem)
        }
        self
    }

    /// Modifies each element in‑place using a closure that returns a new value.
    pub fn map<F: Fn(T) -> T>(&mut self, f: F) -> &mut Self {
        for elem in &mut self.0 {
            *elem = f(*elem);
        }
        self
    }

    pub fn index_map_into<U: CoordType, F: Fn(usize, T) -> U>(self, f: F) -> Point<U, N> {
        Point(std::array::from_fn(|i| f(i, self.0[i])))
    }

    pub fn index_map_ref<U: CoordType, F: Fn(usize, &T) -> U>(&self, f: F) -> Point<U, N> {
        Point(std::array::from_fn(|i| f(i, &self.0[i])))
    }

    pub fn index_map_mut<F: FnMut(usize, &mut T)>(&mut self, mut f: F) -> &mut Self {
        for (i, elem) in self.0.iter_mut().enumerate() {
            f(i, elem)
        }
        self
    }

    pub fn index_map<F: Fn(usize, T) -> T>(&mut self, f: F) -> &mut Self {
        for i in 0..self.0.len() {
            self.0[i] = f(i, self.0[i]);
        }
        self
    }

    pub fn any<F: Fn(&T) -> bool>(&self, f: F) -> bool {
        for elem in &self.0 {
            if f(elem) {
                return true;
            }
        }
        false
    }

    pub fn all<F: Fn(&T) -> bool>(&self, f: F) -> bool {
        for elem in &self.0 {
            if !f(elem) {
                return false;
            }
        }
        true
    }

    pub fn dot(&self, other: &Point<T, N>) -> T {
        let mut sum = T::zero();
        for i in 0..N {
            sum = sum + (self.0[i] * other.0[i]);
        }
        sum
    }

    pub fn is_origin(&self, epsilon: T) -> bool {
        self.0.iter().all(|t| t <= &epsilon)
    }

    pub fn distance(&self, other: &Point<T, N>) -> f64 {
        let mut sum_sq = 0.0;
        for i in 0..N {
            let dx = other.0[i]
                .to_f64()
                .unwrap_or(0.0)
                .saturating_sub(self.0[i].to_f64().unwrap_or(0.0));
            sum_sq += dx * dx;
        }
        sum_sq.sqrt()
    }

    pub fn magnitude(&self) -> f64 {
        let mut sum_sq = 0.0;
        for i in 0..N {
            let dx = T::to_f64(&self.0[i]).unwrap_or(0.0);
            sum_sq += dx * dx;
        }
        sum_sq.sqrt()
    }

    pub fn scale(&self, factor: f64) -> Point<f64, N> {
        self.map_into(|t| t.to_f64().unwrap_or(0.0) * factor)
    }

    pub fn lerp(&self, other: &Point<T, N>, t: f64) -> Point<f64, N> {
        Point(std::array::from_fn(|i| {
            self.0[i].to_f64().unwrap_or(0.0) * (1.0 - t) + other.0[i].to_f64().unwrap_or(0.0) * t
        }))
    }
}

impl<T: CoordType + Default, const N: usize> Default for Point<T, N> {
    fn default() -> Self {
        Point(std::array::from_fn(|_| T::default()))
    }
}

// 2D-specific functions for rotation, angles, translation
impl<T: CoordType> Point<T, 2> {
    pub fn rotate(&self, angle_degrees: f64) -> Point<f64, 2> {
        let angle_radians = angle_degrees.to_radians();
        let cos_theta = angle_radians.cos();
        let sin_theta = angle_radians.sin();
        let x = self.x().to_f64().unwrap_or(0.0) * cos_theta
            - self.y().to_f64().unwrap_or(0.0) * sin_theta;
        let y = self.x().to_f64().unwrap_or(0.0) * sin_theta
            + self.y().to_f64().unwrap_or(0.0) * cos_theta;
        Point([x, y])
    }

    pub fn unit_rotate(&self, angle_degrees: f64) -> Point<T, 2> {
        let rotated = self.rotate(angle_degrees);
        Point([
            T::from_f64(rotated.0[0]).unwrap_or_else(zero),
            T::from_f64(rotated.0[1]).unwrap_or_else(zero),
        ])
    }

    pub fn as_angle(&self) -> f64 {
        self.y()
            .to_f64()
            .unwrap_or(0.0)
            .atan2(self.x().to_f64().unwrap_or(0.0))
            .to_degrees()
    }

    pub fn angle_to(&self, other: &Point<T, 2>) -> f64 {
        let dx = other.x().to_f64().unwrap_or(0.0) - self.x().to_f64().unwrap_or(0.0);
        let dy = other.y().to_f64().unwrap_or(0.0) - self.y().to_f64().unwrap_or(0.0);
        dy.atan2(dx).to_degrees()
    }

    pub fn normalize(&self) -> Point<f64, 2> {
        let length = self.magnitude();
        if length == 0.0 {
            Point([0.0, 0.0])
        } else {
            let x_normalized = self.x().to_f64().unwrap_or(0.0) / length;
            let y_normalized = self.y().to_f64().unwrap_or(0.0) / length;
            Point([x_normalized, y_normalized])
        }
    }

    pub fn unit_normalize(&self) -> Point<T, 2> {
        let length = self.magnitude();
        if length == 0.0 {
            Point([T::zero(), T::zero()])
        } else {
            let x_normalized =
                T::from_f64(self.x().to_f64().unwrap_or(0.0) / length).unwrap_or_else(zero);
            let y_normalized =
                T::from_f64(self.y().to_f64().unwrap_or(0.0) / length).unwrap_or_else(zero);
            Point([x_normalized, y_normalized])
        }
    }

    pub fn unit_vector(&self) -> Point<f64, 2> {
        let length = self.magnitude();
        if length == 0.0 {
            Point([0.0, 0.0])
        } else {
            Point([
                self.x().to_f64().unwrap_or(0.0) / length,
                self.y().to_f64().unwrap_or(0.0) / length,
            ])
        }
    }

    pub fn scale_to_length(&self, length: f64) -> Point<f64, 2> {
        let unit = self.unit_vector();
        unit.scale(length)
    }

    pub fn translate(&self, dx: f64, dy: f64) -> Point<f64, 2> {
        Point([
            self.x().to_f64().unwrap_or(0.0) + dx,
            self.y().to_f64().unwrap_or(0.0) + dy,
        ])
    }
}

// Generic operator implementations for all Point<T, N>
// Add single T to all coordinates
impl<T: CoordType + std::ops::Add<Output = T> + Copy, const N: usize> std::ops::Add<T>
    for Point<T, N>
{
    type Output = Self;

    fn add(self, other: T) -> Self {
        Point(self.0.map(|x| x + other))
    }
}

// Add &T to all coordinates
impl<T: CoordType + std::ops::Add<Output = T> + Copy, const N: usize> std::ops::Add<&T>
    for Point<T, N>
{
    type Output = Self;

    fn add(self, other: &T) -> Self {
        Point(self.0.map(|x| x + *other))
    }
}

// Subtract single T from all coordinates
impl<T: CoordType + std::ops::Sub<Output = T> + Copy, const N: usize> std::ops::Sub<T>
    for Point<T, N>
{
    type Output = Self;

    fn sub(self, other: T) -> Self {
        Point(self.0.map(|x| x - other))
    }
}

// Subtract &T from all coordinates
impl<T: CoordType + std::ops::Sub<Output = T> + Copy, const N: usize> std::ops::Sub<&T>
    for Point<T, N>
{
    type Output = Self;

    fn sub(self, other: &T) -> Self {
        Point(self.0.map(|x| x - *other))
    }
}

// Multiply all coordinates by single T
impl<T: CoordType + std::ops::Mul<Output = T> + Copy, const N: usize> std::ops::Mul<T>
    for Point<T, N>
{
    type Output = Self;

    fn mul(self, scalar: T) -> Self {
        Point(self.0.map(|x| x * scalar))
    }
}

// Multiply all coordinates by &T
impl<T: CoordType + std::ops::Mul<Output = T> + Copy, const N: usize> std::ops::Mul<&T>
    for Point<T, N>
{
    type Output = Self;

    fn mul(self, scalar: &T) -> Self {
        Point(self.0.map(|x| x * *scalar))
    }
}

// Divide all coordinates by single T
impl<T: CoordType + std::ops::Div<Output = T> + Copy, const N: usize> std::ops::Div<T>
    for Point<T, N>
{
    type Output = Self;

    fn div(self, scalar: T) -> Self {
        Point(self.0.map(|x| x / scalar))
    }
}

// Divide all coordinates by &T
impl<T: CoordType + std::ops::Div<Output = T> + Copy, const N: usize> std::ops::Div<&T>
    for Point<T, N>
{
    type Output = Self;

    fn div(self, scalar: &T) -> Self {
        Point(self.0.map(|x| x / *scalar))
    }
}

// Add two points together
impl<T: CoordType + std::ops::Add<Output = T> + Copy, const N: usize> std::ops::Add
    for Point<T, N>
{
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let arr: [T; N] = std::array::from_fn(|i| self.0[i] + other.0[i]);
        Point(arr)
    }
}

// Add &Point to Point
impl<T: CoordType + std::ops::Add<Output = T> + Copy, const N: usize> std::ops::Add<&Point<T, N>>
    for Point<T, N>
{
    type Output = Self;

    fn add(self, other: &Point<T, N>) -> Self {
        let arr: [T; N] = std::array::from_fn(|i| self.0[i] + other.0[i]);
        Point(arr)
    }
}

// Subtract two points
impl<T: CoordType + std::ops::Sub<Output = T> + Copy, const N: usize> std::ops::Sub
    for Point<T, N>
{
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let arr: [T; N] = std::array::from_fn(|i| self.0[i] - other.0[i]);
        Point(arr)
    }
}

// Subtract &Point from Point
impl<T: CoordType + std::ops::Sub<Output = T> + Copy, const N: usize> std::ops::Sub<&Point<T, N>>
    for Point<T, N>
{
    type Output = Self;

    fn sub(self, other: &Point<T, N>) -> Self {
        let arr: [T; N] = std::array::from_fn(|i| self.0[i] - other.0[i]);
        Point(arr)
    }
}

// Multiply two points component-wise
impl<T: CoordType + std::ops::Mul<Output = T> + Copy, const N: usize> std::ops::Mul<Point<T, N>>
    for Point<T, N>
{
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let arr: [T; N] = std::array::from_fn(|i| self.0[i] * other.0[i]);
        Point(arr)
    }
}

// Multiply Point by &Point component-wise
impl<T: CoordType + std::ops::Mul<Output = T> + Copy, const N: usize> std::ops::Mul<&Point<T, N>>
    for Point<T, N>
{
    type Output = Self;

    fn mul(self, other: &Point<T, N>) -> Self {
        let arr: [T; N] = std::array::from_fn(|i| self.0[i] * other.0[i]);
        Point(arr)
    }
}

// Divide two points component-wise
impl<T: CoordType + std::ops::Div<Output = T> + Copy, const N: usize> std::ops::Div<Point<T, N>>
    for Point<T, N>
{
    type Output = Self;

    fn div(self, other: Self) -> Self {
        let arr: [T; N] = std::array::from_fn(|i| self.0[i] / other.0[i]);
        Point(arr)
    }
}

// Divide Point by &Point component-wise
impl<T: CoordType + std::ops::Div<Output = T> + Copy, const N: usize> std::ops::Div<&Point<T, N>>
    for Point<T, N>
{
    type Output = Self;

    fn div(self, other: &Point<T, N>) -> Self {
        let arr: [T; N] = std::array::from_fn(|i| self.0[i] / other.0[i]);
        Point(arr)
    }
}

// Negate all coordinates
impl<T: CoordType + std::ops::Neg<Output = T>, const N: usize> Neg for Point<T, N> {
    type Output = Self;

    fn neg(self) -> Self {
        Point(self.0.map(|x| -x))
    }
}

// Negate all coordinates by reference
impl<T: CoordType + std::ops::Neg<Output = T>, const N: usize> Neg for &Point<T, N> {
    type Output = Point<T, N>;

    fn neg(self) -> Point<T, N> {
        Point(self.0.map(|x| -x))
    }
}

// Implement From<[T; N]> for Point<T, N>
impl<T: CoordType, const N: usize> From<[T; N]> for Point<T, N> {
    fn from(arr: [T; N]) -> Self {
        Point(arr)
    }
}

// Implement From<Point<T, N>> for [T; N]
impl<T: CoordType, const N: usize> From<Point<T, N>> for [T; N] {
    fn from(point: Point<T, N>) -> Self {
        point.0
    }
}

// Convenience: Implement From<(T, T)> for Point<T, 2>
impl<T: CoordType> From<(T, T)> for Point<T, 2> {
    fn from(tuple: (T, T)) -> Self {
        Point([tuple.0, tuple.1])
    }
}

// Convenience: Implement From<Point<T, 2>> for (T, T)
impl<T: CoordType> From<Point<T, 2>> for (T, T) {
    fn from(point: Point<T, 2>) -> Self {
        (point.0[0], point.0[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_1d() {
        let p = Point::new_1d(1);
        assert_eq!(p.x(), 1);
    }

    #[test]
    fn create_2d() {
        let p = Point::new_2d(1, 2);
        assert_eq!(p.x(), 1);
        assert_eq!(p.y(), 2);
    }

    #[test]
    fn create_3d() {
        let p = Point::new_3d(1, 2, 3);
        assert_eq!(p.x(), 1);
        assert_eq!(p.y(), 2);
        assert_eq!(p.z(), 3);
    }

    #[test]
    fn create_4d() {
        let p = Point::new_4d(1, 2, 3, 4);
        assert_eq!(p.x(), 1);
        assert_eq!(p.y(), 2);
        assert_eq!(p.z(), 3);
        assert_eq!(p.w(), 4);
    }

    #[test]
    fn set_x() {
        let mut p = Point::new_2d(5, 10);
        p.set_x(20);
        assert_eq!(p.x(), 20);
    }

    #[test]
    fn set_y() {
        let mut p = Point::new_2d(5, 10);
        p.set_y(25);
        assert_eq!(p.y(), 25);
    }

    #[test]
    fn from_array() {
        let arr = [5, 10];
        let p = Point::from(arr);
        assert_eq!(p.x(), 5);
        assert_eq!(p.y(), 10);
    }

    #[test]
    fn into_array() {
        let p = Point::new_2d(5, 10);
        let arr = p.into_array();
        assert_eq!(arr[0], 5);
        assert_eq!(arr[1], 10);
    }

    #[test]
    fn from_tuple() {
        let t = (5, 10);
        let p = Point::from(t);
        assert_eq!(p.x(), 5);
        assert_eq!(p.y(), 10);
    }

    #[test]
    fn into_tuple() {
        let p = Point::new_2d(5, 10);
        let (x, y) = p.into();
        assert_eq!(x, 5);
        assert_eq!(y, 10);
    }

    #[test]
    fn map_add_one() {
        let mut p = Point::new_2d(5, 10);
        p.map(|x| x + 1);
        assert_eq!(p.x(), 6);
        assert_eq!(p.y(), 11);
    }

    #[test]
    fn map_multiply() {
        let mut p = Point::new_2d(5, 10);
        p.map(|x| x * 2);
        assert_eq!(p.x(), 10);
        assert_eq!(p.y(), 20);
    }

    #[test]
    fn map_zero() {
        let mut p = Point::new_2d(5, 10);
        p.map(|_| 0);
        assert_eq!(p.x(), 0);
        assert_eq!(p.y(), 0);
    }

    #[test]
    fn map_into() {
        let p = Point::new_2d(5u8, 10);
        let p_u16 = p.map_into(|x| x as u16);
        assert_eq!(p_u16.x(), 5);
        assert_eq!(p_u16.y(), 10);
    }

    #[test]
    fn map_ref() {
        let p = Point::new_2d(5, 10);
        let p_doubled = p.map_ref(|x| x * 2);
        assert_eq!(p_doubled.x(), 10);
        assert_eq!(p_doubled.y(), 20);
        // Original unchanged
        assert_eq!(p.x(), 5);
    }

    #[test]
    fn map_mut() {
        let mut p = Point::new_2d(5, 10);
        p.map_mut(|x| *x = *x + 1);
        assert_eq!(p.x(), 6);
        assert_eq!(p.y(), 11);
    }

    #[test]
    fn index_map() {
        let mut p = Point::new_2d(5, 10);
        p.index_map(|i, x| x + i as u8);
        assert_eq!(p.x(), 5); // 5 + 0
        assert_eq!(p.y(), 11); // 10 + 1
    }

    #[test]
    fn index_map_into() {
        let p = Point::new_2d(5u8, 10);
        let p_u16 = p.index_map_into(|i, x| (x as u16) + (i as u16));
        assert_eq!(p_u16.x(), 5);
        assert_eq!(p_u16.y(), 11);
    }

    #[test]
    fn index_map_ref() {
        let p = Point::new_2d(5u8, 10);
        let p_new = p.index_map_ref(|i, x| x + i as u8);
        assert_eq!(p_new.x(), 5);
        assert_eq!(p_new.y(), 11);
    }

    #[test]
    fn index_map_mut() {
        let mut p = Point::new_2d(5, 10);
        p.index_map_mut(|i, x| *x = *x + i as i32);
        assert_eq!(p.x(), 5);
        assert_eq!(p.y(), 11);
    }

    #[test]
    fn any_true() {
        let p = Point::new_2d(5, 10);
        assert!(p.any(|x| x > &8));
    }

    #[test]
    fn any_false() {
        let p = Point::new_2d(5, 10);
        assert!(!p.any(|x| x > &20));
    }

    #[test]
    fn all_true() {
        let p = Point::new_2d(5, 10);
        assert!(p.all(|x| x > &3));
    }

    #[test]
    fn all_false() {
        let p = Point::new_2d(5, 10);
        assert!(!p.all(|x| x > &7));
    }

    #[test]
    fn magnitude_2d() {
        let p = Point::new_2d(3., 4.);
        // sqrt(3^2 + 4^2) = sqrt(25) = 5
        assert_eq!(p.magnitude(), 5.);
    }

    #[test]
    fn magnitude_3d() {
        let p = Point::new_3d(1., 2., 2.);
        // sqrt(1^2 + 2^2 + 2^2) = sqrt(9) = 3
        assert_eq!(p.magnitude(), 3.);
    }

    #[test]
    fn magnitude_zero() {
        let p = Point::new_2d(0., 0.);
        assert_eq!(p.magnitude(), 0.);
    }

    #[test]
    fn distance() {
        let p1 = Point::new_2d(0., 0.);
        let p2 = Point::new_2d(3., 4.);
        assert_eq!(p1.distance(&p2), 5.);
    }

    #[test]
    fn dot_product() {
        let p1 = Point::new_2d(1, 2);
        let p2 = Point::new_2d(3, 4);
        assert_eq!(p1.dot(&p2), 11); // 1*3 + 2*4 = 11
    }

    #[test]
    fn abs_value() {
        let mut p = Point::new_2d(-5, 10);
        p = p.abs_value();
        assert_eq!(p.x(), 5);
        assert_eq!(p.y(), 10);
    }

    #[test]
    fn saturating_ops() {
        let mut p = Point::new_2d(u8::MAX - 1, 5);
        p = p.saturating_add(Point::new_2d(2, 3));
        assert_eq!(p.x(), u8::MAX);
        assert_eq!(p.y(), 8);

        let mut p = Point::new_2d(0u8, 10);
        p = p.saturating_sub(Point::new_2d(2, 3));
        assert_eq!(p.x(), 0);
        assert_eq!(p.y(), 7);
    }

    #[test]
    fn min() {
        let mut p1 = Point::new_2d(5, 10);
        p1 = p1.min(Point::new_2d(3, 15));
        assert_eq!(p1.x(), 3);
        assert_eq!(p1.y(), 10);
    }

    #[test]
    fn max() {
        let mut p1 = Point::new_2d(5, 10);
        p1 = p1.max(Point::new_2d(3, 15));
        assert_eq!(p1.x(), 5);
        assert_eq!(p1.y(), 15);
    }

    #[test]
    fn normalize() {
        let p = Point::new_2d(3., 4.);
        let normalized = p.normalize();
        assert!((normalized.x() - 0.6).abs() < f64::EPSILON);
        assert!((normalized.y() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn normalize_zero() {
        let p = Point::new_2d(0., 0.);
        let normalized = p.normalize();
        assert_eq!(normalized.x(), 0.);
        assert_eq!(normalized.y(), 0.);
    }

    #[test]
    fn as_angle() {
        let p = Point::new_2d(1., 0.);
        assert_eq!(p.as_angle(), 0.);
        let p = Point::new_2d(1., 1.);
        assert_eq!(p.as_angle(), 45.);
        let p = Point::new_2d(0., 1.);
        assert_eq!(p.as_angle(), 90.);
    }

    #[test]
    fn translate() {
        let p = Point::new_2d(5., 10.);
        let translated = p.translate(2., 3.);
        assert_eq!(translated.x(), 7.);
        assert_eq!(translated.y(), 13.);
    }

    #[test]
    fn scale() {
        let p = Point::new_2d(2., 3.);
        let scaled = p.scale(2.);
        assert_eq!(scaled.x(), 4.);
        assert_eq!(scaled.y(), 6.);
    }

    #[test]
    fn scalar_ops() {
        let p = Point::new_2d(4, 9);
        let p = p - 2;
        assert_eq!(p.x(), 2);
        assert_eq!(p.y(), 7);
        
        let p = p + 3;
        assert_eq!(p.x(), 5);
        assert_eq!(p.y(), 10);

        let p = p * 2;
        assert_eq!(p.x(), 10);
        assert_eq!(p.y(), 20);

        let p = p / 2;
        assert_eq!(p.x(), 5);
        assert_eq!(p.y(), 10);
    }

    #[test]
    fn point_ops() {
        let p1 = Point::new_2d(5, 10);
        let p2 = Point::new_2d(2, 3);
        let result = p1 + p2;
        assert_eq!(result.x(), 7);
        assert_eq!(result.y(), 13);

        let p1 = Point::new_2d(10, 15);
        let p2 = Point::new_2d(3, 5);
        let result = p1 - p2;
        assert_eq!(result.x(), 7);
        assert_eq!(result.y(), 10);

        let p1 = Point::new_2d(5, 10);
        let p2 = Point::new_2d(2, 3);
        let result = p1 * p2;
        assert_eq!(result.x(), 10);
        assert_eq!(result.y(), 30);

        let p1 = Point::new_2d(10, 20);
        let p2 = Point::new_2d(2, 4);
        let result = p1 / p2;
        assert_eq!(result.x(), 5);
        assert_eq!(result.y(), 5);

        let p = Point::new_2d(-5, 10);
        let result = -p;
        assert_eq!(result.x(), 5);
        assert_eq!(result.y(), -10);
    }

    #[test]
    fn default() {
        let p = Point::<u8, 2>::default();
        assert_eq!(p.x(), 0);
        assert_eq!(p.y(), 0);
    }

    #[test]
    fn equality() {
        assert_eq!(Point::new_2d(5, 10), Point::new_2d(5, 10));
        assert_ne!(Point::new_2d(5, 10), Point::new_2d(5, 11));
    }
}
