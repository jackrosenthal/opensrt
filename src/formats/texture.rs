//! Texture containers: mesh textures (`*.map`, also embedded in `*.mob`) and
//! the `data/textures/*.bin` files.
//!
//! Every record starts with a u16 version and a u16 "compressed" flag.
//! Versions 0x1002/0x1003 follow that with two u32s and a NUL-terminated
//! string; older ones go straight to the descriptor. Descriptor fields
//! (relative to its start):
//! - `0x00` source image name (char[64])
//! - `0x44` width, `0x48` height, `0x4c` bytes per pixel
//! - `0x54..0x64` R/G/B/A channel masks
//! - `0x64` flags; bits 2/3 mean an alpha plane accompanies compressed data
//! - `0x78` mip level count `n`, `0x80` size of the whole decoded mip chain
//! - `0x8c` `n` u32 offsets of each mip inside the decoded buffer
//!
//! Uncompressed records then hold the mip chain verbatim. Compressed records
//! hold an optional alpha plane (u32 length, then raw bytes if the length
//! equals the pixel count, otherwise `(value, run)` pairs) followed by `n`
//! offsets of per-mip JPEG streams. Those offsets are absolute in old
//! versions and relative to the end of the tables in versions > 0x1002.

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub name: String,
}

impl Rgba {
    pub fn solid(rgba: [u8; 4]) -> Rgba {
        Rgba {
            width: 1,
            height: 1,
            pixels: rgba.to_vec(),
            name: String::new(),
        }
    }
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    let s = b.get(o..o + 4).context("texture record truncated")?;
    Ok(u32::from_le_bytes(s.try_into().unwrap()))
}

fn channel(px: u32, mask: u32) -> Option<u8> {
    if mask == 0 {
        return None;
    }
    let shift = mask.trailing_zeros();
    let bits = (mask >> shift).count_ones();
    let v = (px & mask) >> shift;
    Some(if bits >= 8 {
        (v >> (bits - 8)) as u8
    } else {
        ((v * 255) / ((1 << bits) - 1)) as u8
    })
}

/// Decodes one record starting at `base` and returns the top mip level along
/// with the number of bytes the record occupies.
pub fn decode_record(b: &[u8], base: usize) -> Result<(Rgba, usize)> {
    let version = u16::from_le_bytes(b.get(base..base + 2).context("empty")?.try_into()?);
    let compressed = u16::from_le_bytes(b.get(base + 2..base + 4).context("empty")?.try_into()?);
    let mut s = base + 4;
    if (0x1002..=0x1003).contains(&version) {
        s += 8;
        let nul = b[s..].iter().position(|&c| c == 0).context("bad name")?;
        s += nul + 1;
    }
    let name_bytes = b.get(s..s + 64).context("short header")?;
    let name_end = name_bytes.iter().position(|&c| c == 0).unwrap_or(64);
    let name = String::from_utf8_lossy(&name_bytes[..name_end]).into_owned();
    let width = u32_at(b, s + 0x44)?;
    let height = u32_at(b, s + 0x48)?;
    let bpp = u32_at(b, s + 0x4c)?;
    let masks = [
        u32_at(b, s + 0x54)?,
        u32_at(b, s + 0x58)?,
        u32_at(b, s + 0x5c)?,
        u32_at(b, s + 0x60)?,
    ];
    let flags = u32_at(b, s + 0x64)?;
    let mips = u32_at(b, s + 0x78)? as usize;
    let chain_bytes = u32_at(b, s + 0x80)? as usize;
    if width == 0 || height == 0 || width > 4096 || height > 4096 || mips == 0 || mips > 16 {
        bail!("implausible texture header {width}x{height} mips={mips}");
    }
    if bpp != 4 {
        bail!("unsupported texture depth {bpp}");
    }
    let n = (width * height) as usize;
    let tables = s + 0x8c;
    let mut p = tables + 4 * mips;
    let mut pixels = vec![255u8; n * 4];

    if compressed == 0 {
        let raw = b.get(p..p + n * 4).context("raw texture truncated")?;
        for (dst, src) in pixels.chunks_exact_mut(4).zip(raw.chunks_exact(4)) {
            let px = u32::from_le_bytes(src.try_into().unwrap());
            for c in 0..3 {
                dst[c] = channel(px, masks[c]).unwrap_or(0);
            }
            dst[3] = match channel(px, masks[3]) {
                Some(a) => a,
                None if flags & 0xC != 0 => src[3],
                None => 255,
            };
        }
        let end = p - base + chain_bytes;
        return Ok((
            Rgba {
                width,
                height,
                pixels,
                name,
            },
            end,
        ));
    }

    let alpha_start = p;
    let mut alpha: Option<Vec<u8>> = None;
    if flags & 0xC != 0 {
        let len = u32_at(b, p)? as usize;
        p += 4;
        let data = b.get(p..p + len).context("alpha plane truncated")?;
        p += len;
        let total = chain_bytes / 4;
        if len == total {
            alpha = Some(data.to_vec());
        } else {
            let mut a = Vec::with_capacity(total);
            for pair in data.chunks_exact(2) {
                a.extend(std::iter::repeat_n(pair[0], pair[1] as usize));
            }
            alpha = Some(a);
        }
    }
    let alpha_len = p - alpha_start;
    let mut offs: Vec<usize> = (0..mips)
        .map(|i| u32_at(b, p + 4 * i).map(|v| v as usize))
        .collect::<Result<_>>()?;
    if version > 0x1002 {
        let rel_base = tables - base + alpha_len + mips * 8;
        for o in &mut offs {
            *o += rel_base;
        }
    }
    let stream = |i: usize| -> Result<&[u8]> {
        let start = base + offs[i];
        let tail = b.get(start..).context("jpeg offset out of range")?;
        let eoi = tail
            .windows(2)
            .position(|w| w == [0xFF, 0xD9])
            .map(|e| e + 2)
            .unwrap_or(tail.len());
        Ok(&tail[..eoi])
    };
    let img = image::load_from_memory_with_format(stream(0)?, image::ImageFormat::Jpeg)?.to_rgb8();
    if img.width() != width || img.height() != height {
        bail!("jpeg size mismatch");
    }
    for (dst, src) in pixels.chunks_exact_mut(4).zip(img.pixels()) {
        dst[..3].copy_from_slice(&src.0);
    }
    if let Some(a) = alpha {
        for (dst, &v) in pixels.chunks_exact_mut(4).zip(a.iter()) {
            dst[3] = v;
        }
    }
    let end = offs[mips - 1] + stream(mips - 1)?.len();
    Ok((
        Rgba {
            width,
            height,
            pixels,
            name,
        },
        end,
    ))
}

/// Loads a standalone `.map` or `data/textures/*.bin` file.
pub fn load_record_file(bytes: &[u8]) -> Result<Rgba> {
    Ok(decode_record(bytes, 0)?.0)
}

/// Loads any image the game references. TGA/BMP/PNG go through `image`.
pub fn load_any(path: &std::path::Path) -> Result<Rgba> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "map" | "bin" => load_record_file(&bytes),
        _ => {
            // TGA has no magic number to sniff.
            let img = match ext.as_str() {
                "tga" => image::load_from_memory_with_format(&bytes, image::ImageFormat::Tga)?,
                _ => image::load_from_memory(&bytes)?,
            }
            .to_rgba8();
            Ok(Rgba {
                width: img.width(),
                height: img.height(),
                pixels: img.into_raw(),
                name: path.file_name().unwrap().to_string_lossy().into_owned(),
            })
        }
    }
}
