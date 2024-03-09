use std::{
    marker::PhantomData,
    ops::{AddAssign, MulAssign, SubAssign},
};

use super::stat::{DirtyStat, Stat, StatBundle};
use crate::prelude::*;

// TODO: current doesnt really work for negative values.

#[derive(Bundle, Default)]
pub struct PoolBundle<S: Stat + Component> {
    current: Current<S>,
    stat: StatBundle<S>,
}

impl<S: Stat + Component> PoolBundle<S> {
    #[allow(unused)]
    pub fn new(value: f32) -> Self {
        Self { stat: StatBundle::<S>::new(value), current: Current::<S>::new(value) }
    }

    #[allow(unused)]
    pub fn with_current(mut self, value: f32) -> Self {
        self.current.0 = value;
        self
    }
}

impl<S: Stat + Component> From<f32> for PoolBundle<S> {
    fn from(val: f32) -> Self {
        PoolBundle::new(val)
    }
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
pub struct Pool<S: Stat + Component> {
    pub(super) current: &'static mut Current<S>,
    pub(super) total: &'static S,
}

#[allow(unused)]
impl<'w, S: Stat + Component> PoolReadOnlyItem<'w, S> {
    pub fn total(&self) -> f32 {
        self.total.value()
    }

    pub fn current(&self) -> f32 {
        **self.current
    }

    #[inline]
    pub fn percentage(&self) -> f32 {
        pool_perc(self.current(), self.total())
    }
}

#[allow(unused)]
impl<'w, S: Stat + Component> PoolItem<'w, S> {
    pub fn total(&self) -> f32 {
        self.total.value()
    }

    pub fn current(&self) -> f32 {
        **self.current
    }

    /// [0..1]
    #[inline]
    pub fn percentage(&self) -> f32 {
        pool_perc(self.current(), self.total())
    }

    #[inline]
    pub fn set_current(&mut self, value: f32) {
        match pool_clamp(value, self.total()) {
            Ok(value) => self.current.0 = value,
            Err(value) => self.current.0 = value,
        };
    }
}

impl<'w, S: Stat + Component> AddAssign<f32> for PoolItem<'w, S> {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        self.set_current(self.current() + rhs);
    }
}

impl<'w, S: Stat + Component> SubAssign<f32> for PoolItem<'w, S> {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        self.add_assign(rhs * -1.0);
    }
}

impl<'w, S: Stat + Component> MulAssign<f32> for PoolItem<'w, S> {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.set_current(self.current() * rhs);
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect, From)]
#[reflect(Component)]
pub struct Current<S: Stat + Component>(pub(super) f32, #[reflect(ignore)] PhantomData<S>);

#[allow(unused)]
impl<S: Stat + Component> Current<S> {
    pub(super) fn new(value: f32) -> Self {
        Self(value, PhantomData)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

impl<S: Stat + Component> Default for Current<S> {
    fn default() -> Self {
        Self(0.0, PhantomData)
    }
}

impl<S: Stat + Component> std::ops::Deref for Current<S> {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component, Debug, Clone, Copy, Reflect, From)]
#[reflect(Component)]
pub(super) struct DirtyCurrent<S: Stat + Component> {
    pub(super) percentage: f32,
    #[reflect(ignore)]
    _marker: PhantomData<S>,
}

impl<S: Stat + Component> DirtyCurrent<S> {
    pub(super) fn new(percentage: f32) -> Self {
        Self { percentage, _marker: PhantomData }
    }
}

impl<S: Stat + Component> Default for DirtyCurrent<S> {
    fn default() -> Self {
        Self { percentage: 0.0, _marker: PhantomData }
    }
}

pub(super) fn clamp_current<S: Stat + Component>(
    mut stats: Query<Pool<S>, (Changed<Current<S>>, Without<DirtyStat<S>>)>,
) {
    for mut pool in &mut stats {
        if let Err(err) = pool_clamp(pool.current(), pool.total()) {
            pool.current.0 = err;
        }
    }
}

pub(super) fn dirty_current<S: Stat + Component>(
    mut commands: Commands,
    mut stats: Query<(Entity, &S, &Current<S>), With<DirtyStat<S>>>,
) {
    for (entity, stat, current) in &mut stats {
        let percentage = pool_perc(current.value(), stat.value());
        commands.entity(entity).insert(DirtyCurrent::<S>::new(percentage));
    }
}

pub(super) fn cleanup_dirty_current<S: Stat>(
    mut commands: Commands,
    mut stats: Query<(Entity, Pool<S>, &DirtyCurrent<S>)>,
) where
    S: Component,
{
    for (entity, mut pool, dirty_current) in &mut stats {
        let percentage = dirty_current.percentage;
        pool.current.0 = percentage * pool.total();
        commands.entity(entity).remove::<DirtyCurrent<S>>();
    }
}

#[inline]
pub(crate) fn pool_clamp(current: f32, max: f32) -> Result<f32, f32> {
    if current < 0.0 || current > max {
        Err(current.clamp(0.0, max))
    } else {
        Ok(current)
    }
}

#[inline]
pub(crate) fn pool_perc(current: f32, max: f32) -> f32 {
    if current == 0.0 && max == 0.0 {
        return 1.0;
    }

    current / max
}
