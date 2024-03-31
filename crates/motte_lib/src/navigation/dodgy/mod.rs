//! Local Avoidance, currently using RVO2, implemented by https://lib.rs/crates/dodgy_2d
//! In the future I want to further explore the following:
//! - https://www.jdxdev.com/blog/2021/03/19/boids-for-rts/
//! - https://assetstore.unity.com/packages/tools/behavior-ai/local-avoidance-214347
//! - https://github.com/wayne-wu/webgpu-crowd-simulation
//! - https://cell-devs-02.sce.carleton.ca/publications/2019/Hes19a/hesham-centroidalparticledynamicsanexplicitmodel_compressed.pdf
//! - https://onlinelibrary.wiley.com/doi/full/10.1111/cgf.14737

use std::borrow::Cow;

use bevy_spatial::{kdtree::KDTree3, SpatialAccess};

use super::{
    agent::{Agent, DesiredVelocity, Speed, TargetDistance, TargetReached},
    flow_field::pathing::Goal,
    obstacle::Obstacle,
};
use crate::{movement::motor::Stationary, prelude::*};

pub(super) fn rvo2(
    mut agents: Query<
        (Entity, &Agent, &mut DesiredVelocity, &GlobalTransform, &LinearVelocity, &TargetDistance, &Speed),
        Or<(Without<TargetReached>, Without<Goal>)>,
    >,
    other_agents: Query<
        (&Agent, &GlobalTransform, &LinearVelocity, &TargetDistance, Has<Stationary>),
        Without<Obstacle>,
    >,
    other_obstacles: Query<&Obstacle>,
    agents_kd_tree: Res<KDTree3<Agent>>,
    time: Res<Time>,
) {
    let delta_time = time.delta_seconds();

    agents.par_iter_mut().for_each(|(entity, agent, mut dvel, global_transform, linvel, target_distance, speed)| {
        let neighborhood = agent.radius() * 2.0;
        let position = global_transform.translation();

        let neighbors: Vec<Cow<'static, dodgy_2d::Agent>> = agents_kd_tree
            .within_distance(position, neighborhood)
            .iter()
            .filter_map(|(_, other)| {
                other.filter(|&other| other != entity).and_then(|other| other_agents.get(other).ok())
            })
            .map(|(agent, transform, linvel, other_target_distance, target_reached)| {
                Cow::Owned(dodgy_2d::Agent {
                    position: transform.translation().xz(),
                    velocity: linvel.0.xz(),
                    radius: agent.radius(),
                    avoidance_responsibility: if target_reached { 1.0 } else { other_target_distance.max(1.0) },
                })
            })
            .collect::<Vec<_>>();

        let obstacles: Vec<Cow<'static, dodgy_2d::Obstacle>> = other_obstacles
            .iter()
            .filter_map(|obstacle| {
                if let Obstacle::Shape(vertices) = obstacle {
                    return Some(Cow::Owned(dodgy_2d::Obstacle::Closed { vertices: vertices.clone().into_vec() }));
                } else {
                    return None;
                }
            })
            .collect::<Vec<_>>();

        let mut agent = dodgy_2d::Agent {
            position: position.xz(),
            velocity: linvel.0.xz(),
            radius: agent.radius(),
            avoidance_responsibility: target_distance.max(0.1),
        };
        let time_horizon = 3.0;
        let obstacle_time_horizon = 0.5;
        let velocity = agent.compute_avoiding_velocity(
            &neighbors,
            &[],
            **dvel,
            **speed,
            delta_time,
            &dodgy_2d::AvoidanceOptions { obstacle_margin: 0.1, time_horizon, obstacle_time_horizon },
        );

        **dvel = velocity;
    });
}
