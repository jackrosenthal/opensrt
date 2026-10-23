//! Vector interface art (`hud/**/*.svg`) that replaces original sprites:
//! most of it remakes them, the map view button is a new design. Drawn in
//! the original's layout pixels so it replaces sprites one for one and can
//! be rasterized at the display's scale. A drawing with several looks
//! holds each as a group with `class="state"` (buttons: `normal`,
//! `rollover` and `depress`), one shown at a time.
//!
//! `element` finds the drawing for an original element. The HUD's own
//! files are mapped here; the windows and screens each have a module of
//! their own, one function per screen file, and the widgets many of them
//! share (close buttons, scroll arrows, tabs and so on) are mapped once
//! for every file that has them, in `widgets`.

use resvg::{tiny_skia, usvg};

use super::art::{self, ArtFile, Group};

/// One SVG file, known by its name.
#[derive(Clone, Copy, Debug)]
pub struct Svg {
    pub name: &'static str,
    source: &'static str,
    /// A drawing this one is drawn over, with the same state shown: the
    /// shared frame of a button or tab whose icon this drawing is.
    frame: Option<&'static Svg>,
}

impl Svg {
    /// Where the drawing's top-left sits relative to the sprite it
    /// replaces: its `viewBox` origin, which is above or left of 0 when
    /// it reaches past the sprite (the menu logs' stems reach up into the
    /// log above).
    pub fn origin(&self) -> egui::Vec2 {
        let view_box = self.source.split_once("viewBox=\"").and_then(|(_, rest)| rest.split_once('"'));
        let mut n = view_box.into_iter().flat_map(|(v, _)| v.split_whitespace()).map(|v| v.parse().unwrap_or(0.0));
        egui::Vec2::new(n.next().unwrap_or(0.0), n.next().unwrap_or(0.0))
    }
}

/// A drawing is known by its name and its frame's, as one picture can be
/// drawn over more than one frame.
impl PartialEq for Svg {
    fn eq(&self, other: &Svg) -> bool {
        self.name == other.name && self.frame.map(|f| f.name) == other.frame.map(|f| f.name)
    }
}

impl Eq for Svg {}

impl std::hash::Hash for Svg {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.name.hash(h);
        self.frame.map(|f| f.name).hash(h);
    }
}

/// `svg!("trays/food")` is `hud/trays/food.svg`; `svg!("widgets/focus" on
/// ROUND)` is an icon drawn over the frame `ROUND`.
macro_rules! svg {
    ($name:literal) => {
        Svg { name: $name, source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/hud/", $name, ".svg")), frame: None }
    };
    ($name:literal on $frame:expr) => {
        Svg {
            name: $name,
            source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/hud/", $name, ".svg")),
            frame: Some(&$frame),
        }
    };
}

mod front_end;
mod object_windows;
mod pickers;
mod stat_windows;
pub mod widgets;

/// A drawing and the state to show, if it has states.
pub type Drawing = (Svg, Option<&'static str>);

/// A button's drawing, showing state `index` (normal, rollover, depress).
fn button(svg: Svg, index: usize) -> Option<Drawing> {
    Some((svg, Some(state(index))))
}

/// A drawing showing one named state.
fn shown(svg: Svg, state: &'static str) -> Option<Drawing> {
    Some((svg, Some(state)))
}

/// A drawing with no states.
fn still(svg: Svg) -> Option<Drawing> {
    Some((svg, None))
}

/// The bottom bar, drawn as wide as the window: its root is
/// `width="1024"`, laid out relative to that.
pub const BOTTOM_BAR: Svg = svg!("bottom_bar");
/// The top bar, drawn as wide as the window like the bottom bar.
pub const TOP_BAR: Svg = svg!("top_bar");
pub const SNOWFLAKE: Svg = svg!("snowflake");
/// The front end's mountain scene, behind every menu screen.  It reaches
/// past the canvas on every side, so it fills wide and tall windows.
pub const MENU_SCENE: Svg = svg!("front_end/scene");
/// The map editor's panel under the view buttons.
pub const EDITOR_BOTTOM: Svg = svg!("editor_bottom");

/// What a menu log stretches through while its tray opens, and the cap
/// on the tray's far end.
const LOG_OPEN_SHORT: Svg = svg!("menus/open_short");
const LOG_OPEN_LONG: Svg = svg!("menus/open_long");
const ENDCAP: Svg = svg!("menus/endcap");

/// The vector drawing that stands in for element `index` of a group, with
/// the state to show, if it has one. A screen's own arms come first, so
/// they can claim elements of a group whose other elements are shared
/// widgets (a window's panels, say, where one panel is the plain list).
pub fn element(file: ArtFile, group: Group, index: usize) -> Option<Drawing> {
    hud(file, group, index)
        .or_else(|| object_windows::element(file, group, index))
        .or_else(|| stat_windows::element(file, group, index))
        .or_else(|| pickers::element(file, group, index))
        .or_else(|| front_end::element(file, group, index))
        .or_else(|| widgets::element(file, group, index))
}

/// The bars, menus and trays of the in-game HUD.
fn hud(file: ArtFile, group: Group, index: usize) -> Option<Drawing> {
    let button = |svg: Svg| button(svg, index);
    // A menu log: its three states, element 3 at rest, then the pieces it
    // stretches through while its tray opens.
    let log = |svg: Svg| match index {
        0..=2 => button(svg),
        3 => shown(svg, "normal"),
        4 => still(LOG_OPEN_SHORT),
        _ => still(LOG_OPEN_LONG),
    };
    if group == art::extension::ENDCAP && art::extension::FILES.contains(&file) {
        return still(ENDCAP);
    }
    match file {
        art::nav_bar::FILE => {
            use art::nav_bar::*;
            match group {
                ZOOM_IN => button(svg!("zoom_in")),
                ZOOM_OUT => button(svg!("zoom_out")),
                ROTATE_CW => button(svg!("rotate_cw")),
                ROTATE_CCW => button(svg!("rotate_ccw")),
                MAP_VIEW => button(svg!("map_view")),
                MAIN => button(svg!("main")),
                BUILD => button(svg!("build")),
                TERRAIN => button(svg!("terrain")),
                FINANCES => button(svg!("finances")),
                FIRST_PERSON => button(svg!("first_person")),
                LOG_GRID => button(svg!("grid")),
                LOG_SURVEY => button(svg!("survey")),
                // Each shows what the button is now, then what it will be.
                PAUSE => shown(svg!("pause"), ["pause", "play", "play"][index.min(2)]),
                PAUSE_OFF => shown(svg!("pause"), ["play", "pause_lit", "pause"][index.min(2)]),
                // Drawn over a log while its menu is open.
                CONSTRUCTION_OVERLAY => shown(svg!("build"), "depress"),
                TERRAIN_OVERLAY => shown(svg!("terrain"), "depress"),
                FINANCE_OVERLAY => shown(svg!("finances"), "rollover"),
                STAR => shown(svg!("star"), ["none", "half", "full"][index.min(2)]),
                _ => None,
            }
        }
        art::construction_ext::FILE => {
            use art::construction_ext::*;
            match group {
                LOG_STRUCTURES => log(svg!("menus/structures")),
                LOG_LIFTS => log(svg!("menus/lifts")),
                LOG_OBJECTS => log(svg!("menus/objects")),
                // Drawn over a log while its tray is open.
                STRUCTURES_OVERLAY => shown(svg!("menus/structures"), "open"),
                LIFTS_OVERLAY => shown(svg!("menus/lifts"), "open"),
                OBJECTS_OVERLAY => shown(svg!("menus/objects"), "open"),
                _ => None,
            }
        }
        art::terrain_ext::FILE => {
            use art::terrain_ext::*;
            match group {
                LOG_SURFACEMOD => log(svg!("menus/surface")),
                LOG_TREES => log(svg!("menus/trees")),
                LOG_SKIRUN => log(svg!("menus/runs")),
                SURFACEMOD_OVERLAY => shown(svg!("menus/surface"), "open"),
                TREES_OVERLAY => shown(svg!("menus/trees"), "open"),
                SKIRUN_OVERLAY => shown(svg!("menus/runs"), "open"),
                _ => None,
            }
        }
        art::finance_ext::FILE => {
            use art::finance_ext::*;
            match group {
                LOG_RESORTSTATS => log(svg!("menus/resort_stats")),
                LOG_RESORTFINANCE => log(svg!("menus/finances")),
                LOG_GUESTSTATS => log(svg!("menus/guests")),
                LOG_STAFFSTATS => log(svg!("menus/staff")),
                LOG_ATTRACTION => log(svg!("menus/attractions")),
                _ => None,
            }
        }
        art::build_sub::FILE => {
            use art::build_sub::*;
            match group {
                TRAY_LODGING => button(svg!("trays/lodging")),
                TRAY_FOOD => button(svg!("trays/food")),
                TRAY_COMFORT => button(svg!("trays/comfort")),
                TRAY_ENTERTAINMENT => button(svg!("trays/entertainment")),
                TRAY_RENTALS => button(svg!("trays/rentals")),
                TRAY_MAINTENANCE => button(svg!("trays/maintenance")),
                TRAY_SERVICES => button(svg!("trays/services")),
                TRAY_UTILITIES => button(svg!("trays/utilities")),
                _ => None,
            }
        }
        art::lifts_sub::FILE => {
            use art::lifts_sub::*;
            match group {
                TRAY_LIFTS => button(svg!("trays/lifts")),
                TRAY_JUMPS => button(svg!("trays/jumps")),
                TRAY_MARKERS => button(svg!("trays/markers")),
                _ => None,
            }
        }
        art::fluff_sub::FILE => {
            use art::fluff_sub::*;
            match group {
                TRAY_OBJECTS => button(svg!("trays/objects")),
                TRAY_PATHS => button(svg!("trays/paths")),
                _ => None,
            }
        }
        art::skirun_sub::FILE => {
            use art::skirun_sub::*;
            match group {
                TRAY_RUNFOLLOW => button(svg!("trays/run_follow")),
                TRAY_RUNSUB => button(svg!("trays/run_cut")),
                TRAY_RUNADD => button(svg!("trays/run_fill")),
                _ => None,
            }
        }
        art::surface_sub::FILE => {
            use art::surface_sub::*;
            match group {
                TRAY_HILL => button(svg!("trays/hill")),
                TRAY_PEAK => button(svg!("trays/peak")),
                TRAY_FLATRAISE => button(svg!("trays/flatten")),
                TRAY_RAISEPOINT => button(svg!("trays/point")),
                TRAY_SMOOTH => button(svg!("trays/smooth")),
                TRAY_MOGULS => button(svg!("trays/moguls")),
                TRAY_SNOW => button(svg!("trays/snow")),
                TRAY_GROOMER => button(svg!("trays/groomer")),
                _ => None,
            }
        }
        art::trees_sub::FILE => {
            use art::trees_sub::*;
            match group {
                TRAY_CHAINSAW => button(svg!("trays/clear")),
                TRAY_TREESPARSE => button(svg!("trays/sparse")),
                TRAY_TREEMEDIUM => button(svg!("trays/medium")),
                TRAY_TREEDENSE => button(svg!("trays/dense")),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Group shown for button state `index` (normal, rollover, depress).
pub fn state(index: usize) -> &'static str {
    ["normal", "rollover", "depress"][index.min(2)]
}

/// Draws `svg` (with only state group `state` shown, when given, and over
/// its frame, if it has one) at `scale` pixels per layout pixel. `width`
/// replaces the layout width of a drawing laid out relative to it (its
/// root's `width="1024"`). Returns the image and the drawing's size in
/// layout pixels.
pub fn rasterize(svg: Svg, state: Option<&str>, width: Option<f32>, scale: f32) -> Option<(egui::ColorImage, egui::Vec2)> {
    let mut opt = usvg::Options::default();
    if let Some(s) = state {
        opt.style_sheet = Some(format!(".state{{display:none}} #{s}{{display:inline}}"));
    }
    // usvg only resolves percentages against an explicit root width.
    let source = match width {
        Some(w) => std::borrow::Cow::Owned(svg.source.replacen("width=\"1024\"", &format!("width=\"{w}\""), 1)),
        None => std::borrow::Cow::Borrowed(svg.source),
    };
    let parse = |svg: &Svg, source: &str| usvg::Tree::from_str(source, &opt).map_err(|e| log::warn!("{}.svg: {e}", svg.name)).ok();
    let tree = parse(&svg, &source)?;
    let size = egui::Vec2::new(tree.size().width(), tree.size().height());
    let (w, h) = (((size.x * scale).round() as u32).max(1), ((size.y * scale).round() as u32).max(1));
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let t = tiny_skia::Transform::from_scale(w as f32 / size.x, h as f32 / size.y);
    // The frame, laid out on the same grid, goes underneath.
    if let Some(frame) = svg.frame {
        resvg::render(&parse(frame, frame.source)?, t, &mut pixmap.as_mut());
    }
    resvg::render(&tree, t, &mut pixmap.as_mut());
    // tiny-skia's pixels are premultiplied, as egui's are.
    let pixels = pixmap
        .pixels()
        .iter()
        .map(|p| egui::Color32::from_rgba_premultiplied(p.red(), p.green(), p.blue(), p.alpha()))
        .collect();
    Some((egui::ColorImage { size: [w as usize, h as usize], source_size: egui::Vec2::new(w as f32, h as f32), pixels }, size))
}
