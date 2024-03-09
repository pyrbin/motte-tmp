use super::occupancy::Obstacle;
use crate::{
    movement::motor::{Movement, Stationary},
    prelude::*,
};

pub const DEFAULT_AGENT_RADIUS: f32 = 1.0;
pub const DEFAULT_AGENT_HEIGHT: f32 = 1.0;
pub const DESTINATION_ACCURACY: f32 = 0.1;

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct Agent {
    radius: f32,
}

impl Default for Agent {
    fn default() -> Self {
        Self { radius: DEFAULT_AGENT_RADIUS }
    }
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

#[derive(Component, Clone, Copy, Deref, DerefMut, Default, From, Reflect)]
pub struct Seek(pub Option<Direction2d>);

impl Seek {
    pub fn as_vec(&self) -> Vec2 {
        self.0.map(|d| d.xy()).unwrap_or(Vec2::ZERO)
    }
}

#[derive(Component, Clone, Copy, Deref, DerefMut, Default, From, Reflect)]
pub struct TargetDistance(f32);

#[derive(Stat, Component, Reflect)]
pub struct Speed(f32);

#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Default, Reflect)]
pub struct DesiredVelocity(Vec2);

#[derive(Component, Default, Reflect)]
pub struct TargetReached;

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub enum TargetReachedCondition {
    Distance(f32),
}

impl TargetReachedCondition {
    #[inline]
    pub fn has_reached_target(&self, agent: &Agent, target_distance: f32) -> bool {
        match self {
            TargetReachedCondition::Distance(distance) => {
                target_distance < (agent.radius + distance + DESTINATION_ACCURACY)
            }
        }
    }
}

pub(super) fn setup(mut commands: Commands, agents: Query<Entity, Added<Agent>>) {
    for entity in &agents {
        commands.entity(entity).insert((DesiredVelocity::default(), Seek(None), TargetDistance(0.0)));
    }
}

type MovingAgents = (With<Agent>, Without<TargetReached>);

pub(super) fn seek(mut agents: Query<(Option<&Seek>, &Speed, &mut DesiredVelocity), MovingAgents>) {
    agents.par_iter_mut().for_each(|(seek, speed, mut desired_velocity)| {
        if let Some(seek) = seek
            && let Some(dir) = **seek
        {
            **desired_velocity = dir.xy() * **speed;
        }
    });
}

pub(super) fn apply_velocity(mut agents: Query<(&mut DesiredVelocity, &mut Movement), MovingAgents>) {
    agents.par_iter_mut().for_each(|(mut desired_velocity, mut movement)| {
        if desired_velocity.is_approx_zero() {
            return;
        }

        **movement = **desired_velocity;
        desired_velocity.reset();
    });
}

// TODO: Obstacle if TargetReached or Stationary for x-amount of seconds & x-distance to goal.

pub(super) fn target_reached(
    commands: ParallelCommands,
    mut agents: Query<(Entity, &Agent, &TargetDistance, &TargetReachedCondition, Has<TargetReached>), With<Agent>>,
) {
    agents.par_iter_mut().for_each(|(entity, agent, distance, target_reached_condition, target_reached)| {
        commands.command_scope(|mut c| {
            if target_reached_condition.has_reached_target(&agent, **distance) {
                if !target_reached {
                    c.entity(entity).insert(TargetReached);
                }
            } else if target_reached {
                c.entity(entity).remove::<TargetReached>();
            }
        });
    });
}

pub(super) fn obstacle(
    commands: ParallelCommands,
    mut agents: Query<
        (
            Entity,
            &TargetDistance,
            Has<Obstacle>,
            Option<&ActiveDuration<Stationary>>,
            Has<Stationary>,
            Has<TargetReached>,
        ),
        With<Agent>,
    >,
) {
    agents.par_iter_mut().for_each(
        |(entity, target_distance, has_obstacle, state_duration, is_stationary, target_reached)| {
            commands.command_scope(|mut c| {
                const MIN_STATIONARY_TIME: f32 = 5.0;
                const MIN_TARGET_DISTANCE: f32 = 50.0;
                let should_be_obstacle = target_reached
                    || (**target_distance <= MIN_TARGET_DISTANCE
                        && is_stationary
                        && state_duration.is_some()
                        && state_duration.unwrap().duration().as_secs_f32() >= MIN_STATIONARY_TIME);
                if should_be_obstacle {
                    if !has_obstacle {
                        c.entity(entity).insert(Obstacle);
                    }
                } else if has_obstacle {
                    c.entity(entity).remove::<Obstacle>();
                }
            });
        },
    );
}
