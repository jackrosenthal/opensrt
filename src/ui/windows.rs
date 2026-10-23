//! Windows for one selected object (`One_Structure_Window.bin`,
//! `One_Lift_Window.bin`, `One_Runmarker_Window.bin`,
//! `One_Guest_Window.bin`) and the rename box (`element_rename.bin`).
//! Positions come from the original setup code: a 270-wide dialog with the
//! title band on top, tabs at (12, 56), (56, 56) and (100, 56), and the
//! panel from y = 88.

use egui::{Color32, FontFamily, FontId, Pos2, Rect as ERect, Sense, Vec2};

use super::art::{self, ArtFile, Group};
use super::classic::{Art, BODY, Canvas, TextStyle};
use super::fonts::{ARIAL, ARIAL_BOLD};
use super::picker::Previews;
use super::style::money;
use crate::assets::Assets;
use crate::formats::eng::Strings;
use crate::game::defs::NeedKind;
use crate::game::world::StaffKind;
use crate::game::{Game, Selection};
use crate::text::Text;

const STRUCTURE: ArtFile = art::structure_window::FILE;
const LIFT: ArtFile = art::lift_window::FILE;
const RUNMARKER: ArtFile = art::runmarker_window::FILE;
const GUEST: ArtFile = art::guest_window::FILE;
const RENAME: ArtFile = art::rename::FILE;
const STAFF: ArtFile = art::staff_window::FILE;
/// The employee window's buttons (0x63246c): rename, follow, move (the
/// Hand of God, drawn from the guest window's art) and the camera eye,
/// down the right; fire at the bottom.
const STAFF_BASE: (f32, f32) = (218.0, 107.0);
const STAFF_FIRE: (f32, f32) = (218.0, 247.0);

const PANEL_Y: f32 = 88.0;
const TABS: [(f32, f32); 3] = [(12.0, 56.0), (56.0, 56.0), (100.0, 56.0)];
const NAME: (f32, f32, f32, f32) = (0.0, 10.0, 229.0, 39.0);
const CLOSE: (f32, f32) = (245.0, 0.0);
const STATUS: (f32, f32, f32, f32) = (30.0, 96.0, 183.0, 114.0);
const PREVIEW: (f32, f32, f32, f32) = (33.0, 131.0, 179.0, 287.0);
const STATS: (f32, f32, f32, f32) = (10.0, 118.0, 210.0, 268.0);
const LIST: (f32, f32, f32, f32) = (21.0, 95.0, 229.0, 280.0);
/// The buttons of the info windows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Button {
    Close,
    Open,
    Rename,
    Focus,
    Sell,
    Fire,
    Patseye,
    HandOfGod,
}

impl Button {
    /// The button's group in the window art.
    fn group(self) -> Group {
        match self {
            Button::Close => art::facility_window::CLOSE_BUILDING,
            Button::Open => art::facility_window::OPEN_BUILDING,
            Button::Rename => art::object_window::RENAME,
            Button::Focus => art::object_window::FOCUS,
            Button::Sell => art::facility_window::SELL,
            Button::Fire => art::staff_window::SELL,
            Button::Patseye => art::object_window::PATSEYE,
            Button::HandOfGod => art::guest_window::HAND_OF_GOD,
        }
    }
}

/// Button column for objects without an item price, and its offsets.
const BUTTONS_BASE: (f32, f32) = (195.0, 97.0);
const BUTTONS: [(Button, f32, f32); 6] = [
    (Button::Close, 0.0, 0.0),
    (Button::Open, 0.0, 25.0),
    (Button::Rename, 9.0, 65.0),
    (Button::Focus, 9.0, 100.0),
    (Button::Sell, 9.0, 135.0),
    (Button::Patseye, 9.0, 170.0),
];
/// Button column for buildings that sell items (with a price spinner).
const PRICED_BASE: (f32, f32) = (189.0, 96.0);
const PRICED: [(Button, f32, f32); 6] = [
    (Button::Close, 7.0, 0.0),
    (Button::Open, 7.0, 25.0),
    (Button::Rename, 35.0, 61.0),
    (Button::Focus, 0.0, 97.0),
    (Button::Sell, 35.0, 97.0),
    (Button::Patseye, 0.0, 61.0),
];
const PRICE_TEXT: (f32, f32, f32, f32) = (195.0, 274.0, 224.0, 292.0);
const PRICE_SPIN: (f32, f32) = (234.0, 267.0);
/// The upgrade button, under the picture.
const UPGRADE: (f32, f32) = (66.0, 289.0);
const STAFF_TAB: (f32, f32) = (144.0, 56.0);
const MUSIC_TAB: (f32, f32) = (188.0, 56.0);
const PLAY: (f32, f32) = (86.0, 277.0);
const STOP: (f32, f32) = (123.0, 277.0);
const HIRE: (f32, f32) = (224.0, 273.0);
const GUEST_BASE: (f32, f32) = (213.0, 100.0);
const GUEST_BUTTONS: [(Button, f32, f32); 4] = [
    (Button::Patseye, 0.0, 0.0),
    (Button::Rename, 0.0, 35.0),
    (Button::Focus, 0.0, 70.0),
    (Button::HandOfGod, 0.0, 105.0),
];

fn font(style: TextStyle, s: f32) -> FontId {
    FontId::new(style.size * s, FontFamily::Name(if style.bold { ARIAL_BOLD } else { ARIAL }.into()))
}

const TITLE: TextStyle = TextStyle {
    size: 14.0,
    bold: true,
    color: Color32::from_rgb(0x00, 0x00, 0x99),
};

pub struct ObjWindow {
    pub sel: Selection,
    pub tab: usize,
    pub pos: Vec2,
    pub scroll: usize,
    /// The row picked in a radio tower's song list.
    pub song: usize,
}

impl ObjWindow {
    pub fn new(sel: Selection) -> ObjWindow {
        ObjWindow {
            sel,
            tab: 0,
            pos: Vec2::new(740.0, 90.0),
            scroll: 0,
            song: 0,
        }
    }
}

/// The rename box opened from a window's rename button.
pub struct Rename {
    pub sel: Selection,
    pub text: String,
}

pub enum WindowAction {
    None,
    Close,
    Rename(Selection, String),
    /// The radio tower's Play (a song, or the Random row past the songs)
    /// or Stop.
    Music(Option<usize>),
}

fn name_of(g: &Game, sel: &Selection) -> Option<String> {
    let w = &g.world;
    match sel {
        Selection::Park(id) => w.parks.iter().find(|p| p.id == *id).map(|p| p.name.clone()),
        Selection::Structure(id) => w.structure(*id).map(|s| s.name.clone()),
        Selection::Lift(id) => w.lift(*id).map(|l| l.name.clone()),
        Selection::Run(id) => w.runs.iter().find(|r| r.id == *id).map(|r| r.name.clone()),
        Selection::Guest(id) => w.guest(*id).map(|x| x.name.clone()),
        Selection::Staff(id) => w.staff.iter().find(|s| s.id == *id).map(|s| s.name.clone()),
    }
}

/// Stats on a stats panel, one to a row of its boxes: the label in the
/// gray box (x 17–176), the value in the yellow one (x 180–252). The first
/// row's boxes are 8 below the panel top, `pitch` apart.
fn boxed_rows(ui: &egui::Ui, cv: Canvas, rows: &[(String, String)], pitch: f32) {
    for (i, (k, v)) in rows.iter().enumerate() {
        let mid = PANEL_Y + 8.0 + i as f32 * pitch + 9.0;
        ui.painter().text(cv.pos(20.0, mid), egui::Align2::LEFT_CENTER, k, font(BODY, cv.scale), BODY.color);
        ui.painter().text(cv.pos(249.0, mid), egui::Align2::RIGHT_CENTER, v, font(BODY, cv.scale), BODY.color);
    }
}

/// The stats panel's last row, its boxes shaped for it: the condition,
/// as a bar across the wide yellow box (x 119–252), `y` below the panel
/// top.
fn condition_row(ui: &egui::Ui, cv: Canvas, label: &str, y: f32, v: f32) {
    let mid = PANEL_Y + y + 9.0;
    ui.painter().text(cv.pos(20.0, mid), egui::Align2::LEFT_CENTER, label, font(BODY, cv.scale), BODY.color);
    bar(ui, cv, 123.0, mid - 4.0, 125.0, v);
}

/// A list of lines in the list area, scrolled by `scroll` lines.
fn lines(ui: &egui::Ui, cv: Canvas, items: &[String], scroll: usize) {
    let (x1, y1, x2, y2) = LIST;
    let h = BODY.size + 4.0;
    let n = ((y2 - y1) / h) as usize;
    let clip = cv.rect_xywh(x1, y1, x2 - x1, y2 - y1);
    for (i, t) in items.iter().skip(scroll).take(n).enumerate() {
        ui.painter()
            .with_clip_rect(clip)
            .text(cv.pos(x1, y1 + i as f32 * h), egui::Align2::LEFT_TOP, t, font(BODY, cv.scale), BODY.color);
    }
}

fn bar(ui: &egui::Ui, cv: Canvas, x: f32, y: f32, w: f32, v: f32) {
    let r = cv.rect_xywh(x, y, w, 8.0);
    ui.painter().rect_filled(r, 2.0, Color32::from_rgb(0xd8, 0xdc, 0xe8));
    let col = if v > 0.6 {
        Color32::from_rgb(0x20, 0xa0, 0x30)
    } else if v > 0.3 {
        Color32::from_rgb(0xe0, 0xb0, 0x20)
    } else {
        Color32::from_rgb(0xd0, 0x30, 0x20)
    };
    let mut f = r;
    f.set_width(r.width() * v.clamp(0.0, 1.0));
    ui.painter().rect_filled(f, 2.0, col);
}

/// A building's preview: the original draws its sprite at the zoom level
/// `preview_zoom` names, one texture pixel to a window pixel, so most
/// show whole and the entrance (level 0) as a close-up of its gate.
pub(super) fn world_preview(ui: &egui::Ui, cv: Canvas, area: ERect, previews: &mut Previews, mesh: &str, zoom: u8, yaw: f32) {
    if mesh.is_empty() {
        return;
    }
    if let Some(tex) = previews.get(mesh, super::picker::world_view(zoom, yaw)) {
        let r = ERect::from_center_size(area.center(), tex.size_vec2() * cv.scale);
        ui.painter()
            .with_clip_rect(area)
            .image(tex.id(), r, ERect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    w: &mut ObjWindow,
    ui: &mut egui::Ui,
    art: &mut Art,
    assets: &Assets,
    strings: &Strings,
    previews: &mut Previews,
    g: &mut Game,
    blocks: &mut Vec<ERect>,
    songs: &[String],
) -> WindowAction {
    let Some(name) = name_of(g, &w.sel) else { return WindowAction::Close };
    let screen = ui.ctx().content_rect();
    let s = (screen.height() / 768.0).min(screen.width() / 1024.0);
    w.pos.x = w.pos.x.min(screen.width() / s - 270.0).max(0.0);
    let cv = Canvas {
        origin: screen.min + w.pos * s,
        scale: s,
        ppp: ui.ctx().pixels_per_point(),
    };
    let (rel, tabs): (ArtFile, &[Group]) = match &w.sel {
        Selection::Structure(_) => {
            use art::structure_window::*;
            (STRUCTURE, &[MAIN_TAB, STATS_TAB, GUEST_TAB])
        }
        Selection::Lift(_) => {
            use art::lift_window::*;
            (LIFT, &[MAIN_TAB, STATS_TAB, GUEST_TAB])
        }
        Selection::Run(_) => (RUNMARKER, &[art::runmarker_window::MARKER_TAB]),
        Selection::Park(_) => (RUNMARKER, &[art::runmarker_window::JUMP_TAB]),
        Selection::Guest(_) => {
            use art::guest_window::*;
            (GUEST, &[ONE_GUEST_MAIN, GUEST_STATS_TAB, GUEST_THOUGHTS_TAB])
        }
        Selection::Staff(id) => {
            // One tab, showing what kind of employee this is.
            let kind = g.world.staff.iter().find(|s| s.id == *id).map(|s| s.kind);
            let tab: &'static [Group] = match kind {
                Some(StaffKind::Patrol) => &[art::staff_window::TAB_SKIPATROL],
                Some(StaffKind::Sanitation) => &[art::staff_window::TAB_SANITATION],
                Some(StaffKind::Instructor) => &[art::staff_window::TAB_INSTRUCTOR],
                Some(StaffKind::AvalancheControl) => &[art::staff_window::TAB_AVALANCHE],
                Some(StaffKind::Security) => &[art::staff_window::TAB_SECURITY],
                _ => &[art::staff_window::TAB_MAINTENANCE],
            };
            (STAFF, tab)
        }
    };
    // Priced buildings use the other background and button column.
    // Price controls when the price can go above 1 (0x46db98).
    let priced = match &w.sel {
        Selection::Structure(id) => g.world.structure(*id).is_some_and(|st| g.defs.structures[st.def].item_cost_max > 1.0),
        _ => false,
    };
    let bg = match &w.sel {
        Selection::Structure(_) if !priced => art::structure_window::BACKGROUND_NOCOST,
        _ => art::object_window::BACKGROUND,
    };
    // Frame: title band, panel, name, tabs, close.
    art.draw(ui, assets, cv, rel, art::object_window::TITLE_BAR, 0, 0.0, 0.0);
    // Each tab has its own panel, the background group's element of the
    // same index.
    let panel = if w.tab < art.element_count(assets, rel, bg) { w.tab } else { 0 };
    art.draw(ui, assets, cv, rel, bg, panel, 0.0, PANEL_Y);
    let whole = cv.rect_xywh(0.0, 0.0, 270.0, PANEL_Y + 234.0);
    blocks.push(whole);
    ui.interact(whole, ui.scope_id().with("objwin body"), Sense::click());
    let drag = ui.interact(cv.rect_xywh(0.0, 0.0, 225.0, 50.0), ui.scope_id().with("objwin drag"), Sense::drag());
    if drag.dragged() {
        w.pos += drag.drag_delta() / s;
    }
    let (nx1, ny1, nx2, _) = NAME;
    ui.painter().text(cv.pos((nx1 + nx2) * 0.5, ny1), egui::Align2::CENTER_TOP, &name, font(TITLE, s), TITLE.color);
    let mut order: Vec<usize> = (0..tabs.len()).filter(|&i| i != w.tab).collect();
    order.push(w.tab.min(tabs.len() - 1));
    for i in order {
        let (x, y) = TABS[i];
        let state = tab_state(art, assets, rel, tabs[i], i == w.tab);
        art.draw(ui, assets, cv, rel, tabs[i], state, x, y);
        let r = cv.rect_xywh(x, y, 45.0, 31.0);
        if ui.interact(r, ui.scope_id().with(("objwin tab", i)), Sense::click()).clicked() && w.tab != i {
            w.tab = i;
            w.scroll = 0;
        }
    }
    let staff = match &w.sel {
        Selection::Structure(id) => g.world.structure(*id).and_then(|st| g.defs.structures[st.def].kind.staff()),
        _ => None,
    };
    if staff.is_some() {
        let (x, y) = STAFF_TAB;
        if w.tab != 3 {
            let state = tab_state(art, assets, rel, art::structure_window::STAFF_TAB, false);
            art.draw(ui, assets, cv, rel, art::structure_window::STAFF_TAB, state, x, y);
        }
        if ui.interact(cv.rect_xywh(x, y, 45.0, 31.0), ui.scope_id().with("objwin staff tab"), Sense::click()).clicked() {
            w.tab = 3;
            w.scroll = 0;
        }
        if w.tab == 3 {
            let state = tab_state(art, assets, rel, art::structure_window::STAFF_TAB, true);
            art.draw(ui, assets, cv, rel, art::structure_window::STAFF_TAB, state, x, y);
        }
    } else if w.tab == 3 {
        w.tab = 0;
    }
    // A radio tower also has the music tab (0x46e975).
    let radio = match &w.sel {
        Selection::Structure(id) => g.world.structure(*id).is_some_and(|st| g.defs.structures[st.def].kind == crate::game::defs::StructureKind::RadioStation),
        _ => false,
    };
    if radio {
        let (x, y) = MUSIC_TAB;
        if w.tab != 4 {
            let state = tab_state(art, assets, rel, art::structure_window::MUSIC_TAB, false);
            art.draw(ui, assets, cv, rel, art::structure_window::MUSIC_TAB, state, x, y);
        }
        if ui.interact(cv.rect_xywh(x, y, 45.0, 31.0), ui.scope_id().with("objwin music tab"), Sense::click()).clicked() {
            w.tab = 4;
            w.scroll = 0;
        }
        if w.tab == 4 {
            let state = tab_state(art, assets, rel, art::structure_window::MUSIC_TAB, true);
            art.draw(ui, assets, cv, rel, art::structure_window::MUSIC_TAB, state, x, y);
        }
    } else if w.tab == 4 {
        w.tab = 0;
    }
    if art.button(ui, assets, cv, rel, art::object_window::CLOSE, CLOSE) {
        return WindowAction::Close;
    }
    let mut action = WindowAction::None;
    // Mouse wheel scrolls lists.
    if ui.rect_contains_pointer(whole) {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy < -1.0 {
            w.scroll += 1;
        } else if dy > 1.0 {
            w.scroll = w.scroll.saturating_sub(1);
        }
    }
    let tr = |k: Text| strings.text(k).into_owned();
    let defs = g.defs.clone();
    match w.sel.clone() {
        Selection::Structure(id) => {
            let Some(st) = g.world.structure(id).cloned() else { return WindowAction::Close };
            let d = &defs.structures[st.def];
            match w.tab {
                0 => {
                    status(ui, cv, &st_status(strings, st.broken, st.powered, st.open));
                    let yaw = g.camera.yaw - st.rot as f32 * std::f32::consts::FRAC_PI_2;
                    let (x1, y1, x2, y2) = PREVIEW;
                    world_preview(ui, cv, cv.rect_xywh(x1, y1, x2 - x1, y2 - y1), previews, &d.meshes.day, d.preview_zoom, yaw);
                    let (base, layout) = if priced { (PRICED_BASE, &PRICED) } else { (BUTTONS_BASE, &BUTTONS) };
                    if let Some(a) = buttons(ui, art, assets, cv, rel, base, layout) {
                        action = object_button(a, &w.sel, &name, g);
                    }
                    // Condominiums and hotels upgrade in place (0x46e54a):
                    // the button carries the upgrade's price.
                    if let Some(up) = d.upgrade {
                        let (ux, uy) = UPGRADE;
                        if art.button(ui, assets, cv, rel, art::structure_window::UPGRADE, (ux, uy)) {
                            g.upgrade_structure(id);
                        }
                        // Its text, with the price, shows on hover.
                        if ui.rect_contains_pointer(cv.rect_xywh(ux, uy, 80.0, 23.0)) {
                            let label = format!("{} ${:.2}", tr(Text::SingleBuildingUpgrade), defs.structures[up].cost);
                            let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, ui.scope_id().with("upgrade tip")));
                            let galley = painter.layout_no_wrap(label, font(BODY, s), BODY.color);
                            let r = egui::Rect::from_min_size(cv.pos(ux, uy - 18.0), galley.size()).expand(2.0);
                            painter.rect_filled(r, 2.0, Color32::from_rgb(0xff, 0xff, 0xe0));
                            painter.galley(r.min + egui::vec2(2.0, 2.0), galley, BODY.color);
                        }
                    }
                    if priced {
                        let (px1, py1, px2, _) = PRICE_TEXT;
                        ui.painter().text(cv.pos((px1 + px2) * 0.5, py1), egui::Align2::CENTER_TOP, money(st.price), font(BODY, s), BODY.color);
                        let (sx, sy) = PRICE_SPIN;
                        let step = d.price_step();
                        let set = |g: &mut Game, by: f32| {
                            if let Some(m) = g.world.structure_mut(id) {
                                m.price = (m.price.trunc() + by).clamp(d.item_cost_min, d.item_cost_max);
                            }
                        };
                        if art.button(ui, assets, cv, rel, art::structure_window::SPINUP, (sx, sy)) {
                            set(g, step);
                        }
                        if art.button(ui, assets, cv, rel, art::structure_window::SPINDOWN, (sx, sy + 18.0)) {
                            set(g, -step);
                        }
                    }
                }
                1 => {
                    let total = g.world.guests.iter().filter(|x| x.uses.iter().any(|u| u.0 == id)).count();
                    boxed_rows(
                        ui,
                        cv,
                        &[
                            (tr(Text::StructureMenuCurrentValue), money(st.worth.now)),
                            (tr(Text::StructureMenuOriginalValue), money(d.cost)),
                            (tr(Text::StructureMenuTotalIncome), money(st.revenue_total)),
                            (tr(Text::StructureMenuMaintenance), money(d.maintenance_per_wear * st.wear)),
                            (tr(Text::StructureMenuRunningCosts), money(d.operations_per_day)),
                            (tr(Text::StructureMenuGuests), st.occupants.len().to_string()),
                            (tr(Text::StructureMenuCapacity), d.guest_occupancy.to_string()),
                            (tr(Text::StructureMenuTotalGuests), (st.visits_today as usize).max(total).to_string()),
                        ],
                        23.0,
                    );
                    condition_row(ui, cv, &tr(Text::StructureMenuCondition), 196.0, 1.0 - st.wear / 100.0);
                }
                2 => {
                    let names: Vec<String> = st.occupants.iter().filter_map(|gid| g.world.guest(*gid).map(|x| x.name.clone())).collect();
                    lines(ui, cv, &names, w.scroll);
                }
                4 => {
                    // The songs in data/music, then "Random".
                    let mut rows: Vec<String> = songs.to_vec();
                    rows.push(tr(Text::StructureMenuMusicRandom));
                    let (x1, y1, x2, y2) = LIST;
                    let h = 15.0;
                    let n = ((y2 - 40.0 - y1) / h) as usize;
                    w.song = w.song.min(rows.len() - 1);
                    for (i, t) in rows.iter().enumerate().skip(w.scroll).take(n) {
                        let y = y1 + (i - w.scroll) as f32 * h;
                        if i == w.song {
                            art.draw(ui, assets, cv, rel, art::structure_window::HIGHLIGHT, 0, x1, y);
                        }
                        ui.painter().text(cv.pos(x1 + 4.0, y + 1.0), egui::Align2::LEFT_TOP, t, font(BODY, s), BODY.color);
                        if ui.interact(cv.rect_xywh(x1, y, x2 - x1, h), ui.scope_id().with(("song", i)), Sense::click()).clicked() {
                            w.song = i;
                        }
                    }
                    if art.button(ui, assets, cv, rel, art::structure_window::PLAY, PLAY) {
                        action = WindowAction::Music(Some(w.song));
                    }
                    if art.button(ui, assets, cv, rel, art::structure_window::STOP, STOP) {
                        action = WindowAction::Music(None);
                    }
                }
                _ => {
                    if let Some(kind) = staff {
                        let names: Vec<String> = g
                            .world
                            .staff
                            .iter()
                            .filter(|x| x.kind == kind)
                            .map(|x| format!("{}: {}", x.name, super::staff_activity(strings, x, &g.world)))
                            .collect();
                        lines(ui, cv, &names, w.scroll);
                        if art.button(ui, assets, cv, rel, art::structure_window::HIRE, HIRE) && g.can_hire(id) {
                            g.hire(kind, Some(st.pos), Some(id));
                        }
                    }
                }
            }
        }
        Selection::Lift(id) => {
            let Some(l) = g.world.lift(id).cloned() else { return WindowAction::Close };
            let d = &defs.lifts[l.def];
            match w.tab {
                0 => {
                    status(ui, cv, &st_status(strings, l.broken, true, l.open));
                    // As its picker shows it.
                    let (x1, y1, x2, y2) = PREVIEW;
                    world_preview(ui, cv, cv.rect_xywh(x1, y1, x2 - x1, y2 - y1), previews, &d.begin.day, 1, std::f32::consts::FRAC_PI_4);
                    let cond = 1.0 - l.wear / 100.0;
                    ui.painter().text(cv.pos(36.0, 268.0), egui::Align2::LEFT_TOP, tr(Text::StructureCondition), font(BODY, s), BODY.color);
                    bar(ui, cv, 100.0, 271.0, 76.0, cond);
                    if let Some(a) = buttons(ui, art, assets, cv, rel, BUTTONS_BASE, &BUTTONS) {
                        action = object_button(a, &w.sel, &name, g);
                    }
                }
                1 => {
                    let cap = d.capacity as f32 * 3600.0 / d.secs_between_chairs.max(1.0);
                    boxed_rows(
                        ui,
                        cv,
                        &[
                            (tr(Text::StructureInfoResellValue), money(l.worth.now)),
                            (tr(Text::StructureInfoOrigValue), money(d.base_cost + d.cost_per_pole * l.poles.len() as f32)),
                            (tr(Text::StructureInfoMaintenance), money(d.maintenance_per_wear * l.wear)),
                            (tr(Text::StructureInfoRunningCosts), money(d.operations_per_day)),
                            (tr(Text::LsdChairs), l.chairs.len().to_string()),
                            (tr(Text::LsdCapacityOneWay), format!("{cap:.0}")),
                            (tr(Text::LsdLineLength), l.queue.len().to_string()),
                            (tr(Text::LsdGuestsOnLift), l.chairs.iter().filter(|c| !c.riders.is_empty()).count().to_string()),
                            (tr(Text::LsdLifetimeGuests), l.riders_total.to_string()),
                        ],
                        21.0,
                    );
                    condition_row(ui, cv, &tr(Text::StructureCondition), 197.0, 1.0 - l.wear / 100.0);
                }
                _ => {
                    let names: Vec<String> = l.queue.iter().filter_map(|gid| g.world.guest(*gid).map(|x| x.name.clone())).collect();
                    lines(ui, cv, &names, w.scroll);
                }
            }
        }
        // Run markers and terrain parks: open or closed, the picture, and
        // the buttons (`One_Runmarker_Window`).
        Selection::Run(id) => {
            let Some(r) = g.world.runs.iter().find(|r| r.id == id).cloned() else { return WindowAction::Close };
            let d = &defs.runs[r.def];
            status(ui, cv, &tr(if r.open { Text::ObjectOpen } else { Text::ObjectClosed }));
            let (x1, y1, x2, y2) = PREVIEW;
            // The sign up close, at the nearest zoom level.
            world_preview(ui, cv, cv.rect_xywh(x1, y1, x2 - x1, y2 - y1), previews, &d.mesh, 0, std::f32::consts::FRAC_PI_4);
            if let Some(a) = buttons(ui, art, assets, cv, rel, BUTTONS_BASE, &BUTTONS) {
                action = object_button(a, &w.sel, &name, g);
            }
        }
        Selection::Park(id) => {
            let Some(p) = g.world.parks.iter().find(|p| p.id == id).cloned() else { return WindowAction::Close };
            let d = &defs.parks[p.def];
            status(ui, cv, &tr(if p.open { Text::ObjectOpen } else { Text::ObjectClosed }));
            let (x1, y1, x2, y2) = PREVIEW;
            let mesh = if d.piece(0, 3).is_empty() { d.piece(0, 0) } else { d.piece(0, 3) };
            world_preview(ui, cv, cv.rect_xywh(x1, y1, x2 - x1, y2 - y1), previews, mesh, 1, 3.0 * std::f32::consts::FRAC_PI_4);
            if let Some(a) = buttons(ui, art, assets, cv, rel, BUTTONS_BASE, &BUTTONS) {
                action = object_button(a, &w.sel, &name, g);
            }
        }
        Selection::Guest(id) => {
            let Some(x) = g.world.guest(id).cloned() else { return WindowAction::Close };
            match w.tab {
                0 => {
                    let thought = x
                        .thought
                        .as_ref()
                        .map(|t| super::thought_text(strings, t.0, x.id))
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| tr(Text::SingleGuestTickerGreeting));
                    status(ui, cv, &thought);
                    // Skill icon and label, cash.
                    let skill = x.skill as usize;
                    ui.painter().text(cv.pos(36.0, 128.0), egui::Align2::LEFT_TOP, tr(Text::SingleGuestSkill), font(BODY, s), BODY.color);
                    art.draw(ui, assets, cv, rel, art::guest_window::SKILL, skill.min(3), 150.0, 126.0);
                    ui.painter().text(
                        cv.pos(36.0, 146.0),
                        egui::Align2::LEFT_TOP,
                        format!("{} {}", tr(Text::SingleGuestCash), money(x.cash)),
                        font(BODY, s),
                        BODY.color,
                    );
                    ui.painter().text(
                        cv.pos(36.0, 164.0),
                        egui::Align2::LEFT_TOP,
                        super::state_label(strings, &x, &g.world),
                        font(BODY, s),
                        BODY.color,
                    );
                    if let Some(a) = buttons(ui, art, assets, cv, rel, GUEST_BASE, &GUEST_BUTTONS) {
                        action = object_button(a, &w.sel, &name, g);
                    }
                }
                1 => {
                    let keys = [
                        (Text::SingleGuestFood, x.need(NeedKind::Food)),
                        (Text::SingleGuestEnergy, x.need(NeedKind::Rest)),
                        (Text::SingleGuestWarmth, x.need(NeedKind::Warmth)),
                        (Text::SingleGuestHealth, x.need(NeedKind::Health)),
                        (Text::SingleGuestEntertainment, x.need(NeedKind::Entertainment)),
                        (Text::SingleGuestRestroom, x.need(NeedKind::Bathroom)),
                        (Text::SingleGuestHappiness, x.happiness),
                        (Text::SingleGuestThriftiness, 1.0 - x.tolerance),
                    ];
                    let (x1, y1, _, _) = STATS;
                    for (i, (k, v)) in keys.iter().enumerate() {
                        let y = y1 + i as f32 * 18.0;
                        ui.painter().text(cv.pos(x1 + 14.0, y), egui::Align2::LEFT_TOP, tr(*k), font(BODY, s), BODY.color);
                        bar(ui, cv, 130.0, y + 2.0, 100.0, *v);
                    }
                }
                _ => {
                    let items: Vec<String> = x.thought.iter().map(|t| super::thought_text(strings, t.0, x.id)).collect();
                    lines(ui, cv, &items, w.scroll);
                }
            }
        }
        Selection::Staff(id) => {
            let Some(st) = g.world.staff.iter().find(|s| s.id == id).cloned() else { return WindowAction::Close };
            lines(ui, cv, &[super::staff_activity(strings, &st, &g.world)], 0);
            if let Some(a) = buttons(ui, art, assets, cv, rel, STAFF_BASE, &[(Button::Rename, 0.0, 0.0), (Button::Focus, 0.0, 35.0)]) {
                action = object_button(a, &w.sel, &name, g);
            }
            if let Some(a) = buttons(ui, art, assets, cv, GUEST, STAFF_BASE, &[(Button::HandOfGod, 0.0, 70.0)]) {
                action = object_button(a, &w.sel, &name, g);
            }
            if let Some(a) = buttons(ui, art, assets, cv, rel, STAFF_BASE, &[(Button::Patseye, 0.0, 105.0)]) {
                action = object_button(a, &w.sel, &name, g);
            }
            if let Some(a) = buttons(ui, art, assets, cv, rel, STAFF_FIRE, &[(Button::Fire, 0.0, 0.0)]) {
                action = object_button(a, &w.sel, &name, g);
            }
        }
    }
    action
}


/// The status strip text: the original's `OBJECT_*` strings.
fn st_status(strings: &Strings, broken: bool, powered: bool, open: bool) -> String {
    if broken {
        strings.text(Text::ObjectBroken).into()
    } else if !powered {
        strings.text(Text::ObjectNoPower).into()
    } else if open {
        strings.text(Text::ObjectOpen).into()
    } else {
        strings.text(Text::ObjectClosed).into()
    }
}

/// Text in the gray status strip at the top of the panel.
fn status(ui: &egui::Ui, cv: Canvas, text: &str) {
    let (x1, y1, x2, y2) = STATUS;
    let clip = cv.rect_xywh(x1, y1, x2 - x1, y2 - y1);
    ui.painter().with_clip_rect(clip).text(
        cv.pos((x1 + x2) * 0.5, (y1 + y2) * 0.5),
        egui::Align2::CENTER_CENTER,
        text,
        font(BODY, cv.scale),
        BODY.color,
    );
}

/// Draws a button column; returns the group of the clicked one.
fn buttons(ui: &mut egui::Ui, art: &mut Art, assets: &Assets, cv: Canvas, rel: ArtFile, base: (f32, f32), layout: &[(Button, f32, f32)]) -> Option<Button> {
    let mut clicked = None;
    for &(b, dx, dy) in layout {
        if art.button(ui, assets, cv, rel, b.group(), (base.0 + dx, base.1 + dy)) {
            clicked = Some(b);
        }
    }
    clicked
}

fn object_button(b: Button, sel: &Selection, name: &str, g: &mut Game) -> WindowAction {
    let pos = match sel {
        Selection::Structure(id) => g.world.structure(*id).map(|s| s.pos),
        Selection::Lift(id) => g.world.lift(*id).map(|l| l.start),
        Selection::Run(id) => g.world.runs.iter().find(|r| r.id == *id).and_then(|r| r.gates.first().copied()),
        Selection::Park(id) => g.world.parks.iter().find(|p| p.id == *id).map(|p| p.start),
        Selection::Guest(id) => g.world.guest(*id).map(|x| x.pos),
        Selection::Staff(id) => g.world.staff.iter().find(|s| s.id == *id).map(|s| s.pos),
    };
    match b {
        Button::Close | Button::Open => {
            let open = b == Button::Open;
            match sel {
                Selection::Structure(id) => {
                    if let Some(s) = g.world.structure_mut(*id) {
                        s.open = open;
                    }
                }
                Selection::Lift(id) => {
                    if let Some(l) = g.world.lift_mut(*id) {
                        l.open = open;
                    }
                }
                Selection::Run(id) => {
                    if let Some(r) = g.world.runs.iter_mut().find(|r| r.id == *id) {
                        r.open = open;
                    }
                }
                Selection::Park(id) => {
                    if let Some(p) = g.world.parks.iter_mut().find(|p| p.id == *id) {
                        p.open = open;
                    }
                }
                _ => {}
            }
        }
        Button::Focus => {
            if let Some(p) = pos {
                g.camera.target = p;
            }
        }
        Button::Patseye => {
            // The eye view, once a fog setting is chosen (0x432f10).
            let _ = pos;
            g.fog_request = Some(sel.clone());
        }
        Button::Sell | Button::Fire => {
            g.demolish(sel);
            return WindowAction::Close;
        }
        Button::HandOfGod => {
            if let Selection::Guest(id) | Selection::Staff(id) = sel {
                g.pick_up(*id);
                return WindowAction::Close;
            }
        }
        Button::Rename => return WindowAction::Rename(sel.clone(), name.to_string()),
    }
    WindowAction::None
}

/// The rename box. Returns Some(Some(name)) on OK, Some(None) on cancel.
pub fn rename(r: &mut Rename, ui: &mut egui::Ui, art: &mut Art, assets: &Assets, blocks: &mut Vec<ERect>) -> Option<Option<String>> {
    let screen = ui.ctx().content_rect();
    let s = (screen.height() / 768.0).min(screen.width() / 1024.0);
    let cv = Canvas {
        origin: screen.center() - Vec2::new(100.0, 61.0) * s,
        scale: s,
        ppp: ui.ctx().pixels_per_point(),
    };
    art.draw(ui, assets, cv, RENAME, art::rename::BACKGROUND, 0, 0.0, 0.0);
    let whole = cv.rect_xywh(0.0, 0.0, 200.0, 122.0);
    blocks.push(whole);
    ui.interact(whole, ui.scope_id().with("rename body"), Sense::click());
    let field = cv.rect_xywh(21.0, 48.0, 158.0, 18.0);
    let f = FontId::new(13.0 * s, FontFamily::Name(ARIAL_BOLD.into()));
    let resp = ui.put(
        field,
        egui::TextEdit::singleline(&mut r.text)
            .font(f)
            .text_color(Color32::BLACK)
            .frame(egui::Frame::NONE)
            .char_limit(20),
    );
    resp.request_focus();
    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
    if art.button(ui, assets, cv, RENAME, art::rename::OK, (148.0, 74.0)) || enter {
        return Some(Some(r.text.trim().to_string()));
    }
    if art.button(ui, assets, cv, RENAME, art::rename::CANCEL, (113.0, 75.0)) || art.button(ui, assets, cv, RENAME, art::rename::CLOSE, (159.0, 13.0)) {
        return Some(None);
    }
    None
}

/// Which element of a tab group to draw: the third (`..._Normal`) is the
/// tab in front, merging into the panel; the first (`..._Depress`) one
/// behind.
fn tab_state(art: &mut Art, assets: &Assets, rel: ArtFile, group: Group, front: bool) -> usize {
    if front && art.element_count(assets, rel, group) > 2 { 2 } else { 0 }
}
