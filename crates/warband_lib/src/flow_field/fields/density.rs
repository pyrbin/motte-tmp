use crate::{
    flow_field::{
        field::{Direction, Field},
        layout::FieldLayout,
    },
    navigation::agent::Agent,
    prelude::*,
    util::math::saturate,
};

#[derive(Clone, Reflect)]
pub struct DensityField {
    density: Field<f32>,
    avg_velocity: Field<Vec2>,
}

impl DensityField {
    pub fn from_layout(field_layout: &FieldLayout) -> Self {
        Self {
            density: Field::new(field_layout.width(), field_layout.height(), vec![0.0; field_layout.len()]),
            avg_velocity: Field::new(field_layout.width(), field_layout.height(), vec![Vec2::ZERO; field_layout.len()]),
        }
    }

    pub fn update(&mut self, field_layout: &FieldLayout, position: Vec2, velocity: Vec2) {
        let cell = field_layout.cell(position);
        let delta = saturate(position - (cell.as_vec2() + 0.5));
        let inverse_delta = 1.0 - delta;
        for (index, direction) in cell
            .sample_neighbors(&[Direction::East, Direction::NorthEast, Direction::North])
            .chain(std::iter::once((cell, Direction::None)))
            .filter_map(|(n, d)| field_layout.index(n).map(|i| (i, d)))
        {
            const EXPONENT: f32 = 0.3;

            let density = (match direction {
                Direction::None => inverse_delta.x.min(inverse_delta.y),
                Direction::East => delta.x.min(inverse_delta.y),
                Direction::NorthEast => delta.x.min(delta.y),
                Direction::North => inverse_delta.x.min(delta.y),
                _ => unreachable!("invalid direction"),
            })
            .powf(EXPONENT);

            self.density[index] += density;
            self.avg_velocity[index] += velocity * density;
        }
    }

    pub fn normalize(&mut self) {
        for i in 0..self.density.len() {
            if self.density[i] > 0.0 {
                self.avg_velocity[i] /= self.density[i];
            }
        }
    }

    pub fn clear(&mut self) {
        for i in 0..self.density.len() {
            self.density[i] = 0.0;
            self.avg_velocity[i] = Vec2::ZERO;
        }
    }
}

pub fn update(
    agents: Query<(&GlobalTransform, &LinearVelocity), With<Agent>>,
    layout: Res<FieldLayout>,
    mut cost_fields: ResMut<super::CostFields>,
) {
    cost_fields.density.clear();

    for (transform, velocity) in &agents {
        cost_fields.density.update(&layout, transform.translation().xz(), velocity.xz());
    }

    cost_fields.density.normalize();
}

#[cfg(feature = "debug")]
pub(crate) fn gizmos(mut gizmos: Gizmos, layout: Res<FieldLayout>, cost_fields: Res<super::CostFields>) {
    let density_field = &cost_fields.density;
    for (cell, (&density, &avg_velocity)) in density_field
        .density
        .iter()
        .zip(density_field.avg_velocity.iter())
        .enumerate()
        .map(|(i, d)| (layout.cell_from_index(i), d))
    {
        let position = layout.position(cell).x0y();
        // density
        let density = (density - 0.32) / (1.6 - 0.32);
        let color = Color::rgba(density, density, density, 1.0);
        gizmos.rect(position.y_pad(), Quat::from_rotation_x(PI / 2.), Vec2::ONE / 2.0 * layout.cell_size(), color);

        // avg velocity
        let end = position + avg_velocity.x0y() * (layout.cell_size() / 2.0);
        gizmos.arrow(position.y_pad(), end.y_pad(), Color::RED);
    }
}
