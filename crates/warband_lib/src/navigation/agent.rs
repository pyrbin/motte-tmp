use super::pathing::{Path, PathTarget};
use crate::{
    app_state::AppState,
    navigation::{avoidance::AvoidanceSystems, pathing::PathingSystems},
    prelude::*,
};

const DESTINATION_ACCURACY: f32 = 0.1;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AgentSystems {
    Movement,
}

pub struct AgentPlugin;

impl Plugin for AgentPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Agent, DesiredVelocity, TargetReachedCondition);

        app.add_systems(Update, (agent_follow_path).chain().run_if(in_state(AppState::InGame)));
        app.add_systems(
            PostUpdate,
            (agent_apply_velocity)
                .chain()
                .after(AvoidanceSystems::Apply)
                .before(PhysicsSet::StepSimulation)
                .run_if(in_state(AppState::InGame)),
        );
        app.add_systems(
            PostUpdate,
            (agent_finish_path).run_if(in_state(AppState::InGame)).after(PathingSystems::Pathing),
        );
    }
}

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct Agent {
    pub radius: f32,
}

impl Agent {
    pub fn radius(&self) -> f32 {
        self.radius
    }
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub enum TargetReachedCondition {
    Distance(f32),
    VisibleAtDistance(f32),
}

impl TargetReachedCondition {
    #[inline]
    pub fn has_reached_target(&self, agent: &Agent, path: &Path, position: Vec3) -> bool {
        match self {
            TargetReachedCondition::Distance(distance) => {
                position.xz().distance(path.end().xz()) < (agent.radius + distance) + DESTINATION_ACCURACY
            }
            TargetReachedCondition::VisibleAtDistance(distance) => {
                path.len() == 0
                    && position.xz().distance(path.end().xz()) < (agent.radius + distance) + DESTINATION_ACCURACY
            }
        }
    }
}

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct Hold;

#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Default, Reflect)]
#[reflect(Component)]
pub struct DesiredVelocity(pub Vec3);

impl DesiredVelocity {
    pub const ZERO: LinearVelocity = LinearVelocity(Vector::ZERO);
    pub fn reset(&mut self) {
        self.0 = Vec3::ZERO;
    }
}

fn agent_follow_path(mut agents: Query<(Option<&Path>, &GlobalTransform, &mut DesiredVelocity), With<Agent>>) {
    const MAX_SPEED: f32 = 400.;
    agents.par_iter_mut().for_each(|(path, global_transform, mut desired_velocity)| {
        if let Some(path) = path {
            let position = global_transform.translation();
            if let Some(&waypoint) = path.current() {
                let direction = (waypoint - position).normalize();
                **desired_velocity = direction * MAX_SPEED;
            }
        } else {
            **desired_velocity = Vec3::ZERO;
        }
    });
}

fn agent_apply_velocity(time: Res<Time>, mut agents: Query<(&DesiredVelocity, &mut LinearVelocity), With<Agent>>) {
    let delta_seconds = time.delta_seconds();
    agents.par_iter_mut().for_each(|(desired_velocity, mut linvel)| {
        if desired_velocity.is_approx_zero() {
            return;
        }

        linvel.x += desired_velocity.x * delta_seconds;
        linvel.z += desired_velocity.z * delta_seconds;
    });
}

fn agent_finish_path(
    mut commands: Commands,
    mut agents: Query<
        (Entity, &Agent, &GlobalTransform, &mut DesiredVelocity, &mut Path, Option<&TargetReachedCondition>),
        With<PathTarget>,
    >,
) {
    for (entity, agent, global_transform, mut desired_velocity, mut path, target_reached_condition) in &mut agents {
        let agent_position = global_transform.translation();

        let target_reached = if let Some(target_reached_condition) = target_reached_condition {
            target_reached_condition.has_reached_target(agent, path.as_ref(), agent_position)
        } else {
            TargetReachedCondition::Distance(0.0).has_reached_target(agent, path.as_ref(), agent_position)
        };

        if target_reached {
            desired_velocity.reset();
            commands.entity(entity).remove::<(PathTarget, Path, TargetReachedCondition)>();
            commands.entity(entity).insert(Hold);
            continue;
        }

        if let Some(&waypoint) = path.current() {
            let distance = waypoint.xz().distance(agent_position.xz());
            let threshold = agent.radius + DESTINATION_ACCURACY;
            if distance <= threshold {
                path.pop();
            }
        } else {
            desired_velocity.reset();
            commands.entity(entity).remove::<(PathTarget, Path, TargetReachedCondition)>();
            commands.entity(entity).insert(Hold);
        }
    }
}
