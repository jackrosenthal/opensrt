//! The map editor's starting maps (the original's CLevelEdit screen,
//! 0x456d30 and 0x4572a0, and its CSkiEditLayer world, 0x4182f0), and
//! where edited maps are saved.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::assets::Assets;
use crate::formats::level::Level;
use crate::formats::texture;

/// Where the editor's map comes from: the screen's three choices.
#[derive(Debug, Clone)]
pub enum MapSource {
    /// Flat map of this many height samples a side.
    Flat(usize),
    /// A picture under `data/height_maps`, by relative path.
    HeightMap(String),
    /// An existing level, by relative path.
    Level(String),
}

/// The Size box: its number between 32 and 256, as the original reads it
/// (0x45781d); anything else, or nothing, gives the nearer limit.
pub fn flat_size(text: &str) -> usize {
    let n: i64 = text.trim().parse().unwrap_or(0);
    n.clamp(32, 256) as usize
}

/// The C library's rand(), seeded from the clock as the original seeds it.
pub fn crand() -> impl FnMut() -> u32 {
    let mut state = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(1);
    move || {
        state = state.wrapping_mul(214_013).wrapping_add(2_531_011);
        (state >> 16) & 0x7fff
    }
}

/// Builds the map and names it: a level keeps its name, the others start
/// unnamed.
pub fn make_map(assets: &Assets, source: &MapSource) -> Result<(Level, String)> {
    let mut rand = crand();
    Ok(match source {
        MapSource::Flat(size) => (Level::flat(*size, &mut rand), String::new()),
        MapSource::HeightMap(rel) => {
            let path = assets.resolve(rel).with_context(|| format!("height map {rel} not found"))?;
            (from_pictures(assets, path, rel, &mut rand)?, String::new())
        }
        MapSource::Level(rel) => {
            let path = assets.resolve_level(rel).with_context(|| format!("level {rel} not found"))?;
            let name = Path::new(rel)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            (Level::load(&path)?, name)
        }
    })
}

/// The elevation picture and, beside it, its `_foliage` one if there is
/// one (0x421b80).
fn from_pictures(assets: &Assets, path: &Path, rel: &str, rand: &mut dyn FnMut() -> u32) -> Result<Level> {
    let elevation = texture::load_any(path)?;
    let stem = rel.rsplit_once('.').map_or(rel, |(s, _)| s);
    let foliage = assets
        .resolve(&format!("{stem}_foliage.bmp"))
        .and_then(|p| texture::load_any(p).ok());
    Ok(Level::from_pictures(&elevation, foliage.as_ref(), rand))
}

/// Edited maps are saved here as `<name>.lev`, and listed with the
/// shipped levels (the original writes them to `data\levels`, 0x417f40).
pub fn user_levels_dir() -> PathBuf {
    crate::assets::data_home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("srt/levels")
}

/// Saves the map as a level named `name`.
pub fn save_level(level: &Level, name: &str) -> Result<PathBuf> {
    let dir = user_levels_dir();
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(format!("{name}.lev"));
    std::fs::write(&path, level.write()).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// The Generate Random Map window's numbers, kept with the editor's map
/// (0x4183a2 sets them, 0x44c5f0 keeps them when the window shuts).
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct GenSettings {
    pub seed: i32,
    /// Percent.
    pub rockiness: i32,
    /// Height above which no trees grow.
    pub treeline: i32,
    /// Percent.
    pub tree_density: i32,
}

impl Default for GenSettings {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_millis() as i32 + (d.as_secs() % 100_000) as i32 * 1000)
            .unwrap_or(0);
        GenSettings {
            seed,
            rockiness: 50,
            treeline: 200,
            tree_density: 60,
        }
    }
}

/// The C library's rand() from a given seed (srand).
pub fn crand_seeded(seed: u32) -> impl FnMut() -> u32 {
    let mut state = seed;
    move || {
        state = state.wrapping_mul(214_013).wrapping_add(2_531_011);
        (state >> 16) & 0x7fff
    }
}

/// A random amount up to `x` either way (0x42d6e0).
fn wobble(x: f32, rand: &mut dyn FnMut() -> u32) -> f32 {
    let up = rand() & 1 == 1;
    let v = rand() as f32 / 32767.0 * x;
    if up { v } else { -v }
}

/// The window's nine anchor points, as height samples: the map's corners,
/// the middles of its edges and its middle, in the window's order (top,
/// top-middle-left, left, top-middle-right, right, middle,
/// bottom-middle-left, bottom, bottom-middle-right). `top`, `left`,
/// `right` and `bottom` are the corners on screen (0x4194d0 places them by
/// the view's quarter turn).
pub fn anchor_samples(n: usize, top: (usize, usize), left: (usize, usize), right: (usize, usize), bottom: (usize, usize)) -> [(usize, usize); 9] {
    let mid = |a: (usize, usize), b: (usize, usize)| ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
    [
        top,
        mid(top, left),
        left,
        mid(top, right),
        right,
        (n / 2, n / 2),
        mid(left, bottom),
        bottom,
        mid(right, bottom),
    ]
}

/// Generate Terrain (0x4194d0, 0x41a1a0): the anchors' heights at their
/// samples, then the two triangles that make up the map split again and
/// again, each new middle point raised or lowered at random by up to 0.4
/// of its edge's length times the rockiness, until the edges are under
/// half a cell. With `flatten`, the entrance's corner is levelled with
/// the corner sample first.
pub fn generate_terrain(level: &mut Level, anchors: &[((usize, usize), f32); 9], rockiness: i32, flatten: bool, rand: &mut dyn FnMut() -> u32) {
    let size = level.size;
    let n = size - 1;
    let mut h = vec![-1.0f32; size * size];
    let at = |x: usize, y: usize| y * size + x;
    for &((x, y), z) in anchors {
        h[at(x.min(n), y.min(n))] = z;
    }
    if flatten {
        let q = size / 4;
        for i in 0..q {
            for j in 0..q - i {
                h[at(i, j)] = h[at(0, 0)];
            }
        }
    }
    let far = (10 * (n + 1)) as f32;
    let corner = |h: &[f32], x: usize, y: usize| h[at(x, y)];
    let rock = rockiness as f32 * 0.01;
    let p00 = glam::Vec3::new(-1.0, -1.0, corner(&h, 0, 0));
    let p0n = glam::Vec3::new(-1.0, far, corner(&h, 0, n));
    let pn0 = glam::Vec3::new(far, -1.0, corner(&h, n, 0));
    let pnn = glam::Vec3::new(far, far, corner(&h, n, n));
    let mut split = Splitter { h: &mut h, size, rock, rand };
    split.split(p00, p0n, pn0);
    split.split(pnn, p0n, pn0);
    let heights = h.into_iter().map(|z| z.max(0.0)).collect();
    level.regrade(heights, rand);
}

struct Splitter<'a> {
    h: &'a mut [f32],
    size: usize,
    rock: f32,
    rand: &'a mut dyn FnMut() -> u32,
}

impl Splitter<'_> {
    /// The sample a point falls on: its coordinates over the cell size,
    /// cut toward zero and kept on the map.
    fn sample(&self, p: glam::Vec3) -> usize {
        let n = self.size as i32 - 1;
        let i = |c: f32| ((c as i32) / 10).min(n).max(0) as usize;
        i(p.y) * self.size + i(p.x)
    }

    /// An edge's middle, moved at random; a sample already set keeps its
    /// height, and a new one is never below zero.
    fn middle(&mut self, a: glam::Vec3, b: glam::Vec3) -> Option<glam::Vec3> {
        if a.truncate().distance(b.truncate()) < 5.0 {
            return None;
        }
        let mut m = (a + b) * 0.5;
        m.z += wobble(a.distance(b) * 0.4 * self.rock, self.rand);
        let i = self.sample(m);
        if self.h[i] >= 0.0 {
            m.z = self.h[i];
        } else {
            self.h[i] = m.z.max(0.0);
        }
        Some(m)
    }

    fn split(&mut self, mut a: glam::Vec3, b: glam::Vec3, mut c: glam::Vec3) {
        loop {
            let Some(ab) = self.middle(a, b) else { return };
            let Some(bc) = self.middle(b, c) else { return };
            let Some(ac) = self.middle(a, c) else { return };
            self.split(a, ab, ac);
            self.split(ac, bc, c);
            self.split(ac, ab, bc);
            a = ab;
            c = bc;
        }
    }
}

/// Generate Trees (0x419250): each cell may grow a tree, the more likely
/// the flatter it is and the further below the treeline, scaled by the
/// density; with `flatten`, none in the entrance's corner. Then each
/// forest cell's kind by the forest around it (0x40d4a0).
pub fn generate_trees(level: &mut Level, density: i32, treeline: i32, flatten: bool, rand: &mut dyn FnMut() -> u32) {
    let n = level.cells_per_side();
    let line = treeline.max(1) as f32;
    let density = density as f32 * 0.01;
    for cy in 0..n {
        for cx in 0..n {
            level.cell_mut(cx, cy).set_foliage(0);
            let below = ((line - level.height(cx, cy)) / line).max(0.0);
            let (x, y) = ((cx as f32 + 0.5) * level.spacing, (cy as f32 + 0.5) * level.spacing);
            let flat = level.normal_at(x, y).z;
            let roll = rand() as f32 / 32767.0;
            if roll < flat * below * density && !(flatten && cx <= n / 4 && cy <= n / 4) {
                level.cell_mut(cx, cy).set_foliage(1);
            }
        }
    }
    let forest: Vec<bool> = level.cells.iter().map(|c| c.foliage != 0).collect();
    for cy in 0..n {
        for cx in 0..n {
            if !forest[cy * n + cx] {
                continue;
            }
            let near = (-2..=2i32)
                .flat_map(|dy| (-2..=2i32).map(move |dx| (dx, dy)))
                .filter(|&(dx, dy)| {
                    let (x, y) = (cx as i32 + dx, cy as i32 + dy);
                    x >= 0 && y >= 0 && (x as usize) < n && (y as usize) < n && forest[y as usize * n + x as usize]
                })
                .count();
            let kind = crate::formats::level::tree_kind(near, rand);
            level.cell_mut(cx, cy).set_foliage(kind);
        }
    }
}
