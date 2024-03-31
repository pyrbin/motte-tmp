use crate::{
    navigation::{
        agent::{Agent, TargetReached},
        flow_field::{
            fields::{Cell, Field},
            footprint::{ExpandedFootprint, Footprint},
            layout::{FieldBounds, FieldLayout},
            pathing::Goal,
        },
        obstacle::Obstacle,
    },
    prelude::*,
};

#[derive(Resource, Clone, Reflect)]
pub struct ObstacleField {
    cost: Field<Cost>,
    occupant: Field<Occupant>,
}

impl std::ops::Deref for ObstacleField {
    type Target = Field<Cost>;
    fn deref(&self) -> &Self::Target {
        &self.cost
    }
}

impl ObstacleField {
    pub fn from_layout(field_layout: &FieldLayout) -> Self {
        Self {
            cost: Field::new(field_layout.width(), field_layout.height(), vec![default(); field_layout.len()]),
            occupant: Field::new(field_layout.width(), field_layout.height(), vec![default(); field_layout.len()]),
        }
    }

    #[inline(always)]
    pub fn splat(&mut self, cells: &[Cell], cost: Cost, occupant: Occupant) {
        for &cell in cells {
            if !self.valid(cell) {
                continue;
            }
            self.cost[cell] = cost;
            self.occupant[cell] = occupant;
        }
    }

    #[inline(always)]
    pub fn traversable(&self, cell: Cell, agent_radius: Agent) -> bool {
        self.cost[cell].traversable(agent_radius)
    }

    pub fn occupant(&self, cell: Cell) -> Occupant {
        self.occupant[cell]
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        for i in 0..self.len() {
            self.cost[i] = Cost::default();
            self.occupant[i] = Occupant::Empty;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Reflect, Default)]
#[repr(u8)]
pub enum Occupant {
    #[default]
    Empty,
    Obstacle,
    Agent,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Reflect)]
#[repr(u8)]
pub enum Cost {
    Blocked,
    Traversable(Agent),
}

impl Default for Cost {
    fn default() -> Self {
        Cost::Traversable(Agent::LARGEST)
    }
}

impl Cost {
    pub fn traversable(&self, agent: Agent) -> bool {
        matches!(self, Cost::Traversable(a) if *a >= agent)
    }
}

#[derive(Event, Reflect)]
pub struct DirtyObstacleField;

pub type ObstacleFilter = Or<(With<Obstacle>, (With<Agent>, Without<Goal>), (With<Agent>, With<TargetReached>))>;

#[inline(always)]
pub(in crate::navigation) fn clear(mut obstacle_field: ResMut<ObstacleField>) {
    info!(target: "obstacle_field", "clear");
    obstacle_field.clear();
}

#[inline(always)]
pub(in crate::navigation) fn splat<const AGENT: Agent>(
    mut obstacle_field: ResMut<ObstacleField>,
    obstacles: Query<(&ExpandedFootprint<AGENT>, Has<Agent>), ObstacleFilter>,
    bounds: Res<FieldBounds<AGENT>>,
) {
    for (expanded_footprint, is_agent) in &obstacles {
        if let ExpandedFootprint::Cells(cells) = expanded_footprint {
            obstacle_field.splat(
                &cells,
                expanded_traversable(AGENT),
                if is_agent { Occupant::Agent } else { Occupant::Obstacle },
            );
        }
    }
    obstacle_field.splat(&bounds, expanded_traversable(AGENT), Occupant::Obstacle);
}

/// Cost of cells that exist in [`ExpandedFootprint<{ `agent` }>`].
#[inline(always)]
const fn expanded_traversable(agent: Agent) -> Cost {
    match agent {
        Agent::Small => Cost::Blocked,
        Agent::Medium => Cost::Traversable(Agent::Small),
        Agent::Large => Cost::Traversable(Agent::Medium),
        Agent::Huge => Cost::Traversable(Agent::Large),
    }
}

pub type ObstacleFilterChanged = (ObstacleFilter, Changed<Footprint>);

pub(in crate::navigation) fn changes(
    obstacles: Query<Entity, Changed<Footprint>>,
    mut event: EventWriter<DirtyObstacleField>,
) {
    if obstacles.is_empty() {
        return;
    }
    event.send(DirtyObstacleField);
}

#[cfg(feature = "dev_tools")]
pub(crate) fn gizmos<const AGENT: Agent>(
    mut gizmos: Gizmos,
    layout: Res<FieldLayout>,
    obstacle_field: Res<ObstacleField>,
) {
    use crate::navigation::flow_field::layout::CELL_SIZE_F32;

    for (cell, cost) in obstacle_field.iter().enumerate().map(|(i, cost)| (layout.cell_from_index(i), cost)) {
        let position = layout.position(cell).x0y();
        let color = match cost {
            Cost::Blocked => Color::RED,
            Cost::Traversable(radius) if radius == &Agent::LARGEST => Color::NONE,
            Cost::Traversable(radius) if *radius < AGENT => Color::RED,
            _ => Color::NONE,
        };
        gizmos.rect(position.y_pad(), Quat::from_rotation_x(PI / 2.), Vec2::ONE / 1.5 * CELL_SIZE_F32, color);
    }
}
