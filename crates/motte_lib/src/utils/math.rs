#![allow(unused)]
use bevy::utils::petgraph::matrix_graph::Zero;

use crate::prelude::*;

/// Calculate the intersection point of a vector and a plane defined as a point
/// and normal vector where `pv` is the vector point, `dv` is the vector
/// direction, `pp` is the plane point and `np` is the planes' normal vector
#[inline]
pub fn plane_intersection(pv: Vec3, dv: Vec3, pp: Vec3, np: Vec3) -> Vec3 {
    // TODO: use Ray3d & plane intersection
    let d = dv.dot(np);
    let t = (pp.dot(np) - pv.dot(np)) / d;
    pv + dv * t
}

/// Calculates origin and direction of a ray from cursor ndc to world space.
#[inline]
pub fn world_space_ray_from_ndc(ndc: Vec2, camera: &Camera, camera_transform: &GlobalTransform) -> (Vec3, Vec3) {
    let camera_inverse_matrix = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
    let near = camera_inverse_matrix * Vec3::new(ndc.x, -ndc.y, -1.0).extend(1.0);
    let far = camera_inverse_matrix * Vec3::new(ndc.x, -ndc.y, 1.0).extend(1.0);
    let near = near.truncate() / near.w;
    let far = far.truncate() / far.w;
    let dir: Vec3 = far - near;
    (near, dir)
}

#[allow(unused)]
#[inline]
pub fn random_point_in_square(size: f32) -> Vec2 {
    let x = random::<f32>() * size - size / 2.0;
    let y = random::<f32>() * size - size / 2.0;
    Vec2::new(x, y)
}

/// ref: https://github.com/Jondolf/barry/blob/main/src/utils/point_in_poly2d.rs
#[inline]
pub fn point_in_poly2d(pt: Vec2, poly: &[Vec2]) -> bool {
    if poly.len() == 0 {
        false
    } else {
        let mut sign = 0.0;

        for i1 in 0..poly.len() {
            let i2 = (i1 + 1) % poly.len();
            let seg_dir = poly[i2] - poly[i1];
            let dpt = pt - poly[i1];
            let perp = dpt.perp_dot(seg_dir);

            if sign.is_zero() {
                sign = perp;
            } else if sign * perp < 0.0 {
                return false;
            }
        }

        true
    }
}

#[inline]
pub fn determinant(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}
