use bevy_xpbd_3d_interp::plugin::XPBDInterpolationPlugin;

use crate::prelude::*;

pub struct PhysicsPlugin;
impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(XPBDInterpolationPlugin);
    }
}

#[derive(PhysicsLayer)]
pub(crate) enum CollisionLayer {
    Player,
    Units,
    Terrain,
    Sensor,
}
