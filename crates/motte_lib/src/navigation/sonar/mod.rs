use bevy_spatial::{kdtree::KDTree3, SpatialAccess};
use parry2d::na::{SimdComplexField, SimdPartialOrd, SimdRealField};

use super::agent::{Agent, DesiredVelocity};
use crate::prelude::*;

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Segment {
    /// The segment first point.
    pub a: f32,
    /// The segment second point.
    pub b: f32,
}

#[derive(Component)]
pub struct SonarAvoidance {
    pub radius: f32,
    pub angle: f32,
    pub nodes: SmallVec<[SonarNode; 16]>,
}

impl SonarAvoidance {
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.create(Segment { a: -self.angle, b: self.angle });
        let left = self.create(Segment { a: -self.angle, b: 0.0 });
        let right = self.create(Segment { a: 0.0, b: self.angle });
        self.nodes[0] = SonarNode { segment: Segment { a: -self.angle, b: self.angle }, left, right };
    }

    pub fn create(&mut self, segment: Segment) -> SonarNodeHandle {
        self.nodes.push(SonarNode::new(segment));
        SonarNodeHandle::Index(self.nodes.len() as u8 - 1)
    }
}

#[derive(Default, PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[repr(u8)]
pub enum SonarNodeHandle {
    #[default]
    None,
    Index(u8),
}

pub struct SonarNode {
    pub segment: Segment,
    pub left: SonarNodeHandle,
    pub right: SonarNodeHandle,
}

impl SonarNode {
    pub fn new(segment: Segment) -> Self {
        Self { segment, left: SonarNodeHandle::None, right: SonarNodeHandle::None }
    }

    pub fn is_leaf(&self) -> bool {
        self.left == SonarNodeHandle::None && self.right == SonarNodeHandle::None
    }
}

pub fn sonar(
    mut agents: Query<(Entity, &Agent, &Transform, &mut SonarAvoidance, &DesiredVelocity)>,
    other_agents: Query<(&Transform, &Agent)>,
    agents_kd_tree: Res<KDTree3<Agent>>,
) {
    agents.iter_mut().for_each(|(entity, agent, transform, mut sonar, desired_velocity)| {
        let radius = agent.radius() + sonar.radius;
        let position = transform.translation;
        let nodes = &sonar.nodes;

        sonar.clear();

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
            let mut tangent_line_angle = if distance > sonar.radius {
                let a = opp;
                let b = sonar.radius;
                let c = distance;
                ((a * a - b * b - c * c) / (-2.0 * b * c)).simd_acos()
            } else {
                tangent_line_to_circle_angle(opp, hyp)
            };

            let angle = local_direction.y.simd_atan2(local_direction.x);
            let right = angle - tangent_line_angle;
            let left = angle + tangent_line_angle;

            let segment = Segment { a: right, b: left };
            // let node = sonar.nodes[0];
            // let node_segment = node.segment;
        }
    });
}

#[inline]
fn tangent_line_to_circle_angle(radius: f32, distance: f32) -> f32 {
    let hyp = distance.max(radius);
    let opp = radius;
    let angle = (opp / hyp).simd_clamp(-1.0, 1.0).simd_asin();
    angle
}
