//! The map editor's Generate Random Map window (CGenerateRandomMap,
//! `generate_random_map.bin`; set up at 0x44c700, run by 0x44f850): nine
//! anchor points to drag up and down, the seed, rockiness, tree density
//! and treeline, and buttons to make the terrain, shake the anchors at
//! random, and plant trees.

use egui::{FontFamily, FontId, Rect as ERect, Sense, Stroke, Vec2};

use super::art::{self, ArtFile, Group};
use super::classic::{Art, Canvas, LIST};
use super::fonts::ARIAL_BOLD;
use crate::assets::Assets;
use crate::game::Game;
use crate::game::editor::{self, GenSettings};

const GRM: ArtFile = art::gen_map::FILE;
const SIZE: Vec2 = Vec2::new(559.0, 399.0);

/// The anchors' places on the window, in its order (table 0x614ba8).
const ANCHORS: [(f32, f32); 9] = [
    (398.0, 131.0),
    (363.0, 156.0),
    (328.0, 181.0),
    (443.0, 146.0),
    (488.0, 161.0),
    (408.0, 171.0),
    (373.0, 196.0),
    (418.0, 211.0),
    (453.0, 181.0),
];
/// The wireframe between the anchors: the outline through the edge
/// middles, and the two lines through the middle.
const WIRES: [(usize, usize); 12] = [(0, 1), (1, 2), (0, 3), (3, 4), (2, 6), (6, 7), (4, 8), (8, 7), (1, 5), (5, 8), (3, 5), (5, 6)];
/// How far an anchor moves either way; the height is 5 a step, 0 to 500.
const TRAVEL: f32 = 50.0;
/// The number boxes (left, top, right, bottom): seed, rockiness, tree
/// density, treeline.
const FIELDS: [(f32, f32, f32, f32); 4] = [
    (149.0, 51.0, 226.0, 64.0),
    (149.0, 83.0, 226.0, 96.0),
    (149.0, 115.0, 226.0, 128.0),
    (149.0, 147.0, 226.0, 160.0),
];

pub struct GenWindow {
    /// Each anchor's offset, down positive, -50 to 50.
    dy: [f32; 9],
    fields: [String; 4],
    flatten: bool,
    /// rand() as last seeded from the Seed box (0x44f850 case 3 seeds it
    /// again only when the number changes).
    rand: Box<dyn FnMut() -> u32>,
    seeded: i32,
}

/// The map's samples under the anchors: the corners as they lie on the
/// screen (top, left, right, bottom), the middles between them and the
/// map's middle (0x4194d0 places them by the view's quarter turn).
fn samples(g: &Game) -> [(usize, usize); 9] {
    let lv = g.world.level();
    let n = lv.cells_per_side();
    let size = glam::Vec2::new(1000.0, 1000.0);
    let mut corners: Vec<((usize, usize), glam::Vec2)> = [(0, 0), (n, 0), (0, n), (n, n)]
        .into_iter()
        .map(|(x, y)| {
            let p = glam::Vec3::new(x as f32 * lv.spacing, y as f32 * lv.spacing, lv.height(x, y));
            ((x, y), g.camera.project(p, size).unwrap_or(glam::Vec2::ZERO))
        })
        .collect();
    corners.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
    let (top, bottom) = (corners[0].0, corners[3].0);
    let (left, right) = if corners[1].1.x < corners[2].1.x { (corners[1].0, corners[2].0) } else { (corners[2].0, corners[1].0) };
    editor::anchor_samples(n, top, left, right, bottom)
}

impl GenWindow {
    /// Opens on the map as it is: the anchors at its heights, the numbers
    /// as the editor keeps them.
    pub fn open(g: &Game) -> GenWindow {
        let lv = g.world.level();
        let dy = samples(g).map(|(x, y)| (TRAVEL - lv.height(x, y) * 0.2).trunc().clamp(-TRAVEL, TRAVEL));
        let s = g.world.map_gen;
        GenWindow {
            dy,
            fields: [s.seed, s.rockiness, s.tree_density, s.treeline].map(|v| v.to_string()),
            flatten: false,
            rand: Box::new(editor::crand_seeded(s.seed as u32)),
            seeded: s.seed,
        }
    }

    fn number(&self, i: usize) -> i32 {
        self.fields[i].trim().parse().unwrap_or(0)
    }

    /// The numbers back into the editor (0x44c5f0).
    fn keep(&self, g: &mut Game) {
        g.world.map_gen = GenSettings {
            seed: self.number(0),
            rockiness: self.number(1),
            tree_density: self.number(2),
            treeline: self.number(3),
        };
    }

    /// Draws the window centered over the game. Returns false once closed.
    pub fn show(&mut self, ui: &mut egui::Ui, art: &mut Art, assets: &Assets, g: &mut Game, blocks: &mut Vec<ERect>) -> bool {
        let screen = ui.ctx().content_rect();
        let s = (screen.height() / 768.0).min(screen.width() / 1024.0);
        let cv = Canvas {
            origin: screen.center() - SIZE * 0.5 * s,
            scale: s,
            ppp: ui.ctx().pixels_per_point(),
        };
        let whole = cv.rect_xywh(0.0, 0.0, SIZE.x, SIZE.y);
        blocks.push(whole);
        ui.interact(whole, ui.scope_id().with("genmap body"), Sense::click());
        art.draw(ui, assets, cv, GRM, art::gen_map::BACKGROUND, 0, 0.0, 0.0);

        let font = FontId::new(LIST.size * s, FontFamily::Name(ARIAL_BOLD.into()));
        for (i, (x1, y1, x2, y2)) in FIELDS.into_iter().enumerate() {
            let edit = egui::TextEdit::singleline(&mut self.fields[i])
                .font(font.clone())
                .text_color(LIST.color)
                .frame(egui::Frame::NONE)
                .char_limit(10);
            ui.put(cv.rect_xywh(x1, y1 - 3.0, x2 - x1, y2 - y1 + 6.0), edit);
            self.fields[i].retain(|c| c.is_ascii_digit() || (i == 0 && c == '-'));
        }

        // The wireframe, then the anchors on it, each dragged up or down.
        let at = |i: usize, dy: f32| cv.pos(ANCHORS[i].0, ANCHORS[i].1 + dy);
        let wire = Stroke::new(1.5 * s, LIST.color);
        for (a, b) in WIRES {
            ui.painter().line_segment([at(a, self.dy[a]), at(b, self.dy[b])], wire);
        }
        for i in 0..9 {
            let (x, y) = ANCHORS[i];
            art.draw(ui, assets, cv, GRM, art::gen_map::ANCHOR_POINT, 0, x - 3.0, y - 3.0);
            ui.painter().line_segment([cv.pos(x, y), at(i, self.dy[i])], Stroke::new(1.0 * s, LIST.color.gamma_multiply(0.5)));
            let tab = cv.rect_xywh(x - 4.0, y + self.dy[i] - 4.0, 16.0, 16.0);
            let resp = ui.interact(tab, ui.scope_id().with(("anchor", i)), Sense::drag());
            if resp.dragged() {
                self.dy[i] = (self.dy[i] + resp.drag_delta().y / s).clamp(-TRAVEL, TRAVEL);
            }
            art.draw(ui, assets, cv, GRM, art::gen_map::SCROLL_TAB, 0, x, y + self.dy[i]);
        }

        let check = (238.0, 344.0);
        let r = cv.rect_xywh(check.0, check.1, 27.0, 27.0);
        if ui.interact(r, ui.scope_id().with("flatten"), Sense::click()).clicked() {
            self.flatten = !self.flatten;
        }
        art.draw(ui, assets, cv, GRM, art::gen_map::FLATTEN_ENTRANCE, usize::from(self.flatten), check.0, check.1);

        let button = |art: &mut Art, ui: &mut egui::Ui, group: Group, x: f32, y: f32| art.button(ui, assets, cv, GRM, group, (x, y));
        if button(art, ui, art::gen_map::GENERATE_BUTTON, 449.0, 335.0) {
            // Out-of-range percentages are put right first (0x44f850 case 1).
            for i in [1, 2] {
                self.fields[i] = self.number(i).clamp(0, 100).to_string();
            }
            let anchors: Vec<((usize, usize), f32)> = samples(g).into_iter().zip(self.dy).map(|(p, dy)| (p, (TRAVEL - dy) * 5.0)).collect();
            let anchors: [((usize, usize), f32); 9] = anchors.try_into().unwrap();
            let (rock, flatten) = (self.number(1), self.flatten);
            g.editor_remap(|lv, rand| editor::generate_terrain(lv, &anchors, rock, flatten, rand));
            self.keep(g);
        }
        if button(art, ui, art::gen_map::RAND_WIREFRAME_BUTTON, 227.0, 293.0) {
            let seed = self.number(0);
            if seed != self.seeded {
                self.rand = Box::new(editor::crand_seeded(seed as u32));
                self.seeded = seed;
            }
            for dy in &mut self.dy {
                let up = (self.rand)() & 1 == 1;
                let v = ((self.rand)() as f32 / 32767.0 * TRAVEL).trunc();
                *dy = if up { v } else { -v };
            }
        }
        if button(art, ui, art::gen_map::GEN_TREES, 348.0, 335.0) {
            self.fields[2] = self.number(2).clamp(0, 100).to_string();
            let (density, treeline, flatten) = (self.number(2), self.number(3), self.flatten);
            g.editor_remap(|lv, rand| editor::generate_trees(lv, density, treeline, flatten, rand));
            self.keep(g);
        }
        if button(art, ui, art::gen_map::CLOSE_BUTTON, 520.0, 13.0) || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.keep(g);
            return false;
        }
        true
    }
}
