use std::{cmp::Reverse, collections::BinaryHeap};

use super::{obstacle::ObstacleField, Cell, Direction, Field};
use crate::{
    navigation::{
        agent::Agent,
        flow_field::{
            footprint::ExpandedFootprint,
            layout::{FieldBounds, FieldLayout},
            CellIndex,
        },
        obstacle::Obstacle,
    },
    prelude::*,
};

#[derive(Component, Default, Reflect)]
pub struct FlowField<const AGENT: Agent> {
    flow: Field<Direction>,
    #[reflect(ignore)]
    integration: Field<IntegrationCost>,
    #[reflect(ignore)]
    heap: Heap,
}

impl<const AGENT: Agent> std::ops::Deref for FlowField<AGENT> {
    type Target = Field<Direction>;
    fn deref(&self) -> &Self::Target {
        &self.flow
    }
}

impl<const AGENT: Agent> FlowField<AGENT> {
    pub fn from_layout(layout: &FieldLayout) -> Self {
        let size = layout.width() * layout.height();
        Self {
            flow: Field::new(layout.width(), layout.height(), vec![Direction::None; size]),
            integration: Field::new(layout.width(), layout.height(), vec![IntegrationCost::Blocked; size]),
            heap: Heap::new(layout.width(), layout.height()),
        }
    }

    #[inline(always)]
    pub fn build(
        &mut self,
        goals: impl Iterator<Item = Cell>,
        obstacle_field: &ObstacleField,
        obstacles: impl Iterator<Item = (Cell, &[Cell])>,
        bounds: &[Cell],
    ) {
        debug_assert!(self.len() == obstacle_field.len());

        let (flow, integration, heap) = (&mut self.flow, &mut self.integration, &mut self.heap);
        for (cost, flow) in integration.iter_mut().zip(flow.iter_mut()) {
            *cost = IntegrationCost::Blocked;
            *flow = Direction::None;
        }

        heap.clear();

        for goal in goals.into_iter() {
            if !flow.valid(goal) {
                continue;
            }
            heap.push(goal, IntegrationCost::Goal);
            integration[goal] = IntegrationCost::Goal;
            flow[goal] = Direction::None;
        }

        let is_traversable = |cell: Cell| obstacle_field.traversable(cell, AGENT);

        while let Some((cell, _)) = heap.pop() {
            let mut process = |neighbor: Cell| {
                let traversable = is_traversable(neighbor);
                let is_goal = integration[neighbor] == IntegrationCost::Goal;

                if !traversable && !is_goal {
                    return;
                }

                let cost = if traversable {
                    let distance = cell.manhattan(neighbor) as u16;
                    IntegrationCost::Traversable(integration[cell].cost().saturating_add(distance))
                } else {
                    IntegrationCost::Goal
                };

                if cost < integration[neighbor] {
                    integration[neighbor] = cost;
                    flow[neighbor] = neighbor.direction(cell);
                    if !heap.contains(neighbor) {
                        heap.push(neighbor, cost);
                    }
                }
            };

            for neighbor in obstacle_field.adjacent(cell) {
                process(neighbor);
            }

            let is_diagonal_neighbor_valid = |cell: Cell, direction: Direction| {
                let check = |direction: Direction| {
                    let cell = cell.neighbor(direction);
                    let Some(cell) = cell else {
                        return false;
                    };
                    is_traversable(cell)
                };

                match direction {
                    Direction::NorthEast => check(Direction::North) && check(Direction::East),
                    Direction::SouthEast => check(Direction::South) && check(Direction::East),
                    Direction::SouthWest => check(Direction::South) && check(Direction::West),
                    Direction::NorthWest => check(Direction::North) && check(Direction::West),
                    _ => false,
                }
            };

            for neighbor in obstacle_field
                .diagonal(cell)
                .filter_map(|n| is_diagonal_neighbor_valid(cell, cell.direction(n)).then_some(n))
            {
                process(neighbor);
            }
        }

        // obstacles
        for (origin, footprint) in obstacles {
            for &cell in footprint {
                if !flow.valid(cell) || integration[cell] == IntegrationCost::Goal {
                    continue;
                }

                let direction = (if integration[cell] == IntegrationCost::Goal {
                    origin.as_vec2() - cell.as_vec2()
                } else {
                    cell.as_vec2() - origin.as_vec2()
                })
                .normalize_or_zero();

                flow[cell] = Direction::from_vec(direction);
            }
        }

        // field bounds
        let center = flow.center();
        for &cell in bounds {
            let direction = (center.as_vec2() - cell.as_vec2()).normalize_or_zero();
            flow[cell] = Direction::from_vec(direction);
        }
    }
}

#[derive(Clone, Default)]
struct Heap {
    heap: BinaryHeap<Reverse<(IntegrationCost, Cell)>>,
    contains: Field<bool>,
}

impl Heap {
    #[inline]
    fn new(width: usize, height: usize) -> Self {
        Self { heap: BinaryHeap::new(), contains: Field::new(width, height, vec![false; width * height]) }
    }
    #[inline]
    fn push(&mut self, cell: Cell, cost: IntegrationCost) {
        self.heap.push(Reverse((cost, cell)));
        self.contains[cell] = true;
    }

    #[inline]
    fn pop(&mut self) -> Option<(Cell, IntegrationCost)> {
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

#[derive(Default, Clone, Copy, Eq, Debug, Hash, Reflect)]
#[repr(u16)]
pub(self) enum IntegrationCost {
    #[default]
    Blocked,
    Traversable(u16),
    Goal,
}

impl IntegrationCost {
    #[inline]
    pub const fn cost(&self) -> u16 {
        match self {
            Self::Blocked => u16::MAX,
            Self::Traversable(cost) => *cost,
            Self::Goal => 0,
        }
    }
}

impl PartialEq for IntegrationCost {
    fn eq(&self, other: &Self) -> bool {
        self.cost() == other.cost()
    }
}

impl PartialOrd for IntegrationCost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IntegrationCost {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost().cmp(&other.cost())
    }
}

#[inline(always)]
pub(in crate::navigation) fn build<const AGENT: Agent>(
    mut flow_fields: Query<(&mut FlowField<AGENT>, &CellIndex, Option<&ExpandedFootprint<AGENT>>)>,
    obstacle_field: Res<ObstacleField>,
    obstacles: Query<(&ExpandedFootprint<AGENT>, &CellIndex), With<Obstacle>>,
    bounds: Res<FieldBounds<AGENT>>,
) {
    // TODO: should just rebuild if needed

    flow_fields.par_iter_mut().for_each(|(mut flow_field, cell_index, footprint)| {
        let goals = match footprint {
            Some(footprint) if let ExpandedFootprint::Cells(cells) = footprint => cells.iter().cloned().collect_vec(),
            None if let CellIndex::Valid(cell, _) = cell_index => vec![*cell],
            _ => return,
        };

        let now = std::time::Instant::now();

        flow_field.build(
            goals.into_iter(),
            &obstacle_field,
            obstacles.iter().filter_map(|(footprint, cell_index)| {
                if let ExpandedFootprint::Cells(cells) = footprint
                    && let CellIndex::Valid(cell, _) = cell_index
                {
                    Some((*cell, cells.as_slice()))
                } else {
                    return None;
                }
            }),
            bounds.as_slice(),
        );

        let end = std::time::Instant::now() - now;

        info!(target: "flow_field", "build: {:?}", end);
    });
}

#[cfg(feature = "debug")]
pub(crate) fn gizmos(mut gizmos: Gizmos, layout: Res<FieldLayout>, flow_fields: Query<&FlowField<{ Agent::Huge }>>) {
    for flow_field in &flow_fields {
        for (cell, &direction) in flow_field.iter().enumerate().map(|(i, cost)| (layout.cell_from_index(i), cost)) {
            let position = layout.position(cell).x0y();
            if let Some(direction) = direction.as_direction2d() {
                let start = position;
                let end = start + direction.x0y() * (layout.cell_size() / 2.0);
                gizmos.arrow(start + Vec3::Y * 0.1, end + Vec3::Y * 0.1, Color::WHITE);
            }
        }
    }
}
