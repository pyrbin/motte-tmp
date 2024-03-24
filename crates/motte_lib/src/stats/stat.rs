use std::marker::PhantomData;

use bevy::reflect::TypePath;

use super::{
    modifier::ModifierPlugin,
    pool::{self, Current, PoolBundle},
};
use crate::{
    prelude::*,
    stats::{modifier, pool::DirtyCurrent, StatSystem},
};

pub struct StatPlugin<S: Stat>
where
    S: Component + GetTypeRegistration,
{
    pub clamp_value: ClampValue,
    // TODO: Implement more configuration options, like pool value clamp, etc.
    _marker: PhantomData<S>,
}

impl<S: Stat> Plugin for StatPlugin<S>
where
    S: Component + GetTypeRegistration,
{
    fn build(&self, app: &mut App) {
        app_register_types!(Current<S>, DirtyStat<S>, DirtyCurrent<S>, S);

        app.add_plugins(ModifierPlugin::<S, S>::default());
        app.add_systems(
            PostUpdate,
            (dirty_on_added::<S>, modifier::modifier_target_changed::<S>).in_set(StatSystem::Dirty),
        );
        app.add_systems(
            PostUpdate,
            (pool::dirty_current::<S>.before(reset_dirty::<S>), reset_dirty::<S>, pool::clamp_current::<S>)
                .in_set(StatSystem::Reset),
        );

        app.add_systems(PostUpdate, (cleanup_dirty::<S>, pool::cleanup_dirty_current::<S>).in_set(StatSystem::Cleanup));

        if !matches!(self.clamp_value, ClampValue::None) {
            let clamp_value = self.clamp_value;
            app.add_systems(
                PostUpdate,
                (move |mut stats: Query<&mut S, Changed<S>>| {
                    for mut stat in &mut stats {
                        let value: f32 = stat.value();
                        let (min, max) = match clamp_value {
                            ClampValue::AboveZero => (0.0, value.max(0.0)),
                            ClampValue::Min(min) => (min, value.max(min)),
                            ClampValue::Max(max) => (value.min(max), max),
                            ClampValue::MinMax(min, max) => (min, max),
                            _ => continue,
                        };
                        *stat.value_mut() = value.clamp(min, max);
                    }
                })
                .in_set(StatSystem::Cleanup),
            );
        }
    }
}

impl<S: Stat> Default for StatPlugin<S>
where
    S: Component + GetTypeRegistration,
{
    fn default() -> Self {
        Self { clamp_value: ClampValue::default(), _marker: PhantomData }
    }
}

impl<S: Stat> StatPlugin<S>
where
    S: Component + GetTypeRegistration,
{
    #[allow(unused)]
    fn clamp(mut self, value: ClampValue) -> Self {
        self.clamp_value = value;
        self
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
pub enum ClampValue {
    #[default]
    AboveZero,
    None,
    Min(f32),
    Max(f32),
    MinMax(f32, f32),
}

#[derive(Bundle, Default)]
pub struct StatBundle<S: Stat + Component> {
    stat: S,
    base: modifier::Flat<S>,
}

impl<S: Stat + Component + Default> StatBundle<S> {
    pub fn new(value: f32) -> Self {
        Self { stat: S::default(), base: modifier::Flat(S::new(value)) }
    }
}

impl<S: Stat + Component> From<f32> for StatBundle<S> {
    fn from(val: f32) -> Self {
        StatBundle::new(val)
    }
}

pub trait Stat: Reflect + TypePath + Default + Sync + Send + Sized + 'static {
    /// Creates a new [Stat] with the given value.
    fn new(value: f32) -> Self;

    /// Create a [StatBundle<Self>] with the given base stat value.
    fn base(value: f32) -> StatBundle<Self>
    where
        Self: Component,
    {
        StatBundle::new(value)
    }

    /// Create a [PoolBundle<Self>] with the given base stat value & current set to [100%].
    fn pool(value: f32) -> PoolBundle<Self>
    where
        Self: Component,
    {
        PoolBundle::new(value)
    }

    /// Returns the value of the [Stat].
    fn value(&self) -> f32;

    /// Returns a mutable reference to the value of the [Stat].
    /// !!! Should only be used by the stat systems.
    fn value_mut(&mut self) -> &mut f32;

    /// Resets the [Stat] to its default value.
    /// !!! Should only be used by the stat systems.
    #[inline]
    fn reset(&mut self) {
        *self.value_mut() = 0.0;
    }
}

#[derive(Component, Reflect, Deref, DerefMut, From)]
#[component(storage = "SparseSet")]
pub struct DirtyStat<S: Stat>(#[reflect(ignore)] PhantomData<S>);

impl<S: Stat> Default for DirtyStat<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

fn dirty_on_added<S: Stat + Component>(mut commands: Commands, stats: Query<Entity, Added<S>>) {
    for entity in &stats {
        commands.entity(entity).insert(DirtyStat::<S>::default());
    }
}

fn reset_dirty<S: Stat + Component>(mut stats: Query<&mut S, With<DirtyStat<S>>>) {
    for mut stat in &mut stats {
        stat.reset();
    }
}

fn cleanup_dirty<S: Stat>(mut commands: Commands, mut stats: Query<Entity, With<DirtyStat<S>>>)
where
    S: Component,
{
    for entity in &mut stats {
        commands.entity(entity).remove::<DirtyStat<S>>();
    }
}
