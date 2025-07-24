struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) instance_pos: vec2<f32>,
    @location(2) instance_size: vec2<f32>,
    @location(3) instance_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    let tmp = (input.position * input.instance_size + input.instance_pos) * 2.0;

    output.clip_pos = vec4<f32>(
        tmp.x - 1.0,
        1.0 - tmp.y,
        0.0,
        1.0
    );
    output.color = input.instance_color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
