struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec3<f32>,
    sun: f32,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;
@group(0) @binding(1)
var images: texture_2d_array<f32>;
@group(0) @binding(2)
var image_sampler: sampler;

// Must match `IMAGE_BASE` in mesh.rs.
const IMAGE_BASE: u32 = 100u;
// Must match `MASK_FLAG` in mesh.rs: an image whose alpha cuts holes, the
// cutoff in bits 23-30 (255ths).
const MASK_FLAG: u32 = 0x80000000u;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) kind: u32,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) kind: u32,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.view_proj * vec4<f32>(v.position, 1.0);
    out.normal = v.normal;
    out.color = v.color;
    out.uv = v.uv;
    out.kind = v.kind;
    return out;
}

// --- Procedural patterns ------------------------------------------------------
// `uv` is measured in tiles (one plank, tile or brick course per unit).

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(123.34, 456.21));
    q = q + dot(q, q + 45.32);
    return fract(q.x * q.y);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, s.x), mix(c, d, s.x), s.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var q = p;
    for (var i = 0; i < 4; i = i + 1) {
        sum = sum + amp * noise(q);
        q = q * 2.03 + vec2<f32>(17.1, 9.2);
        amp = amp * 0.5;
    }
    return sum;
}

/// Screen-space size of one pixel in tile units, set at the top of the
/// fragment shader (derivatives are only valid in uniform control flow).
var<private> pixel: f32;

/// 1 on a joint line of half-width `w` at integer `x`, where `x` is `uv`
/// scaled by `k`; anti-aliased over one pixel.
fn joint_k(x: f32, w: f32, k: f32) -> f32 {
    let aa = max(pixel * k, 1e-4);
    let d = abs(fract(x + 0.5) - 0.5);
    return 1.0 - smoothstep(w - aa, w + aa, d);
}

fn joint(x: f32, w: f32) -> f32 {
    return joint_k(x, w, 1.0);
}

/// Running-bond joints: rows of height 1, each row shifted by `shift`.
fn running_bond(uv: vec2<f32>, shift: f32, wx: f32, wy: f32) -> vec3<f32> {
    let row = floor(uv.y);
    let x = uv.x + shift * (row % 2.0);
    let cell = vec2<f32>(floor(x), row);
    let lines = max(joint(uv.y, wy), joint(x, wx));
    return vec3<f32>(lines, hash21(cell), 0.0);
}

fn wood(uv: vec2<f32>) -> f32 {
    let row = floor(uv.y);
    let x = uv.x + hash21(vec2<f32>(row, 3.0)) * 5.0;
    let board = floor(x);
    let tone = 0.82 + 0.3 * hash21(vec2<f32>(board, row));
    let grain = fbm(vec2<f32>(x * 2.0, uv.y * 24.0));
    let rings = 0.5 + 0.5 * sin((uv.y * 18.0 + grain * 6.0) * 3.14159);
    let seams = max(joint(uv.y, 0.02), joint(x, 0.004));
    return tone * (0.8 + 0.12 * grain + 0.1 * rings) * (1.0 - 0.45 * seams);
}

fn parquet(uv: vec2<f32>) -> f32 {
    // Basket weave: 2×2 blocks per tile, alternating plank direction.
    let p = uv * 2.0;
    let cell = floor(p);
    let f = fract(p);
    let flip = (cell.x + cell.y) % 2.0;
    let along = mix(f.x, f.y, flip);
    let across = mix(f.y, f.x, flip) * 3.0;
    let plank = floor(across);
    let tone = 0.8 + 0.32 * hash21(cell * 7.0 + plank);
    let grain = noise(vec2<f32>(along * 20.0, across * 2.0 + cell.x * 3.1));
    let seams = max(max(joint_k(p.x, 0.012, 2.0), joint_k(p.y, 0.012, 2.0)), joint_k(across, 0.03, 6.0));
    return tone * (0.9 + 0.1 * grain) * (1.0 - 0.45 * seams);
}

fn tiles(uv: vec2<f32>) -> f32 {
    let cell = floor(uv);
    let tone = 0.95 + 0.08 * hash21(cell);
    let speck = 0.97 + 0.05 * noise(uv * 40.0);
    let grout = max(joint(uv.x, 0.006), joint(uv.y, 0.006));
    return mix(tone * speck, 0.72, grout);
}

fn subway(uv: vec2<f32>) -> f32 {
    let b = running_bond(uv, 0.5, 0.012, 0.025);
    return mix(0.97 + 0.04 * b.y, 0.75, b.x);
}

fn brick(uv: vec2<f32>) -> f32 {
    let b = running_bond(uv, 0.5, 0.02, 0.07);
    let tone = 0.75 + 0.4 * b.y;
    let rough = 0.88 + 0.16 * fbm(uv * vec2<f32>(12.0, 4.0));
    return mix(tone * rough, 1.25, b.x);
}

// Brick painted a colour of its own: nearly even bricks, shadowed joints
// (`patterns::PAINTED_BRICK`).
fn painted_brick(uv: vec2<f32>) -> f32 {
    let b = running_bond(uv, 0.5, 0.02, 0.07);
    let tone = 0.94 + 0.08 * b.y;
    let rough = 0.95 + 0.06 * fbm(uv * vec2<f32>(12.0, 4.0));
    return mix(tone * rough, 0.8, b.x);
}

fn stone(uv: vec2<f32>) -> f32 {
    // Voronoi cells: distance to nearest and second-nearest seed.
    let i = floor(uv * 3.0);
    let f = fract(uv * 3.0);
    var d1 = 8.0;
    var d2 = 8.0;
    var id = vec2<f32>(0.0);
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let o = vec2<f32>(f32(x), f32(y));
            let seed = o + vec2<f32>(hash21(i + o), hash21(i + o + 19.7)) * 0.8 + 0.1;
            let d = length(seed - f);
            if (d < d1) {
                d2 = d1;
                d1 = d;
                id = i + o;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    let edge = d2 - d1;
    let aa = max(pixel * 3.0, 1e-4);
    let gap = 1.0 - smoothstep(0.04 - aa, 0.04 + aa, edge);
    let tone = 0.75 + 0.4 * hash21(id) + 0.1 * fbm(uv * 10.0);
    return mix(tone, 0.45, gap);
}

fn concrete(uv: vec2<f32>) -> f32 {
    return 0.86 + 0.16 * fbm(uv * 3.0) + 0.06 * (noise(uv * 90.0) - 0.5);
}

fn marble(uv: vec2<f32>) -> f32 {
    let turbulence = fbm(uv * 2.5) * 5.0;
    let vein = abs(sin((uv.x * 1.3 + uv.y * 0.7) * 3.0 + turbulence));
    let tile = max(joint(uv.x, 0.002), joint(uv.y, 0.002));
    return (0.8 + 0.22 * pow(vein, 0.35)) * (1.0 - 0.25 * tile);
}

fn carpet(uv: vec2<f32>) -> f32 {
    return 0.88 + 0.14 * noise(uv * 160.0) + 0.06 * fbm(uv * 6.0);
}

fn grass(uv: vec2<f32>) -> f32 {
    return 0.8 + 0.25 * fbm(uv * 4.0) + 0.08 * (noise(uv * 60.0) - 0.5);
}

fn water(uv: vec2<f32>) -> f32 {
    let warp = fbm(uv * 3.0) * 4.0;
    let ripple = abs(sin(uv.x * 9.0 + warp) * cos(uv.y * 7.0 - warp));
    return 0.88 + 0.2 * pow(ripple, 3.0) + 0.05 * fbm(uv * 8.0);
}

fn deck(uv: vec2<f32>) -> f32 {
    let row = floor(uv.y);
    let x = uv.x + hash21(vec2<f32>(row, 5.0)) * 3.0;
    let tone = 0.8 + 0.3 * hash21(vec2<f32>(floor(x), row));
    let grain = fbm(vec2<f32>(x * 3.0, uv.y * 10.0));
    let gaps = max(joint(uv.y, 0.06), joint(x, 0.003));
    return tone * (0.85 + 0.15 * grain) * (1.0 - 0.7 * gaps);
}

fn pattern_shade(kind: u32, uv: vec2<f32>) -> f32 {
    switch kind {
        case 1u: { return wood(uv); }
        case 2u: { return parquet(uv); }
        case 3u: { return tiles(uv); }
        case 4u: { return subway(uv); }
        case 5u: { return brick(uv); }
        case 6u: { return stone(uv); }
        case 7u: { return concrete(uv); }
        case 8u: { return marble(uv); }
        case 9u: { return carpet(uv); }
        case 10u: { return grass(uv); }
        case 11u: { return water(uv); }
        case 12u: { return deck(uv); }
        case 13u: { return painted_brick(uv); }
        default: { return 1.0; }
    }
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Sample unconditionally: derivatives must be taken in uniform control flow.
    let d = fwidth(in.uv);
    pixel = max(d.x, d.y);
    let kind = in.kind & 0x007fffffu;
    let layer = select(0u, kind - IMAGE_BASE, kind >= IMAGE_BASE);
    let sampled = textureSample(images, image_sampler, vec2<f32>(in.uv.x, -in.uv.y), layer);
    let texel = sampled.rgb;
    if ((in.kind & MASK_FLAG) != 0u && sampled.a < f32((in.kind >> 23u) & 0xffu) / 255.0) {
        discard;
    }
    var albedo = in.color.rgb;
    if (kind >= IMAGE_BASE) {
        albedo = in.color.rgb * texel;
    } else if (kind > 0u) {
        albedo = in.color.rgb * pattern_shade(kind, in.uv);
    }
    let n = normalize(in.normal);
    let diffuse = max(dot(n, -u.light_dir), 0.0);
    // Soft indoor light: bright hemisphere ambient plus a gentle key light,
    // so interiors read like a lit room rather than a dark box.
    if (u.sun < 0.0) {
        let ambient = mix(0.86, 1.0, n.y * 0.5 + 0.5);
        return vec4<f32>(min(albedo * (ambient + diffuse * 0.22), vec3<f32>(1.0)), in.color.a);
    }
    // Live sun from the compass: sky ambient fading towards night plus a warm
    // directional key light.
    let sky = mix(0.28, 0.62, u.sun) * mix(0.8, 1.0, n.y * 0.5 + 0.5);
    let key = vec3<f32>(1.0, 0.95, 0.86) * diffuse * 0.62 * u.sun;
    return vec4<f32>(min(albedo * (vec3<f32>(sky) + key), vec3<f32>(1.0)), in.color.a);
}
