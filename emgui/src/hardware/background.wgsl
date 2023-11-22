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

    // Background is at 0.5
    out.position = vec4(in.position, 0.5, 1.0);
    out.screen_coord = in.screen_coord;

    return out;
}

struct PaletteMemory {
    values: array<vec4<u32>, 4> // 4 is first, next 4 is next, etc.
}

@group(0) @binding(0)
var patterns: texture_2d<u32>;

@group(0) @binding(1)
var name_table_0: texture_1d<u32>;

@group(0) @binding(2)
var name_table_1: texture_1d<u32>;

@group(0) @binding(3)
var palette_colors: texture_1d<f32>;

@group(0) @binding(4)
var<uniform> palette: PaletteMemory;

const PATTERN_WIDTH: u32 = u32(256 * 8);
const PATTERN_HEIGHT: u32 = u32(8);

const ATTRUBTE_OFFSET: u32 = u32(0x3C0);

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let x = u32(in.screen_coord.x * 256.0);
    let y = u32(in.screen_coord.y * 240.0);

    let tile_x = x / u32(8);
    let tile_y = y / u32(8);

    let subtile_x = x % u32(8);
    let subtile_y = y % u32(8);

    let sprite = textureLoad(name_table_0, tile_x + tile_y * u32(32), 0).r;

    let coord = vec2(sprite * u32(8) + subtile_x, subtile_y);

    let value = textureLoad(patterns, coord, 0).r;
    if (value == u32(0)) {
        discard;
    }

    let attribute_x = tile_x / u32(4);
    let attribute_y = tile_y / u32(4);

    let attribute_address = ATTRUBTE_OFFSET + attribute_x + attribute_y * u32(8);
    let attribute_byte = textureLoad(name_table_0, attribute_address, 0).r;

    let attribute_right = (tile_x / u32(2)) % u32(2);
    let attribute_bottom = (tile_y / u32(2)) % u32(2);

    let attribute_shift = attribute_right * u32(2) + attribute_bottom * u32(4);
    let palette_index = (attribute_byte >> attribute_shift) & u32(3);

    let index = palette.values[palette_index][value];

    let palette_color = textureLoad(palette_colors, index, 0);

    return palette_color;
}
