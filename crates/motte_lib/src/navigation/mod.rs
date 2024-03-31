use bevy_spatial::AutomaticUpdate;

use self::agent::Agent;
use crate::{
    app_state::AppState,
    movement::MovementSystems,
    navigation::{
        agent::{agent_type, AgentType, Blocking, DesiredDirection, DesiredVelocity, Speed, TargetDistance},
        flow_field::{FlowFieldAgentPlugin, FlowFieldPlugin, FlowFieldSystems},
        obstacle::Obstacle,
    },
    prelude::*,
    stats::stat::StatPlugin,
};

pub mod agent;
pub mod avoidance;
pub mod flow_field;
pub mod obstacle;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationSystems {
    Setup,
    Maintain,
    Velocity,
    Avoidance,
    ApplyVelocity,
    Cleanup,
}

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Agent, Obstacle, DesiredDirection, TargetDistance, DesiredVelocity, Blocking, Speed);

        app.add_plugins(FlowFieldPlugin);
        app.add_plugins((AutomaticUpdate::<agent::Agent>::new(), AutomaticUpdate::<obstacle::Obstacle>::new()));
        app.add_plugins(StatPlugin::<Speed>::default());

        app.add_plugins(AgentPlugin::<{ Agent::Small }>);
        app.add_plugins(AgentPlugin::<{ Agent::Medium }>);
        app.add_plugins(AgentPlugin::<{ Agent::Large }>);
        app.add_plugins(AgentPlugin::<{ Agent::Huge }>);

        app.configure_sets(
            FixedUpdate,
            (
                NavigationSystems::Setup,
                NavigationSystems::Maintain.before(FlowFieldSystems::Maintain),
                NavigationSystems::Velocity.after(FlowFieldSystems::Pathing),
                NavigationSystems::Avoidance.after(FlowFieldSystems::Pathing),
                NavigationSystems::ApplyVelocity.after(FlowFieldSystems::Pathing).before(MovementSystems::Motor),
                NavigationSystems::Cleanup.after(MovementSystems::State),
            )
                .chain()
                .before(PhysicsSet::Prepare)
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(FixedUpdate, (agent::setup, avoidance::setup).in_set(NavigationSystems::Setup));
        app.add_systems(
            FixedUpdate,
            (
                (obstacle::obstacle, agent::blocking, avoidance::sync).in_set(NavigationSystems::Maintain),
                (avoidance::dodgy).in_set(NavigationSystems::Avoidance),
                (agent::desired_velocity).in_set(NavigationSystems::Velocity),
                (agent::apply_velocity).in_set(NavigationSystems::ApplyVelocity),
            ),
        );
        app.add_systems(FixedUpdate, (agent::target_reached, avoidance::cleanup).in_set(NavigationSystems::Cleanup));
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
