use std::iter::once;

use crate::{
    app_state::AppState,
    flowfield::field::{Cost, CostField, FlowField, IntegrationField},
    prelude::*,
};

pub mod field;

pub struct FlowFieldPlugin;

impl Plugin for FlowFieldPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(CostField, IntegrationField, FlowField, FlowTarget, FlowFieldConfig);

        app.add_systems(OnEnter(AppState::InGame), setup);
        app.add_systems(Update, build_flow_field.run_if(in_state(AppState::InGame)));
        app.add_systems(Update, debug_flowfield.run_if(in_state(AppState::InGame)));
    }
}

#[derive(Component, Clone, Reflect)]
pub struct FlowTarget(pub field::Cell);

#[derive(Component, Clone, Reflect)]
pub struct FlowFieldConfig {
    pub trigger: bool,
}

fn setup(mut commands: Commands) {
    const SIZE: usize = 50;

    // generate a costfield that has a rectangular prison around zero
    let mut cost_field = CostField::new(SIZE);
    for (i, cost) in cost_field.iter_mut().enumerate() {
        // set middle tiles as impassable
        if i > 8 && i < 12 {
            *cost = Cost::Impassable
        }
    }

    let _cost_field = commands.spawn((
        Name::new("flow field"),
        FlowFieldConfig { trigger: false },
        cost_field,
        IntegrationField::new(SIZE),
        FlowField::new(SIZE),
        FlowTarget(field::Cell::new(0, 0)),
        SpatialBundle::default(),
    ));
}

fn build_flow_field(
    mut fields: Query<
        (&FlowTarget, &mut FlowField, &mut IntegrationField, &CostField),
        Or<(Changed<CostField>, Changed<FlowFieldConfig>, Changed<FlowTarget>)>,
    >,
) {
    for (target, mut flow_field, mut integration_field, cost_field) in fields.iter_mut() {
        // profile how long this takes
        const NUM_ITERATIONS: usize = 50;

        let start = std::time::Instant::now();
        for _ in 0..NUM_ITERATIONS {
            integration_field.build(once(target.0), cost_field);
            flow_field.build(&integration_field);
        }
        let end = start.elapsed();
        let time = end / NUM_ITERATIONS as u32;
        info!("partial flow took {:?} ({} iterations)", time, NUM_ITERATIONS);

        let start = std::time::Instant::now();
        for _ in 0..NUM_ITERATIONS {
            flow_field.build_full(once(target.0), cost_field);
        }
        let end = start.elapsed();
        let time = end / NUM_ITERATIONS as u32;
        info!("full flow took {:?} ({} iterations)", time, NUM_ITERATIONS);
    }
}

fn debug_flowfield(
    mut gizmos: Gizmos,
    fields: Query<(&Transform, &FlowTarget, &FlowField, &IntegrationField, &CostField)>,
) {
    for (_transform, target, flow, int, costs) in &fields {
        // Draw the cost field
        for ((i, cost), flow) in int.iter().enumerate().zip(flow.iter()) {
            let cell_size = 1.0;
            let cell_pos = costs.cell(i).as_vec2() - costs.size() as f32 / 2.0;

            // draw cell using 4 gizmos.line
            let top_left = cell_pos.x_y(0.01);
            let _top_right = (cell_pos + Vec2::X * cell_size).x_y(0.01);
            let _bottom_left = (cell_pos + Vec2::Y * cell_size).x_y(0.01);
            let _bottom_right = (cell_pos + Vec2::ONE * cell_size).x_y(0.01);
            let center = top_left + Vec3::new(0.5, 0.0, 0.5);

            let flow_dir = flow.normalize_or_zero() * 0.25;
            gizmos.line(
                center,
                center + flow_dir.x_y(0.01),
                if cost == &u16::MAX { Color::TOMATO } else { Color::TURQUOISE },
            );

            if i == costs.index(target.0) {
                gizmos.circle(center, Vec3::Y, cell_size / 2.0, Color::PINK);

                for neighbor in costs.neighbors8(target.0) {
                    let neighbor_pos = neighbor.as_vec2() - costs.size() as f32 / 2.0;
                    let neighbor_center = neighbor_pos.x_y(0.01) + Vec3::new(0.5, 0.0, 0.5);
                    gizmos.line(center, neighbor_center, Color::PINK);
                }
            } else if cost == &u16::MAX {
                gizmos.circle(center, Vec3::Y, cell_size / 2.0, Color::RED);
            }
        }
    }
}
