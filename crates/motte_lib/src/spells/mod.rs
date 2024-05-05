//! Spells
use bevy::ecs::reflect;

use crate::prelude::*;

#[derive(Component, Reflect)]
pub enum Element {
    Fire,
}

pub struct Fire;

#[derive(Stat, Component, Reflect)]
pub struct Affinity<T: Reflect + TypePath> {
    #[stat(value)]
    value: f32,
    #[reflect(ignore)]
    _marker: PhantomData<T>,
}
