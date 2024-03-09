use std::time::Duration;

use crate::{physics::CollisionLayer, prelude::*};

#[derive(Component, Debug, Clone, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct CharacterMotor;

impl CharacterMotor {
    pub fn capsule(height: f32, radius: f32) -> CharacterMotorBundle {
        let collider = Collider::capsule(height, radius);
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vector::ONE * 0.99, 10);

        CharacterMotorBundle {
            movement: default(),
            jump: default(),
            collider,
            rigid_body: RigidBody::Dynamic,
            locked_axes: LockedAxes::ROTATION_LOCKED,
            damping: DampingFactor(0.9),
            max_slope_angle: MaxSlopeAngle(PI * 0.45),
            ground_caster: ShapeCaster::new(caster_shape, Vector::ZERO, Quaternion::default(), Direction3d::NEG_Y),
            collision_layers: CollisionLayers::new(
                [CollisionLayer::Units],
                [CollisionLayer::Player, CollisionLayer::Terrain, CollisionLayer::Sensor],
            ),
            character_motor: default(),
        }
    }
}

#[derive(Bundle)]
pub struct CharacterMotorBundle {
    movement: Movement,
    jump: Jump,
    collider: Collider,
    rigid_body: RigidBody,
    locked_axes: LockedAxes,
    collision_layers: CollisionLayers,
    character_motor: CharacterMotor,
    ground_caster: ShapeCaster,
    damping: DampingFactor,
    max_slope_angle: MaxSlopeAngle,
}

#[derive(Component, Debug, Clone, PartialEq, Deref, Default, DerefMut, Reflect)]
#[reflect(Component)]
pub struct DampingFactor(f32);

#[derive(Component, Debug, Clone, PartialEq, Deref, Default, DerefMut, Reflect)]
#[reflect(Component)]
pub struct MaxSlopeAngle(f32);

#[derive(Component, Debug, Clone, PartialEq, Deref, Default, DerefMut, Reflect)]
#[reflect(Component)]
pub struct Movement(Vec2);

#[derive(Component, Debug, Clone, PartialEq, Deref, Default, DerefMut, Reflect)]
#[reflect(Component)]
pub struct Jump(bool);

#[derive(Stat, Component, Reflect)]
pub struct JumpHeight(f32);

#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
pub struct Grounded;

#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
pub struct Airborne;

#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
pub struct Stationary;

#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
pub struct Moving;

pub(super) fn movement(time: Res<Time>, mut motors: Query<(&mut Movement, &mut LinearVelocity), With<CharacterMotor>>) {
    let dt = time.delta_seconds();
    motors.par_iter_mut().for_each(|(mut movement, mut linvel)| {
        linvel.x += movement.x * dt;
        linvel.z += movement.y * dt;
        movement.reset();
    });
}

pub(super) fn damping(mut motors: Query<(&DampingFactor, &mut LinearVelocity)>) {
    motors.par_iter_mut().for_each(|(damping, mut linvel)| {
        linvel.x *= damping.0;
        linvel.z *= damping.0;
    });
}

pub(super) fn jumping(
    mut motors: Query<(&mut Jump, &JumpHeight, &mut LinearVelocity, Has<Grounded>), With<CharacterMotor>>,
) {
    motors.par_iter_mut().for_each(|(mut jump, jump_height, mut linvel, is_grounded)| {
        if **jump {
            if is_grounded {
                linvel.y = jump_height.0;
            }
            jump.reset();
        }
    });
}

pub(super) fn grounded(
    commands: ParallelCommands,
    motors: Query<
        (Entity, &ShapeHits, &Rotation, Option<&MaxSlopeAngle>, Has<Grounded>, Has<Airborne>),
        (With<CharacterMotor>, Changed<Position>),
    >,
) {
    motors.par_iter().for_each(|(entity, hits, rotation, max_slope_angle, grounded, airborne)| {
        let is_grounded = hits.iter().any(|hit| {
            if let Some(angle) = max_slope_angle {
                rotation.rotate(-hit.normal2).angle_between(Vector::Y).abs() <= angle.0
            } else {
                true
            }
        });

        commands.command_scope(|mut c| {
            if is_grounded {
                if !grounded {
                    c.entity(entity).insert(Grounded);
                }
                if airborne {
                    c.entity(entity).remove::<Airborne>();
                }
            } else {
                if grounded {
                    c.entity(entity).remove::<Grounded>();
                }
                if !airborne {
                    c.entity(entity).insert(Airborne);
                }
            }
        });
    });
}

pub(super) fn stationary(
    commands: ParallelCommands,
    motors: Query<
        (Entity, &Movement, &LinearVelocity, Has<Stationary>, Has<Moving>),
        (With<CharacterMotor>, Or<(Changed<Movement>, Changed<LinearVelocity>)>),
    >,
) {
    motors.par_iter().for_each(|(entity, movement, linvel, stationary, moving)| {
        let is_stationary = movement.is_approx_zero() && linvel.length_squared() <= 0.1;
        commands.command_scope(|mut c| {
            if is_stationary {
                if !stationary {
                    c.entity(entity).insert(Stationary);
                }
                if moving {
                    c.entity(entity).remove::<Moving>();
                }
            } else {
                if stationary {
                    c.entity(entity).remove::<Stationary>();
                }
                if !moving {
                    c.entity(entity).insert(Moving);
                }
            }
        });
    });
}
