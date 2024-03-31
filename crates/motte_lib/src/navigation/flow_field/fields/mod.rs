use std::ops::{Deref, DerefMut, Index, IndexMut};

pub mod flow;
pub mod obstacle;

use crate::prelude::*;

/// The scalar type used for coordinates.
pub type Scalar = u8;

/// The signed scalar type used for coordinates.
pub type SignedScalar = i16;

/// (X, Y) coordinates in a field.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Deref, DerefMut, From, Reflect)]
pub struct Cell((Scalar, Scalar));

/// Instantiates a new [Cell] from coordinates.
#[inline]
pub const fn cell(x: Scalar, y: Scalar) -> Cell {
    Cell::new(x, y)
}

impl Cell {
    /// The scalar type used for coordinates.
    pub type Scalar = Scalar;

    /// The signed scalar type used for coordinates.
    pub type SignedScalar = SignedScalar;

    /// (0, 0)
    pub const ZERO: Self = Self::new(0, 0);

    /// Creates a new [Cell] from coordinates.
    #[inline]
    pub const fn new(x: Scalar, y: Scalar) -> Self {
        Self((x, y))
    }

    /// Creates a new [Cell] with all coordinates set to `v`.
    #[inline]
    pub const fn splat(v: Scalar) -> Self {
        Self((v, v))
    }

    /// Returns the 2-dimensional [Cell] of a 1-dimensional index.
    #[inline]
    pub const fn from_index(index: usize, size: Scalar) -> Self {
        Cell::new((index % size as usize) as Scalar, (index / size as usize) as Scalar)
    }

    /// Rounds floating point coordinates to [Cell].
    #[inline]
    pub const fn round((mut x, mut y): (f32, f32)) -> Self {
        use parry2d::na::SimdComplexField;

        let z = -x - y;
        let rx = x.simd_round();
        let ry = y.simd_round();
        let rz = z.simd_round();

        let x_diff = (rx - x).simd_abs();
        let y_diff = (ry - y).simd_abs();
        let z_diff = (rz - z).simd_abs();

        if x_diff > y_diff && x_diff > z_diff {
            x = -ry - rz;
        } else if y_diff > z_diff {
            y = -rx - rz;
        }

        Self((x.simd_round() as Scalar, y.simd_round() as Scalar))
    }

    /// Creates a new [Cell] from an array.
    #[inline]
    pub const fn from_array([x, y]: [Scalar; 2]) -> Self {
        Self((x, y))
    }

    /// Creates a new [Cell] from the first 2 values in `slice`.
    #[inline]
    pub const fn from_slice(slice: &[Scalar]) -> Self {
        Self::new(slice[0], slice[1])
    }

    #[inline]
    pub const fn x(&self) -> Scalar {
        self.0 .0
    }

    #[inline]
    pub const fn y(&self) -> Scalar {
        self.0 .1
    }

    /// Returns the 1-dimensional index of a [Cell].
    #[inline]
    pub const fn index(&self, size: Scalar) -> usize {
        (self.y() as usize * size as usize) + self.x() as usize
    }

    #[inline]
    pub const fn as_vec2(self) -> Vec2 {
        Vec2::new(self.x() as f32, self.y() as f32)
    }

    #[inline]
    pub const fn as_array(self) -> [Scalar; 2] {
        [self.x(), self.y()]
    }

    #[inline]
    pub const fn as_tuple(self) -> (Scalar, Scalar) {
        (self.x(), self.y())
    }

    /// Manhattan distance between two [Cell]s.
    #[inline]
    pub const fn manhattan(self, rhs: Cell) -> u32 {
        (self.x().abs_diff(rhs.x()) + self.y().abs_diff(rhs.y())) as u32
    }

    /// Euclidean distance between two [Cell]s.
    #[inline]
    pub const fn euclidean(self, rhs: Cell) -> f32 {
        use parry2d::na::SimdComplexField;
        self.euclidean_sqrt(rhs).simd_sqrt()
    }

    /// Euclidean distance squared between two [Cell]s.
    #[inline]
    pub const fn euclidean_sqrt(self, rhs: Cell) -> f32 {
        let dx = self.x() as f32 - rhs.x() as f32;
        let dy = self.y() as f32 - rhs.y() as f32;
        dx * dx + dy * dy
    }

    /// Chebyshev distance between two [Cell]s.
    #[inline]
    pub const fn chebyshev(self, rhs: Cell) -> u32 {
        self.x().abs_diff(rhs.x()).max(self.y().abs_diff(rhs.y())) as u32
    }

    /// Returns the [Direction] from `self` to `other`.
    #[inline]
    pub const fn direction(&self, other: Cell) -> Direction {
        let dx = other.x() as i32 - self.x() as i32;
        let dy = other.y() as i32 - self.y() as i32;
        match (dx, dy) {
            (0, -1) => Direction::North,
            (1, -1) => Direction::NorthEast,
            (1, 0) => Direction::East,
            (1, 1) => Direction::SouthEast,
            (0, 1) => Direction::South,
            (-1, 1) => Direction::SouthWest,
            (-1, 0) => Direction::West,
            (-1, -1) => Direction::NorthWest,
            _ => Direction::None,
        }
    }

    #[inline]
    pub fn adjacent(self) -> impl Iterator<Item = Cell> {
        const NEIGHBORS: [(SignedScalar, SignedScalar); 4] = [
            Direction::North.as_scalar(),
            Direction::East.as_scalar(),
            Direction::South.as_scalar(),
            Direction::West.as_scalar(),
        ];
        NEIGHBORS.iter().filter_map(move |&(dx, dy)| {
            let x = self.x().checked_add_signed(dx as i8);
            let y = self.y().checked_add_signed(dy as i8);
            x.and_then(|x| y.map(|y| Cell::new(x, y)))
        })
    }

    #[inline]
    pub fn diagonal(self) -> impl Iterator<Item = Cell> {
        const NEIGHBORS: [(SignedScalar, SignedScalar); 4] = [
            Direction::NorthEast.as_scalar(),
            Direction::SouthEast.as_scalar(),
            Direction::SouthWest.as_scalar(),
            Direction::NorthWest.as_scalar(),
        ];
        NEIGHBORS.iter().filter_map(move |&(dx, dy)| {
            let x = self.x().checked_add_signed(dx as i8);
            let y = self.y().checked_add_signed(dy as i8);
            x.and_then(|x| y.map(|y| Cell::new(x, y)))
        })
    }

    #[inline]
    pub fn neighbors(self) -> impl Iterator<Item = Cell> {
        self.adjacent().chain(self.diagonal())
    }

    #[inline]
    pub fn neighbors_at(self, directions: impl Iterator<Item = Direction>) -> impl Iterator<Item = Option<Cell>> {
        directions.map(move |d| self.neighbor(d))
    }

    #[inline]
    pub fn neighbor(self, direction: Direction) -> Option<Cell> {
        let (dx, dy) = direction.as_scalar();
        let x = self.x().checked_add_signed(dx as i8);
        let y = self.y().checked_add_signed(dy as i8);
        x.and_then(|x| y.map(|y| Cell::new(x, y)))
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Reflect)]
#[repr(u8)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    #[default]
    None,
}

impl Direction {
    #[inline]
    pub fn from_vec(vec: Vec2) -> Self {
        let normalized = vec.normalize_or_zero();
        if normalized == Vec2::ZERO {
            return Self::None;
        }
        let x = normalized.x.round();
        let y = normalized.y.round();
        if x == 0.0 && y == -1.0 {
            Self::North
        } else if x == 1.0 && y == -1.0 {
            Self::NorthEast
        } else if x == 1.0 && y == 0.0 {
            Self::East
        } else if x == 1.0 && y == 1.0 {
            Self::SouthEast
        } else if x == 0.0 && y == 1.0 {
            Self::South
        } else if x == -1.0 && y == 1.0 {
            Self::SouthWest
        } else if x == -1.0 && y == 0.0 {
            Self::West
        } else if x == -1.0 && y == -1.0 {
            Self::NorthWest
        } else {
            Self::None
        }
    }

    #[inline]
    pub fn as_direction2d(self) -> Option<Direction2d> {
        let (x, y) = self.as_scalar();
        Direction2d::from_xy(x as f32, y as f32).ok()
    }

    #[inline]
    pub const fn as_scalar(self) -> (SignedScalar, SignedScalar) {
        match self {
            Self::North => (0, -1),
            Self::NorthEast => (1, -1),
            Self::East => (1, 0),
            Self::SouthEast => (1, 1),
            Self::South => (0, 1),
            Self::SouthWest => (-1, 1),
            Self::West => (-1, 0),
            Self::NorthWest => (-1, -1),
            Self::None => (0, 0),
        }
    }

    #[inline]
    pub const fn as_vec2(self) -> Vec2 {
        Vec2::new(self.as_scalar().0 as f32, self.as_scalar().1 as f32)
    }
}

/// A 2-dimensional field of cells.
#[derive(Default, Clone, Reflect)]
pub struct Field<T> {
    width: Scalar,
    height: Scalar,
    data: Vec<T>,
}

impl<T> Field<T> {
    /// Creates a new [Field] with the given dimensions and data.
    pub const fn new(width: Scalar, height: Scalar, data: Vec<T>) -> Self {
        Self { data, width, height }
    }

    #[inline]
    pub const fn width(&self) -> Scalar {
        self.width
    }

    #[inline]
    pub const fn height(&self) -> Scalar {
        self.height
    }

    #[inline]
    pub const fn center(&self) -> Cell {
        Cell::new(self.width / 2, self.height / 2)
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.width as usize * self.height as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the 1-dimensional index of a [Cell] with bounds checking.
    #[inline]
    pub const fn index(&self, cell: Cell) -> Option<usize> {
        if self.valid(cell) {
            Some(cell.index(self.width))
        } else {
            None
        }
    }

    /// Returns the 1-dimensional index of a [Cell]. Does not check if the cell is valid for the field.
    #[inline]
    pub const fn index_no_check(&self, cell: Cell) -> usize {
        debug_assert!(cell.index(self.width) < self.len());
        cell.index(self.width)
    }

    /// Returns the 2-dimensional [Cell] of a 1-dimensional index with bounds checking.
    #[inline]
    pub const fn cell(&self, index: usize) -> Option<Cell> {
        let cell = Cell::from_index(index, self.width);
        if self.valid(cell) {
            Some(cell)
        } else {
            None
        }
    }

    /// Returns the 2-dimensional [Cell] of a 1-dimensional index.
    #[inline]
    pub const fn cell_no_check(&self, index: usize) -> Cell {
        debug_assert!(index < self.len());
        Cell::from_index(index, self.width)
    }

    #[inline]
    pub const fn get(&self, cell: Cell) -> Option<T>
    where
        T: Default + Copy,
    {
        if self.valid(cell) {
            let data = self[cell];
            Some(data)
        } else {
            None
        }
    }

    #[inline]
    pub const fn valid(&self, cell: Cell) -> bool {
        cell.x() < self.width && cell.y() < self.height
    }

    #[inline]
    pub fn adjacent(&self, cell: Cell) -> impl Iterator<Item = Cell> + '_ {
        cell.adjacent().filter(move |&cell| self.valid(cell))
    }

    #[inline]
    pub fn diagonal(&self, cell: Cell) -> impl Iterator<Item = Cell> + '_ {
        cell.diagonal().filter(move |&cell| self.valid(cell))
    }

    #[inline]
    pub fn neighbors(&self, cell: Cell) -> impl Iterator<Item = Cell> + '_ {
        cell.neighbors().filter(move |&cell| self.valid(cell))
    }

    #[inline]
    pub fn neighbor(&self, cell: Cell, direction: Direction) -> Option<Cell> {
        cell.neighbor(direction).filter(move |&cell| self.valid(cell))
    }

    #[inline]
    pub fn neighbors_at<'a>(
        &'a self,
        cell: Cell,
        directions: impl Iterator<Item = Direction> + 'a,
    ) -> impl Iterator<Item = Option<Cell>> + 'a {
        cell.neighbors_at(directions).map(
            move |cell| {
                if cell.is_some() && self.valid(cell.unwrap()) {
                    cell
                } else {
                    None
                }
            },
        )
    }

    #[inline]
    pub fn resize(&mut self, width: Scalar, height: Scalar)
    where
        T: Default + Clone,
    {
        self.width = width;
        self.height = height;
        self.data.resize(self.len(), T::default());
    }
}

impl<T> Deref for Field<T> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Field<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> Index<Cell> for Field<T> {
    type Output = T;
    #[inline]
    fn index(&self, cell: Cell) -> &T {
        &self.data[self.index_no_check(cell)]
    }
}

impl<T> IndexMut<Cell> for Field<T> {
    #[inline]
    fn index_mut(&mut self, cell: Cell) -> &mut T {
        let index = self.index_no_check(cell);
        &mut self.data[index]
    }
}

impl<T> Index<usize> for Field<T> {
    type Output = T;
    #[inline]
    fn index(&self, index: usize) -> &T {
        &self.data[index]
    }
}

impl<T> IndexMut<usize> for Field<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.data[index]
    }
}
