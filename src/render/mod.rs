//! wgpu renderer: textured meshes with instancing, plus offscreen capture.

pub mod lines;
pub mod mesh;
pub mod preview;
pub mod scene;
pub mod terrain;

use anyhow::{Context, Result};
use glam::Mat4;

/// Right-handed view matrix (wgpu clip conventions).
pub fn look_at(eye: glam::Vec3, target: glam::Vec3, up: glam::Vec3) -> Mat4 {
    glam::camera::rh::view::look_at_mat4(eye, target, up)
}

/// Right-handed perspective projection with a [0, 1] depth range.
pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    glam::camera::rh::proj::directx::perspective(fov_y, aspect, near, far)
}

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Gpu {
    #[allow(dead_code)]
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    pub fn new(
        instance: wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Gpu> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        }))
        .context("no suitable GPU adapter")?;
        log::info!("adapter: {:?}", adapter.get_info());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("srt device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))?;
        Ok(Gpu {
            instance,
            adapter,
            device,
            queue,
        })
    }

    pub fn headless() -> Result<Gpu> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        Gpu::new(instance, None)
    }

    pub fn depth_view(&self, width: u32, height: u32, samples: u32) -> wgpu::TextureView {
        self.device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    /// Uploads an RGBA8 image with a full mip chain generated on the CPU.
    pub fn texture_rgba(&self, img: &crate::formats::texture::Rgba) -> wgpu::TextureView {
        let mips = 32 - img.width.max(img.height).leading_zeros();
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&img.name),
            size: wgpu::Extent3d {
                width: img.width,
                height: img.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let (mut w, mut h) = (img.width, img.height);
        let mut level = img.pixels.clone();
        for mip in 0..mips {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
                    mip_level: mip,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &level,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * w),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            if mip + 1 < mips {
                let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
                level = downsample(&level, w, h, nw, nh);
                w = nw;
                h = nh;
            }
        }
        tex.create_view(&Default::default())
    }

    /// Renders with `draw` into an offscreen target and returns RGBA pixels.
    pub fn capture(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        draw: impl FnOnce(&mut wgpu::CommandEncoder, &wgpu::TextureView),
    ) -> Result<Vec<u8>> {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        let row = (4 * width).div_ceil(256) * 256;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture readback"),
            size: (row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        draw(&mut enc, &view);
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        let data = slice.get_mapped_range()?;
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            let s = (y * row) as usize;
            out.extend_from_slice(&data[s..s + (width * 4) as usize]);
        }
        drop(data);
        buf.unmap();
        if matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            for px in out.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
        }
        Ok(out)
    }
}

fn downsample(src: &[u8], w: u32, h: u32, nw: u32, nh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        for x in 0..nw {
            let mut acc = [0u32; 4];
            let mut wsum = 0u32;
            for dy in 0..2 {
                for dx in 0..2 {
                    let sx = (x * 2 + dx).min(w - 1);
                    let sy = (y * 2 + dy).min(h - 1);
                    let i = ((sy * w + sx) * 4) as usize;
                    // Weight color by alpha so cut-out edges do not bleed
                    // black into the lower mips.
                    let a = src[i + 3] as u32;
                    for c in 0..3 {
                        acc[c] += src[i + c] as u32 * a;
                    }
                    acc[3] += a;
                    wsum += a;
                }
            }
            let o = ((y * nw + x) * 4) as usize;
            for c in 0..3 {
                out[o + c] = if wsum > 0 { (acc[c] / wsum) as u8 } else { 0 };
            }
            out[o + 3] = (acc[3] / 4) as u8;
        }
    }
    out
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub view_proj: [[f32; 4]; 4],
    pub sun_dir: [f32; 4],
    pub sun_color: [f32; 4],
    pub ambient: [f32; 4],
    pub fog_color: [f32; 4],
    /// x: fog start, y: fog end, z: time, w: grid spacing (0 = off)
    pub params: [f32; 4],
    pub camera_pos: [f32; 4],
    /// The sprite rig (see `mesh::Lighting`): direction to its sun, its
    /// color and the ambient.
    pub sprite_sun_dir: [f32; 4],
    pub sprite_sun_color: [f32; 4],
    pub sprite_ambient: [f32; 4],
    /// x: number of lamps in use; y: 1 for low scenery detail.
    pub lamp_count: [f32; 4],
    pub lamps: [Lamp; MAX_LAMPS],
}

/// A night light from a model: position and outer radius, color and
/// inner radius, and for a spot its direction and its outer and inner
/// cones' half angles (`spot.w` 0 for a point light).
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Lamp {
    pub pos_outer: [f32; 4],
    pub color_inner: [f32; 4],
    pub spot: [f32; 4],
    pub cones: [f32; 4],
}

pub const MAX_LAMPS: usize = 64;

/// The globals block and lamp lighting, shared by the WGSL shaders.
pub const GLOBALS_WGSL: &str = r#"
struct Lamp {
    pos_outer: vec4<f32>,
    color_inner: vec4<f32>,
    spot: vec4<f32>,
    cones: vec4<f32>,
};
struct Globals {
    view_proj: mat4x4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
    fog_color: vec4<f32>,
    params: vec4<f32>,
    camera_pos: vec4<f32>,
    sprite_sun_dir: vec4<f32>,
    sprite_sun_color: vec4<f32>,
    sprite_ambient: vec4<f32>,
    lamp_count: vec4<f32>,
    lamps: array<Lamp, 64>,
};
@group(0) @binding(0) var<uniform> g: Globals;

// The original lights and fogs 8-bit sRGB colors as they are, so the
// shaders do their sums in sRGB too and only decode for output.
fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

// The original's point and spot lights: full within the inner radius,
// fading to nothing at the outer one by squared distance (0x51a5c0).
fn lamp_light(p: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let count = u32(g.lamp_count.x);
    for (var i = 0u; i < count; i++) {
        let l = g.lamps[i];
        let d = l.pos_outer.xyz - p;
        let d2 = dot(d, d);
        let outer2 = l.pos_outer.w * l.pos_outer.w;
        if (d2 >= outer2) {
            continue;
        }
        let inner2 = l.color_inner.w * l.color_inner.w;
        let fall = clamp((outer2 - d2) / max(outer2 - inner2, 1e-4), 0.0, 1.0);
        let to = normalize(d);
        let facing = max(dot(n, to), 0.0);
        var cone = 1.0;
        if (l.spot.w > 0.5) {
            // Full inside the inner cone, fading by angle to none at the
            // outer one (0x51ac20).
            let a = acos(clamp(dot(-to, l.spot.xyz), -1.0, 1.0));
            cone = clamp((l.cones.x - a) / max(l.cones.x - l.cones.y, 1e-4), 0.0, 1.0);
        }
        sum += l.color_inner.rgb * fall * facing * cone;
    }
    return sum;
}
"#;

impl Globals {
    pub fn new(view_proj: Mat4, camera_pos: glam::Vec3) -> Globals {
        Globals {
            view_proj: view_proj.to_cols_array_2d(),
            sun_dir: glam::Vec3::new(-0.4, -0.5, 0.75).normalize().extend(0.0).into(),
            sun_color: [1.0, 0.97, 0.9, 1.0],
            ambient: [0.45, 0.5, 0.58, 1.0],
            fog_color: [0.72, 0.8, 0.9, 1.0],
            params: [800.0, 4000.0, 0.0, 0.0],
            camera_pos: camera_pos.extend(1.0).into(),
            sprite_sun_dir: [0.0, 0.0, 1.0, 0.0],
            sprite_sun_color: [1.0; 4],
            sprite_ambient: [0.4; 4],
            lamp_count: [0.0; 4],
            lamps: [Lamp::default(); MAX_LAMPS],
        }
    }
}
