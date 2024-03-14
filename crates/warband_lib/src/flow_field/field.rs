use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Deref, DerefMut, From, Reflect)]
pub struct Cell((usize, usize));

impl Cell {
    #[inline]
    pub fn new(x: usize, y: usize) -> Self {
        Self((x, y))
    }

    #[inline]
    #[allow(unused)]
    pub fn splat(v: usize) -> Self {
        Self((v, v))
    }

    #[inline]
    #[allow(unused)]
    pub fn from_index(index: usize, size: usize) -> Self {
        index_to_cell(index, size)
    }

    #[inline]
    pub fn round((mut x, mut y): (f32, f32)) -> Self {
        let z = -x - y;
        let rx = x.round();
        let ry = y.round();
        let rz = z.round();

        let x_diff = (rx - x).abs();
        let y_diff = (ry - y).abs();
        let z_diff = (rz - z).abs();

        if x_diff > y_diff && x_diff > z_diff {
            x = -ry - rz;
        } else if y_diff > z_diff {
            y = -rx - rz;
        }

        Self((x.round() as usize, y.round() as usize))
    }

    #[inline]
    pub fn x(&self) -> usize {
        self.0 .0
    }

    #[inline]
    pub fn y(&self) -> usize {
        self.0 .1
    }

    pub fn index(&self, size: usize) -> usize {
        cell_to_index((self.x(), self.y()), size)
    }

    #[inline]
    pub fn manhattan(self, rhs: Cell) -> usize {
        (self.x() as isize - rhs.x() as isize).unsigned_abs() + (self.y() as isize - rhs.y() as isize).unsigned_abs()
    }

    #[inline]
    pub fn adjacent(self) -> impl Iterator<Item = Cell> {
        const NEIGHBORS4: [(i32, i32); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];
        NEIGHBORS4.iter().filter_map(move |&(dx, dy)| {
            let x = self.x().checked_add_signed(dx as isize);
            let y = self.y().checked_add_signed(dy as isize);
            x.and_then(|x| y.map(|y| Cell::new(x, y)))
        })
    }

    #[inline]
    pub fn diagonal(self) -> impl Iterator<Item = Cell> {
        const NEIGHBORS4: [(i32, i32); 4] = [(1, -1), (-1, 1), (1, 1), (-1, -1)];
        NEIGHBORS4.iter().filter_map(move |&(dx, dy)| {
            let x = self.x().checked_add_signed(dx as isize);
            let y = self.y().checked_add_signed(dy as isize);
            x.and_then(|x| y.map(|y| Cell::new(x, y)))
        })
    }

    #[inline]
    pub fn neighbors(self) -> impl Iterator<Item = Cell> {
        self.adjacent().chain(self.diagonal())
    }

    #[inline]
    pub fn sample_neighbors<'a>(self, directions: &'a [Direction]) -> impl Iterator<Item = (Cell, Direction)> + 'a {
        directions
            .iter()
            .copied()
            .filter_map(move |direction| self.neighbor(direction).map(|neighbor| (neighbor, direction)))
    }

    #[inline]
    pub fn direction(&self, other: Cell) -> Direction {
        let dx = other.x() as isize - self.x() as isize;
        let dy = other.y() as isize - self.y() as isize;
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
    pub fn neighbor(self, direction: Direction) -> Option<Cell> {
        let (dx, dy) = match direction {
            Direction::North => (0, -1),
            Direction::NorthEast => (1, -1),
            Direction::East => (1, 0),
            Direction::SouthEast => (1, 1),
            Direction::South => (0, 1),
            Direction::SouthWest => (-1, 1),
            Direction::West => (-1, 0),
            Direction::NorthWest => (-1, -1),
            Direction::None => return None,
        };

        let x = self.x().checked_add_signed(dx as isize);
        let y = self.y().checked_add_signed(dy as isize);
        x.and_then(|x| y.map(|y| Cell::new(x, y)))
    }

    #[inline]
    pub fn as_vec2(self) -> Vec2 {
        Vec2::new(self.x() as f32, self.y() as f32)
    }
}

/// Returns the 1-dimensional index of a [Cell].
#[inline]
pub fn cell_to_index(cell: (usize, usize), width: usize) -> usize {
    cell.1 * width + cell.0
}

/// Returns the 2-dimensional [Cell] of a 1-dimensional index.
#[inline]
pub fn index_to_cell(index: usize, width: usize) -> Cell {
    Cell::new(index % width, index / width)
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Reflect)]
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
    pub fn as_direction2d(self) -> Option<Direction2d> {
        match self {
            Self::North => Direction2d::from_xy(0.0, -1.0).ok(),
            Self::NorthEast => Direction2d::from_xy(1.0, -1.0).ok(),
            Self::East => Direction2d::from_xy(1.0, 0.0).ok(),
            Self::SouthEast => Direction2d::from_xy(1.0, 1.0).ok(),
            Self::South => Direction2d::from_xy(0.0, 1.0).ok(),
            Self::SouthWest => Direction2d::from_xy(-1.0, 1.0).ok(),
            Self::West => Direction2d::from_xy(-1.0, 0.0).ok(),
            Self::NorthWest => Direction2d::from_xy(-1.0, -1.0).ok(),
            Self::None => None,
        }
    }
}

#[derive(Default, Clone, Reflect)]
pub struct Field<T> {
    width: usize,
    height: usize,
    data: Vec<T>,
}

impl<T> Field<T> {
    pub fn new(width: usize, height: usize, data: Vec<T>) -> Self {
        Self { data, width, height }
    }

    /// Returns the 1-dimensional index of a [Cell].
    #[inline]
    pub fn index(&self, cell: Cell) -> usize {
        cell_to_index(*cell, self.width)
    }

    /// Returns the 2-dimensional [Cell] of a 1-dimensional index.
    #[inline]
    pub fn cell(&self, index: usize) -> Cell {
        index_to_cell(index, self.width)
    }

    #[inline]
    pub fn get(&self, cell: Cell) -> Option<T>
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
    pub fn valid(&self, cell: Cell) -> bool {
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
    #[allow(unused)]
    pub fn neighbors(&self, cell: Cell) -> impl Iterator<Item = Cell> + '_ {
        cell.neighbors().filter(move |&cell| self.valid(cell))
    }

    #[inline]
    pub fn resize(&mut self, width: usize, height: usize)
    where
        T: Default + Clone,
    {
        self.width = width;
        self.height = height;
        self.data.resize(self.len(), T::default());
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.width * self.height
    }

    #[inline]
    pub fn empty(&self) -> bool {
        self.len() == 0
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
        &self.data[self.index(cell)]
    }
}

impl<T> IndexMut<Cell> for Field<T> {
    #[inline]
    fn index_mut(&mut self, cell: Cell) -> &mut T {
        let index = self.index(cell);
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
