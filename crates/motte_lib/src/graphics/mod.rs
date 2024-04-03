use bevy::prelude::{App, Plugin};

pub mod materials;
pub mod pixelate;

pub struct GraphicsPlugin;
impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((pixelate::PixelatePlugin, materials::MaterialsPlugin));
    }
}
