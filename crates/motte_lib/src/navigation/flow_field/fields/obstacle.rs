use crate::{
    navigation::{
        agent::Agent,
        flow_field::{
            fields::{Cell, Field},
            footprint::{ExpandedFootprint, Footprint},
            layout::{FieldBounds, FieldLayout},
        },
        obstacle::Obstacle,
    },
    prelude::*,
};

#[derive(Resource, Clone, Deref, DerefMut, Reflect)]
pub struct ObstacleField(Field<Cost>);

impl ObstacleField {
    pub fn from_layout(field_layout: &FieldLayout) -> Self {
        Self(Field::new(field_layout.width(), field_layout.height(), vec![default(); field_layout.len()]))
    }

    #[inline(always)]
    pub fn splat(&mut self, cells: &[Cell], cost: Cost) {
        for &cell in cells {
            if !self.valid(cell) {
                continue;
            }
            self[cell] = cost
        }
    }

    #[inline]
    pub fn traversable(&self, cell: Cell, agent_radius: Agent) -> bool {
        self[cell].traversable(agent_radius)
    }

    pub fn clear(&mut self) {
        for i in 0..self.len() {
            self[i] = Cost::default()
        }
    }
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
    pub fn traversable(&self, agent_radius: Agent) -> bool {
        matches!(self, Cost::Traversable(radius) if *radius >= agent_radius)
    }
}

#[derive(Event, Reflect)]
pub struct DirtyObstacleField;

#[inline(always)]
pub(in crate::navigation) fn clear(mut obstacle_field: ResMut<ObstacleField>) {
    info!(target: "obstacle_field", "clear");

    obstacle_field.clear();
}

#[inline(always)]
pub(in crate::navigation) fn splat<const AGENT: Agent>(
    mut obstacle_field: ResMut<ObstacleField>,
    obstacles: Query<&ExpandedFootprint<AGENT>, With<Obstacle>>,
    bounds: Res<FieldBounds<AGENT>>,
) {
    for expanded_footprint in &obstacles {
        if let ExpandedFootprint::Cells(cells) = expanded_footprint {
            obstacle_field.splat(cells.as_slice(), expanded_traversable(AGENT));
        }
    }

    obstacle_field.splat(bounds.as_slice(), expanded_traversable(AGENT));
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

pub(in crate::navigation) fn changes(
    obstacles: Query<Entity, (With<Obstacle>, Changed<Footprint>)>,
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
    for (cell, cost) in obstacle_field.iter().enumerate().map(|(i, cost)| (layout.cell_from_index(i), cost)) {
        let position = layout.position(cell).x0y();
        let color = match cost {
            Cost::Blocked => Color::RED,
            Cost::Traversable(radius) if radius == &Agent::LARGEST => Color::NONE,
            Cost::Traversable(radius) if *radius < AGENT => Color::RED,
            _ => Color::NONE,
        };
        gizmos.rect(position.y_pad(), Quat::from_rotation_x(PI / 2.), Vec2::ONE / 1.5 * layout.cell_size(), color);
    }
}
