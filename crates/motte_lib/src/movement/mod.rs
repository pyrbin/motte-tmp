use bevy_xpbd_3d::{SubstepSchedule, SubstepSet};

use self::motor::{DampingFactor, Jump, JumpHeight, MaxSlopeAngle, Movement};
use crate::{
    active_duration::{active_duration, ActiveDuration},
    app_state::AppState,
    movement::motor::{Airborne, Grounded, Moving, Stationary},
    prelude::*,
    stats::stat::StatPlugin,
};

pub mod motor;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MovementSystems {
    Setup,
    Motor,
    State,
}

pub struct MovementPlugin;
impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Movement, DampingFactor, MaxSlopeAngle, Jump, JumpHeight);
        app_register_types!(
            Stationary,
            Airborne,
            Grounded,
            Moving,
            ActiveDuration<Stationary>,
            ActiveDuration<Airborne>,
            ActiveDuration<Grounded>,
            ActiveDuration<Moving>
        );

        app.add_plugins(StatPlugin::<JumpHeight>::default());

        app.configure_sets(
            FixedUpdate,
            (MovementSystems::Setup, MovementSystems::Motor.before(PhysicsSet::Prepare), MovementSystems::State)
                .chain()
                .run_if(in_state(AppState::InGame)),
        );

        app.add_systems(
            FixedUpdate,
            (motor::jumping, (motor::gravity, motor::movement, motor::damping).chain()).in_set(MovementSystems::Motor),
        );

        app.add_systems(SubstepSchedule, motor::collisions.in_set(SubstepSet::SolveUserConstraints));

        app.add_systems(
            FixedUpdate,
            (
                (motor::grounded, motor::stationary),
                (
                    active_duration::<Stationary>,
                    active_duration::<Airborne>,
                    active_duration::<Grounded>,
                    active_duration::<Moving>,
                ),
            )
                .chain()
                .in_set(MovementSystems::State),
        );
    }
}
