#define_import_path motte::edges

#import bevy_pbr::prepass_utils::{prepass_depth, prepass_normal}
#import motte::utils::{clamp01};

struct Edges {
    normal: f32,
    depth: f32,
}

// https://godotshaders.com/shader/3d-pixel-art-outline-highlight-post-processing-shader/

fn detect_edges(frag_coord: vec2<f32>) -> Edges {
	var neg_depth_diff: f32 = 0.5;

    let depth: f32 = get_depth(frag_coord);
    var depth_diff: f32 = 0.0;    
    for (var i = 0u; i < NEIGHBOURS; i++) {
        let dd = get_depth(frag_coord + neighbours[i]);
        depth_diff += clamp01(depth - dd);
        neg_depth_diff += dd - depth;
    }

    neg_depth_diff = clamp01(neg_depth_diff);
    neg_depth_diff = clamp01(smoothstep(0.5, 0.5, neg_depth_diff) * 10.);
    depth_diff = smoothstep(.0002, .001, depth_diff);

    let normal: vec3<f32> = get_normal(frag_coord);
    var normal_diff: f32 = 0.0;
    for (var i = 0u; i < NEIGHBOURS; i++) { 
        let nd = get_normal(frag_coord + neighbours[i]);
        normal_diff += normal_diff(normal, nd, depth_diff);
    }

    normal_diff = smoothstep(.2, .8, normal_diff);
    normal_diff = clamp01(normal_diff - neg_depth_diff);

    var edges = Edges(normal_diff, depth_diff);
    return edges;
}

fn get_depth(frag_coord: vec2<f32>) -> f32 {
    let raw_depth = prepass_depth(vec4(frag_coord, 0.0, 0.0), 0u);
    return bevy_pbr::view_transformations::depth_ndc_to_view_z(raw_depth);
}

fn get_normal(frag_coord: vec2<f32>) -> vec3<f32> {
    return prepass_normal(vec4(frag_coord, 0.0, 0.0), 0u);
}

const NEIGHBOURS: u32 = 4u;
var<private> neighbours: array<vec2<f32>, NEIGHBOURS> = array<vec2<f32>, NEIGHBOURS>(
    // vec2<f32>(-1.0, 1.0),  // 0. top left
    vec2<f32>(0.0, 1.0),   // 1. top center
    // vec2<f32>(1.0, 1.0),   // 2. top right
    vec2<f32>(-1.0, 0.0),  // 3. center left
    //vec2<f32>(0.0, 0.0),   // 4. center center
    vec2<f32>(1.0, 0.0),   // 5. center right
    // vec2<f32>(-1.0, -1.0), // 6. bottom left
    vec2<f32>(0.0, -1.0),  // 7. bottom center
    // vec2<f32>(1.0, -1.0),  // 8. bottom right
);

const NORMAL_EDGE_BIAS: vec3<f32> = vec3<f32>(1.0);
fn normal_diff(normal: vec3<f32>, normal_sample: vec3<f32>, depth_diff: f32) -> f32 {
    var normal_diff = dot(normal - normal_sample, NORMAL_EDGE_BIAS);
    let normal_indicator = clamp01(smoothstep(-.01, .01, normal_diff));
    // Only the shallower pixel should detect the normal edge.
    let depth_indicator = clamp01(sign(depth_diff * .25 + .0025));
    return (1.0 - dot(normal, normal_sample)) * depth_indicator * normal_indicator;
}
