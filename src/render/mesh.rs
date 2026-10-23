//! Instanced textured-mesh pipeline. Meshes are flattened into per-texture
//! parts with face normals; instances carry a model matrix and a tint.

use std::path::Path;

use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use super::{DEPTH_FORMAT, Globals, Gpu};
use crate::formats::{mesh::Mesh, texture::Rgba};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub model: [[f32; 4]; 4],
    pub tint: [f32; 4],
    /// x: 0 scene, 1 sprite rig, 2 pre-lit (see `Lighting`).
    pub light: [f32; 4],
}

/// Which lights shade an instance. The original draws buildings, lift
/// stations, objects and trees from pre-rendered sprites, which darken
/// over dusk on their own schedule (0x42d920), and live 3D only for what
/// moves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lighting {
    Scene,
    Sprite,
    /// Cut from one of the original's sprites, so already lit: drawn as it
    /// is, dimmed as the sprites are, and at night under the scene lights.
    Prelit,
}

impl Instance {
    pub fn new(model: Mat4, tint: Vec4) -> Instance {
        Instance::lit(model, tint, Lighting::Scene)
    }

    pub fn lit(model: Mat4, tint: Vec4, lighting: Lighting) -> Instance {
        Instance {
            model: model.to_cols_array_2d(),
            tint: tint.into(),
            light: [
                match lighting {
                    Lighting::Scene => 0.0,
                    Lighting::Sprite => 1.0,
                    Lighting::Prelit => 2.0,
                },
                0.0,
                0.0,
                0.0,
            ],
        }
    }
}

pub struct GpuPart {
    pub bind_group: wgpu::BindGroup,
    pub vbuf: wgpu::Buffer,
    pub count: u32,
}

pub struct GpuMesh {
    pub parts: Vec<GpuPart>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    /// Its distinct vertex positions, for its outline on screen.
    pub points: std::sync::Arc<[Vec3]>,
    pub attach: Vec<(crate::formats::mesh::AttachPoint, Vec3)>,
    /// Number of vertex-animation frames in the source mesh.
    pub frames: usize,
}

pub struct MeshPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub globals_layout: wgpu::BindGroupLayout,
    pub texture_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub white: wgpu::TextureView,
}

const SHADER: &str = r#"
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) m0: vec4<f32>,
    @location(4) m1: vec4<f32>,
    @location(5) m2: vec4<f32>,
    @location(6) m3: vec4<f32>,
    @location(7) tint: vec4<f32>,
    @location(8) light: vec4<f32>,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) world: vec3<f32>,
    @location(4) sprite: f32,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let world = model * vec4<f32>(v.pos, 1.0);
    var o: VsOut;
    o.clip = g.view_proj * world;
    o.normal = normalize((model * vec4<f32>(v.normal, 0.0)).xyz);
    o.uv = v.uv;
    o.tint = v.tint;
    o.world = world.xyz;
    o.sprite = v.light.x;
    return o;
}

// Soft shoulder so bright snow keeps some detail instead of clipping.
fn tone(x: vec3<f32>) -> vec3<f32> {
    let k = 0.85;
    return select(x, k + (1.0 - k) * (1.0 - exp(-(x - k) / (1.0 - k))), x > vec3<f32>(k));
}

@fragment
fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, i.uv) * i.tint;
    if (c.a < 0.45) { discard; }
    let n = normalize(i.normal);
    // Two-sided lighting: the original models are single-sided billboards
    // in places (trees), so light both faces.
    var light: vec3<f32>;
    if (i.sprite > 1.5) {
        // Pre-lit: the sprite brightness, or the scene lights at night.
        if (g.sprite_sun_dir.w > 0.0) {
            light = vec3<f32>(g.sprite_sun_dir.w);
        } else {
            light = g.ambient.rgb + g.sun_color.rgb * abs(dot(n, g.sun_dir.xyz));
        }
    } else if (i.sprite > 0.5) {
        light = g.sprite_ambient.rgb + g.sprite_sun_color.rgb * abs(dot(n, g.sprite_sun_dir.xyz));
    } else {
        light = g.ambient.rgb + g.sun_color.rgb * abs(dot(n, g.sun_dir.xyz));
    }
    let lit = min(to_srgb(c.rgb) * (light + lamp_light(i.world, n)), vec3<f32>(1.0));
    let dist = distance(i.world, g.camera_pos.xyz);
    let f = clamp((dist - g.params.x) / (g.params.y - g.params.x), 0.0, 1.0);
    return vec4<f32>(to_linear(mix(lit, to_srgb(g.fog_color.rgb), f)), 1.0);
}
"#;

impl MeshPipeline {
    pub fn new(gpu: &Gpu, color_format: wgpu::TextureFormat, samples: u32) -> MeshPipeline {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh shader"),
            source: wgpu::ShaderSource::Wgsl(format!("{}{}", super::GLOBALS_WGSL, SHADER).into()),
        });
        let globals_layout = globals_layout(device);
        let texture_layout = texture_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh layout"),
            bind_group_layouts: &[Some(&globals_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let vattrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
        let iattrs = wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &vattrs,
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &iattrs,
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                mask: !0,
                alpha_to_coverage_enabled: samples > 1,
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mesh sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let white = gpu.texture_rgba(&Rgba::solid([255, 255, 255, 255]));
        MeshPipeline {
            pipeline,
            globals_layout,
            texture_layout,
            sampler,
            white,
        }
    }

    pub fn texture_bind_group(&self, gpu: &Gpu, view: &wgpu::TextureView) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    pub fn globals_bind_group(&self, gpu: &Gpu, buf: &wgpu::Buffer) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        })
    }

    /// Uploads a mesh's textures, indexed by triangle slot.
    pub fn texture_views(gpu: &Gpu, textures: &[Option<Rgba>]) -> Vec<Option<wgpu::TextureView>> {
        textures.iter().map(|t| t.as_ref().map(|img| gpu.texture_rgba(img))).collect()
    }

    /// Converts posed meshes into one GPU mesh: each piece is a parsed mesh,
    /// its vertex positions (an animation frame or a skinned pose, in model
    /// space) and its texture views. The first piece supplies the attach
    /// points and frame count.
    pub fn upload(&self, gpu: &Gpu, pieces: &[(&Mesh, &[Vec3], &[Option<wgpu::TextureView>])]) -> GpuMesh {
        let mut parts = Vec::new();
        let mut bounds_min = Vec3::splat(f32::MAX);
        let mut bounds_max = Vec3::splat(f32::MIN);
        let mut points = std::collections::HashSet::new();
        for &(mesh, pos, views) in pieces {
            let slots = views.len();
            let mut buckets: Vec<Vec<Vertex>> = vec![Vec::new(); slots + 1];
            for t in &mesh.triangles {
                let [a, b, c] = t.verts.map(|i| pos.get(i as usize).copied().unwrap_or(Vec3::ZERO));
                let n = (b - a).cross(c - a).normalize_or_zero();
                let slot = if t.texture >= 0 && (t.texture as usize) < slots {
                    t.texture as usize
                } else {
                    slots
                };
                for (k, p) in [a, b, c].into_iter().enumerate() {
                    bounds_min = bounds_min.min(p);
                    bounds_max = bounds_max.max(p);
                    points.insert(p.to_array().map(f32::to_bits));
                    let uv = mesh.uvs.get(t.uvs[k] as usize).copied().unwrap_or_default();
                    buckets[slot].push(Vertex {
                        pos: p.into(),
                        normal: n.into(),
                        uv: [uv.x, uv.y],
                    });
                }
            }
            for (slot, verts) in buckets.into_iter().enumerate() {
                if verts.is_empty() {
                    continue;
                }
                let view = views.get(slot).and_then(|v| v.as_ref());
                let bind_group = self.texture_bind_group(gpu, view.unwrap_or(&self.white));
                let vbuf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh vb"),
                    contents: bytemuck::cast_slice(&verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                parts.push(GpuPart {
                    bind_group,
                    vbuf,
                    count: verts.len() as u32,
                });
            }
        }
        if parts.is_empty() {
            bounds_min = Vec3::ZERO;
            bounds_max = Vec3::ZERO;
        }
        let first = pieces.first().map(|p| p.0);
        GpuMesh {
            parts,
            bounds_min,
            bounds_max,
            points: points.into_iter().map(|p| Vec3::from_array(p.map(f32::from_bits))).collect(),
            attach: first.map_or(Vec::new(), |m| m.attach.clone()),
            frames: first.map_or(1, |m| m.frames.len()),
        }
    }

    pub fn load(&self, gpu: &Gpu, path: &Path) -> anyhow::Result<GpuMesh> {
        let mesh = Mesh::load(path)?;
        let views = Self::texture_views(gpu, &mesh.load_textures(path));
        Ok(self.upload(gpu, &[(&mesh, &mesh.frames[0], &views)]))
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        mesh: &'a GpuMesh,
        instances: &'a wgpu::Buffer,
        range: std::ops::Range<u32>,
    ) {
        pass.set_vertex_buffer(1, instances.slice(..));
        for part in &mesh.parts {
            pass.set_bind_group(1, &part.bind_group, &[]);
            pass.set_vertex_buffer(0, part.vbuf.slice(..));
            pass.draw(0..part.count, range.clone());
        }
    }
}

pub fn globals_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("globals layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Globals>() as u64),
            },
            count: None,
        }],
    })
}

pub fn texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("texture layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}
