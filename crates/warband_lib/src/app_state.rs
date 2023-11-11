use bevy::{prelude::*, reflect::Reflect};

#[derive(Clone, Eq, PartialEq, Copy, Debug, Hash, Default, States, Reflect)]
pub enum AppState {
    #[default]
    Loading,
    InGame,
}
