//! Navigation
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_spatial::AutomaticUpdate;

use self::{
    agent::{DesiredVelocity, Speed},
    avoidance::AvoidanceOptions,
    occupancy::{Obstacle, Occupancy},
};
use crate::{
    app_state::AppState,
    flow_field::FlowFieldSystems,
    movement::MovementSystems,
    navigation::{
        agent::{Agent, TargetReachedCondition},
        avoidance::Avoidance,
        occupancy::OccupancyAabb,
    },
    prelude::*,
    stats::stat::StatPlugin,
};

pub mod agent;
pub mod avoidance;
pub mod occupancy;

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
        app_register_types!(
            Obstacle,
            Occupancy,
            OccupancyAabb,
            Avoidance,
            Agent,
            DesiredVelocity,
            TargetReachedCondition,
            AvoidanceOptions
        );

        app.add_plugins((AutomaticUpdate::<agent::Agent>::new(), AutomaticUpdate::<occupancy::Obstacle>::new()));
        app.add_plugins(StatPlugin::<Speed>::default());

        app.insert_resource(AvoidanceOptions::default());
        #[cfg(feature = "debug")]
        app.add_plugins((ResourceInspectorPlugin::<AvoidanceOptions>::default(),));

        app.configure_sets(
            Update,
            (NavigationSystems::Setup).after(PhysicsSet::Sync).run_if(in_state(AppState::InGame)),
        );
        app.configure_sets(
            PostUpdate,
            (
                NavigationSystems::Maintain.before(FlowFieldSystems::Maintain),
                NavigationSystems::ApplyVelocity.after(FlowFieldSystems::Poll).before(MovementSystems::Motor),
            )
                .chain()
                .before(PhysicsSet::Prepare)
                .run_if(in_state(AppState::InGame)),
        );
        app.configure_sets(
            Last,
            (NavigationSystems::Cleanup).after(MovementSystems::State).run_if(in_state(AppState::InGame)),
        );

        app.add_systems(Update, (agent::setup, occupancy::setup));
        app.add_systems(
            PostUpdate,
            (occupancy::occupancy, (avoidance::dodgy_obstacle_added, avoidance::dodgy_agent_added))
                .chain()
                .in_set(NavigationSystems::Maintain),
        );
        app.add_systems(
            PostUpdate,
            (
                agent::seek,
                (avoidance::dodgy_agent_sync, avoidance::dodgy_obstacle_sync),
                avoidance::apply_avoidance,
                agent::apply_velocity,
            )
                .chain()
                .in_set(NavigationSystems::ApplyVelocity),
        );
        app.add_systems(
            Last,
            (
                (agent::target_reached, agent::obstacle).chain(),
                avoidance::dodgy_agent_cleanup,
                avoidance::dodgy_obstacle_cleanup,
                occupancy::cleanup,
            )
                .in_set(NavigationSystems::Cleanup),
        );
    }
}
