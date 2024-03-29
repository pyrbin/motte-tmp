use bevy_spatial::{kdtree::KDTree3, SpatialAccess};

use super::{
    agent::{Agent, DesiredVelocity, Speed},
    avoidance::AvoidanceVelocity,
};
use crate::{prelude::*, utils::math::determinant};

pub const EPSILON: f32 = 1.0 / 1024.0;

pub fn clearpath_avoidance(
    mut agents: Query<(Entity, &Agent, &GlobalTransform, &DesiredVelocity, &mut AvoidanceVelocity, &LinearVelocity)>,
    neighbors: Query<(&Agent, &GlobalTransform, &DesiredVelocity)>,
    agents_kd_tree: Res<KDTree3<Agent>>,
) {
    agents.par_iter_mut().for_each(
        |(entity, agent, global_transform, desired_velocity, mut avoidance_velocity, linear_velocity)| {
            let neighborhood = agent.radius() + 5.0;
            let position = global_transform.translation();
            let neighbors = agents_kd_tree
                .within_distance(position, neighborhood)
                .iter()
                .filter_map(|(_, other)| {
                    other.filter(|&other| other != entity).and_then(|other| neighbors.get(other).ok())
                })
                .map(|n| (*n.0, n.1.translation().xz(), **n.2))
                .collect::<Vec<_>>();

            if neighbors.is_empty() {
                return;
            }

            let mut it = neighbors.len() - 1;
            loop {
                if let Some(avel) = compute_avoidance_velocity(
                    (*agent, position.xz(), linear_velocity.xz()),
                    **desired_velocity,
                    &neighbors[0..it],
                    &[],
                ) {
                    **avoidance_velocity = avel;
                    break;
                } else {
                    it -= 1;
                    if it == 0 {
                        break;
                    }
                }
            }
        },
    );
}

pub(super) fn clearpath_integration(mut agents: Query<(&mut AvoidanceVelocity, &mut DesiredVelocity, &Speed)>) {
    agents.par_iter_mut().for_each(|(mut a_vel, mut desvel, speed)| {
        let mut vel = **a_vel;
        let v_dir = vel.normalize_or_zero();
        let max_speed = speed.value() * 1.2;
        if vel.length() > max_speed {
            vel = max_speed * v_dir;
        }

        if vel.length() > 0.01 {
            **desvel = vel;
        }

        **a_vel = Vec2::ZERO;
    })
}

#[derive(Clone, Debug, Default)]
pub struct VO {
    pub apex: Vec2,
    pub left: Vec2,
    pub right: Vec2,
}

#[derive(Clone, Debug, Default)]
pub struct RVO {
    pub apex: Vec2,
    pub left: Vec2,
    pub right: Vec2,
}

#[derive(Clone, Debug, Default)]
pub struct HRVO {
    pub apex: Vec2,
    pub left: Vec2,
    pub right: Vec2,
}

// TODO: ray2d
#[derive(Clone, Debug, Default)]
pub struct Line {
    pub point: Vec2,
    pub direction: Vec2,
}

// Agent , Position , Velocity
type Unit = (Agent, Vec2, Vec2);

#[inline]
fn compute_vo_edges(agent: Unit, neighbor: Unit) -> (Vec2, Vec2) {
    let (agent, position, _) = agent;
    let (neighbor, neighbor_position, _) = neighbor;

    let agent_to_neighbor = (neighbor_position - position).normalize_or_zero();

    const CLEARPATH_BUFFER_RADIUS: f32 = 0.1;
    let right = agent_to_neighbor.perp() * (neighbor.radius() + agent.radius() + CLEARPATH_BUFFER_RADIUS);

    let right_tan = neighbor_position + right;
    let left_tan = neighbor_position - right;

    ((right_tan - position).normalize_or_zero(), (left_tan - position).normalize_or_zero())
}

#[inline]
fn compute_vo(agent: Unit, neighbor: Unit) -> VO {
    let (right, left) = compute_vo_edges(agent, neighbor);
    let (agent, position, velocity) = agent;
    let (neighbor, neighbor_position, neighbor_velocity) = neighbor;
    VO { apex: position + neighbor_velocity, left, right }
}

#[inline]
fn compute_rvo(agent: Unit, neighbor: Unit) -> RVO {
    let (right, left) = compute_vo_edges(agent, neighbor);
    let (agent, position, velocity) = agent;
    let (neighbor, neighbor_position, neighbor_velocity) = neighbor;

    let mut apex_off = Vec2::ZERO;
    apex_off = velocity + neighbor_velocity;
    apex_off *= 0.5;
    apex_off += position;

    RVO { apex: apex_off, left, right }
}

#[inline]
fn compute_hrvo(agent: Unit, neighbor: Unit) -> HRVO {
    let mut hrvo = HRVO::default();
    let rvo = compute_rvo(agent, neighbor);
    let (agent, position, velocity) = agent;
    let (neighbor, neighbor_position, neighbor_velocity) = neighbor;

    let center_line = rvo.left + rvo.right;
    let vo_apex = position + neighbor_velocity;

    let det = determinant(velocity, center_line);
    if det > EPSILON {
        // the entity velocity is left of the RVO centerline
        let l1 = Line { point: rvo.apex, direction: rvo.left };
        let l2 = Line { point: vo_apex, direction: rvo.right };
        let Some(intersection) = line_intersection(&l1, &l2) else { panic!("No intersection found") };
        hrvo.apex = intersection;
    } else if det < -EPSILON {
        // the entity velocity is right of the RVO centerline
        let l1 = Line { point: rvo.apex, direction: rvo.right };
        let l2 = Line { point: vo_apex, direction: rvo.left };
        let Some(intersection) = line_intersection(&l1, &l2) else { panic!("No intersection found") };
        hrvo.apex = intersection;
    } else {
        // The entity velocity is right on the centerline
        hrvo.apex = rvo.apex;
    }

    hrvo.right = rvo.right;
    hrvo.left = rvo.left;
    hrvo
}

fn compute_all_vos(agent: Unit, neighbors: &[Unit]) -> Vec<VO> {
    let (agent, position, velocity) = agent;
    let mut vos: Vec<_> = Vec::new();
    for (neighbor, neighbor_position, neighbor_velocity) in neighbors {
        let distance = position.distance(*neighbor_position);
        if distance.is_approx_zero() {
            continue;
        }
        vos.push(compute_vo((agent, position, velocity), (*neighbor, *neighbor_position, *neighbor_velocity)));
    }
    vos
}

#[inline]
fn compute_all_hrvos(agent: Unit, neighbors: &[Unit]) -> Vec<HRVO> {
    let (agent, position, velocity) = agent;
    let mut hrvos: Vec<_> = Vec::new();
    for (neighbor, neighbor_position, neighbor_velocity) in neighbors {
        let distance = position.distance(*neighbor_position);
        if distance.is_approx_zero() {
            continue;
        }
        hrvos.push(compute_hrvo((agent, position, velocity), (*neighbor, *neighbor_position, *neighbor_velocity)));
    }
    hrvos
}

#[inline]
fn rays_repr(vos: &[VO], hrvos: &[HRVO]) -> Vec<Line> {
    let mut out = Vec::with_capacity((vos.len() + hrvos.len()) * 2); // Preallocate vector with the required capacity
    for hrvo in hrvos {
        out.push(Line { point: hrvo.apex, direction: hrvo.left });
        out.push(Line { point: hrvo.apex, direction: hrvo.right });
    }

    for vo in vos {
        out.push(Line { point: vo.apex, direction: vo.left });
        out.push(Line { point: vo.apex, direction: vo.right });
    }
    out
}

#[inline]
fn point_inside_pcr(point: Vec2, lines: &[Line]) -> bool {
    debug_assert!(lines.len() % 2 == 0, "Lines must be in pairs");

    for line_pair in lines.chunks(2) {
        let left_line = &line_pair[0];
        let right_line = &line_pair[1];

        debug_assert!(
            (left_line.direction.length_squared() - 1.0).abs() < EPSILON,
            "Left direction vector must be a unit vector"
        );
        debug_assert!(
            (right_line.direction.length_squared() - 1.0).abs() < EPSILON,
            "Right direction vector must be a unit vector"
        );

        let det_left = determinant(point - left_line.point, left_line.direction);
        let det_right = determinant(point - right_line.point, right_line.direction);

        let left_of_vo = det_left < EPSILON;
        let right_of_vo = det_right > -EPSILON;

        if left_of_vo && right_of_vo {
            return true;
        }
    }
    false
}

#[inline]
fn ray_ray_intersection_2d(l1: &Line, l2: &Line) -> Option<Vec2> {
    line_intersection(l1, l2).filter(|&intersect| {
        !((intersect.x - l1.point.x) / l1.direction.x < 0.0
            || (intersect.y - l1.point.y) / l1.direction.y < 0.0
            || (intersect.x - l2.point.x) / l2.direction.x < 0.0
            || (intersect.y - l2.point.y) / l2.direction.y < 0.0)
    })
}

#[inline]
fn compute_vo_xpoints(lines: &[Line]) -> Vec<Vec2> {
    let n_rays = lines.len();
    let mut xpoints: Vec<Vec2> = Vec::new();

    for i in 0..n_rays {
        for j in i + 1..n_rays {
            // Start from i + 1 to avoid duplicate computations
            if let Some(p) = ray_ray_intersection_2d(&lines[i], &lines[j]) {
                if point_inside_pcr(p, lines) {
                    continue;
                }
                xpoints.push(p);
            }
        }
    }
    xpoints
}

#[inline]
fn compute_vdes_proj_points(rays: &[Line], desired_velocity: Vec2, out: &mut Vec<Vec2>) {
    for ray in rays {
        assert!((ray.direction.length() - 1.0).abs() < EPSILON, "Ray direction vector must be a unit vector.");

        let len = ray.direction.dot(desired_velocity);
        // Project the desired_velocity onto the ray's direction.
        let proj = ray.direction * len + ray.point;

        // Check if the projected point is not inside the PCR and, if so, add it to the collection.
        if !point_inside_pcr(proj, rays) {
            out.push(proj);
        }
    }
}

#[inline]
fn compute_new_velocity(collision_free_points: &[Vec2], desired_velocity: Vec2, position: Vec2) -> Vec2 {
    let mut min_dist = f32::INFINITY;
    let mut vel = Vec2::ZERO;

    for &point in collision_free_points {
        let current = point - position;
        let diff = desired_velocity - current;
        let len = diff.length();
        if len < min_dist {
            min_dist = len;
            vel = current
        }
    }

    vel
}

#[inline]
fn compute_avoidance_velocity(agent: Unit, vpref: Vec2, neighbors: &[Unit], obstacles: &[Unit]) -> Option<Vec2> {
    let hrvos = compute_all_hrvos(agent, neighbors);
    let vos = compute_all_vos(agent, obstacles);

    let rays = rays_repr(&vos, &hrvos);
    let xp = agent.1 + vpref;

    if !point_inside_pcr(xp, &rays) {
        return Some(agent.2);
    }

    // The line segments are intersected pairwise and the intersection points
    // inside the combined hybrid reciprocal velocity obstacle are discarded.
    // The remaining intersection points are permissible new velocities on the
    // boundary of the combined hybrid reciprocal velocity obstacle.
    let mut xpoints = compute_vo_xpoints(&rays);

    // In addition we project the preferred velocity (des_v) on to the line
    // segments (xz_left_side and xz_right_side of each hrvo) and also retain
    // those points that are outside the combined hybrid reciprocal velocity
    // obstacle.
    compute_vdes_proj_points(&rays, vpref, &mut xpoints);

    if xpoints.is_empty() {
        return None;
    }

    let velocity = compute_new_velocity(&xpoints, vpref, agent.1);

    Some(velocity)
}

#[inline]
fn line_intersection(l1: &Line, l2: &Line) -> Option<Vec2> {
    let l1_slope = if l1.direction.x.abs() < EPSILON { f32::NAN } else { l1.direction.y / l1.direction.x };
    let l2_slope = if l2.direction.x.abs() < EPSILON { f32::NAN } else { l2.direction.y / l2.direction.x };

    // Return None if lines are parallel or coincident
    if (l1_slope - l2_slope).abs() < EPSILON {
        return None;
    }

    if l1_slope.is_nan() && !l2_slope.is_nan() {
        Some(Vec2::new(l1.point.x, (l1.point.x - l2.point.x) * l2_slope + l2.point.y))
    } else if !l1_slope.is_nan() && l2_slope.is_nan() {
        Some(Vec2::new(l2.point.x, (l2.point.x - l1.point.x) * l1_slope + l2.point.y))
    } else {
        let x = (l1_slope * l1.point.x - l2_slope * l2.point.x + l2.point.y - l1.point.y) / (l1_slope - l2_slope);
        let y = l2_slope * (x - l2.point.x) + l2.point.y;
        Some(Vec2::new(x, y))
    }
}
