use bevy::{
    core_pipeline::{
        clear_color::ClearColorConfig,
        prepass::{DepthPrepass, NormalPrepass},
    },
    input::mouse::MouseWheel,
};

use crate::{graphics::pixelate, prelude::*};
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, controls);
        app.add_systems(Last, sync_ui_world_camera);
    }
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct UiWorldCamera;

#[derive(Component)]
pub struct UiCamera;

fn setup(mut commands: Commands, _asset_server: Res<AssetServer>) {
    let main_camera = commands
        .spawn((
            MainCamera,
            Name::new("main_camera"),
            Camera3dBundle {
                camera: Camera { order: -1, ..Default::default() },
                camera_3d: Camera3d { clear_color: ClearColorConfig::Custom(Color::BLACK), ..Default::default() },
                projection: pixelate::orthographic_fixed_vertical(1.0, 30.0, -100.0, 200.0),
                ..Default::default()
            },
            UiCameraConfig { show_ui: false },
            DepthPrepass,
            NormalPrepass,
            camera::RigTransform::default(),
            camera::Zoom::with_zoom(30.0),
            camera::YawPitch::with_yaw_pitch(0.0, -55.0),
            camera::Smoothing::default().with_position(0.0).with_rotation(2.0).with_zoom(0.0),
            pixelate::Pixelate::PixelsPerUnit(6),
            pixelate::SnapTransforms::On,
            pixelate::Snap::translation(),
            pixelate::SubPixelSmoothing::On,
        ))
        .id();

    // commands.spawn((
    //     Camera3dBundle {
    //         camera: Camera { order: 1, ..Default::default() },
    //         camera_3d: Camera3d { clear_color: ClearColorConfig::None, ..Default::default() },
    //         projection: pixelate::orthographic_fixed_vertical(1.0, 30.0, -100.0, 200.0),
    //         ..Default::default()
    //     },
    //     UiCameraConfig { show_ui: false },
    //     UiWorldCamera,
    //     RenderLayers::layer(2),
    // ));

    commands.spawn((
        UiCamera,
        Name::new("ui_camera"),
        Camera2dBundle { ..Default::default() },
        UiCameraConfig { show_ui: true },
        pixelate::Blitter(main_camera.into()),
    ));
}

fn controls(
    mut camera: Query<(&mut camera::YawPitch, &mut camera::Zoom), With<MainCamera>>,
    mut scroll: EventReader<MouseWheel>,
    input: Res<Input<KeyCode>>,
) {
    for (mut yaw_pitch, mut zoom) in &mut camera {
        let yaw_input = if input.just_pressed(KeyCode::Q) { 1.0 } else { 0.0 }
            - if input.just_pressed(KeyCode::E) { 1.0 } else { 0.0 };

        yaw_pitch.rotate_yaw(yaw_input * 90.0);

        let pitch_input = if input.just_pressed(KeyCode::S) { 1.0 } else { 0.0 }
            - if input.just_pressed(KeyCode::W) { 1.0 } else { 0.0 };

        yaw_pitch.rotate_pitch(pitch_input * 5.0);

        if input.just_pressed(KeyCode::R) {
            yaw_pitch.pitch = -35.0;
            yaw_pitch.yaw = 180.0;
        }

        for event in scroll.iter() {
            let zoom_scale = zoom.zoom();
            zoom.set_zoom((zoom_scale - event.y).clamp(1.0, 60.0));
        }
    }
}

fn sync_ui_world_camera(
    main_camera: Query<(&Transform, &GlobalTransform, &Projection), (With<MainCamera>, Without<UiWorldCamera>)>,
    mut ui_world_camera: Query<
        (&mut Transform, &mut GlobalTransform, &mut Projection),
        (Without<MainCamera>, With<UiWorldCamera>),
    >,
) {
    let (main_transform, main_global, main_proj) = main_camera.single();
    for (mut transform, mut global, mut proj) in &mut ui_world_camera {
        *transform = *main_transform;
        *global = *main_global;
        *proj = main_proj.clone();
    }
}
