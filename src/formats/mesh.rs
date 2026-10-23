//! Meshes (`*.MESH`) and their material banks (`*_materials.mob`).
//!
//! A mesh file is a 240-byte header of u32 fields followed by arrays whose
//! sizes come from that header. Only the parts needed for drawing are kept:
//! positions (one set per animation frame), triangles, per-corner UVs and
//! per-triangle texture slots. Textures are either sibling files named
//! `<mesh>.<slot>.map` or records inside a `.mob` bank named by the mesh.
//!
//! People are skeletal. A body (flags 0x10000) has a skin: a bone table
//! with bind transforms and parents, and per-vertex influences (bone,
//! offset in bone space, weight). Bodies and their parts (`<mesh>-a`,
//! `-b`, ...; flags 0x8000) carry keyframe tracks on one 8-second, 30 fps
//! timeline: position and rotation offsets from the bind pose, and for
//! parts a visibility track. Named clips (node type 63) are runs of
//! timeline frames played at their own rate. Parts hang from the body's
//! type-0 nodes, in order: a parent bone name and an attach transform.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use glam::{Mat3, Mat4, Quat, Vec2, Vec3};

use super::texture::{self, Rgba};

const HEADER_WORDS: usize = 60;
const MAGIC: u32 = 0x34;

// Header word indices.
const H_TRIS: usize = 1;
const H_VERTS: usize = 2;
const H_FLAGS: usize = 0x13;
const H_UVS: usize = 0x1d;
const H_EDGES: usize = 0x1f;
const H_GROUPS: usize = 0x23;
const H_TRI_TAGS: usize = 0x24;
const H_NODES: usize = 0x25;
const H_EXTRA: usize = 0x2c;
const H_CHILD: usize = 0x2f;
const H_FRAMES: usize = 0x31;
const H_FACE_NORMALS: usize = 0x34;
const H_TEX_NAMES: usize = 0x37;
const H_TAIL_WORDS: usize = 0x3a;

const F_EDGES_PRESENT: u32 = 0x400;
const F_VERTEX_SHADE: u32 = 0x1000;
const F_TAIL_WORDS: u32 = 0x2000;
const F_MATERIAL_BANK: u32 = 0x20000;
const F_TRACKS: u32 = 0x8000;
const F_SKIN: u32 = 0x10000;

const NODE_ATTACH: u32 = 0;
const NODE_CLIPS: u32 = 63;
/// A light placed in the model (0x49daa0).
const NODE_LIGHT: u32 = 10000;
const CLIP_RECORD: usize = 200;

/// Frame rate of the shared keyframe timeline that clips index into.
pub const TRACK_FPS: f32 = 30.0;

/// A named animation: `frames` timeline frames from `start`, shown at `fps`.
#[derive(Debug, Clone)]
pub struct Clip {
    pub name: String,
    pub looping: bool,
    pub start: u32,
    pub frames: u32,
    pub fps: f32,
}

impl Clip {
    /// The timeline frame shown `t` seconds into the clip.
    pub fn frame_at(&self, t: f32) -> u32 {
        let n = self.frames.max(1);
        let k = (t.max(0.0) * self.fps) as u32;
        self.start + if self.looping { k % n } else { k.min(n - 1) }
    }
}

/// Keyframes for one bone or part joint, as offsets from its bind pose.
#[derive(Debug, Clone, Default)]
pub struct Track {
    pub pos: Vec<(f32, Vec3)>,
    pub rot: Vec<(f32, Quat)>,
    pub vis: Vec<(f32, bool)>,
}

impl Track {
    /// The offset transform at time `t` (linear keys, held at the ends).
    pub fn local(&self, t: f32) -> Mat4 {
        let pos = sample(&self.pos, t, Vec3::ZERO, |a, b, u| a.lerp(b, u));
        let rot = sample(&self.rot, t, Quat::IDENTITY, |a, b, u| a.slerp(b, u));
        Mat4::from_rotation_translation(rot, pos)
    }

    /// Whether the joint is shown at time `t` (stepped keys).
    pub fn visible(&self, t: f32) -> bool {
        self.vis.iter().take_while(|(k, _)| *k <= t + 1e-4).last().is_none_or(|(_, v)| *v)
    }
}

fn sample<T: Copy>(keys: &[(f32, T)], t: f32, none: T, mix: impl Fn(T, T, f32) -> T) -> T {
    let Some(first) = keys.first() else { return none };
    if t <= first.0 {
        return first.1;
    }
    for w in keys.windows(2) {
        let ((ta, a), (tb, b)) = (w[0], w[1]);
        if t <= tb {
            return mix(a, b, ((t - ta) / (tb - ta).max(1e-6)).clamp(0.0, 1.0));
        }
    }
    keys[keys.len() - 1].1
}

#[derive(Debug, Clone)]
pub struct Bone {
    pub name: String,
    pub parent: i32,
    pub bind: Mat4,
}

#[derive(Debug, Clone, Copy)]
pub struct Influence {
    pub bone: u16,
    pub offset: Vec3,
    pub weight: f32,
}

/// Vertex skinning: vertex `v` is the weighted sum of its influences
/// `first[v]..first[v + 1]`, each an offset placed by its bone.
#[derive(Debug, Clone)]
pub struct Skin {
    pub bones: Vec<Bone>,
    pub first: Vec<u16>,
    pub influences: Vec<Influence>,
}

/// Twelve floats as stored: a row-major 3×3 rotation (row vectors) and a
/// translation.
fn mat12(f: &[f32]) -> Mat4 {
    let r = Mat3::from_cols_array(&[f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]]);
    Mat4::from_cols(
        r.x_axis.extend(0.0),
        r.y_axis.extend(0.0),
        r.z_axis.extend(0.0),
        Vec3::new(f[9], f[10], f[11]).extend(1.0),
    )
}

#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub verts: [u16; 3],
    pub uvs: [u32; 3],
    pub texture: i32,
}

#[derive(Debug, Clone)]
pub enum TextureSource {
    /// Separate `.map` files: `<mesh>.<n>.map`, listed by name.
    Files(Vec<String>),
    /// A `.mob` bank next to the mesh.
    Bank(String),
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub frames: Vec<Vec<Vec3>>,
    pub triangles: Vec<Triangle>,
    pub uvs: Vec<Vec2>,
    pub textures: TextureSource,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    /// Attach points (named "string,cableA", "string,tbar", ...): where
    /// cables, riders and hangers attach.
    pub attach: Vec<(AttachPoint, Vec3)>,
    pub clips: Vec<Clip>,
    /// Parts `-a`, `-b`, ... in order: parent bone name and transform.
    pub parts: Vec<(String, Mat4)>,
    /// Light nodes: their transform and raw data.
    pub lights: Vec<(Mat4, Vec<u8>)>,

    /// Keyframe tracks by bone or joint name.
    pub tracks: Vec<(String, Track)>,
    pub skin: Option<Skin>,
}

/// A named attach point in a mesh: a marker the artists placed (a
/// node of type 50000) that the game finds by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachPoint {
    /// `cableA`, `cableB`, ...: where a lift's cable runs, 0 for A. The
    /// original looks these up ignoring case (0x49f4a0, through
    /// `_stricmp`), so the rope tow pole's `CableA` and `CableB` count.
    Cable(u8),
    /// `pt0`, `pt1`, ...: a lift's queue line, or the way off at the top.
    Path(u8),
    /// `ptb0`, `ptb1`, ...: a second queue line.
    PathB(u8),
    /// Where a drag lift hanger hangs from.
    Tbar,
    /// Where a skier's hands are.
    Tongs,
    /// Where a rider sits on a vehicle (the patrol snowmobile).
    Sit,
    Other,
}

impl AttachPoint {
    fn parse(name: &str) -> AttachPoint {
        let index = |rest: &str| rest.parse::<u8>().ok();
        let lower = name.to_ascii_lowercase();
        if let Some(&[c]) = lower.strip_prefix("cable").map(str::as_bytes) {
            if c.is_ascii_lowercase() {
                return AttachPoint::Cable(c - b'a');
            }
        }
        if let Some(i) = lower.strip_prefix("ptb").and_then(index) {
            AttachPoint::PathB(i)
        } else if let Some(i) = lower.strip_prefix("pt").and_then(index) {
            AttachPoint::Path(i)
        } else {
            match lower.as_str() {
                "tbar" => AttachPoint::Tbar,
                "tongs" => AttachPoint::Tongs,
                "sit" => AttachPoint::Sit,
                _ => AttachPoint::Other,
            }
        }
    }
}

/// Finds the attach point records: a 3×3 rotation and a translation as floats,
/// u32 type 50000, u32 name length, then the name (NUL-terminated).
fn attach_points(b: &[u8]) -> Vec<(AttachPoint, Vec3)> {
    let mut out = Vec::new();
    let pat = b"string,";
    let mut i = 8;
    while i + pat.len() < b.len() {
        if i >= 56 && &b[i..i + pat.len()] == pat {
            let typ = u32::from_le_bytes(b[i - 8..i - 4].try_into().unwrap());
            let len = u32::from_le_bytes(b[i - 4..i].try_into().unwrap()) as usize;
            if typ == 50000 && (8..64).contains(&len) && b.get(i + len - 1) == Some(&0) {
                let name = AttachPoint::parse(&String::from_utf8_lossy(&b[i + pat.len()..i + len - 1]));
                let f = |k: usize| b.get(i - 56 + k * 4..i - 52 + k * 4).map(|x| f32::from_le_bytes(x.try_into().unwrap()));
                if let (Some(x), Some(y), Some(z)) = (f(9), f(10), f(11)) {
                    if x.is_finite() && y.is_finite() && z.is_finite() && x.abs() < 1e5 && y.abs() < 1e5 && z.abs() < 1e5 {
                        out.push((name, Vec3::new(x, y, z)));
                    }
                }
                i += len;
                continue;
            }
        }
        i += 1;
    }
    out
}

struct Cursor<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self
            .b
            .get(self.p..self.p + n)
            .with_context(|| format!("mesh truncated at {:#x} (+{n})", self.p))?;
        self.p += n;
        Ok(s)
    }
    fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n).map(|_| ())
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32s<const N: usize>(&mut self) -> Result<[f32; N]> {
        let mut out = [0.0; N];
        for (o, w) in out.iter_mut().zip(self.take(N * 4)?.chunks_exact(4)) {
            *o = f32::from_le_bytes(w.try_into().unwrap());
        }
        Ok(out)
    }
}

fn parse_clips(d: &[u8]) -> Vec<Clip> {
    let word = |r: &[u8], o: usize| u32::from_le_bytes(r[o..o + 4].try_into().unwrap());
    d.get(4..)
        .unwrap_or_default()
        .chunks_exact(CLIP_RECORD)
        .map(|r| Clip {
            name: cstr(&r[..64]),
            looping: word(r, 172) != 0,
            start: word(r, 176),
            frames: word(r, 180),
            fps: f32::from_bits(word(r, 184)),
        })
        .collect()
}

fn parse_tracks(c: &mut Cursor) -> Result<Vec<(String, Track)>> {
    let _version = c.u32()?;
    let n = c.u32()? as usize;
    if n > 256 {
        bail!("implausible bone count {n}");
    }
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let name = cstr(c.take(64)?);
        let [_duration, _fps, _inv] = c.f32s::<3>()?;
        let mut track = Track::default();
        for _ in 0..c.u32()? {
            let [t, _, x, y, z] = c.f32s::<5>()?;
            track.pos.push((t, Vec3::new(x, y, z)));
        }
        for _ in 0..c.u32()? {
            let [t, _, x, y, z, w] = c.f32s::<6>()?;
            track.rot.push((t, Quat::from_xyzw(x, y, z, w).normalize()));
        }
        if c.u32()? != 0 {
            bail!("scale keys are not supported");
        }
        for _ in 0..c.u32()? {
            let [t, _, v, _] = c.f32s::<4>()?;
            track.vis.push((t, v > 0.5));
        }
        out.push((name, track));
    }
    Ok(out)
}

fn parse_skin(c: &mut Cursor) -> Result<Skin> {
    let nbones = c.u32()? as usize;
    let _max_influences = c.u32()?;
    let nvert = c.u32()? as usize;
    let ninf = c.u32()? as usize;
    if nbones > 256 || nvert > 1 << 20 || ninf > 1 << 22 {
        bail!("implausible skin counts");
    }
    let mut bones = Vec::with_capacity(nbones);
    for _ in 0..nbones {
        let m = c.f32s::<12>()?;
        let parent = c.u32()? as i32;
        let name = cstr(c.take(64)?);
        bones.push(Bone {
            name,
            parent,
            bind: mat12(&m),
        });
    }
    let first = c
        .take((nvert + 1) * 2)?
        .chunks_exact(2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .collect();
    let mut influences = Vec::with_capacity(ninf);
    for _ in 0..ninf {
        let r = c.take(20)?;
        let f = |k: usize| f32::from_le_bytes(r[k..k + 4].try_into().unwrap());
        influences.push(Influence {
            bone: u16::from_le_bytes([r[0], r[1]]),
            offset: Vec3::new(f(4), f(8), f(12)),
            weight: f(16),
        });
    }
    Ok(Skin {
        bones,
        first,
        influences,
    })
}

fn f32s(b: &[u8]) -> impl Iterator<Item = f32> + '_ {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

/// A light from a model's light node (0x519760): words 0 type (3 point,
/// 4 spot), 1 flags, 2–4 color, 5–13 rotation, 14–16 position, 18 outer
/// radius, 26 inner radius, 27–28 a spot's outer and inner cone angles.
#[derive(Debug, Clone, Copy)]
pub struct MeshLight {
    pub color: Vec3,
    pub pos: Vec3,
    pub outer: f32,
    pub inner: f32,
    pub spot: Option<Spot>,
}

/// A spot's direction (the rotation's first row, 0x51ac20) and its
/// cones' half angles in radians; the data gives whole angles in degrees,
/// the outer one 1° to 179°.
#[derive(Debug, Clone, Copy)]
pub struct Spot {
    pub dir: Vec3,
    pub outer: f32,
    pub inner: f32,
}

impl MeshLight {
    pub fn parse(data: &[u8]) -> Option<MeshLight> {
        let w = |i: usize| data.get(i * 4..i * 4 + 4).map(|b| f32::from_le_bytes(b.try_into().unwrap()));
        let kind = u32::from_le_bytes(data.get(0..4)?.try_into().ok()?);
        let flags = u32::from_le_bytes(data.get(4..8)?.try_into().ok()?);
        if kind != 3 && kind != 4 {
            return None;
        }
        let color = Vec3::new(w(2)?, w(3)?, w(4)?);
        let range = w(18)?;
        // Without the range flag, or a range, it reaches ten times its
        // brightest channel (0x51a5c0).
        let outer = if flags & 0x40 != 0 && range > 0.0 { range } else { color.max_element() * 10.0 };
        let spot = if kind == 4 {
            let outer_deg = w(27)?.clamp(1.0, 179.0);
            let inner_deg = w(28)?.clamp(0.0, outer_deg);
            let half = |deg: f32| deg * std::f32::consts::PI / 360.0;
            Some(Spot {
                dir: Vec3::new(w(5)?, w(6)?, w(7)?).normalize_or_zero(),
                outer: half(outer_deg),
                inner: half(inner_deg),
            })
        } else {
            None
        };
        // A spot's range is its own (0x51ac20), not the point lights'
        // default.
        let outer = if kind == 4 { range.max(0.0) } else { outer };
        Some(MeshLight {
            color,
            pos: Vec3::new(w(14)?, w(15)?, w(16)?),
            outer,
            inner: w(26)?.clamp(0.0, outer),
            spot,
        })
    }
}

impl Mesh {
    pub fn parse(b: &[u8]) -> Result<Mesh> {
        let mut c = Cursor { b, p: 0 };
        let hb = c.take(HEADER_WORDS * 4)?;
        let h: Vec<u32> = hb
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect();
        if h[0] != MAGIC {
            bail!("not a mesh (magic {:#x})", h[0]);
        }
        let ntri = h[H_TRIS] as usize;
        let nvert = h[H_VERTS] as usize;
        let nframes = (h[H_FRAMES] as usize).max(1);
        let flags = h[H_FLAGS];
        if ntri > 1 << 20 || nvert > 1 << 20 || nframes > 4096 {
            bail!("implausible mesh counts");
        }

        let tri_idx: Vec<[u16; 3]> = c
            .take(ntri * 6)?
            .chunks_exact(6)
            .map(|t| {
                [
                    u16::from_le_bytes([t[0], t[1]]),
                    u16::from_le_bytes([t[2], t[3]]),
                    u16::from_le_bytes([t[4], t[5]]),
                ]
            })
            .collect();

        let pos: Vec<f32> = f32s(c.take(nvert * nframes * 12)?).collect();
        let frames: Vec<Vec<Vec3>> = (0..nframes)
            .map(|f| {
                (0..nvert)
                    .map(|v| {
                        let i = (f * nvert + v) * 3;
                        Vec3::new(pos[i], pos[i + 1], pos[i + 2])
                    })
                    .collect()
            })
            .collect();

        let corners = c.take(ntri * 40)?;
        let nuv = h[H_UVS] as usize;
        let uvs: Vec<Vec2> = f32s(c.take(nuv * 8)?)
            .collect::<Vec<_>>()
            .chunks_exact(2)
            .map(|p| Vec2::new(p[0], p[1]))
            .collect();

        let mut triangles = Vec::with_capacity(ntri);
        for (t, idx) in tri_idx.iter().enumerate() {
            let r: Vec<i32> = corners[t * 40..t * 40 + 40]
                .chunks_exact(4)
                .map(|w| i32::from_le_bytes(w.try_into().unwrap()))
                .collect();
            let uv = |k: usize| {
                let i = r[1 + k * 3];
                if i >= 0 && (i as usize) < nuv { i as u32 } else { 0 }
            };
            triangles.push(Triangle {
                verts: *idx,
                uvs: [uv(0), uv(1), uv(2)],
                texture: r[0],
            });
        }

        // Everything below is skipped over to reach the texture table.
        if h[H_FACE_NORMALS] != 0 {
            c.skip(ntri * nframes * 12)?;
        }
        let nedges = h[H_EDGES] as usize;
        if nedges != 0 {
            if flags & F_EDGES_PRESENT != 0 {
                c.skip(nedges * 12 + ntri * 12 + nedges * nframes * 2)?;
            } else {
                c.skip(((nframes + 1) * nedges + ntri) * 12)?;
            }
        }
        if h[H_GROUPS] != 0 {
            let save = c.p;
            let a = c.u32()? as usize;
            let b2 = c.u32()? as usize;
            c.p = save;
            c.skip((b2 + 2 + 2 * a) * 4)?;
        }
        if h[H_TRI_TAGS] != 0 {
            c.skip(ntri * 4)?;
        }
        let nnodes = h[H_NODES] as usize;
        let mut clips = Vec::new();
        let mut parts = Vec::new();
        let mut lights = Vec::new();
        for _ in 0..nnodes {
            let m = c.f32s::<12>()?;
            let kind = c.u32()?;
            let len = c.u32()? as usize;
            let data = c.take(len)?;
            match kind {
                NODE_CLIPS => clips = parse_clips(data),
                NODE_ATTACH => parts.push((cstr(data), mat12(&m))),
                NODE_LIGHT => lights.push((mat12(&m), data.to_vec())),
                _ => {}
            }
        }
        let mut node_xforms = 0;
        for _ in 0..nnodes {
            if c.u32()? != 0 {
                node_xforms += 1;
            }
        }
        c.skip(node_xforms * nframes * 48)?;
        c.skip(h[H_EXTRA] as usize * 4)?;

        let textures = if flags & F_MATERIAL_BANK != 0 {
            TextureSource::Bank(cstr(c.take(64)?))
        } else if h[H_CHILD] != 0 {
            TextureSource::Files(Vec::new())
        } else {
            let n = h[H_TEX_NAMES] as usize;
            let mut names = Vec::with_capacity(n);
            for _ in 0..n {
                let rec = c.take(32)?;
                names.push(cstr(&rec[4..]));
            }
            TextureSource::Files(names)
        };
        // Skeletal data follows one u16 per vertex and the tail words.
        let mut tracks = Vec::new();
        let mut skin = None;
        if flags & (F_TRACKS | F_SKIN) != 0 {
            let skel = (|| -> Result<()> {
                c.skip(nvert * 2)?;
                c.skip(h[H_TAIL_WORDS] as usize * 4)?;
                if flags & F_TRACKS != 0 {
                    tracks = parse_tracks(&mut c)?;
                }
                if flags & F_SKIN != 0 {
                    skin = Some(parse_skin(&mut c)?);
                }
                Ok(())
            })();
            if let Err(e) = skel {
                log::warn!("skeleton: {e:#}");
                tracks.clear();
                skin = None;
            }
        }
        let _ = (F_VERTEX_SHADE, F_TAIL_WORDS);

        let mut bounds_min = Vec3::splat(f32::MAX);
        let mut bounds_max = Vec3::splat(f32::MIN);
        for p in &frames[0] {
            bounds_min = bounds_min.min(*p);
            bounds_max = bounds_max.max(*p);
        }
        if frames[0].is_empty() {
            bounds_min = Vec3::ZERO;
            bounds_max = Vec3::ZERO;
        }
        Ok(Mesh {
            frames,
            triangles,
            uvs,
            textures,
            bounds_min,
            bounds_max,
            attach: attach_points(b),
            clips,
            parts,
            lights,
            tracks,
            skin,
        })
    }

    pub fn track(&self, name: &str) -> Option<&Track> {
        self.tracks.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }

    pub fn clip(&self, name: &str) -> Option<&Clip> {
        self.clips.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Bone world transforms at time `t` on the track timeline.
    pub fn bone_worlds(&self, t: f32) -> Vec<Mat4> {
        let Some(skin) = &self.skin else { return Vec::new() };
        let mut out: Vec<Mat4> = Vec::with_capacity(skin.bones.len());
        for b in &skin.bones {
            let offset = self.track(&b.name).map_or(Mat4::IDENTITY, |tr| tr.local(t));
            // Position offsets apply in the parent's space, rotations in the bone's.
            let (_, rot, pos) = offset.to_scale_rotation_translation();
            let local = Mat4::from_translation(pos) * b.bind * Mat4::from_quat(rot);
            let parent = usize::try_from(b.parent).ok().and_then(|p| out.get(p)).copied();
            out.push(parent.map_or(local, |p| p * local));
        }
        out
    }

    /// Skinned vertex positions for the given bone transforms.
    pub fn skinned(&self, worlds: &[Mat4]) -> Vec<Vec3> {
        let Some(skin) = &self.skin else { return self.frames[0].clone() };
        (0..self.frames[0].len())
            .map(|v| {
                let (a, b) = (
                    skin.first.get(v).copied().unwrap_or(0) as usize,
                    skin.first.get(v + 1).copied().unwrap_or(0) as usize,
                );
                skin.influences
                    .get(a..b)
                    .unwrap_or_default()
                    .iter()
                    .map(|i| worlds.get(i.bone as usize).map_or(i.offset, |w| w.transform_point3(i.offset)) * i.weight)
                    .sum()
            })
            .collect()
    }

    /// Transform of a bone by name, `Scene Root` and unknown names being
    /// the model origin.
    pub fn bone_world(&self, worlds: &[Mat4], name: &str) -> Mat4 {
        self.skin
            .as_ref()
            .and_then(|s| s.bones.iter().position(|b| b.name == name))
            .and_then(|i| worlds.get(i).copied())
            .unwrap_or(Mat4::IDENTITY)
    }

    pub fn load(path: &Path) -> Result<Mesh> {
        let b = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Mesh::parse(&b).with_context(|| format!("parsing {}", path.display()))
    }

    /// Loads the textures this mesh refers to, indexed by triangle slot.
    /// Missing or undecodable textures become `None`.
    pub fn load_textures(&self, mesh_path: &Path) -> Vec<Option<Rgba>> {
        let dir = mesh_path.parent().unwrap_or(Path::new("."));
        match &self.textures {
            TextureSource::Files(names) => {
                let count = self
                    .triangles
                    .iter()
                    .map(|t| t.texture + 1)
                    .max()
                    .unwrap_or(0)
                    .max(names.len() as i32) as usize;
                let file = mesh_path.file_name().unwrap().to_string_lossy();
                (0..count)
                    .map(|i| {
                        let p = find_ci(dir, &format!("{file}.{i}.map"))?;
                        texture::load_any(&p).ok()
                    })
                    .collect()
            }
            TextureSource::Bank(name) => match find_ci(dir, name) {
                Some(p) => load_bank(&p).unwrap_or_default(),
                None => Vec::new(),
            },
        }
    }
}

/// Reads a `.mob` material bank: u32 version, u32 count, then `count`
/// 76-byte entries (name, ..., data offset at +0x44, data size at +0x48),
/// each pointing at an embedded texture record.
pub fn load_bank(path: &Path) -> Result<Vec<Option<Rgba>>> {
    let b = std::fs::read(path)?;
    let count = u32::from_le_bytes(b.get(4..8).context("short mob")?.try_into()?) as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let e = 8 + i * 76;
        let off = u32::from_le_bytes(b.get(e + 0x44..e + 0x48).context("short mob")?.try_into()?);
        out.push(texture::decode_record(&b, off as usize).ok().map(|r| r.0));
    }
    Ok(out)
}

/// Case-insensitive file lookup inside one directory; the game data was
/// authored on Windows and its references do not match on-disk case.
pub fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.exists() {
        return Some(direct);
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
}
