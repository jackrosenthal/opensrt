//! Pre-rendered building sprites (`generatedsprites/*.spr`), which the
//! original draws its buildings from (loader 0x438830, blitter 0x439310).
//!
//! Layout (little endian):
//! - u32 version (0x43), u32 palette count, u32 offset of the frame table
//! - the palettes: 256 RGB555 colors each, brightest first (one per light
//!   level)
//! - at the table: u32 ×3, u32 frame count, u32, then one u32 offset per
//!   frame, relative to the table
//! - a frame: i16 origin x, i16 origin y, i16 rows, i16 width, one i32 row
//!   offset per row (relative to the frame), then per row a list of runs
//!   ending in 0: a byte `n` < 0x80 skips `n` transparent pixels; a byte
//!   with 0x80 set, then two bytes, is followed by `n & 0x1f` palette
//!   indices.

use anyhow::{Context, Result, bail};

use super::texture::Rgba;

pub struct Sprite {
    palettes: Vec<[u16; 256]>,
    frames: Vec<usize>,
    data: Vec<u8>,
}

fn u32_at(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(p..p + 4).context("sprite truncated")?.try_into()?))
}

fn i16_at(b: &[u8], p: usize) -> Result<i16> {
    Ok(i16::from_le_bytes(b.get(p..p + 2).context("sprite truncated")?.try_into()?))
}

impl Sprite {
    pub fn parse(data: Vec<u8>) -> Result<Sprite> {
        let npal = u32_at(&data, 4)? as usize;
        let table = u32_at(&data, 8)? as usize;
        if npal == 0 || npal > 64 || 12 + npal * 512 > table {
            bail!("not a sprite file");
        }
        let palettes = (0..npal)
            .map(|k| {
                let mut p = [0u16; 256];
                for (i, c) in p.iter_mut().enumerate() {
                    let o = 12 + (k * 256 + i) * 2;
                    *c = u16::from_le_bytes([data[o], data[o + 1]]);
                }
                p
            })
            .collect();
        let count = u32_at(&data, table + 12)? as usize;
        if count > 4096 {
            bail!("implausible frame count {count}");
        }
        let frames = (0..count)
            .map(|i| u32_at(&data, table + 20 + i * 4).map(|o| table + o as usize))
            .collect::<Result<_>>()?;
        Ok(Sprite { palettes, frames, data })
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Decodes a frame with the brightest palette.
    pub fn frame(&self, i: usize) -> Result<Rgba> {
        let b = &self.data;
        let f = *self.frames.get(i).context("no such frame")?;
        let rows = i16_at(b, f + 4)?.max(0) as usize;
        let width = i16_at(b, f + 6)?.max(0) as usize;
        let pal = &self.palettes[0];
        let mut pixels = vec![0u8; width * rows * 4];
        for y in 0..rows {
            let mut p = f + i32::from_le_bytes(b.get(f + 8 + y * 4..f + 12 + y * 4).context("sprite truncated")?.try_into()?) as usize;
            let mut x = 0usize;
            loop {
                let v = *b.get(p).context("sprite truncated")?;
                if v == 0 {
                    break;
                }
                if v & 0x80 == 0 {
                    x += v as usize;
                    p += 1;
                    continue;
                }
                let n = (v & 0x1f) as usize;
                for k in 0..n {
                    let c = pal[*b.get(p + 3 + k).context("sprite truncated")? as usize];
                    if x + k < width {
                        let o = (y * width + x + k) * 4;
                        let ch = |s: u16| (((c >> s) & 31) as u32 * 255 / 31) as u8;
                        pixels[o..o + 4].copy_from_slice(&[ch(10), ch(5), ch(0), 255]);
                    }
                }
                p += n + 3;
                x += n;
            }
        }
        Ok(Rgba {
            width: width as u32,
            height: rows as u32,
            pixels,
            name: format!("sprite frame {i}"),
        })
    }
}
