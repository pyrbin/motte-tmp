//! Local Avoidance, currently using RVO2, implemented by https://lib.rs/crates/dodgy_2d
//! NOTE: Doesn't work exactly how I want it to, but it's a start.
//! In the future I want to further explore the following:
//! - https://www.jdxdev.com/blog/2021/03/19/boids-for-rts/
//! - https://assetstore.unity.com/packages/tools/behavior-ai/local-avoidance-214347
//! - https://github.com/wayne-wu/webgpu-crowd-simulation
//! - https://cell-devs-02.sce.carleton.ca/publications/2019/Hes19a/hesham-centroidalparticledynamicsanexplicitmodel_compressed.pdf
//! - https://onlinelibrary.wiley.com/doi/full/10.1111/cgf.14737

use std::borrow::Cow;

use bevy_spatial::{kdtree::KDTree3, SpatialAccess};

use super::agent::{Agent, Blocking, DesiredVelocity, Speed};
use crate::prelude::*;

#[derive(Component, Debug, Deref, DerefMut, Clone)]
pub(crate) struct DodgyAgent(Cow<'static, dodgy_2d::Agent>);
impl Default for DodgyAgent {
    fn default() -> Self {
        Self(Cow::Owned(dodgy_2d::Agent {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            radius: Agent::SMALLEST.radius(),
            avoidance_responsibility: 1.0,
        }))
    }
}

pub(super) fn dodgy(
    mut agents: Query<(Entity, &Agent, &DodgyAgent, &mut DesiredVelocity, &Speed), (With<Agent>, Without<Blocking>)>,
    other_agents: Query<&DodgyAgent>,
    agents_kd_tree: Res<KDTree3<Agent>>,
    time: Res<Time>,
) {
    let delta_time = time.delta_seconds();
    agents.par_iter_mut().for_each(|(entity, agent, dodgy_agent, mut desired_velocity, speed)| {
        const RADIUS_PADDING: f32 = 1.0;

        let neighborhood = agent.neighborhood() + RADIUS_PADDING;
        let position = dodgy_agent.0.position;

        let neighbors: SmallVec<[Cow<'static, dodgy_2d::Agent>; 16]> = agents_kd_tree
            .within_distance(position.x0y(), neighborhood)
            .iter()
            .filter_map(|(_, other)| {
                other.filter(|&other| other != entity).and_then(|other| other_agents.get(other).ok())
            })
            .filter(|other| other.0.position.distance(position) <= (agent.radius() + RADIUS_PADDING + other.0.radius))
            .map(|other| other.0.clone())
            .collect();

        const AVOIDANCE_OPTIONS: dodgy_2d::AvoidanceOptions =
            dodgy_2d::AvoidanceOptions { obstacle_margin: 0.1, time_horizon: 2.0, obstacle_time_horizon: 0.1 };

        **desired_velocity = dodgy_agent.compute_avoiding_velocity(
            &neighbors,
            &[], // Leaving this empty for now, some reason velocity becomes very slow when obstacles are present.
            **desired_velocity,
            speed.value(),
            delta_time,
            &AVOIDANCE_OPTIONS,
        );
    });
}

pub(super) fn setup(commands: ParallelCommands, mut agents: Query<Entity, (With<Agent>, Without<DodgyAgent>)>) {
    agents.par_iter_mut().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert(DodgyAgent::default());
        })
    });
}

type DodgyAgentNeedsSync =
    Or<(Added<DodgyAgent>, Changed<Agent>, Added<Blocking>, Changed<DesiredVelocity>, Changed<GlobalTransform>)>;

pub(super) fn sync(
    mut agents: Query<(&mut DodgyAgent, &Agent, &GlobalTransform, &LinearVelocity, Has<Blocking>), DodgyAgentNeedsSync>,
) {
    agents.par_iter_mut().for_each(|(mut dodgy_agent, agent, global_transform, velocity, is_blocking)| {
        let dodgy_agent = dodgy_agent.0.to_mut();
        dodgy_agent.position = global_transform.translation().xz();
        dodgy_agent.velocity = velocity.xy();
        dodgy_agent.radius = agent.radius();
        dodgy_agent.avoidance_responsibility = if is_blocking { f32::EPSILON } else { 10.0 };
    });
}

pub(super) fn cleanup(mut commands: Commands, mut removed: RemovedComponents<Agent>) {
    for entity in &mut removed.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<DodgyAgent>();
        }
    }
}

#[cfg(feature = "dev_tools")]
pub(crate) fn gizmos(mut gizmos: Gizmos, agents: Query<(&Agent, &DodgyAgent)>) {
    for (agent, dodgy_agent) in &agents {
        let position = dodgy_agent.0.position;
        gizmos.circle(position.x0y().y_pad(), Direction3d::Y, dodgy_agent.radius + 0.1, Color::PURPLE);
        gizmos.circle(position.x0y().y_pad(), Direction3d::Y, agent.neighborhood(), Color::BLUE);
    }
}
