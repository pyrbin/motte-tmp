use bevy_spatial::{kdtree::KDTree3, SpatialAccess};

use super::{
    agent::{Agent, DesiredVelocity, Speed, TargetReached, DEFAULT_AGENT_RADIUS},
    occupancy::{Obstacle, Occupancy},
};
use crate::prelude::*;

pub const MIN_AVOIDANCE_RESPONSIBILITY: f32 = 1.0;
pub const MAX_AVOIDANCE_RESPONSIBILITY: f32 = 100.0;

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct Avoidance {
    neighbourhood: f32,
    responsibility: Option<f32>,
}

impl Avoidance {
    pub fn neighbourhood(&self) -> f32 {
        self.neighbourhood
    }

    #[allow(unused)]
    pub fn responsibility(&self) -> Option<f32> {
        self.responsibility
    }

    pub fn with_neighbourhood(mut self, neighbourhood: f32) -> Self {
        self.neighbourhood = neighbourhood;
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
    pub time_horizon: f32,
    pub obstacle_time_horizon: f32,
}

impl Default for AvoidanceOptions {
    fn default() -> Self {
        Self { time_horizon: 2.0, obstacle_time_horizon: 0.5 }
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
        let neighbourhood = dodgy_agent.radius + avoidance.neighbourhood();
        let position = dodgy_agent.position.x0y();

        let neigbours = agents_kd_tree
            .within_distance(position, neighbourhood)
            .iter()
            .filter_map(|(_, other)| {
                other.filter(|&other| other != entity).and_then(|other| other_agents.get(other).ok())
            })
            .collect::<Vec<_>>();

        let obstacles = obstacles_kd_tree
            .within_distance(position, neighbourhood)
            .iter()
            .filter_map(|(_, other)| other.filter(|&other| other != entity).and_then(|other| obstacles.get(other).ok()))
            .map(|obstacle| &obstacle.0)
            .collect::<Vec<_>>();

        let neigbours: Vec<&dodgy::Agent> = neigbours.iter().map(|dodgy_agent| &dodgy_agent.0).collect::<Vec<_>>();

        let avoidance_velocity = dodgy_agent.compute_avoiding_velocity(
            &neigbours,
            &obstacles.as_slice(),
            **desired_velocity,
            delta_seconds,
            &dodgy::AvoidanceOptions {
                obstacle_margin: dodgy_agent.radius * 0.25,
                time_horizon: avoidance_options.time_horizon,
                obstacle_time_horizon: avoidance_options.obstacle_time_horizon,
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

type DodgyAgentNeedsSync =
    Or<(Added<DodgyAgent>, Changed<Agent>, Changed<Avoidance>, Changed<LinearVelocity>, ChangedPhysicsPosition)>;

pub(super) fn dodgy_agent_sync(
    mut agents: Query<
        (&mut DodgyAgent, &Agent, &GlobalTransform, &LinearVelocity, Has<TargetReached>, &Speed),
        DodgyAgentNeedsSync,
    >,
) {
    agents.par_iter_mut().for_each(|(mut dodgy_agent, agent, global_transform, velocity, target_reached, speed)| {
        dodgy_agent.0.position = global_transform.translation().xz();
        dodgy_agent.0.velocity = velocity.xy();
        dodgy_agent.0.radius = agent.radius();
        dodgy_agent.0.max_velocity = if target_reached { 0.0 } else { **speed };
        dodgy_agent.0.avoidance_responsibility = MIN_AVOIDANCE_RESPONSIBILITY;
    });
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
        Occupancy::Occupied(shape) => {
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
