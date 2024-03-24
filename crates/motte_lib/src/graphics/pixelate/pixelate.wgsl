#import bevy_core_pipeline::fullscreen_vertex_shader::{FullscreenVertexOutput, fullscreen_vertex_shader};

@group(0) @binding(0)
var screen_texture: texture_2d<f32>;
@group(0) @binding(1)
var linear_sampler: sampler;
@group(0) @binding(2)
var<uniform> scale_bias: ScaleBias;

struct ScaleBias {
    scale: vec2<f32>,
    bias: vec2<f32>,
}

/// Applies scale and bias to uv coordinates, used in the vertex shader.
fn apply_scale_bias(xy: vec2<f32>, scale_bias: ScaleBias) -> vec2<f32> {
    return scale_bias.bias + xy * scale_bias.scale;
}

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    var output = fullscreen_vertex_shader(vertex_index);
    output.uv = apply_scale_bias(output.uv, scale_bias);
    return output;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let resolution = vec2<f32>(textureDimensions(screen_texture));
    let texel_size = 1.0 / resolution;
    // smooth pixel upscaling, see: https://www.youtube.com/watch?v=d6tp43wZqps
    // box filter size in texel units
    let box_size = clamp(fwidth(in.uv) * resolution, vec2<f32>(1e-5), vec2<f32>(1.0));
    // scale uv by texture size to get texel coordinates
    let tx = in.uv * resolution - 0.5 * box_size;
    // compute offset for pixel-sized box filter
    let tx_offset = smoothstep(vec2<f32>(1.0) - box_size, vec2<f32>(1.0), fract(tx));
    // compute bilinear sample uv coordinates
    let uv = (floor(tx) + vec2<f32>(0.5) + tx_offset) * texel_size;

    return textureSampleGrad(screen_texture, linear_sampler, uv, dpdx(in.uv), dpdy(in.uv));
}