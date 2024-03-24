use bevy::{
    prelude::{FromWorld, Resource, World},
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutEntry, BindingType, BufferBindingType, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FilterMode, FragmentState, MultisampleState, PipelineCache, PrimitiveState,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType,
            TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
        },
        renderer::RenderDevice,
        texture::BevyDefault,
    },
};

use super::{camera::ScaleBias, SHADER_HANDLE};

#[derive(Resource)]
pub(super) struct PixelatePipeline {
    pub pipeline_id: CachedRenderPipelineId,
    pub sampler: Sampler,
    pub layout: BindGroupLayout,
}

impl FromWorld for PixelatePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "pixelate_texture_bind_group_layout",
            &[
                // low-res screen texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // linear (bilinear) sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // scale bias
                BindGroupLayoutEntry {
                    binding: 2,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(ScaleBias::min_size()),
                    },
                    visibility: ShaderStages::VERTEX,
                    count: None,
                },
            ],
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: None,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        let pipeline_id = world.resource_mut::<PipelineCache>().queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("pixelate_pipeline".into()),
            layout: vec![layout.clone()],
            vertex: VertexState {
                shader: SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: Vec::new(),
            },
            fragment: FragmentState {
                shader: SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }
            .into(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
        });

        Self { pipeline_id, layout, sampler }
    }
}
