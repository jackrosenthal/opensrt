//! Frame assembly: terrain, mesh batches and a shared globals buffer.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3, Vec4};

use super::lines::{LinePipeline, LineVertex};
use super::mesh::{GpuMesh, Instance, MeshPipeline};
use super::terrain::TerrainRenderer;
use super::{Globals, Gpu};
use crate::assets::Assets;
use crate::formats::mesh::{Clip, Mesh, TRACK_FPS};
use crate::formats::texture::Rgba;

/// Identifies a loaded mesh inside the [`MeshCache`].
pub type MeshId = usize;

/// A parsed mesh and its uploaded textures, shared by all its frames.
pub struct Source {
    pub mesh: Mesh,
    views: Vec<Option<wgpu::TextureView>>,
}

#[derive(Default)]
pub struct MeshCache {
    by_path: HashMap<(String, usize), MeshId>,
    /// The lights in each loaded mesh, in its own space.
    pub lights: HashMap<MeshId, Arc<Vec<crate::formats::mesh::MeshLight>>>,
    pub meshes: Vec<Option<GpuMesh>>,
    sources: HashMap<String, Option<Arc<Source>>>,
    /// Where each mesh came from: its source key and frame.
    origin: HashMap<MeshId, (String, usize)>,
    /// Line-test solids of rigid meshes, built when first asked for.
    solids: HashMap<MeshId, Option<Arc<crate::game::lamplight::Solid>>>,
}

fn key_of(rel: &str) -> String {
    rel.to_ascii_lowercase().replace('\\', "/")
}

/// The path of part `n` of a mesh: `<stem>-a.<ext>`, `<stem>-b.<ext>`, ...
fn part_path(rel: &str, n: usize) -> String {
    let (stem, ext) = rel.rsplit_once('.').unwrap_or((rel, "mesh"));
    format!("{stem}-{}.{ext}", (b'a' + n as u8) as char)
}

impl MeshCache {
    /// Parses (once) the mesh at a game-relative path and uploads its textures.
    pub fn source(&mut self, gpu: &Gpu, assets: &Assets, rel: &str) -> Option<Arc<Source>> {
        let key = key_of(rel);
        if let Some(s) = self.sources.get(&key) {
            return s.clone();
        }
        let src = assets.resolve(rel).and_then(|p| {
            let mesh = Mesh::load(p).map_err(|e| log::warn!("{e:#}")).ok()?;
            let views = MeshPipeline::texture_views(gpu, &mesh.load_textures(p));
            Some(Arc::new(Source { mesh, views }))
        });
        self.sources.insert(key, src.clone());
        src
    }

    /// The green "ENTRANCE" sign over the entrance's arch. The original
    /// draws buildings from pre-rendered sprites and the sign exists only
    /// there, so it is cut out of the front view of
    /// `BUILDINGS_Entrance_Entrance.spr` and set on a quad in the entrance
    /// mesh's frame. That view looks from the (-x, +y) diagonal, 30° up,
    /// at 6.78 pixels per unit; the corners below put the letters' pixel
    /// box (200..520, 116..201) on a vertical plane just in front of the
    /// mesh's sign bar (its front face is 15.6 units out along the view).
    pub fn entrance_sign(&mut self, gpu: &Gpu, pipe: &MeshPipeline, assets: &Assets) -> Option<MeshId> {
        const KEY: &str = "#entrance sign";
        if let Some(&id) = self.by_path.get(&(KEY.to_string(), 0)) {
            return self.meshes[id].is_some().then_some(id);
        }
        let mesh = entrance_sign_mesh(gpu, pipe, assets);
        if mesh.is_none() {
            log::warn!("entrance sign unavailable");
        }
        let id = self.meshes.len();
        let ok = mesh.is_some();
        self.meshes.push(mesh);
        self.by_path.insert((KEY.to_string(), 0), id);
        ok.then_some(id)
    }

    /// A unit square in the x/z plane (facing ±y) textured with an image,
    /// for camera-facing particles. The particle images carry their shape
    /// in alpha and are colored by the engine, so only alpha is kept and
    /// the color is white.
    pub fn billboard(&mut self, gpu: &Gpu, pipe: &MeshPipeline, assets: &Assets, tex: &str) -> Option<MeshId> {
        let key = format!("#billboard {}", key_of(tex));
        if let Some(&id) = self.by_path.get(&(key.clone(), 0)) {
            return self.meshes[id].is_some().then_some(id);
        }
        let mesh = assets
            .resolve(tex)
            .and_then(|p| crate::formats::texture::load_any(p).map_err(|e| log::warn!("{e:#}")).ok())
            .map(|mut img| {
                for px in img.pixels.chunks_exact_mut(4) {
                    px[..3].fill(255);
                }
                let views = MeshPipeline::texture_views(gpu, &[Some(img)]);
                let m = quad([
                    Vec3::new(-0.5, 0.0, 0.5),
                    Vec3::new(0.5, 0.0, 0.5),
                    Vec3::new(-0.5, 0.0, -0.5),
                    Vec3::new(0.5, 0.0, -0.5),
                ]);
                pipe.upload(gpu, &[(&m, &m.frames[0], &views)])
            });
        let id = self.meshes.len();
        let ok = mesh.is_some();
        self.meshes.push(mesh);
        self.by_path.insert((key, 0), id);
        ok.then_some(id)
    }

    /// A clip of a skeletal mesh by name.
    pub fn clip(&mut self, gpu: &Gpu, assets: &Assets, rel: &str, name: &str) -> Option<Clip> {
        self.source(gpu, assets, rel)?.mesh.clip(name).cloned()
    }

    /// Loads (once) the mesh at a game-relative path and frame: a vertex
    /// animation frame, or for a skinned body the timeline frame, posed
    /// with its visible parts attached.
    pub fn get(
        &mut self,
        gpu: &Gpu,
        pipe: &MeshPipeline,
        assets: &Assets,
        rel: &str,
        frame: usize,
    ) -> MeshId {
        let key = (key_of(rel), frame);
        if let Some(&id) = self.by_path.get(&key) {
            return id;
        }
        let mesh = self.source(gpu, assets, rel).map(|src| {
            let m = &src.mesh;
            if m.skin.is_none() {
                let pos = &m.frames[frame.min(m.frames.len() - 1)];
                return pipe.upload(gpu, &[(m, pos, &src.views)]);
            }
            let t = frame as f32 / TRACK_FPS;
            let worlds = m.bone_worlds(t);
            let body = m.skinned(&worlds);
            let mut posed = vec![(src.clone(), body)];
            for (i, (parent, attach)) in m.parts.iter().enumerate() {
                let Some(part) = self.source(gpu, assets, &part_path(rel, i)) else { continue };
                let joint = part.mesh.tracks.first().map(|(_, tr)| tr);
                if !joint.is_none_or(|tr| tr.visible(t)) {
                    continue;
                }
                let local = joint.map_or(glam::Mat4::IDENTITY, |tr| tr.local(t));
                let world = m.bone_world(&worlds, parent) * *attach * local;
                let pos = part.mesh.frames[0].iter().map(|p| world.transform_point3(*p)).collect();
                posed.push((part, pos));
            }
            let pieces: Vec<_> = posed.iter().map(|(s, p)| (&s.mesh, p.as_slice(), s.views.as_slice())).collect();
            pipe.upload(gpu, &pieces)
        });
        let lights: Vec<_> = self
            .source(gpu, assets, rel)
            .map(|s| s.mesh.lights.iter().filter_map(|(_, d)| crate::formats::mesh::MeshLight::parse(d)).collect())
            .unwrap_or_default();
        if mesh.is_none() {
            log::warn!("mesh {rel} unavailable");
        }
        let id = self.meshes.len();
        self.meshes.push(mesh);
        self.origin.insert(id, key.clone());
        self.by_path.insert(key, id);
        if !lights.is_empty() {
            self.lights.insert(id, Arc::new(lights));
        }
        id
    }

    /// A rigid mesh's triangles for line tests (none for skinned ones).
    pub fn solid(&mut self, id: MeshId) -> Option<Arc<crate::game::lamplight::Solid>> {
        if let Some(s) = self.solids.get(&id) {
            return s.clone();
        }
        let solid = self.origin.get(&id).and_then(|(key, frame)| {
            let src = self.sources.get(key)?.as_ref()?;
            let m = &src.mesh;
            if m.skin.is_some() || m.frames.is_empty() {
                return None;
            }
            let pos = &m.frames[(*frame).min(m.frames.len() - 1)];
            let tris = m
                .triangles
                .iter()
                .filter_map(|t| Some([*pos.get(t.verts[0] as usize)?, *pos.get(t.verts[1] as usize)?, *pos.get(t.verts[2] as usize)?]))
                .collect();
            Some(Arc::new(crate::game::lamplight::Solid::new(tris)))
        });
        self.solids.insert(id, solid.clone());
        solid
    }

    /// Animation frame count of a loaded mesh (1 when unavailable).
    pub fn frames(&self, id: MeshId) -> usize {
        self.meshes.get(id).and_then(|m| m.as_ref()).map_or(1, |m| m.frames)
    }
}

/// Per-frame instance lists, grouped by mesh.
#[derive(Default)]
pub struct Batches {
    pub lists: HashMap<MeshId, Vec<Instance>>,
    /// Lift meshes, whose lights the original adds at an eighth of their
    /// color (0x49daa0).
    pub lift_meshes: std::collections::HashSet<MeshId>,
    /// Line segments (pairs of points) such as lift cables.
    pub lines: Vec<LineVertex>,
}

impl Batches {
    pub fn clear(&mut self) {
        for v in self.lists.values_mut() {
            v.clear();
        }
        self.lines.clear();
    }

    /// Adds a polyline in one color.
    pub fn line_strip(&mut self, pts: &[Vec3], color: [f32; 4]) {
        for w in pts.windows(2) {
            self.lines.push(LineVertex { pos: w[0].into(), color });
            self.lines.push(LineVertex { pos: w[1].into(), color });
        }
    }
    pub fn push(&mut self, mesh: MeshId, model: Mat4, tint: Vec4) {
        self.lists.entry(mesh).or_default().push(Instance::new(model, tint));
    }

    /// Pushes something already lit, cut from one of the original's sprites.
    pub fn push_prelit(&mut self, mesh: MeshId, model: Mat4, tint: Vec4) {
        self.lists.entry(mesh).or_default().push(Instance::lit(model, tint, super::mesh::Lighting::Prelit));
    }

    /// Pushes something the original draws from a pre-rendered sprite.
    pub fn push_sprite(&mut self, mesh: MeshId, model: Mat4, tint: Vec4) {
        self.lists.entry(mesh).or_default().push(Instance::lit(model, tint, super::mesh::Lighting::Sprite));
    }
}

pub struct SceneRenderer {
    pub meshes: MeshPipeline,
    lines: LinePipeline,
    line_buf: wgpu::Buffer,
    line_cap: usize,
    pub terrain: TerrainRenderer,
    globals_buf: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    instance_buf: wgpu::Buffer,
    instance_cap: usize,
    pub clear: wgpu::Color,
}

impl SceneRenderer {
    pub fn new(gpu: &Gpu, assets: &Assets, format: wgpu::TextureFormat, samples: u32) -> SceneRenderer {
        let meshes = MeshPipeline::new(gpu, format, samples);
        let terrain = TerrainRenderer::new(gpu, assets, format, samples);
        let globals_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bg = meshes.globals_bind_group(gpu, &globals_buf);
        let lines = LinePipeline::new(gpu, &meshes.globals_layout, format, samples);
        let line_cap = 4096;
        let line_buf = Self::make_lines(gpu, line_cap);
        let instance_cap = 4096;
        let instance_buf = Self::make_instances(gpu, instance_cap);
        SceneRenderer {
            meshes,
            lines,
            line_buf,
            line_cap,
            terrain,
            globals_buf,
            globals_bg,
            instance_buf,
            instance_cap,
            clear: wgpu::Color {
                r: 0.62,
                g: 0.74,
                b: 0.88,
                a: 1.0,
            },
        }
    }

    fn make_lines(gpu: &Gpu, cap: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lines"),
            size: (cap * std::mem::size_of::<LineVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn make_instances(gpu: &Gpu, cap: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (cap * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Records a full scene render into `target`.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        gpu: &Gpu,
        enc: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        depth: &wgpu::TextureView,
        globals: &Globals,
        cache: &MeshCache,
        batches: &Batches,
    ) {
        gpu.queue
            .write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(globals));
        let total: usize = batches.lists.values().map(|v| v.len()).sum();
        if total > self.instance_cap {
            self.instance_cap = total.next_power_of_two();
            self.instance_buf = Self::make_instances(gpu, self.instance_cap);
        }
        let mut all = Vec::with_capacity(total);
        let mut ranges = Vec::new();
        let mut keys: Vec<_> = batches.lists.keys().copied().collect();
        keys.sort_unstable();
        for id in keys {
            let list = &batches.lists[&id];
            if list.is_empty() || cache.meshes.get(id).and_then(|m| m.as_ref()).is_none() {
                continue;
            }
            let start = all.len() as u32;
            all.extend_from_slice(list);
            ranges.push((id, start..all.len() as u32));
        }
        if !all.is_empty() {
            gpu.queue
                .write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(&all));
        }
        if batches.lines.len() > self.line_cap {
            self.line_cap = batches.lines.len().next_power_of_two();
            self.line_buf = Self::make_lines(gpu, self.line_cap);
        }
        if !batches.lines.is_empty() {
            gpu.queue.write_buffer(&self.line_buf, 0, bytemuck::cast_slice(&batches.lines));
        }
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.terrain.draw(&mut pass, &self.globals_bg);
        pass.set_pipeline(&self.meshes.pipeline);
        pass.set_bind_group(0, &self.globals_bg, &[]);
        for (id, range) in ranges {
            if let Some(m) = &cache.meshes[id] {
                self.meshes.draw(&mut pass, m, &self.instance_buf, range);
            }
        }
        if !batches.lines.is_empty() {
            pass.set_pipeline(&self.lines.pipeline);
            pass.set_bind_group(0, &self.globals_bg, &[]);
            pass.set_vertex_buffer(0, self.line_buf.slice(..));
            pass.draw(0..batches.lines.len() as u32, 0..1);
        }
    }
}

/// Cuts the sign's letters out of a sprite frame: bright green pixels and
/// their dark outlines (within two pixels), not the red roof behind them,
/// with enclosed gaps filled.
/// A one-texture quad with corners top-left, top-right, bottom-left,
/// bottom-right.
/// The entrance's sign, in the entrance model's space.
pub fn entrance_sign_mesh(gpu: &Gpu, pipe: &MeshPipeline, assets: &Assets) -> Option<GpuMesh> {
    assets
        .resolve("generatedsprites/BUILDINGS_Entrance_Entrance.spr")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| crate::formats::sprite::Sprite::parse(b).ok())
        .and_then(|s| s.frame(0).ok())
        .and_then(|f| sign_texture(&f, (200, 116, 520, 201)))
        .map(|tex| {
            let m = quad([
                Vec3::new(45.842, 68.470, 30.314),
                Vec3::new(12.297, 34.924, 30.314),
                Vec3::new(45.842, 68.470, 15.903),
                Vec3::new(12.297, 34.924, 15.903),
            ]);
            let views = MeshPipeline::texture_views(gpu, &[Some(tex)]);
            pipe.upload(gpu, &[(&m, &m.frames[0], &views)])
        })
}

fn quad([tl, tr, bl, br]: [Vec3; 4]) -> Mesh {
    use glam::Vec2;
    let tri = |verts: [u16; 3]| crate::formats::mesh::Triangle {
        verts,
        uvs: verts.map(u32::from),
        texture: 0,
    };
    Mesh {
        frames: vec![vec![tl, bl, tr, br]],
        triangles: vec![tri([0, 1, 2]), tri([2, 1, 3])],
        uvs: vec![Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0)],
        textures: crate::formats::mesh::TextureSource::Files(Vec::new()),
        bounds_min: tl.min(tr).min(bl).min(br),
        bounds_max: tl.max(tr).max(bl).max(br),
        attach: Vec::new(),
        clips: Vec::new(),
        parts: Vec::new(),
        lights: Vec::new(),
        tracks: Vec::new(),
        skin: None,
    }
}

fn sign_texture(f: &Rgba, (x0, y0, x1, y1): (u32, u32, u32, u32)) -> Option<Rgba> {
    if x1 > f.width || y1 > f.height {
        return None;
    }
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let px = |x: usize, y: usize| {
        let o = (((y0 as usize + y) * f.width as usize) + x0 as usize + x) * 4;
        let p = &f.pixels[o..o + 4];
        (p[0] as i32, p[1] as i32, p[2] as i32, p[3])
    };
    let red = |x, y| {
        let (r, g, b, _) = px(x, y);
        r > g + 15 && r > b
    };
    let mut near = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let (r, g, b, a) = px(x, y);
            if a > 0 && g > r + 25 && g > b + 25 && g > 140 {
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        let (xx, yy) = (x as i32 + dx, y as i32 + dy);
                        if dx.abs() + dy.abs() <= 2 && xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < h {
                            near[yy as usize * w + xx as usize] = true;
                        }
                    }
                }
            }
        }
    }
    let usable = |x: usize, y: usize| px(x, y).3 > 0 && !red(x, y);
    let mut keep: Vec<bool> = (0..w * h).map(|i| near[i] && usable(i % w, i / w)).collect();
    // Fill gaps the letters enclose: whatever the border cannot reach.
    let mut outside = vec![false; w * h];
    let mut stack: Vec<usize> = (0..w).flat_map(|x| [x, (h - 1) * w + x]).chain((0..h).flat_map(|y| [y * w, y * w + w - 1])).collect();
    while let Some(i) = stack.pop() {
        if outside[i] || keep[i] {
            continue;
        }
        outside[i] = true;
        let (x, y) = (i % w, i / w);
        if x > 0 { stack.push(i - 1); }
        if x + 1 < w { stack.push(i + 1); }
        if y > 0 { stack.push(i - w); }
        if y + 1 < h { stack.push(i + w); }
    }
    for i in 0..w * h {
        if !outside[i] && usable(i % w, i / w) {
            keep[i] = true;
        }
    }
    let mut pixels = vec![0u8; w * h * 4];
    for i in 0..w * h {
        let (r, g, b, _) = px(i % w, i / w);
        pixels[i * 4..i * 4 + 4].copy_from_slice(&[r as u8, g as u8, b as u8, if keep[i] { 255 } else { 0 }]);
    }
    Some(Rgba {
        width: w as u32,
        height: h as u32,
        pixels,
        name: "entrance sign".into(),
    })
}
