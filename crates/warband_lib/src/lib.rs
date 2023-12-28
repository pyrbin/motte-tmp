#![feature(let_chains)]
#![feature(if_let_guard)]
#![feature(const_format_args)]

mod app_state;
mod asset_management;
mod core;
mod flowfield;
mod graphics;
mod in_game;
mod navigation;
mod physics;
mod player;
mod stats;
mod units;
mod util;

#[cfg(feature = "debug")]
mod debug;

mod prelude;
use prelude::*;

pub struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        use crate::app_state::AppState;
        app_register_types!(AppState);
        app.add_state::<AppState>();
        app.add_plugins((
            #[cfg(feature = "debug")]
            debug::DebugPlugin,
            asset_management::AssetManagementPlugin,
            physics::PhysicsPlugin,
            navigation::NavigationPlugin,
            units::UnitsPlugin,
            graphics::GraphicsPlugin,
            player::PlayerPlugin,
            core::CorePlugin,
            stats::StatsPlugin,
            in_game::InGamePlugin,
            flowfield::FlowFieldPlugin,
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
