// explicitIntegration
// contactSolve
// constraintSolve
// finalizeVelocity

use bevy_spatial::{kdtree::KDTree3, SpatialAccess};

use super::{
    agent::{Agent, DesiredVelocity, Speed},
    obstacle::Obstacle,
};
use crate::{
    app_state::AppState,
    navigation::{
        clearpath::{clearpath_avoidance, clearpath_integration},
        NavigationSystems,
    },
    prelude::*,
};

pub struct AvoidancePlugin;

impl Plugin for AvoidancePlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(
            Avoidance,
            AvoidanceVelocity,
            InverseMass,
            ExtrapolatedTranslation,
            ContrainedTranslation,
            PreConstraintTranslation
        );
        // app.add_systems(
        //     FixedUpdate,
        //     (explicit_integration, contact_solve, constraint_solve, finalize_velocity)
        //         .chain()
        //         .in_set(NavigationSystems::Avoidance)
        //         .run_if(in_state(AppState::InGame)),
        // );

        app.add_systems(
            FixedUpdate,
            (clearpath_avoidance, clearpath_integration)
                .chain()
                .in_set(NavigationSystems::Avoidance)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct Avoidance {
    neighborhood: f32,
}

impl Avoidance {
    pub fn new(n: f32) -> Self {
        Self { neighborhood: n }
    }
}

#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Default, Reflect)]
pub struct AvoidanceVelocity(pub Vec2);

#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Default, Reflect)]
pub struct ExtrapolatedTranslation(pub Vec3);

#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Default, Reflect)]
pub struct PreConstraintTranslation(pub Vec3);

#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Default, Reflect)]
pub struct ContrainedTranslation(pub Vec3);

pub(super) fn explicit_integration(
    mut agents: Query<(
        &mut AvoidanceVelocity,
        &mut ExtrapolatedTranslation,
        &GlobalTransform,
        &DesiredVelocity,
        &LinearVelocity,
    )>,
    time: Res<Time>,
) {
    let delta_time = time.delta_seconds();
    agents.par_iter_mut().for_each(|(mut a_vel, mut xp, global_transform, d_vel, linvel)| {
        // VELOCITY PLANNING
        const KSI: f32 = 0.0385;
        // velocity blending
        **a_vel = (1.0 - KSI) * linvel.0.xz() + KSI * **d_vel;
        let translation = global_transform.translation();
        **xp = translation + a_vel.x0y() * delta_time;
    })
}

// near
pub(super) fn contact_solve(
    mut agents: Query<(
        Entity,
        &Avoidance,
        &Agent,
        &InverseMass,
        &ExtrapolatedTranslation,
        &mut PreConstraintTranslation,
        &GlobalTransform,
    )>,
    neighbors: Query<(&Agent, &InverseMass, &ExtrapolatedTranslation, &GlobalTransform)>,
    obstacles: Query<(Entity, &Obstacle, &GlobalTransform)>,
    agents_kd_tree: Res<KDTree3<Agent>>,
) {
    agents.par_iter_mut().for_each(|(entity, avoidance, agent, mass, old, mut xp, global_transform)| {
        let mut total_dx = Vec3::ZERO;
        let mut neighbor_count = 0;

        **xp = **old;

        let position = global_transform.translation();

        // Agent Collisions
        for neighbor in agents_kd_tree
            .within_distance(position, agent.radius() + avoidance.neighborhood)
            .iter()
            .filter_map(|(_, other)| other.filter(|&other| other != entity).and_then(|other| neighbors.get(other).ok()))
        {
            let (other, other_mass, other_xp, other_global_transform) = neighbor;
            let other_position = other_global_transform.translation();

            let n = **xp - **other_xp;
            let d = n.length();
            let f = d - (agent.radius() + other.radius());
            if f > 0.0 {
                continue;
            }

            let n = n.normalize();
            let w = mass.0 / (mass.0 + other_mass.0);
            const SHORT_RANGE: f32 = 1.0;
            let mut dx = -w * SHORT_RANGE * f * n;
            const FRICTION: bool = false;
            if FRICTION {
                let ax = **xp + dx;
                let nx = **other_xp - dx;

                let d_rel = (ax - position) - (nx - other_position); // relative displacement
                let mut d_tan = d_rel - d_rel.dot(n) * n; // tangential displacement
                let d_tan_norm = d_tan.length(); // tangential displacement norm

                const MU_STATIC: f32 = 0.21;
                const MU_KINEMATIC: f32 = 0.10;

                if d_tan_norm > MU_STATIC * d {
                    d_tan *= (MU_KINEMATIC * d / d_tan_norm).min(1.0)
                }

                dx = dx + w + d_tan;
            }

            total_dx = total_dx + dx;
            neighbor_count = neighbor_count + 1;
        }

        if neighbor_count > 0 {
            const AVG_COEFFICIENT: f32 = 1.2; // paper = 1.2 [1,2]
            let dx = AVG_COEFFICIENT * total_dx / neighbor_count as f32; // average displacement
            **xp = **xp + dx; // update extrapolated position
        }

        // Obstacle Collisions
        total_dx = Vec3::ZERO;
        neighbor_count = 0;

        for (obstacle_entity, obstacle, obstacle_global_transform) in &obstacles {
            let Some(segments) = obstacle.line_segments() else {
                continue;
            };

            for segment in segments {
                if let Some(dx) = wall_constraint(xp.xz(), agent.radius(), segment) {
                    total_dx += dx;
                    neighbor_count += 1;
                }
            }
        }

        if neighbor_count > 0 {
            const AVG_COEFFICIENT: f32 = 1.2; // paper = 1.2 [1,2]
            let dx = AVG_COEFFICIENT * total_dx / neighbor_count as f32; // average displacement
            **xp = **xp + dx; // update extrapolated position
        }
    });
}

// long
pub(super) fn constraint_solve(
    mut agents: Query<(
        Entity,
        &Avoidance,
        &Agent,
        &AvoidanceVelocity,
        &InverseMass,
        &PreConstraintTranslation,
        &mut ContrainedTranslation,
        &GlobalTransform,
    )>,
    neighbors: Query<(&Agent, &InverseMass, &PreConstraintTranslation, &GlobalTransform, &AvoidanceVelocity)>,
    obstacles: Query<(Entity, &Obstacle, &GlobalTransform)>,
    agents_kd_tree: Res<KDTree3<Agent>>,
    time: Res<Time>,
) {
    const FAR_RADIUS: f32 = 6.0;
    let delta_time = time.delta_seconds();

    agents.par_iter_mut().for_each(|(entity, avoidance, agent, velocity, mass, old, mut xp, global_transform)| {
        let mut total_dx = Vec3::ZERO;
        let mut neighbor_count = 0;

        **xp = **old;

        let position = global_transform.translation();

        // long_range_constraint
        for neighbor in agents_kd_tree
            .within_distance(position, agent.radius() + FAR_RADIUS)
            .iter()
            .filter_map(|(_, other)| other.filter(|&other| other != entity).and_then(|other| neighbors.get(other).ok()))
        {
            let (other, other_mass, other_xp, other_global_transform, other_velocity) = neighbor;
            let other_position = other_global_transform.translation();

            let r = agent.radius() + other.radius();
            let mut r_sqrt = r * r;

            let distance = position.distance(other_position);
            if distance > r {
                r_sqrt = (r - distance) * (r - distance);
            }

            // relative displacement
            let x_ij = position - other_position;

            // relative velocity
            let v_ij = (1.0 / delta_time) * (**xp - position - **other_xp + other_position);

            let a = v_ij.dot(v_ij);
            let b = -x_ij.dot(v_ij);
            let c = x_ij.dot(x_ij) - r_sqrt;
            let discr = b * b - a * c;
            if discr < 0.0 || a.abs() < f32::EPSILON {
                continue;
            }

            let discr = discr.sqrt();

            // Compute exact time to collision
            let t = (b - discr) / a;
            const T0: f32 = 20.0;
            // Prune out invalid case
            if t < f32::EPSILON || t > T0 {
                continue;
            }

            // Get time before and after collision
            let t_nocollision = delta_time * (t / delta_time).floor();
            let t_collision = delta_time + t_nocollision;

            // Get collision and collision-free positions
            let xi_nocollision = position + t_nocollision * velocity.x0y();
            let xi_collision = position + t_collision * velocity.x0y();
            let xj_nocollision = other_position + t_nocollision * other_velocity.x0y();
            let xj_collision = other_position + t_collision * other_velocity.x0y();

            // Enforce collision free for x_collision using distance constraint
            let n = xi_collision - xj_collision;
            let d = n.length();

            let f = d - r;
            if f < 0.0 {
                const K_LONG_RANGE: f32 = 0.15; // paper = 0.24 [0-1]

                let n = n.normalize_or_zero();
                let k = K_LONG_RANGE * (-t_nocollision * t_nocollision / T0).exp();
                const ITERATION: usize = 1;
                let k = 1.0 - (1.0 - k).powf(1.0 / (ITERATION + 1) as f32);
                let w = mass.0 / (mass.0 + other_mass.0);
                let mut dx = -w * f * n;

                // Avoidance Model
                let xi_collision = xi_collision + dx;
                let xj_collision = xj_collision - dx;

                // total relative displacement
                let d_vec = (xi_collision - xi_nocollision) - (xj_collision - xj_nocollision);

                // tangetial displacement
                let d_tangent = d_vec - d_vec.dot(n) * n;

                dx = dx + w * d_tangent;

                total_dx += k * dx;
                neighbor_count += 1;
            }
        }

        if neighbor_count > 0 {
            const AVG_COEFFICIENT: f32 = 1.2; // paper = 1.2 [1,2]
            let dx = AVG_COEFFICIENT * total_dx / neighbor_count as f32; // average displacement
            **xp = **xp + dx; // update extrapolated position
        }

        for (obstacle_entity, obstacle, obstacle_global_transform) in &obstacles {
            let Some(segments) = obstacle.line_segments() else {
                continue;
            };

            for segment in segments {
                if let Some(dx) = wall_constraint(xp.xz(), agent.radius(), segment) {
                    total_dx += dx;
                    neighbor_count += 1;
                }
            }
        }

        if neighbor_count > 0 {
            const AVG_COEFFICIENT: f32 = 1.2; // paper = 1.2 [1,2]¨
            const K_OBSTACLE: f32 = 0.24;
            const ITERATION: usize = 1;
            let k = 1.0 - (1.0 - K_OBSTACLE).powf(1.0 / (ITERATION + 1) as f32);
            let dx = AVG_COEFFICIENT * k * total_dx / neighbor_count as f32; // average displacement
            **xp = **xp + dx; // update extrapolated position
        }
    });
}

pub(super) fn finalize_velocity(
    mut agents: Query<(
        Entity,
        &Avoidance,
        &Agent,
        &AvoidanceVelocity,
        &InverseMass,
        &ContrainedTranslation,
        &GlobalTransform,
        &mut DesiredVelocity,
        &Speed,
    )>,
    obstacles: Query<(Entity, &Obstacle, &GlobalTransform)>,
    time: Res<Time>,
) {
    let delta_time = time.delta_seconds();

    agents.par_iter_mut().for_each(
        |(entity, avoidance, agent, velocity, mass, xp, global_transform, mut desired, speed)| {
            let position = global_transform.translation();
            let mut new_v = (**xp - position) / delta_time;

            // TODO: cohesion (xsph)

            // 4.7 Obstacle Avoidance (Open Steer)
            for (obstacle_entity, obstacle, obstacle_global_transform) in &obstacles {
                let Some(segments) = obstacle.line_segments() else {
                    continue;
                };
                const T_OBSTACLE: f32 = 20.0;

                let v = new_v;
                let a0 = xp.xz();
                let a1 = (**xp + T_OBSTACLE * v).xz();

                // Intersection tests with all edges
                let mut n_min: Vec2 = Vec2::ZERO;
                let mut t_min: f32 = T_OBSTACLE;

                for segment in segments {
                    if let Some((n, t)) = intersect_line(a0, a1, segment.0, segment.1) {
                        if t > f32::EPSILON && t < t_min {
                            t_min = t;
                            n_min = n;
                        }
                    }
                }

                if t_min < 1.0 {
                    let t_min = t_min * T_OBSTACLE;
                    // Use the radial normal as the contact normal so that there's some tangential velocity
                    let n = (**xp + t_min * v) - obstacle_global_transform.translation();
                    const K_AVOID: f32 = 0.2;
                    let v_avoid = K_AVOID * n;
                    new_v = new_v + v_avoid;
                }
            }

            // Clamp
            let v_dir = new_v.normalize_or_zero();
            let max_speed = speed.value() * 1.2;
            if new_v.length() > max_speed {
                new_v = max_speed * v_dir;
            }

            **desired = new_v.xz();
        },
    );
}

#[inline(always)]
fn intersect_line(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Option<(Vec2, f32)> {
    let s1 = p1 - p0;
    let s2 = p3 - p2;

    let den = -s2.x * s1.y + s1.x * s2.y;
    if den < f32::EPSILON {
        return None;
    }

    let den = 1.0 / den;
    let s = (-s1.y * (p0.x - p2.x) + s1.x * (p0.y - p2.y)) * den;
    let t = (s2.x * (p0.y - p2.y) - s2.y * (p0.x - p2.x)) * den;
    if s > 0.0 && s < 1.0 && t > 0.0 && t < 1.0 {
        let n = Vec2::new(-s2.y, s2.x); // normal
        return Some((n, t));
    }

    return None;
}

#[inline(always)]
fn wall_constraint(xp: Vec2, radius: f32, segment: (Vec2, Vec2)) -> Option<Vec3> {
    let a = xp - segment.0;
    let b = segment.1 - segment.0; // segment direction
    let b_norm = b.normalize_or_zero();
    let l = a.dot(b_norm);

    if l < f32::EPSILON || l > b.length() {
        return None;
    }

    let c = l * b_norm;

    let xj = segment.0 + c; // closest point on segment
    let n = a - c; // normal to segment
    let d = n.length(); // distance to segment
    const WALL_THICKNESS: f32 = 0.5;
    let r = radius + WALL_THICKNESS; // agent radius + wall thickness
    let f = d - r; // penetration depth
    if f < 0.0 {
        let n = n.normalize_or_zero();
        let dx = -f * n;
        return Some(dx.x0y());
    }

    return None;
}
