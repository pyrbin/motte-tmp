use std::time::Duration;

use crate::prelude::*;

pub(crate) mod camera;
pub(crate) mod cursor;
pub(crate) mod despawn;
pub(crate) mod previous;

pub(crate) struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Owner);
        app.add_plugins((despawn::DespawnPlugin, cursor::CursorPlugin, camera::CameraPlugin::in_schedule(Last)));
    }
}

#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, DerefMut, From)]
pub struct Owner(pub Entity);

/// Generic component to mark component [`T`] as dirty.
#[derive(Component, Default, Deref, DerefMut, From, Reflect)]
#[component(storage = "SparseSet")]
pub struct Dirty<T: Component>(#[reflect(ignore)] pub PhantomData<T>);

/// Generic component to mark component [`T`] as deactivated.
#[derive(Component, Default, Deref, DerefMut, From, Reflect)]
#[component(storage = "SparseSet")]
pub struct Deactivated<T: Component>(#[reflect(ignore)] pub PhantomData<T>);

/// Type alias for [`ChangedPhysicsPosition`].
/// Should be used instead of [`Changed<Transform>`] or [`Changed<GlobalTransform>`]
pub type ChangedPhysicsPosition = Or<(Changed<Position>, Added<Position>)>;

#[derive(Component, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct ActiveDuration<T: Component> {
    duration: Duration,
    #[reflect(ignore)]
    _m: PhantomData<T>,
}

impl<T: Component> ActiveDuration<T> {
    pub fn duration(&self) -> Duration {
        self.duration
    }

    fn tick(&mut self, delta: Duration) {
        self.duration = self.duration.saturating_add(delta);
    }
}

impl<T: Component> Default for ActiveDuration<T> {
    fn default() -> Self {
        Self { duration: Duration::from_secs(0), _m: PhantomData }
    }
}

pub fn active_duration<T: Component>(
    commands: ParallelCommands,
    time: Res<Time>,
    mut active: Query<(&mut ActiveDuration<T>, &T)>,
    added: Query<Entity, (Added<T>, Without<ActiveDuration<T>>)>,
    mut removed: RemovedComponents<T>,
) {
    added.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert(ActiveDuration::<T>::default());
        });
    });

    let delta = time.delta();
    active.par_iter_mut().for_each(|(mut duration, _)| {
        duration.tick(delta);
    });

    for entity in &mut removed.read() {
        commands.command_scope(|mut c| {
            if let Some(mut commands) = c.get_entity(entity) {
                commands.remove::<ActiveDuration<T>>();
            }
        });
    }
}
