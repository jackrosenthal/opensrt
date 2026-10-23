//! The message dialog with Bob (`bob.bin`), used for challenge messages,
//! and the Bob button that brings a challenge's last message back.
//! Text area (242, 72)–(526, 298) in font 10. The OK button's layout
//! rectangle is in 640×480 screen terms, where the background sits
//! centered at (35, 56), and the original offsets it by (-5, -6), so over
//! the background it is at (-40, -62) from its rectangle: on the OK drawn
//! into the background.

use egui::{Color32, FontFamily, FontId, Rect as ERect, Sense, Vec2};

use super::art::{self, ArtFile};
use super::classic::{Art, Canvas};
use super::fonts::ARIAL;
use crate::assets::Assets;

const BOB: ArtFile = art::bob::FILE;
const SIZE: Vec2 = Vec2::new(570.0, 367.0);
const TEXT: (f32, f32, f32, f32) = (242.0, 72.0, 526.0, 298.0);

/// Bob's expression: the four backgrounds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {
    Happy = 0,
    Normal = 1,
    Pointing = 2,
    Unhappy = 3,
}

pub struct BobMessage {
    pub mood: Mood,
    pub text: String,
    /// The challenge step this message belongs to; OK moves it on.
    pub step: Option<u32>,
}

impl Mood {
    /// Bob's look by background index, as the challenges give it.
    pub fn from_index(i: u8) -> Mood {
        match i {
            0 => Mood::Happy,
            1 => Mood::Normal,
            2 => Mood::Pointing,
            _ => Mood::Unhappy,
        }
    }
}

/// The Bob button a challenge shows while it watches for its goal
/// (0x440420 mode 1), centered above the bottom bar. Returns true when
/// clicked.
pub fn button(ui: &mut egui::Ui, art: &mut Art, assets: &Assets, blocks: &mut Vec<ERect>) -> bool {
    // On the HUD's 1024×768 canvas, as wide as the window.
    let screen = ui.ctx().content_rect();
    let s = (screen.height() / 768.0).min(screen.width() / 1024.0);
    let cv = Canvas {
        origin: screen.min,
        scale: s,
        ppp: ui.ctx().pixels_per_point(),
    };
    let (w, h) = (51.0, 50.0);
    let (x, y) = ((screen.width() / s - w) * 0.5, screen.height() / s - 44.0 - h);
    blocks.push(cv.rect_xywh(x, y, w, h));
    art.button(ui, assets, cv, BOB, art::bob::BOB_BUTTON, (x, y))
}

/// Draws the dialog centered over the game. Returns true when OK is
/// clicked.
pub fn show(m: &BobMessage, ui: &mut egui::Ui, art: &mut Art, assets: &Assets, blocks: &mut Vec<ERect>) -> bool {
    let screen = ui.ctx().content_rect();
    let s = (screen.height() / 768.0).min(screen.width() / 1024.0) * 1.2;
    let cv = Canvas {
        origin: screen.center() - SIZE * 0.5 * s,
        scale: s,
        ppp: ui.ctx().pixels_per_point(),
    };
    blocks.push(screen);
    // The dialog is modal.
    ui.interact(screen, ui.scope_id().with("bob modal"), Sense::click());
    art.draw(ui, assets, cv, BOB, art::bob::BACKGROUND, m.mood as usize, 0.0, 0.0);
    let (x1, y1, x2, y2) = TEXT;
    let font = FontId::new(11.0 * s, FontFamily::Name(ARIAL.into()));
    let color = Color32::from_rgb(0x00, 0x00, 0x99);
    let text = m.text.replace("<BR>", "\n").replace("<br>", "\n");
    let galley = ui.painter().layout(text, font, color, (x2 - x1) * s);
    let clip = cv.rect_xywh(x1, y1, x2 - x1, y2 - y1);
    ui.painter().with_clip_rect(clip).galley(clip.min, galley, color);
    let ok = art.button(ui, assets, cv, BOB, art::bob::OK, (-40.0, -62.0));
    ok || ui.input(|i| i.key_pressed(egui::Key::Enter))
}
