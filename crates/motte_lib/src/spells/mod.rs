//! Spells
use crate::prelude::*;

mod projectile;

// #[derive(Stat, Component, Reflect)]
// pub struct Affinity<T: Reflect + TypePath> {
//     #[stat(value)]
//     value: f32,
//     #[reflect(ignore)]
//     _marker: PhantomData<T>,
// }

#[derive(Component, Reflect, Default, Clone, Copy)]
#[reflect(Component)]
pub enum DeliveryMethod {
    #[default]
    Beam,
    Projectile,
    Area,
}

#[derive(Component, Reflect, Default, Clone, Copy)]
#[reflect(Component)]
pub enum Target {
    Location(Vec3),
    Entity(Entity),
    #[default]
    None,
}

// shared delivery
#[derive(Stat, Component, Reflect)]
#[reflect(Component)]
pub struct Speed(f32);

#[derive(Stat, Component, Reflect)]
#[reflect(Component)]
pub struct Size(f32);
