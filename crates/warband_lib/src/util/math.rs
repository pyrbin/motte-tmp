#![allow(unused)]

use crate::prelude::*;

#[inline]
pub fn saturate(v: Vec2) -> Vec2 {
    let x = v.x.max(0.0).min(1.0);
    let y = v.y.max(0.0).min(1.0);
    Vec2::new(x, y)
}

/// Calculate the intersection point of a vector and a plane defined as a point
/// and normal vector where `pv` is the vector point, `dv` is the vector
/// direction, `pp` is the plane point and `np` is the planes' normal vector
#[inline]
pub fn plane_intersection(pv: Vec3, dv: Vec3, pp: Vec3, np: Vec3) -> Vec3 {
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
