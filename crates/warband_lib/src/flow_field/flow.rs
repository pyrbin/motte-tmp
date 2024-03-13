use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
    time::Duration,
};

use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};

use super::{
    cost::{self, Cost, OccupancyCells},
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
                // FIXME: should we panic here?
                continue;
            }
            self.pq.push(goal, 0);
            self.distance_field[goal] = 0;
            self[goal] = Direction::None;
        }

        let not_blocked = |cell: Cell| cost_field[cell].is_traversable() || cost_field[cell].is_occupied();

        while let Some((cell, _)) = self.pq.pop() {
            let mut update = |neighbor: Cell, not_blocked: bool| {
                let cost = match cost_field[neighbor] {
                    Cost::Traversable => self.distance_field[cell].saturating_add(cell.distance(neighbor) as u16),
                    Cost::Occupied => {
                        const OCCUPIED_MODIFIER: f32 = 100.0;
                        self.distance_field[cell]
                            .saturating_add(cell.distance_scaled(neighbor, OCCUPIED_MODIFIER) as u16)
                        // u16::MAX - 1
                    }
                    Cost::Blocked => u16::MAX,
                };

                if !not_blocked || cost < self.distance_field[neighbor] {
                    self.distance_field[neighbor] = cost;
                    self[neighbor] = neighbor.direction(cell);

                    if not_blocked && !self.pq.contains(neighbor) {
                        self.pq.push(neighbor, cost);
                    }
                }
            };

            let current_is_not_blocked: bool = not_blocked(cell);
            for (neighbor, not_blocked) in cost_field
                .adjacent(cell)
                .map(|n| (n, not_blocked(n)))
                .filter(|(_, not_blocked)| current_is_not_blocked || *not_blocked)
            {
                update(neighbor, not_blocked);
            }

            for (neighbor, not_blocked, direction) in cost_field
                .diagonal(cell)
                .map(|n| (n, not_blocked(n), cell.direction(n)))
                .filter(|(_, not_blocked, _)| current_is_not_blocked || *not_blocked)
            {
                let check_traversable = |dir: Direction| {
                    let cell = cell.at_direction(dir);
                    cost_field.in_bounds(cell) && cost_field[cell].is_traversable()
                };

                let valid = match direction {
                    Direction::NorthEast => check_traversable(Direction::North) && check_traversable(Direction::East),
                    Direction::SouthEast => check_traversable(Direction::South) && check_traversable(Direction::East),
                    Direction::SouthWest => check_traversable(Direction::South) && check_traversable(Direction::West),
                    Direction::NorthWest => check_traversable(Direction::North) && check_traversable(Direction::West),
                    _ => false,
                };

                if !valid {
                    continue;
                }

                update(neighbor, not_blocked);
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
struct PriorityQueue {
    heap: BinaryHeap<Reverse<(u16, Cell)>>,
    contains: Field<bool>,
}

impl PriorityQueue {
    #[inline]
    fn new(size: usize) -> Self {
        Self { heap: BinaryHeap::new(), contains: Field::new(size, vec![false; size * size]) }
    }
    #[inline]
    fn push(&mut self, cell: Cell, cost: u16) {
        self.heap.push(Reverse((cost, cell)));
        self.contains[cell] = true;
    }

    #[inline]
    fn pop(&mut self) -> Option<(Cell, u16)> {
        let Reverse((cost, cell)) = self.heap.pop()?;
        self.contains[cell] = false;
        Some((cell, cost))
    }
    #[inline]
    fn contains(&self, cell: Cell) -> bool {
        self.contains[cell]
    }
    #[inline]
    fn clear(&mut self) {
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
        (Entity, &mut super::FlowField, &CellIndex, Option<&OccupancyCells>),
        (With<Dirty<super::FlowField>>, Without<Rebuild>, Without<Deactivated<super::FlowField>>),
    >,
    cost_field: Res<CostField>,
    time: Res<Time>,
) {
    // perf: limit update rate.

    for (entity, flow_field, index, cells) in &mut flow_fields {
        let thread_pool = AsyncComputeTaskPool::get();

        let goals = match cells {
            Some(cells) => cells.iter().cloned().collect(),
            None => vec![**index],
        };

        let task: Task<_> = thread_pool.spawn(async_build(
            flow_field.clone(),
            cost_field.clone(),
            goals.into_iter(),
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

    field.build(goals, &cost_field);

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
                    info!(target: "flow_field", "{:?}ms", elapsed_time.as_millis());
                }
                Err(err) => {
                    error!(target: "flow_field", "failed {:?}", err);
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

pub(super) fn cells_changed(
    mut commands: Commands,
    flow_fields: Query<
        Entity,
        (
            Without<Dirty<super::FlowField>>,
            Changed<OccupancyCells>,
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
