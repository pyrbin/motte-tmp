use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
    time::Duration,
};

use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};

use super::{
    cost::{self, Cost},
    field::{self, Cell, Direction, Field},
    CellIndex, CostField,
};
use crate::prelude::*;

#[derive(Clone, Reflect)]
pub struct FlowField {
    field: Field<Direction>,
    #[reflect(ignore)]
    pq: PriorityQueue,
    #[reflect(ignore)]
    distance_field: Field<u16>,
}

impl Default for FlowField {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Deref for FlowField {
    type Target = Field<Direction>;
    fn deref(&self) -> &Self::Target {
        &self.field
    }
}

impl DerefMut for FlowField {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.field
    }
}

impl FlowField {
    pub fn new(size: usize) -> Self {
        Self {
            field: Field::new(size, vec![Direction::None; size * size]),
            pq: PriorityQueue::new(size),
            distance_field: Field::new(size, vec![u16::MAX; size * size]),
        }
    }

    pub fn build(&mut self, goals: impl Iterator<Item = Cell>, cost_field: &Field<Cost>) {
        debug_assert!(self.size() == cost_field.size());

        self.pq.clear();

        let (flows, distance) = (&mut self.field, &mut self.distance_field);
        for (distance, flow) in distance.iter_mut().zip(flows.iter_mut()) {
            *distance = u16::MAX;
            *flow = Direction::None;
        }

        for goal in goals.into_iter() {
            if !self.in_bounds(goal) {
                // TODO: should we panic here?
                continue;
            }
            self.pq.push(goal, 0);
            self.distance_field[goal] = 0;
            self[goal] = Direction::None;
        }

        let is_traversable = |cell: Cell| cost_field[cell].is_traversable() || cost_field[cell].is_occupied();

        while let Some((cell, _)) = self.pq.pop() {
            let mut update = |neighbor: Cell, traversable: bool| {
                let cost = match cost_field[neighbor] {
                    Cost::Traversable => self.distance_field[cell].saturating_add(cell.distance(neighbor) as u16),
                    Cost::Occupied => {
                        const OCCUPIED_COST: u16 = 1000;
                        self.distance_field[cell].saturating_add(cell.distance(neighbor) as u16 + OCCUPIED_COST)
                    }
                    Cost::Blocked => u16::MAX,
                };

                if !traversable || cost < self.distance_field[neighbor] {
                    self.distance_field[neighbor] = cost;
                    self[neighbor] = neighbor.direction(cell);

                    if traversable && !self.pq.contains(neighbor) {
                        self.pq.push(neighbor, cost);
                    }
                }
            };

            let current_is_traversable: bool = is_traversable(cell);
            for (neighbor, traversable) in cost_field
                .adjacent(cell)
                .map(|n| (n, is_traversable(n)))
                .filter(|(_, traversable)| current_is_traversable || *traversable)
            {
                update(neighbor, traversable);
            }

            for (neighbor, traversable, direction) in cost_field
                .diagonal(cell)
                .map(|n| (n, is_traversable(n), cell.direction(n)))
                .filter(|(_, traversable, _)| current_is_traversable || *traversable)
            {
                let check_cost = |dir: Direction| {
                    let cell = cell.at_direction(dir);
                    cost_field.in_bounds(cell) && is_traversable(cell)
                };

                let valid = match direction {
                    Direction::NorthEast => check_cost(Direction::North) && check_cost(Direction::East),
                    Direction::SouthEast => check_cost(Direction::South) && check_cost(Direction::East),
                    Direction::SouthWest => check_cost(Direction::South) && check_cost(Direction::West),
                    Direction::NorthWest => check_cost(Direction::North) && check_cost(Direction::West),
                    _ => false,
                };

                if !valid {
                    continue;
                }

                update(neighbor, traversable);
            }
        }
    }

    pub fn resize(&mut self, size: usize) {
        self.field.resize(size);
        self.pq = PriorityQueue::new(size);
        self.distance_field = Field::new(size, vec![u16::MAX; size * size]);
    }
}

#[derive(Clone, Default)]
pub(super) struct PriorityQueue {
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

#[derive(Component, Deref, DerefMut)]
pub(super) struct Rebuild(Task<Result<FlowFieldBuildInfo, FlowFieldBuildError>>);

#[derive(Error, Default, Clone, Copy, Debug, Reflect)]
pub enum FlowFieldBuildError {
    #[default]
    #[error("FlowField build failed.")]
    Failed,
}

#[derive(Default, Clone, Copy, Debug, Reflect)]
pub struct FlowFieldBuildInfo {
    start: Duration,
}

pub(super) fn setup(
    mut commands: Commands,
    mut flow_fields: Query<
        Entity,
        (Without<Dirty<super::FlowField>>, Added<super::FlowField>, Without<Deactivated<super::FlowField>>),
    >,
) {
    for entity in &mut flow_fields {
        commands.entity(entity).insert(Dirty::<super::FlowField>::default());
    }
}

pub(super) fn build(
    mut commands: Commands,
    mut flow_fields: Query<
        (Entity, &mut super::FlowField, &CellIndex),
        (With<Dirty<super::FlowField>>, Without<Rebuild>, Without<Deactivated<super::FlowField>>),
    >,
    cost_field: Res<CostField>,
    time: Res<Time>,
) {
    for (entity, flow_field, index) in &mut flow_fields {
        let thread_pool = AsyncComputeTaskPool::get();
        let task: Task<_> = thread_pool.spawn(async_build(
            flow_field.clone(),
            cost_field.clone(),
            std::iter::once(**index),
            FlowFieldBuildInfo { start: time.elapsed() },
        ));
        commands.entity(entity).remove::<Dirty<super::FlowField>>().insert(Rebuild(task));
    }
}

async fn async_build(
    flow_field: Arc<RwLock<FlowField>>,
    cost_field: Arc<RwLock<cost::CostField>>,
    goals: impl Iterator<Item = field::Cell>,
    build_info: FlowFieldBuildInfo,
) -> Result<FlowFieldBuildInfo, FlowFieldBuildError> {
    let Ok(mut field) = flow_field.write() else {
        return Err(FlowFieldBuildError::Failed);
    };

    let Ok(cost_field) = cost_field.read() else {
        return Err(FlowFieldBuildError::Failed);
    };

    if field.size() != cost_field.size() {
        field.resize(cost_field.size());
    }

    field.build(goals, &*cost_field);

    Ok(build_info)
}

pub(super) fn poll_rebuild_tasks(
    mut commands: Commands,
    mut build_tasks: Query<(Entity, &mut Rebuild), With<super::FlowField>>,
    time: Res<Time>,
) {
    for (entity, mut task) in &mut build_tasks {
        if let Some(result) = block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).remove::<Rebuild>();
            match result {
                Ok(result) => {
                    let elapsed_time = time.elapsed() - result.start;
                    // info!("flowfield build, took {:?}ms", elapsed_time.as_millis());
                }
                Err(err) => {
                    // error!("flowfield build failed {:?}", err);
                }
            }
        }
    }
}

pub(super) fn moved(
    mut commands: Commands,
    flow_fields: Query<
        Entity,
        (
            Without<Dirty<super::FlowField>>,
            Changed<CellIndex>,
            With<super::FlowField>,
            Without<Deactivated<super::FlowField>>,
        ),
    >,
) {
    for entity in &flow_fields {
        commands.entity(entity).insert(Dirty::<super::FlowField>::default());
    }
}

pub(super) fn cost_changed(
    mut commands: Commands,
    flow_fields: Query<
        Entity,
        (Without<Dirty<super::FlowField>>, With<super::FlowField>, Without<Deactivated<super::FlowField>>),
    >,
) {
    for entity in &flow_fields {
        commands.entity(entity).insert(Dirty::<super::FlowField>::default());
    }
}

pub(super) fn layout_resize(mut commands: Commands, flow_fields: Query<Entity, With<super::FlowField>>) {
    for entity in &flow_fields {
        commands.entity(entity).insert(Dirty::<super::FlowField>::default());
    }
}
