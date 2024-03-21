use bevy_xpbd_3d::parry::{
    na::{Const, OPoint},
    shape::TypedShape,
};

use crate::prelude::*;

#[derive(Component, Clone, Default, Reflect)]
#[reflect(Component)]
pub enum Obstacle {
    #[default]
    Empty,
    Shape(SmallVec<[Vec2; 8]>),
}

impl Obstacle {
    pub fn is_empty(&self) -> bool {
        matches!(self, Obstacle::Empty)
    }

    pub fn line_segments(&self) -> Option<SmallVec<[(Vec2, Vec2); 4]>> {
        let Obstacle::Shape(shape) = self else {
            return None;
        };

        let mut segments = SmallVec::default();
        for i in 0..shape.len() - 1 {
            segments.push((shape[i], shape[i + 1]));
        }

        segments.push((shape[shape.len() - 1], shape[0]));

        Some(segments)
    }
}

pub(super) fn obstacle(
    mut obstacles: Query<
        (&mut Obstacle, &Collider, &ColliderAabb, &GlobalTransform),
        Or<(Changed<GlobalTransform>, Changed<Collider>, Changed<ColliderAabb>)>,
    >,
) {
    // TODO: sample height if/whenever we have a generated height-field.
    let plane_height = 0.0;
    // TODO: we would need another solution to properly support varying agent heights, not a concern for now tho.
    let max_agent_height = 1.0;

    let border_expansion = 0.0;

    obstacles.par_iter_mut().for_each(|(mut obstacle, collider, aabb, global_transform)| {
        if aabb.min.y > max_agent_height || aabb.max.y < plane_height {
            if !obstacle.is_empty() {
                *obstacle = Obstacle::Empty;
            }
            return;
        }

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
            *obstacle = Obstacle::Empty;
            return;
        }

        let mut shape: SmallVec<[Vec2; 8]> = parry2d::transformation::convex_hull(&vertices)
            .iter()
            .map(|point| transform.transform_point(Vec3::new(point.x, 0.0, point.y)).xz())
            .collect();

        if border_expansion > 0.0 {
            expand_shape(&mut shape, border_expansion);
        }

        *obstacle = Obstacle::Shape(shape);
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

#[cfg(feature = "debug")]
pub(crate) fn gizmos(mut gizmos: Gizmos, obstacles: Query<&mut Obstacle>) {
    for obstacle in obstacles.iter() {
        match obstacle {
            Obstacle::Empty => {}
            Obstacle::Shape(_) => {
                let Some(segments) = obstacle.line_segments() else {
                    continue;
                };
                for (start, end) in segments {
                    gizmos.line(start.x0y(), end.x0y(), Color::RED);
                }
            }
        }
    }
}
