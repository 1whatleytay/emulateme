struct VertexInput {
    @location(0) position: vec2<f32>
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = vec4(in.position, 0.0, 1.0);

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(1.0, 0.0, 0.5, 1.0);
}
