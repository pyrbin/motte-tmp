use std::borrow::Cow;

use crate::{
    app_state::AppState,
    cleanup::{Cleanup, OnEnterState, OnExitState},
    prelude::*,
};

pub mod active_duration;
pub mod camera;
pub mod cleanup;
pub mod cursor;
pub mod despawn;
pub mod previous;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Owner);
        app.add_plugins(bevy_mod_picking::DefaultPickingPlugins);
        app.add_plugins((despawn::DespawnPlugin, cursor::CursorPlugin, camera::CameraPlugin::in_schedule(Last)));
        app.add_systems(OnEnter(AppState::InGame), cleanup::cleanup::<Cleanup<OnEnterState<{ AppState::InGame }>>>);
        app.add_systems(OnExit(AppState::InGame), cleanup::cleanup::<Cleanup<OnExitState<{ AppState::InGame }>>>);
    }
}

mod name_tags {
    pub const UI: &str = ":ui";
    pub const UNIT: &str = ":unit";
    pub const CAMERA: &str = ":camera";
    pub const LIGHT: &str = ":light";
}

pub trait NameTags {
    fn ui(name: impl Into<Cow<'static, str>>) -> Name {
        Name::new(format!("{} {}", name_tags::UI, name.into()))
    }

    fn light(name: impl Into<Cow<'static, str>>) -> Name {
        Name::new(format!("{} {}", name_tags::LIGHT, name.into()))
    }

    fn unit(name: impl Into<Cow<'static, str>>) -> Name {
        Name::new(format!("{} {}", name_tags::UNIT, name.into()))
    }

    fn camera(name: impl Into<Cow<'static, str>>) -> Name {
        Name::new(format!("{} {}", name_tags::CAMERA, name.into()))
    }
}

impl NameTags for Name {}

#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, DerefMut, From)]
pub struct Owner(pub Entity);

/// Generic component to mark component [`T`] as dirty.
#[derive(Component, Default, Deref, DerefMut, From, Reflect)]
#[component(storage = "SparseSet")]
pub struct Dirty<T: Component>(#[reflect(ignore)] pub PhantomData<T>);

/// Generic component to mark component [`T`] as deactivated.
#[derive(Component, Default, Deref, DerefMut, From, Reflect)]
#[component(storage = "SparseSet")]
pub struct Disabled<T: Component>(#[reflect(ignore)] pub PhantomData<T>);
