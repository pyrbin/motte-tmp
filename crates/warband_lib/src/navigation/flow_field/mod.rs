use self::{fields::Cell, footprint::Footprint, layout::FieldLayout};
use crate::{
    app_state::AppState,
    navigation::{
        agent::Agent,
        flow_field::{cache::FlowFieldCache, fields::flow::FlowField},
    },
    prelude::*,
};

pub mod cache;
pub mod fields;
pub mod footprint;
pub mod layout;
pub mod pathing;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlowFieldSystems {
    Setup,
    Maintain,
    Splat,
    Build,
    Seek,
    Cleanup,
}

pub struct FlowFieldPlugin;

impl Plugin for FlowFieldPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(CellIndex, Footprint);

        app.configure_sets(FixedUpdate, (FlowFieldSystems::Setup).chain().run_if(in_state(AppState::InGame)));
        app.configure_sets(
            FixedPostUpdate,
            (FlowFieldSystems::Maintain, FlowFieldSystems::Splat, FlowFieldSystems::Build, FlowFieldSystems::Seek)
                .chain()
                .run_if(in_state(AppState::InGame)),
        );
        app.configure_sets(FixedLast, (FlowFieldSystems::Cleanup).chain().run_if(in_state(AppState::InGame)));

        app.add_systems(
            FixedPostUpdate,
            (cell_index, (footprint::agents, footprint::obstacles)).chain().in_set(FlowFieldSystems::Maintain),
        );

        app.add_systems(
            FixedPostUpdate,
            (
                fields::obstacle::clear,
                // Would like to put this into [`FlowFieldAgentPlugin`], but not sure how to ensure the order.
                // The order is important, should be splatting from large to small.
                fields::obstacle::splat::<{ Agent::Huge }>,
                fields::obstacle::splat::<{ Agent::Large }>,
                fields::obstacle::splat::<{ Agent::Medium }>,
                fields::obstacle::splat::<{ Agent::Small }>,
            )
                .chain()
                .in_set(FlowFieldSystems::Splat),
        );
    }
}

pub struct FlowFieldAgentPlugin<const AGENT: Agent>;

impl<const AGENT: Agent> Plugin for FlowFieldAgentPlugin<AGENT> {
    fn build(&self, app: &mut App) {
        app_register_types!(FlowField<AGENT>, FlowFieldCache<AGENT>);

        app.insert_resource(FlowFieldCache::<AGENT>::default());

        app.add_systems(
            FixedUpdate,
            (cache::spawn::<AGENT>, cache::insert::<AGENT>, footprint::setup_expand::<AGENT>)
                .in_set(FlowFieldSystems::Setup),
        );
        app.add_systems(
            FixedPostUpdate,
            (
                cache::tick::<AGENT>,
                cache::despawn::<AGENT>,
                footprint::expand::<AGENT>.after(footprint::agents).after(footprint::obstacles),
            )
                .in_set(FlowFieldSystems::Maintain),
        );
        app.add_systems(
            FixedPostUpdate,
            (
                fields::flow::build::<AGENT>.in_set(FlowFieldSystems::Build),
                pathing::seek::<AGENT>.in_set(FlowFieldSystems::Seek),
            ),
        );
        app.add_systems(FixedLast, (cache::despawn::<AGENT>).in_set(FlowFieldSystems::Cleanup));
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
