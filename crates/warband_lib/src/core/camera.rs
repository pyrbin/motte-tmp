use bevy::{ecs::schedule::ScheduleLabel, transform::TransformSystem};

use crate::prelude::*;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CameraDriverSystem {
    Reset,
    Drivers,
    Apply,
}

pub struct CameraPlugin {
    schedule: Interned<dyn ScheduleLabel>,
}

impl CameraPlugin {
    pub fn in_schedule(schedule: impl ScheduleLabel) -> Self {
        Self { schedule: schedule.intern() }
    }
}

impl Default for CameraPlugin {
    fn default() -> Self {
        Self::in_schedule(PostUpdate)
    }
}

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(RigTransform, YawPitch, Offset, Follow, Zoom, Smoothing);

        app.configure_sets(
            self.schedule,
            (CameraDriverSystem::Reset, CameraDriverSystem::Drivers, CameraDriverSystem::Apply)
                // .after(PhysicsSet::Writeback)
                .before(TransformSystem::TransformPropagate)
                // .before(pixelate::SnapSystem::Transforms)
                // .before(pixelate::SnapSystem::Camera)
                .chain(),
        );

        app.add_systems(
            self.schedule,
            (
                reset_rig_transform.in_set(CameraDriverSystem::Reset),
                (driver_yaw_pitch, driver_follow, driver_offset.after(driver_follow), driver_zoom)
                    .in_set(CameraDriverSystem::Drivers),
                sync_rig_transform.in_set(CameraDriverSystem::Apply),
            ),
        );
    }
}

fn reset_rig_transform(mut rig: Query<&mut RigTransform>) {
    for mut rig_transform in &mut rig {
        rig_transform.reset();
    }
}

#[derive(Component, Reflect, Default, Copy, Clone)]
#[reflect(Component)]
pub struct RigTransform {
    pub translation: Vec3,
    pub rotation: Quat,

    // only used for orthographic cameras
    pub zoom: Option<f32>,
}

impl RigTransform {
    pub fn reset(&mut self) {
        self.translation = Vec3::ZERO;
        self.rotation = Quat::IDENTITY;
        self.zoom = None;
    }
}

// Driver components

#[derive(Component, Reflect, Copy, Clone)]
#[reflect(Component)]
pub struct YawPitch {
    // [0, 720]
    pub yaw: f32,
    // [-90, 90]
    pub pitch: f32,
}

impl Default for YawPitch {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: 0.0 }
    }
}

#[allow(unused)]
impl YawPitch {
    pub fn with_yaw_pitch(yaw: f32, pitch: f32) -> Self {
        Self { yaw, pitch }
    }
    pub fn yaw(mut self, yaw: f32) {
        self.yaw = yaw;
    }
    pub fn pitch(mut self, pitch: f32) {
        self.pitch = pitch;
    }
    pub fn rotate_yaw(&mut self, yaw: f32) {
        self.yaw = (self.yaw + yaw) % 720_f32;
    }
    pub fn rotate_pitch(&mut self, pitch: f32) {
        self.pitch = (self.pitch + pitch).clamp(-90.0, 90.0);
    }
    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.rotate_yaw(yaw);
        self.rotate_pitch(pitch);
    }
}

fn driver_yaw_pitch(mut rig: Query<(&mut RigTransform, &YawPitch)>) {
    for (mut rig_transform, yaw_pitch) in &mut rig {
        let rotation = Quat::from_euler(EulerRot::YXZ, yaw_pitch.yaw.to_radians(), yaw_pitch.pitch.to_radians(), 0.0);
        rig_transform.rotation = rotation;
    }
}

#[derive(Component, Reflect, Copy, Clone, Default)]
#[reflect(Component)]
pub struct Offset(pub Vec3);

fn driver_offset(mut rig: Query<(&mut RigTransform, &Offset)>) {
    for (mut rig_transform, offset) in &mut rig {
        rig_transform.translation += offset.0;
    }
}

#[derive(Component, Reflect, Copy, Clone, Default)]
#[reflect(Component)]
pub enum Follow {
    Entity(Entity),
    Position(Vec3),
    #[default]
    None,
}

fn driver_follow(mut rig: Query<(&mut RigTransform, &Follow)>, transforms: Query<&Transform, Without<Follow>>) {
    for (mut rig_transform, follow) in &mut rig {
        match follow {
            Follow::Entity(entity) => {
                if let Ok(transform) = transforms.get(*entity) {
                    rig_transform.translation = transform.translation;
                }
            }
            Follow::Position(position) => {
                rig_transform.translation = *position;
            }
            _ => {}
        }
    }
}

#[derive(Component, Reflect, Copy, Clone, Default)]
#[reflect(Component)]
pub struct Zoom(f32);

impl Zoom {
    pub fn with_zoom(zoom: f32) -> Self {
        Self(zoom)
    }

    pub fn zoom(&self) -> f32 {
        self.0
    }

    #[allow(unused)]
    pub fn delta_zoom(&mut self, zoom: f32) {
        self.set_zoom(self.0 + zoom);
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.0 = zoom;
        self.0 = self.0.max(0.0);
    }
}

fn driver_zoom(mut rig: Query<(&mut RigTransform, &Zoom)>) {
    for (mut rig_transform, zoom) in &mut rig {
        rig_transform.zoom = Some(zoom.0);
    }
}

#[derive(Component, Reflect, Copy, Clone, Default)]
#[reflect(Component)]
pub struct Smoothing {
    pub position: f32,
    pub rotation: f32,
    pub zoom: f32,
}

impl Smoothing {
    pub fn with_position(mut self, position: f32) -> Self {
        self.position = position;
        self
    }
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }
    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }
}

fn sync_rig_transform(
    mut camera: Query<(&mut Transform, &RigTransform, Option<&Smoothing>, Option<&mut Projection>)>,
    time: Res<Time>,
) {
    for (mut camera_transform, rig_transform, smoothing, projection) in &mut camera {
        let mut translation = rig_transform.translation;
        let mut rotation = rig_transform.rotation;

        if let Some(smoothing) = smoothing {
            const SMOOTHNESS_MULT: f32 = 8.0;

            let smoothstep = |value: f32| 1.0 - (-SMOOTHNESS_MULT * time.delta_seconds() / value.max(1e-5)).exp();

            translation =
                camera_transform.translation.lerp_radius(translation, smoothstep(smoothing.position), f32::EPSILON);

            rotation = Quat::slerp(
                camera_transform.rotation.normalize(),
                rotation.normalize(),
                smoothstep(smoothing.rotation),
            )
            .normalize();

            if rig_transform.zoom.is_some()
                && let Some(mut projection) = projection
                && let Projection::Orthographic(orthographic_projection) = projection.as_mut()
            {
                orthographic_projection.scale = orthographic_projection.scale.lerp_radius(
                    rig_transform.zoom.unwrap(),
                    smoothstep(smoothing.zoom),
                    f32::EPSILON,
                );
            }
        }

        camera_transform.translation = translation;
        camera_transform.rotation = rotation;
    }
}
