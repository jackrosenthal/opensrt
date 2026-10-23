//! In-game interface from the original art. The 1024×768 navigation frame
//! is scaled to the window height and stretched across its width; the
//! readouts, buttons and extending menus use the original layout tables.

use egui::{Color32, FontFamily, FontId, Pos2, Rect as ERect, Sense, Vec2};

use super::art::{self, ArtFile, Group};
use super::classic::{Art, Canvas, TextStyle};
use super::fonts::{ARIAL, ARIAL_BOLD};
use crate::assets::Assets;
use crate::formats::eng::Strings;
use crate::game::{Game, TerrainTool};
use crate::text::{SurveyRating, Text};

const NAV: ArtFile = art::nav_bar::FILE;
const CONSTRUCTION: ArtFile = art::construction_ext::FILE;
const TERRAIN: ArtFile = art::terrain_ext::FILE;
const FINANCE: ArtFile = art::finance_ext::FILE;
const BUILD_SUB: ArtFile = art::build_sub::FILE;
const LIFTS_SUB: ArtFile = art::lifts_sub::FILE;
const FLUFF_SUB: ArtFile = art::fluff_sub::FILE;
const SKIRUN_SUB: ArtFile = art::skirun_sub::FILE;
const SURFACE_SUB: ArtFile = art::surface_sub::FILE;
const TREES_SUB: ArtFile = art::trees_sub::FILE;

/// Width and height of the layout the frame art was drawn for.
const LAYOUT: Vec2 = Vec2::new(1024.0, 768.0);

/// Font 0 of the original table (money, ticker).
const FONT0: TextStyle = TextStyle {
    size: 14.0,
    bold: true,
    color: Color32::from_rgb(0x00, 0x00, 0x99),
};
/// Font 1 of the original table (week/day, occupancy).
const FONT1: TextStyle = TextStyle {
    size: 11.0,
    bold: true,
    color: Color32::from_rgb(0x0a, 0x4b, 0xce),
};

/// Thresholds on the 0..1000 resort score for half and full stars.
const STAR_HALF: [f32; 5] = [50.0, 200.0, 400.0, 600.0, 800.0];
const STAR_FULL: [f32; 5] = [125.0, 300.0, 500.0, 700.0, 900.0];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nav {
    Grid,
    Build,
    Terrain,
    Finances,
    Main,
    ZoomIn,
    ZoomOut,
    MapView,
    Pause,
    FirstPerson,
    RotateCw,
    RotateCcw,
    Survey,
}

/// The 1024×768 navigation table (group, button, x, y, tooltip).
const NAV_BUTTONS: [(Group, Nav, f32, f32, Text); 13] = [
    (art::nav_bar::BUILD, Nav::Build, 179.0, 22.0, Text::NavBarBuild),
    (art::nav_bar::TERRAIN, Nav::Terrain, 237.0, 22.0, Text::NavBarTerrain),
    (art::nav_bar::FINANCES, Nav::Finances, 295.0, 22.0, Text::NavBarFinances),
    (art::nav_bar::MAIN, Nav::Main, 684.0, 22.0, Text::NavBarMainMenu),
    (art::nav_bar::ZOOM_IN, Nav::ZoomIn, 8.0, 719.0, Text::NavBarZoomIn),
    (art::nav_bar::ZOOM_OUT, Nav::ZoomOut, 44.0, 719.0, Text::NavBarZoomOut),
    (art::nav_bar::FIRST_PERSON, Nav::FirstPerson, 801.0, 22.0, Text::NavBarFirstPerson),
    (art::nav_bar::PAUSE, Nav::Pause, 743.0, 22.0, Text::NavBarPause),
    (art::nav_bar::LOG_GRID, Nav::Grid, 119.0, 22.0, Text::NavBarGrid),
    (art::nav_bar::ROTATE_CW, Nav::RotateCw, 82.0, 719.0, Text::NavBarRotateCw),
    (art::nav_bar::ROTATE_CCW, Nav::RotateCcw, 119.0, 719.0, Text::NavBarRotateCcw),
    (art::nav_bar::MAP_VIEW, Nav::MapView, 981.0, 721.0, Text::NavBarToggleMinimap),
    (art::nav_bar::LOG_SURVEY, Nav::Survey, 859.0, 22.0, Text::NavBarSurvey),
];

/// The map editor's 1024×768 navigation table (CTerrainEditNavigationBar,
/// 0x472b70, table 0x63b458): terrain, main menu, the view controls, the
/// grid and the survey.
const EDITOR_NAV_BUTTONS: [(Group, Nav, f32, f32, Text); 9] = [
    (art::nav_bar::TERRAIN, Nav::Terrain, 532.0, 7.0, Text::TenTerrain),
    (art::nav_bar::MAIN, Nav::Main, 395.0, 7.0, Text::TenMain),
    (art::nav_bar::ZOOM_IN, Nav::ZoomIn, 426.0, 719.0, Text::TenZoomIn),
    (art::nav_bar::ZOOM_OUT, Nav::ZoomOut, 461.0, 719.0, Text::TenZoomOut),
    (art::nav_bar::LOG_GRID, Nav::Grid, 448.0, 7.0, Text::TenGrid),
    (art::nav_bar::ROTATE_CW, Nav::RotateCw, 496.0, 719.0, Text::TenRotateCw),
    (art::nav_bar::ROTATE_CCW, Nav::RotateCcw, 530.0, 719.0, Text::TenRotateCcw),
    (art::nav_bar::MAP_VIEW, Nav::MapView, 577.0, 721.0, Text::TenMapView),
    (art::nav_bar::LOG_SURVEY, Nav::Survey, 585.0, 7.0, Text::TenSurvey),
];

/// What a menu item does when clicked.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Command {
    /// Opens the building picker on one of the eight categories.
    Buildings(usize),
    Lifts,
    Jumps,
    RunMarkers,
    Objects,
    Paths,
    RunPath(u8),
    Surface(TerrainTool),
    Trees(u8),
    ResortStats,
    Finances,
    GuestStats,
    StaffStats,
    AttractionStats,
}

/// One button of an extending menu: either a plain button or a log with
/// its own tray.
struct Item {
    group: Group,
    tip: Text,
    action: Option<Command>,
    tray: Option<Ext>,
    /// Group drawn over the log while its tray is open.
    overlay: Option<Group>,
}

/// A menu that slides out of its parent log, as `CUIExtendingMenu` does:
/// children are added two ticks apart and every tick all of them move by
/// the step for the current state.
struct Ext {
    rel: ArtFile,
    /// Direction children slide in: the nav bar's menus open downwards,
    /// their logs' trays to the right.
    dir: (f32, f32),
    /// Only trays have an end cap.
    has_endcap: bool,
    /// Items in visual order, nearest to the parent first.
    items: Vec<Item>,
    /// How many of `items` are shown (the rest are hidden).
    shown: usize,
    steps: (f32, f32),
    state: i32,
    added: usize,
    extending: bool,
    collapsing: bool,
    /// Offsets of the added children (added order: farthest first).
    offsets: Vec<f32>,
    endcap: Option<f32>,
}

impl Ext {
    fn new(rel: ArtFile, steps: (f32, f32), items: Vec<Item>) -> Ext {
        Ext {
            rel,
            dir: (0.0, 1.0),
            has_endcap: false,
            shown: items.len(),
            items,
            steps,
            state: 0,
            added: 0,
            extending: false,
            collapsing: false,
            offsets: Vec::new(),
            endcap: None,
        }
    }

    fn is_open(&self) -> bool {
        self.state > 0 || self.extending
    }

    fn open(&mut self) {
        self.extending = true;
        self.collapsing = false;
    }

    fn close(&mut self) {
        if self.state > 0 {
            self.collapsing = true;
            self.extending = false;
        }
    }

    fn toggle(&mut self) {
        if self.is_open() && !self.collapsing {
            self.close();
        } else {
            self.open();
        }
    }

    /// Item index of the `k`-th added child: the farthest item is added
    /// first.
    fn item_of(&self, k: usize) -> usize {
        self.shown - 1 - k
    }

    fn step(&self) -> f32 {
        match self.state {
            2 => self.steps.0,
            3 => self.steps.1,
            _ => 0.0,
        }
    }

    fn move_all(&mut self, d: f32) {
        for o in &mut self.offsets {
            *o += d;
        }
        if let Some(e) = &mut self.endcap {
            *e += d;
        }
    }

    fn tick(&mut self) {
        for it in &mut self.items {
            if let Some(t) = &mut it.tray {
                t.tick();
            }
        }
        let n = self.shown;
        if self.extending {
            if (self.state == 2 || self.state == 4) && self.added < n {
                if self.added == 0 && self.has_endcap {
                    self.endcap = Some(self.steps.0 + self.steps.1);
                }
                self.offsets.push(0.0);
                self.added += 1;
            }
            let d = self.step();
            self.move_all(d);
            self.state += 1;
            if self.state == 4 {
                if self.added < n {
                    self.state = 2;
                } else {
                    self.extending = false;
                }
            }
        } else if self.collapsing {
            // Close any open tray first.
            if self.items.iter().any(|it| it.tray.as_ref().is_some_and(|t| t.state > 0)) {
                for it in &mut self.items {
                    if let Some(t) = &mut it.tray {
                        t.close();
                    }
                }
                return;
            }
            if self.added > 0 && self.state == 2 {
                self.added -= 1;
                self.offsets.pop();
                if self.added == 0 {
                    self.endcap = None;
                }
            }
            if self.state < 4 {
                let d = self.step();
                self.move_all(-d);
            }
            self.state -= 1;
            if self.state == 1 && self.added > 0 {
                self.state = 3;
            }
            if self.state <= 0 {
                self.state = 0;
                self.collapsing = false;
                self.offsets.clear();
                self.endcap = None;
            }
        }
    }
}

fn tray(rel: ArtFile, items: &[(Group, Text, Command)]) -> Ext {
    let mut e = Ext::new(
        rel,
        (41.0, 0.0),
        items
            .iter()
            .map(|&(group, tip, c)| Item {
                group,
                tip,
                action: Some(c),
                tray: None,
                overlay: None,
            })
            .collect(),
    );
    e.dir = (1.0, 0.0);
    e.has_endcap = true;
    e
}

fn log(group: Group, tip: Text, overlay: Group, tray: Ext) -> Item {
    Item {
        group,
        tip,
        action: None,
        tray: Some(tray),
        overlay: Some(overlay),
    }
}

fn construction_menu() -> Ext {
    use Command::*;
    Ext::new(
        CONSTRUCTION,
        (27.0, 28.0),
        vec![
            log(
                art::construction_ext::LOG_STRUCTURES,
                Text::CemBuildings,
                art::construction_ext::STRUCTURES_OVERLAY,
                tray(
                    BUILD_SUB,
                    &[
                        (art::build_sub::TRAY_LODGING, Text::BesmLodging, Buildings(0)),
                        (art::build_sub::TRAY_FOOD, Text::BesmFood, Buildings(1)),
                        (art::build_sub::TRAY_COMFORT, Text::BesmGuestComfort, Buildings(3)),
                        (art::build_sub::TRAY_ENTERTAINMENT, Text::BesmEntertainment, Buildings(2)),
                        (art::build_sub::TRAY_RENTALS, Text::BesmRentals, Buildings(4)),
                        (art::build_sub::TRAY_MAINTENANCE, Text::BesmMaintenance, Buildings(7)),
                        (art::build_sub::TRAY_SERVICES, Text::BesmServices, Buildings(5)),
                        (art::build_sub::TRAY_UTILITIES, Text::BesmUtilities, Buildings(6)),
                    ],
                ),
            ),
            log(
                art::construction_ext::LOG_LIFTS,
                Text::CemLifts,
                art::construction_ext::LIFTS_OVERLAY,
                tray(
                    LIFTS_SUB,
                    &[
                        (art::lifts_sub::TRAY_LIFTS, Text::LemLift, Lifts),
                        (art::lifts_sub::TRAY_JUMPS, Text::LemJumps, Jumps),
                        (art::lifts_sub::TRAY_MARKERS, Text::LemRunmarker, RunMarkers),
                    ],
                ),
            ),
            log(
                art::construction_ext::LOG_OBJECTS,
                Text::CemObjects,
                art::construction_ext::OBJECTS_OVERLAY,
                tray(
                    FLUFF_SUB,
                    &[(art::fluff_sub::TRAY_OBJECTS, Text::FluffEmObjects, Objects), (art::fluff_sub::TRAY_PATHS, Text::FluffEmPaths, Paths)],
                ),
            ),
        ],
    )
}

fn terrain_menu() -> Ext {
    use Command::*;
    use TerrainTool as T;
    Ext::new(
        TERRAIN,
        (27.0, 28.0),
        vec![
            log(
                art::terrain_ext::LOG_SURFACEMOD,
                Text::TerrainExtensionSurface,
                art::terrain_ext::SURFACEMOD_OVERLAY,
                tray(
                    SURFACE_SUB,
                    &[
                        (art::surface_sub::TRAY_HILL, Text::SurfaceEditHill, Surface(T::Hill)),
                        (art::surface_sub::TRAY_PEAK, Text::SurfaceEditPeak, Surface(T::Peak)),
                        (art::surface_sub::TRAY_FLATRAISE, Text::SurfaceEditFlatten, Surface(T::Flatten)),
                        (art::surface_sub::TRAY_RAISEPOINT, Text::SurfaceEditSingle, Surface(T::Point)),
                        (art::surface_sub::TRAY_SMOOTH, Text::SurfaceEditSmooth, Surface(T::Smooth)),
                        (art::surface_sub::TRAY_MOGULS, Text::SurfaceEditMoguls, Surface(T::Moguls)),
                        (art::surface_sub::TRAY_SNOW, Text::SurfaceEditMakeSnow, Surface(T::MakeSnow)),
                        (art::surface_sub::TRAY_GROOMER, Text::SurfaceEditGroomer, Surface(T::Groom)),
                    ],
                ),
            ),
            log(
                art::terrain_ext::LOG_TREES,
                Text::TerrainExtensionTrees,
                art::terrain_ext::TREES_OVERLAY,
                tray(
                    TREES_SUB,
                    &[
                        (art::trees_sub::TRAY_CHAINSAW, Text::TesmClear, Trees(0)),
                        (art::trees_sub::TRAY_TREESPARSE, Text::TesmSparse, Trees(1)),
                        (art::trees_sub::TRAY_TREEMEDIUM, Text::TesmMedium, Trees(2)),
                        (art::trees_sub::TRAY_TREEDENSE, Text::TesmDense, Trees(3)),
                    ],
                ),
            ),
            log(
                art::terrain_ext::LOG_SKIRUN,
                Text::TerrainExtensionSkiruns,
                art::terrain_ext::SKIRUN_OVERLAY,
                tray(
                    SKIRUN_SUB,
                    &[
                        (art::skirun_sub::TRAY_RUNFOLLOW, Text::SresmFollow, RunPath(0)),
                        (art::skirun_sub::TRAY_RUNSUB, Text::SresmSubtractive, RunPath(1)),
                        (art::skirun_sub::TRAY_RUNADD, Text::SresmAddative, RunPath(2)),
                    ],
                ),
            ),
        ],
    )
}

fn finance_menu() -> Ext {
    use Command::*;
    let item = |group, tip, c| Item {
        group,
        tip,
        action: Some(c),
        tray: None,
        overlay: None,
    };
    Ext::new(
        FINANCE,
        (27.0, 28.0),
        vec![
            item(art::finance_ext::LOG_RESORTSTATS, Text::FemResortstats, ResortStats),
            item(art::finance_ext::LOG_RESORTFINANCE, Text::FemFinances, Finances),
            item(art::finance_ext::LOG_GUESTSTATS, Text::FemGueststats, GuestStats),
            item(art::finance_ext::LOG_STAFFSTATS, Text::FemStaffstats, StaffStats),
            item(art::finance_ext::LOG_ATTRACTION, Text::FemAttraction, AttractionStats),
        ],
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Anchor {
    Left,
    Center,
    Right,
}

fn anchor_of(x: f32) -> Anchor {
    if x < 430.0 {
        Anchor::Left
    } else if x < 594.0 {
        Anchor::Center
    } else {
        Anchor::Right
    }
}

/// Frame geometry for the current window.
#[derive(Clone, Copy)]
struct Frame {
    screen: ERect,
    s: f32,
    ppp: f32,
}

impl Frame {
    fn cv(&self, a: Anchor, bottom: bool) -> Canvas {
        let s = self.s;
        let x = match a {
            Anchor::Left => self.screen.min.x,
            Anchor::Center => self.screen.center().x - LAYOUT.x * 0.5 * s,
            Anchor::Right => self.screen.max.x - LAYOUT.x * s,
        };
        let y = if bottom { self.screen.max.y - LAYOUT.y * s } else { self.screen.min.y };
        Canvas {
            origin: Pos2::new(x, y),
            scale: s,
            ppp: self.ppp,
        }
    }

    /// Canvas for a layout position, picked by which part of the frame it
    /// belongs to.
    fn at(&self, x: f32, y: f32) -> Canvas {
        self.cv(anchor_of(x), y > LAYOUT.y * 0.5)
    }
}

/// What the HUD asks the rest of the interface to do.
pub enum HudEvent {
    Command(Command),
    MainMenu,
    ToggleMinimap,
}

pub struct Hud {
    menus: Vec<(Nav, Ext)>,
    tick_acc: f32,
    /// Tooltip key and how long the pointer has rested on it.
    hover: Option<(Text, f32)>,
    hover_seen: bool,
    /// World tooltip text and how long it has been unchanged.
    world_tip: Option<(String, f32)>,
    ticker_x: f32,
    ticker_msg: usize,
    /// Screen areas the HUD covers; clicks there do not reach the world.
    pub blocks: Vec<ERect>,
    collapse: bool,
}

impl Default for Hud {
    fn default() -> Hud {
        Hud {
            menus: vec![
                (Nav::Build, construction_menu()),
                (Nav::Terrain, terrain_menu()),
                (Nav::Finances, finance_menu()),
            ],
            tick_acc: 0.0,
            hover: None,
            hover_seen: false,
            world_tip: None,
            ticker_x: 0.0,
            ticker_msg: usize::MAX,
            blocks: Vec::new(),
            collapse: false,
        }
    }
}

fn font(style: TextStyle, s: f32) -> FontId {
    FontId::new(style.size * s, FontFamily::Name(if style.bold { ARIAL_BOLD } else { ARIAL }.into()))
}

impl Hud {
    /// True when the pointer is over a part of the HUD.
    pub fn covers(&self, p: Option<Pos2>) -> bool {
        p.is_some_and(|p| self.blocks.iter().any(|r| r.contains(p)))
    }

    pub fn show(&mut self, ui: &mut egui::Ui, art: &mut Art, assets: &Assets, strings: &Strings, g: &mut Game) -> Vec<HudEvent> {
        let mut events = Vec::new();
        self.blocks.clear();
        self.hover_seen = false;
        let screen = ui.ctx().content_rect();
        let s = (screen.height() / LAYOUT.y).min(screen.width() / LAYOUT.x);
        let f = Frame {
            screen,
            s,
            ppp: ui.ctx().pixels_per_point(),
        };
        // The surface tray has make-snow and groom only once a Snow
        // Maker/Groomer Shop exists (0x43ef60): 8 tools, else 6.
        let shop = g
            .world
            .structures
            .iter()
            .any(|st| g.defs.structures[st.def].kind == crate::game::defs::StructureKind::Groomer);
        if let Some(t) = self
            .menus
            .iter_mut()
            .find(|(n, _)| *n == Nav::Terrain)
            .and_then(|(_, m)| m.items.first_mut())
            .and_then(|it| it.tray.as_mut())
        {
            let want = if shop { t.items.len() } else { t.items.len().min(6) };
            // Only change it while the tray is shut.
            if t.state == 0 && !t.extending {
                t.shown = want;
            }
        }
        // The original advances its menus two ticks per frame.
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        self.tick_acc += dt * 60.0;
        while self.tick_acc >= 1.0 {
            self.tick_acc -= 1.0;
            for (_, m) in &mut self.menus {
                m.tick();
            }
        }
        if self.menus.iter().any(|(_, m)| m.extending || m.collapsing) {
            ui.ctx().request_repaint();
        }

        let editor = g.world.editor;
        let buttons: &[(Group, Nav, f32, f32, Text)] = if editor { &EDITOR_NAV_BUTTONS } else { &NAV_BUTTONS };
        // The editor's buttons are laid out around the middle.
        let at = |x: f32, y: f32| if editor { f.cv(Anchor::Center, y > LAYOUT.y * 0.5) } else { f.at(x, y) };
        if editor {
            self.editor_frame(ui, art, assets, f);
        } else {
            self.frame(ui, art, assets, f);
            self.readouts(ui, art, assets, strings, f, g);
            self.ticker(ui, strings, f, g, dt);
        }
        for &(group, nav, x, y, tip) in buttons {
            if matches!(nav, Nav::Build | Nav::Terrain | Nav::Finances) {
                continue;
            }
            // `pause_off` shows the play arrow; the original shows it while
            // the game runs.
            let group = if nav == Nav::Pause && !g.paused { art::nav_bar::PAUSE_OFF } else { group };
            let cv = at(x, y);
            if self.button(ui, art, assets, cv, NAV, group, x, y, tip) {
                self.nav(nav, g, &mut events);
            }
        }
        // The three menu logs first, then the menus so they draw over the
        // other logs.
        let mut clicked = None;
        let navs: Vec<Nav> = self.menus.iter().map(|m| m.0).collect();
        for (i, nav) in navs.into_iter().enumerate() {
            let Some(&(group, _, x, y, tip)) = buttons.iter().find(|b| b.1 == nav) else { continue };
            if self.button(ui, art, assets, at(x, y), NAV, group, x, y, tip) {
                clicked = Some(i);
            }
        }
        for i in 0..self.menus.len() {
            let nav = self.menus[i].0;
            let Some(&(_, _, x, y, _)) = buttons.iter().find(|b| b.1 == nav) else { continue };
            let cv = at(x, y);
            let overlay = match nav {
                Nav::Build => art::nav_bar::CONSTRUCTION_OVERLAY,
                Nav::Terrain => art::nav_bar::TERRAIN_OVERLAY,
                _ => art::nav_bar::FINANCE_OVERLAY,
            };
            if !self.menus[i].1.is_open() {
                continue;
            }
            let mut menu = std::mem::replace(&mut self.menus[i].1, Ext::new(NAV, (0.0, 0.0), Vec::new()));
            self.ext(ui, art, assets, cv, &mut menu, x, y, &mut events);
            art.draw(ui, assets, cv, NAV, overlay, 0, x, y);
            self.menus[i].1 = menu;
        }
        if let Some(i) = clicked {
            for (j, (_, m)) in self.menus.iter_mut().enumerate() {
                if j == i {
                    m.toggle();
                } else {
                    m.close();
                }
            }
        }
        if std::mem::take(&mut self.collapse) {
            self.close_menus();
        }
        self.tooltip(ui, art, assets, strings, f, dt);
        self.survey(ui, art, assets, strings, f, g);
        self.tree_rect(ui, art, assets, f, g);
        self.placement_boxes(ui, f, g);
        self.world_tooltip(ui, art, assets, f, g, dt);
        events
    }

    /// Opens a menu and optionally one of its trays, for screenshots:
    /// `build`, `terrain/1`, `finances`.
    /// Opens one of the extending menus, and optionally one of its trays.
    pub fn open_menu(&mut self, nav: Nav, tray: Option<usize>) {
        for (n, m) in &mut self.menus {
            if *n == nav {
                m.open();
                if let Some(t) = tray.and_then(|i| m.items.get_mut(i)).and_then(|it| it.tray.as_mut()) {
                    t.open();
                }
            }
        }
    }

    /// Closes every extending menu.
    pub fn close_menus(&mut self) {
        for (_, m) in &mut self.menus {
            m.close();
        }
    }

    fn nav(&mut self, nav: Nav, g: &mut Game, events: &mut Vec<HudEvent>) {
        match nav {
            Nav::Grid => g.show_grid = !g.show_grid,
            Nav::Main => events.push(HudEvent::MainMenu),
            Nav::ZoomIn => g.camera.zoom(1),
            Nav::ZoomOut => g.camera.zoom(-1),
            Nav::RotateCw => g.camera.quarter_turn(-1.0),
            Nav::RotateCcw => g.camera.quarter_turn(1.0),
            Nav::Pause => g.paused = !g.paused,
            Nav::MapView => events.push(HudEvent::ToggleMinimap),
            Nav::Survey => {
                g.tool = if matches!(g.tool, crate::game::Tool::Survey { .. }) {
                    crate::game::Tool::Inspect
                } else {
                    crate::game::Tool::Survey { start: None }
                }
            }
            // A skier of the player's own, to put down and ride (0x461fc7).
            Nav::FirstPerson => g.spawn_own_skier(),
            Nav::Build | Nav::Terrain | Nav::Finances => {}
        }
    }

    /// Draws the frame: three top parts and two bottom parts at their
    /// anchors, with the plain bar stretched between them.
    fn frame(&mut self, ui: &egui::Ui, art: &mut Art, assets: &Assets, f: Frame) {
        let Some(sprite) = art.group_element(assets, NAV, art::nav_bar::BACKGROUND_1024, 0).and_then(|e| e.0) else { return };
        let s = f.s;
        let src = |x1: f32, y1: f32, x2: f32, y2: f32| ERect::from_min_max(Pos2::new(x1, y1), Pos2::new(x2, y2));
        let left = f.cv(Anchor::Left, false);
        let center = f.cv(Anchor::Center, false);
        let right = f.cv(Anchor::Right, false);
        let bl = f.cv(Anchor::Left, true);
        let br = f.cv(Anchor::Right, true);
        // Top: rows 0..74, plain bar 0..23 between the parts.
        let parts = [
            (left, 0.0, 430.0),
            (center, 430.0, 594.0),
            (right, 594.0, 1024.0),
        ];
        for (cv, x1, x2) in parts {
            art.sprite_stretch(ui, assets, cv, NAV, sprite, src(x1, 0.0, x2, 74.0), cv.rect_xywh(x1, 0.0, x2 - x1, 74.0));
        }
        let gap = |a: Pos2, b: Pos2| ERect::from_min_max(a, b);
        art.sprite_stretch(ui, assets, left, NAV, sprite, src(420.0, 0.0, 421.0, 23.0), gap(left.pos(430.0, 0.0), center.pos(430.0, 23.0)));
        art.sprite_stretch(ui, assets, left, NAV, sprite, src(610.0, 0.0, 611.0, 23.0), gap(center.pos(594.0, 0.0), right.pos(594.0, 23.0)));
        // Bottom: rows 708..768, message box stretched in the middle.
        art.sprite_stretch(ui, assets, bl, NAV, sprite, src(0.0, 708.0, 230.0, 768.0), bl.rect_xywh(0.0, 708.0, 230.0, 60.0));
        art.sprite_stretch(ui, assets, br, NAV, sprite, src(790.0, 708.0, 1024.0, 768.0), br.rect_xywh(790.0, 708.0, 234.0, 60.0));
        art.sprite_stretch(ui, assets, bl, NAV, sprite, src(500.0, 708.0, 501.0, 768.0), gap(bl.pos(230.0, 708.0), br.pos(790.0, 768.0)));
        self.blocks.push(ERect::from_min_max(f.screen.min, Pos2::new(f.screen.max.x, f.screen.min.y + 23.0 * s)));
        self.blocks.push(ERect::from_min_max(center.pos(440.0, 0.0), center.pos(584.0, 74.0)));
        self.blocks.push(ERect::from_min_max(Pos2::new(f.screen.min.x, bl.pos(0.0, 708.0).y), f.screen.max));
    }

    /// The map editor's frame: just the panel under the view controls
    /// (`1024TEBackground`, rows 708..768).
    fn editor_frame(&mut self, ui: &egui::Ui, art: &mut Art, assets: &Assets, f: Frame) {
        let Some(sprite) = art.group_element(assets, NAV, art::nav_bar::TE_BACKGROUND_1024, 0).and_then(|e| e.0) else { return };
        let src = ERect::from_min_max(Pos2::new(387.0, 708.0), Pos2::new(641.0, 768.0));
        let cv = f.cv(Anchor::Center, true);
        let area = cv.rect_xywh(387.0, 708.0, 254.0, 60.0);
        art.sprite_stretch(ui, assets, cv, NAV, sprite, src, area);
        self.blocks.push(area);
    }

    fn readouts(&mut self, ui: &egui::Ui, art: &mut Art, assets: &Assets, strings: &Strings, f: Frame, g: &Game) {
        let center = f.cv(Anchor::Center, false);
        let br = f.cv(Anchor::Right, true);
        let w = &g.world;
        let painter = ui.painter();
        let text = |cv: Canvas, style: TextStyle, x1: f32, y1: f32, x2: f32, t: &str| {
            painter.text(
                cv.pos((x1 + x2) * 0.5, y1),
                egui::Align2::CENTER_TOP,
                t,
                font(style, cv.scale),
                style.color,
            );
        };
        text(center, FONT0, 469.0, 24.0, 560.0, &super::style::money(w.cash));
        let score = w.rating.stars * 200.0;
        for i in 0..5 {
            let e = if score >= STAR_FULL[i] {
                2
            } else if score >= STAR_HALF[i] {
                1
            } else {
                0
            };
            art.draw(ui, assets, center, NAV, art::nav_bar::STAR, e, 475.0 + 15.0 * i as f32, 41.0);
        }
        let week = w.day / 7 + 1;
        let day = w.day % 7 + 1;
        let date = strings.text(Text::NavBarDayWeek).replacen("%d", &week.to_string(), 1).replacen("%d", &day.to_string(), 1);
        text(br, FONT1, 856.0, 725.0, 963.0, &date);
        let occupancy = strings.text(Text::NavBarOccupancy).replacen("%d", &w.active_guests().to_string(), 1);
        text(br, FONT1, 857.0, 737.0, 964.0, &occupancy);
    }

    /// The message box scrolls the latest message from right to left.
    fn ticker(&mut self, ui: &egui::Ui, strings: &Strings, f: Frame, g: &Game, dt: f32) {
        let bl = f.cv(Anchor::Left, true);
        let br = f.cv(Anchor::Right, true);
        let area = ERect::from_min_max(bl.pos(216.0, 732.0), br.pos(790.0, 758.0));
        // Placement problems the original names show in the message box
        // while they last.
        let problem = g
            .placement
            .as_ref()
            .and_then(|p| p.reason)
            .or_else(|| g.last_error.as_ref().map(|e| e.0))
            .and_then(|p| p.text());
        if let Some(t) = problem {
            let t = strings.text(t).into_owned();
            let red = Color32::from_rgb(0xb0, 0x10, 0x10);
            ui.painter().with_clip_rect(area).text(
                Pos2::new(area.center().x, area.min.y),
                egui::Align2::CENTER_TOP,
                t,
                font(FONT0, f.s),
                red,
            );
            return;
        }
        let Some(msg) = g.world.messages.last() else { return };
        if self.ticker_msg != g.world.messages.len() {
            self.ticker_msg = g.world.messages.len();
            self.ticker_x = area.width();
        }
        let galley = ui.painter().layout_no_wrap(msg.text.clone(), font(FONT0, f.s), FONT0.color);
        self.ticker_x -= dt * 60.0 * f.s;
        if self.ticker_x < -galley.size().x {
            self.ticker_x = area.width();
        }
        let pos = Pos2::new(area.min.x + self.ticker_x, area.min.y);
        ui.painter().with_clip_rect(area).galley(pos, galley, FONT0.color);
        ui.ctx().request_repaint();
    }

    /// A three-state button from a group at layout position (x, y).
    #[allow(clippy::too_many_arguments)]
    fn button(&mut self, ui: &mut egui::Ui, art: &mut Art, assets: &Assets, cv: Canvas, rel: ArtFile, group: Group, x: f32, y: f32, tip: Text) -> bool {
        let Some(r) = art.hit_rect(assets, rel, group, 0) else { return false };
        let area = cv.rect_xywh(x + r.x1 as f32, y + r.y1 as f32, r.width() as f32, r.height() as f32);
        let resp = ui.interact(area, ui.scope_id().with((rel, group, x as i32, y as i32)), Sense::click());
        let state = if resp.is_pointer_button_down_on() {
            2
        } else if resp.hovered() {
            1
        } else {
            0
        };
        art.draw(ui, assets, cv, rel, group, state, x, y);
        self.track(&resp, area, tip, assets);
        if resp.clicked() {
            crate::audio::sfx(assets, crate::audio::Sound::IngameDownclick);
        }
        resp.clicked()
    }

    fn track(&mut self, resp: &egui::Response, area: ERect, tip: Text, assets: &Assets) {
        self.blocks.push(area);
        if resp.hovered() {
            self.hover_seen = true;
            match &self.hover {
                Some((t, _)) if *t == tip => {}
                _ => {
                    self.hover = Some((tip, 0.0));
                    crate::audio::sfx(assets, crate::audio::Sound::IngameRollover);
                }
            }
        }
    }

    /// Draws an extending menu whose parent log is at (px, py).
    #[allow(clippy::too_many_arguments)]
    fn ext(&mut self, ui: &mut egui::Ui, art: &mut Art, assets: &Assets, cv: Canvas, m: &mut Ext, px: f32, py: f32, events: &mut Vec<HudEvent>) {
        let (dx, dy) = m.dir;
        if let Some(e) = m.endcap {
            let (ex, ey) = (px + dx * e, py + dy * e);
            art.draw(ui, assets, cv, m.rel, art::extension::ENDCAP, 0, ex, ey);
            if let Some(r) = art.hit_rect(assets, m.rel, art::extension::ENDCAP, 0) {
                self.blocks.push(cv.rect_xywh(ex + r.x1 as f32, ey + r.y1 as f32, r.width() as f32, r.height() as f32));
            }
        }
        let mut toggled = None;
        for k in 0..m.added {
            let i = m.item_of(k);
            let x = px + dx * m.offsets[k];
            let py = py + dy * m.offsets[k];
            let rel = m.rel;
            let it = &mut m.items[i];
            match &mut it.tray {
                Some(t) => {
                    // A log: element 3 at rest, the tray pieces while open.
                    let Some(r) = art.hit_rect(assets, rel, it.group, 0) else { continue };
                    let area = cv.rect_xywh(x + r.x1 as f32, py + r.y1 as f32, r.width() as f32, r.height() as f32);
                    let resp = ui.interact(area, ui.scope_id().with((rel, it.group)), Sense::click());
                    let count = art.element_count(assets, rel, it.group).max(1);
                    let e = (t.state as usize + 3) % count;
                    art.draw(ui, assets, cv, rel, it.group, e, x, py);
                    let tip = it.tip;
                    let overlay = it.overlay;
                    let open = t.state > 0;
                    let mut tray = std::mem::replace(t, Ext::new(rel, (0.0, 0.0), Vec::new()));
                    self.ext(ui, art, assets, cv, &mut tray, x, py, events);
                    if open {
                        if let Some(o) = overlay {
                            art.draw(ui, assets, cv, rel, o, 0, x, py);
                        }
                    }
                    m.items[i].tray = Some(tray);
                    self.track(&resp, area, tip, assets);
                    if resp.clicked() {
                        crate::audio::sfx(assets, crate::audio::Sound::IngameDownclick);
                        toggled = Some(i);
                    }
                }
                None => {
                    let (group, tip, action) = (it.group, it.tip, it.action);
                    if self.button(ui, art, assets, cv, rel, group, x, py, tip) {
                        if let Some(c) = action {
                            events.push(HudEvent::Command(c));
                            // Picking a tool folds the menus away.
                            self.collapse = true;
                        }
                    }
                }
            }
        }
        if let Some(i) = toggled {
            for (j, it) in m.items.iter_mut().enumerate() {
                if let Some(t) = &mut it.tray {
                    if j == i {
                        t.toggle();
                    } else {
                        t.close();
                    }
                }
            }
        }
    }

    /// Survey readout next to the pointer, in the original's format:
    /// elevation, slope in degrees and its rating against the skill
    /// ranges from `skiers.ini`.
    fn survey(&mut self, ui: &egui::Ui, art: &mut Art, assets: &Assets, strings: &Strings, f: Frame, g: &Game) {
        let crate::game::Tool::Survey { start: Some(a) } = g.tool else { return };
        let Some(h) = g.hover else { return };
        if self.covers(ui.ctx().pointer_hover_pos()) {
            return;
        }
        // Distance and signed slope angle from the start vertex to the
        // vertex under the pointer, rated by the absolute angle against
        // the skill ranges from `skiers.ini`.
        let b = g.nearest_vertex(h);
        let d = b - a;
        let dist = d.length();
        let horiz = d.truncate().length();
        let angle = if dist == 0.0 || d.z == 0.0 { 0.0 } else { d.z.atan2(horiz).to_degrees() };
        let sk = &g.defs.skiers;
        let limits = [
            sk.skill[0].min_slope,
            sk.skill[1].min_slope,
            sk.skill[2].min_slope,
            sk.skill[3].min_slope,
            sk.skill[3].max_slope,
            sk.danger_min_slope,
        ];
        let rating = limits.iter().filter(|&&l| angle.abs() > l).count();
        let text = format!(
            "{:.0}m, {} {:.1} {}, [{}]",
            dist,
            strings.text(Text::SurveyCursorDeg1),
            angle,
            strings.text(Text::SurveyCursorDeg2),
            strings.text(SurveyRating(rating))
        );
        // A line on the ground between the two points.
        let rect = ui.ctx().content_rect();
        let ppp = f.ppp;
        let size = glam::Vec2::new(rect.width() * ppp, rect.height() * ppp);
        if let (Some(pa), Some(pb)) = (g.camera.project(a, size), g.camera.project(b, size)) {
            ui.painter().line_segment(
                [Pos2::new(pa.x / ppp, pa.y / ppp), Pos2::new(pb.x / ppp, pb.y / ppp)],
                egui::Stroke::new(2.0, Color32::YELLOW),
            );
        }
        self.popup(ui, art, assets, f, &text);
    }

    /// Outline of the tree rectangle being dragged, with the chainsaw's
    /// cost next to the pointer.
    fn tree_rect(&mut self, ui: &egui::Ui, art: &mut Art, assets: &Assets, f: Frame, g: &Game) {
        let Some(((x1, y1), (x2, y2))) = g.tree_rect() else { return };
        let lv = g.world.level();
        let s = lv.spacing;
        let rect = ui.ctx().content_rect();
        let size = glam::Vec2::new(rect.width() * f.ppp, rect.height() * f.ppp);
        let corner = |x: i32, y: i32| {
            let (xu, yu) = (x.clamp(0, lv.size as i32 - 1) as usize, y.clamp(0, lv.size as i32 - 1) as usize);
            glam::Vec3::new(x as f32 * s, y as f32 * s, lv.height(xu, yu) + 0.3)
        };
        // Walk the border vertex by vertex so it follows the ground.
        let mut ring = Vec::new();
        for x in x1..=x2 + 1 {
            ring.push(corner(x, y1));
        }
        for y in y1..=y2 + 1 {
            ring.push(corner(x2 + 1, y));
        }
        for x in (x1..=x2 + 1).rev() {
            ring.push(corner(x, y2 + 1));
        }
        for y in (y1..=y2 + 1).rev() {
            ring.push(corner(x1, y));
        }
        let pts: Vec<Pos2> = ring
            .iter()
            .filter_map(|p| g.camera.project(*p, size))
            .map(|p| Pos2::new(p.x / f.ppp, p.y / f.ppp))
            .collect();
        ui.painter().add(egui::Shape::line(pts, egui::Stroke::new(2.0, Color32::YELLOW)));
        if let crate::game::Tool::Trees { density: 0 } = g.tool {
            let cost = g.chop_cost(((x1, y1), (x2, y2)));
            self.popup(ui, art, assets, f, &super::style::money(cost));
        }
    }

    /// One-pixel boxes around the on-screen bounds of whatever is attached
    /// to the pointer (see `game::Highlight`).
    fn placement_boxes(&self, ui: &egui::Ui, f: Frame, g: &Game) {
        let rect = ui.ctx().content_rect();
        let size = glam::Vec2::new(rect.width() * f.ppp, rect.height() * f.ppp);
        let stroke = |red: bool| {
            let c = if red { Color32::from_rgb(0xff, 0, 0) } else { Color32::from_rgb(0, 0xff, 0) };
            egui::Stroke::new(1.0 / f.ppp, c)
        };
        for h in &g.highlights {
            let mut lo = glam::Vec2::splat(f32::MAX);
            let mut hi = glam::Vec2::splat(f32::MIN);
            for &c in h.points.iter() {
                let Some(p) = g.camera.project(h.model.transform_point3(c), size) else { continue };
                lo = lo.min(p);
                hi = hi.max(p);
            }
            if lo.x > hi.x {
                continue;
            }
            // Pixel-aligned, like the original's line list.
            let r = ERect::from_min_max(
                Pos2::new(lo.x.floor() + 0.5, lo.y.floor() + 0.5) / f.ppp,
                Pos2::new(hi.x.floor() + 0.5, hi.y.floor() + 0.5) / f.ppp,
            );
            ui.painter().rect_stroke(r, 0.0, stroke(h.red), egui::StrokeKind::Middle);
        }
    }

    /// Over the world the original shows the name under the pointer (see
    /// `Game::hover_label`), after the same rest as button tooltips.
    #[allow(clippy::too_many_arguments)]
    fn world_tooltip(&mut self, ui: &egui::Ui, art: &mut Art, assets: &Assets, f: Frame, g: &Game, dt: f32) {
        let over_hud = self.hover_seen || self.covers(ui.ctx().pointer_hover_pos());
        let label = if over_hud { None } else { g.hover_label() };
        let Some(label) = label else {
            self.world_tip = None;
            return;
        };
        match &mut self.world_tip {
            Some((t, age)) if *t == label => *age += dt,
            // A changing cost stays up while the lift is dragged out.
            Some((t, age)) if t.starts_with('-') && label.starts_with('-') => {
                *t = label;
                *age += dt;
            }
            _ => self.world_tip = Some((label, 0.0)),
        }
        let Some((text, age)) = &self.world_tip else { return };
        if *age < 10.0 / 30.0 {
            ui.ctx().request_repaint();
            return;
        }
        let text = text.clone();
        self.popup(ui, art, assets, f, &text);
    }

    /// A tooltip: bold dark red text (measured 0x940021) on a plain white
    /// box (0xfffbff) without a border, centered just above the pointer.
    fn popup(&mut self, ui: &egui::Ui, _art: &mut Art, _assets: &Assets, f: Frame, text: &str) {
        let Some(p) = ui.ctx().pointer_hover_pos() else { return };
        let style = TextStyle {
            size: 12.0,
            bold: true,
            color: Color32::from_rgb(0x94, 0x00, 0x21),
        };
        let s = f.s;
        let galley = ui.painter().layout_no_wrap(text.to_string(), font(style, s), style.color);
        let size = Vec2::new(galley.size().x + 8.0 * s, galley.size().y + 2.0 * s);
        let mut min = p - Vec2::new(size.x * 0.5, size.y + s);
        min.x = min.x.clamp(f.screen.min.x, f.screen.max.x - size.x);
        min.y = min.y.max(f.screen.min.y);
        let full = ERect::from_min_size(min, size);
        let fg = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("hud_tip")));
        fg.rect_filled(full, 0.0, Color32::from_rgb(0xff, 0xfb, 0xff));
        fg.galley(full.center() - galley.size() * 0.5, galley, style.color);
    }

    /// The original shows a tooltip after the pointer rests on a button
    /// for ten ticks, on the PopupTextBG strip.
    fn tooltip(&mut self, ui: &egui::Ui, art: &mut Art, assets: &Assets, strings: &Strings, f: Frame, dt: f32) {
        if !self.hover_seen {
            self.hover = None;
            return;
        }
        let Some((key, t)) = &mut self.hover else { return };
        *t += dt;
        if *t < 10.0 / 30.0 {
            ui.ctx().request_repaint();
            return;
        }
        let text = strings.text(*key).into_owned();
        self.popup(ui, art, assets, f, &text);
    }
}
