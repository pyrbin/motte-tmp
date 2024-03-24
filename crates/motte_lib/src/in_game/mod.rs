use bevy::render::{
    mesh::VertexAttributeValues,
    texture::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
};

use self::cursor::{CursorClick, CursorPosition};
use crate::{
    app_state::AppState,
    asset_management::ImageAssets,
    graphics::pixelate,
    movement::motor::CharacterMotor,
    navigation::{
        agent::{Agent, Speed},
        avoidance::{
            Avoidance, AvoidanceVelocity, ContrainedTranslation, ExtrapolatedTranslation, PreConstraintTranslation,
        },
        flow_field::{
            fields::obstacle::ObstacleField, footprint::Footprint, layout::FieldLayout, pathing::Goal, CellIndex,
        },
        obstacle::Obstacle,
    },
    player::camera::MainCamera,
    prelude::*,
    utils::math::random_point_in_square,
};

pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), setup);
        app.add_systems(Update, click);

        const DEFAULT_SIZE: (usize, usize) = (16 * 8, 9 * 8);
        const DEFAULT_CELL_SIZE: f32 = 1.0;

        let layout = FieldLayout::new(DEFAULT_SIZE.0, DEFAULT_SIZE.1).with_cell_size(DEFAULT_CELL_SIZE);
        let obstacles = ObstacleField::from_layout(&layout);

        app.insert_resource(layout);
        app.insert_resource(obstacles);
    }
}

#[derive(Component)]
pub struct Target;

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
                mesh: meshes.add(Sphere::new(3.0).mesh().ico(5).unwrap()),
                material: materials.add(Color::BLUE.with_a(0.33)),
                transform: (Vec3::ZERO + Vec3::NEG_Y * 2.5).into_transform(),
                ..default()
            },
            Collider::from(Sphere::new(3.0)),
            RigidBody::Static,
            Position::default(),
            Footprint::default(),
            Obstacle::default(),
            CellIndex::default(),
            Target,
        ))
        .id();

    for i in 0..10 {
        let translation = random_point_in_square(70.0);
        let radius = thread_rng().gen_range(2.0..3.0);
        let height = thread_rng().gen_range(2.0..6.0);
        let shape = thread_rng().gen_range(0..2) >= 1;

        commands.spawn((
            Name::unit(format!("obstacle {i}")),
            PbrBundle {
                mesh: meshes.add(if shape {
                    Mesh::from(Capsule3d::new(radius, height))
                } else {
                    Mesh::from(Cuboid { half_size: Vec3::ONE * height })
                }),
                material: materials.add(Color::PURPLE),
                transform: Vec3::new(translation.x, 0.0, translation.y).into_transform(),
                ..default()
            },
            Footprint::default(),
            if shape {
                Collider::from(Capsule3d::new(radius, height))
            } else {
                Collider::from(Cuboid { half_size: Vec3::ONE * height })
            },
            pixelate::Snap::translation(),
            RigidBody::Static,
            LinearVelocity::ZERO,
            Obstacle::default(),
            CellIndex::default(),
        ));
    }
    const RADIUS: f32 = 1.0;
    const HALF_RADIUS: f32 = RADIUS / 2.0;

    for i in 0..10 {
        let agent = Agent::Medium; // Agent::ALL[thread_rng().gen_range(0..Agent::ALL.len())];
        let translation = random_point_in_square(30.0);
        let transform = Vec3::new(translation.x, 1.0, translation.y).into_transform();
        commands.spawn((
            Name::unit(format!("agent {i}")),
            PbrBundle {
                mesh: meshes.add(Mesh::from(Cylinder { radius: agent.radius(), half_height: agent.height() / 2.0 })),
                material: materials.add(Color::GREEN.with_a(0.75)),
                transform,
                ..default()
            },
            Goal::Entity(target),
            CharacterMotor::cylinder(RADIUS, HALF_RADIUS),
            pixelate::Snap::translation(),
            agent,
            Speed::base(200.0),
            CellIndex::default(),
            Avoidance::new(2.0),
            InverseMass(1.0),
            ExtrapolatedTranslation::default(),
            PreConstraintTranslation::default(),
            ContrainedTranslation::default(),
            AvoidanceVelocity::default(),
            // Footprint::default(),
        ));
    }
}

fn click(
    cursor: Res<CursorPosition>,
    mut event_reader: EventReader<CursorClick>,
    mut fields: Query<(&mut Transform, &mut CellIndex), With<Target>>,
    main_cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    _field_layout: Res<FieldLayout>,
) {
    for cursor_click in event_reader.read() {
        if !matches!(cursor_click.button, MouseButton::Right) {
            continue;
        }
        for (mut transform, _cell_index) in &mut fields {
            let (camera, camera_transform) = main_cam.get_single().expect("there should be a main camera");
            let (origin, direction) = math::world_space_ray_from_ndc(cursor.ndc(), camera, camera_transform);
            let position = math::plane_intersection(origin, direction, Vec3::ZERO, Vec3::Y);
            transform.translation = position;
        }
    }
}
