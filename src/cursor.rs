//! The mouse cursors, installed as native window cursors. Each is a
//! vector remake (`art/cursors/*.svg`) of the original sprite in
//! `data/ui/global/MouseCursor.bin`, drawn in the sprite's pixel units so
//! it can be rasterized at the interface's scale; the hotspots still come
//! from the original file. The original sprite stands in if a remake
//! can't be drawn.

use std::collections::HashMap;

use resvg::{tiny_skia, usvg};
use winit::event_loop::ActiveEventLoop;
use winit::window::{CustomCursor, Window};

use crate::assets::Assets;
use crate::formats::texture::Rgba;
use crate::formats::ui::Screen;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorKind {
    Arrow,
    NoCash,
    TooLong,
    Saw,
    Survey,
    Rotate,
    Pan,
    /// Carrying a guest.
    Hand,
}

const KINDS: [CursorKind; 8] = [
    CursorKind::Arrow,
    CursorKind::NoCash,
    CursorKind::TooLong,
    CursorKind::Saw,
    CursorKind::Survey,
    CursorKind::Rotate,
    CursorKind::Pan,
    CursorKind::Hand,
];

impl CursorKind {
    /// Sprite name in the cursor file.
    fn sprite(self) -> &'static str {
        match self {
            CursorKind::Arrow => "Cursor",
            CursorKind::NoCash => "No_cash",
            CursorKind::TooLong => "TO_LONG",
            CursorKind::Saw => "saw",
            CursorKind::Survey => "SurveyCursor",
            CursorKind::Rotate => "Cursor1",
            CursorKind::Pan => "Cursor2",
            CursorKind::Hand => "Hand_cursor",
        }
    }

    /// The vector remake.
    fn svg(self) -> &'static str {
        match self {
            CursorKind::Arrow => include_str!("../art/cursors/arrow.svg"),
            CursorKind::NoCash => include_str!("../art/cursors/no_cash.svg"),
            CursorKind::TooLong => include_str!("../art/cursors/too_long.svg"),
            CursorKind::Saw => include_str!("../art/cursors/saw.svg"),
            CursorKind::Survey => include_str!("../art/cursors/survey.svg"),
            CursorKind::Rotate => include_str!("../art/cursors/rotate.svg"),
            CursorKind::Pan => include_str!("../art/cursors/pan.svg"),
            CursorKind::Hand => include_str!("../art/cursors/hand.svg"),
        }
    }
}

/// One cursor as the original file has it.
struct Original {
    image: Rgba,
    hotspot: (u32, u32),
}

/// The original cursors with their hotspots.
fn originals(assets: &Assets) -> HashMap<CursorKind, Original> {
    let mut originals = HashMap::new();
    let screen = assets
        .resolve("data/ui/global/MouseCursor.bin")
        .and_then(|p| Screen::load(p).map_err(|e| log::warn!("{e:#}")).ok());
    let Some(screen) = screen else { return originals };
    let group = screen.group("MouseCursor");
    for kind in KINDS {
        let Some(si) = screen.sprites.iter().position(|s| s.name == kind.sprite()) else { continue };
        let Some(img) = &screen.sprites[si].image else { continue };
        // The layer offset is the sprite's position relative to the
        // pointer, so the hotspot is its negation.
        let (hx, hy) = group
            .and_then(|g| g.elements.iter().find(|e| e.sprite == Some(si)))
            .and_then(|e| e.layers.first())
            .map(|l| ((-l.dst.0).max(0) as u32, (-l.dst.1).max(0) as u32))
            .unwrap_or((0, 0));
        let hotspot = (hx.min(img.width - 1), hy.min(img.height - 1));
        originals.insert(kind, Original { image: img.clone(), hotspot });
    }
    originals
}

pub struct Cursors {
    originals: HashMap<CursorKind, Original>,
    cursors: HashMap<CursorKind, CustomCursor>,
    /// The scale the cursors were last made at.
    scale: f32,
    current: Option<CursorKind>,
}

impl Cursors {
    pub fn load(el: &ActiveEventLoop, assets: &Assets, scale: f32) -> Cursors {
        let mut cursors = Cursors { originals: originals(assets), cursors: HashMap::new(), scale: 0.0, current: None };
        cursors.rescale(el, scale);
        cursors
    }

    /// Remakes the cursors for an interface `scale` (physical pixels per
    /// pixel of the original's 1024x768 layout), in quarter steps and
    /// never below the original size.
    pub fn rescale(&mut self, el: &ActiveEventLoop, scale: f32) {
        let scale = ((scale * 4.0).round() / 4.0).max(1.0);
        if scale == self.scale {
            return;
        }
        self.scale = scale;
        self.cursors.clear();
        for (&kind, orig) in &self.originals {
            let made = remake(kind, orig, scale).unwrap_or_else(|| {
                log::warn!("cursor {}: drawing the original sprite", kind.sprite());
                (orig.image.clone(), orig.hotspot)
            });
            let (img, (hx, hy)) = made;
            match CustomCursor::from_rgba(img.pixels, img.width as u16, img.height as u16, hx as u16, hy as u16) {
                Ok(src) => {
                    self.cursors.insert(kind, el.create_custom_cursor(src));
                }
                Err(e) => log::warn!("cursor {}: {e}", kind.sprite()),
            }
        }
        self.current = None;
    }

    /// Forgets the applied cursor so the next `set` re-applies it. egui
    /// resets the window cursor whenever the pointer re-enters the window.
    pub fn invalidate(&mut self) {
        self.current = None;
    }

    /// Switches the window cursor when the wanted kind changes.
    pub fn set(&mut self, window: &Window, kind: CursorKind) {
        if self.current == Some(kind) {
            return;
        }
        let pick = self.cursors.get(&kind).or_else(|| self.cursors.get(&CursorKind::Arrow));
        if let Some(c) = pick {
            window.set_cursor(c.clone());
            self.current = Some(kind);
        }
    }
}

/// The remake drawn `scale` times the original's size, with the
/// original's hotspot scaled to match.
fn remake(kind: CursorKind, orig: &Original, scale: f32) -> Option<(Rgba, (u32, u32))> {
    let tree = usvg::Tree::from_str(kind.svg(), &usvg::Options::default())
        .map_err(|e| log::warn!("cursor {}: {e}", kind.sprite()))
        .ok()?;
    let (w, h) = ((orig.image.width as f32 * scale).ceil() as u32, (orig.image.height as f32 * scale).ceil() as u32);
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let sx = orig.image.width as f32 * scale / tree.size().width();
    let sy = orig.image.height as f32 * scale / tree.size().height();
    resvg::render(&tree, tiny_skia::Transform::from_scale(sx, sy), &mut pixmap.as_mut());
    let pixels = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    let hotspot = |v: u32, max: u32| ((v as f32 * scale).round() as u32).min(max - 1);
    let hotspot = (hotspot(orig.hotspot.0, w), hotspot(orig.hotspot.1, h));
    Some((Rgba { width: w, height: h, pixels, name: kind.sprite().into() }, hotspot))
}
