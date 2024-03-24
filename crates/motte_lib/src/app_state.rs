use std::marker::ConstParamTy;

use bevy::{prelude::*, reflect::Reflect};

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash, ConstParamTy, Reflect)]
pub enum AppState {
    #[default]
    Loading,
    InGame,
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
