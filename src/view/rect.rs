use crate::core::geom::Vec2;
use std::ops::Add;

/// A non-empty rectangle on the 2D character grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
  /// Top-left corner, inclusive
  pub top_left: Vec2,

  /// Bottom-right corner, inclusive
  pub bottom_right: Vec2,
}

impl<T> Add<T> for Rect
where
  T: Into<Vec2>,
{
  type Output = Rect;

  fn add(mut self, rhs: T) -> Self {
    self.offset(rhs);
    self
  }
}

impl Rect {
  /// Creates a new `Rect` around a single point.
  #[must_use]
  pub fn from_point<T: Into<Vec2>>(point: T) -> Self {
    Self::from_size(point, (1, 1))
  }

  /// Creates a new `Rect` with the given position and size (minimum 1x1).
  #[must_use]
  pub fn from_size<U: Into<Vec2>, V: Into<Vec2>>(top_left: U, size: V) -> Self {
    let size = size.into();
    let top_left = top_left.into();
    let bottom_right = top_left + size.saturating_sub(Vec2::new(1, 1));
    Self::from_corners(top_left, bottom_right)
  }

  /// Creates a new `Rect` from two opposite corners.
  #[must_use]
  pub fn from_corners<U: Into<Vec2>, V: Into<Vec2>>(a: U, b: V) -> Self {
    let a = a.into();
    let b = b.into();
    Rect {
      top_left: Vec2::min(a, b),
      bottom_right: Vec2::max(a, b),
    }
  }

  /// Grow this rectangle to include `other`.
  pub fn expand_to<R: Into<Rect>>(&mut self, other: R) {
    let other = other.into();
    self.top_left = self.top_left.or_min(other.top_left);
    self.bottom_right = self.bottom_right.or_max(other.bottom_right);
  }

  /// Returns a new rectangle that includes both `self` and `other`.
  #[must_use]
  pub fn expanded_to<R: Into<Rect>>(mut self, other: R) -> Self {
    self.expand_to(other);
    self
  }

  /// Adds the given offset to this rectangle.
  pub fn offset<V: Into<Vec2>>(&mut self, offset: V) {
    let offset = offset.into();
    self.top_left = self.top_left + offset;
    self.bottom_right = self.bottom_right + offset;
  }

  pub fn offset_subtract_y(&mut self) {
    self.top_left = self.top_left.checked_sub(Vec2::new(0, 1)).unwrap();
    self.bottom_right = self.bottom_right.checked_sub(Vec2::new(0, 1)).unwrap();
  }

  pub fn offset_subtract_x(&mut self) {
    self.top_left = self.top_left.checked_sub(Vec2::new(1, 0)).unwrap();
    self.bottom_right = self.bottom_right.checked_sub(Vec2::new(1, 0)).unwrap();
  }

  /// Returns the size of the rectangle.
  pub fn size(self) -> Vec2 {
    self.bottom_right - self.top_left + (1, 1)
  }

  pub fn width(self) -> usize {
    self.size().x
  }

  pub fn height(self) -> usize {
    self.size().y
  }

  pub fn top_left(self) -> Vec2 {
    self.top_left
  }

  pub fn bottom_right(self) -> Vec2 {
    self.bottom_right
  }

  pub fn top_right(self) -> Vec2 {
    Vec2::new(self.right(), self.top())
  }

  pub fn bottom_left(self) -> Vec2 {
    Vec2::new(self.left(), self.bottom())
  }

  pub fn top(self) -> usize {
    self.top_left.y
  }

  pub fn left(self) -> usize {
    self.top_left.x
  }

  pub fn right(self) -> usize {
    self.bottom_right.x
  }

  pub fn bottom(self) -> usize {
    self.bottom_right.y
  }

  pub fn surface(self) -> usize {
    self.width() * self.height()
  }

  /// Returns true if `point` is within [top_left, bottom_right] (inclusive).
  pub fn contains(self, point: Vec2) -> bool {
    point.fits(self.top_left) && point.fits_in(self.bottom_right)
  }
}
