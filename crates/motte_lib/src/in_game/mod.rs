use bevy::render::{
    mesh::VertexAttributeValues,
    texture::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
};

use self::cursor::{CursorClick, CursorPosition};
use crate::{
    app_state::AppState,
    asset_management::{GlbAssets, ImageAssets},
    graphics::pixelate,
    movement::motor::CharacterMotor,
    navigation::{
        agent::{Agent, Speed, TargetReachedCondition},
        flow_field::{
            fields::obstacle::ObstacleField, footprint::Footprint, layout::FieldLayout, pathing::Goal, CellIndex,
        },
        obstacle::Obstacle,
    },
    physics::CollisionLayer,
    player::camera::MainCamera,
    prelude::*,
    utils::math::random_point_in_square,
};

pub struct InGamePlugin;

impl Plugin for InGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), setup);
        app.add_systems(Update, click);

        const DEFAULT_SIZE: (u8, u8) = (150, 150);

        let layout = FieldLayout::new(DEFAULT_SIZE.0, DEFAULT_SIZE.1);
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
    _glb_assets: Res<GlbAssets>,
    mut asset_image: ResMut<Assets<Image>>,
) {
    commands.spawn((
        Name::light("sun"),
        DirectionalLightBundle {
            directional_light: DirectionalLight { illuminance: 5000.0, color: Color::WHITE, ..default() },
            transform: Transform::from_xyz(30., 100., 30.).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
    ));

    // Plane
    let plane_size = 150.0;
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
        Name::unit("plane"),
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
            Name::unit("target"),
            // SceneBundle {
            //     scene: glb_assets.crystal.clone(),
            //     transform: (Vec3::ZERO + Vec3::NEG_Y * 2.5).into_transform(),
            //     ..Default::default()
            // },
            PbrBundle {
                mesh: meshes.add(Mesh::from(Sphere::new(3.0))),
                material: materials.add(Color::GREEN),
                transform: (Vec3::ZERO + Vec3::Y * 3.0).into_transform(),
                ..default()
            },
            pixelate::Snap::translation(),
            Collider::from(Sphere::new(3.0)),
            RigidBody::Static,
            Footprint::default(),
            Obstacle::default(),
            CellIndex::default(),
            Target,
        ))
        .id();

    for i in 0..5 {
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
                material: materials.add(Color::BEIGE),
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
            CollisionLayers::new([CollisionLayer::Terrain], [CollisionLayer::Terrain, CollisionLayer::Units]),
            RigidBody::Static,
            LinearVelocity::ZERO,
            Obstacle::default(),
            CellIndex::default(),
        ));
    }
    // TODO: agents are now broken??
    // for i in 0..1 {
    //     let agent = Agent::Medium; // Agent::ALL[thread_rng().gen_range(0..Agent::ALL.len())];
    //     let translation = random_point_in_square(50.0);
    //     let transform = Vec3::new(translation.x, 1.0, translation.y).into_transform();
    //     let agent = commands
    //         .spawn((
    //             Name::unit(format!("agent {i}")),
    //             PbrBundle {
    //                 mesh: meshes
    //                     .add(Mesh::from(Cylinder { radius: agent.radius(), half_height: agent.height() / 2.0 })),
    //                 material: materials.add(Color::RED),
    //                 transform,
    //                 ..default()
    //             },
    //             CharacterMotor::cylinder(agent.height(), agent.radius()),
    //             pixelate::Snap::translation(),
    //             agent,
    //             Speed::base(100.0),
    //             CellIndex::default(),
    //             TargetReachedCondition::Distance(1.0),
    //         ))
    //         .id();

    //     commands.entity(agent).insert(Goal::Entity(target));
    // }
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
            transform.translation = position + Vec3::Y * 3.0;
        }
    }
}
