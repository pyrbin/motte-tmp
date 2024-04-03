#import bevy_pbr::{
  forward_io::{VertexOutput, FragmentOutput},
  pbr_fragment::pbr_input_from_standard_material,
  pbr_functions::{
    alpha_discard,
    PbrLightingOutput,
    generate_pbr_lighting,
    apply_pbr_lighting,
    main_pass_post_lighting_processing
  },
  view_transformations,
  mesh_view_bindings::view,
}
#import bevy_core_pipeline::tonemapping::screen_space_dither;
#import motte::colors;

struct CelMaterial {
    luminance_bands: f32,
    luminance_power: f32,
    dither_factor: f32,
}

@group(2) @binding(100)
var<uniform> cel_material: CelMaterial;

// https://stackoverflow.com/questions/596216/formula-to-determine-perceived-brightness-of-rgb-color
fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3(0.2126, 0.7152, 0.0722));
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // calculate vanilla pbr lighting
    var out: FragmentOutput;
    let pbr_output_color = apply_pbr_lighting(pbr_input);
    alpha_discard(pbr_input.material, pbr_output_color);

    // convert into oklch
    let oklch = colors::srgb2oklch(pbr_output_color.rgb);
    var luminance: f32 = oklch.x;
    var chroma: f32 = oklch.y;
    var hue: f32 = oklch.z;

    // apply world space dither to luminance
    let dither_factor = cel_material.dither_factor;
    let dither = length(screen_space_dither(in.position.xy)) * dither_factor;
    luminance += dither;

    // quantize luminance
    let bands = cel_material.luminance_bands;
    let power = cel_material.luminance_power;
    luminance = pow(floor(pow(luminance, 1.0 / power) * bands) / bands, power);

    // convert back to srgb
    let out_rgb = colors::oklch2srgb(vec3<f32>(luminance, chroma, hue));
    out.color = vec4(out_rgb, pbr_output_color.a);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    return out;
}