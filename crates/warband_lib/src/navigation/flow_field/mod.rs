use self::{field::Cell, footprint::Footprint, layout::FieldLayout};
use crate::{
    navigation::{agent::AgentRadius, flow_field::flow::FlowField},
    prelude::*,
};

pub mod cost;
pub mod field;
pub mod flow;
pub mod footprint;
pub mod layout;

pub struct FlowFieldPlugin;

impl Plugin for FlowFieldPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(CellIndex, Footprint, FlowField<{ AgentRadius::Small }>);

        // app.add_systems(FixedUpdate, (cost::density::update, cost::obstacle::update));
        app.add_systems(
            FixedUpdate,
            (flow::update::<{ AgentRadius::Small }>).after(cost::obstacle::update).after(cost::density::update),
        );

        app.add_systems(FixedUpdate, (footprint::agents, footprint::obstacles));
        app.add_systems(FixedUpdate, cell_index);
    }
}

#[derive(Component, Default, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub enum CellIndex {
    #[default]
    Invalid,
    Valid(Cell, usize),
}

pub fn cell_index(
    mut transforms: Query<(&mut CellIndex, &GlobalTransform), Or<(Changed<GlobalTransform>, Added<CellIndex>)>>,
    layout: Res<FieldLayout>,
) {
    transforms.par_iter_mut().for_each(|(mut cell_index, global)| {
        let cell = layout.cell(global.translation().xz());
        let index = layout.index(cell);
        let value = index.map(|index| CellIndex::Valid(cell, index)).unwrap_or(CellIndex::Invalid);

        if *cell_index != value {
            *cell_index = value;
        }
    });
}

#[cfg(feature = "debug")]
pub(crate) fn gizmos_cell_index(mut gizmos: Gizmos, agents: Query<&CellIndex>, layout: Res<FieldLayout>) {
    for cell_index in &agents {
        let CellIndex::Valid(cell, _) = cell_index else {
            continue;
        };

        let position = layout.position(*cell);
        gizmos.rect(
            position.x0y().y_pad(),
            Quat::from_rotation_x(PI / 2.),
            Vec2::ONE * layout.cell_size(),
            Color::YELLOW.with_a(1.0),
        );
    }
}
