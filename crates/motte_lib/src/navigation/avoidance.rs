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

use super::{
    agent::{Agent, Blocking, DesiredVelocity, TargetDistance},
    flow_field::layout::FieldBorders,
};
use crate::{navigation::obstacle::Obstacle, prelude::*};

#[derive(Component, Debug, Deref, DerefMut, Clone)]
pub(crate) struct DodgyAgent(Cow<'static, dodgy_2d::Agent>);
impl Default for DodgyAgent {
    fn default() -> Self {
        Self(Cow::Owned(dodgy_2d::Agent {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            radius: Agent::SMALLEST.radius(),
            avoidance_responsibility: f32::EPSILON,
        }))
    }
}

#[derive(Component, Debug, Deref, DerefMut, Clone, Default)]
pub(crate) struct DodgyObstacle(Option<Cow<'static, dodgy_2d::Obstacle>>);

pub(super) fn rvo2(
    mut agents: Query<(Entity, &Agent, &DodgyAgent, &mut DesiredVelocity)>,
    other_agents: Query<&DodgyAgent, Without<Blocking>>,
    agents_kd_tree: Res<KDTree3<Agent>>,
    obstacles: Query<&DodgyObstacle>,
    field_borders: Res<FieldBorders>,
    time: Res<Time>,
) {
    let delta_time = time.delta_seconds();

    // TODO: only get nearby obstacles.
    let mut obstacles: Vec<Cow<'static, dodgy_2d::Obstacle>> =
        obstacles.iter().filter_map(|obstacle| obstacle.0.clone()).collect::<Vec<_>>();

    obstacles.push(Cow::Owned(dodgy_2d::Obstacle::Open { vertices: (**field_borders).into() }));

    agents.par_iter_mut().for_each(|(entity, agent, dodgy_agent, mut desired_velocity)| {
        const fn neighborhood(agent: &Agent) -> f32 {
            agent.radius() + Agent::LARGEST.radius()
        }

        let neighborhood = neighborhood(agent);
        let position = dodgy_agent.0.position;
        let neighbors: SmallVec<[Cow<'static, dodgy_2d::Agent>; 16]> = agents_kd_tree
            .within_distance(position.x0y(), neighborhood)
            .iter()
            .filter_map(|(_, other)| {
                other.filter(|&other| other != entity).and_then(|other| other_agents.get(other).ok())
            })
            .filter(|other| other.0.position.distance(position) <= (agent.radius() + other.0.radius))
            .map(|other| other.0.clone())
            .collect();

        const AVOIDANCE_OPTIONS: dodgy_2d::AvoidanceOptions =
            dodgy_2d::AvoidanceOptions { obstacle_margin: 0.1, time_horizon: 3.0, obstacle_time_horizon: 0.1 };

        const MAX_SPEED_MULTIPLIER: f32 = 1.2;

        **desired_velocity = dodgy_agent.compute_avoiding_velocity(
            &neighbors,
            &obstacles,
            **desired_velocity,
            MAX_SPEED_MULTIPLIER * desired_velocity.length(),
            delta_time,
            &AVOIDANCE_OPTIONS,
        );
    });
}

pub(super) fn setup(
    commands: ParallelCommands,
    agents: Query<Entity, (With<Agent>, Without<DodgyAgent>)>,
    blocking: Query<Entity, (With<Agent>, With<Blocking>, With<DodgyAgent>, Without<DodgyObstacle>)>,
    obstacles: Query<Entity, (With<Obstacle>, Without<DodgyObstacle>)>,
) {
    agents.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert(DodgyAgent::default());
        })
    });

    blocking.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert(DodgyObstacle::default());
        })
    });

    obstacles.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert(DodgyObstacle::default());
        })
    });
}

type DodgyAgentNeedsSync =
    Or<(Added<DodgyAgent>, Changed<Agent>, Added<Blocking>, Changed<DesiredVelocity>, Changed<GlobalTransform>)>;

pub(super) fn sync_agents(
    mut agents: Query<
        (&mut DodgyAgent, &Agent, &GlobalTransform, &LinearVelocity, Has<Blocking>, &TargetDistance),
        DodgyAgentNeedsSync,
    >,
) {
    agents.par_iter_mut().for_each(
        |(mut dodgy_agent, agent, global_transform, velocity, is_blocking, target_distance)| {
            let dodgy_agent = dodgy_agent.0.to_mut();
            dodgy_agent.position = global_transform.translation().xz();
            dodgy_agent.velocity = velocity.xy();
            dodgy_agent.radius = agent.radius();

            const fn calculate_avoidance_priority(agent: &Agent, distance: f32) -> f32 {
                use parry2d::na::SimdPartialOrd;
                const MAX_RANGE: f32 = 1000.0;
                let clamped_distance = distance.simd_clamp(0.0, MAX_RANGE);
                let size_priority = (Agent::LARGEST.size() + 1.0) - agent.size();
                let avoidance_priority = MAX_RANGE * size_priority + clamped_distance;
                avoidance_priority * avoidance_priority
            }

            dodgy_agent.avoidance_responsibility =
                if is_blocking { f32::EPSILON } else { calculate_avoidance_priority(agent, **target_distance) };
        },
    );
}

type DodgyObstacleNeedsSync = Or<(Added<DodgyObstacle>, Changed<Obstacle>, Changed<ColliderAabb>)>;

pub(super) fn sync_obstacles(mut obstacles: Query<(&mut DodgyObstacle, &Obstacle), DodgyObstacleNeedsSync>) {
    obstacles.par_iter_mut().for_each(|(mut dodgy_obstacle, obstacle)| {
        if let Some(obstacle) = obstacle.try_into_dodgy() {
            **dodgy_obstacle = Some(Cow::Owned(obstacle));
        } else {
            **dodgy_obstacle = None;
        }
    });
}

type DodgyBlockingAgentNeedsSync =
    Or<(Added<DodgyObstacle>, Changed<Agent>, Added<Blocking>, Changed<GlobalTransform>)>;

pub(super) fn sync_blocking(
    mut blocking: Query<(&mut DodgyObstacle, &GlobalTransform, &Agent), DodgyBlockingAgentNeedsSync>,
) {
    blocking.par_iter_mut().for_each(|(mut dodgy_obstacle, global_transform, agent)| {
        const SUBDIVISIONS: usize = 8;
        const fn circle_footprint(agent: &Agent, position: Vec2) -> [Vec2; SUBDIVISIONS] {
            use parry2d::na::SimdComplexField;
            const RADIUS_PADDING: f32 = 0.1;
            let radius = agent.radius() + RADIUS_PADDING;
            let mut vertices: [Vec2; SUBDIVISIONS] = [Vec2 { x: 0.0, y: 0.0 }; SUBDIVISIONS];
            let mut i = 0;
            while i < SUBDIVISIONS {
                let angle: f32 = i as f32 * 2.0 * std::f32::consts::PI / SUBDIVISIONS as f32;
                vertices[i].x = position.x + radius * angle.simd_cos();
                vertices[i].y = position.y + radius * angle.simd_sin();
                i += 1;
            }
            vertices
        }

        dodgy_obstacle.0 = Some(Cow::Owned(dodgy_2d::Obstacle::Closed {
            vertices: circle_footprint(agent, global_transform.translation().xz()).into(),
        }))
    });
}

pub(super) fn cleanup(
    mut commands: Commands,
    mut removed_agents: RemovedComponents<Agent>,
    mut removed_obstacle: RemovedComponents<Obstacle>,
    mut removed_blocking: RemovedComponents<Blocking>,
) {
    for entity in &mut removed_agents.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<DodgyAgent>();
        }
    }

    for entity in &mut removed_obstacle.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<DodgyObstacle>();
        }
    }

    for entity in &mut removed_blocking.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<DodgyObstacle>();
        }
    }
}

#[cfg(feature = "dev_tools")]
pub(crate) fn gizmos(mut gizmos: Gizmos, agents: Query<(&Agent, &DodgyAgent)>) {
    for (agent, dodgy_agent) in &agents {
        let position = dodgy_agent.0.position;
        gizmos.circle(position.x0y().y_pad(), Direction3d::Y, dodgy_agent.radius + 0.1, Color::PURPLE);
    }
}
