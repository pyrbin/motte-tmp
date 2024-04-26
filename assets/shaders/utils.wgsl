#define_import_path motte::utils

fn clamp01(value: f32) -> f32 {
    return clamp(value, 0.0, 1.0);
}

fn random(st: vec2<f32>) -> f32 {
    return fract(sin(dot(st.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453123);
}

fn is_nan(val: f32) -> bool {
    return !(val < 0.0 || 0.0 < val || val == 0.0);
}

/// returns the (0.0, 0.0) .. (1.0, 1.0) position within the viewport for the current render target
/// [0 .. render target viewport size] eg. [(0.0, 0.0) .. (1280.0, 720.0)] to [(0.0, 0.0) .. (1.0, 1.0)]
fn coords_to_uv(coords: vec2<i32>, size: vec2<i32>) -> vec2<f32> {
    return (vec2<f32>(coords) + 0.5) / vec2<f32>(size);
}
