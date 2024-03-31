use self::{fields::Cell, footprint::Footprint, layout::FieldLayout};
use crate::{
    app_state::AppState,
    navigation::{
        agent::Agent,
        flow_field::{
            cache::FlowFieldCache,
            fields::{
                flow::FlowField,
                obstacle::{DirtyObstacleField, ObstacleField},
            },
            footprint::ExpandedFootprint,
            layout::FieldBounds,
        },
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
    DetectChanges,
    Splat,
    Build,
    Pathing,
    Cleanup,
}

pub struct FlowFieldPlugin;

impl Plugin for FlowFieldPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(CellIndex, Footprint, DirtyObstacleField);

        app.configure_sets(
            FixedUpdate,
            (
                FlowFieldSystems::Setup,
                FlowFieldSystems::Maintain,
                FlowFieldSystems::DetectChanges,
                FlowFieldSystems::Splat.run_if(on_event::<DirtyObstacleField>()),
                FlowFieldSystems::Build,
                FlowFieldSystems::Pathing,
                FlowFieldSystems::Cleanup,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        );

        app.add_event::<DirtyObstacleField>();

        app.add_systems(
            FixedUpdate,
            (cell_index, (footprint::agents, footprint::obstacles)).chain().in_set(FlowFieldSystems::Maintain),
        );

        app.add_systems(
            FixedUpdate,
            (apply_deferred, fields::obstacle::changes).chain().in_set(FlowFieldSystems::DetectChanges),
        );

        app.add_systems(
            FixedUpdate,
            (
                fields::obstacle::clear,
                // Would like to put this into [`FlowFieldAgentPlugin`], but not sure how to ensure the order.
                // The order is important, should be 'splat' from largest to smallest.
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
        app_register_types!(FlowField<AGENT>, FlowFieldCache<AGENT>, FieldBounds<AGENT>, ExpandedFootprint<AGENT>);

        app.insert_resource(FlowFieldCache::<AGENT>::default());
        app.insert_resource(FieldBounds::<AGENT>::default());

        app.add_systems(
            FixedUpdate,
            (cache::spawn::<AGENT>, cache::insert::<AGENT>, footprint::setup::<AGENT>).in_set(FlowFieldSystems::Setup),
        );
        app.add_systems(
            FixedUpdate,
            (
                cache::tick::<AGENT>,
                cache::despawn::<AGENT>,
                layout::field_bounds::<AGENT>,
                footprint::expand::<AGENT>.after(footprint::agents).after(footprint::obstacles),
            )
                .in_set(FlowFieldSystems::Maintain),
        );
        app.add_systems(
            FixedUpdate,
            (
                (
                    fields::flow::moved::<AGENT>,
                    fields::flow::changed::<AGENT>.run_if(resource_exists_and_changed::<ObstacleField>),
                ),
                apply_deferred,
                fields::flow::build::<AGENT>.in_set(FlowFieldSystems::Build),
                pathing::direction::<AGENT>.in_set(FlowFieldSystems::Pathing),
            )
                .chain(),
        );
        app.add_systems(FixedUpdate, (cache::despawn::<AGENT>).in_set(FlowFieldSystems::Cleanup));
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

#[cfg(feature = "dev_tools")]
pub(crate) fn gizmos_cell_index(mut gizmos: Gizmos, agents: Query<&CellIndex>, layout: Res<FieldLayout>) {
    use self::layout::CELL_SIZE_F32;

    for cell_index in &agents {
        let CellIndex::Valid(cell, _) = cell_index else {
            continue;
        };

        let position = layout.position(*cell);
        gizmos.rect(
            position.x0y().y_pad(),
            Quat::from_rotation_x(PI / 2.),
            Vec2::ONE * CELL_SIZE_F32,
            Color::YELLOW.with_a(1.0),
        );
    }
}
