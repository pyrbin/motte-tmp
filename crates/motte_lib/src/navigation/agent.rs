use std::marker::ConstParamTy;

use super::flow_field::{footprint::Footprint, layout::CELL_SIZE, pathing::Goal};
use crate::{movement::motor::Movement, prelude::*};

#[derive(Component, Default, Debug, ConstParamTy, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(Component)]
#[repr(u8)]
pub enum Agent {
    #[default]
    Small = CELL_SIZE,
    Medium = CELL_SIZE * 3,
    Large = CELL_SIZE * 5,
    Huge = CELL_SIZE * 7,
}

impl Agent {
    // The largest agent radius.
    pub const LARGEST: Self = Self::Huge;

    // The smallest agent radius.
    pub const SMALLEST: Self = Self::Small;

    // All agent sizes from large-to-small.
    pub const ALL: [Self; 4] = [Self::Huge, Self::Large, Self::Medium, Self::Small];

    pub const fn radius(&self) -> f32 {
        self.size() / 2.0
    }

    pub const fn size(self) -> f32 {
        self as u8 as f32
    }

    pub const fn height(self) -> f32 {
        self.size()
    }
}

impl std::fmt::Display for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Component, Default, Reflect)]
pub struct AgentType<const AGENT: Agent>;

#[derive(Component, Clone, Copy, Deref, DerefMut, Default, From, Reflect)]
pub struct DesiredDirection(pub Option<Direction2d>);

#[derive(Component, Debug, Clone, Copy, Deref, DerefMut, Default, Reflect)]
pub struct DesiredVelocity(Vec2);

#[derive(Component, Default, Reflect)]
#[component(storage = "SparseSet")]
pub struct Blocking;

#[derive(Stat, Component, Reflect)]
pub struct Speed(f32);

#[derive(Component, Clone, Copy, Deref, DerefMut, Default, From, Reflect)]
pub struct TargetDistance(f32);

#[derive(Component, Default, Reflect)]
#[component(storage = "SparseSet")]
pub struct TargetReached;

#[derive(Component, Debug, Clone, Copy, Reflect)]
pub enum TargetReachedCondition {
    Distance(f32),
}

impl TargetReachedCondition {
    #[inline]
    pub fn has_reached_target(&self, agent: &Agent, target_distance: f32) -> bool {
        pub const DESTINATION_ACCURACY: f32 = 0.1;
        pub const MAX_DESTINATION_ACCURACY: f32 = 0.5;

        match self {
            TargetReachedCondition::Distance(distance) => {
                target_distance
                    < (agent.radius()
                        + distance
                        + (DESTINATION_ACCURACY * agent.radius()).max(MAX_DESTINATION_ACCURACY))
            }
        }
    }
}

pub(super) fn setup(mut commands: Commands, agents: Query<Entity, Added<Agent>>) {
    for entity in &agents {
        commands.entity(entity).insert((DesiredVelocity::default(), DesiredDirection(None), TargetDistance(0.0)));
    }
}

type MovingAgents = (With<Agent>, Without<TargetReached>);

pub(super) fn desired_velocity(
    mut agents: Query<(Option<&DesiredDirection>, &Speed, &mut DesiredVelocity), MovingAgents>,
) {
    agents.par_iter_mut().for_each(|(desired_direction, speed, mut desired_velocity)| {
        if let Some(desired_direction) = desired_direction
            && let Some(dir) = **desired_direction
        {
            **desired_velocity = dir.xy() * **speed;
        }
    });
}

pub(super) fn apply_velocity(mut agents: Query<(&mut DesiredVelocity, &mut Movement), MovingAgents>) {
    agents.par_iter_mut().for_each(|(mut desired_velocity, mut movement)| {
        if desired_velocity.is_approx_zero() {
            desired_velocity.reset();
            return;
        }

        **movement = **desired_velocity;
        desired_velocity.reset();
    });
}

pub(super) fn target_reached(
    commands: ParallelCommands,
    mut agents: Query<
        (Entity, &Agent, &DesiredDirection, &TargetDistance, &TargetReachedCondition, Has<TargetReached>),
        With<Agent>,
    >,
) {
    agents.par_iter_mut().for_each(
        |(entity, agent, desired_direction, distance, target_reached_condition, target_reached)| {
            commands.command_scope(|mut c| {
                if desired_direction.is_some() && target_reached_condition.has_reached_target(agent, **distance) {
                    if !target_reached {
                        c.entity(entity).insert(TargetReached);
                    }
                } else if target_reached {
                    c.entity(entity).remove::<TargetReached>();
                }
            });
        },
    );
}

pub(super) fn blocking(
    commands: ParallelCommands,
    idle: Query<Entity, (With<Agent>, Or<(Without<Goal>, With<TargetReached>)>, Without<Blocking>)>,
    pathing: Query<Entity, (With<Agent>, With<Goal>, Without<TargetReached>, With<Blocking>)>,
) {
    idle.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).insert((Footprint::default(), Blocking));
        });
    });

    pathing.par_iter().for_each(|entity| {
        commands.command_scope(|mut c| {
            c.entity(entity).remove::<Footprint>().remove::<Blocking>();
        });
    });
}

pub(super) fn agent_type<const AGENT: Agent>(
    commands: ParallelCommands,
    agents: Query<(Entity, &Agent), (Changed<Agent>, Without<AgentType<AGENT>>)>,
    mut removed: RemovedComponents<Agent>,
) {
    agents.par_iter().for_each(|(entity, agent)| {
        commands.command_scope(|mut c| {
            if *agent == AGENT {
                c.entity(entity).insert(AgentType::<AGENT>);
            }
        });
    });

    for entity in &mut removed.read() {
        commands.command_scope(|mut c| {
            if let Some(mut commands) = c.get_entity(entity) {
                commands.remove::<AgentType<AGENT>>();
            }
        });
    }
}

#[cfg(feature = "dev_tools")]
pub(crate) fn gizmos(mut gizmos: Gizmos, agents: Query<(&Agent, &GlobalTransform)>) {
    for (agent, transform) in &agents {
        let position = transform.translation();
        gizmos.circle(position.x0z().y_pad(), Direction3d::Y, agent.radius(), Color::YELLOW);
        gizmos.line(position.x0z().y_pad(), position.x0z() + agent.height() * Vec3::Y, Color::YELLOW);
        gizmos.circle(position.x0z() + agent.height() * Vec3::Y, Direction3d::Y, agent.radius(), Color::YELLOW);
    }
}
