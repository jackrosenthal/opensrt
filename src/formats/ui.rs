//! Interface screens (`data/ui/**/*.bin`).
//!
//! A screen file holds named sprites plus named groups (buttons, panels)
//! laid out on a 640×480 canvas:
//! - `0x00` u32 version (100), `0x0c` screen name (char[64])
//! - `0x50` u32 sprite count, `0x54` u32 group count
//! - `0x58` group offsets, then sprite offsets
//! - group: name[64], u32 element count, u32 offset of its elements
//! - element: hit rect x1,y1,x2,y2 (inclusive), f32, u32 layer count n,
//!   u32, then n u32 offsets of layers (so elements are 28 + 4n bytes)
//! - layer (28 bytes): source rect x1,y1,x2,y2 within the sprite,
//!   destination x,y and u32 sprite offset. The sprite's origin is drawn
//!   at the group position plus the destination.
//! - sprite: name[64] followed by an image. Images are "CD" run-length
//!   sprites: u32 size at +2, i16 width/height at +6/+8, i16 palette size
//!   at +0xb, 15-bit (x1r5g5b5) palette at +0x21, then one u32 row offset
//!   per row (relative to the image header) and the rows themselves.
//!   Row codes: `0x80|n` repeats the next palette index n times and
//!   `0xc0|n` copies n literal indices. Otherwise the byte is an opcode
//!   followed by a u16 count n: 1 skips n transparent pixels, 2 copies n
//!   raw 15-bit colors, 3 copies n palette indices, 4 copies n shade
//!   values and 5 repeats one shade value n times (shades darken what is
//!   underneath); 0 ends the row.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use super::texture::Rgba;

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.x2 - self.x1 + 1
    }
    pub fn height(&self) -> i32 {
        self.y2 - self.y1 + 1
    }
}

#[derive(Debug, Clone)]
pub struct Element {
    /// Clickable area, relative to the group position.
    pub rect: Rect,
    /// Sprite of the first layer (index into `Screen::sprites`).
    pub sprite: Option<usize>,
    pub layers: Vec<Layer>,
}

/// One sprite drawn as part of an element.
#[derive(Debug, Clone)]
pub struct Layer {
    /// Part of the sprite to draw.
    pub src: Rect,
    /// Where the sprite's origin goes, relative to the group position.
    pub dst: (i32, i32),
    pub sprite: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub name: String,
    pub image: Option<Rgba>,
}

#[derive(Debug, Clone)]
pub struct Screen {
    pub name: String,
    pub sprites: Vec<Sprite>,
    pub groups: Vec<Group>,
    by_name: HashMap<String, usize>,
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).context("ui file truncated")?.try_into()?))
}
fn i32_at(b: &[u8], o: usize) -> Result<i32> {
    Ok(u32_at(b, o)? as i32)
}
fn i16_at(b: &[u8], o: usize) -> Result<i16> {
    Ok(i16::from_le_bytes(b.get(o..o + 2).context("ui file truncated")?.try_into()?))
}
fn cstr(b: &[u8], o: usize, max: usize) -> String {
    let s = b.get(o..(o + max).min(b.len())).unwrap_or(&[]);
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    String::from_utf8_lossy(&s[..end]).into_owned()
}
fn rect_at(b: &[u8], o: usize) -> Result<Rect> {
    Ok(Rect {
        x1: i32_at(b, o)?,
        y1: i32_at(b, o + 4)?,
        x2: i32_at(b, o + 8)?,
        y2: i32_at(b, o + 12)?,
    })
}

pub fn x1r5g5b5(v: u16) -> [u8; 4] {
    let c = |s: u16| (((v >> s) & 31) as u32 * 255 / 31) as u8;
    [c(10), c(5), c(0), 255]
}

/// Opacity for a shade value (0 = clear, 31 = darkest).
fn shade(v: u8) -> u8 {
    ((v.min(31) as u32) * 200 / 31) as u8
}

/// Decodes a "CD" run-length sprite starting at `o`.
pub fn decode_cd(b: &[u8], o: usize) -> Result<Rgba> {
    if b.get(o..o + 2) != Some(b"CD") {
        bail!("not a CD sprite");
    }
    let w = i16_at(b, o + 6)?.max(0) as usize;
    let h = i16_at(b, o + 8)?.max(0) as usize;
    let n = i16_at(b, o + 0xb)?.max(0) as usize;
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        bail!("bad sprite size {w}x{h}");
    }
    let mut pal = Vec::with_capacity(n);
    for i in 0..n {
        let v = u16::from_le_bytes(b.get(o + 0x21 + i * 2..o + 0x23 + i * 2).context("palette")?.try_into()?);
        pal.push(x1r5g5b5(v));
    }
    let rows = o + 0x21 + n * 2;
    let mut px = vec![0u8; w * h * 4];
    // Byte 0x0a is 0 for uncompressed sprites: opaque 8-bit palette
    // indices, row by row, straight after the palette.
    if b.get(o + 10) == Some(&0) {
        let data = b.get(rows..rows + w * h).context("raw sprite data")?;
        for (i, &v) in data.iter().enumerate() {
            let c = pal.get(v as usize).copied().unwrap_or([255, 0, 255, 255]);
            px[i * 4..i * 4 + 4].copy_from_slice(&c);
        }
        return Ok(Rgba {
            width: w as u32,
            height: h as u32,
            pixels: px,
            name: String::new(),
        });
    }
    for y in 0..h {
        let mut p = o + u32_at(b, rows + y * 4)? as usize;
        let mut x = 0usize;
        while x < w {
            let c = *b.get(p).context("row data")?;
            p += 1;
            if c == 0 {
                break;
            }
            let col = |i: u8| pal.get(i as usize).copied().unwrap_or([255, 0, 255, 255]);
            let mut put = |x: usize, rgba: [u8; 4]| {
                if x < w {
                    px[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&rgba);
                }
            };
            if c & 0x80 != 0 {
                let n = (c & 0x3f) as usize;
                if c & 0x40 != 0 {
                    for _ in 0..n {
                        put(x, col(*b.get(p).context("row data")?));
                        p += 1;
                        x += 1;
                    }
                } else {
                    let v = *b.get(p).context("row data")?;
                    p += 1;
                    for _ in 0..n {
                        put(x, col(v));
                        x += 1;
                    }
                }
                continue;
            }
            let n = u16::from_le_bytes(b.get(p..p + 2).context("row data")?.try_into()?) as usize;
            p += 2;
            match c {
                1 => x += n,
                2 => {
                    // Anti-aliased edge pixels: a palette index in the low
                    // byte and coverage, out of 31, in the high one.
                    for _ in 0..n {
                        let (i, a) = (*b.get(p).context("row data")?, *b.get(p + 1).context("row data")?);
                        p += 2;
                        let [r, g, bl, _] = col(i);
                        put(x, [r, g, bl, ((a.min(31) as u32) * 255 / 31) as u8]);
                        x += 1;
                    }
                }
                3 => {
                    for _ in 0..n {
                        put(x, col(*b.get(p).context("row data")?));
                        p += 1;
                        x += 1;
                    }
                }
                4 => {
                    for _ in 0..n {
                        let v = *b.get(p).context("row data")?;
                        p += 1;
                        put(x, [0, 0, 0, shade(v)]);
                        x += 1;
                    }
                }
                5 => {
                    let v = *b.get(p).context("row data")?;
                    p += 1;
                    for _ in 0..n {
                        put(x, [0, 0, 0, shade(v)]);
                        x += 1;
                    }
                }
                _ => break,
            }
        }
    }
    Ok(Rgba {
        width: w as u32,
        height: h as u32,
        pixels: px,
        name: String::new(),
    })
}

impl Screen {
    pub fn parse(b: &[u8]) -> Result<Screen> {
        if u32_at(b, 0)? != 100 {
            bail!("unsupported ui file version");
        }
        let name = cstr(b, 0x0c, 64);
        let nsprites = u32_at(b, 0x50)? as usize;
        let ngroups = u32_at(b, 0x54)? as usize;
        if nsprites > 10_000 || ngroups > 10_000 {
            bail!("implausible ui counts");
        }
        let mut sprite_offsets = Vec::with_capacity(nsprites);
        let mut sprites = Vec::with_capacity(nsprites);
        for i in 0..nsprites {
            let o = u32_at(b, 0x58 + (ngroups + i) * 4)? as usize;
            sprite_offsets.push(o);
            let sname = cstr(b, o, 64);
            let image = decode_cd(b, o + 0x40).ok().map(|mut img| {
                img.name = sname.clone();
                img
            });
            sprites.push(Sprite { name: sname, image });
        }
        let mut groups = Vec::with_capacity(ngroups);
        for i in 0..ngroups {
            let o = u32_at(b, 0x58 + i * 4)? as usize;
            let gname = cstr(b, o, 64);
            let count = u32_at(b, o + 0x40)? as usize;
            let list = u32_at(b, o + 0x44)? as usize;
            let mut elements = Vec::with_capacity(count.min(256));
            let mut e = list;
            for _ in 0..count.min(256) {
                let rect = rect_at(b, e)?;
                let n = (u32_at(b, e + 0x14)? as usize).min(16);
                let mut layers = Vec::with_capacity(n);
                for k in 0..n {
                    let l = u32_at(b, e + 0x1c + k * 4)? as usize;
                    let so = u32_at(b, l + 0x18).unwrap_or(0) as usize;
                    layers.push(Layer {
                        src: rect_at(b, l)?,
                        dst: (i32_at(b, l + 0x10)?, i32_at(b, l + 0x14)?),
                        sprite: sprite_offsets.iter().position(|&x| x == so),
                    });
                }
                let sprite = layers.first().and_then(|l| l.sprite);
                elements.push(Element { rect, sprite, layers });
                e += 0x1c + n * 4;
            }
            groups.push(Group {
                name: gname,
                elements,
            });
        }
        let by_name = groups
            .iter()
            .enumerate()
            .map(|(i, g)| (g.name.to_ascii_lowercase(), i))
            .collect();
        Ok(Screen {
            name,
            sprites,
            groups,
            by_name,
        })
    }

    pub fn load(path: &std::path::Path) -> Result<Screen> {
        let b = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Screen::parse(&b).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn group(&self, name: &str) -> Option<&Group> {
        self.by_name.get(&name.to_ascii_lowercase()).map(|&i| &self.groups[i])
    }
}
