// display-wgpu: cell-based terminal renderer shader
//
// Each ScreenCell is rendered as an instanced quad.
// Background fills the cell; foreground glyph is sampled from a texture atlas.

struct Uniforms {
    screen_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct InstanceInput {
    @location(0) cell_pos: vec2<f32>,
    @location(1) fg_color: u32,
    @location(2) bg_color: u32,
    @location(3) char_code: u32,
    @location(4) flags: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fg: vec4<f32>,
    @location(2) bg: vec4<f32>,
    @location(3) char_idx: u32,
    @location(4) flags: u32,
};

fn decode_color(c: u32) -> vec4<f32> {
    let r = f32((c >> 24u) & 0xFFu) / 255.0;
    let g = f32((c >> 16u) & 0xFFu) / 255.0;
    let b = f32((c >> 8u) & 0xFFu) / 255.0;
    let a = f32(c & 0xFFu) / 255.0;
    // Approximate sRGB -> linear
    return vec4<f32>(pow(r, 2.2), pow(g, 2.2), pow(b, 2.2), a);
}

@vertex
fn vs_main(inst: InstanceInput, @builtin(vertex_index) vid: u32) -> VertexOutput {
    // Expand quad from instance: 4 vertices per cell
    let corners = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );
    let uvs = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
    );

    let corner = corners[vid];
    let cell_px = inst.cell_pos + corner;

    // Convert pixel coords to NDC (top-left origin, Y+ down)
    let ndc_x = (cell_px.x / u.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (cell_px.y / u.screen_size.y) * 2.0;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = uvs[vid];
    out.fg = decode_color(inst.fg_color);
    out.bg = decode_color(inst.bg_color);
    out.char_idx = inst.char_code;
    out.flags = inst.flags;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Atlas layout: 16x16 grid of 16x16 pixel glyphs in a 256x256 texture
    var uv = in.uv;

    // Italic: skew UV horizontally
    if (in.flags & 4u) != 0u {
        uv.x = uv.x + (0.5 - uv.y) * 0.3;
        // Clamp to valid range
        uv.x = clamp(uv.x, 0.0, 1.0);
    }

    let col = in.char_idx % 16u;
    let row = in.char_idx / 16u;
    let atlas_uv = (vec2<f32>(f32(col), f32(row)) + uv) / 16.0;

    var glyph_alpha = textureSample(atlas, atlas_sampler, atlas_uv).r;

    // Bold: increase effective coverage by boosting alpha
    if (in.flags & 2u) != 0u {
        glyph_alpha = min(glyph_alpha * 1.5, 1.0);
    }

    var fg = in.fg;
    var bg = in.bg;

    // Reverse video: swap fg/bg
    if (in.flags & 1u) != 0u {
        let tmp = fg;
        fg = bg;
        bg = tmp;
    }

    // Dim: reduce fg brightness to 50%
    if (in.flags & 32u) != 0u {
        fg = vec4<f32>(fg.rgb * 0.5, fg.a);
    }

    // Blend: glyph pixels get fg color, empty pixels get bg color
    var color = mix(bg, fg, glyph_alpha);

    // Underline: draw a line at the bottom 15% of the cell
    if (in.flags & 8u) != 0u {
        if in.uv.y > 0.85 {
            color = vec4<f32>(fg.rgb, 1.0);
        }
    }

    // Strikethrough: draw a line at the middle 10% of the cell
    if (in.flags & 16u) != 0u {
        if in.uv.y > 0.45 && in.uv.y < 0.55 {
            color = vec4<f32>(fg.rgb, 1.0);
        }
    }

    return vec4<f32>(color.rgb, 1.0);
}
