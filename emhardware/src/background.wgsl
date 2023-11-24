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

struct Offset {
    offset: vec2<u32>,
    base: vec2<u32>,
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

@group(0) @binding(5)
var<uniform> offset: Offset;

@group(0) @binding(6)
var offset_details: texture_1d<u32>;

const PATTERN_WIDTH: u32 = u32(256 * 8);
const PATTERN_HEIGHT: u32 = u32(8);

const ATTRUBTE_OFFSET: u32 = u32(0x3C0);

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let x_raw = u32(in.screen_coord.x * 256.0);
    let y_raw = u32(in.screen_coord.y * 240.0);

    let scan_x = textureLoad(offset_details, y_raw, 0);
    let offset_x = scan_x.r;
    let base_x = scan_x.g;

    var x = x_raw + offset_x;
    var y = y_raw + offset.offset.y;

    var use_table_1 = (base_x == u32(1)) != (offset.base.y == u32(1));

    if (x >= u32(256)) {
        x -= u32(256);

        use_table_1 = !use_table_1;
    }

    if (y >= u32(240)) {
        y -= u32(240);

        use_table_1 = !use_table_1;
    }

    let tile_x = x / u32(8);
    let tile_y = y / u32(8);

    let subtile_x = x % u32(8);
    let subtile_y = y % u32(8);

    let tile_address = tile_x + tile_y * u32(32);

    var sprite: u32;
    if (use_table_1) {
        sprite = textureLoad(name_table_1, tile_address, 0).r;
    } else {
        sprite = textureLoad(name_table_0, tile_address, 0).r;
    }

    let coord = vec2(sprite * u32(8) + subtile_x, subtile_y);

    let value = textureLoad(patterns, coord, 0).r;
    if (value == u32(0)) {
        discard;
    }

    let attribute_x = tile_x / u32(4);
    let attribute_y = tile_y / u32(4);

    let attribute_address = ATTRUBTE_OFFSET + attribute_x + attribute_y * u32(8);

    var attribute_byte: u32;

    if (use_table_1) {
        attribute_byte = textureLoad(name_table_1, attribute_address, 0).r;
    } else {
        attribute_byte = textureLoad(name_table_0, attribute_address, 0).r;
    }

    let attribute_right = (tile_x / u32(2)) % u32(2);
    let attribute_bottom = (tile_y / u32(2)) % u32(2);

    let attribute_shift = attribute_right * u32(2) + attribute_bottom * u32(4);
    let palette_index = (attribute_byte >> attribute_shift) & u32(3);

    let index = palette.values[palette_index][value];

    let palette_color = textureLoad(palette_colors, index, 0);

    return palette_color;
}
