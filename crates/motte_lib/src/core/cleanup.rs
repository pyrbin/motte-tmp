use crate::{app_state::AppState, prelude::*};

#[derive(Component, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct Cleanup<T>(#[reflect(ignore)] PhantomData<T>);

pub struct OnEnterState<const S: AppState>;
pub struct OnExitState<const S: AppState>;

// TODO: want to improve this API.
// ex.
// spawn(Cleanup.on_enter(AppState::InGame))
// spawn(Cleanup.on_exit(AppState::InGame))
// spawn(Cleanup.on_exit::<{ AppState::InGame }>

pub fn cleanup<T: Component>(commands: ParallelCommands, entities: Query<Entity, With<Cleanup<T>>>) {
    entities.par_iter().for_each(|e| {
        commands.command_scope(|mut c| {
            c.entity(e).despawn_recursive();
        })
    });
}
