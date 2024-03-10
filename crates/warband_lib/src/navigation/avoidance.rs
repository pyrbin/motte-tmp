use bevy_spatial::{kdtree::KDTree3, SpatialAccess};

use super::{
    agent::{Agent, DesiredVelocity, Speed, TargetReached, DEFAULT_AGENT_RADIUS},
    occupancy::{Obstacle, Occupancy},
};
use crate::prelude::*;

// TODO: try a custom solution other than "dodgy", maybe simple boids?

pub const MIN_AVOIDANCE_RESPONSIBILITY: f32 = 1.0;
pub const MAX_AVOIDANCE_RESPONSIBILITY: f32 = 100.0;

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct Avoidance {
    neighborhood: f32,
    responsibility: Option<f32>,
}

impl Avoidance {
    pub fn neighborhood(&self) -> f32 {
        self.neighborhood
    }

    #[allow(unused)]
    pub fn responsibility(&self) -> Option<f32> {
        self.responsibility
    }

    pub fn with_neighborhood(mut self, neighborhood: f32) -> Self {
        self.neighborhood = neighborhood;
        self
    }

    #[allow(unused)]
    pub fn with_responsibility(mut self, responsibility: Option<f32>) -> Self {
        self.responsibility = responsibility.map(|r| r.max(MIN_AVOIDANCE_RESPONSIBILITY));
        self
    }
}

#[derive(Resource, Debug, Clone, Copy, Reflect)]
#[reflect(Resource)]
pub struct AvoidanceOptions {
    /// If [`None`], will use [`Avoidance::neighborhood`].
    pub agent_neighborhood: Option<f32>,
    /// If [`None`], will use [`Agent::radius`] * _some modifier_.
    pub obstacle_margin: Option<f32>,
    pub time_horizon: f32,
    pub obstacle_time_horizon: f32,
}

impl Default for AvoidanceOptions {
    fn default() -> Self {
        Self { obstacle_margin: None, agent_neighborhood: Some(1.5), time_horizon: 3.0, obstacle_time_horizon: 0.5 }
    }
}

pub(super) fn apply_avoidance(
    mut agents: Query<(Entity, &DodgyAgent, &Avoidance, &mut DesiredVelocity), (With<Agent>, Without<TargetReached>)>,
    obstacles: Query<&DodgyObstacle>,
    other_agents: Query<&DodgyAgent>,
    agents_kd_tree: Res<KDTree3<Agent>>,
    obstacles_kd_tree: Res<KDTree3<Obstacle>>,
    avoidance_options: Res<AvoidanceOptions>,
    time: Res<Time>,
) {
    let delta_seconds = time.delta_seconds();
    if delta_seconds == 0.0 {
        return;
    }

    agents.par_iter_mut().for_each(|(entity, dodgy_agent, avoidance, mut desired_velocity)| {
        let neighborhood =
            dodgy_agent.radius + avoidance_options.agent_neighborhood.unwrap_or(avoidance.neighborhood());
        let position = dodgy_agent.position.x0y();

        let neighbors = agents_kd_tree
            .within_distance(position, neighborhood)
            .iter()
            .filter_map(|(_, other)| {
                other.filter(|&other| other != entity).and_then(|other| other_agents.get(other).ok())
            })
            .collect::<Vec<_>>();

        let obstacles = obstacles_kd_tree
            .within_distance(position, neighborhood)
            .iter()
            .filter_map(|(_, other)| other.filter(|&other| other != entity).and_then(|other| obstacles.get(other).ok()))
            .map(|obstacle| &obstacle.0)
            .collect::<Vec<_>>();

        let neighbors: Vec<&dodgy::Agent> = neighbors.iter().map(|dodgy_agent| &dodgy_agent.0).collect::<Vec<_>>();

        let avoidance_velocity = dodgy_agent.compute_avoiding_velocity(
            &neighbors,
            obstacles.as_slice(),
            **desired_velocity,
            delta_seconds,
            &dodgy::AvoidanceOptions {
                obstacle_margin: avoidance_options.obstacle_margin.unwrap_or(dodgy_agent.radius * 0.5).max(0.1),
                time_horizon: avoidance_options.time_horizon.max(0.1),
                obstacle_time_horizon: avoidance_options.obstacle_time_horizon.max(0.1),
            },
        );

        desired_velocity.x = avoidance_velocity.x;
        desired_velocity.y = avoidance_velocity.y;
    });
}

#[derive(Component, Debug, Deref, DerefMut, Clone)]
pub(crate) struct DodgyAgent(pub(crate) dodgy::Agent);
impl Default for DodgyAgent {
    fn default() -> Self {
        Self(dodgy::Agent {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            radius: DEFAULT_AGENT_RADIUS,
            max_velocity: 0.0,
            avoidance_responsibility: MIN_AVOIDANCE_RESPONSIBILITY,
        })
    }
}

pub(super) fn dodgy_agent_added(mut commands: Commands, mut agents: Query<Entity, (With<Agent>, Without<DodgyAgent>)>) {
    for entity in &mut agents {
        commands.entity(entity).insert(DodgyAgent::default());
    }
}

type DodgyAgentNeedsSync = Or<(
    Added<DodgyAgent>,
    Changed<Agent>,
    Changed<Avoidance>,
    Changed<Avoidance>,
    Changed<LinearVelocity>,
    ChangedPhysicsPosition,
)>;

pub(super) fn dodgy_agent_sync(
    mut agents: Query<
        (&mut DodgyAgent, &Agent, &Avoidance, &GlobalTransform, &LinearVelocity, Has<TargetReached>, &Speed),
        DodgyAgentNeedsSync,
    >,
) {
    agents.par_iter_mut().for_each(
        |(mut dodgy_agent, agent, avoidance, global_transform, velocity, target_reached, speed)| {
            dodgy_agent.0.position = global_transform.translation().xz();
            dodgy_agent.0.velocity = velocity.xy();
            dodgy_agent.0.radius = agent.radius();
            dodgy_agent.0.max_velocity = **speed * 2.0;
            dodgy_agent.0.avoidance_responsibility = if target_reached {
                MIN_AVOIDANCE_RESPONSIBILITY
            } else {
                avoidance.responsibility().unwrap_or(MIN_AVOIDANCE_RESPONSIBILITY).max(MAX_AVOIDANCE_RESPONSIBILITY)
            };
        },
    );
}

pub(super) fn dodgy_agent_cleanup(mut commands: Commands, mut removed: RemovedComponents<Agent>) {
    for entity in &mut removed.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<DodgyAgent>();
        }
    }
}

#[derive(Component, Debug, Deref, DerefMut, Clone)]
pub struct DodgyObstacle(pub(crate) dodgy::Obstacle);

impl Default for DodgyObstacle {
    fn default() -> Self {
        Self(dodgy::Obstacle::Closed { vertices: Vec::new() })
    }
}

pub(super) fn dodgy_obstacle_added(
    mut commands: Commands,
    mut obstacles: Query<Entity, (With<Obstacle>, Without<DodgyObstacle>, Without<Agent>)>,
) {
    for entity in &mut obstacles {
        commands.entity(entity).insert(DodgyObstacle::default());
    }
}

pub(super) fn dodgy_obstacle_sync(
    mut obstacles: Query<
        (&Occupancy, &mut DodgyObstacle),
        (With<Obstacle>, Without<Agent>, Or<(Changed<Occupancy>, Added<DodgyObstacle>)>),
    >,
) {
    obstacles.par_iter_mut().for_each(|(occupancy, mut dodgy_obstacle)| match occupancy {
        Occupancy::Empty => *dodgy_obstacle = DodgyObstacle::default(),
        Occupancy::Shape(shape) => {
            **dodgy_obstacle = dodgy::Obstacle::Closed { vertices: shape.to_vec() };
        }
    });
}

pub(super) fn dodgy_obstacle_cleanup(mut commands: Commands, mut removed: RemovedComponents<Obstacle>) {
    for entity in &mut removed.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<DodgyObstacle>();
        }
    }
}
