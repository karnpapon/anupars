/// 2D coordinate types replacing cursive::Vec2 / cursive::XY.
use std::ops::{Add, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct XY<T> {
  pub x: T,
  pub y: T,
}

impl<T> XY<T> {
  pub fn new(x: T, y: T) -> Self {
    Self { x, y }
  }
}

/// Unsigned 2D point - the workhorse replacement for cursive::Vec2.
pub type Vec2 = XY<usize>;

impl Vec2 {
  pub const ZERO: Vec2 = Vec2 { x: 0, y: 0 };
  pub const MAX: Vec2 = Vec2 {
    x: usize::MAX,
    y: usize::MAX,
  };

  pub fn zero() -> Self {
    Self::ZERO
  }

  pub fn min(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x.min(b.x), a.y.min(b.y))
  }

  pub fn max(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x.max(b.x), a.y.max(b.y))
  }

  pub fn or_min(self, other: Vec2) -> Vec2 {
    Vec2::min(self, other)
  }

  pub fn or_max(self, other: Vec2) -> Vec2 {
    Vec2::max(self, other)
  }

  pub fn saturating_sub(self, rhs: Vec2) -> Vec2 {
    Vec2::new(self.x.saturating_sub(rhs.x), self.y.saturating_sub(rhs.y))
  }

  pub fn checked_sub(self, rhs: Vec2) -> Option<Vec2> {
    Some(Vec2::new(
      self.x.checked_sub(rhs.x)?,
      self.y.checked_sub(rhs.y)?,
    ))
  }

  /// true if self >= other component-wise (self contains other).
  pub fn fits(self, other: Vec2) -> bool {
    self.x >= other.x && self.y >= other.y
  }

  /// true if self <= other component-wise (self fits inside other).
  pub fn fits_in(self, other: Vec2) -> bool {
    self.x <= other.x && self.y <= other.y
  }

  /// Component-wise saturating add.
  pub fn saturating_add(self, rhs: Vec2) -> Vec2 {
    Vec2::new(self.x.saturating_add(rhs.x), self.y.saturating_add(rhs.y))
  }

  /// true if top_left <= self < top_left + size (exclusive upper bound).
  pub fn fits_in_rect(self, top_left: Vec2, size: Vec2) -> bool {
    self.fits(top_left) && self.x < top_left.x + size.x && self.y < top_left.y + size.y
  }
}

impl PartialOrd for Vec2 {
  /// Component-wise: a < b iff a.x < b.x && a.y < b.y.
  fn partial_cmp(&self, other: &Vec2) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering::*;
    if self == other {
      Some(Equal)
    } else if self.x < other.x && self.y < other.y {
      Some(Less)
    } else if self.x > other.x && self.y > other.y {
      Some(Greater)
    } else {
      None
    }
  }
}

// --- Add impls ---

impl Add<Vec2> for Vec2 {
  type Output = Vec2;
  fn add(self, rhs: Vec2) -> Vec2 {
    Vec2::new(self.x + rhs.x, self.y + rhs.y)
  }
}

impl Add<(usize, usize)> for Vec2 {
  type Output = Vec2;
  fn add(self, (rx, ry): (usize, usize)) -> Vec2 {
    Vec2::new(self.x + rx, self.y + ry)
  }
}

// --- Sub impls ---

impl Sub<Vec2> for Vec2 {
  type Output = Vec2;
  fn sub(self, rhs: Vec2) -> Vec2 {
    Vec2::new(self.x - rhs.x, self.y - rhs.y)
  }
}

impl Sub<(usize, usize)> for Vec2 {
  type Output = Vec2;
  fn sub(self, (rx, ry): (usize, usize)) -> Vec2 {
    Vec2::new(self.x - rx, self.y - ry)
  }
}

// --- From impls ---

impl From<(usize, usize)> for Vec2 {
  fn from((x, y): (usize, usize)) -> Self {
    Vec2::new(x, y)
  }
}

impl From<(u32, u32)> for Vec2 {
  fn from((x, y): (u32, u32)) -> Self {
    Vec2::new(x as usize, y as usize)
  }
}

impl From<(u16, u16)> for Vec2 {
  fn from((x, y): (u16, u16)) -> Self {
    Vec2::new(x as usize, y as usize)
  }
}

impl From<(u8, u8)> for Vec2 {
  fn from((x, y): (u8, u8)) -> Self {
    Vec2::new(x as usize, y as usize)
  }
}

impl From<(i32, i32)> for Vec2 {
  fn from((x, y): (i32, i32)) -> Self {
    Vec2::new(x as usize, y as usize)
  }
}

impl From<(i32, i32)> for XY<isize> {
  fn from((x, y): (i32, i32)) -> Self {
    XY::new(x as isize, y as isize)
  }
}

impl Add<XY<isize>> for XY<isize> {
  type Output = XY<isize>;
  fn add(self, rhs: XY<isize>) -> XY<isize> {
    XY::new(self.x + rhs.x, self.y + rhs.y)
  }
}
