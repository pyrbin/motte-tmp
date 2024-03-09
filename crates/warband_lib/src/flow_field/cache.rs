use super::{flow::Rebuild, goal::Goal, CellIndex, FieldLayout, FlowField};
use crate::prelude::*;

pub const DEFAULT_CACHE_TTL_SEC: f32 = 30.0;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct FlowFieldCache(HashMap<Goal, FlowFieldCacheEntry>);

#[derive(Component, Reflect)]
pub(super) struct TrackedByCache;

#[derive(Debug, Reflect)]
pub struct FlowFieldCacheEntry {
    pub field: Entity,
    pub ttl: Timer,
}

impl Default for FlowFieldCacheEntry {
    fn default() -> Self {
        Self { field: Entity::from_raw(0), ttl: Timer::from_seconds(DEFAULT_CACHE_TTL_SEC, TimerMode::Once) }
    }
}

pub(super) fn detect(
    mut commands: Commands,
    agents: Query<&Goal, Changed<Goal>>,
    field_layout: Res<FieldLayout>,
    mut cache: ResMut<FlowFieldCache>,
) {
    for goal in &agents {
        match cache.get_mut(goal) {
            Some(entry) => {
                entry.ttl.reset();
            }
            None => {
                if let Goal::Cell(cell) = goal {
                    let field = commands
                        .spawn((
                            Name::new(format!("FlowField {:?}", goal)),
                            FlowField::default(),
                            // TODO: doesn't currently handle field resize(s).
                            SpatialBundle {
                                transform: field_layout.cell_to_world(*cell).into_transform(),
                                ..default()
                            },
                            CellIndex::default(),
                            TrackedByCache,
                            Dirty::<FlowField>::default(),
                        ))
                        .id();

                    cache.insert_unique_unchecked(
                        *goal,
                        FlowFieldCacheEntry { field, ttl: Timer::from_seconds(DEFAULT_CACHE_TTL_SEC, TimerMode::Once) },
                    );
                }
            }
        }
    }
}

pub(super) fn insert(
    mut commands: Commands,
    mut cache: ResMut<FlowFieldCache>,
    flow_fields: Query<Entity, (Added<FlowField>, Without<TrackedByCache>, Without<Deactivated<FlowField>>)>,
) {
    for entity in &flow_fields {
        cache.insert_unique_unchecked(
            Goal::Entity(entity),
            FlowFieldCacheEntry { field: entity, ttl: Timer::from_seconds(DEFAULT_CACHE_TTL_SEC, TimerMode::Once) },
        );
        commands.entity(entity).insert(TrackedByCache);
    }
}

pub(super) fn lifetime(mut commands: Commands, mut cache: ResMut<FlowFieldCache>, time: Res<Time>) {
    for (_, entry) in cache.0.extract_if(|_, entry| entry.ttl.tick(time.delta()).just_finished()) {
        commands.entity(entry.field).remove::<TrackedByCache>().insert(Deactivated::<FlowField>::default());
    }
}

pub(super) fn despawn(
    mut commands: Commands,
    flow_fields: Query<Entity, (With<FlowField>, With<Deactivated<FlowField>>, Without<Rebuild>)>,
) {
    for entity in &flow_fields {
        commands.entity(entity).despawn_recursive();
    }
}
