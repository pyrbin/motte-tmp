use crate::{
    navigation::{
        agent::Agent,
        flow_field::{
            field::{Cell, Field},
            footprint::Footprint,
            layout::FieldLayout,
        },
    },
    prelude::*,
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

    #[inline]
    pub fn update(
        &mut self,
        layout: &FieldLayout,
        position: Vec2,
        radius: f32,
        mass: f32,
        velocity: Vec2,
        footprint: &[Cell],
    ) {
        for (index, _, density) in
            footprint.iter().filter_map(|&cell| layout.index(cell).map(|i| (i, cell))).map(|(i, cell)| {
                let cell_center = layout.position(cell);
                let distance = position.distance(cell_center);
                let density = (1.0 - distance / radius).max(0.0).min(1.0); // Bilinear interpolation
                (i, cell, density)
            })
        {
            self.density[index] = (density * mass).max(self.density[index]);
            self.avg_velocity[index] += density * velocity * mass;
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

    pub fn avg_velocity(&self) -> &Field<Vec2> {
        &self.avg_velocity
    }

    pub fn density(&self) -> &Field<f32> {
        &self.density
    }
}

pub fn update(
    mut cost_fields: ResMut<super::CostFields>,
    agents: Query<(&Agent, &Footprint, &GlobalTransform, &LinearVelocity)>,
    layout: Res<FieldLayout>,
) {
    let density_field = &mut cost_fields.density;

    density_field.clear();

    for (agent, footprint, transform, velocity) in &agents {
        if let Footprint::Cells(cells) = footprint {
            density_field.update(
                &layout,
                transform.translation().xz(),
                agent.radius().into(),
                agent.mass(),
                velocity.xz(),
                cells,
            );
        }
    }

    density_field.normalize();
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
        let color = Color::PURPLE.with_a(density);
        gizmos.rect(position.y_pad(), Quat::from_rotation_x(PI / 2.), Vec2::ONE / 2.0 * layout.cell_size(), color);

        let end = position + avg_velocity.x0y() * (layout.cell_size() / 2.0);
        gizmos.arrow(position.y_pad(), end.y_pad(), Color::PURPLE);
    }
}
