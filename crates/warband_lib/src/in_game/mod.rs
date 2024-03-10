use bevy::render::{
    mesh::VertexAttributeValues,
    texture::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
};

use self::cursor::{CursorClick, CursorPosition};
use crate::{
    app_state::AppState,
    asset_management::ImageAssets,
    flow_field::{goal::Goal, CellIndex, CostField, FieldLayout, FlowField},
    graphics::pixelate,
    movement::motor::CharacterMotor,
    navigation::{
        agent::{Agent, DesiredVelocity, Speed, TargetReachedCondition},
        avoidance::{Avoidance, AvoidanceOptions},
        occupancy::Obstacle,
    },
    player::camera::MainCamera,
    prelude::*,
    util::math::random_point_in_square,
};

pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), setup);
        app.add_systems(Update, click);

        const DEFAULT_SIZE: usize = 100;
        const DEFAULT_CELL_SIZE: f32 = 1.0;
        app.insert_resource(FieldLayout::default().with_size(DEFAULT_SIZE).with_cell_size(DEFAULT_CELL_SIZE));
        app.insert_resource(CostField::new(DEFAULT_SIZE));
        app.insert_resource(AvoidanceOptions {
            obstacle_margin: None,
            agent_neighborhood: Some(1.5),
            time_horizon: 1.0,
            obstacle_time_horizon: 0.5,
        });
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
    let plane_size = 200.0;
    let _half_plane_size = plane_size / 2.0;

    let mut mesh_plane = Mesh::from(Plane3d::default().mesh().size(plane_size, plane_size));
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
            material: materials.add(StandardMaterial { base_color_texture: Some(panel), unlit: true, ..default() }),
            transform: Transform::IDENTITY,
            ..default()
        },
        Collider::cuboid(plane_size, 0.1, plane_size),
        pixelate::Snap::translation(),
        RigidBody::Static,
    ));

    let target = commands
        .spawn((
            Name::new("target"),
            PbrBundle {
                mesh: meshes.add(Sphere::new(5.0).mesh().ico(5).unwrap()),
                material: materials.add(Color::BLUE.with_a(0.33)),
                transform: (Vec3::ZERO + Vec3::NEG_Y * 2.5).into_transform(),
                ..default()
            },
            Collider::from(Sphere::new(5.0)),
            RigidBody::Static,
            Obstacle,
            Position::default(),
            CellIndex::default(),
            FlowField::default(),
        ))
        .id();

    for i in 0..16 {
        let translation = random_point_in_square(50.0);
        let radius = thread_rng().gen_range(2.0..3.0);
        let height = thread_rng().gen_range(2.0..6.0);
        commands.spawn((
            Name::new(format!("obstacle {i}")),
            PbrBundle {
                mesh: meshes.add(Mesh::from(Capsule3d::new(radius, height))),
                material: materials.add(Color::RED.with_a(0.5)),
                transform: Vec3::new(translation.x, 0.0, translation.y).into_transform(),
                ..default()
            },
            Collider::from(Capsule3d::new(radius, height)),
            pixelate::Snap::translation(),
            CellIndex::default(),
            RigidBody::Static,
            Obstacle,
            LinearVelocity::ZERO,
        ));
    }
    const RADIUS: f32 = 1.0;
    const HALF_RADIUS: f32 = RADIUS / 2.0;
    for i in 0..50 {
        let translation = random_point_in_square(100.0);
        let transform = Vec3::new(translation.x, 50.0, translation.y).into_transform();
        commands.spawn((
            Name::new(format!("agent {i}")),
            PbrBundle {
                mesh: meshes.add(Mesh::from(Capsule3d { radius: HALF_RADIUS, half_length: RADIUS })),
                material: materials.add(Color::GREEN),
                transform,
                ..default()
            },
            CharacterMotor::capsule(RADIUS, HALF_RADIUS),
            pixelate::Snap::translation(),
            Goal::Entity(target),
            CellIndex::default(),
            Agent::default().with_radius(RADIUS),
            TargetReachedCondition::Distance(1.),
            Avoidance::default().with_neighborhood(2.0),
            DesiredVelocity::default(),
            Speed::base(500.0),
        ));
    }
}

fn click(
    cursor: Res<CursorPosition>,
    mut event_reader: EventReader<CursorClick>,
    mut fields: Query<(&mut Transform, &mut CellIndex, &FlowField)>,
    main_cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    field_layout: Res<FieldLayout>,
) {
    for cursor_click in event_reader.read() {
        if !matches!(cursor_click.button, MouseButton::Right) {
            continue;
        }
        for (mut transform, mut cell_index, _) in &mut fields {
            let (camera, camera_transform) = main_cam.get_single().expect("there should be a main camera");
            let (origin, direction) = math::world_space_ray_from_ndc(cursor.ndc(), camera, camera_transform);
            let position = math::plane_intersection(origin, direction, Vec3::ZERO, Vec3::Y);
            transform.translation = position;
            **cell_index = field_layout.world_to_cell(position);
        }
    }
}