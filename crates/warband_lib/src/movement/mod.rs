use self::motor::{DampingFactor, Jump, JumpHeight, MaxSlopeAngle, Movement};
use crate::{
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

        app.configure_sets(FixedUpdate, (MovementSystems::Setup).chain().run_if(in_state(AppState::InGame)));
        app.configure_sets(
            FixedPostUpdate,
            (MovementSystems::Motor).chain().before(PhysicsSet::Prepare).run_if(in_state(AppState::InGame)),
        );
        app.configure_sets(Last, (MovementSystems::State).chain().run_if(in_state(AppState::InGame)));

        app.add_systems(
            FixedPostUpdate,
            (motor::jumping, (motor::movement, motor::damping).chain()).in_set(MovementSystems::Motor),
        );

        app.add_systems(
            FixedLast,
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
