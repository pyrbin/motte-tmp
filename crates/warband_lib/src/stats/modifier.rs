use std::marker::PhantomData;

use bevy::utils::all_tuples;
use smallvec::SmallVec;

use super::{
    stat::{DirtyStat, Stat},
    StatSystem,
};
use crate::{core::previous::PreviousValue, prelude::*};

pub struct ModifierPlugin<M: Stat, S: ModifiableStats>(PhantomData<M>, PhantomData<S>)
where
    M: GetTypeRegistration;

impl<M: Stat, S: ModifiableStats> Plugin for ModifierPlugin<M, S>
where
    M: GetTypeRegistration,
{
    fn build(&self, app: &mut App) {
        app_register_types!(Flat<M>, Mult<M>, M);
        S::register::<M>(app);
    }
}

impl<M: Stat, S: ModifiableStats> Default for ModifierPlugin<M, S>
where
    M: GetTypeRegistration,
{
    fn default() -> Self {
        Self(PhantomData, PhantomData)
    }
}

fn register_single_modifier_pair<M: Stat, S: Stat>(app: &mut App)
where
    M: GetTypeRegistration,
    S: Component,
{
    app.add_systems(
        PostUpdate,
        (
            modifier_changed::<Flat<M>, M, S>,
            modifier_changed::<Mult<M>, M, S>,
            modifier_removed::<Flat<M>, M, S>,
            modifier_removed::<Mult<M>, M, S>,
        )
            .chain()
            .in_set(StatSystem::Dirty),
    );

    app.add_systems(PostUpdate, apply_modifier::<Flat<M>, M, S>.in_set(StatSystem::ModifierFlat));
    app.add_systems(PostUpdate, apply_modifier::<Mult<M>, M, S>.in_set(StatSystem::ModifierMult));
}

pub trait ModifiableStats: Send + Sync + 'static {
    fn register<M: Stat + GetTypeRegistration>(app: &mut App);
}

impl<S: Stat + Component + GetTypeRegistration> ModifiableStats for S {
    fn register<M: Stat + GetTypeRegistration>(app: &mut App) {
        register_single_modifier_pair::<M, S>(app);
    }
}

macro_rules! impl_modifiable_stats_tuple{
    ($($name: ident),*) => {
        impl<$($name: Stat + Component + GetTypeRegistration),*> ModifiableStats for ($($name,)*)
        {
            #[allow(unused)]
            fn register<M: Stat + GetTypeRegistration>(app: &mut App) {
                $(
                    register_single_modifier_pair::<M, $name>(app);
                )*
            }
        }
    };
}

all_tuples!(impl_modifiable_stats_tuple, 0, 15, B);

pub trait Modifier<S: Stat>: Default + Send + Sync + 'static {
    fn apply(&self, stat: &mut S);
    fn value(&self) -> f32;
}

#[derive(Component, Default, Reflect, FromReflect, Deref, DerefMut, From)]
#[reflect(from_reflect = false)]
pub struct Flat<S: Stat>(pub S);

impl<S: Stat, M: Stat> Modifier<S> for Flat<M> {
    #[inline]
    fn apply(&self, stat: &mut S) {
        *stat.value_mut() += <Flat<M> as Modifier<S>>::value(self);
    }

    fn value(&self) -> f32 {
        self.0.value()
    }
}

#[derive(Component, Default, Reflect, FromReflect, Deref, DerefMut, From)]
#[reflect(from_reflect = false)]
pub struct Mult<S: Stat>(pub S);

impl<S: Stat, M: Stat> Modifier<S> for Mult<M> {
    #[inline]
    fn apply(&self, stat: &mut S) {
        *stat.value_mut() *= <Mult<M> as Modifier<S>>::value(self);
    }

    fn value(&self) -> f32 {
        self.0.value()
    }
}

#[derive(Component, Clone, Reflect, From)]
pub enum Modifies {
    Single(Entity),
    Many(SmallVec<[Entity; 8]>),
}

type NonDirtyStatFilter<S> = (With<S>, Without<DirtyStat<S>>);

fn modifier_changed<M: Modifier<T>, T: Stat, S: Stat>(
    mut commands: Commands,
    mut stats: Query<Entity, NonDirtyStatFilter<S>>,
    modifiers: Query<(Entity, Option<&Parent>, Option<&Modifies>), Changed<M>>,
    modifier_parents: Query<(Entity, &Modifies)>,
) where
    M: Component + Modifier<S>,
    S: Component,
{
    let mut add_dirty_stat = |entity: &Entity| {
        if stats.get_mut(*entity).is_ok() {
            commands.entity(*entity).insert(DirtyStat::<S>::default());
        }
    };

    for (entity, maybe_parent, maybe_target) in modifiers.iter() {
        let modifier_target =
            maybe_target.or(maybe_parent.and_then(|p| modifier_parents.get(p.get()).ok().map(|(_, t)| t)));

        match modifier_target {
            Some(Modifies::Single(entity)) => add_dirty_stat(entity),
            Some(Modifies::Many(entities)) => {
                for entity in entities.iter() {
                    add_dirty_stat(entity)
                }
            }
            None => {
                if let Some(parent) = maybe_parent {
                    add_dirty_stat(&parent.get())
                }

                add_dirty_stat(&entity)
            }
        }
    }
}

/// exported as it should only be registered once per [Stat] (in [StatPlugin])
pub(super) fn modifier_target_changed<S: Stat>(
    mut commands: Commands,
    mut stats: Query<Entity, NonDirtyStatFilter<S>>,
    modifiers: Query<(&Modifies, &PreviousValue<Modifies>), Changed<Modifies>>,
) where
    S: Component,
{
    let mut add_dirty_stat = |entity: &Entity| {
        if stats.get_mut(*entity).is_ok() {
            commands.entity(*entity).insert(DirtyStat::<S>::default());
        }
    };

    for (modifier, previous) in modifiers.iter() {
        for target in [modifier, previous.get()] {
            match target {
                Modifies::Single(entity) => add_dirty_stat(entity),
                Modifies::Many(entities) => {
                    for entity in entities.iter() {
                        add_dirty_stat(entity)
                    }
                }
            }
        }
    }
}

// perf: should only add dirty to the removed modifiers' target (but how we do that?).
fn modifier_removed<M: Modifier<T>, T: Stat, S: Stat>(
    query: Query<Entity, NonDirtyStatFilter<S>>,
    mut removed: RemovedComponents<M>,
    mut commands: Commands,
) where
    M: Component + Modifier<S>,
    S: Component,
{
    for _ in removed.read().take(1) {
        for entity in query.iter() {
            commands.entity(entity).insert(DirtyStat::<S>::default());
        }
    }
}

fn apply_modifier<M: Modifier<T>, T: Stat, S: Stat>(
    mut stats: Query<&mut S, With<DirtyStat<S>>>,
    modifiers: Query<(Entity, &M, Option<&Parent>, Option<&Modifies>)>,
    modifier_parents: Query<(Entity, &Modifies)>,
) where
    M: Component + Modifier<S>,
    S: Component,
{
    for (entity, modifier, maybe_parent, maybe_target) in modifiers.iter() {
        let mut apply_modifier = |entity: &Entity| {
            if let Ok(mut stat) = stats.get_mut(*entity) {
                <M as Modifier<S>>::apply(modifier, &mut stat);
            }
        };

        let modifier_target =
            maybe_target.or(maybe_parent.and_then(|p| modifier_parents.get(p.get()).ok().map(|(_, t)| t)));

        match modifier_target {
            Some(Modifies::Single(entity)) => apply_modifier(entity),
            Some(Modifies::Many(entities)) => {
                for entity in entities.iter() {
                    apply_modifier(entity)
                }
            }
            None => {
                if let Some(parent) = maybe_parent {
                    apply_modifier(&parent.get())
                }

                apply_modifier(&entity)
            }
        }
    }
}
