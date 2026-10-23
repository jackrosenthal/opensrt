//! Installs the game data straight from the original CD image:
//! CUE/BIN (or ISO) → ISO 9660 → InstallShield 5 cabinet (`data1.hdr` +
//! `data1.cab`) → files on disk. Nothing from the original installer runs.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Sector-addressed access to the data track of a disc image.
struct Disc {
    file: File,
    /// Bytes per raw sector (2048 for ISO, 2352 for MODE1/2352).
    raw: u64,
    /// Offset of user data inside a raw sector.
    skip: u64,
}

impl Disc {
    fn open(path: &Path) -> Result<Disc> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "cue" {
            let text = std::fs::read_to_string(path)?;
            let mut bin = None;
            let mut mode = None;
            for line in text.lines() {
                let l = line.trim();
                if let Some(rest) = l.strip_prefix("FILE") {
                    let name = rest.trim().trim_start_matches('"');
                    let name = name.split('"').next().unwrap_or(name).trim();
                    bin.get_or_insert_with(|| name.to_string());
                } else if let Some(rest) = l.strip_prefix("TRACK") {
                    if mode.is_none() {
                        mode = rest.split_whitespace().nth(1).map(|s| s.to_string());
                    }
                }
            }
            let bin = bin.context("cue sheet has no FILE line")?;
            let dir = path.parent().unwrap_or(Path::new("."));
            let bin_path = find_ci(dir, &bin).with_context(|| format!("{bin} not found next to the cue sheet"))?;
            let (raw, skip) = match mode.as_deref() {
                Some("MODE1/2048") => (2048, 0),
                Some("MODE1/2352") | None => (2352, 16),
                Some("MODE2/2352") => (2352, 24),
                Some(m) => bail!("unsupported track mode {m}"),
            };
            return Ok(Disc {
                file: File::open(bin_path)?,
                raw,
                skip,
            });
        }
        let mut file = File::open(path)?;
        // Raw images start with a sync pattern; cooked ISOs do not.
        let mut sync = [0u8; 12];
        file.read_exact(&mut sync)?;
        let raw_sync = [0u8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0];
        let (raw, skip) = if sync == raw_sync { (2352, 16) } else { (2048, 0) };
        Ok(Disc { file, raw, skip })
    }

    fn read(&mut self, lba: u64, len: u64, out: &mut Vec<u8>) -> Result<()> {
        out.clear();
        let mut remaining = len;
        let mut sector = lba;
        let mut buf = vec![0u8; 2048];
        while remaining > 0 {
            self.file.seek(SeekFrom::Start(sector * self.raw + self.skip))?;
            self.file.read_exact(&mut buf)?;
            let n = remaining.min(2048);
            out.extend_from_slice(&buf[..n as usize]);
            remaining -= n;
            sector += 1;
        }
        Ok(())
    }
}

struct IsoFile {
    lba: u64,
    size: u64,
}

fn iso_index(disc: &mut Disc) -> Result<HashMap<String, IsoFile>> {
    let mut pvd = Vec::new();
    disc.read(16, 2048, &mut pvd)?;
    if &pvd[1..6] != b"CD001" {
        bail!("not an ISO 9660 image");
    }
    let root = &pvd[156..156 + 34];
    let mut out = HashMap::new();
    let mut stack = vec![(
        String::new(),
        u32::from_le_bytes(root[2..6].try_into()?) as u64,
        u32::from_le_bytes(root[10..14].try_into()?) as u64,
    )];
    let mut buf = Vec::new();
    while let Some((prefix, lba, size)) = stack.pop() {
        disc.read(lba, size, &mut buf)?;
        let mut p = 0usize;
        while p < buf.len() {
            let len = buf[p] as usize;
            if len == 0 {
                // Records never straddle sectors; skip to the next one.
                p = (p / 2048 + 1) * 2048;
                continue;
            }
            let r = &buf[p..p + len];
            let ext_lba = u32::from_le_bytes(r[2..6].try_into()?) as u64;
            let ext_size = u32::from_le_bytes(r[10..14].try_into()?) as u64;
            let flags = r[25];
            let nlen = r[32] as usize;
            let name = &r[33..33 + nlen];
            p += len;
            if name == [0] || name == [1] {
                continue;
            }
            let mut name = String::from_utf8_lossy(name).into_owned();
            if let Some(i) = name.find(';') {
                name.truncate(i);
            }
            let name = name.trim_end_matches('.').to_ascii_lowercase();
            let full = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
            if flags & 2 != 0 {
                stack.push((full, ext_lba, ext_size));
            } else {
                out.insert(full, IsoFile { lba: ext_lba, size: ext_size });
            }
        }
    }
    Ok(out)
}

struct CabFile {
    dir: String,
    name: String,
    flags: u16,
    expanded: u64,
    compressed: u64,
    offset: u64,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(b.get(o..o + 2).context("header truncated")?.try_into()?))
}
fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(b.get(o..o + 4).context("header truncated")?.try_into()?))
}
fn cstr(b: &[u8], o: usize) -> String {
    let end = b[o..].iter().position(|&c| c == 0).map(|e| o + e).unwrap_or(b.len());
    String::from_utf8_lossy(&b[o..end]).into_owned()
}

/// Parses an InstallShield 5 header (`data1.hdr`).
fn cab_files(hdr: &[u8]) -> Result<Vec<CabFile>> {
    if u32_at(hdr, 0)? != 0x2863_5349 {
        bail!("data1.hdr is not an InstallShield cabinet header");
    }
    let version = u32_at(hdr, 4)?;
    let major = if version >> 24 == 1 { (version >> 12) & 0xf } else { (version & 0xffff) / 100 };
    if major != 5 {
        bail!("unsupported InstallShield version {major} ({version:#x})");
    }
    let desc = u32_at(hdr, 12)? as usize;
    let table = desc + u32_at(hdr, desc + 0x0c)? as usize;
    let dirs = u32_at(hdr, desc + 0x1c)? as usize;
    let files = u32_at(hdr, desc + 0x28)? as usize;
    let offset = |i: usize| -> Result<usize> { Ok(table + u32_at(hdr, table + i * 4)? as usize) };
    let dir_names: Vec<String> = (0..dirs).map(|i| offset(i).map(|o| cstr(hdr, o))).collect::<Result<_>>()?;
    let mut out = Vec::with_capacity(files);
    for i in 0..files {
        let o = offset(dirs + i)?;
        let name = cstr(hdr, table + u32_at(hdr, o)? as usize);
        let dir_index = u32_at(hdr, o + 4)? as usize;
        let flags = u16_at(hdr, o + 8)?;
        let expanded = u32_at(hdr, o + 10)? as u64;
        let compressed = u32_at(hdr, o + 14)? as u64;
        let offset = u32_at(hdr, o + 38)? as u64;
        out.push(CabFile {
            dir: dir_names.get(dir_index).cloned().unwrap_or_default(),
            name,
            flags,
            expanded,
            compressed,
            offset,
        });
    }
    Ok(out)
}

const FLAG_INVALID: u16 = 0x8;
const FLAG_COMPRESSED: u16 = 0x4;
const FLAG_OBFUSCATED: u16 = 0x2;

fn deobfuscate(data: &mut [u8], seed: &mut u32) {
    for b in data.iter_mut() {
        let v = (*b ^ 0xd5).rotate_right(2);
        *b = v.wrapping_sub((*seed % 0x47) as u8);
        *seed += 1;
    }
}

pub fn extract(image: &Path, out: &Path) -> Result<()> {
    println!("Reading {} ...", image.display());
    let mut disc = Disc::open(image)?;
    let iso = iso_index(&mut disc)?;
    let hdr_entry = iso.get("data1.hdr").context("data1.hdr not found on the disc")?;
    let mut hdr = Vec::new();
    disc.read(hdr_entry.lba, hdr_entry.size, &mut hdr)?;
    let files = cab_files(&hdr)?;
    let cab = iso.get("data1.cab").context("data1.cab not found on the disc")?;
    // The cabinet is a contiguous extent; read files straight from the image.
    let cab_bytes_at = |disc: &mut Disc, off: u64, len: u64, buf: &mut Vec<u8>| -> Result<()> {
        let first = cab.lba + off / 2048;
        let skip = off % 2048;
        disc.read(first, skip + len, buf)?;
        buf.drain(..skip as usize);
        Ok(())
    };
    let wanted = |dir: &str| {
        let d = dir.to_ascii_lowercase();
        d.is_empty() || d == "data" || d.starts_with("data\\") || d == "generatedsprites"
    };
    let total = files.iter().filter(|f| wanted(&f.dir) && f.flags & FLAG_INVALID == 0).count();
    std::fs::create_dir_all(out)?;
    let mut done = 0;
    let mut buf = Vec::new();
    for f in &files {
        if f.flags & FLAG_INVALID != 0 || !wanted(&f.dir) {
            continue;
        }
        let rel: PathBuf = f.dir.split('\\').filter(|s| !s.is_empty()).chain([f.name.as_str()]).collect();
        let dest = out.join(&rel);
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p)?;
        }
        let stored = if f.flags & FLAG_COMPRESSED != 0 { f.compressed } else { f.expanded };
        cab_bytes_at(&mut disc, f.offset, stored, &mut buf)?;
        if f.flags & FLAG_OBFUSCATED != 0 {
            let mut seed = 0;
            deobfuscate(&mut buf, &mut seed);
        }
        let data = if f.flags & FLAG_COMPRESSED != 0 {
            let mut outv = Vec::with_capacity(f.expanded as usize);
            let mut p = 0usize;
            while p + 2 <= buf.len() && (outv.len() as u64) < f.expanded {
                let n = u16::from_le_bytes([buf[p], buf[p + 1]]) as usize;
                p += 2;
                let chunk = buf.get(p..p + n).context("truncated compressed chunk")?;
                p += n;
                let mut dec = flate2::read::DeflateDecoder::new(chunk);
                dec.read_to_end(&mut outv)?;
            }
            outv
        } else {
            buf.clone()
        };
        if data.len() as u64 != f.expanded {
            bail!("{}: expected {} bytes, got {}", rel.display(), f.expanded, data.len());
        }
        File::create(&dest)?.write_all(&data)?;
        done += 1;
        if done % 200 == 0 || done == total {
            println!("  {done}/{total} files");
        }
    }
    println!("Game data installed to {}", out.display());
    Ok(())
}

fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.exists() {
        return Some(direct);
    }
    std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    })
}
