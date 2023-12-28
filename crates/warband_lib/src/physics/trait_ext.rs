use bevy_xpbd_3d::{math::Scalar, prelude::Collider};

pub trait ColliderExt {
    fn cuboid_splat(size: Scalar) -> Self;
    fn cuboid_plane(width: Scalar, height: Scalar) -> Self;
}

impl ColliderExt for Collider {
    fn cuboid_splat(size: Scalar) -> Self {
        Self::cuboid(size, size, size)
    }

    fn cuboid_plane(width: Scalar, height: Scalar) -> Self {
        Self::cuboid(width, 0.1, height)
    }
}
