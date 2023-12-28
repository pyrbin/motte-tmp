use bevy::render::{
    mesh::VertexAttributeValues,
    texture::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
};

use crate::{
    app_state::AppState,
    asset_management::ImageAssets,
    graphics::pixelate,
    navigation::{
        agent::{Agent, DesiredVelocity, Hold, TargetReachedCondition},
        avoidance::Avoidance,
        pathing::PathTarget,
    },
    physics::character_controller::CharacterControllerBundle,
    player::camera::MainCamera,
    prelude::{cursor::CursorDoubleClick, *},
};

pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), setup);
        app.add_systems(Update, move_to);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    image_assets: Res<ImageAssets>,
    mut asset_image: ResMut<Assets<Image>>,
) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight { illuminance: 5000.0, color: Color::WHITE, ..default() },
        transform: Transform::from_xyz(30., 100., 30.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Plane
    let plane_size = 96.0;
    let half_plane_size = plane_size / 2.0;

    let mut mesh_plane = Mesh::from(shape::Plane { size: plane_size, subdivisions: 16 });
    match mesh_plane.attribute_mut(Mesh::ATTRIBUTE_UV_0).unwrap() {
        VertexAttributeValues::Float32x2(uvs) => {
            for uv in uvs {
                uv[0] *= 16.0; // Make the UV 4x larger, so 4x4 = 16 images
                uv[1] *= 16.0;
            }
        }
        _ => panic!(),
    };

    let panel = image_assets.proto_dark.clone();
    let panel_image = asset_image.get_mut(&panel).unwrap(); // Assuming image is already loaded
    match &mut panel_image.sampler {
        ImageSampler::Default => {
            panel_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                address_mode_w: ImageAddressMode::Repeat,
                ..default()
            });
        }
        ImageSampler::Descriptor(sampler_descriptor) => {
            sampler_descriptor.address_mode_u = ImageAddressMode::Repeat;
            sampler_descriptor.address_mode_v = ImageAddressMode::Repeat;
            sampler_descriptor.address_mode_w = ImageAddressMode::Repeat;
        }
    };

    commands.spawn((
        Name::new("plane"),
        PbrBundle {
            mesh: meshes.add(mesh_plane),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(panel),
                unlit: true,
                ..Default::default()
            }),
            transform: Transform::IDENTITY,
            ..default()
        },
        Collider::cuboid(plane_size, 0.1, plane_size),
        pixelate::Snap::translation(),
        RigidBody::Static,
        oxidized_navigation::NavMeshAffector,
    ));

    commands.spawn((
        Name::new("wall"),
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 8.0 })),
            material: materials.add(Color::BLUE.into()),
            transform: (Vec3::X * (half_plane_size - 10.0) + Vec3::Z * (half_plane_size - 10.0)).into_transform(),
            ..default()
        },
        Collider::cuboid(8.0, 8.0, 8.0),
        pixelate::Snap::translation(),
        RigidBody::Static,
        oxidized_navigation::NavMeshAffector,
    ));

    let avoidance = 3.0;
    for i in 0..10 {
        let mouse_button = if i % 2 == 0 { MouseButton::Left } else { MouseButton::Right };
        let color = if i % 2 == 0 { Color::GREEN.into() } else { Color::RED.into() };
        let name = if i % 2 == 0 { "green" } else { "red" };

        let point = math::random_point_in_square(half_plane_size - 5.0);
        commands.spawn((
            Name::new(format!("agent {} ({})", i, name)),
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                material: materials.add(color),
                transform: Vec3::new(point.x, 0.5, point.y).into_transform(),
                ..default()
            },
            pixelate::Snap::translation(),
            GravityScale(2.0),
            Group(mouse_button),
            CharacterControllerBundle::new(Collider::cuboid_splat(1.0)).with_movement(0.92, 7.0, 30.0_f32.to_radians()),
            Agent::default().with_radius(1.0),
            Avoidance::default().with_neighbourhood(avoidance),
            DesiredVelocity(Vec3::ZERO),
        ));
    }
}

#[derive(Component, DerefMut, Deref)]
struct Group(pub(self) MouseButton);

fn move_to(
    mut commands: Commands,
    agents: Query<(Entity, &Group), With<Agent>>,
    mut cursor: EventReader<CursorDoubleClick>,
    main_cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let (camera, camera_transform) = main_cam.get_single().expect("there should be a main camera");
    for click in cursor.read() {
        let (origin, direction) = math::world_space_ray_from_ndc(click.ndc, camera, camera_transform);
        let position = math::plane_intersection(origin, direction, Vec3::ZERO, Vec3::Y);
        for (agent, group) in agents.iter() {
            if **group != click.button {
                continue;
            }
            commands.entity(agent).insert(PathTarget::Position(position)).insert(TargetReachedCondition::Distance(5.0));
            commands.entity(agent).remove::<Hold>();
        }
    }
}
