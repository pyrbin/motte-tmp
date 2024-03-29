use super::{fields::Cell, layout::FieldLayout, CellIndex};
use crate::{
    navigation::{agent::Agent, obstacle::Obstacle},
    prelude::*,
    utils::math::point_in_poly2d,
};

/// Footprint of an entity on the field.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub enum Footprint {
    #[default]
    Empty,
    Cells(SmallVec<[Cell; 8]>),
}

impl Footprint {
    pub fn empty(&self) -> bool {
        matches!(self, Footprint::Empty)
    }

    #[inline]
    pub fn cells(&self) -> Option<&[Cell]> {
        if let Footprint::Cells(cells) = self {
            return Some(cells);
        }
        None
    }

    #[inline]
    pub fn expand(&self, radius: usize) -> Option<impl Iterator<Item = Cell> + '_> {
        debug_assert!(radius > 0);
        if let Footprint::Cells(cells) = self {
            return Some(cells.iter().flat_map(move |&cell| {
                (-(radius as isize)..=radius as isize).flat_map(move |dx| {
                    (-(radius as isize)..=radius as isize).filter_map(move |dy| {
                        let x = (cell.x() as isize + dx) as usize;
                        let y = (cell.y() as isize + dy) as usize;
                        let expanded_cell = Cell::new(x, y);
                        if cell.coordinate_distance(expanded_cell) <= radius {
                            Some(expanded_cell)
                        } else {
                            None
                        }
                    })
                })
            }));
        }
        None
    }
}

pub(super) fn agents(
    mut agents: Query<
        (&mut Footprint, &Agent, &CellIndex, &GlobalTransform),
        Or<(Changed<CellIndex>, Changed<GlobalTransform>, Changed<Agent>)>,
    >,
    layout: Res<FieldLayout>,
) {
    agents.par_iter_mut().for_each(|(mut footprint, agent, cell_index, global_transform)| match cell_index {
        CellIndex::Invalid => {
            if !footprint.empty() {
                *footprint = Footprint::Empty;
            }
        }
        CellIndex::Valid(_, _) => {
            let layout = *layout;
            let border_radius = layout.cell_size() / 4.0;

            let agent_radius: f32 = agent.radius();
            let agent_position = global_transform.translation().xz();

            let min_cell = layout.cell(Vec2::new(
                agent_position.x - (agent_radius + border_radius),
                agent_position.y - (agent_radius + border_radius),
            ));
            let max_cell = layout.cell(Vec2::new(
                agent_position.x + (agent_radius + border_radius),
                agent_position.y + (agent_radius + border_radius),
            ));

            *footprint = Footprint::Cells(
                (min_cell.x()..=max_cell.x())
                    .step_by(layout.cell_size() as usize)
                    .flat_map(|x| {
                        (min_cell.y()..=max_cell.y()).step_by(layout.cell_size() as usize).map(move |y| Cell::new(x, y))
                    })
                    .filter(|cell| {
                        agent_position.distance_squared(layout.position(*cell)) <= (agent_radius * agent_radius)
                    })
                    .collect(),
            );
        }
    });
}

pub(super) fn obstacles(
    mut obstacles: Query<(&mut Footprint, &Obstacle, &ColliderAabb), (Changed<Obstacle>, Without<Agent>)>,
    layout: Res<FieldLayout>,
) {
    obstacles.par_iter_mut().for_each(|(mut footprint, obstacle, aabb)| {
        let Obstacle::Shape(shape) = obstacle else {
            if !footprint.empty() {
                *footprint = Footprint::Empty;
            }
            return;
        };

        let min_cell = layout.cell(aabb.min.xz());
        let max_cell = layout.cell(aabb.max.xz());

        *footprint = Footprint::Cells(
            (min_cell.x()..=max_cell.x())
                .step_by(layout.cell_size() as usize)
                .flat_map(|x| {
                    (min_cell.y()..=max_cell.y()).step_by(layout.cell_size() as usize).map(move |y| Cell::new(x, y))
                })
                .filter(|&cell| layout.index(cell).is_some() && point_in_poly2d(layout.position(cell), shape))
                .collect(),
        );
    });
}

/// A [`Footprint`] expanded to size how given [`Agent`] views it when on the field.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub enum ExpandedFootprint<const AGENT: Agent> {
    #[default]
    Empty,
    Cells(SmallVec<[Cell; 8]>),
}

pub(super) fn setup_expand<const AGENT: Agent>(
    commands: ParallelCommands,
    agents: Query<Entity, Added<Footprint>>,
    mut removed: RemovedComponents<ExpandedFootprint<AGENT>>,
) {
    agents.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert(ExpandedFootprint::<AGENT>::default());
        })
    });

    for entity in &mut removed.read() {
        commands.command_scope(|mut c| {
            if let Some(mut commands) = c.get_entity(entity) {
                commands.remove::<ExpandedFootprint<AGENT>>();
            }
        });
    }
}

pub(super) fn expand<const AGENT: Agent>(
    mut footprints: Query<(&Footprint, &mut ExpandedFootprint<AGENT>), Changed<Footprint>>,
) {
    let expansion = AGENT.radius().ceil();
    footprints.par_iter_mut().for_each(|(footprint, mut expanded_footprint)| {
        if expansion <= 0.0 {
            let Footprint::Cells(cells) = footprint else {
                *expanded_footprint = ExpandedFootprint::Empty;
                return;
            };
            *expanded_footprint = ExpandedFootprint::Cells(cells.clone());
        }

        let Some(cells) = footprint.expand(AGENT.radius().ceil() as usize) else {
            *expanded_footprint = ExpandedFootprint::Empty;
            return;
        };
        *expanded_footprint = ExpandedFootprint::Cells(cells.collect());
    })
}

#[cfg(feature = "dev_tools")]
pub(crate) fn gizmos(mut gizmos: Gizmos, footprints: Query<&Footprint>, layout: Res<FieldLayout>) {
    for footprint in &footprints {
        let Footprint::Cells(cells) = footprint else {
            continue;
        };

        for cell in cells {
            let position = layout.position(*cell);
            gizmos.rect(
                position.x0y().y_pad(),
                Quat::from_rotation_x(PI / 2.),
                Vec2::ONE * layout.cell_size(),
                Color::CYAN,
            );
        }
    }
}
