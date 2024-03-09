use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    prelude::*,
    render::{
        extract_component::{ExtractComponentPlugin, UniformComponentPlugin},
        render_graph::{RenderGraphApp, RenderLabel, ViewNodeRunner},
        RenderApp,
    },
};

mod camera;
mod node;
mod pipeline;
mod snap;

pub use camera::*;
use node::PixelateNode;
use pipeline::PixelatePipeline;
use snap::*;
pub use snap::{Snap, SnappedTranslation};

pub(crate) mod constants {
    use bevy::prelude::UVec2;
    pub const MIN_SCALE_FACTOR: f32 = 1.0;
    pub const MIN_PIXELS_PER_UNIT: u8 = 1;
    pub const MIN_RENDER_TEXTURE_SIZE: UVec2 = UVec2::splat(1);
}

/// Set enum for the systems related to snapping transforms & cameras.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SnapSystem {
    Transforms,
    Camera,
}

const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(6669923404675166368);

#[derive(RenderLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PixelateRenderLabel;

pub struct PixelatePlugin;
impl Plugin for PixelatePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER_HANDLE, "pixelate.wgsl", Shader::from_wgsl);

        app.register_type::<Pixelate>()
            .register_type::<SnapTransforms>()
            .register_type::<SubPixelSmoothing>()
            .register_type::<UnitsPerPixel>()
            .register_type::<SnapOffset>()
            .register_type::<OrthographicFixedVertical>()
            .register_type::<RenderResolution>()
            .register_type::<ScaleBias>()
            .register_type::<RenderTexture>()
            .register_type::<Blitter>()
            .register_type::<Snap>()
            .register_type::<SnappedTranslation>();

        use bevy::{render::camera::CameraUpdateSystem, transform::TransformSystem};
        app.configure_sets(
            PostUpdate,
            (SnapSystem::Camera, SnapSystem::Transforms)
                .chain()
                .after(TransformSystem::TransformPropagate)
                .before(CameraUpdateSystem),
        );

        app.add_plugins((
            ExtractComponentPlugin::<Blitter>::default(),
            ExtractComponentPlugin::<RenderTexture>::default(),
            ExtractComponentPlugin::<ScaleBias>::default(),
            UniformComponentPlugin::<ScaleBias>::default(),
        ));

        app.add_systems(
            Update,
            (pixelate_added, sync_orthographic_fixed_height, apply_deferred, pixelate_render_texture).chain(),
        );

        app.add_systems(Update, (add_snapped_translation).chain().before(SnapSystem::Transforms));

        app.add_systems(
            PostUpdate,
            (snap_camera.in_set(SnapSystem::Camera), snap_transforms.in_set(SnapSystem::Transforms)),
        );

        app.add_systems(Last, sync_blitter_camera.after(SnapSystem::Camera));

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<PixelateNode>>(Core2d, PixelateRenderLabel)
            .add_render_graph_edges(
                Core2d,
                (Node2d::ConstrastAdaptiveSharpening, PixelateRenderLabel, Node2d::EndMainPassPostProcessing),
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<PixelatePipeline>();
    }
}
