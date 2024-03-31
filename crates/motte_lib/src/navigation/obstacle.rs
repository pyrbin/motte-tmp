use bevy_xpbd_3d::parry::{
    na::{Const, OPoint},
    shape::TypedShape,
};
use parry2d::shape::ConvexPolygon;

use crate::{navigation::agent::Agent, prelude::*};

#[derive(Component, Clone, Default, Reflect)]
#[reflect(Component)]
pub enum Obstacle {
    #[default]
    Empty,
    Shape(SmallVec<[Vec2; 16]>),
}

impl Obstacle {
    pub fn empty(&self) -> bool {
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
    const FIELD_HEIGHT: f32 = 0.0;
    // TODO: we would need another solution to properly support varying agent heights, not a concern for now tho.
    const MAX_AGENT_HEIGHT: f32 = Agent::LARGEST.height() / 2.0;

    obstacles.par_iter_mut().for_each(|(mut obstacle, collider, aabb, global_transform)| {
        if aabb.min.y > MAX_AGENT_HEIGHT || aabb.max.y < FIELD_HEIGHT {
            if !obstacle.empty() {
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

        // TODO: compute the convex hull more efficiently. Or at least use a 'glam-native' solution
        let Some(mut polygon) = ConvexPolygon::from_convex_hull(
            &parry2d::transformation::convex_hull(&vertices)
                .iter()
                .map(|point| transform.transform_point(Vec3::new(point.x, 0.0, point.y)).xz().into())
                .collect_vec(),
        ) else {
            *obstacle = Obstacle::Empty;
            return;
        };

        const BORDER_EXPANSION: f32 = 1.0;
        if BORDER_EXPANSION > 0.0 {
            polygon = polygon.scaled(&[BORDER_EXPANSION; 2].into()).unwrap();
        }

        *obstacle = Obstacle::Shape(polygon.points().into_iter().map(|p| Vec2::new(p.x, p.y)).collect());
    });
}

#[cfg(feature = "dev_tools")]
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
