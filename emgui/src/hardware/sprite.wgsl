struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) sprite: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) sprite_number: u32,
    @location(2) sprite_mask: u32,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let sprite_y = in.sprite[0];
    let sprite_x = in.sprite[3];

    let sprite_mask = in.sprite[2];

    let offset = vec2(f32(sprite_x) / 256.0 * 2.0 - 1.0, -(f32(sprite_y) / 240.0 * 2.0 - 1.0));

    let behind_background = (sprite_mask & u32(0x20)) != u32(0);

    var depth: f32;

    if (behind_background) {
        depth = 0.75; // further back???
    } else {
        depth = 0.25;
    }

    out.position = vec4(in.position + offset, depth, 1.0);
    out.tex_coord = in.tex_coord;
    out.sprite_number = in.sprite[1];
    out.sprite_mask = in.sprite[2];

    return out;
}

struct PaletteMemory {
    values: array<vec4<u32>, 4> // 4 is first, next 4 is next, etc.
}

@group(0) @binding(0)
var patterns: texture_2d<u32>;

@group(0) @binding(1)
var palette_colors: texture_1d<f32>;

@group(0) @binding(2)
var<uniform> palette: PaletteMemory;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let sprite = in.sprite_number;

    let flip_x = (in.sprite_mask & u32(0x40)) != u32(0);
    let flip_y = (in.sprite_mask & u32(0x80)) != u32(0);

    var x = u32(in.tex_coord.x * 8.0);
    var y = u32(in.tex_coord.y * 8.0);

    if (flip_x) {
        x = u32(7) - x;
    }

    if (flip_y) {
        y = u32(7) - x;
    }

    let palette_index = in.sprite_mask & u32(3);

    let coord = vec2(sprite * u32(8) + x, y);

    let value = textureLoad(patterns, coord, 0).r;
    if (value == u32(0)) {
        discard;
    }

    let index = palette.values[palette_index][value];

    let palette_color = textureLoad(palette_colors, index, 0);

    return palette_color;
}
