#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{
        alpha_discard,
        PbrLightingOutput,
        apply_pbr_lighting,
        main_pass_post_lighting_processing
    }
}
#import motte::colors;
#import motte::utils::{clamp01};

struct CelMaterial {
    lit: f32,
    shadow: f32,
    cut_off: f32,
}

@group(2) @binding(100)
var<uniform> material: CelMaterial;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // convert into oklch
    let oklch = colors::srgb2oklch(pbr_input.material.base_color.rgb);

    // calculate bevy pbr lighting
    var out: FragmentOutput;
    let pbr_output_color = apply_pbr_lighting(pbr_input);
    alpha_discard(pbr_input.material, pbr_output_color);

    // quantize luminance
    var luminance: f32 = luminance(pbr_output_color.rgb);
    let lit = material.lit;
    let shadow = material.shadow;
    let cut_off = clamp01(material.cut_off);
    luminance = clamp01(clamp(step(cut_off, luminance), shadow * oklch.x, lit * oklch.x));

    // convert back to srgb
    let out_rgb = colors::oklch2srgb(vec3<f32>(luminance, oklch.y, oklch.z));
    out.color = vec4(out_rgb, pbr_output_color.a);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    
    return out;
}

fn luminance(rgb: vec3<f32>) -> f32 {
    let oklch = colors::srgb2oklch(rgb);
    return oklch.x;
}

// fn apply_outlines(rgb: vec3<f32>, frag_coord: vec2<f32>) -> vec3<f32> {
//     let edges = motte::edges::detect_edges(frag_coord);
//     return vec3(
//         mix(mix(rgb, vec3(1.0), clamp01(edges.normal) * .2), vec3(0.0), clamp01(edges.depth) * .6),
//     );
// }
