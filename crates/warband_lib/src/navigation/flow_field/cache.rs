// TODO: Cache<R: Radius>
// Has two modes, either duration till expiration or timer trigger,

use super::{fields::flow::FlowField, layout::FieldLayout, pathing::Goal, CellIndex};
use crate::{
    navigation::agent::{Agent, AgentType},
    prelude::*,
};

pub const CACHE_TTL_SEC: f32 = 30.0;

#[derive(Resource, Default, Deref, DerefMut, Reflect)]
pub struct FlowFieldCache<const AGENT: Agent>(HashMap<Goal, (Entity, Timer)>);

#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
pub(super) enum Cached {
    Managed,
    Unmanaged,
}

pub(super) fn spawn<const AGENT: Agent>(
    mut commands: Commands,
    agents: Query<&Goal, (Or<(Changed<Goal>, Changed<AgentType<AGENT>>)>, With<AgentType<AGENT>>)>,
    layout: Res<FieldLayout>,
    mut cache: ResMut<FlowFieldCache<AGENT>>,
) {
    for goal in &agents {
        match cache.get_mut(goal) {
            Some((_, timer)) => {
                timer.reset();
            }
            None if let Goal::Cell(cell) = goal => {
                let flow_field = commands
                    .spawn((
                        Name::new(format!("FlowField {:?}", goal)),
                        FlowField::<AGENT>::from_layout(&layout),
                        SpatialBundle { transform: layout.position(*cell).x0y().into_transform(), ..default() },
                        CellIndex::default(),
                        Cached::Managed,
                        Dirty::<FlowField<AGENT>>::default(),
                    ))
                    .id();

                cache.insert_unique_unchecked(*goal, (flow_field, Timer::from_seconds(CACHE_TTL_SEC, TimerMode::Once)));
            }
            None if let Goal::Entity(entity) = goal => {
                commands.entity(*entity).insert((
                    FlowField::<AGENT>::from_layout(&layout),
                    CellIndex::default(),
                    Cached::Unmanaged,
                    Dirty::<FlowField<AGENT>>::default(),
                ));

                cache.insert_unique_unchecked(*goal, (*entity, Timer::from_seconds(CACHE_TTL_SEC, TimerMode::Once)));
            }
            _ => {}
        }
    }
}

pub(super) fn insert<const AGENT: Agent>(
    mut commands: Commands,
    mut cache: ResMut<FlowFieldCache<AGENT>>,
    flow_fields: Query<Entity, (Added<FlowField<AGENT>>, Without<Cached>, Without<Disabled<FlowField<AGENT>>>)>,
) {
    for entity in &flow_fields {
        cache.insert_unique_unchecked(
            Goal::Entity(entity),
            (entity, Timer::from_seconds(CACHE_TTL_SEC, TimerMode::Once)),
        );
        commands.entity(entity).insert(Cached::Unmanaged);
    }
}

pub(super) fn tick<const AGENT: Agent>(
    mut commands: Commands,
    mut cache: ResMut<FlowFieldCache<AGENT>>,
    time: Res<Time>,
) {
    for (_, (entity, _)) in cache.0.extract_if(|_, (_, timer)| timer.tick(time.delta()).just_finished()) {
        commands.entity(entity).insert(Disabled::<FlowField<AGENT>>::default());
    }
}

pub(super) fn despawn<const AGENT: Agent>(
    mut commands: Commands,
    flow_fields: Query<(Entity, &Cached), (With<FlowField<AGENT>>, With<Disabled<FlowField<AGENT>>>)>,
) {
    for (entity, cached) in &flow_fields {
        match cached {
            Cached::Managed => commands.entity(entity).despawn_recursive(),
            Cached::Unmanaged => {
                commands
                    .entity(entity)
                    .remove::<Cached>()
                    .remove::<FlowField<AGENT>>()
                    .remove::<Disabled<FlowField<AGENT>>>()
                    .remove::<Dirty<FlowField<AGENT>>>();
            }
        }
    }
}
