//! Procedural surface patterns, matching the GPU shader so software renders
//! and the live 3D view look alike. `uv` is in tiles; `pixel` is the size of
//! one screen pixel in tile units, used for anti-aliasing joints.

#![allow(clippy::many_single_char_names)]

fn fract(v: f32) -> f32 {
    v - v.floor()
}

fn hash21(x: f32, y: f32) -> f32 {
    let (mut qx, mut qy) = (fract(x * 123.34), fract(y * 456.21));
    let d = qx * (qx + 45.32) + qy * (qy + 45.32);
    qx += d;
    qy += d;
    fract(qx * qy)
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn noise(x: f32, y: f32) -> f32 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (smooth(x - ix), smooth(y - iy));
    let a = hash21(ix, iy);
    let b = hash21(ix + 1.0, iy);
    let c = hash21(ix, iy + 1.0);
    let d = hash21(ix + 1.0, iy + 1.0);
    let top = a + (b - a) * fx;
    let bottom = c + (d - c) * fx;
    top + (bottom - top) * fy
}

fn fbm(x: f32, y: f32) -> f32 {
    let (mut sum, mut amp, mut qx, mut qy) = (0.0, 0.5, x, y);
    for _ in 0..4 {
        sum += amp * noise(qx, qy);
        qx = qx * 2.03 + 17.1;
        qy = qy * 2.03 + 9.2;
        amp *= 0.5;
    }
    sum
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    smooth(((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0))
}

/// 1 on a joint of half-width `w` at integer `x` (scaled by `k` from uv).
fn joint(x: f32, w: f32, k: f32, pixel: f32) -> f32 {
    let aa = (pixel * k).max(1e-4);
    let d = (fract(x + 0.5) - 0.5).abs();
    1.0 - smoothstep(w - aa, w + aa, d)
}

fn running_bond(u: f32, v: f32, shift: f32, wx: f32, wy: f32, pixel: f32) -> (f32, f32) {
    let row = v.floor();
    let x = u + shift * row.rem_euclid(2.0);
    let lines = joint(v, wy, 1.0, pixel).max(joint(x, wx, 1.0, pixel));
    (lines, hash21(x.floor(), row))
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Brick painted a colour of its own: the same bond with the bricks nearly
/// even and the joints a little shadowed. Plain brick's tones run from dark to
/// over-bright with light mortar, which on a white wall turn into grey
/// mottle; a pattern above the core's ones, chosen only by the renderer.
pub(crate) const PAINTED_BRICK: u32 = 13;

/// The renderer's kind for `pattern` drawn with or without a colour given.
pub(crate) fn kind_of(pattern: newera_core::Pattern, tinted: bool) -> u32 {
    if tinted && pattern == newera_core::Pattern::Brick {
        PAINTED_BRICK
    } else {
        pattern.index()
    }
}

/// Brightness multiplier of pattern `kind` (1-based, see
/// `newera_core::Pattern::index`) at `uv`.
pub(crate) fn shade(kind: u32, u: f32, v: f32, pixel: f32) -> f32 {
    match kind {
        1 => {
            // Wood planks.
            let row = v.floor();
            let x = u + hash21(row, 3.0) * 5.0;
            let tone = 0.82 + 0.3 * hash21(x.floor(), row);
            let grain = fbm(x * 2.0, v * 24.0);
            let rings = 0.5 + 0.5 * ((v * 18.0 + grain * 6.0) * std::f32::consts::PI).sin();
            let seams = joint(v, 0.02, 1.0, pixel).max(joint(x, 0.004, 1.0, pixel));
            tone * (0.8 + 0.12 * grain + 0.1 * rings) * (1.0 - 0.45 * seams)
        }
        2 => {
            // Basket-weave parquet.
            let (px, py) = (u * 2.0, v * 2.0);
            let (cx, cy) = (px.floor(), py.floor());
            let (fx, fy) = (px - cx, py - cy);
            let flip = (cx + cy).rem_euclid(2.0);
            let along = mix(fx, fy, flip);
            let across = mix(fy, fx, flip) * 3.0;
            let tone = 0.8 + 0.32 * hash21(cx * 7.0 + across.floor(), cy * 7.0 + across.floor());
            let grain = noise(along * 20.0, across * 2.0 + cx * 3.1);
            let seams = joint(px, 0.012, 2.0, pixel)
                .max(joint(py, 0.012, 2.0, pixel))
                .max(joint(across, 0.03, 6.0, pixel));
            tone * (0.9 + 0.1 * grain) * (1.0 - 0.45 * seams)
        }
        3 => {
            // Tiles.
            let tone = 0.95 + 0.08 * hash21(u.floor(), v.floor());
            let speck = 0.97 + 0.05 * noise(u * 40.0, v * 40.0);
            let grout = joint(u, 0.006, 1.0, pixel).max(joint(v, 0.006, 1.0, pixel));
            mix(tone * speck, 0.72, grout)
        }
        4 => {
            let (lines, h) = running_bond(u, v, 0.5, 0.012, 0.025, pixel);
            mix(0.97 + 0.04 * h, 0.75, lines)
        }
        5 => {
            let (lines, h) = running_bond(u, v, 0.5, 0.02, 0.07, pixel);
            let tone = 0.75 + 0.4 * h;
            let rough = 0.88 + 0.16 * fbm(u * 12.0, v * 4.0);
            mix(tone * rough, 1.25, lines)
        }
        6 => stone(u, v, pixel),
        7 => 0.86 + 0.16 * fbm(u * 3.0, v * 3.0) + 0.06 * (noise(u * 90.0, v * 90.0) - 0.5),
        8 => {
            let turbulence = fbm(u * 2.5, v * 2.5) * 5.0;
            let vein = ((u * 1.3 + v * 0.7) * 3.0 + turbulence).sin().abs();
            let tile = joint(u, 0.002, 1.0, pixel).max(joint(v, 0.002, 1.0, pixel));
            (0.8 + 0.22 * vein.powf(0.35)) * (1.0 - 0.25 * tile)
        }
        9 => 0.88 + 0.14 * noise(u * 160.0, v * 160.0) + 0.06 * fbm(u * 6.0, v * 6.0),
        10 => 0.8 + 0.25 * fbm(u * 4.0, v * 4.0) + 0.08 * (noise(u * 60.0, v * 60.0) - 0.5),
        11 => {
            // Water: soft caustic ripples.
            let warp = fbm(u * 3.0, v * 3.0) * 4.0;
            let ripple = ((u * 9.0 + warp).sin() * (v * 7.0 - warp).cos()).abs();
            0.88 + 0.2 * ripple.powf(3.0) + 0.05 * fbm(u * 8.0, v * 8.0)
        }
        12 => {
            // Deck boards with open joints.
            let row = v.floor();
            let x = u + hash21(row, 5.0) * 3.0;
            let tone = 0.8 + 0.3 * hash21(x.floor(), row);
            let grain = fbm(x * 3.0, v * 10.0);
            let gaps = joint(v, 0.06, 1.0, pixel).max(joint(x, 0.003, 1.0, pixel));
            tone * (0.85 + 0.15 * grain) * (1.0 - 0.7 * gaps)
        }
        PAINTED_BRICK => {
            let (lines, h) = running_bond(u, v, 0.5, 0.02, 0.07, pixel);
            let tone = 0.94 + 0.08 * h;
            let rough = 0.95 + 0.06 * fbm(u * 12.0, v * 4.0);
            mix(tone * rough, 0.8, lines)
        }
        _ => 1.0,
    }
}

fn stone(u: f32, v: f32, pixel: f32) -> f32 {
    let (sx, sy) = (u * 3.0, v * 3.0);
    let (ix, iy) = (sx.floor(), sy.floor());
    let (fx, fy) = (sx - ix, sy - iy);
    let (mut d1, mut d2, mut id) = (8.0_f32, 8.0_f32, (0.0, 0.0));
    for y in -1..=1 {
        for x in -1..=1 {
            #[allow(clippy::cast_precision_loss)]
            let (ox, oy) = (x as f32, y as f32);
            let seed_x = ox + hash21(ix + ox, iy + oy) * 0.8 + 0.1;
            let seed_y = oy + hash21(ix + ox + 19.7, iy + oy + 19.7) * 0.8 + 0.1;
            let d = (seed_x - fx).hypot(seed_y - fy);
            if d < d1 {
                d2 = d1;
                d1 = d;
                id = (ix + ox, iy + oy);
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    let edge = d2 - d1;
    let aa = (pixel * 3.0).max(1e-4);
    let gap = 1.0 - smoothstep(0.04 - aa, 0.04 + aa, edge);
    let tone = 0.75 + 0.4 * hash21(id.0, id.1) + 0.1 * fbm(u * 10.0, v * 10.0);
    mix(tone, 0.45, gap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_stay_in_a_sane_range() {
        for kind in 1..=10 {
            for i in 0..200 {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f32 * 0.137;
                let s = shade(kind, t, t * 0.7, 0.01);
                assert!((0.0..=1.6).contains(&s), "kind {kind}: {s}");
            }
        }
        assert!((shade(0, 1.0, 2.0, 0.1) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn brick_given_a_colour_stays_close_to_that_colour() {
        use newera_core::Pattern;
        let range = |kind: u32| {
            (0..400).fold((f32::MAX, f32::MIN), |(lo, hi), i| {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f32 * 0.173;
                // Away from the joints, on even and on shifted rows alike.
                let (u, v) = (t.floor() + 0.3 + 0.15 * t.fract(), t * 0.61 + 0.4);
                let s = shade(kind, u, v.floor() + 0.5, 0.001);
                (lo.min(s), hi.max(s))
            })
        };
        // Plain brick keeps its mottle; a white one stays white.
        let (lo, hi) = range(kind_of(Pattern::Brick, false));
        assert!(hi - lo > 0.3, "{lo}..{hi}");
        let (lo, hi) = range(kind_of(Pattern::Brick, true));
        assert!(lo > 0.85 && hi < 1.06, "{lo}..{hi}");
        assert_eq!(kind_of(Pattern::Tiles, true), Pattern::Tiles.index());
    }
}
