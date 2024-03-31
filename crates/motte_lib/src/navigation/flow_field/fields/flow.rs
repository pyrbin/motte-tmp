use std::{cmp::Reverse, collections::BinaryHeap};

use super::{
    obstacle::{ObstacleField, Occupant},
    Cell, Direction, Field,
};
use crate::{
    navigation::{
        agent::Agent,
        flow_field::{
            footprint::{ExpandedFootprint, Footprint},
            layout::FieldLayout,
            CellIndex,
        },
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
        let len: usize = layout.len();
        Self {
            flow: Field::new(layout.width(), layout.height(), vec![Direction::None; len]),
            integration: Field::new(layout.width(), layout.height(), vec![IntegrationCost::default(); len]),
            heap: Heap::new(layout.width(), layout.height()),
        }
    }

    #[inline]
    pub fn build(&mut self, goals: impl Iterator<Item = Cell>, obstacle_field: &ObstacleField) {
        debug_assert!(self.len() == obstacle_field.len());

        let (flow, integration, heap) = (&mut self.flow, &mut self.integration, &mut self.heap);
        for (cost, flow) in integration.iter_mut().zip(flow.iter_mut()) {
            *cost = IntegrationCost::default();
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
        let is_diagonal_move_traversable = |cell: Cell, direction: Direction| {
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

        while let Some((cell, _)) = heap.pop() {
            let mut process = |neighbor: Cell| {
                let current: IntegrationCost = integration[cell];
                let cost = if is_traversable(neighbor) {
                    // Traversable
                    let distance = cell.manhattan(neighbor) as u8;
                    IntegrationCost::Traversable(current.cost().saturating_add(distance))
                } else if integration[neighbor] == IntegrationCost::Goal {
                    // Goal
                    IntegrationCost::Goal
                } else {
                    // Blocked or Occupied
                    let distance = cell.manhattan(neighbor) as u8;
                    let (depth, cost) = current.depth_and_cost();
                    if matches!(obstacle_field.occupant(neighbor), Occupant::Agent) {
                        IntegrationCost::Occupied(depth.saturating_add(1), cost.saturating_add(distance))
                    } else {
                        IntegrationCost::Blocked(depth.saturating_add(1), cost.saturating_add(distance))
                    }
                };

                if current.valid_traversal(cost) && cost < integration[neighbor] {
                    integration[neighbor] = cost;
                    if !heap.contains(neighbor) {
                        heap.push(neighbor, cost);
                    }
                }
            };

            for neighbor in obstacle_field.adjacent(cell) {
                process(neighbor);
            }

            for neighbor in
                obstacle_field.diagonal(cell).filter(|&n| is_diagonal_move_traversable(cell, cell.direction(n)))
            {
                process(neighbor);
            }
        }

        let width = integration.width();
        for i in 0..integration.len() {
            let cell = Cell::from_index(i, width);
            if let Some(min) = integration
                .adjacent(cell)
                .chain(integration.diagonal(cell).filter(|&n| is_diagonal_move_traversable(cell, cell.direction(n))))
                .min_by(|a, b| integration[*a].cmp(&integration[*b]))
            {
                flow[cell] = cell.direction(min);
            }
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
    fn new(width: super::Scalar, height: super::Scalar) -> Self {
        Self {
            heap: BinaryHeap::new(),
            contains: Field::new(width, height, vec![false; width as usize * height as usize]),
        }
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

pub const GOAL_DEPTH: u8 = u8::MAX;

#[derive(Clone, Copy, Eq, Debug, Reflect)]
#[repr(u16)]
enum IntegrationCost {
    /// Blocked by an obstacle, with a depth and cost
    Blocked(u8, u8),
    /// Occupied by an agent, with a depth and cost
    Occupied(u8, u8),
    Traversable(u8),
    Goal,
}

impl Default for IntegrationCost {
    #[inline]
    fn default() -> Self {
        Self::Blocked(GOAL_DEPTH, u8::MAX)
    }
}

impl IntegrationCost {
    #[inline]
    pub const fn cost(&self) -> u8 {
        use IntegrationCost::*;
        match self {
            Blocked(d, c) => d.saturating_mul(*c),
            Occupied(d, c) => {
                if *d == GOAL_DEPTH {
                    // We don't carry the depth cost for Occupied(GOAL_DEPTH, _), as this means the goal is
                    // surrounded by agents, and we want to prefer moving towards the goal.
                    *c
                } else {
                    d.saturating_mul(*c)
                }
            }
            Traversable(c) => *c,
            Goal => 0,
        }
    }

    #[inline]
    pub const fn valid_traversal(&self, neighbor: IntegrationCost) -> bool {
        use IntegrationCost::*;
        match self {
            Blocked(_, _) => matches!(neighbor, Blocked(_, _) | Goal),
            Occupied(d, _) => {
                if *d == GOAL_DEPTH {
                    // We can transition from Occupied(GOAL_DEPTH, _) to Traversable(_) as this means the goal is
                    // surrounded by agents, and we want to continue the traversal.
                    matches!(neighbor, Traversable(_) | Occupied(_, _) | Goal)
                } else {
                    matches!(neighbor, Occupied(_, _) | Goal)
                }
            }
            Traversable(_) => true,
            Goal => true,
        }
    }

    #[inline]
    pub const fn depth_and_cost(&self) -> (u8, u8) {
        use IntegrationCost::*;
        match self {
            Blocked(d, c) => (*d, *c),
            Occupied(d, c) => (*d, *c),
            Traversable(c) => (0, *c),
            Goal => (GOAL_DEPTH, 0),
        }
    }
}

impl PartialEq for IntegrationCost {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp(other), std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for IntegrationCost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IntegrationCost {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use IntegrationCost::*;
        match (self, other) {
            // Handle comparisons where both are Blocked or both are Occupied
            (Blocked(a1, a2), Blocked(b1, b2)) | (Occupied(a1, a2), Occupied(b1, b2)) => {
                a1.cmp(b1).then_with(|| a2.cmp(b2))
            }
            // When one is Blocked/Occupied, and the other is also Blocked/Occupied but different variants
            (Blocked(_, _), Occupied(_, _)) => std::cmp::Ordering::Greater,
            (Occupied(_, _), Blocked(_, _)) => std::cmp::Ordering::Less,
            // Comparisons between Blocked/Occupied and other types
            (Blocked(_, _), _) => std::cmp::Ordering::Greater,
            (_, Blocked(_, _)) => std::cmp::Ordering::Less,
            (Occupied(_, _), _) => std::cmp::Ordering::Greater,
            (_, Occupied(_, _)) => std::cmp::Ordering::Less,
            // Direct comparison for Traversable variants
            (Traversable(a), Traversable(b)) => a.cmp(b),
            (Traversable(_), _) => std::cmp::Ordering::Greater,
            (_, Traversable(_)) => std::cmp::Ordering::Less,
            // Goal comparisons
            (Goal, Goal) => std::cmp::Ordering::Equal,
        }
    }
}

#[inline]
pub(in crate::navigation) fn build<const AGENT: Agent>(
    commands: ParallelCommands,
    mut flow_fields: Query<
        (Entity, &mut FlowField<AGENT>, &CellIndex, Option<&ExpandedFootprint<AGENT>>),
        With<Dirty<FlowField<AGENT>>>,
    >,
    obstacle_field: Res<ObstacleField>,
) {
    flow_fields.par_iter_mut().for_each(|(entity, mut flow_field, cell_index, footprint)| {
        let goals = match footprint {
            Some(ExpandedFootprint::Cells(cells)) => cells.iter().cloned().collect_vec(),
            None if let CellIndex::Valid(cell, _) = cell_index => vec![*cell],
            _ => return,
        };

        let now = std::time::Instant::now();

        flow_field.build(goals.into_iter(), &obstacle_field);

        let end = std::time::Instant::now() - now;

        info!(target: "flow_field", "{:?}::build: {:?}", AGENT, end);

        commands.command_scope(|mut c| {
            c.entity(entity).remove::<Dirty<FlowField<AGENT>>>();
        })
    });
}

pub(in crate::navigation) fn moved<const AGENT: Agent>(
    commands: ParallelCommands,
    flow_fields: Query<
        Entity,
        (
            Or<(Changed<CellIndex>, Changed<Footprint>)>,
            With<FlowField<AGENT>>,
            Without<Dirty<FlowField<AGENT>>>,
            Without<Disabled<FlowField<AGENT>>>,
        ),
    >,
) {
    flow_fields.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert(Dirty::<FlowField<AGENT>>::default());
        })
    });
}

pub(in crate::navigation) fn changed<const AGENT: Agent>(
    commands: ParallelCommands,
    flow_fields: Query<
        Entity,
        (With<FlowField<AGENT>>, Without<Dirty<FlowField<AGENT>>>, Without<Disabled<FlowField<AGENT>>>),
    >,
) {
    flow_fields.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert(Dirty::<FlowField<AGENT>>::default());
        })
    });
}

#[cfg(feature = "dev_tools")]
pub(crate) fn gizmos<const AGENT: Agent>(
    mut gizmos: Gizmos,
    layout: Res<FieldLayout>,
    flow_fields: Query<&FlowField<AGENT>>,
) {
    use crate::navigation::flow_field::layout::HALF_CELL_SIZE;

    for flow_field in &flow_fields {
        for (cell, &direction) in flow_field.iter().enumerate().map(|(i, cost)| (layout.cell_from_index(i), cost)) {
            let position = layout.position(cell).x0y();
            if let Some(direction) = direction.as_direction2d() {
                let start = position;
                let end = start + direction.x0y() * HALF_CELL_SIZE;
                gizmos.arrow(start.y_pad(), end.y_pad(), Color::WHITE);
            }
        }
    }
}
