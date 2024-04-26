use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    render::render_resource::*,
};

use crate::prelude::*;

pub type CelMaterial = ExtendedMaterial<StandardMaterial, CelExtension>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct CelExtension {
    #[uniform(100)]
    pub lit: f32,
    #[uniform(100)]
    pub shadow: f32,
    #[uniform(100)]
    pub cut_off: f32,
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
        Self { lit: 1.0, shadow: 0.5, cut_off: 0.5 }
    }
}
