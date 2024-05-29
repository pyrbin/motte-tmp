//! Projectile
use std::marker::ConstParamTy;

use crate::prelude::*;

#[derive(
    Component, Default, Debug, ConstParamTy, Clone, Display, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[reflect(Component)]
#[repr(u8)]
pub enum Projectile {
    #[default]
    Beam,
    Missile,
    Area,
}

#[derive(Component, Default, Reflect)]
pub struct ProjectileType<const PROJECTILE: Projectile>;

// Spell Origin
#[derive(Component, Default, Reflect)]
pub struct Origin(Vec3);

// Spell Origin Offset + Owner(Entity) Translation
#[derive(Component, Default, Reflect)]
pub struct DynamicOrigin(Vec3);

#[derive(Component, Default, Reflect)]
pub enum ProjectileMotion {
    #[default]
    Velocity,
    Instant,
}

pub(super) fn projectile_type<const PROJECTILE: Projectile>(
    commands: ParallelCommands,
    projectiles: Query<(Entity, &Projectile), (Changed<Projectile>, Without<ProjectileType<PROJECTILE>>)>,
    mut removed: RemovedComponents<Projectile>,
) {
    projectiles.par_iter().for_each(|(entity, projectile)| {
        commands.command_scope(|mut c| {
            if *projectile == PROJECTILE {
                c.entity(entity).insert(ProjectileType::<PROJECTILE>);
            }
        });
    });

    for entity in &mut removed.read() {
        commands.command_scope(|mut c| {
            if let Some(mut commands) = c.get_entity(entity) {
                commands.remove::<ProjectileType<PROJECTILE>>();
            }
        });
    }
}

pub(super) fn motion() {}
