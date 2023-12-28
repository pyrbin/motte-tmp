use bevy_spatial::{kdtree::KDTree3, SpatialAccess};
use dodgy::AvoidanceOptions;

use super::{
    agent::{Agent, DesiredVelocity, Hold},
    obstacle::Obstacle,
};
use crate::{app_state::AppState, prelude::*};

const MIN_AVOIDANCE_RESPONSIBILITY: f32 = 2.0;
const MIN_AVOIDANCE_MAX_VELOCITY: f32 = 400.0;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvoidanceSystems {
    Sync,
    Apply,
}

pub struct AvoidancePlugin;

impl Plugin for AvoidancePlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Avoidance);

        app.configure_sets(
            PostUpdate,
            (AvoidanceSystems::Apply, AvoidanceSystems::Sync)
                .chain()
                .before(PhysicsSet::StepSimulation)
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            PostUpdate,
            ((added_dodgy_agent, added_dodgy_obstacle), apply_deferred, (sync_dodgy_agents))
                .chain()
                .in_set(AvoidanceSystems::Sync),
        );

        app.add_systems(PostUpdate, (apply_avoidance_to_agents).chain().in_set(AvoidanceSystems::Apply));

        app.add_systems(Last, (cleanup_agent, cleanup_obstacle));
    }
}

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

    pub fn responsibility(&self) -> Option<f32> {
        self.responsibility
    }

    pub fn with_neighbourhood(mut self, neighbourhood: f32) -> Self {
        self.neighbourhood = neighbourhood;
        self
    }
    pub fn with_responsibility(mut self, responsibility: Option<f32>) -> Self {
        self.responsibility = responsibility.map(|r| r.max(MIN_AVOIDANCE_RESPONSIBILITY));
        self
    }
}

#[derive(Component, Debug, Deref, DerefMut, Clone)]
pub(crate) struct DodgyAgent(pub(crate) dodgy::Agent);
impl Default for DodgyAgent {
    fn default() -> Self {
        Self(dodgy::Agent {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            radius: 1.0,
            max_velocity: MIN_AVOIDANCE_MAX_VELOCITY,
            avoidance_responsibility: 1.0,
        })
    }
}

#[derive(Component, Debug, Deref, DerefMut, Clone)]
pub(crate) struct DodgyObstacle(pub(crate) dodgy::Obstacle);
impl Default for DodgyObstacle {
    fn default() -> Self {
        // TODO: fix obstacle generation
        Self(dodgy::Obstacle::Closed { vertices: Vec::new() })
    }
}

impl DodgyObstacle {
    pub fn has_vertices(&self) -> bool {
        match self.0 {
            dodgy::Obstacle::Closed { ref vertices } => !vertices.is_empty(),
            _ => false,
        }
    }
}

fn apply_avoidance_to_agents(
    agents_kd_tree: Res<KDTree3<Agent>>,
    mut agents: Query<(Entity, &DodgyAgent, &Avoidance, &mut DesiredVelocity), Without<Hold>>,
    time: Res<Time>,
    neighbours: Query<(&DodgyAgent, &DodgyObstacle)>,
) {
    let delta_seconds = time.delta_seconds();
    if delta_seconds == 0.0 {
        return;
    }

    agents.iter_mut().for_each(|(entity, dodgy_agent, avoidance, mut linvel)| {
        let agents = agents_kd_tree
            .within_distance(dodgy_agent.position.x0y(), avoidance.neighbourhood)
            .iter()
            .filter(|(_, other)| other.is_some() && other.unwrap() != entity)
            .map(|(_, other)| neighbours.get(other.unwrap()).unwrap())
            .collect::<Vec<_>>();

        let neigbours = agents.iter().map(|(dodgy_agent, _)| &dodgy_agent.0).collect::<Vec<_>>();
        let obstacles = agents
            .iter()
            .filter_map(|(dodgy_agent, dodgy_obstacle)| {
                if dodgy_agent.velocity.is_approx_zero() && dodgy_obstacle.has_vertices() {
                    None
                } else {
                    Some(&dodgy_obstacle.0)
                }
            })
            .collect::<Vec<_>>();

        if neighbours.is_empty() {
            return;
        }

        const AVOIDANCE_OPTIONS: AvoidanceOptions =
            AvoidanceOptions { obstacle_margin: 0.1, time_horizon: 3.0, obstacle_time_horizon: 1.0 };
        let empty = if true { [].into() } else { obstacles };

        let avoidance_velocity =
            dodgy_agent.compute_avoiding_velocity(&neigbours, &empty, linvel.xz(), delta_seconds, &AVOIDANCE_OPTIONS);

        linvel.x = avoidance_velocity.x;
        linvel.z = avoidance_velocity.y;
    });
}

fn added_dodgy_agent(
    mut commands: Commands,
    mut agents: Query<Entity, (With<Agent>, Or<(Without<DodgyAgent>, Without<DodgyObstacle>)>)>,
) {
    for entity in &mut agents {
        commands.entity(entity).insert(DodgyAgent::default());
        commands.entity(entity).insert(DodgyObstacle::default());
    }
}

fn added_dodgy_obstacle(mut commands: Commands, mut agents: Query<Entity, (With<Obstacle>, Without<DodgyObstacle>)>) {
    for entity in &mut agents {
        commands.entity(entity).insert(DodgyObstacle::default());
    }
}

type DodgyAgentNeedsSync =
    Or<(Changed<Agent>, Changed<Avoidance>, Added<DodgyAgent>, Changed<GlobalTransform>, Changed<DesiredVelocity>)>;

fn sync_dodgy_agents(
    mut agents: Query<
        (&mut DodgyAgent, &mut DodgyObstacle, &Agent, &Avoidance, &GlobalTransform, &DesiredVelocity, Option<&Hold>),
        DodgyAgentNeedsSync,
    >,
) {
    agents.par_iter_mut().for_each(
        |(mut dodgy_agent, mut dodgy_obstacle, agent, _avoidance, global_transform, linvel, hold)| {
            dodgy_agent.0.position = global_transform.translation().xz();
            dodgy_agent.0.velocity = linvel.xz();
            dodgy_agent.0.radius = agent.radius();
            let is_holding = hold.is_some();
            dodgy_agent.0.max_velocity = if is_holding { 0.0 } else { MIN_AVOIDANCE_MAX_VELOCITY };
            dodgy_agent.0.avoidance_responsibility = if is_holding { 1.0 } else { 100.0 };

            // create a Vec of Vec2 vertices that should represent a sqaure of the agents radius. The vertices array
            // should be closed
            let mut vertices = Vec::new();
            let radius = agent.radius() * 0.75;
            let position = dodgy_agent.0.position;
            vertices.push(position + Vec2::new(-radius, -radius));
            vertices.push(position + Vec2::new(-radius, radius));
            vertices.push(position + Vec2::new(radius, radius));
            vertices.push(position + Vec2::new(radius, -radius));

            dodgy_obstacle.0 = dodgy::Obstacle::Closed { vertices };
        },
    );
}

fn cleanup_agent(mut commands: Commands, mut removed: RemovedComponents<Agent>) {
    for entity in &mut removed.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<DodgyAgent>();
        }
    }
}

fn cleanup_obstacle(mut commands: Commands, mut removed: RemovedComponents<Obstacle>) {
    for entity in &mut removed.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<DodgyObstacle>();
        }
    }
}
