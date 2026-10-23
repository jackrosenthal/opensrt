//! The fog dialog (CFogSettingsDialog, `fog.bin`; 0x44ad50, 0x44af30):
//! how far to see in the follow view, before it starts. Dense, medium,
//! sparse or none; there is no way out but to choose.

use egui::{Rect as ERect, Sense, Vec2};

use super::art::{self, ArtFile};
use super::classic::{Art, Canvas};
use crate::assets::Assets;

const FOG: ArtFile = art::fog::FILE;
const SIZE: Vec2 = Vec2::new(231.0, 264.0);

/// The four buttons and the distance each sets (0x69e844).
const CHOICES: [(art::Group, f32); 4] = [
    (art::fog::FOG100, 100.0),
    (art::fog::FOG250, 200.0),
    (art::fog::FOG500, 350.0),
    (art::fog::NO_FOG, 50000.0),
];

/// Draws the dialog in the middle of the screen; returns the distance
/// chosen, if one was.
pub fn show(ui: &mut egui::Ui, art: &mut Art, assets: &Assets, blocks: &mut Vec<ERect>) -> Option<f32> {
    let screen = ui.ctx().content_rect();
    let s = (screen.height() / 768.0).min(screen.width() / 1024.0);
    let cv = Canvas {
        origin: screen.center() - SIZE * 0.5 * s,
        scale: s,
        ppp: ui.ctx().pixels_per_point(),
    };
    blocks.push(screen);
    ui.interact(screen, ui.scope_id().with("fog modal"), Sense::click());
    art.draw(ui, assets, cv, FOG, art::fog::BACKGROUND, 0, 0.0, 0.0);
    let mut chosen = None;
    for (group, sight) in CHOICES {
        if art.button(ui, assets, cv, FOG, group, (0.0, 0.0)) {
            chosen = Some(sight);
        }
    }
    chosen
}
