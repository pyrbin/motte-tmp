use crate::prelude::*;

pub struct DespawnPlugin;

impl Plugin for DespawnPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Despawn, Despawning);
        app.configure_sets(Last, (DespawnSystem::Timer, DespawnSystem::Despawn).chain());
        app.add_systems(
            Last,
            (despawn_timer.in_set(DespawnSystem::Timer), apply_deferred, despawning.in_set(DespawnSystem::Timer))
                .chain(),
        );
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) enum DespawnSystem {
    Timer,
    Despawn,
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub enum Despawn {
    // Despawn immediately.
    #[default]
    Immediate,
    // Despawn after a delay in seconds.
    Delay(f32),
    // Despawn after a delay in frames.
    WaitFrames(u32),
}

#[derive(Component, Default, Reflect)]
pub struct Despawning;

fn despawn_timer(mut commands: Commands, mut despawns: Query<(Entity, &mut Despawn)>, time: Res<Time>) {
    for (entity, mut despawn) in &mut despawns {
        let despawn = match *despawn {
            Despawn::Immediate => true,
            Despawn::Delay(ref mut dur) => {
                *dur -= time.delta_seconds();
                *dur <= 0.0
            }
            Despawn::WaitFrames(ref mut frame) => {
                if *frame == 0 {
                    true
                } else {
                    *frame -= 1;
                    *frame == 0
                }
            }
        };
        if despawn {
            commands.entity(entity).remove::<Despawn>().insert(Despawning);
        }
    }
}

fn despawning(mut commands: Commands, query: Query<Entity, Added<Despawning>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
