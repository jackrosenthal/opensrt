//! Window, input and frame loop.

use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use glam::{Vec2, Vec3};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::assets::Assets;
use crate::camera::Camera;
use crate::game::Game;
use crate::render::scene::SceneRenderer;
use crate::render::{Globals, Gpu};
use crate::ui::Ui;

pub const MSAA: u32 = 4;

#[derive(Default)]
pub struct Input {
    pub keys: std::collections::HashSet<KeyCode>,
    pub mouse: Vec2,
    pub mouse_delta: Vec2,
    pub left_down: bool,
    pub right_down: bool,
    pub middle_down: bool,
    pub left_clicked: bool,
    pub right_clicked: bool,
    pub scroll: f32,
    pub pressed: Vec<KeyCode>,
    /// The pointer is inside the window (for edge scrolling).
    pub inside: bool,
    /// A right click released without dragging.
    pub right_tap: bool,
    right_press: Vec2,
}

impl Input {
    pub fn held(&self, k: KeyCode) -> bool {
        self.keys.contains(&k)
    }
    fn end_frame(&mut self) {
        self.mouse_delta = Vec2::ZERO;
        self.left_clicked = false;
        self.right_clicked = false;
        self.right_tap = false;
        self.scroll = 0.0;
        self.pressed.clear();
    }
}

struct Gfx {
    window: Arc<Window>,
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    msaa_view: wgpu::TextureView,
    depth: wgpu::TextureView,
    scene: SceneRenderer,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    cursors: crate::cursor::Cursors,
    previewer: Option<crate::render::preview::Previewer>,
}

pub struct App {
    assets: Arc<Assets>,
    gfx: Option<Gfx>,
    game: Option<Game>,
    ui: Ui,
    input: Input,
    last_frame: Instant,
    egui_ctx: egui::Context,
    start_level: Option<String>,
    had_pointer: bool,
}

impl App {
    pub fn run(assets: Assets, start_level: Option<String>) -> Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        // Testing hook: open a front-end screen directly.
        let screen = match std::env::var("SRT_SCREEN").as_deref() {
            Ok("instant") => crate::ui::Screen::NewGame,
            Ok("challenges") => crate::ui::Screen::Challenges,
            Ok("load") => crate::ui::Screen::LoadGame,
            Ok("settings") => crate::ui::Screen::Settings,
            Ok("editor") => crate::ui::Screen::MapEditor,
            Ok("credits") => crate::ui::Screen::Credits,
            _ => crate::ui::Screen::MainMenu,
        };
        let mut app = App {
            assets: Arc::new(assets),
            gfx: None,
            game: None,
            ui: Ui {
                settings: crate::settings::Settings::load(),
                screen,
                start_cash: 50_000.0,
                difficulty: 1,
                ..Default::default()
            },
            input: Input::default(),
            last_frame: Instant::now(),
            egui_ctx: egui::Context::default(),
            start_level,
            had_pointer: false,
        };
        event_loop.run_app(&mut app)?;
        Ok(())
    }

    fn make_targets(gpu: &Gpu, config: &wgpu::SurfaceConfiguration) -> (wgpu::TextureView, wgpu::TextureView) {
        let msaa = gpu
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("msaa color"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: MSAA,
                dimension: wgpu::TextureDimension::D2,
                format: config.format.add_srgb_suffix(),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let depth = gpu.depth_view(config.width, config.height, MSAA);
        (msaa, depth)
    }

    fn init_gfx(&mut self, el: &ActiveEventLoop) -> Result<()> {
        let mut attrs = Window::default_attributes().with_title("Ski Resort Tycoon: Deep Powder");
        // As F11 last left it: fullscreen, or a window of the size it had.
        let (w, h) = self.ui.settings.window.unwrap_or((1280, 960));
        attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(w as f64, h as f64));
        if self.ui.settings.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let window = Arc::new(el.create_window(attrs)?);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(
            Box::new(el.owned_display_handle()),
        ));
        let surface = instance.create_surface(window.clone())?;
        let gpu = Gpu::new(instance, Some(&surface))?;
        let caps = surface.get_capabilities(&gpu.adapter);
        // The surface is linear for egui; the 3D scene renders through an
        // sRGB view of the same texture.
        let view_formats_ok = gpu
            .adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::SURFACE_VIEW_FORMATS);
        let format = if view_formats_ok {
            caps.formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb() && f.add_srgb_suffix() != *f)
                .unwrap_or(caps.formats[0])
        } else {
            caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0])
        };
        let scene_format = format.add_srgb_suffix();
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: Default::default(),
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: if scene_format != format { vec![scene_format] } else { vec![] },
        };
        surface.configure(&gpu.device, &config);
        let (msaa_view, depth) = Self::make_targets(&gpu, &config);
        let scene = SceneRenderer::new(&gpu, &self.assets, scene_format, MSAA);
        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(gpu.device.limits().max_texture_dimension_2d as usize),
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                depth_stencil_format: None,
                dithering: true,
                ..Default::default()
            },
        );
        crate::ui::style::apply(&self.egui_ctx);
        crate::ui::fonts::install(&self.egui_ctx);
        let cursors = crate::cursor::Cursors::load(el, &self.assets, ui_scale(config.width, config.height));
        self.gfx = Some(Gfx {
            window,
            gpu,
            surface,
            config,
            msaa_view,
            depth,
            scene,
            egui_state,
            egui_renderer,
            cursors,
            previewer: None,
        });
        if let Some(lvl) = self.start_level.take() {
            self.start_game(&lvl);
        }
        Ok(())
    }

    fn start_game(&mut self, level_rel: &str) {
        let (rel, challenge) = match level_rel.split_once("#challenge") {
            Some((r, n)) => (r.to_string(), n.parse::<usize>().ok()),
            None => (level_rel.to_string(), None),
        };
        let game = Game::new_from_level(&self.assets, &rel, challenge).map(|mut g| {
            if challenge.is_none() {
                g.world.cash = self.ui.start_cash;
            }
            g
        });
        self.enter_game(game, level_rel);
    }

    fn start_editor(&mut self, source: &crate::game::editor::MapSource) {
        let game = Game::new_editor(&self.assets, source);
        self.enter_game(game, &format!("{source:?}"));
    }

    fn enter_game(&mut self, game: anyhow::Result<Game>, what: &str) {
        let Some(gfx) = self.gfx.as_mut() else { return };
        match game {
            Ok(mut g) => {
                g.settings = self.ui.settings.clone();
                g.attach_renderer(&gfx.gpu, &mut gfx.scene);
                // The original opens a game with the mini-map showing.
                self.ui.show_minimap = true;
                if let Some(hook) = crate::testhook::TestHook::from_env() {
                    self.apply_hook(hook, &mut g);
                }
                self.game = Some(g);
                self.ui.screen = crate::ui::Screen::InGame;
            }
            Err(e) => {
                log::error!("failed to start {what}: {e:#}");
                self.ui.error = Some(format!("{e:#}"));
            }
        }
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        let Some(gfx) = self.gfx.as_mut() else { return };

        let raw = gfx.egui_state.take_egui_input(&gfx.window);
        let mut actions = Vec::new();
        let full = self.egui_ctx.run_ui(raw, |ui_root| {
            actions = self.ui.show(ui_root, &self.assets, self.game.as_mut());
        });
        // Render model previews the pickers asked for.
        if !self.ui.previews.requests.is_empty() {
            let previewer = gfx.previewer.get_or_insert_with(|| crate::render::preview::Previewer::new(&gfx.gpu));
            let reqs: Vec<(String, u8)> = self.ui.previews.requests.drain(..).take(3).collect();
            for (mesh, view) in reqs {
                let (w, h) = (180u32, 188u32);
                // A `#sign` suffix asks for the entrance's sign on it.
                let (path, sign) = match mesh.strip_suffix(crate::ui::picker::WITH_SIGN) {
                    Some(p) => (p, true),
                    None => (mesh.as_str(), false),
                };
                let tex = self
                    .assets
                    .resolve(path)
                    .and_then(|p| previewer.render(&gfx.gpu, &self.assets, p, sign, view, w, h).map_err(|e| log::warn!("{e:#}")).ok())
                    .map(|px| {
                        let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &px);
                        self.egui_ctx.load_texture(format!("preview:{mesh}:{view}"), img, egui::TextureOptions::LINEAR)
                    });
                self.ui.previews.textures.insert((mesh, view), tex);
            }
            self.egui_ctx.request_repaint();
        }
        let over_hud = self.ui.screen == crate::ui::Screen::InGame && self.ui.hud.covers(self.egui_ctx.pointer_hover_pos());
        let wants_pointer = self.egui_ctx.egui_wants_pointer_input() || self.egui_ctx.is_pointer_over_egui() || over_hud;
        let wants_keys = self.egui_ctx.text_edit_focused();
        // The game draws its own cursors; keep egui from switching to
        // system ones.
        let mut platform = full.platform_output.clone();
        platform.cursor_icon = egui::CursorIcon::Default;
        gfx.egui_state.handle_platform_output(&gfx.window, platform);
        // egui-winit puts the system cursor back the first frame the
        // pointer is inside the window again; re-apply ours after it.
        let has_pointer = self.egui_ctx.input(|i| i.pointer.has_pointer());
        if has_pointer && !self.had_pointer {
            gfx.cursors.invalidate();
        }
        self.had_pointer = has_pointer;
        let kind = match self.game.as_ref() {
            Some(g) if !wants_pointer => g.cursor_kind(&self.input),
            _ => crate::cursor::CursorKind::Arrow,
        };
        gfx.cursors.set(&gfx.window, kind);

        let size = Vec2::new(gfx.config.width as f32, gfx.config.height as f32);
        if let Some(game) = self.game.as_mut() {
            game.update(dt, &self.input, wants_pointer, wants_keys, size, &self.ui);
            game.prepare_render(&gfx.gpu, &mut gfx.scene);
        }

        let frame = match gfx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            other => {
                log::warn!("surface unavailable: {other:?}");
                gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let scene_view = if gfx.config.view_formats.is_empty() {
            frame.texture.create_view(&Default::default())
        } else {
            frame.texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(gfx.config.format.add_srgb_suffix()),
                ..Default::default()
            })
        };
        let mut enc = gfx.gpu.device.create_command_encoder(&Default::default());
        if let Some(game) = self.game.as_ref() {
            let mut cam = game.camera.clone();
            cam.aspect = size.x / size.y;
            let mut globals = Globals::new(cam.view_proj(), cam.eye());
            game.environment(&mut globals);
            gfx.scene.clear = wgpu::Color {
                r: globals.fog_color[0] as f64,
                g: globals.fog_color[1] as f64,
                b: globals.fog_color[2] as f64,
                a: 1.0,
            };
            gfx.scene.render(
                &gfx.gpu,
                &mut enc,
                &gfx.msaa_view,
                Some(&scene_view),
                &gfx.depth,
                &globals,
                &game.meshes,
                &game.batches,
            );
        } else {
            let _ = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("menu clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.08,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        let ppp = full.pixels_per_point;
        let jobs = self.egui_ctx.tessellate(full.shapes, ppp);
        let sd = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gfx.config.width, gfx.config.height],
            pixels_per_point: ppp,
        };
        for (id, deltas) in &full.textures_delta.set {
            for delta in deltas.iter() {
                gfx.egui_renderer
                    .update_texture(&gfx.gpu.device, &gfx.gpu.queue, *id, delta);
            }
        }
        let extra = gfx
            .egui_renderer
            .update_buffers(&gfx.gpu.device, &gfx.gpu.queue, &mut enc, &jobs, &sd);
        {
            let pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut pass = pass.forget_lifetime();
            gfx.egui_renderer.render(&mut pass, &jobs, &sd);
        }
        for id in &full.textures_delta.free {
            gfx.egui_renderer.free_texture(id);
        }
        gfx.gpu
            .queue
            .submit(extra.into_iter().chain(std::iter::once(enc.finish())));
        gfx.window.pre_present_notify();
        gfx.gpu.queue.present(frame);
        self.input.end_frame();

        for a in actions {
            self.apply(a);
        }
    }

    fn apply(&mut self, action: crate::ui::Action) {
        use crate::ui::Action;
        match action {
            Action::StartLevel(rel) => self.start_game(&rel),
            Action::StartEditor(source) => self.start_editor(&source),
            Action::LoadSave(path) => {
                let Some(gfx) = self.gfx.as_mut() else { return };
                match Game::load_save(&self.assets, &path) {
                    Ok(mut g) => {
                        g.settings = self.ui.settings.clone();
                        g.attach_renderer(&gfx.gpu, &mut gfx.scene);
                        self.game = Some(g);
                        self.ui.screen = crate::ui::Screen::InGame;
                    }
                    Err(e) => self.ui.error = Some(format!("{e:#}")),
                }
            }
            Action::QuitToMenu => {
                self.game = None;
                self.ui.screen = crate::ui::Screen::MainMenu;
            }
            Action::ToggleFullscreen => {
                if let Some(gfx) = &self.gfx {
                    let fs = gfx.window.fullscreen().is_some();
                    gfx.window
                        .set_fullscreen(if fs { None } else { Some(Fullscreen::Borderless(None)) });
                    self.ui.settings.fullscreen = !fs;
                    self.ui.settings.save();
                }
            }
            Action::Exit => {
                self.ui.settings.save();
                std::process::exit(0)
            }
        }
    }
}

impl App {
    /// Sets up what a `SRT_HUD_OPEN` test hook asks for.
    fn apply_hook(&mut self, hook: crate::testhook::TestHook, g: &mut Game) {
        use crate::game::Selection;
        use crate::game::defs::StructureKind;
        use crate::testhook::TestHook;
        let near_entrance = |g: &Game| g.world.structures.first().map(|e| e.pos.truncate() + glam::Vec2::new(60.0, 60.0));
        let build = |g: &mut Game, def: Option<usize>| -> Option<crate::game::world::Id> {
            let (def, at) = (def?, near_entrance(g)?);
            g.place_near(def, at)
        };
        match hook {
            TestHook::Menu(nav, tray) => self.ui.hud.open_menu(nav, tray),
            TestHook::Picker(k) => self.ui.picker = Some(crate::ui::picker::Picker::new(crate::ui::picker::PickKind::Buildings(k))),
            TestHook::Stats { window, tab, weather } => {
                use crate::ui::statwin::{StatKind, StatWindow};
                if weather {
                    let def = g.defs.structures.iter().position(|d| d.kind == StructureKind::WeatherStation);
                    build(g, def);
                }
                let kind = [StatKind::Resort, StatKind::Finances, StatKind::Guests, StatKind::Staff, StatKind::Attractions][window % 5];
                let mut w = StatWindow::new(kind);
                if let Some(t) = tab {
                    w.tab = t;
                }
                self.ui.stat_windows.push(w);
            }
            TestHook::Staff => {
                let at = g.world.structures.first().map(|e| e.pos);
                g.hire(crate::game::world::StaffKind::Maintenance, at, None);
                g.selection = g.world.staff.last().map(|s| Selection::Staff(s.id));
            }
            TestHook::Radio | TestHook::Upgrade => {
                let def = if hook == TestHook::Radio {
                    g.defs.structures.iter().position(|d| d.kind == StructureKind::RadioStation)
                } else {
                    g.defs.structures.iter().position(|d| d.upgrade.is_some() && !d.is_upgrade)
                };
                if let Some(id) = build(g, def) {
                    g.selection = Some(Selection::Structure(id));
                }
            }
            TestHook::Night(built) => {
                if built {
                    g.autobuild_starter();
                }
                g.world.daylight.phase = crate::game::daylight::Phase::Night;
            }
            TestHook::Minimap => self.ui.show_minimap = true,
            TestHook::Select => g.selection = g.world.structures.first().map(|s| Selection::Structure(s.id)),
            TestHook::Surface(n) => {
                use crate::game::TerrainTool as T;
                const TOOLS: [T; 8] = [T::Hill, T::Peak, T::Flatten, T::Point, T::Smooth, T::Moguls, T::MakeSnow, T::Groom];
                if let Some(t) = TOOLS.get(n) {
                    g.tool = crate::game::Tool::Terrain(*t);
                }
            }
            TestHook::RunPath(mode) => g.tool = crate::game::Tool::RunPath { mode },
            TestHook::Vehicles => {
                for kind in [StructureKind::SnowmobilePatrol, StructureKind::ChopperPatrol, StructureKind::SnowmobileTour, StructureKind::Groomer, StructureKind::AvalancheControl] {
                    let def = g.defs.structures.iter().position(|d| d.kind == kind);
                    build(g, def);
                }
                let schools: Vec<usize> = (0..g.defs.structures.len()).filter(|&i| g.defs.structures[i].kind == StructureKind::SkiSchool).collect();
                for def in schools {
                    build(g, Some(def));
                }
            }
            TestHook::Follow => {
                g.autobuild_starter();
                let ticks = (g.defs.resort.seconds_per_day / crate::game::sim::TICK) as usize;
                for i in 0..ticks * 2 {
                    crate::game::sim::Sim { defs: &g.defs, w: &mut g.world }.step(crate::game::sim::TICK);
                    if i < ticks {
                        continue;
                    }
                    let skier = g.world.guests.iter().find(|v| matches!(&v.state, crate::game::world::GuestState::Skiing { path, next, .. } if path.len() - next > 20));
                    if let Some(v) = skier {
                        let id = v.id;
                        g.enter_eye(crate::game::Selection::Guest(id), 350.0);
                        break;
                    }
                }
            }
            TestHook::Ride => {
                g.autobuild_starter();
                g.spawn_own_skier();
                // Put down at the top of the first run.
                let at = g.world.runs.first().and_then(|r| r.gates.first().copied()).unwrap_or(glam::Vec3::new(300.0, 300.0, 0.0));
                g.put_down_at(at);
                if let Some(sel) = g.fog_request.take() {
                    g.enter_eye(sel, 350.0);
                }
            }
            TestHook::Rescue => {
                g.autobuild_starter();
                if !g.world.structures.iter().any(|s| g.defs.structures[s.def].kind == StructureKind::SkiPatrol) {
                    let def = g.defs.structures.iter().position(|d| d.kind == StructureKind::SkiPatrol);
                    log::info!("rescue hook: patrol hut {:?}", build(g, def));
                }
                let ticks = (g.defs.resort.seconds_per_day / crate::game::sim::TICK) as usize;
                for i in 0..ticks * 2 {
                    crate::game::sim::Sim { defs: &g.defs, w: &mut g.world }.step(crate::game::sim::TICK);
                    let skiing = g.world.guests.iter().any(|v| matches!(v.state, crate::game::world::GuestState::Skiing { .. }));
                    if i >= ticks && skiing {
                        break;
                    }
                }
                let hut = g
                    .world
                    .structures
                    .iter()
                    .find(|s| g.defs.structures[s.def].category == crate::game::defs::Category::SkiPatrol)
                    .map(|s| s.pos);
                if let Some(hut) = hut {
                    let v = g
                        .world
                        .guests
                        .iter_mut()
                        .filter(|v| matches!(v.state, crate::game::world::GuestState::Skiing { .. } | crate::game::world::GuestState::Walking { .. }) && v.pos.z > -100.0)
                        .min_by(|a, b| a.pos.distance(hut).total_cmp(&b.pos.distance(hut)));
                    if let Some(v) = v {
                        v.state = crate::game::world::GuestState::Injured;
                        v.needs.health = 0.1;
                        // Close by, to watch the whole rescue.
                        let at = g.world.ground(hut.truncate() + glam::Vec2::new(40.0, 30.0));
                        let v = g.world.guests.iter_mut().find(|v| v.state == crate::game::world::GuestState::Injured).expect("just hurt");
                        v.pos = at;
                        g.camera.target = at;
                        g.camera.step = 0;
                        g.camera.show_step();
                        log::info!("rescue hook: guest hurt at {at}");
                    }
                }
            }
            TestHook::Paths => {
                if let Some(e) = g.world.structures.first().map(|s| s.pos) {
                    let s = g.world.level().spacing;
                    let (ex, ey) = ((e.x / s) as i32, (e.y / s) as i32);
                    // A cross with a long arm, and a corner off to the side.
                    let cells = [(8, 4), (8, 5), (8, 6), (8, 7), (8, 8), (7, 6), (9, 6), (10, 6), (11, 6), (11, 5), (11, 4), (5, 8), (5, 9), (4, 9)];
                    let n = g.world.level().cells_per_side() as i32;
                    for (dx, dy) in cells {
                        let (x, y) = (ex + dx, ey + dy);
                        if x >= 0 && y >= 0 && x < n && y < n {
                            g.world.level_mut().cell_mut(x as usize, y as usize).set_path(Some(0));
                        }
                    }
                    g.world.scenery_dirty = true;
                }
            }
            TestHook::Dirt => {
                if let Some(e) = g.world.structures.first().map(|s| s.pos) {
                    let s = g.world.level().spacing;
                    for k in 0..12 {
                        let (dx, dy) = ((k % 4) as f32 - 1.5, -2.0 - (k / 4) as f32);
                        let mid = glam::Vec2::new(((e.x / s).floor() + dx + 0.5) * s, ((e.y / s).floor() + dy + 0.5) * s);
                        let pos = g.world.ground(mid);
                        g.world.litter.push(crate::game::world::Litter { pos, yellow: k % 3 == 2 });
                    }
                }
            }
            TestHook::Starter(pick) => {
                g.autobuild_starter();
                g.selection = match pick {
                    crate::testhook::StarterPick::Run => g.world.runs.first().map(|r| Selection::Run(r.id)),
                    crate::testhook::StarterPick::Lift => g.world.lifts.first().map(|l| Selection::Lift(l.id)),
                };
            }
            TestHook::Escape | TestHook::Save => {
                self.ui.show_menu = true;
                self.ui.save_open = hook == TestHook::Save;
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_none() {
            if let Err(e) = self.init_gfx(el) {
                eprintln!("failed to initialize graphics: {e:#}");
                el.exit();
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Some(gfx) = self.gfx.as_mut() {
            let _ = gfx.egui_state.on_window_event(&gfx.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => {
                self.ui.settings.save();
                el.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gfx) = self.gfx.as_mut() {
                    // Remembered for the next run, saved on leaving.
                    if gfx.window.fullscreen().is_none() {
                        let l = size.to_logical::<f64>(gfx.window.scale_factor());
                        self.ui.settings.window = Some((l.width.round() as u32, l.height.round() as u32));
                    }
                    gfx.config.width = size.width.max(1);
                    gfx.config.height = size.height.max(1);
                    gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                    let (m, d) = Self::make_targets(&gfx.gpu, &gfx.config);
                    gfx.msaa_view = m;
                    gfx.depth = d;
                    gfx.cursors.rescale(el, ui_scale(gfx.config.width, gfx.config.height));
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !event.repeat {
                                self.input.pressed.push(code);
                            }
                            self.input.keys.insert(code);
                            if code == KeyCode::F11 {
                                self.apply(crate::ui::Action::ToggleFullscreen);
                            }
                        }
                        ElementState::Released => {
                            self.input.keys.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::CursorEntered { .. } | WindowEvent::CursorLeft { .. } | WindowEvent::Focused(_) => {
                match &event {
                    WindowEvent::CursorEntered { .. } => self.input.inside = true,
                    WindowEvent::CursorLeft { .. } | WindowEvent::Focused(false) => self.input.inside = false,
                    _ => {}
                }
                if let Some(gfx) = self.gfx.as_mut() {
                    gfx.cursors.invalidate();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = Vec2::new(position.x as f32, position.y as f32);
                self.input.mouse_delta += p - self.input.mouse;
                self.input.mouse = p;
                self.input.inside = true;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        if down {
                            self.input.left_clicked = true;
                        }
                        self.input.left_down = down;
                    }
                    MouseButton::Right => {
                        if down {
                            self.input.right_clicked = true;
                            self.input.right_press = self.input.mouse;
                        } else if self.input.right_down && self.input.mouse.distance(self.input.right_press) < 5.0 {
                            self.input.right_tap = true;
                        }
                        self.input.right_down = down;
                    }
                    MouseButton::Middle => self.input.middle_down = down,
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.scroll += match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(gfx) = &self.gfx {
            gfx.window.request_redraw();
        }
    }
}

/// Unused helper kept for parity with picking code in `game`.
pub fn screen_to_ndc(p: Vec2, size: Vec2) -> Vec2 {
    Vec2::new(p.x / size.x * 2.0 - 1.0, 1.0 - p.y / size.y * 2.0)
}

pub fn ground_hit(cam: &Camera, level: &crate::formats::level::Level, ndc: Vec2) -> Option<Vec3> {
    let (o, d) = cam.ray(ndc);
    let ext = level.extent();
    // Start where the ray comes down to the highest ground: the
    // orthographic view's rays begin far behind the eye.
    let top = level.height_range().1 + 1.0;
    let mut t = if d.z < 0.0 && o.z > top { (o.z - top) / -d.z } else { 0.0 };
    let step = level.spacing * 0.25;
    let mut prev_above = true;
    let mut prev_inside = false;
    for _ in 0..20000 {
        let p = o + d * t;
        if p.z < -100.0 {
            break;
        }
        let inside = p.x >= 0.0 && p.y >= 0.0 && p.x <= ext && p.y <= ext;
        let above = !inside || p.z > level.height_at(p.x, p.y);
        if inside && !above && !prev_inside {
            // Entered through the edge wall, which is not ground.
            return None;
        }
        prev_inside = inside;
        if inside && !above && prev_above {
            // Refine by bisection.
            let (mut a, mut b) = (t - step, t);
            for _ in 0..20 {
                let m = (a + b) * 0.5;
                let q = o + d * m;
                if q.z > level.height_at(q.x, q.y) {
                    a = m;
                } else {
                    b = m;
                }
            }
            let q = o + d * b;
            return Some(Vec3::new(q.x, q.y, level.height_at(q.x, q.y)));
        }
        prev_above = above;
        t += step;
    }
    None
}

/// Physical pixels per pixel of the interface's 1024x768 layout.
fn ui_scale(width: u32, height: u32) -> f32 {
    (height as f32 / 768.0).min(width as f32 / 1024.0)
}
