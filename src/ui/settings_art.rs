//! The settings screen without its Renderer and Resolution rows. Their
//! words, boxes, and arrows are painted into the original's background,
//! so the left panel is redrawn at load: a smooth frosted panel (the
//! see-through panel with its lettering filled in from around it, then
//! blurred well), with the lettering of Scenery Detail, Sound, Music, and
//! Scroll rate laid back on, spread evenly down it. Only the lettering
//! itself is carried over, so none of the scenery behind it comes along.

use crate::formats::texture::Rgba;

/// Where each remaining row's band (its top) goes: Scenery Detail, Sound,
/// Music, and Scroll rate, from 173, 251, 330, and 409.
pub const ROW_TOPS: [f32; 4] = [44.0, 156.0, 268.0, 380.0];

/// How far a row of the original moves up: index 0 Scenery Detail.
pub fn lift(row: usize) -> f32 {
    BANDS[row + 2].0 as f32 - ROW_TOPS[row]
}

/// The left panel's box, columns and rows (inclusive).
const X0: usize = 19;
const X1: usize = 306;
const Y0: usize = 20;
const Y1: usize = 462;
/// Each row's heading and controls, with their shadows.
const BANDS: [(usize, usize); 6] = [(20, 75), (94, 149), (173, 230), (251, 306), (330, 385), (409, 462)];
/// How soft the frosted panel is (Gaussian sigma, pixels).
const FROST: f32 = 45.0;

/// Redraws the settings background (640x480) as described above; any
/// other size is left alone.
pub fn lift_controls(img: &mut Rgba) {
    let (w, h) = (img.width as usize, img.height as usize);
    if (w, h) != (640, 480) || img.pixels.len() != w * h * 4 {
        return;
    }
    let started = std::time::Instant::now();
    let n = w * h;
    let orig: Vec<[f32; 3]> = (0..n)
        .map(|i| [img.pixels[i * 4] as f32, img.pixels[i * 4 + 1] as f32, img.pixels[i * 4 + 2] as f32])
        .collect();
    let lum_of = |p: [f32; 3]| 0.3 * p[0] + 0.59 * p[1] + 0.11 * p[2];
    let lum: Vec<f32> = orig.iter().map(|&p| lum_of(p)).collect();
    // Dark lettering and its shadow stand below a closing of the image;
    // the light rims of the boxes above an opening.
    let closed = filter(&filter(&lum, w, h, 15, f32::max), w, h, 15, f32::min);
    let opened = filter(&filter(&lum, w, h, 7, f32::min), w, h, 7, f32::max);
    // The panel's bright inside, short of its dark rounded border.
    let in_box = |i: usize| (X0..=X1).contains(&(i % w)) && (Y0..=Y1).contains(&(i / w));
    let bright: Vec<bool> = (0..n).map(|i| closed[i] > 140.0).collect();
    let inside: Vec<bool> = erode(&bright, w, h, 1).into_iter().enumerate().map(|(i, b)| b && in_box(i)).collect();
    let in_band = |i: usize| BANDS.iter().any(|&(a, b)| (a..=b).contains(&(i / w)));
    let band: Vec<bool> = (0..n).map(|i| inside[i] && in_band(i)).collect();
    let mut mask: Vec<bool> = (0..n).map(|i| band[i] && (closed[i] - lum[i] > 5.0 || lum[i] - opened[i] > 5.0)).collect();
    for _ in 0..2 {
        mask = dilate(&mask, w, h, &band);
    }
    // The panel without lettering, filled in from around it, and that
    // frosted.
    let keep: Vec<bool> = (0..n).map(|i| inside[i] && !mask[i]).collect();
    let fill = masked_blur(&orig, &keep, w, h, 6.0);
    let clean: Vec<[f32; 3]> = (0..n).map(|i| if mask[i] { fill[i] } else { orig[i] }).collect();
    let frost = masked_blur(&clean, &inside, w, h, FROST);
    // How much of each pixel is lettering: around its strong dark
    // strokes (shadows included), by how far it darkens the panel, kept
    // off the panel's edge.
    let diff: Vec<[f32; 3]> = (0..n).map(|i| [orig[i][0] - clean[i][0], orig[i][1] - clean[i][1], orig[i][2] - clean[i][2]]).collect();
    let dl: Vec<f32> = diff.iter().map(|&d| lum_of(d)).collect();
    let mut core: Vec<bool> = (0..n).map(|i| mask[i] && dl[i] < -40.0).collect();
    let all = vec![true; n];
    for _ in 0..4 {
        core = dilate(&core, w, h, &all);
    }
    for (c, m) in core.iter_mut().zip(&mask) {
        *c &= *m;
    }
    let weight: Vec<f32> = (0..n).map(|i| if core[i] { ((dl[i].abs() - 2.0) / 8.0).clamp(0.0, 1.0) } else { 0.0 }).collect();
    let weight = blur(&weight, w, h, 0.6);
    let away = erode(&inside, w, h, 4);
    let mut out: Vec<[f32; 3]> = (0..n).map(|i| if inside[i] { frost[i] } else { orig[i] }).collect();
    for (row, top) in ROW_TOPS.iter().enumerate() {
        let (a, b) = BANDS[row + 2];
        for y in a..=b {
            let t = *top as usize + (y - a);
            for x in X0..=X1 {
                let (i, j) = (y * w + x, t * w + x);
                let k = if away[i] { weight[i] } else { 0.0 };
                if k > 0.0 {
                    for c in 0..3 {
                        out[j][c] += diff[i][c] * k;
                    }
                }
            }
        }
    }
    for i in (0..n).filter(|&i| in_box(i)) {
        for c in 0..3 {
            img.pixels[i * 4 + c] = out[i][c].round().clamp(0.0, 255.0) as u8;
        }
    }
    log::debug!("settings background redrawn in {:?}", started.elapsed());
}

/// The mask grown by one pixel (a cross), only where `keep` allows.
fn dilate(m: &[bool], w: usize, h: usize, keep: &[bool]) -> Vec<bool> {
    let mut out = m.to_vec();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if out[i] || !keep[i] {
                continue;
            }
            out[i] = (x > 0 && m[i - 1]) || (x + 1 < w && m[i + 1]) || (y > 0 && m[i - w]) || (y + 1 < h && m[i + w]);
        }
    }
    out
}

/// The mask shrunk by `k` pixels (a cross each time); off the image
/// counts as outside.
fn erode(m: &[bool], w: usize, h: usize, k: usize) -> Vec<bool> {
    let mut cur = m.to_vec();
    for _ in 0..k {
        let prev = cur.clone();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                cur[i] = prev[i] && x > 0 && x + 1 < w && y > 0 && y + 1 < h && prev[i - 1] && prev[i + 1] && prev[i - w] && prev[i + w];
            }
        }
    }
    cur
}

/// A blur of the pixels where `m` is set, ignoring the rest (normalized
/// convolution).
fn masked_blur(src: &[[f32; 3]], m: &[bool], w: usize, h: usize, sigma: f32) -> Vec<[f32; 3]> {
    let weight: Vec<f32> = m.iter().map(|&b| if b { 1.0 } else { 0.0 }).collect();
    let den = blur(&weight, w, h, sigma);
    let ch: Vec<Vec<f32>> = (0..3)
        .map(|c| blur(&src.iter().zip(&weight).map(|(p, k)| p[c] * k).collect::<Vec<_>>(), w, h, sigma))
        .collect();
    (0..w * h)
        .map(|i| {
            let d = den[i].max(1e-4);
            [ch[0][i] / d, ch[1][i] / d, ch[2][i] / d]
        })
        .collect()
}

/// A Gaussian blur, edges held: by kernel out to four sigma when small
/// (the fill under the lettering needs that reach), else three box blurs
/// of about the same spread.
fn blur(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
    if sigma > 8.0 {
        let r = (((12.0 * sigma * sigma / 3.0 + 1.0).sqrt() - 1.0) * 0.5).round().max(1.0) as usize;
        let mut v = src.to_vec();
        for _ in 0..3 {
            v = box_pass(&v, w, h, r, true);
            v = box_pass(&v, w, h, r, false);
        }
        return v;
    }
    let r = (sigma * 4.0).ceil() as isize;
    let k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let sum: f32 = k.iter().sum();
    let pass = |src: &[f32], horizontal: bool| -> Vec<f32> {
        let mut out = vec![0.0; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut v = 0.0;
                for (n, kv) in k.iter().enumerate() {
                    let d = n as isize - r;
                    let (sx, sy) = if horizontal {
                        ((x as isize + d).clamp(0, w as isize - 1) as usize, y)
                    } else {
                        (x, (y as isize + d).clamp(0, h as isize - 1) as usize)
                    };
                    v += src[sy * w + sx] * kv;
                }
                out[y * w + x] = v / sum;
            }
        }
        out
    };
    pass(&pass(src, true), false)
}

/// One box blur of radius `r` along rows or columns, by a running sum.
fn box_pass(src: &[f32], w: usize, h: usize, r: usize, horizontal: bool) -> Vec<f32> {
    let (len, lines) = if horizontal { (w, h) } else { (h, w) };
    let at = |line: usize, k: usize| if horizontal { line * w + k } else { k * w + line };
    let mut out = vec![0.0; w * h];
    let span = (2 * r + 1) as f32;
    for line in 0..lines {
        let get = |k: isize| src[at(line, k.clamp(0, len as isize - 1) as usize)];
        let mut sum: f32 = (-(r as isize)..=r as isize).map(get).sum();
        for k in 0..len {
            out[at(line, k)] = sum / span;
            sum += get(k as isize + r as isize + 1) - get(k as isize - r as isize);
        }
    }
    out
}

/// A square `k` x `k` max or min filter, done a row and a column at a time.
fn filter(src: &[f32], w: usize, h: usize, k: usize, f: fn(f32, f32) -> f32) -> Vec<f32> {
    let r = k / 2;
    let mut tmp = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            tmp[y * w + x] = (x.saturating_sub(r)..=(x + r).min(w - 1)).map(|i| src[y * w + i]).reduce(f).unwrap_or(0.0);
        }
    }
    let mut out = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            out[y * w + x] = (y.saturating_sub(r)..=(y + r).min(h - 1)).map(|j| tmp[j * w + x]).reduce(f).unwrap_or(0.0);
        }
    }
    out
}
