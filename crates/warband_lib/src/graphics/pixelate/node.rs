use bevy::{
    ecs::query::QueryItem,
    prelude::{Image, World},
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex},
        render_asset::RenderAssets,
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::{
            BindGroupEntries, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        },
        renderer::RenderContext,
        view::ViewTarget,
    },
};

use super::{
    camera::{Blitter, RenderTexture, ScaleBias},
    pipeline::PixelatePipeline,
};

#[derive(Default)]
pub(super) struct PixelateNode {}

impl PixelateNode {
    pub const NAME: &'static str = "pixelate_node";
}

impl ViewNode for PixelateNode {
    type ViewQuery =
        (&'static ViewTarget, &'static RenderTexture, &'static DynamicUniformIndex<ScaleBias>, &'static Blitter);

    fn run(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, render_texture, scale_bias_index, _): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pixelate_pipeline = world.resource::<PixelatePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let scale_bias_uniforms = world.resource::<ComponentUniforms<ScaleBias>>();

        let (Some(scale_bias_uniforms), Some(pipeline), Some(image_handle)) = (
            scale_bias_uniforms.binding(),
            pipeline_cache.get_render_pipeline(pixelate_pipeline.pipeline_id),
            render_texture.handle(),
        ) else {
            return Ok(());
        };

        let gpu_images = world.resource::<RenderAssets<Image>>();
        let gpu_render_image = &gpu_images.get(image_handle).expect("Image not loaded");

        // perf: cache this
        let render_image_texture = &gpu_render_image.texture_view;

        let bind_group = render_context.render_device().create_bind_group(
            None,
            &pixelate_pipeline.layout,
            &BindGroupEntries::sequential((render_image_texture, &pixelate_pipeline.sampler, scale_bias_uniforms)),
        );

        let post_process_write = target.post_process_write();
        let pass_descriptor = RenderPassDescriptor {
            label: Some("pixelate_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process_write.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
        };

        let mut render_pass = render_context.command_encoder().begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[scale_bias_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
