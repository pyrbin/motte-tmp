//! Prelude for internal use.
#![allow(unused)]

pub(crate) use std::{default, marker::PhantomData};

pub(crate) use bevy::{
    ecs::{query::QueryData, schedule::ScheduleLabel},
    log::*,
    math::*,
    prelude::*,
    reflect::{GetTypeRegistration, TypePath},
    utils::{intern::Interned, HashMap, HashSet},
};
pub(crate) use bevy_xpbd_3d::{math::*, prelude::*};
pub(crate) use derive_more::From;
pub(crate) use itertools::Itertools;
pub(crate) use motte_macros::*;
pub(crate) use rand::prelude::*;
pub(crate) use smallvec::SmallVec;
pub(crate) use thiserror::Error;

#[cfg(feature = "dev_tools")]
pub(crate) use crate::dev_tools::*;
pub(crate) use crate::{
    core::*,
    stats::stat::Stat,
    utils::{trait_ext::*, *},
};
