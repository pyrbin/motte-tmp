use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    render::render_resource::*,
};

use crate::prelude::*;

pub type CelMaterial = ExtendedMaterial<StandardMaterial, CelExtension>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct CelExtension {
    #[uniform(100)]
    pub luminance_bands: f32,
    #[uniform(100)]
    pub luminance_power: f32,
    #[uniform(100)]
    pub dither_factor: f32,
}

impl MaterialExtension for CelExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/cel.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/cel.wgsl".into()
    }
}

impl Default for CelExtension {
    fn default() -> Self {
        Self { luminance_bands: 8.0, luminance_power: 2.0, dither_factor: 0.0 }
    }
}
