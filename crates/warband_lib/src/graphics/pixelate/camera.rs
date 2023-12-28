use bevy::{
    math::{Vec3A, Vec3Swizzles},
    prelude::*,
    render::{
        camera::{CameraProjection, ScalingMode},
        extract_component::ExtractComponent,
        render_resource::{Extent3d, ShaderType},
        texture::ImageSampler,
    },
    window::{PrimaryWindow, WindowResized},
};

use super::{constants, snap::Snap};

/// A [`Bundle`] with all components required to setup a pixelate camera.
#[derive(Bundle)]
pub struct PixelateBundle {
    pub pixelate: Pixelate,
    pub snap: Snap,
    pub sub_pixel_smoothing: SubPixelSmoothing,
    pub snap_transforms: SnapTransforms,
}

impl Default for PixelateBundle {
    fn default() -> Self {
        Self {
            pixelate: Pixelate::ScaleFactor(1.0),
            snap: Snap::translation(),
            sub_pixel_smoothing: SubPixelSmoothing::On,
            snap_transforms: SnapTransforms::On,
        }
    }
}

/// If added to a [`bevy::prelude::Camera3d`] then the camera will render to a texture instead of
/// the screen. The texture can then be blitted to the screen using a [`Blitter`] camera.
/// Currently assumes there will only be one [`Blitter`] camera & one [`Pixelate`] camera.
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
pub enum Pixelate {
    /// The texture will be rendered at resolution to achieve a fixed number of pixels per unit
    /// (world units). This is currently only supported for cameras with an
    /// [`OrthographicProjection`] & [`bevy::render::camera::ScalingMode::FixedVertical`] scaling
    /// mode.
    PixelsPerUnit(u8),
    /// The texture will be rendered at a fixed resolution.
    Fixed(u32, u32),
    /// The texture will be rendered at a fixed scale factor ([0..1]) of the window resolution.
    ScaleFactor(f32),
}

impl Default for Pixelate {
    fn default() -> Self {
        Self::PixelsPerUnit(constants::MIN_PIXELS_PER_UNIT)
    }
}

impl Pixelate {
    /// Desired render resolution of the render texture based on it's configuration & the provided
    /// window resolution. An [`OrthographicProjection`] &
    /// [`bevy::render::camera::ScalingMode::FixedVertical`] projection is required for
    /// [`Pixelate::PixelsPerUnit`] variant.
    #[inline]
    pub(crate) fn render_resolution(
        &self,
        window_resolution: UVec2,
        orthographic_fixed_height: Option<&OrthographicFixedVertical>,
    ) -> UVec2 {
        let render_resolution = match *self {
            Self::PixelsPerUnit(pixels_per_unit) => {
                let Some(orthographic_fixed_height) = orthographic_fixed_height else {
                    warn!(
                        "PixelsPerUnit is only supported for cameras with an OrthographicProjection & \
                         ScalingMode::FixedVertical scaling mode."
                    );
                    return window_resolution;
                };

                let pixel_scale =
                    orthographic_fixed_height.height * pixels_per_unit as f32 * orthographic_fixed_height.scale;

                let scale_factor = (window_resolution.x as f32 / pixel_scale)
                    .max(constants::MIN_SCALE_FACTOR)
                    .min(window_resolution.y as f32 / pixel_scale);

                UVec2::new(
                    (window_resolution.x as f32 / scale_factor) as u32,
                    (window_resolution.y as f32 / scale_factor) as u32,
                )
            }
            Self::Fixed(width, height) => UVec2::new(width, height),
            Self::ScaleFactor(scale_factor) => {
                let scale_factor = scale_factor.min(constants::MIN_SCALE_FACTOR).max(0.0);
                UVec2::new(
                    (window_resolution.x as f32 * scale_factor) as u32,
                    (window_resolution.y as f32 * scale_factor) as u32,
                )
            }
        };

        render_resolution.min(window_resolution).max(constants::MIN_RENDER_TEXTURE_SIZE)
    }
}

/// Disables or enables sub-pixel smoothing. Only supported for [`OrthographicProjection`] &
/// [`bevy::render::camera::ScalingMode::FixedVertical`] cameras.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub enum SubPixelSmoothing {
    #[default]
    On,
    Off,
}

impl SubPixelSmoothing {
    pub fn is_on(&self) -> bool {
        matches!(self, Self::On)
    }
}

/// Disables or enables snapping logic for transforms with [`Snap`] for this camera (excluding the actual camera).
/// !!! It's assumed only one [`SnapTransforms::On`] camera is active at a time.
// TODO: want a better name for this. Or restructure how Pixelate/Snap feature components API.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub enum SnapTransforms {
    #[default]
    On,
    Off,
}

/// Handle to the texture that the camera renders to.
#[derive(Component, Reflect, Clone, Debug, Default, PartialEq, Eq, ExtractComponent)]
#[extract_component_filter((With<Camera2d>, With<Camera>))]
#[reflect(Component)]
pub enum RenderTexture {
    /// Texture handle.
    Texture(Handle<Image>),
    /// The texture has not been initialized yet.
    #[default]
    Uninitialized,
}

impl RenderTexture {
    pub fn handle(&self) -> Option<Handle<Image>> {
        match self {
            Self::Texture(handle) => Some(handle.clone()),
            _ => None,
        }
    }
    pub(crate) fn set_handle(&mut self, handle: Handle<Image>) {
        *self = Self::Texture(handle);
    }
}

/// Caches the [`OrthographicProjection`] height & scale for a [`Camera3d`] with an
/// [`OrthographicProjection`] & [`bevy::render::camera::ScalingMode::FixedVertical`] scaling mode.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub(crate) struct OrthographicFixedVertical {
    pub(crate) height: f32,
    pub(crate) scale: f32,
}

/// Creates a new [`OrthographicProjection`] with [ScalingMode::FixedVertical].
pub fn orthographic_fixed_vertical(height: f32, scale: f32, near: f32, far: f32) -> Projection {
    OrthographicProjection { scale, scaling_mode: ScalingMode::FixedVertical(height), near, far, ..default() }.into()
}

/// Offset applied when snapping the camera.
/// Used in [`ScaleBias`] when blitting the texture to the [`Blitter`].
#[derive(Component, Reflect, Clone, Copy, Debug, Deref, DerefMut, Default)]
#[reflect(Component)]
pub struct SnapOffset(pub(crate) Vec3A);
impl SnapOffset {
    pub fn offset(&self) -> Vec3A {
        self.0
    }
}

/// Units per pixel for [`Pixelate`] camera. This is only available for cameras with an
/// [`OrthographicProjection`] & [`bevy::render::camera::ScalingMode::FixedVertical`] scaling mode.
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub enum UnitsPerPixel {
    Value(f32),
    #[default]
    Unavailable,
}

impl UnitsPerPixel {
    pub fn get_value(&self) -> Option<f32> {
        match self {
            Self::Value(value) => Some(*value),
            _ => None,
        }
    }
    pub fn value(&self) -> f32 {
        match self {
            Self::Value(value) => *value,
            _ => panic!("UnitsPerPixel is unavailable."),
        }
    }
    pub fn value_or(&self, default: f32) -> f32 {
        match self {
            Self::Value(value) => *value,
            _ => default,
        }
    }
}

#[derive(Component, Reflect, Clone, Copy, Debug, Deref, DerefMut, Default)]
#[reflect(Component)]
pub struct RenderResolution(pub(crate) UVec2);
impl RenderResolution {
    pub fn value(&self) -> UVec2 {
        self.0
    }
}

/// If added to a [`bevy::prelude::Camera2d`] & it's value is a valid entity with a [`Pixelate`]
/// component, then the render texture from that entity will be blitted to render target.
#[derive(Component, Reflect, Clone, Copy, Debug, Deref, DerefMut, Default, ExtractComponent)]
#[extract_component_filter((With<Camera2d>, With<Camera>))]
#[reflect(Component)]
pub struct Blitter(pub Option<Entity>);

/// Scale bias applied when blitting the texture to the screen camera for smooth sub-pixel movement.
/// This is derived from the [`SnapOffset`] generated when snapping the [`Pixelate`] camera.
#[derive(Component, Reflect, Clone, Copy, Debug, Default, ShaderType, ExtractComponent)]
#[extract_component_filter((With<Camera2d>, With<Camera>))]
#[reflect(Component)]
pub(crate) struct ScaleBias {
    pub scale: Vec2,
    pub bias: Vec2,
}

impl ScaleBias {
    #[allow(unused)]
    pub(crate) fn new(scale: Vec2, bias: Vec2) -> Self {
        Self { scale, bias }
    }
    /// Creates a new [`ScaleBias`] with the given bias & a scale of [`Vec2::ONE`].
    pub(crate) fn with_bias(bias: Vec2) -> Self {
        Self { scale: Vec2::ONE, bias }
    }
}

/// Sets the [`OrthographicFixedVertical`] component for all [`Pixelate`] cameras with an
/// [`OrthographicProjection`] and [`ScalingMode::FixedVertical`] scaling mode.
pub(crate) fn sync_orthographic_fixed_height(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut Projection), (With<Camera3d>, Changed<Projection>)>,
) {
    for (entity, mut projection) in &mut cameras {
        let mut found_fixed_height = false;
        if let Projection::Orthographic(orthographic_projection) = projection.as_mut() {
            if let ScalingMode::FixedVertical(orthographic_fixed_world_height) = orthographic_projection.scaling_mode {
                commands.entity(entity).insert(OrthographicFixedVertical {
                    height: orthographic_fixed_world_height.abs(),
                    scale: orthographic_projection.scale.abs(),
                });
                found_fixed_height = true;
            }
        }
        if !found_fixed_height {
            commands.entity(entity).remove::<OrthographicFixedVertical>();
        }
    }
}

/// Initializes required components for all [`Pixelate`] cameras when they are added.
pub(crate) fn pixelate_added(mut commands: Commands, cameras: Query<Entity, Added<Pixelate>>) {
    for camera in cameras.iter() {
        commands
            .entity(camera)
            .insert(RenderTexture::default())
            .insert(RenderResolution(UVec2::ONE))
            .insert(UnitsPerPixel::Unavailable);
    }
}

/// Initializes and/or resizes the render textures of all [`Pixelate`]s if any changes are detected.
pub(crate) fn pixelate_render_texture(
    mut cameras: Query<(
        Entity,
        &mut Camera,
        &mut Projection,
        Ref<Pixelate>,
        &mut RenderTexture,
        Option<&OrthographicFixedVertical>,
        &mut RenderResolution,
        &mut UnitsPerPixel,
    )>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut window_resized_events: EventReader<WindowResized>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok((window_entity, window)) = windows.get_single() else {
        warn!("No window found.");
        return;
    };

    let window_changed = window_resized_events.read().any(|e| e.window == window_entity);

    let window_resolution = UVec2::new(window.physical_width(), window.physical_height());

    for (
        _entity,
        mut camera,
        mut projection,
        pixelate,
        mut render_texture,
        ortho_fixed_height,
        mut render_resolution,
        mut units_per_pixel,
    ) in &mut cameras
    {
        let changed = window_changed || pixelate.is_changed() || projection.is_changed();
        if !changed {
            continue;
        }

        let mut upp = None;
        if let Some(ortho_fixed_height) = ortho_fixed_height {
            let upscaled_units_per_pixel = (ortho_fixed_height.height.abs() * ortho_fixed_height.scale.abs())
                / window_resolution.y.min(window_resolution.x) as f32;
            upp = match *pixelate {
                Pixelate::PixelsPerUnit(ppu) => Some(1.0 / ppu.max(1) as f32),
                Pixelate::Fixed(w, h) => {
                    let scale_factor = (window_resolution.x as f32 / w as f32)
                        .max(constants::MIN_SCALE_FACTOR)
                        .min(window_resolution.y as f32 / h as f32);
                    Some(upscaled_units_per_pixel * scale_factor)
                }
                Pixelate::ScaleFactor(scale) => Some(upscaled_units_per_pixel / scale.min(constants::MIN_SCALE_FACTOR)),
            };
        }

        if let Some(upp) = upp {
            *units_per_pixel = UnitsPerPixel::Value(upp);
        } else {
            *units_per_pixel = UnitsPerPixel::Unavailable;
        }

        let resolution = pixelate.render_resolution(window_resolution, ortho_fixed_height);
        let size = Extent3d { width: resolution.x, height: resolution.y, depth_or_array_layers: 1 };

        let render_texture_handle = if let Some(render_texture_handle) = render_texture.handle() {
            render_texture_handle
        } else {
            use bevy::render::{
                camera::RenderTarget,
                render_resource::{TextureDescriptor, TextureDimension, TextureFormat, TextureUsages},
            };
            let mut image = Image {
                texture_descriptor: TextureDescriptor {
                    label: None,
                    size,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                },
                sampler: ImageSampler::linear(),
                ..default()
            };

            image.resize(size);

            let image_handle = images.add(image);
            render_texture.set_handle(image_handle.clone());
            camera.target = RenderTarget::Image(image_handle.clone());
            **render_resolution = resolution;
            projection.update(size.width as f32, size.height as f32);
            image_handle
        };

        let image =
            images.get_mut(&render_texture_handle).expect("RenderTexture asset not found. This should never happen.");

        if image.texture_descriptor.size != size {
            image.resize(size);
            **render_resolution = resolution;
            projection.update(size.width as f32, size.height as f32);
        };
    }
}

pub(crate) fn sync_blitter_camera(
    mut commands: Commands,
    mut cameras: Query<
        (
            &Camera,
            &GlobalTransform,
            &mut RenderTexture,
            &RenderResolution,
            Option<&UnitsPerPixel>,
            Option<&SubPixelSmoothing>,
            Option<&SnapOffset>,
        ),
        (With<Pixelate>, Without<Blitter>),
    >,
    mut blit_cameras: Query<
        (Entity, &Blitter, &Camera, Option<&mut ScaleBias>, Option<&mut RenderTexture>),
        (Without<Pixelate>, With<Camera2d>),
    >,
    _images: ResMut<Assets<Image>>,
) {
    for (entity, blitter, camera, scale_bias, render_texture) in &mut blit_cameras {
        let Some(pixelate_camera) = **blitter else {
            continue;
        };

        let Ok((
            pixelate_camera_data,
            _global_transform,
            pixelate_render_texture,
            render_resolution,
            units_per_pixel,
            sub_pixel_smoothing,
            snap_offset,
        )) = cameras.get_mut(pixelate_camera)
        else {
            warn!("Blitter target camera not found.");
            continue;
        };

        #[cfg(debug_assertions)]
        if pixelate_camera_data.order >= camera.order {
            warn!(
                "Pixelate camera should have a lower order than the Blitter camera. {:?} >= {:?}",
                pixelate_camera_data.order, camera.order
            );
        }

        // extract render texture from pixelate camera
        if let Some(mut render_texture) = render_texture
            && render_texture.as_ref() != pixelate_render_texture.as_ref()
        {
            *render_texture = pixelate_render_texture.clone();
        } else {
            commands.entity(entity).insert(pixelate_render_texture.clone());
        }

        // extract scale bias from pixelate camera
        let bias = if let Some(sub_pixel_smoothing) = sub_pixel_smoothing
            && matches!(sub_pixel_smoothing, SubPixelSmoothing::On)
            && let Some(&snap_offset) = snap_offset
            && let Some(units_per_pixel) = units_per_pixel
            && let Some(units_per_pixel) = units_per_pixel.get_value()
        {
            let mut bias = snap_offset.xy() / units_per_pixel;
            // displacement in relation to render resolution.
            bias /= render_resolution.as_vec2();
            // gridSizeZ = gridSizeX / (Mathf.Sin(viewAngle * Mathf.Deg2Rad));
            // gridSizeY = gridSizeX / (Mathf.Cos(viewAngle * Mathf.Deg2Rad));
            // let (scale, rt, trans) = global_transform.to_scale_rotation_translation();
            // let (x, y, z) = rt.to_euler(EulerRot::XYZ);
            // bias *= x.sinZX();
            bias.y *= -1.0;
            bias
        } else {
            Vec2::ZERO
        };

        if let Some(mut scale_bias) = scale_bias {
            scale_bias.bias = bias;
        } else {
            commands.entity(entity).insert(ScaleBias::with_bias(bias));
        }
    }
}
