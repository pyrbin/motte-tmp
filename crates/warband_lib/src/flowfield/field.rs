use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    ops::{Deref, DerefMut, Index, IndexMut},
};

use rayon::iter::{IndexedParallelIterator, ParallelIterator};

use crate::prelude::*;

#[derive(Default, Clone, Reflect)]
pub struct Field<T> {
    size: usize,
    data: Vec<T>,
}

impl<T> Field<T> {
    pub fn new(size: usize, data: Vec<T>) -> Self {
        Self { data, size }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the 1-dimensional index of a [Cell].
    #[inline]
    pub fn index(&self, cell: Cell) -> usize {
        cell_to_index(*cell, self.size())
    }

    /// Returns the 2-dimensional [Cell] of a 1-dimensional index.
    #[inline]
    pub fn cell(&self, index: usize) -> Cell {
        index_to_cell(index, self.size())
    }

    pub fn within_bounds(&self, cell: Cell) -> bool {
        cell.x() < self.size() && cell.y() < self.size()
    }

    pub fn neighbors(&self, cell: Cell) -> impl Iterator<Item = Cell> + '_ {
        cell.neighbors().filter(move |&cell| self.within_bounds(cell))
    }

    pub fn neighbors8(&self, cell: Cell) -> impl Iterator<Item = Cell> + '_ {
        cell.neighbors8().filter(move |&cell| self.within_bounds(cell))
    }
}

impl<T> Deref for Field<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Field<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> Index<Cell> for Field<T> {
    type Output = T;
    fn index(&self, cell: Cell) -> &T {
        &self.data[self.index(cell)]
    }
}

impl<T> IndexMut<Cell> for Field<T> {
    fn index_mut(&mut self, cell: Cell) -> &mut T {
        let index = self.index(cell);
        &mut self.data[index]
    }
}

/// Returns the 1-dimensional index of a [Cell].
#[inline]
pub fn cell_to_index(cell: (usize, usize), size: usize) -> usize {
    cell.1 * size + cell.0
}

/// Returns the 2-dimensional [Cell] of a 1-dimensional index.
#[inline]
pub fn index_to_cell(index: usize, size: usize) -> Cell {
    Cell::new(index % size, index / size)
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash, Deref, DerefMut, From, Reflect)]
pub struct Cell((usize, usize));

impl Cell {
    pub fn new(x: usize, y: usize) -> Self {
        Self((x, y))
    }

    pub fn from_index(index: usize, size: usize) -> Self {
        index_to_cell(index, size)
    }

    pub fn x(&self) -> usize {
        self.0 .0
    }

    pub fn y(&self) -> usize {
        self.0 .1
    }

    /// Returns the 4-directional neighbors of a coordinate.
    pub fn neighbors(self) -> impl Iterator<Item = Cell> {
        const NEIGHBORS4: [(i32, i32); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];
        NEIGHBORS4.iter().filter_map(move |&(dx, dy)| {
            let x = self.x().checked_add_signed(dx as isize);
            let y = self.y().checked_add_signed(dy as isize);
            x.and_then(|x| y.map(|y| Cell::new(x, y)))
        })
    }

    /// Returns the 8-directional neighbors of a coordinate.
    pub fn neighbors8(self) -> impl Iterator<Item = Cell> {
        const NEIGHBORS8: [(i32, i32); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];
        NEIGHBORS8.iter().filter_map(move |&(dx, dy)| {
            let x = self.x().checked_add_signed(dx as isize);
            let y = self.y().checked_add_signed(dy as isize);
            x.and_then(|x| y.map(|y| Cell::new(x, y)))
        })
    }

    pub fn distance_manhattan(self, other: Cell) -> usize {
        manhattan_dist(self, other)
    }

    pub fn direction(&self, other: Cell) -> Vec2 {
        let dx = other.x() as isize - self.x() as isize;
        let dy = other.y() as isize - self.y() as isize;
        Vec2::new(dx as f32, dy as f32)
    }

    pub fn as_vec2(self) -> Vec2 {
        Vec2::new(self.x() as f32, self.y() as f32)
    }
}

#[inline]
pub fn manhattan_dist(a: Cell, b: Cell) -> usize {
    (a.x() as isize - b.x() as isize).unsigned_abs() + (a.y() as isize - b.y() as isize).unsigned_abs()
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Reflect)]
pub enum Cost {
    Impassable,
    #[default]
    Passable,
}

impl Cost {
    pub fn passable(self) -> bool {
        matches!(self, Cost::Passable)
    }
}

#[derive(Component, Clone, Deref, DerefMut, Reflect)]
pub struct CostField(Field<Cost>);

impl CostField {
    pub fn new(size: usize) -> Self {
        Self(Field::new(size, vec![Cost::Passable; size * size]))
    }

    pub fn from_slice(size: usize, data: &[Cost]) -> Self {
        Self(Field::new(size, data.to_vec()))
    }
}

pub struct PriorityQueue {
    heap: BinaryHeap<Reverse<(u16, Cell)>>,
    contains: Field<bool>,
}

impl PriorityQueue {
    pub fn new(size: usize) -> Self {
        Self { heap: BinaryHeap::new(), contains: Field::new(size, vec![false; size * size]) }
    }

    pub fn push(&mut self, cell: Cell, cost: u16) {
        self.heap.push(Reverse((cost, cell)));
        self.contains[cell] = true;
    }

    pub fn pop(&mut self) -> Option<(Cell, u16)> {
        let Reverse((cost, cell)) = self.heap.pop()?;
        self.contains[cell] = false;
        Some((cell, cost))
    }

    pub fn contains(&self, cell: Cell) -> bool {
        self.contains[cell]
    }

    pub fn clear(&mut self) {
        self.heap.clear();
        for cell in self.contains.iter_mut() {
            *cell = false;
        }
    }
}

// TODO:
// - ditch integration field (see gdc talk)
// - ComputeTask for flowfield calculation
// - FlowFieldTarget + Cache system
// - Varying agent sizes

#[derive(Component, Clone, Deref, DerefMut, Reflect)]
pub struct IntegrationField(Field<u16>);

impl IntegrationField {
    pub fn new(size: usize) -> Self {
        Self(Field::new(size, vec![u16::MAX; size * size]))
    }

    pub fn build(&mut self, goals: impl Iterator<Item = Cell>, cost_field: &CostField) {
        let mut pq = PriorityQueue::new(self.size());

        for cell in self.iter_mut() {
            *cell = u16::MAX;
        }

        for goal in goals.into_iter() {
            pq.push(goal, 0);
            self[goal] = 0;
        }

        while let Some((cell, _)) = pq.pop() {
            for neighbor in cost_field.neighbors8(cell).filter(|&n| cost_field[n].passable()) {
                let cost = self[cell] + manhattan_dist(cell, neighbor) as u16;
                if cost < self[neighbor] {
                    self[neighbor] = cost;
                    if !pq.contains(neighbor) {
                        pq.push(neighbor, cost);
                    }
                }
            }
        }
    }
}

#[derive(Component, Clone, Deref, DerefMut, Reflect)]
pub struct FlowField(Field<Vec2>);

impl FlowField {
    pub const ZERO_FLOW: Vec2 = Vec2::ZERO;

    pub fn new(size: usize) -> Self {
        Self(Field::new(size, vec![FlowField::ZERO_FLOW; size * size]))
    }
}

impl FlowField {
    pub fn build_full(&mut self, goals: impl Iterator<Item = Cell>, cost_field: &CostField) {
        let mut pq: PriorityQueue = PriorityQueue::new(self.size());
        let mut distance_field = Field::new(self.size(), vec![u16::MAX; self.size() * self.size()]);

        for (distance, flow) in distance_field.iter_mut().zip(self.iter_mut()) {
            *distance = u16::MAX;
            *flow = FlowField::ZERO_FLOW;
        }

        for goal in goals.into_iter() {
            pq.push(goal, 0);
            distance_field[goal] = 0;
            self[goal] = FlowField::ZERO_FLOW;
        }

        while let Some((cell, _)) = pq.pop() {
            let is_impassable = !cost_field[cell].passable();
            for (neighbor, passable) in cost_field.neighbors8(cell).map(|n| (n, cost_field[n].passable())) {
                if is_impassable && !passable {
                    continue;
                }

                let cost =
                    if passable { distance_field[cell] + manhattan_dist(cell, neighbor) as u16 } else { u16::MAX };
                if cost < distance_field[neighbor] || !passable {
                    distance_field[neighbor] = cost;
                    self[neighbor] = (cell.as_vec2() - neighbor.as_vec2()).normalize();
                    if passable && !pq.contains(neighbor) {
                        pq.push(neighbor, cost);
                    }
                }
            }
        }
    }

    pub fn build(&mut self, integration_field: &IntegrationField) {
        let size = self.size();
        for ((index, flow), _) in self.iter_mut().enumerate().zip(integration_field.iter()) {
            let cell = index_to_cell(index, size);

            if integration_field[cell] == 0 {
                *flow = FlowField::ZERO_FLOW;
                continue;
            }

            let min_cost = integration_field
                .neighbors8(cell)
                .filter(|&n| integration_field[n] != u16::MAX)
                .min_by(|&a, &b| integration_field[a].cmp(&integration_field[b]));

            if let Some(min_cost) = min_cost {
                *flow = cell.direction(min_cost).normalize();
            } else {
                *flow = FlowField::ZERO_FLOW;
            }
        }
    }
}
