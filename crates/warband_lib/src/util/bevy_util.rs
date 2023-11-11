#![allow(unused)]

use crate::prelude::*;

/// Despawn all entities with the given component.
pub fn despawn_all_with<C: Component>(mut commands: Commands, query: Query<Entity, With<C>>) {
    for e in query.iter() {
        commands.entity(e).despawn_recursive();
    }
}
