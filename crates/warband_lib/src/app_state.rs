use bevy::{prelude::*, reflect::Reflect};

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash, Reflect)]
pub enum AppState {
    #[default]
    Loading,
    InGame,
}
