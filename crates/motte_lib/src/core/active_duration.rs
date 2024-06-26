use std::time::Duration;

use crate::prelude::*;

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
