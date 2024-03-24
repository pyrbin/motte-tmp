use core::panic;

use bevy::{
    math::{Affine3A, Vec3A},
    prelude::*,
};

use super::camera::{OrthographicFixedVertical, SnapOffset, SnapTransforms, UnitsPerPixel};

#[inline]
pub fn snap(number: f32, divisor: f32) -> f32 {
    if divisor == 0.0 {
        return number;
    }
    (number / divisor).round() * divisor
}

/// A [`Component`] that configures snapping for a [`Transform`] & all it's descendants.
#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Default)]
#[reflect(Component, Default)]
pub struct Snap {
    translation: bool,
    rotation: bool,
    angle: Option<f32>,
}

impl Snap {
    // Creates a new [`SnapTest`] with no snapping enabled.
    pub fn none() -> Self {
        Self { translation: false, rotation: false, angle: None }
    }
    // Creates a new [`SnapTest`] with translation snapping enabled.
    pub fn translation() -> Self {
        Self { translation: true, rotation: false, angle: None }
    }
    // Creates a new [`SnapTest`] with rotation snapping enabled.
    pub fn rotation() -> Self {
        Self { translation: false, rotation: true, angle: None }
    }
    // Creates a new [`SnapTest`] with translation & rotation snapping enabled.
    pub fn all() -> Self {
        Self { translation: true, rotation: true, angle: None }
    }
    // Sets the angle resolution (in radians) for rotation snapping.
    pub fn with_angle(mut self, angle_radians: f32) -> Self {
        self.angle = Some(angle_radians);
        self
    }
    // Sets the angle resolution (in degrees) for rotation snapping.
    pub fn with_angle_degrees(&mut self, angle_degrees: f32) -> Self {
        self.angle = Some(angle_degrees.to_radians());
        *self
    }

    // Enables translation snapping.
    pub fn snap_translation(mut self) -> Self {
        self.translation = true;
        self
    }

    // Enables rotation snapping.
    pub fn snap_rotation(mut self) -> Self {
        self.translation = true;
        self
    }

    // Disabled translation snapping.
    pub fn unsnap_translation(mut self) -> Self {
        self.translation = true;
        self
    }

    // Disabled rotation snapping.
    pub fn unsnap_rotation(mut self) -> Self {
        self.translation = true;
        self
    }

    // Returns true if translation is snapped
    pub fn is_translation_snapped(&self) -> bool {
        self.translation
    }

    // Returns true if rotation is snapped
    pub fn is_rotation_snapped(&self) -> bool {
        self.rotation
    }

    /// Snaps the given [`Vec3A`] based on the [`Snap`] settings.
    #[inline]
    pub(crate) fn apply_to_vector3a(&self, mut translation: Vec3A, divisor: f32) -> Vec3A {
        if !self.is_translation_snapped() {
            return translation;
        }
        translation /= divisor;
        translation.round() * divisor
    }

    /// Snaps the given [`Quat`] based on the [`Snap`] settings.
    #[inline]
    pub(crate) fn apply_to_rotation(&self, mut rotation: Quat, divisor: f32) -> Quat {
        let divisor = match self.angle {
            Some(angle) => angle,
            _ => divisor,
        };

        if self.is_rotation_snapped() {
            let (x, y, z) = rotation.to_euler(EulerRot::XYZ);
            rotation = Quat::from_euler(EulerRot::XYZ, snap(x, divisor), snap(y, divisor), snap(z, divisor));
        }
        rotation
    }
}

#[derive(Component, Reflect, Clone, Copy, Debug, Default, Deref, DerefMut, PartialEq)]
#[reflect(Component)]
pub struct SnappedTranslation(Vec3A);

impl SnappedTranslation {
    pub(crate) fn new(translation: Vec3A) -> Self {
        Self(translation)
    }
    pub fn translation(&self) -> Vec3 {
        self.0.into()
    }
}

pub(crate) fn add_snapped_translation(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform), (With<Snap>, Without<SnappedTranslation>)>,
) {
    for (entity, global_transform) in &query {
        commands.entity(entity).insert(SnappedTranslation::new(global_transform.translation().into()));
    }
}

#[inline]
pub(crate) fn snap_camera(
    mut commands: Commands,
    mut cameras: Query<
        (
            Entity,
            &mut GlobalTransform,
            &mut Transform,
            &Snap,
            &mut SnappedTranslation,
            &UnitsPerPixel,
            Option<&mut SnapOffset>,
        ),
        With<OrthographicFixedVertical>,
    >,
) {
    for (entity, mut global_transform, _transform, snap, mut snapped_translation, units_per_pixel, mut snap_offset) in
        &mut cameras
    {
        let Some(units_per_pixel) = units_per_pixel.get_value() else {
            warn!("No units per pixel found for camera: {:?}", entity);
            return;
        };

        let mut affine = global_transform.affine();

        let cam_to_world = affine;
        let world_to_cam = cam_to_world.inverse();

        let offset = snap_to_camera_projection_grid(snap, &cam_to_world, &world_to_cam, units_per_pixel, &mut affine);

        **snapped_translation = affine.translation;
        *global_transform = affine.into();

        // to trigger a tranform propagation
        // this will reset the snapped positions
        // transform.set_changed();

        if let Some(ref mut snap_offset) = snap_offset {
            snap_offset.0 = offset;
        } else {
            commands.entity(entity).insert(SnapOffset(offset));
        }
    }
}

/// Iterates transforms with [`Snap`] & their descendants & apply snapping in relation to the
/// active [`SnapTransforms`] camera depending on the [`Snap`] configuration. Currently only
/// supports a single camera with [`SnapTransforms::On`], will panic if more than one is found. This
/// has to run after [`bevy::transform::TransformSystem::TransformPropagate`] to work & assure
/// safety.
#[inline]
pub(super) fn snap_transforms(
    cameras: Query<(Entity, &GlobalTransform, &UnitsPerPixel, &SnapTransforms), With<OrthographicFixedVertical>>,
    mut snappable: Query<
        (&mut GlobalTransform, &mut SnappedTranslation, &Transform, &Snap, Option<&Children>),
        Without<SnapTransforms>,
    >,
    descendants: Query<(&mut GlobalTransform, &Transform, Option<&Children>), (Without<Snap>, Without<SnapTransforms>)>,
) {
    let valid_cameras: Vec<_> =
        cameras.iter().filter(|(_, _, _, snap_transforms)| matches!(snap_transforms, SnapTransforms::On)).collect();

    debug_assert!(
        valid_cameras.len() <= 1,
        "Found more than one camera with SnapTransforms::On, {} cameras found.",
        valid_cameras.len()
    );

    let Some((cam, cam_global_transform, units_per_pixel, _)) = valid_cameras.first() else {
        return;
    };

    let Some(units_per_pixel) = units_per_pixel.get_value() else {
        warn!("No units per pixel found for camera: {:?}, is it Orthographic?", cam);
        return;
    };

    let cam_to_world = cam_global_transform.affine();
    let world_to_cam = cam_to_world.inverse();

    snappable.par_iter_mut().for_each(|(mut global_transform, mut snapped_translation, _transform, snap, children)| {
        let mut affine = global_transform.affine();
        let _ = snap_to_camera_projection_grid(snap, &cam_to_world, &world_to_cam, units_per_pixel, &mut affine);

        **snapped_translation = affine.translation;
        *global_transform = affine.into();

        // to trigger a tranform propagation
        // this will reset the snapped position.
        // transform.set_changed();

        let Some(children) = children else {
            return;
        };

        for &child in children {
            // SAFETY: Save as long as [`propagate_transforms`] & [`sync_simple_transforms`] is
            // ran before this.
            unsafe {
                snap_transforms_recursive(snap, cam_to_world, world_to_cam, units_per_pixel, &descendants, child);
            }
        }
    });
}

/// Recursively snap the transform of an entity and its children.
/// Modified from bevy's [`propagate_transforms`].
unsafe fn snap_transforms_recursive(
    snap: &Snap,
    cam_to_world: Affine3A,
    world_to_cam: Affine3A,
    units_per_pixel: f32,
    transforms: &Query<(&mut GlobalTransform, &Transform, Option<&Children>), (Without<Snap>, Without<SnapTransforms>)>,
    entity: Entity,
) {
    let children = {
        let Ok((mut global_transform, _transform, children)) =
            // SAFETY: This call cannot create aliased mutable references.
            (unsafe { transforms.get_unchecked(entity) })
        else {
            return;
        };

        let mut affine = global_transform.affine();

        snap_to_camera_projection_grid(snap, &cam_to_world, &world_to_cam, units_per_pixel, &mut affine);

        *global_transform = affine.into();

        // to trigger a tranform propagation
        // this will reset the snapped position.
        // transform.set_changed();

        children
    };

    let Some(children) = children else { return };
    for &child in children {
        // SAFETY: The caller guarantees that `transforms` will not be fetched
        // for any descendants of `entity`, so it is safe to call `snap_transforms_recursive` for
        // each child.
        unsafe {
            snap_transforms_recursive(snap, cam_to_world, world_to_cam, units_per_pixel, transforms, child);
        }
    }
}

/// Snaps the given [`Affine3A`] based on the [`Snap`] settings to world space achieved by aligning the transform with a
/// camera rotation. Returns the offset in world space.
#[inline]
fn snap_to_camera_projection_grid(
    snap: &Snap,
    cam_to_world: &Affine3A,
    world_to_cam: &Affine3A,
    units_per_pixel: f32,
    affine: &mut Affine3A,
) -> Vec3A {
    // perf: is there a better way to do this other than `to_scale_rotation_translation`?
    if snap.is_rotation_snapped() {
        let (scale, rotation, _) = affine.to_scale_rotation_translation();
        let snapped_rotation = snap.apply_to_rotation(rotation, units_per_pixel);
        affine.matrix3 = (Mat3::from_quat(snapped_rotation) * Mat3::from_diagonal(scale)).into();
    }

    let grid_position = world_to_cam.transform_vector3a(affine.translation);
    let grid_position_snapped = snap.apply_to_vector3a(grid_position, units_per_pixel);
    affine.translation = cam_to_world.transform_vector3a(grid_position_snapped);

    grid_position - grid_position_snapped
}
