#![allow(unused)]

use std::simd::{f32x4, num::SimdFloat};

use parry2d::na::SimdPartialOrd;

use crate::prelude::*;

pub(crate) trait ColorExt {
    /// Convert this type into a `Transform`.
    fn lerp(&self, rhs: Color, s: f32) -> Color;
}

impl ColorExt for Color {
    #[inline]
    fn lerp(&self, rhs: Color, s: f32) -> Color {
        let linear_self = self.as_rgba_linear();
        let linear_rhs = rhs.as_rgba_linear();
        linear_self * (1.0 - s) + linear_rhs * s
    }
}

pub(crate) trait IntoTransform {
    /// Convert this type into a `Transform`.
    fn into_transform(self) -> Transform;
}

impl IntoTransform for Vec3 {
    #[inline]
    fn into_transform(self) -> Transform {
        Transform::from_xyz(self.x, self.y, self.z)
    }
}

impl IntoTransform for Quat {
    #[inline]
    fn into_transform(self) -> Transform {
        Transform::from_rotation(self)
    }
}

pub(crate) trait TransformExt {
    fn horizontally_looking_at(self, target: Vec3, up: Vec3) -> Transform;
}

impl TransformExt for Transform {
    #[inline]
    fn horizontally_looking_at(self, target: Vec3, up: Vec3) -> Transform {
        let direction = target - self.translation;
        let horizontal_direction = direction - up * direction.dot(up);
        let look_target = self.translation + horizontal_direction;
        self.looking_at(look_target, up)
    }
}

pub(crate) trait LerpRadius {
    /// Linearly interpolate between two values, but if the distance between them is less than the radius, return the
    /// other value.
    fn lerp_radius(self, other: Self, t: f32, radius: f32) -> Self;
}

impl LerpRadius for f32 {
    #[inline]
    fn lerp_radius(self, other: Self, t: f32, radius: f32) -> Self {
        let mut result = bevy::prelude::FloatExt::lerp(self, other, t);
        if (result - other).abs() < radius {
            result = other;
        }
        result
    }
}

impl LerpRadius for Vec3 {
    #[inline]
    fn lerp_radius(self, other: Self, t: f32, radius: f32) -> Self {
        let mut result = self.lerp(other, t);
        if (result - other).length_squared() < radius {
            result = other;
        }
        result
    }
}

impl LerpRadius for Quat {
    #[inline]
    fn lerp_radius(self, other: Self, t: f32, radius: f32) -> Self {
        let mut result = self.lerp(other, t);
        if (result - other).length_squared() < radius {
            result = other;
        }
        result
    }
}

pub(crate) trait Clamp01 {
    fn clamp01(self) -> Self;
}

impl Clamp01 for f32 {
    #[inline]
    fn clamp01(self) -> Self {
        self.simd_clamp(0.0, 1.0)
    }
}

impl Clamp01 for Vec2 {
    #[inline]
    fn clamp01(self) -> Self {
        self.clamp(Vec2::ZERO, Vec2::ONE)
    }
}

impl Clamp01 for Vec3 {
    #[inline]
    fn clamp01(self) -> Self {
        self.clamp(Vec3::ZERO, Vec3::ONE)
    }
}

impl Clamp01 for Vec4 {
    #[inline]
    fn clamp01(self) -> Self {
        self.clamp(Vec4::ZERO, Vec4::ONE)
    }
}

impl Clamp01 for f32x4 {
    #[inline]
    fn clamp01(self) -> Self {
        self.simd_clamp(f32x4::splat(0.0), f32x4::splat(0.0))
    }
}

pub(crate) trait F32Ext: Copy {
    fn is_approx_zero(self) -> bool;
}

impl F32Ext for f32 {
    #[inline]
    fn is_approx_zero(self) -> bool {
        self.abs() < f32::EPSILON
    }
}

pub(crate) trait Vec2Ext: Copy {
    fn is_approx_zero(self) -> bool;
    fn x0y(self) -> Vec3;
    fn x_y(self, y: f32) -> Vec3;
}

impl Vec2Ext for Vec2 {
    #[inline]
    fn is_approx_zero(self) -> bool {
        self.length_squared() < f32::EPSILON
    }

    #[inline]
    fn x0y(self) -> Vec3 {
        Vec3::new(self.x, 0., self.y)
    }

    fn x_y(self, y: f32) -> Vec3 {
        Vec3::new(self.x, y, self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SplitVec3 {
    pub(crate) vertical: Vec3,
    pub(crate) horizontal: Vec3,
}

pub(crate) trait Vec3Ext: Copy {
    fn is_approx_zero(self) -> bool;
    fn split(self, up: Vec3) -> SplitVec3;
    fn x0z(self) -> Vec3;
    // Pads the y coordinate by 0.1 to prevent clipping (mostly for debug rendering).
    fn y_pad(self) -> Vec3;
}

impl Vec3Ext for Vec3 {
    #[inline]
    fn is_approx_zero(self) -> bool {
        self.length_squared() < f32::EPSILON
    }

    #[inline]
    fn split(self, up: Vec3) -> SplitVec3 {
        let vertical = up * self.dot(up);
        let horizontal = self - vertical;
        SplitVec3 { vertical, horizontal }
    }

    #[inline]
    fn x0z(self) -> Vec3 {
        Vec3::new(self.x, 0., self.z)
    }

    #[inline]
    fn y_pad(self) -> Vec3 {
        self + Vec3::Y * 0.1
    }
}

pub trait Reset: Default {
    fn reset(&mut self);
}

impl<T: Default> Reset for T {
    #[inline]
    fn reset(&mut self) {
        *self = T::default();
    }
}

pub trait Zero: PartialEq + Sized {
    const ZERO: Self;

    fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }
}

impl Zero for Vec3 {
    const ZERO: Self = Self::ZERO;
}

impl Zero for f32 {
    const ZERO: Self = 0.0;
}

// Base trait for types that can be wrapped in a [`NotZero`](struct.NotZero.html).
//
// Implementors must provide a singleton object that will be used to mark empty edges in a
// [`MatrixGraph`](struct.MatrixGraph.html).
//
// Note that this trait is already implemented for the base numeric types.
// pub trait Zero {
// Return the singleton object which can be used as a sentinel value.
// fn zero() -> Self;
//
// Return true if `self` is equal to the sentinel value.
// fn is_zero(&self) -> bool;
// }
//
// macro_rules! not_zero_impl {
// ($t:ty,$z:expr) => {
// impl Zero for $t {
// fn zero() -> Self {
// $z as $t
// }
//
// #[allow(clippy::float_cmp)]
// fn is_zero(&self) -> bool {
// self == &Self::zero()
// }
// }
// };
// }
//
// macro_rules! not_zero_impls {
// ($($t:ty),*) => {
// $(
// not_zero_impl!($t, 0);
// )*
// }
// }
//
// not_zero_impls!(u8, u16, u32, u64, usize);
// not_zero_impls!(i8, i16, i32, i64, isize);
// not_zero_impls!(f32, f64);
//
