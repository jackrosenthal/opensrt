//! The mini-map (`mini_map_window.bin`): a 234×153 panel with the map in
//! (47, 13)–(176, 142) and the filter buttons from the original table.

use egui::{Color32, Pos2, Rect as ERect, Sense, Stroke, Vec2};

use super::art::{self, ArtFile};
use super::classic::{Art, Canvas};
use crate::assets::Assets;
use crate::game::Game;

const MAP: ArtFile = art::mini_map::FILE;
const AREA: (f32, f32, f32, f32) = (47.0, 13.0, 177.0, 143.0);
const SIZE: Vec2 = Vec2::new(235.0, 154.0);

pub struct Minimap {
    pub structures: bool,
    pub skiers: bool,
    pub lifts: bool,
    pub terrain: bool,
    texture: Option<egui::TextureHandle>,
    built_at: f64,
}

impl Default for Minimap {
    fn default() -> Minimap {
        Minimap {
            structures: true,
            skiers: true,
            lifts: true,
            terrain: true,
            texture: None,
            built_at: f64::NEG_INFINITY,
        }
    }
}

/// The level from above, one pixel per cell. The original renders the
/// terrain top-down under the scene lights (0x45b930), forest in a flat
/// green; its 16-bit surface swaps red and blue, which shows only at
/// night, when the snow goes olive.
fn terrain_image(g: &Game) -> egui::ColorImage {
    use crate::game::daylight::*;
    let lv = g.world.level();
    let n = lv.cells_per_side();
    let l = g.world.daylight.light();
    let sun = SUN_NIGHT + (SUN_DAY - SUN_NIGHT) * l;
    let ambient = glam::Vec3::from(AMBIENT_NIGHT).lerp(glam::Vec3::splat(AMBIENT), l);
    let to_sun = -glam::Vec3::from(SUN_DIR).normalize();
    // The snow texture's average, in sRGB.
    const SNOW: f32 = 245.0;
    let mut px = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let c = lv.cell(x, y);
            if c.foliage != 0 {
                px.push(Color32::from_rgb(24, 130, 24));
                continue;
            }
            let p = (x as f32 + 0.5) * lv.spacing;
            let q = (y as f32 + 0.5) * lv.spacing;
            let d = lv.normal_at(p, q).dot(to_sun).max(0.0);
            let lit = (ambient + glam::Vec3::splat(sun * d)) * SNOW;
            let ch = |v: f32| v.min(255.0) as u8;
            px.push(Color32::from_rgb(ch(lit.z), ch(lit.y), ch(lit.x)));
        }
    }
    egui::ColorImage::new([n, n], px)
}

/// Draws the mini-map at the bottom right, above the bottom bar. In the
/// map editor it has the Generate Map button instead of the filters
/// (0x45b180); returns true when that is clicked.
pub fn show(m: &mut Minimap, ui: &mut egui::Ui, art: &mut Art, assets: &Assets, g: &mut Game, blocks: &mut Vec<ERect>) -> bool {
    let screen = ui.ctx().content_rect();
    let s = (screen.height() / 768.0).min(screen.width() / 1024.0);
    let origin = Pos2::new(screen.max.x - (SIZE.x + 4.0) * s, screen.max.y - (60.0 + SIZE.y + 2.0) * s);
    let cv = Canvas {
        origin,
        scale: s,
        ppp: ui.ctx().pixels_per_point(),
    };
    art.draw(ui, assets, cv, MAP, art::mini_map::BACKGROUND, 0, 0.0, 0.0);
    let whole = cv.rect_xywh(0.0, 0.0, SIZE.x, SIZE.y);
    blocks.push(whole);
    let now = ui.input(|i| i.time);
    if m.texture.is_none() || now - m.built_at > 2.0 {
        let img = terrain_image(g);
        match &mut m.texture {
            Some(t) => t.set(img, egui::TextureOptions::LINEAR),
            None => m.texture = Some(ui.ctx().load_texture("minimap", img, egui::TextureOptions::LINEAR)),
        }
        m.built_at = now;
    }
    let (x1, y1, x2, y2) = AREA;
    let area = cv.rect_xywh(x1, y1, x2 - x1, y2 - y1);
    // The map turned so the view's direction points up, a diamond at the
    // usual quarter turns, on black.
    ui.painter().rect_filled(area, 0.0, Color32::BLACK);
    let ext = g.world.level().extent();
    let (fwd, _) = g.camera.ground_axes();
    let fwd = glam::Vec2::new(fwd.x, fwd.y).normalize_or(glam::Vec2::Y);
    let right = glam::Vec2::new(fwd.y, -fwd.x);
    let k = area.width() / (ext * std::f32::consts::SQRT_2);
    let mid = glam::Vec2::splat(ext * 0.5);
    let to_map = |p: glam::Vec2| {
        let v = p - mid;
        area.center() + Vec2::new(v.dot(right), -v.dot(fwd)) * k
    };
    let from_map = |q: Pos2| {
        let d = (q - area.center()) / k;
        mid + right * d.x - fwd * d.y
    };
    if m.terrain {
        if let Some(t) = &m.texture {
            let mut mesh = egui::Mesh::with_texture(t.id());
            for (u, v) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
                let pos = to_map(glam::Vec2::new(u * ext, v * ext));
                mesh.vertices.push(egui::epaint::Vertex { pos, uv: Pos2::new(u, v), color: Color32::WHITE });
            }
            mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
            ui.painter().with_clip_rect(area).add(egui::Shape::mesh(mesh));
        }
    }
    let at = |p: glam::Vec3| to_map(p.truncate());
    let painter = ui.painter().with_clip_rect(area);
    if m.lifts {
        for l in &g.world.lifts {
            painter.line_segment([at(l.start), at(l.end)], Stroke::new(1.5 * s, Color32::BLACK));
        }
    }
    if m.structures {
        for st in &g.world.structures {
            painter.rect_filled(ERect::from_center_size(at(st.pos), Vec2::splat(3.0 * s)), 0.0, Color32::from_rgb(0xc0, 0x20, 0x20));
        }
    }
    if m.skiers {
        for x in g.world.guests.iter().filter(|x| x.pos.z > -100.0) {
            painter.rect_filled(ERect::from_center_size(at(x.pos), Vec2::splat(1.5 * s)), 0.0, Color32::from_rgb(0x20, 0x20, 0xc0));
        }
    }
    // What the view shows: the screen's corners on the ground, at the
    // height the camera looks at.
    let z = g.camera.target.z;
    let corners: Vec<Pos2> = [(-1.0, 1.0), (1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)]
        .iter()
        .filter_map(|&(x, y)| {
            let (o, d) = g.camera.ray(glam::Vec2::new(x, y));
            (d.z.abs() > 1e-4).then(|| at(o + d * ((z - o.z) / d.z)))
        })
        .collect();
    if corners.len() == 4 {
        let mut pts = corners;
        pts.push(pts[0]);
        painter.add(egui::Shape::line(pts, Stroke::new(1.0 * s, Color32::from_rgb(0x29, 0xcd, 0x20))));
    }
    let resp = ui.interact(area, ui.scope_id().with("minimap"), Sense::click_and_drag());
    if resp.clicked() || resp.dragged() {
        if let Some(p) = resp.interact_pointer_pos() {
            let w = from_map(p).clamp(glam::Vec2::ZERO, glam::Vec2::splat(ext));
            g.camera.target.x = w.x;
            g.camera.target.y = w.y;
        }
    }
    ui.interact(whole, ui.scope_id().with("minimap body"), Sense::click());
    if g.world.editor {
        return art.button(ui, assets, cv, MAP, art::mini_map::RANDOM_GEN, (188.0, 17.0));
    }
    // Filters, as in the original table.
    for (group, x, y, flag) in [
        (art::mini_map::FILTER_STRUCTS, 9.0, 23.0, &mut m.structures),
        (art::mini_map::FILTER_SKIERS, 9.0, 52.0, &mut m.skiers),
        (art::mini_map::FILTER_LIFTS, 9.0, 79.0, &mut m.lifts),
        (art::mini_map::FILTER_ALL, 9.0, 108.0, &mut m.terrain),
    ] {
        if art.button(ui, assets, cv, MAP, group, (x, y)) {
            *flag = !*flag;
        }
    }
    false
}
