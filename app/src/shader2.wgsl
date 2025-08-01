struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) frag_uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    output.clip_pos = vec4<f32>(
        (input.position * 2.0) - 1.0,
        0.0,
        1.0
    );
    output.frag_uv = input.position;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.5, 0.5, 0.5, 0.25);
}
