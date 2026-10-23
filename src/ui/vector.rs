//! Vector interface art (`hud/*.svg`) that replaces original sprites: most
//! of it remakes them, the map view button is a new design. Drawn in the
//! original's layout pixels so it replaces sprites one for one and can be
//! rasterized at the display's scale. Buttons hold their three states as
//! groups `normal`, `rollover` and `depress`, one shown at a time.

use resvg::{tiny_skia, usvg};

use super::art::{self, ArtFile, Group};

/// One SVG file, known by its name.
#[derive(Clone, Copy, Debug)]
pub struct Svg {
    pub name: &'static str,
    source: &'static str,
}

impl PartialEq for Svg {
    fn eq(&self, other: &Svg) -> bool {
        self.name == other.name
    }
}

impl Eq for Svg {}

impl std::hash::Hash for Svg {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.name.hash(h);
    }
}

macro_rules! svg {
    ($name:literal) => {
        Svg { name: $name, source: include_str!(concat!("../../hud/", $name, ".svg")) }
    };
}

/// The bottom bar, drawn as wide as the window: its root is
/// `width="1024"`, laid out relative to that.
pub const BOTTOM_BAR: Svg = svg!("bottom_bar");
pub const SNOWFLAKE: Svg = svg!("snowflake");
/// The map editor's panel under the view buttons.
pub const EDITOR_BOTTOM: Svg = svg!("editor_bottom");

/// The vector version of a three-state button, if it has one.
pub fn button(file: ArtFile, group: Group) -> Option<Svg> {
    if file != art::nav_bar::FILE {
        return None;
    }
    Some(match group {
        art::nav_bar::ZOOM_IN => svg!("zoom_in"),
        art::nav_bar::ZOOM_OUT => svg!("zoom_out"),
        art::nav_bar::ROTATE_CW => svg!("rotate_cw"),
        art::nav_bar::ROTATE_CCW => svg!("rotate_ccw"),
        art::nav_bar::MAP_VIEW => svg!("map_view"),
        _ => return None,
    })
}

/// Group shown for button state `index` (normal, rollover, depress).
pub fn state(index: usize) -> &'static str {
    ["normal", "rollover", "depress"][index.min(2)]
}

/// Draws `svg` (with only group `state` shown, when given) at `scale`
/// pixels per layout pixel. `width` replaces the layout width of a drawing
/// laid out relative to it (its root's `width="1024"`). Returns the image
/// and the drawing's size in layout pixels.
pub fn rasterize(svg: Svg, state: Option<&str>, width: Option<f32>, scale: f32) -> Option<(egui::ColorImage, egui::Vec2)> {
    let mut opt = usvg::Options::default();
    if let Some(s) = state {
        opt.style_sheet = Some(format!("#normal,#rollover,#depress{{display:none}} #{s}{{display:inline}}"));
    }
    // usvg only resolves percentages against an explicit root width.
    let source = match width {
        Some(w) => std::borrow::Cow::Owned(svg.source.replacen("width=\"1024\"", &format!("width=\"{w}\""), 1)),
        None => std::borrow::Cow::Borrowed(svg.source),
    };
    let tree = usvg::Tree::from_str(&source, &opt).map_err(|e| log::warn!("{}.svg: {e}", svg.name)).ok()?;
    let size = egui::Vec2::new(tree.size().width(), tree.size().height());
    let (w, h) = (((size.x * scale).round() as u32).max(1), ((size.y * scale).round() as u32).max(1));
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let t = tiny_skia::Transform::from_scale(w as f32 / size.x, h as f32 / size.y);
    resvg::render(&tree, t, &mut pixmap.as_mut());
    // tiny-skia's pixels are premultiplied, as egui's are.
    let pixels = pixmap
        .pixels()
        .iter()
        .map(|p| egui::Color32::from_rgba_premultiplied(p.red(), p.green(), p.blue(), p.alpha()))
        .collect();
    Some((egui::ColorImage { size: [w as usize, h as usize], source_size: egui::Vec2::new(w as f32, h as f32), pixels }, size))
}
