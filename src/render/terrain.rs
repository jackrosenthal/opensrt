//! Terrain: one textured quad per level cell, tiles drawn from a texture
//! array built out of the original `data/textures` snow tiles.

use glam::Vec3;
use wgpu::util::DeviceExt;

use super::{DEPTH_FORMAT, Gpu, mesh::globals_layout};
use crate::assets::Assets;
use crate::formats::level::{Level, cell_flags};
use crate::formats::texture;

const TILE: u32 = 128;

/// Texture array layers, in order.
const LAYERS: &[&str] = &[
    "data/textures/PL_snow1.bin",
    "data/textures/PL_snow2.bin",
    "data/textures/PL_snow3.bin",
    "data/textures/RK_snow1.bin",
    "data/textures/RK_snow2.bin",
    "data/textures/RK_snow3.bin",
    "data/textures/RK_snow4.bin",
    "data/textures/RK_snow5.bin",
    "data/textures/RK_snow6.bin",
    "data/textures/RK_snow7.bin",
    "data/textures/TR_snow_single.bin",
    "data/textures/mogula.bin",
    "data/textures/mogulb.bin",
    "data/textures/mogulc.bin",
    "data/textures/moguld.bin",
    "data/textures/worn1a.bin",
    "data/textures/worn2a.bin",
    "data/textures/worn3a.bin",
    "data/textures/worn4a.bin",
    "data/textures/world_edge.bin",
    "data/textures/new_paths/rock1_path.bin",
    "data/textures/new_paths/rock2_path.bin",
    "data/textures/new_paths/brick1_path.bin",
    "data/textures/new_paths/brick2_path.bin",
    "data/textures/new_paths/rock1_path_s.bin",
    "data/textures/new_paths/rock2_path_s.bin",
    "data/textures/new_paths/brick1_path_s.bin",
    "data/textures/new_paths/brick2_path_s.bin",
    "data/textures/litter.bin",
    "data/textures/yellow_snow.bin",
    // The shapes a footpath takes in a cell, by its neighbors (see
    // PATH_MASKS); only their alpha is used.
    "data/textures/paths/cross0.bin",
    "data/textures/paths/cross1.bin",
    "data/textures/paths/cross2.bin",
    "data/textures/paths/cross3.bin",
    "data/textures/paths/cross4.bin",
    "data/textures/paths/cross5.bin",
    "data/textures/paths/cross6.bin",
    "data/textures/paths/cross7.bin",
    "data/textures/paths/cross8.bin",
    "data/textures/paths/cross9.bin",
    "data/textures/paths/crossa.bin",
    "data/textures/paths/crossb.bin",
    "data/textures/paths/crossc.bin",
    "data/textures/paths/crossd.bin",
    "data/textures/paths/crosse.bin",
    "data/textures/paths/crossf.bin",
    "data/textures/paths/dot0.bin",
    "data/textures/paths/down-left0.bin",
    "data/textures/paths/down-left8.bin",
    "data/textures/paths/down-right0.bin",
    "data/textures/paths/down-right4.bin",
    "data/textures/paths/down0.bin",
    "data/textures/paths/horiz0.bin",
    "data/textures/paths/left0.bin",
    "data/textures/paths/right0.bin",
    "data/textures/paths/t-down0.bin",
    "data/textures/paths/t-down4.bin",
    "data/textures/paths/t-down8.bin",
    "data/textures/paths/t-downc.bin",
    "data/textures/paths/t-left0.bin",
    "data/textures/paths/t-left1.bin",
    "data/textures/paths/t-left8.bin",
    "data/textures/paths/t-left9.bin",
    "data/textures/paths/t-right0.bin",
    "data/textures/paths/t-right2.bin",
    "data/textures/paths/t-right4.bin",
    "data/textures/paths/t-right6.bin",
    "data/textures/paths/t-up0.bin",
    "data/textures/paths/t-up1.bin",
    "data/textures/paths/t-up2.bin",
    "data/textures/paths/t-up3.bin",
    "data/textures/paths/up-left0.bin",
    "data/textures/paths/up-left1.bin",
    "data/textures/paths/up-right0.bin",
    "data/textures/paths/up-right2.bin",
    "data/textures/paths/up0.bin",
    "data/textures/paths/vert0.bin",
];
const L_PLAIN: u32 = 0;
const L_ROCK: u32 = 3;
const L_FOREST: u32 = 10;
const L_MOGUL: u32 = 11;
const L_WORN: u32 = 15;
const L_EDGE: u32 = 19;
/// Footpaths, then their snowy versions for deep snow (0x40bce0).
const L_PATH: u32 = 20;
const L_PATH_SNOW: u32 = 24;
/// The first footpath shape layer.
const L_PATH_MASK: u32 = 30;
/// Which footpath shape a cell takes, by which of its eight neighbors
/// have the same footpath (0x40c380, 0x40a540 and its table at
/// 0x5ddf80): bits 0 to 3 the corners (-x+y, +x+y, +x-y, -x-y), 4 +y,
/// 5 +x, 6 -y and 7 -x. Values index the shape layers from L_PATH_MASK.
const PATH_MASKS: [u8; 256] = [
    16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    43, 43, 44, 44, 43, 43, 44, 44, 43, 43, 44, 44, 43, 43, 44, 44,
    21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21,
    46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46,
    19, 19, 19, 19, 20, 20, 20, 20, 19, 19, 19, 19, 20, 20, 20, 20,
    33, 33, 34, 34, 35, 35, 36, 36, 33, 33, 34, 34, 35, 35, 36, 36,
    23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
    41, 42, 41, 42, 41, 42, 41, 42, 41, 42, 41, 42, 41, 42, 41, 42,
    22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22,
    37, 38, 39, 40, 37, 38, 39, 40, 37, 38, 39, 40, 37, 38, 39, 40,
    17, 17, 17, 17, 17, 17, 17, 17, 18, 18, 18, 18, 18, 18, 18, 18,
    29, 30, 29, 30, 29, 30, 29, 30, 31, 32, 31, 32, 31, 32, 31, 32,
    25, 25, 25, 25, 26, 26, 26, 26, 27, 27, 27, 27, 28, 28, 28, 28,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];
/// Dirt drawn over a cell; the shader names these layers by number.
const L_LITTER: u32 = 28;
const L_YELLOW: u32 = 29;
const _: () = assert!(L_YELLOW == L_LITTER + 1 && L_PATH_MASK == L_YELLOW + 1 && LAYERS.len() == L_PATH_MASK as usize + 47 && LAYERS.len() % 6 != 0);
const NO_OVERLAY: u32 = 255;

/// Slope (degrees) above which bare rock tiles are used.
pub const ROCK_SLOPE: f32 = 42.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TVertex {
    pos: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    layers: [u32; 2],
    /// Cell coordinates for the placement highlight (u32::MAX: none).
    cell: [u32; 2],
    /// The cell's footpath layer (NO_OVERLAY: none) and its shape layer.
    path: [u32; 2],
}

/// Placement highlight of a terrain cell, as the original shows while
/// something is attached to the pointer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CellMark {
    None = 0,
    Ok = 1,
    Blocked = 2,
}

const SHADER: &str = r#"
@group(1) @binding(0) var tiles: texture_2d_array<f32>;
@group(1) @binding(1) var samp: sampler;
@group(1) @binding(2) var marks: texture_2d<u32>;
@group(1) @binding(3) var ground_lamps: texture_2d<f32>;

// Lamp light worked out at the height samples (see game::lamplight),
// blended across the cell as the original's vertex colors are.
fn ground_lamp(cell: vec2<u32>, uv: vec2<f32>) -> vec3<f32> {
    let c = vec2<i32>(cell);
    let f = clamp(vec2<f32>(uv.x, 1.0 - uv.y), vec2<f32>(0.0), vec2<f32>(1.0));
    let a = textureLoad(ground_lamps, c, 0).rgb;
    let b = textureLoad(ground_lamps, c + vec2<i32>(1, 0), 0).rgb;
    let d = textureLoad(ground_lamps, c + vec2<i32>(0, 1), 0).rgb;
    let e = textureLoad(ground_lamps, c + vec2<i32>(1, 1), 0).rgb;
    return mix(mix(a, b, f.x), mix(d, e, f.x), f.y);
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) layers: vec2<u32>,
    @location(3) world: vec3<f32>,
    @location(4) @interpolate(flat) cell: vec2<u32>,
    @location(5) @interpolate(flat) path: vec2<u32>,
};

@vertex
fn vs_main(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>,
           @location(2) uv: vec2<f32>, @location(3) layers: vec2<u32>,
           @location(4) cell: vec2<u32>, @location(5) path: vec2<u32>) -> VsOut {
    var o: VsOut;
    o.clip = g.view_proj * vec4<f32>(pos, 1.0);
    o.normal = normal;
    o.uv = uv;
    o.layers = layers;
    o.world = pos;
    o.cell = cell;
    o.path = path;
    return o;
}

// Soft shoulder so bright snow keeps some detail instead of clipping.
fn tone(x: vec3<f32>) -> vec3<f32> {
    let k = 0.85;
    return select(x, k + (1.0 - k) * (1.0 - exp(-(x - k) / (1.0 - k))), x > vec3<f32>(k));
}

@fragment
fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
    var c = textureSample(tiles, samp, i.uv, i.layers.x).rgb;
    // Low scenery detail (lamp_count.y): no textures, only the lit color.
    let plain = g.lamp_count.y > 0.0;
    if (i.layers.y != 255u) {
        let o = textureSample(tiles, samp, i.uv, i.layers.y).rgb;
        c = mix(c, o, 0.6);
    }
    // Footpaths: the path texture through the cell's shape (its alpha),
    // as the original makes each cell's path texture (0x40a540). The
    // shapes run +y down their rows.
    if (i.path.x != 255u) {
        let p = textureSample(tiles, samp, i.uv, i.path.x).rgb;
        let shape = textureSample(tiles, samp, vec2<f32>(i.uv.x, 1.0 - i.uv.y), i.path.y).a;
        c = mix(c, p, shape);
    }
    // Litter (layer 28) and yellow snow (29), sampled outside any branch
    // so their mipmaps stay defined.
    let litter = textureSample(tiles, samp, i.uv, 28u);
    let yellow = textureSample(tiles, samp, i.uv, 29u);
    if (plain) {
        c = vec3<f32>(1.0);
    }
    if (i.cell.x != 0xffffffffu) {
        let m = textureLoad(marks, vec2<i32>(i.cell), 0).r;
        let dirt = select((m >> 2u) & 3u, 0u, plain);
        if (dirt == 1u) {
            c = mix(c, litter.rgb, litter.a);
        } else if (dirt == 2u) {
            c = mix(c, yellow.rgb, yellow.a);
        }
        // Placement highlight: the cell's color replaces its texture,
        // still lit (measured from the original: 0x6bef73 green, 0xef6d73
        // red).
        let h = m & 3u;
        if (h == 1u) {
            c = vec3<f32>(0.147, 0.863, 0.171);
        } else if (h == 2u) {
            c = vec3<f32>(0.863, 0.153, 0.171);
        }
    }
    let n = normalize(i.normal);
    let d = max(dot(n, g.sun_dir.xyz), 0.0);
    // Building lights also light the ground round them at night
    // (0x42a4f0 adds each to the terrain patches it reaches).
    var lamps = vec3<f32>(0.0);
    if (i.cell.x != 0xffffffffu) {
        lamps = ground_lamp(i.cell, i.uv);
    }
    var lit = min(to_srgb(c) * (g.ambient.rgb + g.sun_color.rgb * d + lamps), vec3<f32>(1.0));
    // Optional cell grid, params.w = cell spacing.
    if (g.params.w > 0.0) {
        let q = i.world.xy / g.params.w;
        let w = fwidth(q);
        let e = abs(fract(q - 0.5) - 0.5) / max(w, vec2<f32>(1e-4));
        let line = 1.0 - clamp(min(e.x, e.y), 0.0, 1.0);
        lit = mix(lit, to_srgb(vec3<f32>(0.15, 0.2, 0.35)), line * 0.6);
    }
    let dist = distance(i.world, g.camera_pos.xyz);
    let f = clamp((dist - g.params.x) / (g.params.y - g.params.x), 0.0, 1.0);
    return vec4<f32>(to_linear(mix(lit, to_srgb(g.fog_color.rgb), f)), 1.0);
}
"#;

pub struct TerrainRenderer {
    pipeline: wgpu::RenderPipeline,
    tex_layout: wgpu::BindGroupLayout,
    tiles: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
    marks: Option<wgpu::Texture>,
    /// Lamp light at each height sample (see `set_ground_lamps`).
    lamps: Option<wgpu::Texture>,
    marks_side: usize,
    marks_now: Vec<u8>,
    vbuf: Option<wgpu::Buffer>,
    count: u32,
    /// Where the world-edge walls begin in the vertex buffer.
    walls: u32,
    /// Low scenery detail: the original draws the ground untextured, in
    /// its lit vertex colors only, and leaves out the world-edge walls
    /// (0x434aa1 and 0x40bce0, 0x435970).
    pub plain: bool,
}

fn hash2(x: usize, y: usize) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
}

impl TerrainRenderer {
    pub fn new(gpu: &Gpu, assets: &Assets, color_format: wgpu::TextureFormat, samples: u32) -> TerrainRenderer {
        let device = &gpu.device;
        let n = LAYERS.len() as u32;
        let mips = 32 - TILE.leading_zeros();
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain tiles"),
            size: wgpu::Extent3d {
                width: TILE,
                height: TILE,
                depth_or_array_layers: n,
            },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (i, rel) in LAYERS.iter().enumerate() {
            let img = assets
                .resolve(rel)
                .and_then(|p| texture::load_any(p).ok())
                .unwrap_or_else(|| {
                    log::warn!("terrain tile {rel} unavailable");
                    texture::Rgba::solid([230, 235, 245, 255])
                });
            let rgba = image::RgbaImage::from_raw(img.width, img.height, img.pixels).unwrap();
            let mut level = image::imageops::resize(&rgba, TILE, TILE, image::imageops::FilterType::Triangle);
            for mip in 0..mips {
                let s = TILE >> mip;
                gpu.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &tex,
                        mip_level: mip,
                        origin: wgpu::Origin3d { x: 0, y: 0, z: i as u32 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &level,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * s),
                        rows_per_image: Some(s),
                    },
                    wgpu::Extent3d {
                        width: s,
                        height: s,
                        depth_or_array_layers: 1,
                    },
                );
                if mip + 1 < mips {
                    level = image::imageops::resize(&level, s / 2, s / 2, image::imageops::FilterType::Triangle);
                }
            }
        }
        let view = tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("terrain sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain tex layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let glayout = globals_layout(device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain layout"),
            bind_group_layouts: &[Some(&glayout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terrain shader"),
            source: wgpu::ShaderSource::Wgsl(format!("{}{}", super::GLOBALS_WGSL, SHADER).into()),
        });
        let attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Uint32x2, 4 => Uint32x2, 5 => Uint32x2];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attrs,
                })],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 1.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        TerrainRenderer {
            pipeline,
            tex_layout,
            tiles: view,
            sampler,
            bind_group: None,
            walls: 0,
            plain: false,
            lamps: None,
            marks: None,
            marks_side: 0,
            marks_now: Vec::new(),
            vbuf: None,
            count: 0,
        }
    }

    /// (Re)creates the per-cell highlight texture for an n×n level.
    fn make_marks(&mut self, gpu: &Gpu, n: usize) {
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain marks"),
            size: wgpu::Extent3d {
                width: n as u32,
                height: n as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let marks_view = tex.create_view(&Default::default());
        let lamps = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain lamps"),
            size: wgpu::Extent3d {
                width: n as u32 + 1,
                height: n as u32 + 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let lamps_view = lamps.create_view(&Default::default());
        self.bind_group = Some(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain tiles"),
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.tiles),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&marks_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&lamps_view),
                },
            ],
        }));
        self.marks = Some(tex);
        self.lamps = Some(lamps);
        self.set_ground_lamps(gpu, &vec![[0.0; 4]; (n + 1) * (n + 1)]);
        self.marks_side = n;
        self.marks_now = Vec::new();
        self.set_marks(gpu, &[], &[]);
    }

    /// Sets the lamp light at each height sample, rows of x, one more a
    /// side than there are cells.
    pub fn set_ground_lamps(&mut self, gpu: &Gpu, light: &[[f32; 4]]) {
        let side = self.marks_side as u32 + 1;
        let Some(tex) = &self.lamps else { return };
        if light.len() != (side * side) as usize {
            return;
        }
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(light),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(side * 16),
                rows_per_image: Some(side),
            },
            wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Sets the highlighted cells and the dirty ones (1 litter, 2 yellow
    /// snow); all others are cleared.
    pub fn set_marks(&mut self, gpu: &Gpu, cells: &[((usize, usize), CellMark)], dirt: &[((usize, usize), u8)]) {
        let n = self.marks_side;
        let Some(tex) = &self.marks else { return };
        let mut buf = vec![0u8; n * n];
        for &((x, y), d) in dirt {
            if x < n && y < n {
                // Litter shows over yellow snow.
                let i = y * n + x;
                let old = buf[i] >> 2;
                buf[i] = (if old == 1 { 1 } else { d }) << 2;
            }
        }
        for &((x, y), m) in cells {
            if x < n && y < n {
                buf[y * n + x] |= m as u8;
            }
        }
        if buf == self.marks_now {
            return;
        }
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &buf,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(n as u32),
                rows_per_image: Some(n as u32),
            },
            wgpu::Extent3d {
                width: n as u32,
                height: n as u32,
                depth_or_array_layers: 1,
            },
        );
        self.marks_now = buf;
    }

    /// Rebuilds the terrain mesh from the level (call after terrain edits).
    /// `deep_snow` says whether ground at a height is under deep snow,
    /// where footpaths are snowy.
    pub fn rebuild(&mut self, gpu: &Gpu, level: &Level, deep_snow: &dyn Fn(f32) -> bool) {
        let n = level.cells_per_side();
        let s = level.spacing;
        if self.marks_side != n || self.marks.is_none() {
            self.make_marks(gpu, n);
        }
        let mut normals = vec![Vec3::Z; level.size * level.size];
        for y in 0..level.size {
            for x in 0..level.size {
                let hl = level.height(x.saturating_sub(1), y);
                let hr = level.height((x + 1).min(level.size - 1), y);
                let hd = level.height(x, y.saturating_sub(1));
                let hu = level.height(x, (y + 1).min(level.size - 1));
                normals[y * level.size + x] = Vec3::new(hl - hr, hd - hu, 2.0 * s).normalize();
            }
        }
        let mut verts = Vec::with_capacity(n * n * 6);
        for cy in 0..n {
            for cx in 0..n {
                let cell = level.cell(cx, cy);
                let h = hash2(cx, cy);
                let center_x = (cx as f32 + 0.5) * s;
                let center_y = (cy as f32 + 0.5) * s;
                let slope = level.slope_deg_at(center_x, center_y);
                let mut base = L_PLAIN + cell.variant.min(2);
                let mut overlay = NO_OVERLAY;
                if cell.flags & cell_flags::FOREST != 0 {
                    base = L_FOREST;
                } else if cell.flags & cell_flags::MOGULS != 0 {
                    base = L_MOGUL + h % 4;
                } else if slope > ROCK_SLOPE {
                    base = L_ROCK + h % 7;
                }
                // Worn snow steps at 20, 40, 60 and 80, though not on trees,
                // moguls or paths (0x40d200).
                let plain_snow = cell.flags & (cell_flags::FOREST | cell_flags::MOGULS) == 0 && cell.path().is_none();
                if cell.wear > 20.0 && plain_snow {
                    overlay = L_WORN + (((cell.wear - 20.0) / 20.0) as u32).min(3);
                }
                let path = match cell.path() {
                    Some(t) => {
                        let snowy = deep_snow(level.height(cx, cy));
                        let near = |dx: i32, dy: i32| {
                            let (x, y) = (cx as i32 + dx, cy as i32 + dy);
                            x >= 0 && y >= 0 && (x as usize) < n && (y as usize) < n && level.cell(x as usize, y as usize).path() == Some(t)
                        };
                        const AROUND: [(i32, i32); 8] = [(-1, 1), (1, 1), (1, -1), (-1, -1), (0, 1), (1, 0), (0, -1), (-1, 0)];
                        let bits = AROUND.iter().enumerate().fold(0usize, |m, (k, &(dx, dy))| m | (near(dx, dy) as usize) << k);
                        [if snowy { L_PATH_SNOW } else { L_PATH } + t as u32, L_PATH_MASK + PATH_MASKS[bits] as u32]
                    }
                    None => [NO_OVERLAY, 0],
                };
                let corner = |dx: usize, dy: usize, u: f32, v: f32| TVertex {
                    pos: [
                        (cx + dx) as f32 * s,
                        (cy + dy) as f32 * s,
                        level.height(cx + dx, cy + dy),
                    ],
                    normal: normals[(cy + dy) * level.size + cx + dx].into(),
                    uv: [u, v],
                    layers: [base, overlay],
                    cell: [cx as u32, cy as u32],
                    path,
                };
                let a = corner(0, 0, 0.0, 1.0);
                let b = corner(1, 0, 1.0, 1.0);
                let c = corner(0, 1, 0.0, 0.0);
                let d = corner(1, 1, 1.0, 0.0);
                verts.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        // The map ends in sheer walls of the world-edge texture down into
        // the background, as in the original: a fixed depth (about eight
        // cells, measured) below the edge.
        self.walls = verts.len() as u32;
        let depth = s * 8.0;
        let tile = s * 4.0;
        // Each side: corner points in order and the outward normal.
        let sides: [(Vec<(usize, usize)>, [f32; 3]); 4] = [
            ((0..=n).map(|i| (n - i, 0)).collect(), [0.0, -1.0, 0.0]),
            ((0..=n).map(|i| (n, n - i)).collect(), [1.0, 0.0, 0.0]),
            ((0..=n).map(|i| (i, n)).collect(), [0.0, 1.0, 0.0]),
            ((0..=n).map(|i| (0, i)).collect(), [-1.0, 0.0, 0.0]),
        ];
        for (pts, normal) in sides {
            let wall = |k: usize, top: bool| -> TVertex {
                let (x, y) = pts[k];
                let z = level.height(x, y);
                let zz = if top { z } else { z - depth };
                TVertex {
                    pos: [x as f32 * s, y as f32 * s, zz],
                    normal,
                    uv: [k as f32 * s / tile, (z - zz) / tile],
                    layers: [L_EDGE, NO_OVERLAY],
                    cell: [u32::MAX; 2],
                    path: [NO_OVERLAY, 0],
                }
            };
            for k in 0..pts.len() - 1 {
                let (a, b, c, d) = (wall(k, true), wall(k + 1, true), wall(k, false), wall(k + 1, false));
                verts.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        self.count = verts.len() as u32;
        self.vbuf = Some(gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain vb"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        }));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, globals: &'a wgpu::BindGroup) {
        let (Some(vb), Some(bg)) = (&self.vbuf, &self.bind_group) else { return };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, globals, &[]);
        pass.set_bind_group(1, bg, &[]);
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.draw(0..if self.plain { self.walls } else { self.count }, 0..1);
    }
}
