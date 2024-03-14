pub mod field;
pub mod fields;
pub mod layout;

use crate::prelude::*;

pub struct FlowFieldPlugin;

impl Plugin for FlowFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, fields::density::update);
    }
}
