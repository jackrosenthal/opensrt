//! The build pickers (`structures_toolbar.bin`, `lifts_toolbar.bin`): a
//! draggable dialog with previous/current/next model previews, a scroller,
//! a description box and Build/Rotate buttons. Layout from the original
//! picker setup code.

use egui::{Color32, FontFamily, FontId, Pos2, Rect as ERect, Sense, Vec2};

use super::art::{self, ArtFile};
use super::classic::{Art, BODY, Canvas, TextStyle};
use super::fonts::{ARIAL, ARIAL_BOLD};
use super::BUILD_CATEGORIES;
use crate::game::defs::{Requirement, StructureKind};
use crate::assets::Assets;
use crate::game::{Game, Tool};
use crate::text::Text;

const STRUCTURES: ArtFile = art::structures_toolbar::FILE;
const LIFTS: ArtFile = art::lifts_toolbar::FILE;
const FLUFF: ArtFile = art::fluff_menu::FILE;
/// The objects menu puts its scroller and rotate button elsewhere.
const FLUFF_SCROLLER: (f32, f32) = (25.0, 150.0);
const FLUFF_ROTATE: (f32, f32) = (109.0, 181.0);

/// Preview boxes: previous, current, next.
const PREVIEWS: [(f32, f32, f32, f32); 3] = [(19.0, 44.0, 64.0, 138.0), (79.0, 44.0, 169.0, 138.0), (184.0, 44.0, 230.0, 138.0)];
const SCROLLER: (f32, f32) = (45.0, 150.0);
const COUNT_LABEL: (f32, f32, f32, f32) = (86.0, 155.0, 161.0, 172.0);
const TEXT: (f32, f32, f32, f32) = (19.0, 193.0, 164.0, 279.0);
const TEXT_SCROLL: (f32, f32) = (174.0, 188.0);
const BUILD: (f32, f32) = (201.0, 202.0);
const ROTATE: (f32, f32) = (201.0, 241.0);
/// Title bar height that drags the dialog.
const DRAG_H: f32 = 40.0;

/// Types the maintenance tray adds on top of its list.
const MAINTENANCE_EXTRA: [StructureKind; 2] = [StructureKind::Snowmaker, StructureKind::AvalancheControl];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PickKind {
    Buildings(usize),
    Lifts,
    Jumps,
    Runs,
    Objects,
    Paths,
}

pub struct Picker {
    pub kind: PickKind,
    pub index: usize,
    pub view: u8,
    /// Dialog position in layout pixels from the window's top-left; `None`
    /// until first shown.
    pub pos: Option<Vec2>,
    pub text_scroll: usize,
}

impl Picker {
    pub fn new(kind: PickKind) -> Picker {
        Picker {
            kind,
            index: 0,
            view: 0,
            pos: None,
            text_scroll: 0,
        }
    }
}

/// One entry of a picker.
struct Entry {
    name: String,
    cost: f32,
    desc: String,
    missing: Vec<String>,
    mesh: String,
    /// Art shown instead of a model: the element of `path` (center box)
    /// and `halfpath` (side boxes) for the paths and their eraser.
    icon: Option<usize>,
    /// The zoom level its sprite is shown at, one pixel to a pixel.
    zoom: Option<u8>,
    tool: Tool,
}

fn entries(kind: PickKind, g: &Game) -> Vec<Entry> {
    let defs = &g.defs;
    let name = |key: &str, fallback: &str| -> String {
        match defs.strings.get(key) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => fallback.to_string(),
        }
    };
    let missing = |reqs: &[Requirement]| -> Vec<String> {
        reqs.iter().filter(|r| !g.unlocked(std::slice::from_ref(r))).map(|r| r.name(defs).to_string()).collect()
    };
    match kind {
        PickKind::Buildings(k) => {
            let (_, types) = BUILD_CATEGORIES[k.min(7)];
            let extra: &[StructureKind] = if k == 7 { &MAINTENANCE_EXTRA } else { &[] };
            defs.structures
                .iter()
                .enumerate()
                .filter(|(_, d)| {
                    (types.contains(&d.kind) || extra.contains(&d.kind)) && !d.is_upgrade
                })
                .map(|(i, d)| Entry {
                    name: defs.structure_name(d).to_string(),
                    cost: d.cost,
                    desc: defs.lookup(&d.desc_key).to_string(),
                    missing: missing(&d.predecessors),
                    mesh: d.meshes.day.clone(),
                    icon: None,
                    zoom: Some(d.preview_zoom),
                    tool: Tool::Structure { def: i, rot: 0 },
                })
                .collect()
        }
        PickKind::Lifts => defs
            .lifts
            .iter()
            .enumerate()
            .map(|(i, d)| Entry {
                name: name(&d.name_key, &d.unique_name),
                cost: d.base_cost + d.cost_per_pole,
                desc: defs.lookup(&d.desc_key).to_string(),
                missing: missing(&d.predecessors),
                mesh: d.begin.day.clone(),
                icon: None,
                zoom: Some(1),
                tool: Tool::Lift { def: i, start: None },
            })
            .collect(),
        PickKind::Jumps => defs
            .parks
            .iter()
            .enumerate()
            .map(|(i, d)| Entry {
                name: name(&d.name_key, &d.unique_name),
                cost: d.cost,
                desc: defs.lookup(&d.desc_key).to_string(),
                missing: missing(&d.predecessors),
                // The ramp, as the original's `JUMP_ramp` sprite shows a
                // jump; courses without one show their start.
                mesh: if d.piece(0, 3).is_empty() { d.piece(0, 0) } else { d.piece(0, 3) }.to_string(),
                icon: None,
                zoom: Some(1),
                tool: Tool::Park { def: i, start: None },
            })
            .collect(),
        PickKind::Objects => defs
            .decos
            .iter()
            .enumerate()
            .map(|(i, d)| Entry {
                name: name(&d.name_key, &d.unique_name),
                cost: d.cost,
                desc: String::new(),
                missing: Vec::new(),
                mesh: d.meshes.day.clone(),
                icon: None,
                zoom: Some(d.preview_zoom),
                tool: Tool::Deco { def: i, rot: 0.0 },
            })
            .collect(),
        // The eraser, then the paths (`FLUFF_MENU_PATHS_TITLE`); a cell
        // of path costs a dollar.
        PickKind::Paths => std::iter::once(Entry {
            name: String::new(),
            cost: 0.0,
            desc: String::new(),
            missing: Vec::new(),
            mesh: String::new(),
            icon: Some(0),
            zoom: None,
            tool: Tool::Path { kind: None },
        })
        .chain((0..4u8).map(|t| Entry {
            name: String::new(),
            cost: 1.0,
            desc: String::new(),
            missing: Vec::new(),
            mesh: String::new(),
            icon: Some(1 + t as usize),
            zoom: None,
            tool: Tool::Path { kind: Some(t) },
        }))
        .collect(),
        PickKind::Runs => defs
            .runs
            .iter()
            .enumerate()
            .map(|(i, d)| Entry {
                name: name(&d.name_key, &d.unique_name),
                cost: d.cost,
                desc: defs.lookup(&d.desc_key).to_string(),
                missing: missing(&d.predecessors),
                mesh: d.mesh.clone(),
                icon: None,
                zoom: Some(1),
                tool: Tool::Run { def: i, gates: Vec::new() },
            })
            .collect(),
    }
}

fn font(style: TextStyle, s: f32) -> FontId {
    FontId::new(style.size * s, FontFamily::Name(if style.bold { ARIAL_BOLD } else { ARIAL }.into()))
}

/// Preview textures, rendered by the app on request.
#[derive(Default)]
pub struct Previews {
    pub textures: std::collections::HashMap<(String, u8), Option<egui::TextureHandle>>,
    pub requests: Vec<(String, u8)>,
}

/// Appended to a preview's model to have the entrance's sign drawn on it.
pub const WITH_SIGN: &str = "#sign";

/// A window preview's view: the object as the game shows it at a zoom
/// level, from the camera's direction (64 steps around), rather than a
/// picker's turnaround.
pub fn world_view(zoom: u8, yaw: f32) -> u8 {
    let step = (yaw.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * 64.0).round() as u8 % 64;
    0x80 | (zoom.min(1) << 6) | step
}

impl Previews {
    pub(super) fn get(&mut self, mesh: &str, view: u8) -> Option<egui::TextureHandle> {
        let key = (mesh.to_string(), view);
        match self.textures.get(&key) {
            Some(t) => t.clone(),
            None => {
                if !self.requests.contains(&key) {
                    self.requests.push(key);
                }
                None
            }
        }
    }
}

pub enum PickerResult {
    Open,
    Close,
}

/// The quarter turns a picked model is placed at, so that from the
/// camera it faces as its preview did: previews look from π/4 + view·π/2
/// round the model.
fn placed_turns(yaw: f32, view: u8) -> i32 {
    let quarter = std::f32::consts::FRAC_PI_2;
    (((yaw - std::f32::consts::FRAC_PI_4) / quarter).round() as i32 - view as i32).rem_euclid(4)
}

/// Draws a picker. Returns `Close` when the close box is clicked.
#[allow(clippy::too_many_arguments)]
pub fn show(
    p: &mut Picker,
    ui: &mut egui::Ui,
    art: &mut Art,
    assets: &Assets,
    strings: &crate::formats::eng::Strings,
    previews: &mut Previews,
    g: &mut Game,
    blocks: &mut Vec<ERect>,
) -> PickerResult {
    let screen = ui.ctx().content_rect();
    let s = (screen.height() / 768.0).min(screen.width() / 1024.0);
    let rel = match p.kind {
        PickKind::Buildings(_) => STRUCTURES,
        PickKind::Objects | PickKind::Paths => FLUFF,
        _ => LIFTS,
    };
    let fluff = rel == FLUFF;
    let bg = art.hit_rect(assets, rel, art::picker::BACKGROUND, 0).map_or((250.0, 300.0), |r| (r.width() as f32, r.height() as f32));
    // Where the original opens them (measured at 640×480): building pickers
    // centered with their top at 68, the lift toolbar (lifts, jumps, runs)
    // at the left edge with its top at 138, and the objects and paths
    // menu in the bottom right corner, 2 from the right and 43 from the
    // bottom.
    let pos = *p.pos.get_or_insert_with(|| match rel {
        STRUCTURES => Vec2::new((screen.width() / s - bg.0) * 0.5, 68.0),
        LIFTS => Vec2::new(0.0, 138.0),
        _ => Vec2::new(screen.width() / s - bg.0 - 2.0, screen.height() / s - bg.1 - 43.0),
    });
    let cv = Canvas {
        origin: screen.min + pos * s,
        scale: s,
        ppp: ui.ctx().pixels_per_point(),
    };
    let items = entries(p.kind, g);
    if items.is_empty() {
        return PickerResult::Close;
    }
    p.index = p.index.min(items.len() - 1);
    // Background and dragging by the top band.
    art.draw(ui, assets, cv, rel, art::picker::BACKGROUND, 0, 0.0, 0.0);
    let whole = cv.rect_xywh(0.0, 0.0, bg.0, bg.1);
    blocks.push(whole);
    let drag = ui.interact(cv.rect_xywh(0.0, 0.0, bg.0 - 30.0, DRAG_H), ui.scope_id().with(("picker drag", rel)), Sense::drag());
    if drag.dragged() {
        p.pos = Some(pos + drag.drag_delta() / s);
    }
    // Swallow clicks on the dialog body.
    ui.interact(whole, ui.scope_id().with(("picker body", rel)), Sense::click());
    // Title.
    let title = match p.kind {
        PickKind::Buildings(k) => strings.text(BUILD_CATEGORIES[k.min(7)].0).into_owned(),
        PickKind::Lifts => strings.text(Text::LemLift).to_string(),
        PickKind::Jumps => strings.text(Text::LemJumps).to_string(),
        PickKind::Runs => strings.text(Text::LemRunmarker).to_string(),
        PickKind::Objects => strings.text(Text::FluffMenuObjectTitle).to_string(),
        PickKind::Paths => strings.text(Text::FluffMenuPathsTitle).to_string(),
    };
    let title = title.split(" (").next().unwrap_or("").to_string();
    let tstyle = TextStyle {
        size: 14.0,
        bold: true,
        color: Color32::from_rgb(0x00, 0x00, 0x99),
    };
    ui.painter().text(cv.pos(bg.0 * 0.5, 14.0), egui::Align2::CENTER_TOP, title, font(tstyle, s), tstyle.color);
    // Close box at the top right.
    let close_w = art.hit_rect(assets, rel, art::picker::CLOSE, 0).map_or(26.0, |r| r.width() as f32);
    let close_x = bg.0 - close_w - 4.0;
    if art.button(ui, assets, cv, rel, art::picker::CLOSE, (close_x, 6.0)) {
        return PickerResult::Close;
    }
    // Previews: previous, current, next.
    let n = items.len();
    for (slot, &(x1, y1, x2, y2)) in PREVIEWS.iter().enumerate() {
        let idx = match slot {
            0 if p.index > 0 => Some(p.index - 1),
            1 => Some(p.index),
            2 if p.index + 1 < n => Some(p.index + 1),
            _ => None,
        };
        let area = cv.rect_xywh(x1, y1, x2 - x1, y2 - y1);
        let Some(idx) = idx else { continue };
        let e = &items[idx];
        if !e.missing.is_empty() {
            // The question marks over locked items; the structures and
            // lifts pickers both have them.
            use art::structures_toolbar::{QUESTION_MARK, QUESTION_MARK_LEFT, QUESTION_MARK_RIGHT};
            let group = [QUESTION_MARK_LEFT, QUESTION_MARK, QUESTION_MARK_RIGHT][slot];
            art.draw(ui, assets, cv, rel, group, 0, 0.0, 0.0);
        } else if let Some(k) = e.icon {
            // Crops sized to the boxes: whole in the middle, half each side.
            let group = if slot == 1 { art::fluff_menu::PATH } else { art::fluff_menu::HALFPATH };
            art.draw(ui, assets, cv, rel, group, k, x1, y1);
        } else if let Some(zoom) = e.zoom {
            let mut yaw = g.camera.yaw - placed_turns(g.camera.yaw, p.view) as f32 * std::f32::consts::FRAC_PI_2;
            // Jumps show the next of their sprite's views, ramp rising
            // away to the left.
            if p.kind == PickKind::Jumps {
                yaw += std::f32::consts::FRAC_PI_2;
            }
            super::windows::world_preview(ui, cv, area, previews, &e.mesh, zoom, yaw);
        }
        let resp = ui.interact(area, ui.scope_id().with(("preview", slot)), Sense::click());
        if resp.clicked() && slot != 1 {
            p.index = idx;
            p.text_scroll = 0;
            crate::audio::sfx(assets, crate::audio::Sound::IngameDownclick);
        } else if resp.clicked() && fluff {
            // The objects menu has no build button: the center picks,
            // turned as the preview shows it.
            g.tool = e.tool.clone();
            if let Tool::Deco { rot, .. } = &mut g.tool {
                *rot = placed_turns(g.camera.yaw, p.view) as f32 * std::f32::consts::FRAC_PI_2;
            }
            crate::audio::sfx(assets, crate::audio::Sound::IngameDownclick);
        }
    }
    // Scroller: arrows either side of the bar, with "n of m" inside.
    let (sx, sy) = if fluff { FLUFF_SCROLLER } else { SCROLLER };
    if art.button(ui, assets, cv, rel, art::picker::SCROLLLEFT, (sx, sy)) && p.index > 0 {
        p.index -= 1;
        p.text_scroll = 0;
    }
    let lw = art.hit_rect(assets, rel, art::picker::SCROLLLEFT, 0).map_or(37.0, |r| r.width() as f32);
    art.draw(ui, assets, cv, rel, art::picker::HORIZ_SCROLLBAR, 0, sx + lw, sy);
    let bw = art.hit_rect(assets, rel, art::picker::HORIZ_SCROLLBAR, 0).map_or(87.0, |r| r.width() as f32);
    if art.button(ui, assets, cv, rel, art::picker::SCROLLRIGHT, (sx + lw + bw, sy)) && p.index + 1 < n {
        p.index += 1;
        p.text_scroll = 0;
    }
    // The scroller shows the price, lifts as "%s min." (0x458390), without
    // thousands separators.
    let e = &items[p.index];
    let (lx1, ly1, lx2, ly2) = COUNT_LABEL;
    if fluff {
        // The objects and paths scroller has a knob along the bar instead
        // of a price.
        let nub_w = art.hit_rect(assets, rel, art::fluff_menu::HORIZ_SCROLLNUB, 0).map_or(18.0, |r| r.width() as f32);
        let t = if n > 1 { p.index as f32 / (n - 1) as f32 } else { 0.0 };
        art.draw(ui, assets, cv, rel, art::fluff_menu::HORIZ_SCROLLNUB, 0, sx + lw + (bw - nub_w) * t, sy);
        if art.button(ui, assets, cv, rel, art::fluff_menu::ROTATE, FLUFF_ROTATE) {
            p.view = (p.view + 1) % 4;
        }
        return PickerResult::Open;
    }
    let price = format!("${}", e.cost.round() as i64);
    let label = if p.kind == PickKind::Lifts { format!("{price} min.") } else { price };
    // In the title's bold blue, centered in its box.
    ui.painter().text(
        cv.pos((lx1 + lx2) * 0.5, (ly1 + ly2) * 0.5),
        egui::Align2::CENTER_CENTER,
        label,
        font(tstyle, s),
        tstyle.color,
    );
    // Description box.
    let mut text = format!("{}:\n", e.name);
    if e.missing.is_empty() {
        text.push_str(&e.desc);
    } else {
        text.push_str(&strings.text(Text::ScmStillNeeds));
        for m in &e.missing {
            text.push('\n');
            text.push_str(g.defs.strings.get(m).unwrap_or(m));
        }
    }
    let (tx1, ty1, tx2, ty2) = TEXT;
    // Lines 13 apart, the lift toolbar's 17 (measured at 640×480).
    let line_h = if rel == LIFTS { 17.0 } else { 13.0 } * s;
    let mut job = egui::text::LayoutJob::single_section(
        text,
        egui::TextFormat {
            font_id: font(BODY, s),
            color: BODY.color,
            line_height: Some(line_h),
            ..Default::default()
        },
    );
    job.wrap.max_width = (tx2 - tx1) * s;
    let galley = ui.painter().layout_job(job);
    let clip = cv.rect_xywh(tx1, ty1, tx2 - tx1, ty2 - ty1);
    let lines = (galley.size().y / line_h).ceil() as usize;
    let visible = ((ty2 - ty1) * s / line_h).floor().max(1.0) as usize;
    p.text_scroll = p.text_scroll.min(lines.saturating_sub(visible));
    ui.painter()
        .with_clip_rect(clip)
        .galley(Pos2::new(clip.min.x, clip.min.y - p.text_scroll as f32 * line_h), galley, BODY.color);
    // The scroll bar shows even when the text fits, its knob at the top.
    {
        use art::structures_toolbar::{SCROLLDOWN, SCROLLUP, VERT_SCROLLBAR, VERT_SCROLLNUB};
        let (vx, vy) = TEXT_SCROLL;
        if art.button(ui, assets, cv, rel, SCROLLUP, (vx, vy)) {
            p.text_scroll = p.text_scroll.saturating_sub(1);
        }
        let uh = art.hit_rect(assets, rel, SCROLLUP, 0).map_or(17.0, |r| r.height() as f32);
        art.draw(ui, assets, cv, rel, VERT_SCROLLBAR, 0, vx, vy + uh);
        let vh = art.hit_rect(assets, rel, VERT_SCROLLBAR, 0).map_or(59.0, |r| r.height() as f32);
        let nh = art.hit_rect(assets, rel, VERT_SCROLLNUB, 0).map_or(18.0, |r| r.height() as f32);
        let range = lines.saturating_sub(visible);
        let t = if range > 0 { p.text_scroll as f32 / range as f32 } else { 0.0 };
        art.draw(ui, assets, cv, rel, VERT_SCROLLNUB, 0, vx, vy + uh + (vh - nh) * t);
        if art.button(ui, assets, cv, rel, SCROLLDOWN, (vx, vy + uh + vh)) && p.text_scroll < range {
            p.text_scroll += 1;
        }
    }
    // Build and rotate.
    let build_group = if rel == STRUCTURES { art::structures_toolbar::BUILD } else { art::lifts_toolbar::LIFT };
    if art.button(ui, assets, cv, rel, build_group, BUILD) {
        if e.missing.is_empty() {
            g.tool = e.tool.clone();
            // Placed facing the screen as the preview shows it.
            if let Tool::Structure { rot, .. } = &mut g.tool {
                *rot = placed_turns(g.camera.yaw, p.view) as u8;
            }
            // The original closes the picker once something is picked.
            return PickerResult::Close;
        }
    }
    if rel == STRUCTURES && art.button(ui, assets, cv, rel, art::structures_toolbar::ROTATE, ROTATE) {
        p.view = (p.view + 1) % 4;
    }
    PickerResult::Open
}
