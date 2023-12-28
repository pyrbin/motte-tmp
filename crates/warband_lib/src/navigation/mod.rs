use bevy_spatial::AutomaticUpdate;

use crate::prelude::*;

pub mod agent;
pub mod avoidance;
pub mod avoidance_2;
pub mod obstacle;
pub mod pathing;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((AutomaticUpdate::<agent::Agent>::new(), AutomaticUpdate::<obstacle::Obstacle>::new()));
        app.add_plugins((pathing::PathingPlugin, agent::AgentPlugin, avoidance::AvoidancePlugin));
    }
}

#[derive(Bundle, Default)]
pub struct AgentBundle {
    pub agent: agent::Agent,
    pub avoidance: avoidance::Avoidance,
    pub desired_velocity: agent::DesiredVelocity,
}
