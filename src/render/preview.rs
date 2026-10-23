//! Small offscreen renders of models for the build pickers, like the
//! original's rotating preview boxes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use super::mesh::{GpuMesh, Instance, MeshPipeline};
use super::{Globals, Gpu, look_at, perspective};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub struct Previewer {
    pipe: MeshPipeline,
    meshes: HashMap<PathBuf, Option<GpuMesh>>,
    sign: Option<Option<GpuMesh>>,
}

impl Previewer {
    pub fn new(gpu: &Gpu) -> Previewer {
        Previewer {
            pipe: MeshPipeline::new(gpu, FORMAT, 1),
            meshes: HashMap::new(),
            sign: None,
        }
    }

    /// Renders a model seen from one of four sides (`view` 0..4) on a
    /// transparent background. Returns RGBA pixels.
    /// With `sign`, the entrance's sign is drawn on the model too.
    pub fn render(&mut self, gpu: &Gpu, assets: &crate::assets::Assets, path: &Path, sign: bool, view: u8, w: u32, h: u32) -> Result<Vec<u8>> {
        if sign && self.sign.is_none() {
            self.sign = Some(super::scene::entrance_sign_mesh(gpu, &self.pipe, assets));
        }
        let mesh = self
            .meshes
            .entry(path.to_path_buf())
            .or_insert_with(|| self.pipe.load(gpu, path).map_err(|e| log::warn!("preview {}: {e:#}", path.display())).ok());
        let Some(mesh) = mesh.as_ref() else { anyhow::bail!("cannot load {}", path.display()) };
        let center = (mesh.bounds_min + mesh.bounds_max) * 0.5;
        let radius = (mesh.bounds_max - mesh.bounds_min).length().max(0.01) * 0.5;
        let (eye, center, proj) = if view & 0x80 != 0 {
            // As the game's overview shows it (`ui::picker::world_view`):
            // orthographic at its pitch, a pixel being what one is at
            // 640 wide and the level's view width, centered on the
            // model's middle.
            let width = if view & 0x40 != 0 { 300.0 } else { 90.0 };
            let yaw = (view & 0x3f) as f32 / 64.0 * std::f32::consts::TAU;
            let pitch = crate::camera::OVERVIEW_PITCH;
            let at = center;
            let dir = Vec3::new(pitch.cos() * yaw.cos(), pitch.cos() * yaw.sin(), pitch.sin());
            let (hw, hh) = (w as f32 * 0.5 * width / 640.0, h as f32 * 0.5 * width / 640.0);
            let d = radius * 4.0 + 100.0;
            let proj = glam::camera::rh::proj::directx::orthographic(-hw, hw, -hh, hh, 0.0, d * 2.0);
            (at + dir * d, at, proj)
        } else {
            let ang = std::f32::consts::FRAC_PI_4 + view as f32 * std::f32::consts::FRAC_PI_2;
            let eye = center + Vec3::new(ang.cos(), ang.sin(), 0.7).normalize() * radius * 2.0;
            (eye, center, perspective(0.8, w as f32 / h as f32, radius * 0.05, radius * 10.0))
        };
        let view_m = look_at(eye, center, Vec3::Z);
        let mut globals = Globals::new(proj * view_m, eye);
        globals.params = [1e6, 2e6, 12.0, 0.0];
        let gbuf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("preview globals"),
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let gbg = self.pipe.globals_bind_group(gpu, &gbuf);
        let inst = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("preview instance"),
            contents: bytemuck::bytes_of(&Instance::new(Mat4::IDENTITY, Vec4::ONE)),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let depth = gpu.depth_view(w, h, 1);
        let pipe = &self.pipe;
        let sign = if sign { self.sign.as_ref().and_then(|s| s.as_ref()) } else { None };
        gpu.capture(w, h, FORMAT, |enc, view| {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("preview"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
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
            pass.set_pipeline(&pipe.pipeline);
            pass.set_bind_group(0, &gbg, &[]);
            pipe.draw(&mut pass, mesh, &inst, 0..1);
            if let Some(sign) = sign {
                pipe.draw(&mut pass, sign, &inst, 0..1);
            }
        })
    }
}
