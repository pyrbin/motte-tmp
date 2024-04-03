#![allow(incomplete_features)]
#![feature(let_chains)]
#![feature(if_let_guard)]
#![feature(const_format_args)]
#![feature(hash_extract_if)]
#![feature(adt_const_params)]
#![feature(iterator_try_collect)]
#![feature(portable_simd)]
#![feature(const_trait_impl)]
#![feature(effects)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(inherent_associated_types)]
#![feature(const_for)]

mod app_state;
mod asset_management;
mod core;
#[cfg(feature = "dev_tools")]
mod dev_tools;
mod graphics;
mod in_game;
mod movement;
mod navigation;
mod physics;
mod player;
mod prelude;
mod stats;
mod utils;

use prelude::*;

pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        use crate::app_state::AppState;
        app_register_types!(AppState);
        app.init_state::<AppState>();
        app.add_plugins((
            #[cfg(feature = "dev_tools")]
            dev_tools::DevToolsPlugin,
            asset_management::AssetManagementPlugin,
            physics::PhysicsPlugin,
            graphics::GraphicsPlugin,
            player::PlayerPlugin,
            core::CorePlugin,
            stats::StatsPlugin,
            in_game::InGamePlugin,
            navigation::NavigationPlugin,
            movement::MovementPlugin,
        ));
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Semver {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl std::fmt::Display for Semver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub const GIT_VERSION: &str = git_version::git_version!();

lazy_static::lazy_static! {
    pub static ref VERSION: Semver = Semver {
        major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
        patch: env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
    };
}

pub fn version() -> &'static str {
    use const_format::concatcp;
    concatcp!(
        env!("CARGO_PKG_VERSION_MAJOR"),
        ".",
        env!("CARGO_PKG_VERSION_MINOR"),
        ".",
        env!("CARGO_PKG_VERSION_PATCH"),
        "+",
        GIT_VERSION,
    )
}
