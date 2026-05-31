struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
};

@group(0) @binding(0)
var live2d_texture: texture_2d<f32>;

@group(0) @binding(1)
var live2d_sampler: sampler;

@group(1) @binding(0)
var<uniform> live2d_transform: mat4x4<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = live2d_transform * vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.opacity = input.opacity;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(live2d_texture, live2d_sampler, input.uv);
    let alpha = sample.a * input.opacity;
    return vec4<f32>(sample.rgb * alpha, alpha);
}
