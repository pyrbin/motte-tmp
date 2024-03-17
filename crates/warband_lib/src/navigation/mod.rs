use bevy_spatial::AutomaticUpdate;

use self::agent::{Agent, AgentRadius};
use crate::{
    navigation::{flow_field::FlowFieldPlugin, obstacle::Obstacle},
    prelude::*,
};

pub mod agent;
pub mod flow_field;
pub mod obstacle;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationSystems {
    Setup,
    Maintain,
    ApplyVelocity,
    Cleanup,
}

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Agent, AgentRadius, Obstacle);

        app.add_plugins(FlowFieldPlugin);
        app.add_plugins(AutomaticUpdate::<agent::Agent>::new());

        app.add_systems(FixedUpdate, obstacle::obstacle);
    }
}
