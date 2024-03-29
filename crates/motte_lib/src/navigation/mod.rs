use bevy_spatial::AutomaticUpdate;

use self::agent::Agent;
use crate::{
    app_state::AppState,
    movement::MovementSystems,
    navigation::{
        agent::{agent_type, AgentType, DesiredVelocity, Seek, Speed, TargetDistance},
        avoidance::AvoidancePlugin,
        flow_field::{FlowFieldAgentPlugin, FlowFieldPlugin, FlowFieldSystems},
        obstacle::Obstacle,
    },
    prelude::*,
    stats::stat::StatPlugin,
};

// TODO: Resource for RebuildFlows* RebuildObstacle* unsafe poke for when to trigger rebuild.

pub mod agent;
pub mod avoidance;
pub mod boids;
pub mod clearpath;
pub mod flow_field;
pub mod obstacle;
pub mod sonar;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationSystems {
    Setup,
    Maintain,
    Seek,
    Avoidance,
    ApplyVelocity,
    Cleanup,
}

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Agent, Obstacle, Seek, TargetDistance, DesiredVelocity, Speed);

        app.add_plugins(FlowFieldPlugin);
        app.add_plugins((AutomaticUpdate::<agent::Agent>::new(), AutomaticUpdate::<obstacle::Obstacle>::new()));
        app.add_plugins(StatPlugin::<Speed>::default());
        app.add_plugins(AvoidancePlugin);

        app.add_plugins(AgentPlugin::<{ Agent::Small }>);
        app.add_plugins(AgentPlugin::<{ Agent::Medium }>);
        app.add_plugins(AgentPlugin::<{ Agent::Large }>);
        app.add_plugins(AgentPlugin::<{ Agent::Huge }>);

        app.configure_sets(
            FixedUpdate,
            (
                NavigationSystems::Setup,
                NavigationSystems::Maintain.before(FlowFieldSystems::Maintain),
                NavigationSystems::Seek.after(FlowFieldSystems::Pathing),
                NavigationSystems::Avoidance.after(FlowFieldSystems::Pathing),
                NavigationSystems::ApplyVelocity.after(FlowFieldSystems::Pathing).before(MovementSystems::Motor),
                NavigationSystems::Cleanup.after(MovementSystems::State),
            )
                .chain()
                .before(PhysicsSet::Prepare)
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(FixedUpdate, (agent::setup).in_set(NavigationSystems::Setup));
        app.add_systems(
            FixedUpdate,
            (
                (obstacle::obstacle).in_set(NavigationSystems::Maintain),
                (agent::seek).in_set(NavigationSystems::Seek),
                (agent::apply_velocity).in_set(NavigationSystems::ApplyVelocity),
            ),
        );
        app.add_systems(FixedUpdate, (agent::target_reached).in_set(NavigationSystems::Cleanup));
    }
}

struct AgentPlugin<const AGENT: Agent>;

impl<const AGENT: Agent> Plugin for AgentPlugin<AGENT> {
    fn build(&self, app: &mut App) {
        app_register_types!(AgentType<AGENT>);

        app.add_plugins(FlowFieldAgentPlugin::<AGENT>);
        app.add_systems(FixedUpdate, agent_type::<AGENT>.in_set(NavigationSystems::Setup));
    }
}
