use bevy::{
    asset::load_internal_asset,
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    pbr::ShadowFilteringMethod,
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

use bevy_xpbd_3d::PhysicsSet;
pub use camera::*;
use node::PixelateNode;
use pipeline::PixelatePipeline;
pub use snap::{Snap, SnappedTransform};

pub(crate) mod constants {
    use bevy::prelude::UVec2;
    pub const MIN_SCALE_FACTOR: f32 = 1.0;
    pub const MIN_PIXELS_PER_UNIT: u8 = 1;
    pub const MIN_RENDER_TEXTURE_SIZE: UVec2 = UVec2::splat(1);
}

/// Set for the systems related to snapping transforms & cameras.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SnapSystems {
    Revert,
    Camera,
    Transforms,
    Apply,
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
            .register_type::<SnappedTransform>();

        use bevy::{render::camera::CameraUpdateSystem, transform::TransformSystem};

        app.configure_sets(First, SnapSystems::Revert.run_if(snap_transforms_camera_active));
        app.configure_sets(
            PostUpdate,
            (SnapSystems::Camera, SnapSystems::Transforms.run_if(snap_transforms_camera_active))
                .chain()
                .after(TransformSystem::TransformPropagate)
                .after(PhysicsSet::Sync)
                .before(CameraUpdateSystem),
        );
        app.configure_sets(
            Last,
            SnapSystems::Apply.run_if(snap_transforms_camera_active).after(SnapSystems::Transforms),
        );

        app.add_plugins((
            ExtractComponentPlugin::<Blitter>::default(),
            ExtractComponentPlugin::<RenderTexture>::default(),
            ExtractComponentPlugin::<ScaleBias>::default(),
            UniformComponentPlugin::<ScaleBias>::default(),
        ));

        app.insert_resource(Msaa::Off);
        app.init_resource::<MainSnapTransformsCamera>();

        app.add_systems(
            Update,
            (camera::setup, camera::orthographic_fixed_height, apply_deferred, camera::render_texture).chain(),
        );

        app.add_systems(First, (snap::revert.run_if(snap_transforms_camera_active)).in_set(SnapSystems::Revert));

        app.add_systems(Update, (snap::setup, camera::main_camera).chain().before(SnapSystems::Camera));

        app.add_systems(
            PostUpdate,
            (snap::camera.in_set(SnapSystems::Camera), snap::transforms.in_set(SnapSystems::Transforms)),
        );

        app.add_systems(Last, camera::blitter.after(SnapSystems::Camera).before(SnapSystems::Apply));
        app.add_systems(Last, snap::apply.in_set(SnapSystems::Apply));

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

pub fn snap_transforms_camera_active(cam: Option<Res<MainSnapTransformsCamera>>) -> bool {
    cam.map(|cam| cam.is_some()).unwrap_or(false)
}
