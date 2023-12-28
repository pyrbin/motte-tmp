use crate::prelude::*;

pub struct CharacterControllerPlugin;

// KinematicCharacterMotor

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(CharacterController, Grounded, DampingFactor, JumpImpulse, MaxSlopeAngle, MovementAction);

        app.add_event::<MovementAction>();
        app.add_systems(Update, (check_grounded, apply_deferred, movement, apply_movement_damping).chain());
    }
}

/// An event sent for a movement input action.
#[derive(Event, Reflect)]
pub enum MovementAction {
    Move { entity: Entity, velocity: Vec2 },
    Jump { entity: Entity },
}

#[derive(Bundle)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    rigidbody: RigidBody,
    collider: Collider,
    ground_caster: ShapeCaster,
    locked_axes: LockedAxes,
    movement: MovementBundle,
}

impl CharacterControllerBundle {
    pub fn new(collider: Collider) -> Self {
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vector::ONE * 0.99, 10);

        Self {
            character_controller: CharacterController,
            rigidbody: RigidBody::Kinematic,
            collider,
            ground_caster: ShapeCaster::new(caster_shape, Vector::ZERO, Quaternion::default(), Vector::NEG_Y)
                .with_max_time_of_impact(0.2),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            movement: MovementBundle::default(),
        }
    }

    pub fn with_movement(mut self, damping: f32, jump_impulse: f32, max_slope_angle: f32) -> Self {
        self.movement = MovementBundle::new(damping, jump_impulse, max_slope_angle);
        self
    }
}

#[derive(Bundle)]
pub struct MovementBundle {
    damping: DampingFactor,
    jump_impulse: JumpImpulse,
    max_slope_angle: MaxSlopeAngle,
}

impl MovementBundle {
    pub const fn new(damping: f32, jump_impulse: f32, max_slope_angle: f32) -> Self {
        Self {
            damping: DampingFactor(damping),
            jump_impulse: JumpImpulse(jump_impulse),
            max_slope_angle: MaxSlopeAngle(max_slope_angle),
        }
    }
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self::new(0.9, 7.0, PI * 0.45)
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct CharacterController;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct DampingFactor(f32);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct JumpImpulse(f32);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct MaxSlopeAngle(f32);

fn check_grounded(
    mut commands: Commands,
    mut query: Query<(Entity, &ShapeHits, &Rotation, Option<&MaxSlopeAngle>), With<CharacterController>>,
) {
    for (entity, hits, rotation, max_slope_angle) in &mut query {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let is_grounded = hits.iter().any(|hit| {
            if let Some(angle) = max_slope_angle {
                rotation.rotate(-hit.normal2).angle_between(Vector::Y).abs() <= angle.0
            } else {
                true
            }
        });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

fn movement(
    time: Res<Time>,
    mut movement_event_reader: EventReader<MovementAction>,
    mut controllers: Query<(&JumpImpulse, &mut LinearVelocity, Has<Grounded>)>,
) {
    let delta_time = time.delta_seconds();
    for event in movement_event_reader.read() {
        match event {
            MovementAction::Move { entity, velocity } => {
                let (_jump_impulse, mut linear_velocity, _is_grounded) = controllers.get_mut(*entity).unwrap();
                linear_velocity.x += velocity.x * delta_time;
                linear_velocity.z += velocity.y * delta_time;
            }
            MovementAction::Jump { entity } => {
                let (jump_impulse, mut linear_velocity, is_grounded) = controllers.get_mut(*entity).unwrap();
                if is_grounded {
                    linear_velocity.y = jump_impulse.0;
                }
            }
        }
    }
}

/// Slows down movement in the XZ plane.
fn apply_movement_damping(mut query: Query<(&DampingFactor, &mut LinearVelocity)>) {
    for (damping_factor, mut linear_velocity) in &mut query {
        // We could use `LinearDamping`, but we don't want to dampen movement along the Y axis
        linear_velocity.x *= damping_factor.0;
        linear_velocity.z *= damping_factor.0;
    }
}
