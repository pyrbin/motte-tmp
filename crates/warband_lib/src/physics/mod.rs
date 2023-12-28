use crate::prelude::*;

pub mod character_controller;
pub mod trait_ext;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(character_controller::CharacterControllerPlugin);
    }
}
