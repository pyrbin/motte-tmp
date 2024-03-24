use std::marker::PhantomData;

use crate::prelude::*;

pub struct PreviousValuePlugin<T: Component + Clone> {
    schedule: Interned<dyn ScheduleLabel>,
    _marker: PhantomData<T>,
}

impl<T: Component + Clone> PreviousValuePlugin<T> {
    pub fn in_schedule(schedule: impl ScheduleLabel) -> Self {
        Self { schedule: schedule.intern(), _marker: PhantomData }
    }
}

impl<T: Component + Clone> Default for PreviousValuePlugin<T> {
    fn default() -> Self {
        Self::in_schedule(PostUpdate)
    }
}

impl<T: Component + Clone> Plugin for PreviousValuePlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(self.schedule, propagate_previous_changed::<T>);
    }
}

#[derive(Component, Default, Deref, DerefMut, Reflect, From)]
pub struct PreviousValue<T: Component + Clone>(T);

impl<T: Component + Clone> PreviousValue<T> {
    #[allow(unused)]
    pub fn get(&self) -> &T {
        &self.0
    }
}

pub(crate) fn propagate_previous_changed<T: Component + Clone>(
    mut commands: Commands,
    mut values: Query<(Entity, Option<&mut PreviousValue<T>>, &T), Changed<T>>,
) {
    for (entity, mut previous_value, current_value) in values.iter_mut() {
        if let Some(previous_value) = &mut previous_value {
            previous_value.0 = current_value.clone();
        } else {
            commands.entity(entity).insert(PreviousValue(current_value.clone()));
        }
    }
}
