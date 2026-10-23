//! Terrain levels (`data/levels/*.lev`, `data/challenges/*.lev`).
//!
//! Layout (little endian):
//! - `"ski\0"`, u32 version (1 or 2), u32
//! - version 2 only: 32 bytes of terrain-generator settings
//! - u32 cells x, u32 cells y (recomputed from the height grid on load)
//! - 8 bytes, 8 bytes (map window), u32, 16 bytes (camera angles, unused),
//!   u32 detail level
//! - f32 grid spacing, f32, f32
//! - u32 rows, u32 columns, then `rows` runs of `columns` f64 heights,
//!   indexed `[x][y]`
//! - one record per cell (`(rows-1) * (columns-1)`): u32 present flag and,
//!   when it is 1, 33 bytes of surface data (see [`Cell`]).

use anyhow::{Context, Result, bail};

use super::texture::Rgba;

pub const MAGIC: &[u8; 4] = b"ski\0";

/// The terrain-generator settings the shipped levels carry, for new maps.
const GENERATOR_DEFAULTS: [u8; 32] = {
    let words: [u32; 8] = [0, 0x3f80_0000, 0x3f80_0000, 45, 0x3f80_0000, 0x3f80_0000, 4, 0];
    let mut b = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        b[i] = words[i / 4].to_le_bytes()[i % 4];
        i += 1;
    }
    b
};

/// Cell flag bits observed in the shipped levels and the original code.
pub mod cell_flags {
    /// Forest floor; draws the `TR_snow` tile set.
    pub const FOREST: u32 = 0x2;
    /// Moguls.
    pub const MOGULS: u32 = 0x4;
    /// A footpath; the cell's object id is its type (0x40bce0).
    pub const PATH: u32 = 0x1000;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Cell {
    pub owner: u32,
    pub object_id: i32,
    /// Accumulated skier traffic; 20..100 selects one of the worn overlays.
    pub wear: f32,
    /// Base snow tile variant (0..=2).
    pub variant: u32,
    pub center: [f32; 3],
    /// Index into the built-in tree/plant table (0 = none).
    pub foliage: u8,
    pub flags: u32,
}

impl Cell {
    /// The footpath here, by type (0 rock, 1 rock 2, 2 brick, 3 brick 2).
    pub fn path(&self) -> Option<u8> {
        (self.flags & cell_flags::PATH != 0).then(|| self.object_id.clamp(0, 3) as u8)
    }

    /// Lays a footpath, or with `None` takes it up.
    pub fn set_path(&mut self, path: Option<u8>) {
        match path {
            Some(t) => {
                self.flags |= cell_flags::PATH;
                self.object_id = t as i32;
            }
            None => {
                self.flags &= !cell_flags::PATH;
                self.object_id = -1;
            }
        }
    }

    /// Plants (nonzero) or clears trees; the forest flag goes with them, as
    /// in the original (0x40d7f0).
    pub fn set_foliage(&mut self, foliage: u8) {
        self.foliage = foliage;
        if foliage == 0 {
            self.flags &= !cell_flags::FOREST;
        } else {
            self.flags |= cell_flags::FOREST;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Level {
    #[allow(dead_code)]
    pub version: u32,
    pub generator: [u8; 32],
    pub detail: u32,
    pub spacing: f32,
    /// Number of height samples along x and y.
    pub size: usize,
    heights: Vec<f32>,
    pub cells: Vec<Cell>,
}

struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Rd<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.p..self.p + n).context("level truncated")?;
        self.p += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
}

impl Level {
    pub fn parse(b: &[u8]) -> Result<Level> {
        let mut r = Rd { b, p: 0 };
        if r.bytes(4)? != MAGIC {
            bail!("not a level file");
        }
        let version = r.u32()?;
        r.u32()?;
        let mut generator = [0u8; 32];
        match version {
            1 => {}
            2 => generator.copy_from_slice(r.bytes(32)?),
            v => bail!("unsupported level version {v}"),
        }
        r.u32()?;
        r.u32()?;
        r.bytes(8)?;
        r.bytes(8)?;
        r.u32()?;
        r.bytes(16)?;
        let detail = r.u32()?;
        let spacing = r.f32()?;
        r.f32()?;
        r.f32()?;
        let rows = r.u32()? as usize;
        let cols = r.u32()? as usize;
        if rows != cols || rows < 2 || rows > 2048 {
            bail!("unsupported height grid {rows}x{cols}");
        }
        let size = rows;
        let mut heights = vec![0f32; size * size];
        for x in 0..size {
            let run = r.bytes(size * 8)?;
            for (y, c) in run.chunks_exact(8).enumerate() {
                heights[y * size + x] = f64::from_le_bytes(c.try_into().unwrap()) as f32;
            }
        }
        let n = size - 1;
        let mut cells = Vec::with_capacity(n * n);
        for _ in 0..n * n {
            if r.u32()? != 1 {
                cells.push(Cell::default());
                continue;
            }
            let owner = r.u32()?;
            let object_id = r.u32()? as i32;
            let wear = r.f32()?;
            let variant = r.u32()?;
            let center = [r.f32()?, r.f32()?, r.f32()?];
            let foliage = r.bytes(1)?[0];
            let flags = r.u32()?;
            cells.push(Cell {
                owner,
                object_id,
                wear,
                variant,
                center,
                foliage,
                flags,
            });
        }
        Ok(Level {
            version,
            generator,
            detail,
            spacing: if spacing > 0.0 { spacing } else { 10.0 },
            size,
            heights,
            cells,
        })
    }

    pub fn load(path: &std::path::Path) -> Result<Level> {
        let b = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Level::parse(&b).with_context(|| format!("parsing {}", path.display()))
    }

    /// A map with `size` height samples a side, as the original's editor
    /// starts one (0x4182f0, 0x4215b0): 10 units apart, every cell bare
    /// snow of a random variant. `rand` stands for the C library's rand().
    fn blank(size: usize, heights: Vec<f32>, rand: &mut dyn FnMut() -> u32) -> Level {
        let n = size - 1;
        let spacing = 10.0;
        let mut level = Level {
            version: 2,
            generator: GENERATOR_DEFAULTS,
            detail: 2,
            spacing,
            size,
            heights,
            cells: Vec::with_capacity(n * n),
        };
        for cy in 0..n {
            for cx in 0..n {
                let z = (0..4).map(|i| level.height(cx + i % 2, cy + i / 2)).sum::<f32>() * 0.25;
                level.cells.push(Cell {
                    owner: 0,
                    object_id: -1,
                    wear: 0.0,
                    variant: rand() % 3,
                    center: [1000.0 + (cx as f32 + 0.5) * spacing, 1000.0 + (cy as f32 + 0.5) * spacing, 1000.0 + z],
                    foliage: 0,
                    flags: 0x600,
                });
            }
        }
        level
    }

    /// The editor's Flat map: `size` samples a side (32 to 256), all at
    /// height 1 (0x4182f0).
    pub fn flat(size: usize, rand: &mut dyn FnMut() -> u32) -> Level {
        let size = size.clamp(32, 256);
        Level::blank(size, vec![1.0; size * size], rand)
    }

    /// The editor's Height map (0x421b80): the elevation picture's green
    /// is half the height, row x and column y giving sample (x, y); its
    /// `_foliage` picture plants a tree in each cell whose pixel (column x,
    /// row y) has any green, its kind by how many of the 5x5 around it do
    /// too (0x408f10).
    pub fn from_pictures(elevation: &Rgba, foliage: Option<&Rgba>, rand: &mut dyn FnMut() -> u32) -> Level {
        let green = |img: &Rgba, col: usize, row: usize| -> u8 {
            if col >= img.width as usize || row >= img.height as usize {
                return 0;
            }
            img.pixels[(row * img.width as usize + col) * 4 + 1]
        };
        let size = elevation.width.min(elevation.height) as usize;
        let mut heights = vec![0.0; size * size];
        for x in 0..size {
            for y in 0..size {
                heights[y * size + x] = green(elevation, y, x) as f32 * 2.0;
            }
        }
        let mut level = Level::blank(size, heights, rand);
        let Some(foliage) = foliage else { return level };
        let tree = |cx: i32, cy: i32| cx > 0 && cy > 0 && green(foliage, cx as usize, cy as usize) != 0;
        let n = level.cells_per_side();
        for cy in 0..n {
            for cx in 0..n {
                if green(foliage, cx, cy) == 0 {
                    continue;
                }
                let near = (-2..=2)
                    .flat_map(|dx| (-2..=2).map(move |dy| (dx, dy)))
                    .filter(|&(dx, dy)| tree(cx as i32 + dx, cy as i32 + dy))
                    .count();
                level.cell_mut(cx, cy).set_foliage(tree_kind(near, rand));
            }
        }
        level
    }

    /// Replaces the heights (`size` a side, sample (x, y) at y * size + x)
    /// and starts every cell afresh, as the editor's generator rebuilds
    /// the map (0x4194d0).
    pub fn regrade(&mut self, heights: Vec<f32>, rand: &mut dyn FnMut() -> u32) {
        let generator = self.generator;
        *self = Level::blank(self.size, heights, rand);
        self.generator = generator;
    }

    /// Height sample at grid vertex (x, y).
    pub fn height(&self, x: usize, y: usize) -> f32 {
        self.heights[y.min(self.size - 1) * self.size + x.min(self.size - 1)]
    }

    pub fn set_height(&mut self, x: usize, y: usize, h: f32) {
        if x < self.size && y < self.size {
            self.heights[y * self.size + x] = h;
        }
    }

    /// The lowest and highest ground on the map.
    pub fn height_range(&self) -> (f32, f32) {
        let mut r = (f32::MAX, f32::MIN);
        for y in 0..=self.cells_per_side() {
            for x in 0..=self.cells_per_side() {
                let h = self.height(x, y);
                r = (r.0.min(h), r.1.max(h));
            }
        }
        r
    }

    pub fn cells_per_side(&self) -> usize {
        self.size - 1
    }

    pub fn cell(&self, cx: usize, cy: usize) -> &Cell {
        &self.cells[cy * self.cells_per_side() + cx]
    }

    pub fn cell_mut(&mut self, cx: usize, cy: usize) -> &mut Cell {
        let n = self.cells_per_side();
        &mut self.cells[cy * n + cx]
    }

    /// World-space extent along x and y.
    pub fn extent(&self) -> f32 {
        self.cells_per_side() as f32 * self.spacing
    }

    /// Bilinear terrain height at world position (x, y).
    pub fn height_at(&self, x: f32, y: f32) -> f32 {
        let gx = (x / self.spacing).clamp(0.0, (self.size - 1) as f32);
        let gy = (y / self.spacing).clamp(0.0, (self.size - 1) as f32);
        let (x0, y0) = (gx.floor() as usize, gy.floor() as usize);
        let (fx, fy) = (gx - x0 as f32, gy - y0 as f32);
        let h00 = self.height(x0, y0);
        let h10 = self.height(x0 + 1, y0);
        let h01 = self.height(x0, y0 + 1);
        let h11 = self.height(x0 + 1, y0 + 1);
        // Matches the triangle split used by the terrain mesh.
        if fx + fy <= 1.0 {
            h00 + (h10 - h00) * fx + (h01 - h00) * fy
        } else {
            h11 + (h01 - h11) * (1.0 - fx) + (h10 - h11) * (1.0 - fy)
        }
    }

    /// Surface normal (z up) from central differences.
    pub fn normal_at(&self, x: f32, y: f32) -> glam::Vec3 {
        let e = self.spacing * 0.5;
        let dx = self.height_at(x + e, y) - self.height_at(x - e, y);
        let dy = self.height_at(x, y + e) - self.height_at(x, y - e);
        glam::Vec3::new(-dx, -dy, 2.0 * e).normalize()
    }

    /// Steepness in degrees at world (x, y).
    pub fn slope_deg_at(&self, x: f32, y: f32) -> f32 {
        self.normal_at(x, y).z.clamp(-1.0, 1.0).acos().to_degrees()
    }

    pub fn write(&self) -> Vec<u8> {
        let mut o = Vec::new();
        o.extend_from_slice(MAGIC);
        o.extend_from_slice(&2u32.to_le_bytes());
        o.extend_from_slice(&2u32.to_le_bytes());
        o.extend_from_slice(&self.generator);
        let n = self.cells_per_side() as u32;
        o.extend_from_slice(&n.to_le_bytes());
        o.extend_from_slice(&n.to_le_bytes());
        o.extend_from_slice(&[0u8; 8]);
        o.extend_from_slice(&[0u8; 8]);
        o.extend_from_slice(&0u32.to_le_bytes());
        o.extend_from_slice(&[0u8; 16]);
        o.extend_from_slice(&self.detail.to_le_bytes());
        o.extend_from_slice(&self.spacing.to_le_bytes());
        o.extend_from_slice(&0.1f32.to_le_bytes());
        o.extend_from_slice(&1.0f32.to_le_bytes());
        o.extend_from_slice(&(self.size as u32).to_le_bytes());
        o.extend_from_slice(&(self.size as u32).to_le_bytes());
        for x in 0..self.size {
            for y in 0..self.size {
                o.extend_from_slice(&(self.height(x, y) as f64).to_le_bytes());
            }
        }
        for c in &self.cells {
            o.extend_from_slice(&1u32.to_le_bytes());
            o.extend_from_slice(&c.owner.to_le_bytes());
            o.extend_from_slice(&c.object_id.to_le_bytes());
            o.extend_from_slice(&c.wear.to_le_bytes());
            o.extend_from_slice(&c.variant.to_le_bytes());
            for v in c.center {
                o.extend_from_slice(&v.to_le_bytes());
            }
            o.push(c.foliage);
            o.extend_from_slice(&c.flags.to_le_bytes());
        }
        o
    }
}

/// The kind of tree for a forest cell with `near` forest cells in the 5x5
/// around it, itself included (0x408f10, 0x40d4a0): denser forest grows
/// the taller pines.
pub fn tree_kind(near: usize, rand: &mut dyn FnMut() -> u32) -> u8 {
    let mut kind = match near {
        0..8 => rand() % 3,
        8..12 => rand() % 4 + 2,
        12..18 => rand() % 5 + 4,
        _ => rand() % 6 + 8,
    };
    if kind > 9 {
        kind = 9 - rand() % 3;
    }
    kind as u8 + 9
}

/// Built-in foliage table: the byte stored in each cell indexes this list.
pub const FOLIAGE_MESHES: &[&str] = &[
    "",
    "data/scenery/trees/planters/aspen.mesh",
    "data/scenery/trees/planters/alpine_fir.mesh",
    "data/scenery/trees/planters/blue_spruce.mesh",
    "data/scenery/trees/planters/cypress.mesh",
    "data/scenery/trees/planters/grand_fir.mesh",
    "data/scenery/trees/planters/lodgepole.mesh",
    "data/scenery/trees/planters/noble.mesh",
    "data/scenery/trees/planters/spruce.mesh",
    "data/scenery/trees/shrubA.mesh",
    "data/scenery/trees/pineA.mesh",
    "data/scenery/trees/pineB.mesh",
    "data/scenery/trees/pineC.mesh",
    "data/scenery/trees/pineD.mesh",
    "data/scenery/trees/pineE.mesh",
    "data/scenery/trees/pineF.mesh",
    "data/scenery/trees/pineG.mesh",
    "data/scenery/trees/pineH.mesh",
    "data/scenery/trees/pineI.mesh",
    "data/scenery/trees/planters/hedge_A.mesh",
    "data/scenery/trees/planters/hedge_B.mesh",
    "data/scenery/trees/planters/hedge_C.mesh",
    "data/scenery/trees/planters/hedge_E.mesh",
    "data/scenery/trees/planters/pottedA.mesh",
    "data/scenery/trees/planters/pottedB.mesh",
    "data/scenery/trees/planters/pottedC.mesh",
    "data/scenery/trees/planters/pottedD.mesh",
    "data/scenery/trees/planters/shrub_a.mesh",
    "data/scenery/trees/planters/shrub_b.mesh",
    "data/scenery/trees/planters/shrub_c.mesh",
    "data/scenery/trees/planters/sm_treea.mesh",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let size = 4;
        let mut b = Vec::new();
        b.extend_from_slice(MAGIC);
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&[0u8; 32]);
        b.extend_from_slice(&[0u8; 8 + 8 + 8 + 4 + 16 + 4]);
        b.extend_from_slice(&10f32.to_le_bytes());
        b.extend_from_slice(&[0u8; 8]);
        b.extend_from_slice(&(size as u32).to_le_bytes());
        b.extend_from_slice(&(size as u32).to_le_bytes());
        for x in 0..size {
            for y in 0..size {
                b.extend_from_slice(&((x * 10 + y) as f64).to_le_bytes());
            }
        }
        for _ in 0..9 {
            b.extend_from_slice(&0u32.to_le_bytes());
        }
        let lvl = Level::parse(&b).unwrap();
        assert_eq!(lvl.height(2, 1), 21.0);
        let again = Level::parse(&lvl.write()).unwrap();
        assert_eq!(again.height(3, 2), 32.0);
        assert!((lvl.height_at(15.0, 0.0) - 15.0).abs() < 1e-4);
    }
}
