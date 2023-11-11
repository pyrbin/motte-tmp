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
