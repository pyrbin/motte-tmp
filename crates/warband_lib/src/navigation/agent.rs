use std::marker::ConstParamTy;

use crate::prelude::*;

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct Agent {
    radius: AgentRadius,
    height: f32,
    mass: f32,
}

impl Agent {
    pub fn cylinder(radius: AgentRadius, height: f32) -> AgentBundle {
        AgentBundle { agent: Agent::new(radius, height) }
    }

    pub fn small() -> AgentBundle {
        AgentBundle { agent: Agent::from_radius(AgentRadius::Small) }
    }

    pub fn medium() -> AgentBundle {
        AgentBundle { agent: Agent::from_radius(AgentRadius::Medium) }
    }

    pub fn large() -> AgentBundle {
        AgentBundle { agent: Agent::from_radius(AgentRadius::Large) }
    }

    pub fn huge() -> AgentBundle {
        AgentBundle { agent: Agent::from_radius(AgentRadius::Huge) }
    }

    pub fn from_radius(radius: AgentRadius) -> Self {
        Self::new(radius, radius as u8 as f32 * 2.0)
    }

    pub fn new(radius: AgentRadius, height: f32) -> Self {
        Self { radius, height, mass: 1.0 }
    }

    pub fn with_radius(mut self, radius: AgentRadius) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    pub fn radius(&self) -> AgentRadius {
        self.radius
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn mass(&self) -> f32 {
        self.height
    }
}

#[derive(Bundle)]
pub struct AgentBundle {
    agent: Agent,
}

impl AgentBundle {
    pub fn agent(&mut self) -> &mut Agent {
        &mut self.agent
    }
}

#[derive(Component, Default, ConstParamTy, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Reflect)]
#[repr(u8)]
pub enum AgentRadius {
    #[default]
    Small = 1,
    Medium = 3,
    Large = 5,
    Huge = 7,
}

impl AgentRadius {
    // The largest agent radius.
    pub const LARGEST: Self = Self::Huge;

    // The smallest agent radius.
    pub const SMALLEST: Self = Self::Small;

    // Iterate over all agent radiuses from largest to smallest.
    #[inline]
    pub const fn largest_to_smallest() -> [Self; 4] {
        [Self::Huge, Self::Large, Self::Medium, Self::Small]
    }

    // Get the agent radius as a f32.
    #[inline]
    pub const fn as_f32(self) -> f32 {
        self as u8 as f32
    }
}

impl From<AgentRadius> for u8 {
    fn from(r: AgentRadius) -> u8 {
        r as u8
    }
}

impl From<AgentRadius> for f32 {
    fn from(r: AgentRadius) -> f32 {
        r as u8 as f32
    }
}

impl std::fmt::Display for AgentRadius {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Small => write!(f, "Small"),
            Self::Medium => write!(f, "Medium"),
            Self::Large => write!(f, "Large"),
            Self::Huge => write!(f, "Huge"),
        }
    }
}

#[cfg(feature = "debug")]
pub(crate) fn gizmos(mut gizmos: Gizmos, agents: Query<(&Agent, &GlobalTransform)>) {
    for (agent, transform) in &agents {
        let position = transform.translation();
        gizmos.circle(position.x0z().y_pad(), Direction3d::Y, agent.radius().into(), Color::YELLOW);
        gizmos.line(position.x0z().y_pad(), position.x0z() + agent.height() * Vec3::Y, Color::YELLOW);
        gizmos.circle(position.x0z() + agent.height() * Vec3::Y, Direction3d::Y, agent.radius().into(), Color::YELLOW);
    }
}
