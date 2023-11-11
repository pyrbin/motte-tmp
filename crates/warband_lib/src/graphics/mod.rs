use bevy::prelude::{App, Msaa, Plugin};

pub mod pixelate;
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off);
        app.add_plugins((pixelate::PixelatePlugin,));
    }
}
