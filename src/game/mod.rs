//! Game session: world + definitions + tools + rendering glue.

pub mod autobuild;
pub mod daylight;
pub mod avalanche;
pub mod challenge;
pub mod defs;
pub mod editor;
pub mod lamplight;
pub mod nav;
pub mod sim;
pub mod thought;
pub mod weather;
pub mod wildlife;
pub mod world;

mod build;
mod control;
mod effects;
mod input;
mod lifts;
mod placement;
mod render;
mod run_path;
mod terrain_tools;

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use winit::keyboard::KeyCode;

use crate::app::{Input, ground_hit, screen_to_ndc};
use crate::assets::Assets;
use crate::camera::Camera;
use crate::formats::level::{FOLIAGE_MESHES, Level};
use crate::formats::mesh::AttachPoint;
use crate::render::scene::{Batches, MeshCache, MeshId, SceneRenderer};
use crate::render::terrain::CellMark;
use crate::render::{Globals, Gpu};
use crate::audio::Sound;
use crate::text::Text;
use defs::{Category, Defs, Requirement, StructureKind};
use sim::Sim;
use world::*;
pub use run_path::{RunLayout, RunNode, RunPhase};

#[derive(Debug, Clone, PartialEq)]
pub enum Tool {
    Inspect,
    Structure { def: usize, rot: u8 },
    Lift { def: usize, start: Option<Vec3> },
    Run { def: usize, gates: Vec<Vec3> },
    Park { def: usize, start: Option<Vec3> },
    Deco { def: usize, rot: f32 },
    Terrain(TerrainTool),
    /// Plant trees at a density (1 sparse ..= 3 dense) or clear them (0,
    /// the chainsaw) over a rectangle dragged out on the grid.
    Trees { density: u8 },
    /// Lays out and cuts a ski run (see `run_path`): mode 0 follows the
    /// ground, 1 only cuts, 2 cuts and fills.
    RunPath { mode: u8 },
    Bulldoze,
    /// Lays footpaths of a type (0..4) cell by cell while the button is
    /// held, or takes them up (`None`, the eraser).
    Path { kind: Option<u8> },
    /// The original survey: the first click picks a terrain point, then the
    /// distance and slope from it to the pointer are shown; the second
    /// click ends the survey.
    Survey { start: Option<Vec3> },
}

/// The original surface tools. Shift lowers instead of raising.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerrainTool {
    /// Rounded rise.
    Hill,
    /// Pointed rise.
    Peak,
    /// Pulls the area toward the height first clicked.
    Flatten,
    /// Moves the nearest single height point.
    Point,
    Smooth,
    Moguls,
    /// Improves snow conditions (removes wear).
    MakeSnow,
    /// Grooms the snow, removing moguls.
    Groom,
}

impl TerrainTool {
    /// The height tools, which keep their rectangle to reshape it.
    pub fn shapes(self) -> bool {
        matches!(self, TerrainTool::Hill | TerrainTool::Peak | TerrainTool::Flatten | TerrainTool::Point)
    }
}

/// A rectangle of cells picked with a surface tool (corners inclusive),
/// and what the pointer is doing with it (0x423b40).
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainSel {
    pub from: (i32, i32),
    pub to: (i32, i32),
    pub phase: SelPhase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelPhase {
    /// The button is down and the far corner follows the pointer.
    Dragging,
    /// Let go: the rectangle stays picked.
    Picked,
    /// Pressed on one of its height points: dragging reshapes it about
    /// that point, `center` being where it lies across the rectangle.
    Shaping { point: (usize, usize), center: Vec2, last_y: f32 },
}

impl TerrainSel {
    /// The lowest and highest cells.
    pub fn rect(&self) -> ((i32, i32), (i32, i32)) {
        (
            (self.from.0.min(self.to.0), self.from.1.min(self.to.1)),
            (self.from.0.max(self.to.0), self.from.1.max(self.to.1)),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Selection {
    Park(Id),
    Structure(Id),
    Lift(Id),
    Run(Id),
    Guest(Id),
    Staff(Id),
}

pub use control::{Control, Eye};

/// Why something can't be built or done. The original says only a few of
/// these, at the pointer; the rest it shows with red cells or not at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    TooExpensive,
    TooLong,
    TooShort,
    OffMap,
    Occupied,
    TooSteep,
    WrongWay,
    TooFewGates,
    GateNotLower,
    Entrance,
    Indoors,
}

impl Problem {
    /// The words for it, if the original has any.
    pub fn text(self) -> Option<Text> {
        Some(match self {
            Problem::TooExpensive => Text::GlobalBuildTooExpensive,
            Problem::TooLong => Text::LcmTooLong,
            Problem::TooShort => Text::LcmTooShort,
            _ => return None,
        })
    }
}

pub struct Placement {
    pub valid: bool,
    pub reason: Option<Problem>,
    pub cost: f32,
    pub model: Mat4,
}

/// The original outlines the selected object, and lift pieces being
/// placed, with a one-pixel box around their on-screen bounds (0x439630 →
/// 0x438cf0): green, or red when a lift can't go there (0x49dec0,
/// 0x4a2710). Buildings being placed get no box; their footprint cells
/// are tinted instead (`CellMark`).
pub struct Highlight {
    /// The model's vertices: the box is their extent on screen, as the
    /// original's is its pre-rendered sprite's frame.
    pub points: std::sync::Arc<[Vec3]>,
    pub model: Mat4,
    pub red: bool,
}

/// How long a money text takes to reach the corner. Estimated from three
/// placements in the original (it eases in over about four seconds).
pub const FLOATER_SECS: f32 = 4.0;

/// Lift mesh animation rate in frames per second of simulated time.
const LIFT_FPS: f32 = 10.0;

/// Cable color: the original draws cables as unlit lines in 0x4f404040,
/// which shows as solid 0x404040 (alpha unused); here in linear terms.
const CABLE: [f32; 4] = [0.0513, 0.0513, 0.0513, 1.0];

fn highlight(meshes: &MeshCache, id: MeshId, model: Mat4, red: bool) -> Option<Highlight> {
    let m = meshes.meshes.get(id)?.as_ref()?;
    Some(Highlight {
        points: m.points.clone(),
        model,
        red,
    })
}

/// Money text flying from where it was spent (see `Game::after_build`).
pub struct Floater {
    pub at: Vec3,
    pub text: String,
    pub age: f32,
}

const PUFF_LIFE: f32 = 2.5;

struct Puff {
    pos: Vec3,
    vel: Vec3,
    age: f32,
    grow: f32,
}

impl Puff {
    fn size(&self) -> f32 {
        4.0 + self.grow * self.age * 2.0
    }
}

/// A `menu_sound` from the ini files: lifts give full paths, buildings
/// paths under `data/sounds`.
fn sound_path(s: &str) -> String {
    let s = s.replace('\\', "/");
    if s.is_empty() || s.to_ascii_lowercase().starts_with("data/") {
        s
    } else {
        format!("data/sounds/{s}")
    }
}

pub struct Game {
    pub assets: Arc<Assets>,
    pub defs: Arc<Defs>,
    pub world: World,
    pub camera: Camera,
    pub speed: f32,
    pub paused: bool,
    pub tool: Tool,
    pub selection: Option<Selection>,
    pub hover: Option<Vec3>,
    pub placement: Option<Placement>,
    /// Terrain cells tinted under the thing being placed.
    pub cell_marks: Vec<((usize, usize), CellMark)>,
    pub floaters: Vec<Floater>,
    /// Avalanche snow clouds (see `puffs`).
    puffs: Vec<Puff>,
    seen_avalanches: u32,
    rumble: Option<crate::audio::Handle>,
    /// Ticks since each building last made its sound.
    building_quiet: std::collections::HashMap<Id, f32>,
    groomer_loops: std::collections::HashMap<Id, crate::audio::Handle>,
    pub highlights: Vec<Highlight>,
    pub meshes: MeshCache,
    pub batches: Batches,
    terrain_dirty: bool,
    /// Which footpaths were under deep snow when the terrain was built.
    snowy_paths: Vec<(usize, usize)>,
    /// Frames until they are looked at again.
    snow_check: u32,
    foliage: Vec<(MeshId, Mat4)>,
    foliage_dirty: bool,
    sim_accum: f32,
    /// The surface tools' picked rectangle.
    pub terrain_sel: Option<TerrainSel>,
    /// A run being laid out with the Path Skirun tools.
    pub run_layout: RunLayout,
    /// Draws cell lines on the terrain.
    pub show_grid: bool,
    /// First cell of a tree rectangle being dragged.
    pub tree_drag: Option<(i32, i32)>,
    pub save_path: Option<std::path::PathBuf>,
    pub last_error: Option<(Problem, f32)>,
    /// The follow view (see `control`).
    pub eye: Option<Eye>,
    /// Someone to follow once a fog setting is chosen: the player's own
    /// skier, just put down.
    pub fog_request: Option<Selection>,
    pub control: Option<Control>,
    /// The guest or employee the player has picked up, if any.
    pub carrying: Option<Selection>,
    /// Night lights to shade with this frame.
    lamps: Vec<crate::render::Lamp>,
    /// What the ground's lamp light was last worked out from, so it is
    /// only redone when lamps, buildings or ground change.
    ground_lamps_key: u64,
    /// The worn-snow digest the terrain was last built with.
    worn: u64,
    /// The steered skier's skiing or walking loop.
    ski_loop: Option<(Sound, crate::audio::Handle)>,
    pub settings: crate::settings::Settings,
}

/// A guest's body mesh.
fn guest_mesh(g: &world::Guest) -> String {
    const SKI_COLORS: [&str; 6] = ["blue", "red", "green", "yellow", "orange", "olive"];
    const BOARD_COLORS: [&str; 4] = ["blue", "green", "orange", "olive"];
    if g.boarder {
        format!("data/scenery/people/boarders/boarder_{}.mesh", BOARD_COLORS[g.color as usize % 4])
    } else {
        format!("data/scenery/people/skiers/skier_{}.mesh", SKI_COLORS[g.color as usize % 6])
    }
}

/// The skier clip for a guest's state. The original's choice is not yet
/// traced; these are picked from the clip names (`0x656820` lists them).
fn guest_clip(g: &world::Guest) -> &'static str {
    match &g.state {
        GuestState::Rescued => "FacePlantStill",
        GuestState::Walking { .. } | GuestState::Leaving => {
            if g.has_equipment { "WalkCarrySkis" } else { "WalkNoSkis" }
        }
        GuestState::Queued { .. } | GuestState::AtDoor { .. } => "InLineIdle",
        GuestState::Riding { .. } => "GlideSlow",
        GuestState::Crashed { .. } => "YardSale",
        // Held up by the player, flailing (0x452f99).
        GuestState::Held => {
            if g.has_equipment { "Flail" } else { "FlailNoSkis" }
        }
        GuestState::Injured => "FacePlantStill",
        // Poling, sidestepping up, or gliding by speed: over 4/9 a tick
        // fast, over 8/27 medium (0x48f711 clips 3, 4 and 0-2).
        GuestState::Skiing { .. } => match g.ski_mode {
            world::SkiMode::Pole => "DoublePole",
            world::SkiMode::Climb => "SkiWalk",
            world::SkiMode::Ski => match g.speed * sim::TICK {
                v if v > 0.444_444_45 => "GlideFast",
                v if v > 0.296_296_3 => "GlideMedium",
                _ => "GlideSlow",
            },
        },
        GuestState::Controlled => g.ctl_clip.unwrap_or("GlideSlow"),
        _ => "IdleNoSkis",
    }
}

/// The boarder equivalent of a skier clip (`0x656914`, same order). The
/// patrol mesh's idle clips are spelled "Skies".
fn boarder_clip(skier: &str) -> String {
    match skier {
        "DoublePole" => "Boarder_Skate".into(),
        "YardSale" => "Boarder_Crash2".into(),
        "IdleCarrySkies" => "Boarder_IdleCarrySkis".into(),
        "IdleNoSkies" => "Boarder_IdleNoSkis".into(),
        other => format!("Boarder_{other}"),
    }
}

impl Game {
    pub fn new_from_level(assets: &Arc<Assets>, level_rel: &str, challenge: Option<usize>) -> Result<Game> {
        let defs = Arc::new(Defs::load(assets)?);
        let path = assets
            .resolve_level(level_rel)
            .with_context(|| format!("level {level_rel} not found"))?;
        let level = Level::load(&path)?;
        let name = Path::new(level_rel)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Game::on_level(assets, defs, level, name, level_rel.to_string(), challenge))
    }

    /// A map editor session (the original's CSkiEditLayer): the map from
    /// the Map Editor screen, nothing running, money no object.
    pub fn new_editor(assets: &Arc<Assets>, source: &editor::MapSource) -> Result<Game> {
        let defs = Arc::new(Defs::load(assets)?);
        let (level, name) = editor::make_map(assets, source)?;
        let rel = match source {
            editor::MapSource::Level(rel) => rel.clone(),
            _ => String::new(),
        };
        let mut game = Game::on_level(assets, defs, level, name, rel, None);
        game.world.editor = true;
        game.world.sandbox = true;
        game.world.cash = 10_000_000.0;
        game.paused = true;
        Ok(game)
    }

    fn on_level(assets: &Arc<Assets>, defs: Arc<Defs>, level: Level, name: String, level_rel: String, challenge: Option<usize>) -> Game {
        let n = level.cells_per_side();
        let ext = level.extent();
        let world = World {
            level_name: name,
            level_file: level_rel,
            level: Some(level),
            level_bytes: Vec::new(),
            next_id: 0,
            structures: Vec::new(),
            lifts: Vec::new(),
            runs: Vec::new(),
            parks: Vec::new(),
            decos: Vec::new(),
            guests: Vec::new(),
            staff: Vec::new(),
            litter: Vec::new(),
            groom_jobs: Vec::new(),
            snow_jobs: Vec::new(),
            tick: 0,
            walk_routes: Vec::new(),
            occupied: vec![None; n * n],
            lift_cells: Vec::new(),
            powered_cells: Vec::new(),
            power_key: None,
            leave_counts: Vec::new(),
            avalanche_clock: 0.0,
            avalanche_risk: Vec::new(),
            avalanches: Vec::new(),
            blasts: Vec::new(),
            avalanches_started: 0,
            daylight: Default::default(),
            animals: Vec::new(),
            wildlife_spawned: false,
            controlled: None,
            sounds: Vec::new(),
            scenery_dirty: false,
            cash: defs.resort.initial_cash.max(10000.0) * 5.0,
            day: 0,
            hour: 6.0,
            weather: Weather::default(),
            today: DayStats::default(),
            history: Vec::new(),
            rating: Rating::default(),
            max_guests: 10,
            messages: Vec::new(),
            // Set from the entrance's item_cost when it is placed.
            ticket_price: 10.0,
            challenge,
            challenge_state: challenge.map(challenge::ChallengeState::new),
            total_guests: 0,
            rng: 0x9E37_79B9_7F4A_7C15 ^ (std::process::id() as u64) << 7,
            // Landscaping is paid for in every game; only the map editor
            // has it free (0x423b40 skips the charge when +0x1ac is set,
            // which only the editor's map load sets, 0x4182f0).
            sandbox: false,
            editor: false,
            map_gen: Default::default(),
            options: Default::default(),
        };
        let mut game = Game::from_world(assets, defs, world);
        game.camera = Camera::new(Vec3::new(ext * 0.5, ext * 0.3, 0.0));
        game.camera.target.z = game.world.level().height_at(game.camera.target.x, game.camera.target.y);
        game.place_entrance();
        if let Some(n) = challenge {
            challenge::setup(n, &mut game.world);
        }
        game
    }

    fn from_world(assets: &Arc<Assets>, defs: Arc<Defs>, world: World) -> Game {
        let ext = world.level().extent();
        Game {
            assets: assets.clone(),
            defs,
            world,
            camera: Camera::new(Vec3::new(ext * 0.5, ext * 0.5, 0.0)),
            speed: 1.0,
            paused: false,
            tool: Tool::Inspect,
            selection: None,
            hover: None,
            placement: None,
            cell_marks: Vec::new(),
            floaters: Vec::new(),
            puffs: Vec::new(),
            seen_avalanches: 0,
            rumble: None,
            building_quiet: std::collections::HashMap::new(),
            groomer_loops: std::collections::HashMap::new(),
            highlights: Vec::new(),
            meshes: MeshCache::default(),
            batches: Batches::default(),
            terrain_dirty: true,
            snowy_paths: Vec::new(),
            snow_check: 0,
            foliage: Vec::new(),
            foliage_dirty: true,
            sim_accum: 0.0,
            terrain_sel: None,
            run_layout: RunLayout::default(),
            show_grid: false,
            tree_drag: None,
            save_path: None,
            last_error: None,
            eye: None,
            fog_request: None,
            control: None,
            carrying: None,
            lamps: Vec::new(),
            ground_lamps_key: 0,
            worn: 0,
            ski_loop: None,
            settings: crate::settings::Settings::default(),
        }
    }

    /// Puts the resort entrance in the corner the levels reserve for it:
    /// every shipped level flags cells (0,0)–(8,8) (0x40) for the 8×8
    /// entrance building.
    fn place_entrance(&mut self) {
        let Some(def) = self
            .defs
            .structures
            .iter()
            .position(|d| d.category == Category::Entrance)
        else {
            return;
        };
        {
            let cell = (0, 0);
            // Its arched front faces out of the corner, as in the original.
            let rot: u8 = 1;
            self.place_structure(def, cell, rot, true);
            if !self.world.structures.is_empty() {
                // The original opens looking at the map's corner, from
                // straight across the diagonal (yaw 5π/4).
                let lv = self.world.level();
                self.camera.target = Vec3::new(0.0, 0.0, lv.height(0, 0));
                self.camera.yaw = -3.0 * std::f32::consts::FRAC_PI_4;
                // The original opens at its middle zoom step (0x432518).
                self.camera.step = crate::camera::OPENING_STEP;
                self.camera.show_step();
                self.camera.pitch = crate::camera::OVERVIEW_PITCH;
            }
        }
    }

    /// The map editor's generators remake the map: the entrance goes
    /// down again on the new ground, and the view stays where it was, on
    /// the new ground.
    pub fn editor_remap(&mut self, edit: impl FnOnce(&mut Level, &mut dyn FnMut() -> u32)) {
        let mut rand = editor::crand();
        edit(self.world.level_mut(), &mut rand);
        self.world.structures.clear();
        self.world.occupied.fill(None);
        let camera = self.camera.clone();
        self.place_entrance();
        self.camera = camera;
        let t = self.camera.target;
        self.camera.target.z = self.world.level().height_at(t.x, t.y);
        self.terrain_dirty = true;
        self.foliage_dirty = true;
    }

    pub fn load_save(assets: &Arc<Assets>, path: &Path) -> Result<Game> {
        let defs = Arc::new(Defs::load(assets)?);
        let text = std::fs::read(path)?;
        let mut world: World = serde_json::from_slice(&text).context("reading save")?;
        world.level = Some(Level::parse(&world.level_bytes)?);
        world.level_bytes.clear();
        // Older saves had every Instant Action game free to landscape.
        world.sandbox = world.editor;
        // Saves from before worth was kept: count everything as new.
        for s in world.structures.iter_mut().filter(|s| s.worth.base == 0.0) {
            s.worth = Worth::new(defs.structures[s.def].cost);
        }
        for l in world.lifts.iter_mut().filter(|l| l.worth.base == 0.0) {
            let d = &defs.lifts[l.def];
            l.worth = Worth::new(d.base_cost + d.cost_per_pole * l.poles.len() as f32);
        }
        for r in world.runs.iter_mut().filter(|r| r.worth.base == 0.0) {
            r.worth = Worth::new(defs.runs[r.def].cost * r.gates.len() as f32);
        }
        for p in world.parks.iter_mut().filter(|p| p.worth.base == 0.0) {
            p.worth = Worth::new(defs.parks[p.def].cost);
        }
        // Saves from before staff had a home: instructors teach at the
        // nearest ski school.
        let schools: Vec<(Id, Vec3)> = world
            .structures
            .iter()
            .filter(|s| defs.structures[s.def].category == Category::SkiSchool)
            .map(|s| (s.id, s.pos))
            .collect();
        for st in world.staff.iter_mut().filter(|s| s.home.is_none() && s.kind == StaffKind::Instructor) {
            st.home = schools.iter().min_by(|a, b| a.1.distance(st.pos).total_cmp(&b.1.distance(st.pos))).map(|s| s.0);
        }
        let mut g = Game::from_world(assets, defs, world);
        g.refresh_lifts();
        // Groomer shops from before the snowmaking machine get theirs.
        let shops: Vec<(Id, Vec3)> = g
            .world
            .structures
            .iter()
            .filter(|s| g.defs.structures[s.def].kind == StructureKind::Groomer)
            .filter(|s| !g.world.staff.iter().any(|st| st.kind == StaffKind::Snowmaker && st.home == Some(s.id)))
            .map(|s| (s.id, s.pos))
            .collect();
        for (id, pos) in shops {
            g.hire(StaffKind::Snowmaker, Some(pos), Some(id));
        }
        // Wages as the staff are paid now, whatever the save says.
        for i in 0..g.world.staff.len() {
            g.world.staff[i].wage = g.wage_of(g.world.staff[i].kind, g.world.staff[i].home);
        }
        g.save_path = Some(path.to_path_buf());
        Ok(g)
    }

    pub fn save(&mut self, path: &Path) -> Result<()> {
        self.world.level_bytes = self.world.level().write();
        let data = serde_json::to_vec(&self.world);
        self.world.level_bytes.clear();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, data?)?;
        self.save_path = Some(path.to_path_buf());
        Ok(())
    }

    pub fn attach_renderer(&mut self, gpu: &Gpu, scene: &mut SceneRenderer) {
        self.refresh_lifts();
        scene.terrain.rebuild(gpu, self.world.level(), &self.deep_snow());
        self.terrain_dirty = false;
    }

    fn sim(&mut self) -> Sim<'_> {
        Sim {
            defs: &self.defs,
            w: &mut self.world,
        }
    }

    // ------------------------------------------------------------ building

    /// Plays one of the fixed effects once.
    fn sfx(&self, sound: Sound) {
        crate::audio::sfx(&self.assets, sound);
    }

    /// Plays an effect named by the game data (a building's sound).
    fn sfx_file(&self, rel: &str) {
        if !rel.is_empty() {
            crate::audio::play_ui(&self.assets, rel);
        }
    }

    fn error(&mut self, p: Problem) {
        self.last_error = Some((p, 3.0));
    }

    // ----------------------------------------------------------- per frame

    /// Whether ground at a height is under deep snow (0x40aa40's top
    /// variant), for the snowy footpaths.
    fn deep_snow(&self) -> impl Fn(f32) -> bool {
        let (wx, range) = (self.world.weather.clone(), self.world.level().height_range());
        move |z| wx.snow_variant(z, range) >= 2
    }

    /// The footpath cells under deep snow now.
    fn find_snowy_paths(&self) -> Vec<(usize, usize)> {
        let lv = self.world.level();
        let deep = self.deep_snow();
        let n = lv.cells_per_side();
        (0..n)
            .flat_map(|y| (0..n).map(move |x| (x, y)))
            .filter(|&(x, y)| lv.cell(x, y).path().is_some() && deep(lv.height(x, y)))
            .collect()
    }

    pub fn avalanche_danger(&self) -> f32 {
        avalanche::avalanche_danger(&self.world)
    }

    /// Sets off an avalanche now (the Ctrl+Shift+X debug key).
    pub fn force_avalanche(&mut self) {
        self.sim().force_avalanche();
    }
}

fn attach_point(pts: &[(AttachPoint, Vec3)], h: AttachPoint) -> Option<Vec3> {
    pts.iter().find(|(n, _)| *n == h).map(|(_, p)| *p)
}

