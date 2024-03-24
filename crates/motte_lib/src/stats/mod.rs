use self::modifier::Modifies;
use crate::{
    core::previous::{propagate_previous_changed, PreviousValue},
    prelude::*,
};

// TODO: Add configurations for max/min values for a Stat.

// TODO: Add configuration for modifiers to be additive or multiplicative, coefficients, etc.

// TODO: Parallelize stat systems if it has any impact on performance.

pub mod modifier;
pub mod pool;
pub mod stat;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) enum StatSystem {
    Dirty,
    DirtyFlush,
    Reset,
    ResetFlush,
    ModifierFlat,
    ModifierMult,
    Cleanup,
}

#[derive(Default)]
pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app_register_types!(Modifies, PreviousValue<Modifies>);

        app.configure_sets(
            PostUpdate,
            (
                StatSystem::Dirty,
                StatSystem::DirtyFlush,
                StatSystem::Reset,
                StatSystem::ResetFlush,
                StatSystem::ModifierFlat,
                StatSystem::ModifierMult,
                StatSystem::Cleanup,
            )
                .chain(),
        );

        app.add_systems(PostUpdate, apply_deferred.in_set(StatSystem::DirtyFlush));
        app.add_systems(PostUpdate, apply_deferred.in_set(StatSystem::ResetFlush));
        app.add_systems(PostUpdate, propagate_previous_changed::<Modifies>.in_set(StatSystem::Cleanup));
    }
}
