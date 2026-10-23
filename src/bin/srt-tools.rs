//! Diagnostic tools for the game data and the engine: decoding checks,
//! offline renders and headless simulation.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use srt::{assets, formats, game, render};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
}

fn check_textures(root: &Path) -> Result<()> {
    let mut files = Vec::new();
    walk(root, &mut files);
    let (mut ok, mut bad) = (0, 0);
    for f in files {
        let ext = f
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let is_tex = ext == "map"
            || (ext == "bin" && f.to_string_lossy().to_ascii_lowercase().contains("textures"));
        if !is_tex {
            continue;
        }
        match formats::texture::load_any(&f) {
            Ok(_) => ok += 1,
            Err(e) => {
                bad += 1;
                println!("FAIL {}: {e:#}", f.display());
            }
        }
    }
    println!("textures ok={ok} failed={bad}");
    Ok(())
}

/// Renders a mesh from four angles into a PNG contact sheet.
fn render_mesh(path: &Path, out: &Path) -> Result<()> {
    render_meshes(&[path.to_path_buf()], out)
}

/// Renders several meshes together (identity transforms) from four angles.
fn render_meshes(paths: &[PathBuf], out: &Path) -> Result<()> {
    let gpu = render::Gpu::headless()?;
    let pipe = render::mesh::MeshPipeline::new(&gpu, wgpu::TextureFormat::Rgba8UnormSrgb, 1);
    let meshes: Vec<_> = paths.iter().map(|p| pipe.load(&gpu, p)).collect::<Result<_>>()?;
    let refs: Vec<&render::mesh::GpuMesh> = meshes.iter().collect();
    let angles: Vec<f32> = (0..4).map(|i| std::f32::consts::FRAC_PI_4 + i as f32 * std::f32::consts::FRAC_PI_2).collect();
    let tiles: Vec<(Vec<&render::mesh::GpuMesh>, f32)> = angles.iter().map(|a| (refs.clone(), *a)).collect();
    draw_tiles(&gpu, &pipe, &tiles, 2, out)?;
    for (p, m) in paths.iter().zip(&meshes) {
        println!("{} bounds {:?} {:?}", p.display(), m.bounds_min, m.bounds_max);
    }
    Ok(())
}

/// Renders the preview of a model at 16 yaws, side by side.
fn render_preview(data: Option<&Path>, rel: &str, zoom: u8, out: &Path) -> Result<()> {
    let assets = assets::Assets::discover(data)?;
    let gpu = render::Gpu::headless()?;
    let mut previewer = render::preview::Previewer::new(&gpu);
    let path = assets.resolve(rel).with_context(|| format!("no {rel}"))?.to_path_buf();
    let (w, h) = (180u32, 188u32);
    let mut sheet = image::RgbaImage::from_pixel(w * 16, h, image::Rgba([255, 255, 255, 255]));
    for k in 0..16u32 {
        let yaw = k as f32 / 16.0 * std::f32::consts::TAU;
        let view = srt::ui::picker::world_view(zoom, yaw);
        let px = previewer.render(&gpu, &assets, &path, false, view, w, h)?;
        let tile = image::RgbaImage::from_raw(w, h, px).context("preview size")?;
        image::imageops::overlay(&mut sheet, &tile, (k * w) as i64, 0);
    }
    sheet.save(out)?;
    Ok(())
}

/// Renders every frame of a skeletal mesh's clip, side by side.
fn render_clip(data: Option<&Path>, rel: &str, clip: &str, yaw: f32, out: &Path) -> Result<()> {
    let assets = assets::Assets::discover(data)?;
    let gpu = render::Gpu::headless()?;
    let pipe = render::mesh::MeshPipeline::new(&gpu, wgpu::TextureFormat::Rgba8UnormSrgb, 1);
    let mut cache = render::scene::MeshCache::default();
    let c = cache
        .clip(&gpu, &assets, rel, clip)
        .with_context(|| {
            let names: Vec<String> = cache.source(&gpu, &assets, rel).map(|s| s.mesh.clips.iter().map(|c| c.name.clone()).collect()).unwrap_or_default();
            format!("{rel} has no clip {clip}; clips: {names:?}")
        })?;
    println!("{} start {} frames {} at {} fps, looping {}", c.name, c.start, c.frames, c.fps, c.looping);
    let ids: Vec<_> = (c.start..c.start + c.frames.max(1))
        .map(|f| cache.get(&gpu, &pipe, &assets, rel, f as usize))
        .collect();
    let tiles: Vec<(Vec<&render::mesh::GpuMesh>, f32)> = ids
        .iter()
        .filter_map(|id| cache.meshes[*id].as_ref())
        .map(|m| (vec![m], yaw))
        .collect();
    draw_tiles(&gpu, &pipe, &tiles, tiles.len().min(6) as u32, out)
}

/// Draws groups of meshes into a grid of 512×512 tiles, each group from its
/// own yaw angle, framed on the union of all bounds.
fn draw_tiles(
    gpu: &render::Gpu,
    pipe: &render::mesh::MeshPipeline,
    tiles: &[(Vec<&render::mesh::GpuMesh>, f32)],
    cols: u32,
    out: &Path,
) -> Result<()> {
    use glam::{Mat4, Vec3, Vec4};
    use wgpu::util::DeviceExt;
    let fmt = wgpu::TextureFormat::Rgba8UnormSrgb;
    let meshes: Vec<&render::mesh::GpuMesh> = tiles.iter().flat_map(|t| t.0.iter().copied()).collect();
    let bmin = meshes.iter().fold(Vec3::splat(f32::MAX), |a, m| a.min(m.bounds_min));
    let bmax = meshes.iter().fold(Vec3::splat(f32::MIN), |a, m| a.max(m.bounds_max));
    let center = (bmin + bmax) * 0.5;
    let radius = (bmax - bmin).length().max(0.01) * 0.5;
    let inst = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&render::mesh::Instance::new(Mat4::IDENTITY, Vec4::ONE)),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let (w, h) = (512u32, 512u32);
    let cols = cols.max(1);
    let rows = (tiles.len() as u32).div_ceil(cols).max(1);
    let mut sheet = image::RgbaImage::new(w * cols, h * rows);
    for (i, (group, ang)) in tiles.iter().enumerate() {
        let i = i as u32;
        let ang = *ang;
        let eye = center + Vec3::new(ang.cos(), ang.sin(), 0.6).normalize() * radius * 2.8;
        let view = render::look_at(eye, center, Vec3::Z);
        let proj = render::perspective(0.8, 1.0, radius * 0.05, radius * 10.0);
        let mut globals = render::Globals::new(proj * view, eye);
        globals.params = [1e6, 2e6, 0.0, 0.0];
        let gbuf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let gbg = pipe.globals_bind_group(&gpu, &gbuf);
        let depth = gpu.depth_view(w, h, 1);
        let px = gpu.capture(w, h, fmt, |enc, view| {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.3, g: 0.35, b: 0.45, a: 1.0 }),
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
            for mesh in group {
                pipe.draw(&mut pass, mesh, &inst, 0..1);
            }
        })?;
        let img = image::RgbaImage::from_raw(w, h, px).unwrap();
        image::imageops::overlay(&mut sheet, &img, ((i % cols) * w) as i64, ((i / cols) * h) as i64);
    }
    sheet.save(out)?;
    Ok(())
}

/// Places foliage instances for every cell of a level.
fn foliage_batches(
    gpu: &render::Gpu,
    sr: &render::scene::SceneRenderer,
    assets: &assets::Assets,
    cache: &mut render::scene::MeshCache,
    level: &formats::level::Level,
    batches: &mut render::scene::Batches,
) {
    use glam::{Mat4, Quat, Vec3, Vec4};
    let n = level.cells_per_side();
    for cy in 0..n {
        for cx in 0..n {
            let c = level.cell(cx, cy);
            let Some(path) = formats::level::FOLIAGE_MESHES.get(c.foliage as usize) else { continue };
            if path.is_empty() {
                continue;
            }
            let id = cache.get(gpu, &sr.meshes, assets, path, 0);
            let x = (cx as f32 + 0.5) * level.spacing;
            let y = (cy as f32 + 0.5) * level.spacing;
            let z = level.height_at(x, y);
            let rot = ((cx * 7 + cy * 13) % 16) as f32 * 0.39;
            let m = Mat4::from_rotation_translation(Quat::from_rotation_z(rot), Vec3::new(x, y, z));
            batches.push(id, m, Vec4::ONE);
        }
    }
}

fn shot_level(data: &Path, lev: &str, out: &Path, yaw: f32) -> Result<()> {
    use glam::Vec3;
    let assets = assets::Assets::open(data)?;
    // A level of the game's, or any level file.
    let path = match assets.resolve_level(lev) {
        Some(p) => p,
        None if Path::new(lev).is_file() => PathBuf::from(lev),
        None => anyhow::bail!("no level {lev}"),
    };
    let level = formats::level::Level::load(&path)?;
    let gpu = render::Gpu::headless()?;
    let fmt = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut sr = render::scene::SceneRenderer::new(&gpu, &assets, fmt, 1);
    sr.terrain.rebuild(&gpu, &level, &|_| false);
    let mut cache = render::scene::MeshCache::default();
    let mut batches = render::scene::Batches::default();
    foliage_batches(&gpu, &sr, &assets, &mut cache, &level, &mut batches);
    let ext = level.extent();
    let target = Vec3::new(ext * 0.5, ext * 0.5, level.height_at(ext * 0.5, ext * 0.5));
    let eye = target + Vec3::new(yaw.cos() * ext * 0.93, yaw.sin() * ext * 0.93, ext * 0.6);
    let (w, h) = (1600u32, 900u32);
    let vp = render::perspective(0.8, w as f32 / h as f32, 5.0, ext * 4.0) * render::look_at(eye, target, Vec3::Z);
    let mut globals = render::Globals::new(vp, eye);
    globals.params = [ext * 1.2, ext * 3.0, 0.0, 0.0];
    let depth = gpu.depth_view(w, h, 1);
    let px = gpu.capture(w, h, fmt, |enc, view| {
        sr.render(&gpu, enc, view, None, &depth, &globals, &cache, &batches);
    })?;
    image::RgbaImage::from_raw(w, h, px).unwrap().save(out)?;
    println!("level size {} spacing {} trees {}", level.size, level.spacing,
        batches.lists.values().map(|v| v.len()).sum::<usize>());
    Ok(())
}

/// Loads a save and steps it `days` sim days, reporting guests and cash.
fn check_save(data: Option<&Path>, save: &Path, days: f32) -> Result<()> {
    use std::sync::Arc;
    let assets = Arc::new(assets::Assets::discover(data)?);
    let mut g = game::Game::load_save(&assets, save)?;
    println!(
        "loaded: day {} structures {} lifts {} runs {} guests {} cash {:.0}",
        g.world.day,
        g.world.structures.len(),
        g.world.lifts.len(),
        g.world.runs.len(),
        g.world.active_guests(),
        g.world.cash
    );
    let ticks = (days * g.defs.resort.seconds_per_day / game::sim::TICK) as usize;
    for _ in 0..ticks {
        game::sim::Sim { defs: &g.defs, w: &mut g.world }.step(game::sim::TICK);
    }
    println!("after {days} days: day {} guests {} cash {:.0}", g.world.day, g.world.active_guests(), g.world.cash);
    Ok(())
}

/// Headless run: auto-build a starter resort, simulate, print stats and
/// optionally render a screenshot.
fn simulate(data: Option<&Path>, level: &str, days: f32, shot: Option<&Path>) -> Result<()> {
    use std::sync::Arc;
    let assets = Arc::new(assets::Assets::discover(data)?);
    let mut g = game::Game::new_from_level(&assets, level, None)?;
    let built = g.autobuild_starter();
    println!(
        "autobuild: lift={built} structures={} lifts={} runs={} cash={:.0}",
        g.world.structures.len(),
        g.world.lifts.len(),
        g.world.runs.len(),
        g.world.cash
    );
    let ticks = (days * g.defs.resort.seconds_per_day / game::sim::TICK) as usize;
    let t0 = std::time::Instant::now();
    // SRT_AVALANCHE_CONTROL=1 adds an avalanche control building.
    if std::env::var_os("SRT_AVALANCHE_CONTROL").is_some() {
        let def = g.defs.structures.iter().position(|d| d.kind == game::defs::StructureKind::AvalancheControl);
        let at = g.world.structures.first().map(|e| e.pos.truncate() + glam::Vec2::new(80.0, 80.0));
        if let (Some(def), Some(at)) = (def, at) {
            println!("avalanche control: {:?}", g.place_near(def, at));
        }
    }
    // SRT_BUILD_EXTRA=<name> adds the building whose unique name starts
    // with it, near the entrance.
    if let Ok(name) = std::env::var("SRT_BUILD_EXTRA") {
        let def = g.defs.structures.iter().position(|d| d.unique_name.to_ascii_lowercase().starts_with(&name.to_ascii_lowercase()));
        let at = g.world.structures.first().map(|e| e.pos.truncate() + glam::Vec2::new(60.0, 60.0));
        if let (Some(def), Some(at)) = (def, at) {
            let name = g.defs.structures[def].unique_name.clone();
            println!("{name}: {:?}", g.place_near(def, at));
        }
    }
    game::sim::Sim { defs: &g.defs, w: &mut g.world }.spawn_wildlife(true);
    g.world.wildlife_spawned = true;
    // SRT_FLUSH_ANIMALS=t clears the trees under every animal t ticks in.
    let flush_at = std::env::var("SRT_FLUSH_ANIMALS").ok().and_then(|v| v.parse::<usize>().ok());
    // SRT_FORCE_AVALANCHE=t sets one off t ticks in.
    let avalanche_at = std::env::var("SRT_FORCE_AVALANCHE").ok().and_then(|v| v.parse::<usize>().ok());
    // SRT_SNOW_ALL=t orders snow on every worn cell t ticks in.
    let snow_at = std::env::var("SRT_SNOW_ALL").ok().and_then(|v| v.parse::<usize>().ok());
    // SRT_INJURE=t lays the farthest guest from the entrance up hurt t
    // ticks in, then prints the patrols every second for a minute.
    let injure_at = std::env::var("SRT_INJURE").ok().and_then(|v| v.parse::<usize>().ok());
    for i in 0..ticks {
        if injure_at == Some(i) {
            let gate = g.world.structures.first().map(|e| e.pos).unwrap_or_default();
            if let Some(v) = g.world.guests.iter_mut().max_by(|a, b| a.pos.distance(gate).total_cmp(&b.pos.distance(gate))) {
                v.state = game::world::GuestState::Injured;
                println!("injured guest {} at {:.0},{:.0}", v.id, v.pos.x, v.pos.y);
            }
        }
        if let Some(t) = injure_at {
            if i > t && i < t + 1800 && (i - t) % 30 == 0 {
                let w = &g.world;
                for s in w.staff.iter().filter(|s| s.kind == game::world::StaffKind::Patrol && !matches!(s.task, game::world::StaffTask::Idle)) {
                    let up = s.pos.z - w.ground(s.pos.truncate()).z;
                    println!("  t+{}s {} {:?} at {:.0},{:.0} up {up:.1}", (i - t) / 30, s.name, s.task, s.pos.x, s.pos.y);
                }
            }
        }
        if snow_at == Some(i) {
            let n = g.world.level().cells_per_side();
            let worn: Vec<(usize, usize)> = (0..n * n).map(|k| (k % n, k / n)).filter(|&(x, y)| g.world.level().cell(x, y).wear > 20.0).collect();
            println!("snow ordered on {} cells", worn.len());
            g.world.snow_jobs.extend(worn);
        }
        if flush_at == Some(i) {
            let s = g.world.level().spacing;
            let cells: Vec<(usize, usize)> = g.world.animals.iter().map(|a| ((a.pos.x / s) as usize, (a.pos.y / s) as usize)).collect();
            for (x, y) in cells {
                g.world.level_mut().cell_mut(x, y).flags &= !formats::level::cell_flags::FOREST;
            }
        }
        if avalanche_at == Some(i) {
            g.force_avalanche();
            let a = g.world.avalanches.first().map(|a| a.pos);
            println!("avalanche at {a:?}");
        }
        game::sim::Sim { defs: &g.defs, w: &mut g.world }.step(game::sim::TICK);
        if !g.world.avalanches.is_empty() {
            g.puffs(game::sim::TICK);
        }
        if i % (ticks / 10).max(1) == 0 {
            let w = &g.world;
            let states = w.guests.iter().fold([0usize; 6], |mut a, x| {
                use game::world::GuestState::*;
                let k = match x.state {
                    Walking { .. } => 0,
                    InStructure { .. } => 1,
                    Queued { .. } | AtDoor { .. } => 2,
                    Riding { .. } => 3,
                    Skiing { .. } => 4,
                    _ => 5,
                };
                a[k] += 1;
                a
            });
            println!(
                "day {:>3} {:05.2}h guests {:>4}/{:<4} walk {} in {} queue {} ride {} ski {} other {} cash {:>9.0} stars {:.2} happy {:.2} | {:?} {:.0}F {:.2}in depth {:.1} warmth {:.2}",
                w.day, w.hour, w.active_guests(), w.max_guests, states[0], states[1], states[2], states[3], states[4], states[5],
                w.cash, w.rating.stars,
                game::sim::Sim { defs: &g.defs, w: &mut g.world.clone() }.average_happiness(),
                w.weather.sky, w.weather.temperature, w.weather.pressure, w.weather.snow_depth,
                w.guests.iter().filter(|x| x.is_active()).map(|x| x.need(game::defs::NeedKind::Warmth)).sum::<f32>() / w.active_guests().max(1) as f32
            );
            // What the guests heading up mean to ski (runs, parks).
            let aims = w.guests.iter().fold([0usize; 2], |mut a, x| {
                match x.aim {
                    Some(game::world::Aim::Run(_)) => a[0] += 1,
                    Some(game::world::Aim::Park(_)) => a[1] += 1,
                    None => {}
                }
                a
            });
            println!("    aiming at runs {} parks {}", aims[0], aims[1]);
            let worn = w.level().cells.iter().fold([0usize; 3], |mut a, c| {
                a[if c.wear > 60.0 { 2 } else if c.wear > 20.0 { 1 } else { 0 }] += 1;
                a
            });
            println!("    worn cells: >20 {} >60 {}", worn[1], worn[2]);
            let speeds: Vec<f32> = w.guests.iter().filter(|x| matches!(x.state, game::world::GuestState::Skiing { .. })).map(|x| x.speed).collect();
            if !speeds.is_empty() {
                let max = speeds.iter().cloned().fold(0.0f32, f32::max);
                println!("    skiing {} avg speed {:.1} max {:.1}; hurt/down {}", speeds.len(), speeds.iter().sum::<f32>() / speeds.len() as f32, max,
                    w.guests.iter().filter(|x| matches!(x.state, game::world::GuestState::Injured | game::world::GuestState::Crashed { .. })).count());
            }
            for s in w.staff.iter().filter(|s| matches!(s.kind, game::world::StaffKind::SnowTour | game::world::StaffKind::Snowmaker | game::world::StaffKind::Groomer | game::world::StaffKind::Patrol)) {
                let up = s.pos.z - w.ground(s.pos.truncate()).z;
                println!("    {} {:?} at {:.0},{:.0} up {up:.1}", s.name, s.task, s.pos.x, s.pos.y);
            }
            if !w.snow_jobs.is_empty() || !w.groom_jobs.is_empty() {
                println!("    jobs: snow {} groom {}", w.snow_jobs.len(), w.groom_jobs.len());
            }
        }
    }
    for st in g.world.staff.iter().filter(|s| s.kind == game::world::StaffKind::AvalancheControl) {
        println!("{} {:?} at {:?}", st.name, st.task, st.pos);
    }
    println!("avalanche danger {:.2}", g.avalanche_danger());
    println!("simulated {days} days in {:.2}s", t0.elapsed().as_secs_f32());
    for m in g.world.messages.iter().rev().take(40).rev() {
        println!("  msg day {}: {}", m.day, m.text);
    }
    if let Some(d) = g.world.history.last() {
        println!("last day income {:.0} expense {:.0}", d.total_income(), d.total_expense());
        for (l, v) in d.income.iter().chain(d.expense.iter()) {
            println!("    {:<14} {:>9.0}", l.label(), v);
        }
    }
    if let Some(out) = shot {
        let gpu = render::Gpu::headless()?;
        let fmt = wgpu::TextureFormat::Rgba8UnormSrgb;
        let mut sr = render::scene::SceneRenderer::new(&gpu, &assets, fmt, 1);
        g.attach_renderer(&gpu, &mut sr);
        let along: f32 = std::env::var("SRT_SHOT_ALONG").ok().and_then(|v| v.parse().ok()).unwrap_or(0.3);
        if let Some(l) = g.world.lifts.first() {
            g.camera.target = l.start.lerp(l.end, along);
        }
        if std::env::var_os("SRT_DUMP_CABLE").is_some() {
            for l in &g.world.lifts {
                let dir = (l.end - l.start).truncate().normalize_or_zero();
                let side = glam::Vec2::new(-dir.y, dir.x);
                println!("lift {} poles {}", l.name, l.poles.len());
                if let (Some(b), Some(q)) = (l.board_s, l.queue_lines.first().and_then(|q| q.first())) {
                    println!("  board_s {b:.1} of {:.1}: chair at {:?}, pt0 {:?}", l.length() * 2.0, l.point_at(b).0, q);
                }
                let d = &g.defs.lifts[l.def];
                println!("  begin {:?}\n  middle {:?}\n  end {:?}", d.begin_pts, d.middle_pts, d.end_pts);
                for p in &l.cable_loop {
                    let r = p.truncate() - l.start.truncate();
                    println!("  along {:8.2} side {:7.2} z {:7.2}", r.dot(dir), r.dot(side), p.z - l.start.z);
                }
            }
        }
        // SRT_SHOT_GUEST=n centers on the n-th guest outdoors instead.
        if let Some(n) = std::env::var("SRT_SHOT_GUEST").ok().and_then(|v| v.parse::<usize>().ok()) {
            if let Some(x) = g.world.guests.iter().filter(|x| x.pos.z > -100.0).nth(n) {
                println!("guest {} state {:?} speed {:.1}", x.name, std::mem::discriminant(&x.state), x.speed);
                g.camera.target = x.pos;
            }
        }
        if let Some(a) = g.world.animals.iter().find(|a| a.clip.is_some()) {
            println!("animal {:?} at {:?} state {:?} clip {:?}", a.kind, a.pos, a.state, a.clip);
            g.camera.target = a.pos;
        }
        if let Some(a) = g.world.avalanches.first() {
            println!("avalanche at {:?} after {} ticks", a.pos, a.ticks);
            g.camera.target = a.pos;
        }
        g.camera.distance = std::env::var("SRT_SHOT_DIST").ok().and_then(|v| v.parse().ok()).unwrap_or(420.0);
        g.camera.pitch = std::env::var("SRT_SHOT_PITCH").ok().and_then(|v| v.parse().ok()).unwrap_or(0.6);
        g.camera.yaw = std::env::var("SRT_SHOT_YAW").ok().and_then(|v| v.parse().ok()).unwrap_or(-2.4);
        g.world.hour = 12.0;
        // SRT_SHOT_NIGHT=1 renders at night.
        if std::env::var_os("SRT_SHOT_NIGHT").is_some() {
            g.world.daylight.phase = game::daylight::Phase::Night;
        }
        g.prepare_render(&gpu, &mut sr);
        let (w, h) = (1600u32, 900u32);
        g.camera.aspect = w as f32 / h as f32;
        let mut globals = render::Globals::new(g.camera.view_proj(), g.camera.eye());
        g.environment(&mut globals);
        sr.clear = wgpu::Color { r: globals.fog_color[0] as f64, g: globals.fog_color[1] as f64, b: globals.fog_color[2] as f64, a: 1.0 };
        let depth = gpu.depth_view(w, h, 1);
        let px = gpu.capture(w, h, fmt, |enc, view| {
            sr.render(&gpu, enc, view, None, &depth, &globals, &g.meshes, &g.batches);
        })?;
        image::RgbaImage::from_raw(w, h, px).unwrap().save(out)?;
        println!("screenshot written to {}", out.display());
    }
    Ok(())
}

fn scan_lights(dir: &Path) -> Result<()> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)?.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("mesh")) {
                let Ok(m) = formats::mesh::Mesh::load(&p) else { continue };
                for (xf, data) in &m.lights {
                    let floats: Vec<String> = data.chunks_exact(4).take(40).map(|c| {
                        let v = f32::from_le_bytes(c.try_into().unwrap());
                        let u = u32::from_le_bytes(c.try_into().unwrap());
                        if v.is_finite() && v.abs() < 1e7 && (v == 0.0 || v.abs() > 1e-6) { format!("{v}") } else { format!("#{u:x}") }
                    }).collect();
                    println!("{} at {:?} len {}: {}", p.display(), xf.w_axis.truncate(), data.len(), floats.join(" "));
                }
            }
        }
    }
    Ok(())
}

fn dump_ui(path: &Path, out: Option<&Path>) -> Result<()> {
    let scr = formats::ui::Screen::load(path)?;
    let ok = scr.sprites.iter().filter(|s| s.image.is_some()).count();
    println!("screen {} sprites {}/{} decoded", scr.name, ok, scr.sprites.len());
    for g in &scr.groups {
        let names: Vec<String> = g
            .elements
            .iter()
            .map(|e| {
                format!(
                    "{}@{},{},{},{}",
                    e.sprite.map(|i| scr.sprites[i].name.as_str()).unwrap_or("-"),
                    e.rect.x1, e.rect.y1, e.rect.x2, e.rect.y2
                )
            })
            .collect();
        println!("  {} [{}]", g.name, names.join(" "));
        if std::env::var_os("SRT_DUMP_LAYERS").is_some() {
            for (k, e) in g.elements.iter().enumerate() {
                for l in &e.layers {
                    let size = l.sprite.and_then(|i| scr.sprites[i].image.as_ref()).map(|im| (im.width, im.height));
                    println!("    element {k} layer dst {:?} src {},{},{},{} size {:?}", l.dst, l.src.x1, l.src.y1, l.src.x2, l.src.y2, size);
                }
            }
        }
    }
    if let Some(out) = out {
        std::fs::create_dir_all(out)?;
        for s in &scr.sprites {
            if let Some(img) = &s.image {
                image::RgbaImage::from_raw(img.width, img.height, img.pixels.clone())
                    .unwrap()
                    .save(out.join(format!("{}.png", s.name)))?;
            }
        }
    }
    Ok(())
}

/// Diagnostic tools for srt.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Folder containing the game's DATA directory. Defaults to ./gamedata,
    /// then srt/gamedata in the per-user data folder.
    #[arg(long, global = true, env = "SRT_DATA")]
    data: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a headless simulation of an auto-built resort.
    Simulate {
        #[arg(default_value = "data/levels/Aspen Colorado.lev")]
        level: String,
        /// Number of sim days.
        #[arg(default_value_t = 3.0)]
        days: f32,
        /// Write a screenshot here when done.
        shot: Option<PathBuf>,
    },
    /// Load a saved game and run it for a while, to check old saves still
    /// load and play.
    CheckSave {
        save: PathBuf,
        /// Number of sim days.
        #[arg(default_value_t = 1.0)]
        days: f32,
    },
    /// Decode every texture under a folder and report failures.
    CheckTextures { dir: PathBuf },
    /// Decode one texture (.bin, .tga, ...) to a PNG.
    DumpTexture { file: PathBuf, out: PathBuf },
    /// Render a mesh from four angles to a PNG.
    RenderMesh { mesh: PathBuf, out: PathBuf },
    /// Render every frame of a skeletal mesh's clip to a PNG.
    RenderClip {
        /// Game-relative mesh path, e.g., data/scenery/people/skiers/skier_red.mesh.
        mesh: String,
        clip: String,
        #[arg(long, default_value_t = 1.57)]
        yaw: f32,
        out: PathBuf,
    },
    /// Render a model as the windows and pickers preview it, at 16 yaws
    /// (every 22.5°, starting at 0) in a row.
    RenderPreview {
        /// Game-relative mesh path.
        mesh: String,
        /// Zoom level: 0 nearest, 1 the usual.
        #[arg(long, default_value_t = 1)]
        zoom: u8,
        out: PathBuf,
    },
    /// Render several meshes together to a PNG.
    RenderMeshes {
        #[arg(required = true, num_args = 1..)]
        meshes: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Render an overview of a level to a PNG.
    ShotLevel {
        level: String,
        out: PathBuf,
        /// Camera direction around the map, radians.
        #[arg(long, default_value_t = -2.2, allow_hyphen_values = true)]
        yaw: f32,
    },
    /// Make a map as the map editor starts one and save it as a level:
    /// `flat:<size>`, a height map (`data/height_maps/<name>.bmp`), or a
    /// level.
    MakeMap { source: String, out: PathBuf },
    /// Redraw the settings background (a 640x480 PNG) as the game does.
    SettingsArt { input: PathBuf, out: PathBuf },
    /// List the groups of a UI screen file, optionally saving its sprites.
    DumpUi { file: PathBuf, out: Option<PathBuf> },
    /// List the light nodes of every mesh under a folder.
    ScanLights { dir: PathBuf },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,srt=info")).init();
    let cli = Cli::parse();
    let data = cli.data.as_deref();
    match cli.command {
        Command::Simulate { level, days, shot } => simulate(data, &level, days, shot.as_deref()),
        Command::CheckSave { save, days } => check_save(data, &save, days),
        Command::CheckTextures { dir } => check_textures(&dir),
        Command::DumpTexture { file, out } => {
            let t = formats::texture::load_any(&file)?;
            image::RgbaImage::from_raw(t.width, t.height, t.pixels)
                .context("bad texture size")?
                .save(&out)?;
            println!("{}x{} written to {}", t.width, t.height, out.display());
            Ok(())
        }
        Command::RenderMesh { mesh, out } => render_mesh(&mesh, &out),
        Command::RenderMeshes { meshes, out } => render_meshes(&meshes, &out),
        Command::RenderPreview { mesh, zoom, out } => render_preview(data, &mesh, zoom, &out),
        Command::RenderClip { mesh, clip, yaw, out } => render_clip(data, &mesh, &clip, yaw, &out),
        Command::ShotLevel { level, out, yaw } => {
            let root = assets::Assets::discover(data)?.root.clone();
            shot_level(&root, &level, &out, yaw)
        }
        Command::MakeMap { source, out } => make_map(data, &source, &out),
        Command::SettingsArt { input, out } => {
            let mut img = formats::texture::load_any(&input)?;
            srt::ui::settings_art::lift_controls(&mut img);
            image::RgbaImage::from_raw(img.width, img.height, img.pixels).context("bad image")?.save(&out)?;
            Ok(())
        }
        Command::DumpUi { file, out } => dump_ui(&file, out.as_deref()),
        Command::ScanLights { dir } => scan_lights(&dir),
    }
}

fn make_map(data: Option<&Path>, source: &str, out: &Path) -> Result<()> {
    use game::editor::MapSource;
    let assets = assets::Assets::discover(data)?;
    if let Some(n) = source.strip_prefix("random:") {
        // Flat map, random anchors (Randomize Wireframe), then Generate
        // Terrain and Generate Trees with the window's defaults.
        let mut rand = game::editor::crand();
        let mut level = formats::level::Level::flat(game::editor::flat_size(n), &mut rand);
        let k = level.cells_per_side();
        let settings = game::editor::GenSettings::default();
        let samples = game::editor::anchor_samples(k, (k, k), (0, k), (k, 0), (0, 0));
        let anchors = samples.map(|s| (s, 5.0 * (rand() % 101) as f32));
        game::editor::generate_terrain(&mut level, &anchors, settings.rockiness, true, &mut rand);
        game::editor::generate_trees(&mut level, settings.tree_density, settings.treeline, true, &mut rand);
        let trees = level.cells.iter().filter(|c| c.foliage != 0).count();
        let (lo, hi) = level.height_range();
        println!("{} samples a side, heights {lo}..{hi}, {trees} trees", level.size);
        std::fs::write(out, level.write())?;
        return Ok(());
    }
    let src = match source.strip_prefix("flat:") {
        Some(n) => MapSource::Flat(game::editor::flat_size(n)),
        None if source.to_ascii_lowercase().ends_with(".bmp") => MapSource::HeightMap(source.to_string()),
        None => MapSource::Level(source.to_string()),
    };
    let (level, _) = game::editor::make_map(&assets, &src)?;
    let trees = level.cells.iter().filter(|c| c.foliage != 0).count();
    let (lo, hi) = level.height_range();
    println!("{} samples a side, heights {lo}..{hi}, {trees} trees", level.size);
    std::fs::write(out, level.write())?;
    Ok(())
}
