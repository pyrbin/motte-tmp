use bevy_spatial::{kdtree::KDTree3, SpatialAccess};

use super::{
    agent::{Agent, DesiredVelocity, Speed, TargetReached},
    avoidance::AvoidanceVelocity,
    flow_field::pathing::Goal,
};
use crate::{prelude::*, utils::math::determinant};

#[derive(Component, Default)]
pub struct Boided {
    pub is_right_side: Option<bool>,
}

pub struct Steering(Option<Direction2d>);
pub struct Seek(Option<Direction2d>);
pub struct Avoidance(Option<Direction2d>);

pub fn boid_avoidance(
    mut gizmos: Gizmos,
    mut agents: Query<(Entity, &Agent, &GlobalTransform, &mut DesiredVelocity, &Speed, &mut Boided)>,
    agents_kd_tree: Res<KDTree3<Agent>>,
) {
    agents.iter_mut().for_each(|(entity, agent, global_transform, mut desired_velocity, speed, mut boided)| {
        let neighborhood = agent.radius() + Agent::LARGEST.radius();
        let position = global_transform.translation();
        let direction = desired_velocity.normalize_or_zero();
        let right = Vec2::new(-direction.y, direction.x);

        gizmos.circle(position.x0z().y_pad(), Direction3d::Y, neighborhood, Color::CYAN);

        let (seperation, count) = agents_kd_tree
            .within_distance(position, neighborhood)
            .iter()
            .filter_map(|(other_position, other)| {
                if let Some(other) = other
                    && *other == entity
                {
                    return None;
                }

                let distance = (position - *other_position).xz();
                let is_right_side = if let Some(is_right_side) = boided.is_right_side {
                    is_right_side
                } else {
                    let is_right_side = determinant(distance, right) > 0.0;
                    boided.is_right_side = Some(is_right_side);
                    is_right_side
                };
                let distance = if !is_right_side { distance - right } else { distance + right };
                Some(distance)
            })
            .fold((Vec2::ZERO, 0), |acc, s| (acc.0 + s, acc.1 + 1));

        gizmos.arrow(position.y_pad(), (position.xz() + **desired_velocity).x0y().y_pad() * 5.0, Color::GREEN);

        let mut flocking_force = if count > 0 {
            let avg_separation = seperation / count as f32;
            avg_separation.normalize_or_zero()
        } else {
            // boided.is_right_side = None;
            return;
        };

        // let direction = desired_velocity.normalize_or_zero();

        // let right = Vec2::new(-direction.y, direction.x);
        // let is_right_side = determinant(flocking_force, right) > 0.0;

        // // if right side, steer left, else steer right
        // flocking_force = if is_right_side { flocking_force - right } else { flocking_force + right };

        gizmos.arrow(position.y_pad(), (position.xz() + flocking_force).x0y().y_pad() * 5.0, Color::RED);

        let force = (direction + flocking_force).normalize_or_zero() * speed.value();

        **desired_velocity = force;

        gizmos.arrow(position.y_pad(), (position.xz() + force).x0y().y_pad() * 5.0, Color::YELLOW);
    });

    // agents.par_iter_mut().for_each(
    //     |(entity, agent, global_transform, desired_velocity, mut avoidance_velocity, linear_velocity)| {
    //         let mut avoidance_force = Vec2::ZERO;
    //         let neighborhood = agent.radius() * 2.0;
    //         let position = global_transform.translation();
    //         const AVOIDANCE_STRENGTH: f32 = 1.0;

    //         for (neighbor_agent, neighbor_transform, neighbor_velocity) in agents_kd_tree
    //             .within_distance(position, neighborhood)
    //             .iter()
    //             .filter_map(|(_, other)| {
    //                 other.filter(|&other| other != entity).and_then(|other| neighbors.get(other).ok())
    //             })
    //             .map(|n| (*n.0, n.1.translation().xz(), **n.2))
    //         {
    //             let to_neighbor = neighbor_transform - global_transform.translation().xz();
    //             let right = Vec2::new(-desired_velocity.y, desired_velocity.x);
    //             let side_dot = to_neighbor.normalize().dot(right);
    //             if side_dot > 0.0 {
    //                 // Neighbor is on the right, steer left.
    //                 avoidance_force -= right * AVOIDANCE_STRENGTH; // Assume AVOIDANCE_STRENGTH is defined.
    //             } else {
    //                 // Neighbor is on the left, steer right.
    //                 avoidance_force += right * AVOIDANCE_STRENGTH;
    //             }
    //         }

    //         // Adjust the avoidance force to be perpendicular to the goal vector in specific cases.
    //         if avoidance_force.dot(**desired_velocity).abs() < 0.5 {
    //             // Threshold to decide when to adjust to perpendicular.
    //             let perp_goal = Vec2::new(desired_velocity.y, -desired_velocity.x); // Perpendicular to the goal.
    //             avoidance_force =
    //                 if avoidance_force.dot(perp_goal) > 0.0 { perp_goal } else { -perp_goal } * AVOIDANCE_STRENGTH;
    //         }

    //         **avoidance_velocity = avoidance_force;
    //     },
    // )
}
