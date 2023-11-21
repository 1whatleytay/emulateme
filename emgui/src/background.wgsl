struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) screen_coord: vec2<f32>
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) screen_coord: vec2<f32>
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = vec4(in.position, 0.0, 1.0);
    out.screen_coord = in.screen_coord;

    return out;
}

struct PaletteMemory {
    values: array<vec4<u32>, 4> // 4 is first, next 4 is next, etc.
}

@group(0) @binding(0)
var patterns: texture_2d<u32>;

@group(0) @binding(1)
var name_table_0: texture_2d<u32>;

@group(0) @binding(2)
var name_table_1: texture_2d<u32>;

@group(0) @binding(3)
var palette_colors: texture_1d<f32>;

@group(0) @binding(4)
var<uniform> palette: PaletteMemory;

const PATTERN_WIDTH: u32 = u32(256 * 8);
const PATTERN_HEIGHT: u32 = u32(8);

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var x = u32(in.screen_coord.x * 256.0);
    var y = u32(in.screen_coord.y * 240.0);

    var tile_x = x / u32(8);
    var tile_y = y / u32(8);

    var subtile_x = x % u32(8);
    var subtile_y = y % u32(8);

    var sprite = textureLoad(name_table_0, vec2(tile_x, tile_y), 0).r;

    var coord = vec2(sprite * u32(8) + subtile_x, subtile_y);

    var value = textureLoad(patterns, coord, 0).r;
    if (value == u32(0)) {
        discard;
    }

    var attribute_x = x / 4;
    var attribute_y = y / 3;

    var attribute_address = 0x3C0 + attribute_x + attribute_y * 8;
//    var attribute_byte = textureLoad(name_table_0, attribute_address);

    var index = palette.values[0][value];

    var palette_color = textureLoad(palette_colors, index, 0);

    return palette_color;
}
