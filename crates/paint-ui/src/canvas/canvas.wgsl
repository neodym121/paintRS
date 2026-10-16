// PaintRS canvas: premultiplied image over a screen-space checkerboard, optional pixel grid.
// The callback viewport covers only the visible part of the canvas, so texture coordinates
// are derived from the fragment position (framebuffer pixels) and the full canvas rectangle.

struct Uniforms {
    checker_a: vec4<f32>,
    checker_b: vec4<f32>,
    grid_color: vec4<f32>,
    canvas_origin: vec2<f32>, // top-left of the whole canvas, framebuffer pixels
    canvas_size: vec2<f32>,   // size of the whole canvas, framebuffer pixels
    tex_size: vec2<f32>,      // image size in image pixels
    checker_px: f32,          // checker square, framebuffer pixels
    grid_alpha: f32,          // 0 = no pixel grid
    srgb_target: f32,         // 1 = framebuffer converts linear -> sRGB
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    // Full-viewport triangle.
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn linear_from_gamma(c: vec3<f32>) -> vec3<f32> {
    let cutoff = c < vec3<f32>(0.04045);
    let lower = c / vec3<f32>(12.92);
    let higher = pow((c + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    return select(higher, lower, cutoff);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = (pos.xy - u.canvas_origin) / u.canvas_size;
    let img = textureSample(tex, samp, uv);

    let cell = floor(pos.xy / u.checker_px);
    let odd = (i32(cell.x) + i32(cell.y)) & 1;
    let bg = select(u.checker_a.rgb, u.checker_b.rgb, odd == 1);
    var col = img.rgb + bg * (1.0 - img.a);

    if (u.grid_alpha > 0.0) {
        // One device pixel at the left/top edge of every image pixel.
        let px_per_texel = u.canvas_size / u.tex_size;
        let f = fract(uv * u.tex_size) * px_per_texel;
        if (f.x < 1.0 || f.y < 1.0) {
            col = mix(col, u.grid_color.rgb, u.grid_alpha);
        }
    }

    if (u.srgb_target > 0.5) {
        col = linear_from_gamma(col);
    }
    return vec4<f32>(col, 1.0);
}
