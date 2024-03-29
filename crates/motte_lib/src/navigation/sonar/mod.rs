use bevy_spatial::{kdtree::KDTree3, SpatialAccess};

use super::agent::{Agent, DesiredVelocity};
use crate::prelude::*;

#[derive(Component)]
pub struct SonarAvoidance {
    pub radius: f32,
    pub angle: f32,
    pub nodes: SmallVec<[SonarNode; 16]>,
}

#[derive(Default, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[repr(u8)]
pub enum SonarNodeHandle {
    #[default]
    None,
    Index(u8),
}

pub struct SonarNode {
    pub segment: Segment2d,
    pub left: SonarNodeHandle,
    pub right: SonarNodeHandle,
}

impl SonarNode {
    pub fn new(segment: Segment2d) -> Self {
        Self { segment, left: SonarNodeHandle::None, right: SonarNodeHandle::None }
    }

    pub fn is_leaf(&self) -> bool {
        self.left == SonarNodeHandle::None && self.right == SonarNodeHandle::None
    }
}

pub fn sonar(
    mut agents: Query<(Entity, &Agent, &Transform, &SonarAvoidance, &DesiredVelocity)>,
    other_agents: Query<(&Transform, &Agent)>,
    agents_kd_tree: Res<KDTree3<Agent>>,
) {
    agents.iter_mut().for_each(|(entity, agent, transform, sonar, desired_velocity)| {
        let radius = agent.radius() + sonar.radius;
        let position = transform.translation;

        for (neighbor_transform, neighbor_agent) in
            agents_kd_tree.within_distance(position, radius).iter().filter_map(|(_, other)| {
                other.filter(|&other| other != entity).and_then(|other| other_agents.get(other).ok())
            })
        {
            let local_position =
                transform.rotation.conjugate().mul_vec3(neighbor_transform.translation - position).xz();
            let local_direction = local_position.normalize_or_zero();
            let distance = local_position.length();

            let opp = neighbor_agent.radius() + agent.radius();
            let hyp = distance.max(neighbor_agent.radius());
        }
    });
}
