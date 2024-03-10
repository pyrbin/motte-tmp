use bevy_xpbd_3d::parry::{
    na::{Const, OPoint},
    shape::TypedShape,
};

use crate::{flow_field::FieldLayout, navigation::agent::DEFAULT_AGENT_HEIGHT, prelude::*};

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Obstacle;

#[derive(Component, Clone, Default, Reflect)]
#[reflect(Component)]
pub enum Occupancy {
    #[default]
    Empty,
    Shape(SmallVec<[Vec2; 8]>),
}

impl Occupancy {
    pub fn shape(&self) -> Option<&[Vec2]> {
        match self {
            Occupancy::Empty => None,
            Occupancy::Shape(shape) => Some(shape),
        }
    }
}

#[derive(Component, Clone, Copy, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct OccupancyAabb {
    pub min: Vec2,
    pub max: Vec2,
}

impl OccupancyAabb {
    #[inline]
    pub fn center(self) -> Vec2 {
        (self.min + self.max) / 2.0
    }
    #[inline]
    pub fn size(self) -> Vec2 {
        self.max - self.min
    }
}

pub(super) fn setup(mut commands: Commands, obstacles: Query<Entity, (With<Obstacle>, Without<Occupancy>)>) {
    for entity in &obstacles {
        commands.entity(entity).insert(Occupancy::Empty).insert(OccupancyAabb::default());
    }
}

pub(super) fn occupancy(
    field_layout: Res<FieldLayout>,
    mut obstacles: Query<
        (&mut Occupancy, &mut OccupancyAabb, &Collider, &ColliderAabb, &GlobalTransform),
        Or<(ChangedPhysicsPosition, Changed<Collider>, Changed<ColliderAabb>)>,
    >,
) {
    const PLANE_Y: f32 = 0.0;
    let border_expansion = field_layout.cell_size();

    obstacles.par_iter_mut().for_each(|(mut occupancy, mut occupancy_aabb, collider, aabb, global_transform)| {
        if aabb.min.y > DEFAULT_AGENT_HEIGHT || aabb.max.y < PLANE_Y {
            if !matches!(*occupancy, Occupancy::Empty) {
                *occupancy = Occupancy::Empty;
                *occupancy_aabb = OccupancyAabb::default();
            }
            return;
        }

        occupancy_aabb.min = aabb.min.xz() - border_expansion;
        occupancy_aabb.max = aabb.max.xz() + border_expansion;

        const SUBDIVISIONS: u32 = 8;
        let Some((vertices, _)) = (match collider.shape_scaled().as_typed_shape() {
            TypedShape::Ball(ball) => ball.to_outline(SUBDIVISIONS).into(),
            TypedShape::Cuboid(cuboid) => cuboid.to_outline().into(),
            TypedShape::Capsule(capsule) => capsule.to_outline(SUBDIVISIONS).into(),
            TypedShape::Cylinder(cylinder) => cylinder.to_outline(SUBDIVISIONS).into(),
            TypedShape::Cone(cone) => cone.to_outline(SUBDIVISIONS).into(),
            _ => {
                error!("Failed to convert shape to outline polylines.");
                None
            }
        }) else {
            return;
        };

        // Very simple and naive approach to get a intersection shape for the collider on the 2d plane.
        let transform: Transform = global_transform.compute_transform();
        let vertices: Vec<OPoint<f32, Const<2>>> = vertices
            .iter()
            .map(|v| OPoint::<f32, Const<2>>::from_slice(&[v.x, v.z])) // Ignore y-coordinate
            .collect();

        if vertices.len() < 3 {
            *occupancy = Occupancy::Empty;
            return;
        }

        let mut shape: SmallVec<[Vec2; 8]> = parry2d::transformation::convex_hull(&vertices)
            .iter()
            .map(|point| transform.transform_point(Vec3::new(point.x, 0.0, point.y)).xz())
            .collect();

        expand_shape(&mut shape, border_expansion);

        *occupancy = Occupancy::Shape(shape);
    });
}

#[inline]
fn expand_shape(hull: &mut SmallVec<[Vec2; 8]>, expansion: f32) {
    // perf: could probably be improved
    let center = hull.iter().fold(Vec2::ZERO, |acc, p| acc + *p) / hull.len() as f32; // Calculate center of polygon
    for point in hull.iter_mut() {
        let direction = *point - center;
        let normalized_direction = direction.normalize();
        let expanded_point = center + normalized_direction * (direction.length() + expansion);
        *point = expanded_point;
    }
}

pub(super) fn cleanup(mut commands: Commands, mut occupancy: RemovedComponents<Occupancy>) {
    for entity in &mut occupancy.read() {
        if let Some(mut commands) = commands.get_entity(entity) {
            commands.remove::<OccupancyAabb>();
        }
    }
}
