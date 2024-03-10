use bevy::{
    ecs::{
        entity::EntityHashMap,
        system::{CommandQueue, SystemState},
    },
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use parry2d::{
    na::{Const, OPoint},
    utils::point_in_poly2d,
};

use super::{
    field::{Cell, Field},
    FieldLayout,
};
use crate::{
    navigation::{
        agent::Agent,
        occupancy::{Obstacle, Occupancy, OccupancyAabb},
    },
    prelude::*,
};

#[derive(Clone, Deref, DerefMut, Reflect)]
pub struct CostField(Field<Cost>);

impl CostField {
    pub fn new(size: usize) -> Self {
        Self(Field::new(size, vec![Cost::Traversable; size * size]))
    }

    #[allow(unused)]
    pub fn from_slice(size: usize, data: &[Cost]) -> Self {
        Self(Field::new(size, data.to_vec()))
    }
}

impl Default for CostField {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Reflect)]
pub enum Cost {
    #[default]
    Traversable,
    Blocked,
    Occupied,
}

impl Cost {
    pub fn is_traversable(self) -> bool {
        matches!(self, Cost::Traversable)
    }

    pub fn is_occupied(self) -> bool {
        matches!(self, Cost::Occupied)
    }
}

#[derive(Resource, Deref, DerefMut, Default, Reflect)]
pub struct CellOccupants(HashMap<Cell, HashSet<Entity>>);

/// Inverse of [`CellOccupants`]
#[derive(Resource, Deref, DerefMut, Default, Reflect)]
pub struct CellOccupantsReverse(EntityHashMap<SmallVec<[Cell; 8]>>);

#[derive(Resource, Default)]
pub struct CostUpdateTasks(Vec<Task<CommandQueue>>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct DirtyCells(HashSet<Cell>);

#[derive(Component, Default, Deref, DerefMut, Reflect)]
pub struct OccupancyCells(SmallVec<[Cell; 8]>);

pub(super) fn occupancy_cells_setup(
    mut commands: Commands,
    obstacles: Query<Entity, (With<Occupancy>, Without<OccupancyCells>)>,
) {
    for entity in &obstacles {
        commands.entity(entity).insert(OccupancyCells::default());
    }
}

pub(super) fn occupancy_cells(
    field_layout: Res<FieldLayout>,
    mut obstacles: Query<
        (&mut OccupancyCells, &Occupancy, &OccupancyAabb),
        Or<(Changed<Occupancy>, Changed<OccupancyAabb>, Added<OccupancyCells>)>,
    >,
) {
    obstacles.par_iter_mut().for_each(|(mut occupied_cells, occupancy, aabb)| {
        occupied_cells.clear();

        let Some(shape) = occupancy.shape() else {
            return;
        };

        let min_cell = field_layout.world_to_cell(aabb.min.x0y());
        let max_cell = field_layout.world_to_cell(aabb.max.x0y());

        // FIXME: annoying glam to nalgebra conversion
        let shape = shape.iter().map(|v| OPoint::<f32, Const<2>>::from_slice(&[v.x, v.y])).collect::<Vec<_>>();

        for x in min_cell.x()..=max_cell.x() {
            for y in min_cell.y()..=max_cell.y() {
                let cell = Cell::new(x, y);
                let world_position = field_layout.cell_to_world(cell);
                if point_in_poly2d(&OPoint::from_slice(&[world_position.x, world_position.z]), &shape) {
                    occupied_cells.push(cell);
                }
            }
        }
    });
}

pub(super) fn obstacles(
    mut cell_occupants: ResMut<CellOccupants>,
    mut cell_occupants_reverse: ResMut<CellOccupantsReverse>,
    mut dirty_cells: ResMut<DirtyCells>,
    mut obstacles: Query<(Entity, &OccupancyCells), (Changed<OccupancyCells>, With<Obstacle>)>,
) {
    // perf: par_iter() this?
    obstacles.iter_mut().for_each(|(entity, occupied_cells)| {
        if occupied_cells.is_empty() {
            if let Some(cells) = cell_occupants_reverse.get_mut(&entity) {
                for &cell in cells.iter() {
                    if let Some(occupants) = cell_occupants.get_mut(&cell) {
                        occupants.remove(&entity);
                        dirty_cells.0.insert(cell);
                    }
                }
                cells.clear();
            }
            return;
        }

        let cells = if let Some(cells) = cell_occupants_reverse.get_mut(&entity) {
            // remove from previous
            for old_cell in cells.iter() {
                if !occupied_cells.contains(old_cell) {
                    if let Some(occupants) = cell_occupants.get_mut(old_cell) {
                        occupants.remove(&entity);
                        dirty_cells.0.insert(*old_cell);
                    }
                }
            }

            cells.clear();
            cells
        } else {
            cell_occupants_reverse.insert_unique_unchecked(entity, SmallVec::default()).1
        };

        for &cell in occupied_cells.iter() {
            let occupants = if let Some(occupants) = cell_occupants.get_mut(&cell) {
                occupants
            } else {
                cell_occupants.insert_unique_unchecked(cell, HashSet::default()).1
            };

            cells.push(cell);
            if occupants.insert(entity) {
                dirty_cells.0.insert(cell);
            }
        }
    });
}

pub(super) fn obstacles_cleanup(
    mut removed_obstacles: RemovedComponents<Obstacle>,
    mut cell_occupants: ResMut<CellOccupants>,
    mut cell_occupants_reverse: ResMut<CellOccupantsReverse>,
    mut dirty_cells: ResMut<DirtyCells>,
) {
    for (entity, occupied_cells) in
        removed_obstacles.read().filter_map(|removed| cell_occupants_reverse.remove(&removed).map(|x| (removed, x)))
    {
        for cell in occupied_cells {
            if let Some(occupants) = cell_occupants.get_mut(&cell) {
                occupants.remove(&entity);
            }
            dirty_cells.0.insert(cell);
        }
    }
}

pub(super) fn occupancy_cleanup(
    mut removed_occupancy: RemovedComponents<Occupancy>,
    mut cell_occupants: ResMut<CellOccupants>,
    mut cell_occupants_reverse: ResMut<CellOccupantsReverse>,
    mut dirty_cells: ResMut<DirtyCells>,
) {
    for (entity, occupied_cells) in
        removed_occupancy.read().filter_map(|removed| cell_occupants_reverse.remove(&removed).map(|x| (removed, x)))
    {
        for cell in occupied_cells {
            if let Some(occupants) = cell_occupants.get_mut(&cell) {
                occupants.remove(&entity);
            }
            dirty_cells.0.insert(cell);
        }
    }
}

pub(super) fn update(
    field_layout: Res<FieldLayout>,
    mut cost_update_tasks: ResMut<CostUpdateTasks>,
    mut dirty_cells: ResMut<DirtyCells>,
) {
    // perf: handle in batches if update rates are high

    if dirty_cells.is_empty() {
        return;
    }
    let dirty_cells: Vec<Cell> = dirty_cells.drain().filter(|&cell| field_layout.in_bounds(cell)).collect();

    if dirty_cells.is_empty() {
        return;
    }

    let thread_pool = AsyncComputeTaskPool::get();
    let task = thread_pool.spawn(async move {
        let mut command_queue = CommandQueue::default();
        command_queue.push(move |world: &mut World| {
            let mut system_stat = SystemState::<(
                Query<(Entity, Has<Agent>), With<Obstacle>>,
                Res<CellOccupants>,
                ResMut<super::CostField>,
            )>::new(world);
            let (obstacles, cell_occupants, mut cost_field) = system_stat.get_mut(world);

            cost_field.set_changed();

            let Ok(mut cost_field) = cost_field.write() else {
                error!(target: "cost_field", "lock has been poisoned. Cost updates can no longer be continued.");
                return;
            };

            for (cell, cost) in dirty_cells.iter().map(|cell| {
                let cost = cell_occupants
                    .get(cell)
                    .map(|occupants| {
                        if occupants.is_empty() {
                            Cost::Traversable
                        } else {
                            let mut found = false;
                            for (_, is_agent) in occupants.into_iter().filter_map(|e| obstacles.get(*e).ok()) {
                                if !is_agent {
                                    return Cost::Blocked;
                                }
                                found = true;
                            }

                            if found {
                                return Cost::Occupied;
                            }

                            Cost::Traversable
                        }
                    })
                    .unwrap_or(Cost::Traversable);
                (cell, cost)
            }) {
                cost_field[*cell] = cost;
            }
        });

        command_queue
    });

    cost_update_tasks.0.push(task);
}

pub(super) fn poll_update_tasks(mut commands: Commands, mut cost_update_tasks: ResMut<CostUpdateTasks>) {
    cost_update_tasks.0.retain_mut(|mut task| {
        if let Some(mut commands_queue) = block_on(future::poll_once(&mut task)) {
            commands.append(&mut commands_queue);
            false
        } else {
            true
        }
    });
}

pub(super) fn layout_resize(
    mut cell_occupants: ResMut<CellOccupants>,
    mut cell_occupants_reverse: ResMut<CellOccupantsReverse>,
    mut cost_field: ResMut<super::CostField>,
    field_layout: Res<FieldLayout>,
) {
    cell_occupants.clear();
    cell_occupants_reverse.clear();
    cost_field.write().unwrap().resize(field_layout.size());
    cost_field.set_changed();
    // TODO: update cost of every cell
}
