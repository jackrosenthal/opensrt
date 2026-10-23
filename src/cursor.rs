//! The original mouse cursors from `data/ui/global/MouseCursor.bin`,
//! installed as native window cursors.

use std::collections::HashMap;

use winit::event_loop::ActiveEventLoop;
use winit::window::{CustomCursor, Window};

use crate::assets::Assets;
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
}

pub struct Cursors {
    cursors: HashMap<CursorKind, CustomCursor>,
    current: Option<CursorKind>,
}

impl Cursors {
    pub fn load(el: &ActiveEventLoop, assets: &Assets) -> Cursors {
        let mut cursors = HashMap::new();
        let screen = assets
            .resolve("data/ui/global/MouseCursor.bin")
            .and_then(|p| Screen::load(p).map_err(|e| log::warn!("{e:#}")).ok());
        if let Some(screen) = screen {
            let group = screen.group("MouseCursor");
            for kind in [
                CursorKind::Arrow,
                CursorKind::NoCash,
                CursorKind::TooLong,
                CursorKind::Saw,
                CursorKind::Survey,
                CursorKind::Rotate,
                CursorKind::Pan,
                CursorKind::Hand,
            ] {
                let Some(si) = screen.sprites.iter().position(|s| s.name == kind.sprite()) else { continue };
                let Some(img) = &screen.sprites[si].image else { continue };
                // The layer offset is the sprite's position relative to
                // the pointer, so the hotspot is its negation.
                let (hx, hy) = group
                    .and_then(|g| g.elements.iter().find(|e| e.sprite == Some(si)))
                    .and_then(|e| e.layers.first())
                    .map(|l| ((-l.dst.0).max(0), (-l.dst.1).max(0)))
                    .unwrap_or((0, 0));
                let hx = (hx as u32).min(img.width - 1) as u16;
                let hy = (hy as u32).min(img.height - 1) as u16;
                match CustomCursor::from_rgba(img.pixels.clone(), img.width as u16, img.height as u16, hx, hy) {
                    Ok(src) => {
                        cursors.insert(kind, el.create_custom_cursor(src));
                    }
                    Err(e) => log::warn!("cursor {}: {e}", kind.sprite()),
                }
            }
        }
        Cursors { cursors, current: None }
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
