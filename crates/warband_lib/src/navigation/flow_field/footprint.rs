use bevy_xpbd_3d::parry::na::{Const, OPoint};

use super::{field::Cell, layout::FieldLayout, CellIndex};
use crate::{
    navigation::{agent::Agent, obstacle::Obstacle},
    prelude::*,
};

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
    pub fn expand(&self, radius: usize) -> Option<impl Iterator<Item = Cell> + '_> {
        if let Footprint::Cells(cells) = self {
            return Some(cells.iter().flat_map(move |&cell| {
                (-(radius as isize)..=radius as isize).flat_map(move |dx| {
                    (-(radius as isize)..=radius as isize).filter_map(move |dy| {
                        let x = (cell.x() as isize + dx) as usize;
                        let y = (cell.y() as isize + dy) as usize;
                        let expanded_cell = Cell::new(x, y);
                        if cell.manhattan(expanded_cell) <= radius {
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

            let agent_radius: f32 = agent.radius().into();
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
    mut obstacles: Query<(&mut Footprint, &Obstacle, &ColliderAabb), Changed<Obstacle>>,
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

        // FIXME: annoying glam to nalgebra conversion
        let shape = shape.iter().map(|v| OPoint::<f32, Const<2>>::from_slice(&[v.x, v.y])).collect::<Vec<_>>();

        *footprint = Footprint::Cells(
            (min_cell.x()..=max_cell.x())
                .step_by(layout.cell_size() as usize)
                .flat_map(|x| {
                    (min_cell.y()..=max_cell.y()).step_by(layout.cell_size() as usize).map(move |y| Cell::new(x, y))
                })
                .filter(|&cell| {
                    let world_position = layout.position(cell);
                    parry2d::utils::point_in_poly2d(&OPoint::from_slice(&[world_position.x, world_position.y]), &shape)
                        && layout.index(cell).is_some()
                })
                .collect(),
        );
    });
}

#[cfg(feature = "debug")]
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