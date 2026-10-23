//! The Finances menu windows: resort statistics (`resort_stats_toolbar`),
//! finances (`resort_expenses_toolbar`), guests (`guests_toolbar`), staff
//! (`Staff_Window`) and attractions (`structure_list_toolbar`). Tab and
//! widget positions come from their setup code; boxes inside the panels
//! from the background art.

use egui::{Color32, FontFamily, FontId, Pos2, Rect as ERect, Sense, Stroke, Vec2};

use super::art::{self, ArtFile, Group};
use super::classic::{Art, BODY, Canvas, TextStyle};
use super::fonts::{ARIAL, ARIAL_BOLD};
use super::picker::Previews;
use super::style::money;
use crate::assets::Assets;
use crate::formats::eng::Strings;
use crate::game::defs::NeedKind;
use crate::game::world::{Ledger, StaffKind};
use crate::game::{Game, Selection};
use crate::text::Text;

/// Days the history graphs show; the original keeps 30 (0x428d92).
const HISTORY_DAYS: usize = 30;

const RESORT: ArtFile = art::resort_stats::FILE;
const FINANCES: ArtFile = art::finances_stats::FILE;
const GUESTS: ArtFile = art::guests_stats::FILE;
const STAFF: ArtFile = art::staff_window::FILE;
const LIST: ArtFile = art::attractions::FILE;

const PANEL_Y: f32 = 88.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatKind {
    Resort,
    Finances,
    Guests,
    Staff,
    Attractions,
}

pub struct StatWindow {
    pub kind: StatKind,
    pub tab: usize,
    pub pos: Vec2,
    pub scroll: usize,
    /// Guest lists show actions instead of thoughts.
    pub actions: bool,
    /// The resort window's ticker: its text and scroll position.
    pub ticker: Option<(String, f32)>,
    /// Horizontal scroll of the history list, in steps of 10 pixels.
    pub hscroll: usize,
}

impl StatWindow {
    pub fn new(kind: StatKind) -> StatWindow {
        StatWindow {
            kind,
            tab: 0,
            pos: Vec2::new(if kind == StatKind::Finances { 180.0 } else { 16.0 }, 90.0),
            scroll: 0,
            actions: false,
            ticker: None,
            hscroll: 0,
        }
    }
}

const TITLE: TextStyle = TextStyle {
    size: 14.0,
    bold: true,
    color: Color32::from_rgb(0x00, 0x00, 0x99),
};

fn font(style: TextStyle, s: f32) -> FontId {
    FontId::new(style.size * s, FontFamily::Name(if style.bold { ARIAL_BOLD } else { ARIAL }.into()))
}

/// A tab's look behind the others (`..._Depress`, gray) and in front,
/// merging into the panel (`..._Normal`).
#[derive(Clone, Copy)]
enum TabArt {
    /// Two groups, behind and in front.
    Groups(Group, Group),
    /// One group: element 0 behind, element 2 in front.
    States(Group),
}

impl TabArt {
    #[allow(clippy::too_many_arguments)]
    fn draw(self, ui: &egui::Ui, art: &mut Art, assets: &Assets, cv: Canvas, rel: ArtFile, front: bool, x: f32, y: f32) {
        match (self, front) {
            (TabArt::Groups(behind, _), false) => art.draw(ui, assets, cv, rel, behind, 0, x, y),
            (TabArt::Groups(_, front), true) => art.draw(ui, assets, cv, rel, front, 0, x, y),
            (TabArt::States(group), front) => art.draw(ui, assets, cv, rel, group, if front { 2 } else { 0 }, x, y),
        };
    }
}

struct Layout {
    rel: ArtFile,
    width: f32,
    /// Each tab's art and x.
    tabs: &'static [(TabArt, f32)],
    title: Text,
    close: (f32, f32),
}

fn layout(kind: StatKind) -> Layout {
    match kind {
        StatKind::Resort => Layout {
            rel: RESORT,
            width: 270.0,
            tabs: &[
                (TabArt::Groups(art::resort_stats::TAB_MAIN, art::resort_stats::TAB_MAIN_ANI), 14.0),
                (TabArt::Groups(art::resort_stats::TAB_GUESTS, art::resort_stats::TAB_GUESTS_ANI), 58.0),
                (TabArt::Groups(art::resort_stats::TAB_RATE, art::resort_stats::TAB_RATE_ANI), 102.0),
                (TabArt::Groups(art::resort_stats::TAB_WEATHER, art::resort_stats::TAB_WEATHER_ANI), 146.0),
                (TabArt::Groups(art::resort_stats::TAB_HISTORY, art::resort_stats::TAB_HISTORY_ANI), 190.0),
            ],
            title: Text::FemResortstats,
            close: (245.0, 0.0),
        },
        StatKind::Finances => Layout {
            rel: FINANCES,
            width: 650.0,
            tabs: &[
                (TabArt::Groups(art::finances_stats::CHART_TAB, art::finances_stats::CHART_TAB_ANI), 12.0),
                (TabArt::Groups(art::finances_stats::CASH_TAB, art::finances_stats::CASH_TAB_ANI), 56.0),
                (TabArt::Groups(art::finances_stats::ASSET_TAB, art::finances_stats::ASSET_TAB_ANI), 100.0),
            ],
            title: Text::FemFinances,
            close: (610.0, 13.0),
        },
        StatKind::Guests => Layout {
            rel: GUESTS,
            width: 270.0,
            tabs: &[
                (TabArt::Groups(art::guests_stats::AVE, art::guests_stats::AVE_ANI), 12.0),
                (TabArt::Groups(art::guests_stats::GROUP, art::guests_stats::GROUP_ANI), 56.0),
                (TabArt::Groups(art::guests_stats::INDIVIDUAL, art::guests_stats::INDIVIDUAL_ANI), 100.0),
            ],
            title: Text::AllGuestsTitle,
            close: (229.0, 13.0),
        },
        StatKind::Staff => Layout {
            rel: STAFF,
            width: 270.0,
            tabs: &[
                (TabArt::States(art::staff_window::TAB_SKIPATROL), 12.0),
                (TabArt::States(art::staff_window::TAB_MAINTENANCE), 56.0),
                (TabArt::States(art::staff_window::TAB_INSTRUCTOR), 100.0),
                (TabArt::States(art::staff_window::TAB_SANITATION), 144.0),
                (TabArt::States(art::staff_window::TAB_AVALANCHE), 188.0),
            ],
            title: Text::FemStaffstats,
            close: (245.0, 0.0),
        },
        StatKind::Attractions => Layout {
            rel: LIST,
            width: 270.0,
            tabs: &[
                (TabArt::Groups(art::attractions::TAB_STRUCTURE, art::attractions::TAB_STRUCTURE_ANI), 12.0),
                (TabArt::Groups(art::attractions::TAB_LIFT, art::attractions::TAB_LIFT_ANI), 56.0),
                (TabArt::Groups(art::attractions::TAB_RUN, art::attractions::TAB_RUN_ANI), 100.0),
                (TabArt::Groups(art::attractions::TAB_JUMP, art::attractions::TAB_JUMP_ANI), 144.0),
            ],
            title: Text::FemAttraction,
            close: (229.0, 13.0),
        },
    }
}

/// Staff kinds on the staff window tabs.
const STAFF_TABS: [StaffKind; 5] = [
    StaffKind::Patrol,
    StaffKind::Maintenance,
    StaffKind::Instructor,
    StaffKind::Sanitation,
    StaffKind::AvalancheControl,
];

pub enum StatAction {
    None,
    Close,
    Select(Selection),
    Rename(Selection, String),
}

/// A strip that scrolls a message from right to left, taking the next
/// from `next` once the last has gone.
fn ticker(ui: &egui::Ui, strip: egui::Rect, state: &mut Option<(String, f32)>, s: f32, next: impl FnOnce() -> String) {
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let (msg, x) = state.get_or_insert_with(|| (String::new(), f32::MIN));
    let galley = ui.painter().layout_no_wrap(msg.clone(), font(BODY, s), BODY.color);
    if *x < -galley.size().x {
        *msg = next();
        *x = strip.width();
    } else {
        *x -= dt * 60.0 * s;
    }
    let galley = ui.painter().layout_no_wrap(msg.clone(), font(BODY, s), BODY.color);
    ui.painter().with_clip_rect(strip).galley(Pos2::new(strip.min.x + *x, strip.center().y - galley.size().y * 0.5), galley, BODY.color);
    ui.ctx().request_repaint();
}

fn text(ui: &egui::Ui, cv: Canvas, x: f32, y: f32, align: egui::Align2, t: &str, style: TextStyle) {
    ui.painter().text(cv.pos(x, y), align, t, font(style, cv.scale), style.color);
}

/// A line graph of `values` (oldest first) in a canvas box, scaled to
/// `max`.
fn graph(ui: &egui::Ui, cv: Canvas, r: (f32, f32, f32, f32), values: &[f32], max: f32, color: Color32) {
    if values.len() < 2 || max <= 0.0 {
        return;
    }
    let (x1, y1, x2, y2) = r;
    let n = values.len() - 1;
    let pts: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| cv.pos(x1 + (x2 - x1) * i as f32 / n as f32, y2 - (y2 - y1) * (v / max).clamp(0.0, 1.0)))
        .collect();
    ui.painter().add(egui::Shape::line(pts, Stroke::new(2.0 * cv.scale.max(0.5), color)));
}

/// Lines of text in a list box with a scroll offset; returns the index of
/// a clicked line.
fn list(ui: &mut egui::Ui, cv: Canvas, r: (f32, f32, f32, f32), items: &[String], scroll: usize, id: &str) -> Option<usize> {
    let (x1, y1, x2, y2) = r;
    let h = BODY.size + 4.0;
    let n = ((y2 - y1) / h) as usize;
    let clip = cv.rect_xywh(x1, y1, x2 - x1, y2 - y1);
    let mut clicked = None;
    for (i, t) in items.iter().enumerate().skip(scroll).take(n) {
        let y = y1 + (i - scroll) as f32 * h;
        let row = cv.rect_xywh(x1, y, x2 - x1, h);
        let resp = ui.interact(row, ui.scope_id().with((id, i)), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(row, 0.0, Color32::from_rgba_unmultiplied(0x99, 0xb0, 0xe0, 90));
        }
        if resp.clicked() {
            clicked = Some(i);
        }
        ui.painter()
            .with_clip_rect(clip)
            .text(cv.pos(x1 + 2.0, y), egui::Align2::LEFT_TOP, t, font(BODY, cv.scale), BODY.color);
    }
    clicked
}

/// Weekly sums of a ledger selection over the last six weeks, oldest
/// first.
fn weekly(g: &Game, ledgers: &[Ledger]) -> [f32; 6] {
    let mut out = [0.0; 6];
    let today = g.world.day;
    for d in &g.world.history {
        let age = today.saturating_sub(d.day) / 7;
        if age >= 6 {
            continue;
        }
        let v: f32 = d.income.iter().chain(d.expense.iter()).filter(|(l, _)| ledgers.contains(l)).map(|(_, v)| *v).sum();
        out[5 - age as usize] += v;
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    w: &mut StatWindow,
    ui: &mut egui::Ui,
    art: &mut Art,
    assets: &Assets,
    strings: &Strings,
    previews: &mut Previews,
    g: &mut Game,
    blocks: &mut Vec<ERect>,
) -> StatAction {
    let lay = layout(w.kind);
    let screen = ui.ctx().content_rect();
    let s = (screen.height() / 768.0).min(screen.width() / 1024.0);
    w.pos.x = w.pos.x.min(screen.width() / s - lay.width).max(0.0);
    let cv = Canvas {
        origin: screen.min + w.pos * s,
        scale: s,
        ppp: ui.ctx().pixels_per_point(),
    };
    let tr = |k: Text| strings.text(k).into_owned();
    // Frame.
    art.draw(ui, assets, cv, lay.rel, art::stat_window::TITLE_BAR, 0, 0.0, 0.0);
    // The resort and guest windows have one background per tab.
    let bg_index = match w.kind {
        StatKind::Resort | StatKind::Guests => w.tab,
        _ => 0,
    };
    let bg_y = if w.kind == StatKind::Finances { 89.0 } else { PANEL_Y };
    art.draw(ui, assets, cv, lay.rel, art::stat_window::BACKGROUND, bg_index, 0.0, bg_y);
    let bg_h = art.hit_rect(assets, lay.rel, art::stat_window::BACKGROUND, 0).map_or(234.0, |r| r.height() as f32);
    let whole = cv.rect_xywh(0.0, 0.0, lay.width, bg_y + bg_h);
    blocks.push(whole);
    ui.interact(whole, ui.scope_id().with(("statwin body", w.kind as u8)), Sense::click());
    let drag = ui.interact(cv.rect_xywh(0.0, 0.0, lay.width - 45.0, 50.0), ui.scope_id().with(("statwin drag", w.kind as u8)), Sense::drag());
    if drag.dragged() {
        w.pos += drag.drag_delta() / s;
    }
    let entrance = g
        .world
        .structures
        .iter()
        .find(|e| g.defs.structures[e.def].category == crate::game::defs::Category::Entrance)
        .cloned();
    // The resort window is titled with the resort's name.
    let title = match (&w.kind, &entrance) {
        (StatKind::Resort, Some(e)) => e.name.clone(),
        _ => tr(lay.title),
    };
    let title = title.split(" (").next().unwrap_or("").to_string();
    // Without a weather station the weather tab is locked (0x466360).
    let weather_built = g.world.structures.iter().any(|st| g.defs.structures[st.def].kind == crate::game::defs::StructureKind::WeatherStation);
    text(ui, cv, (lay.width - 40.0) * 0.5, 14.0, egui::Align2::CENTER_TOP, &title, TITLE);
    for (i, &(tab, x)) in lay.tabs.iter().enumerate() {
        if i == w.tab {
            continue;
        }
        let locked = w.kind == StatKind::Resort && i == 3 && !weather_built;
        if locked {
            tab.draw(ui, art, assets, cv, lay.rel, false, x, 56.0);
            let r = cv.rect_xywh(x, 56.0, 45.0, 33.0);
            if ui.interact(r, ui.scope_id().with(("statwin tab", w.kind as u8, i)), Sense::hover()).hovered() {
                egui::Tooltip::always_open(ui.ctx().clone(), ui.layer_id(), ui.scope_id().with("weather tip"), egui::PopupAnchor::Pointer)
                    .show(|ui| ui.label(tr(Text::RsdWeatherTabNotBuilt)));
            }
            continue;
        }
        tab.draw(ui, art, assets, cv, lay.rel, false, x, 56.0);
        let r = cv.rect_xywh(x, 56.0, 45.0, 33.0);
        if ui.interact(r, ui.scope_id().with(("statwin tab", w.kind as u8, i)), Sense::click()).clicked() {
            if w.tab != i {
                w.ticker = None;
            }
            w.tab = i;
            w.scroll = 0;
        }
    }
    let (tab, x) = lay.tabs[w.tab.min(lay.tabs.len() - 1)];
    tab.draw(ui, art, assets, cv, lay.rel, true, x, 56.0);
    if art.button(ui, assets, cv, lay.rel, art::stat_window::CLOSE, lay.close) {
        return StatAction::Close;
    }
    if ui.rect_contains_pointer(whole) {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy < -1.0 {
            w.scroll += 1;
        } else if dy > 1.0 {
            w.scroll = w.scroll.saturating_sub(1);
        }
    }
    let p = PANEL_Y;
    let mut action = StatAction::None;
    match (w.kind, w.tab) {
        (StatKind::Resort, 0) => {
            // The status strip scrolls a message from right to left; when
            // it has gone, the next is picked as 0x467510 does: one in 25
            // "Have fun!", else three in five the occupancy, then the
            // two sayings.
            let strip = cv.rect_xywh(32.0, p + 9.0, 150.0, 16.0);
            let r = (g.world.rand() * 1_000_000.0) as u32;
            let guests = g.world.active_guests();
            ticker(ui, strip, &mut w.ticker, s, || {
                if r % 50 == 17 || r % 50 == 19 {
                    "Have fun!".to_string()
                } else {
                    match (r / 50) % 5 {
                        0..=2 => tr(Text::RsdTickerGuestCount).replace("%d", &guests.to_string()),
                        3 => tr(Text::RsdTickerThanks),
                        _ => tr(Text::RsdTickerBadDaySkiing),
                    }
                }
            });
            if let Some(e) = &entrance {
                let d = &g.defs.structures[e.def];
                let area = cv.rect_xywh(34.0, p + 46.0, 146.0, 145.0);
                let yaw = g.camera.yaw - e.rot as f32 * std::f32::consts::FRAC_PI_2;
                let mesh = format!("{}{}", d.meshes.day, super::picker::WITH_SIGN);
                super::windows::world_preview(ui, cv, area, previews, &mesh, d.preview_zoom, yaw);
                // Lift ticket price, between the entrance's item_cost limits;
                // setting it makes the daily charge a quarter of it
                // (0x466260).
                let (lo, hi, step) = {
                    let d = &g.defs.structures[e.def];
                    (d.item_cost_min, d.item_cost_max.max(d.item_cost_min), d.price_step())
                };
                text(ui, cv, 214.0, p + 187.0, egui::Align2::CENTER_TOP, &money(e.price), BODY);
                let set = |g: &mut Game, by: f32| {
                    if let Some(m) = g.world.structure_mut(e.id) {
                        m.price = (m.price.trunc() + by).clamp(lo, hi);
                        m.daily_fee = Some(m.price * 0.25);
                    }
                };
                if art.button(ui, assets, cv, RESORT, art::resort_stats::SPINUP, (236.0, p + 176.0)) {
                    set(g, step);
                }
                if art.button(ui, assets, cv, RESORT, art::resort_stats::SPINDOWN, (236.0, p + 194.0)) {
                    set(g, -step);
                }
                if art.button(ui, assets, cv, RESORT, art::resort_stats::RENAME, (207.0, p + 65.0)) {
                    action = StatAction::Rename(Selection::Structure(e.id), e.name.clone());
                }
                if art.button(ui, assets, cv, RESORT, art::resort_stats::GOTO, (207.0, p + 104.0)) {
                    g.camera.target = e.pos;
                }
            }
        }
        (StatKind::Resort, 1) => {
            let v: Vec<f32> = g.world.history.iter().rev().take(HISTORY_DAYS).rev().map(|d| d.guests as f32).collect();
            graph(ui, cv, (64.0, p + 22.0, 243.0, p + 189.0), &v, 1000.0, Color32::from_rgb(0x20, 0x40, 0xc0));
        }
        (StatKind::Resort, 2) => {
            let v: Vec<f32> = g.world.history.iter().rev().take(HISTORY_DAYS).rev().map(|d| d.rating).collect();
            graph(ui, cv, (64.0, p + 22.0, 243.0, p + 189.0), &v, 5.0, Color32::from_rgb(0xd0, 0x90, 0x10));
        }
        (StatKind::Resort, 3) => {
            // As 0x4671cd fills it in.
            let wx = &g.world.weather;
            let strip = cv.rect_xywh(20.0, p + 7.0, 154.0, 20.0);
            ticker(ui, strip, &mut w.ticker, s, || {
                let (t, b) = wx.report();
                tr(Text::WeatherReportBase).replacen("%s", &strings.text(t), 1).replacen("%s", &strings.text(b), 1)
            });
            let temp = wx.temperature as i32;
            let baro = if g.defs.money_style == 1 {
                format!("{}", (wx.pressure * 33.86388) as i32)
            } else {
                format!("{:.2}", wx.pressure)
            };
            for (i, (k, v)) in [
                (Text::RsdTemperature, temp.to_string()),
                (Text::RsdBarometer, baro),
                (Text::RsdHumidity, format!("{}%", (wx.humidity * 100.0) as i32)),
            ]
            .iter()
            .enumerate()
            {
                let y = p + 36.0 + 20.0 * i as f32;
                text(ui, cv, 20.0, y, egui::Align2::LEFT_TOP, &tr(*k), BODY);
                text(ui, cv, 134.0, y, egui::Align2::CENTER_TOP, v, BODY);
            }
            // The wind compass, 16 headings.
            let dir = (wx.wind_dir.rem_euclid(std::f32::consts::TAU) * 2.546_479) as usize % 16;
            art.draw(ui, assets, cv, RESORT, art::resort_stats::WIND, dir, 179.0, p + 27.0);
            // Under it the original prints this figure, taken from the
            // temperature.
            text(ui, cv, 213.0, p + 117.0, egui::Align2::CENTER_TOP, &(((temp as f32 * 0.1) as i32 % 4) * 5).to_string(), BODY);
            // Avalanche danger (0x467392).
            let danger = g.avalanche_danger();
            let key = if danger > 0.66 {
                Text::RsdAvalancheDangerHigh
            } else if danger > 0.33 {
                Text::RsdAvalancheDangerMedium
            } else {
                Text::RsdAvalancheDangerLow
            };
            text(ui, cv, 24.0, p + 116.0, egui::Align2::LEFT_TOP, &tr(key), BODY);
            // Snow depth over the last week. The scale is an estimate: the
            // depth's 20..96 range spans the graph.
            let v: Vec<f32> = wx.depth_history.iter().rev().take(7).rev().chain(std::iter::once(&wx.snow_depth)).map(|d| (d - crate::game::weather::DEPTH_MIN).max(0.0)).collect();
            graph(ui, cv, (38.0, p + 151.0, 247.0, p + 211.0), &v, crate::game::weather::DEPTH_MAX - crate::game::weather::DEPTH_MIN, Color32::from_rgb(0x30, 0x60, 0xd0));
        }
        (StatKind::Resort, _) => {
            // Past messages, newest first, with scroll bars both ways as
            // in the original.
            let items: Vec<String> = g.world.messages.iter().rev().map(|m| m.text.clone()).collect();
            let (x1, y1, x2, y2) = (20.0, p + 8.0, 212.0, p + 172.0);
            let clip = cv.rect_xywh(x1, y1, x2 - x1, y2 - y1);
            let h = BODY.size + 4.0;
            let rows = ((y2 - y1) / h) as usize;
            w.scroll = w.scroll.min(items.len().saturating_sub(rows));
            let dx = w.hscroll as f32 * 10.0;
            for (i, t) in items.iter().enumerate().skip(w.scroll).take(rows) {
                let y = y1 + (i - w.scroll) as f32 * h;
                ui.painter()
                    .with_clip_rect(clip)
                    .text(cv.pos(x1 + 2.0 - dx, y), egui::Align2::LEFT_TOP, t, font(BODY, s), BODY.color);
            }
            let up_h = art.hit_rect(assets, RESORT, art::resort_stats::SCROLLUP, 0).map_or(33.0, |r| r.height() as f32);
            let bar_h = art.hit_rect(assets, RESORT, art::resort_stats::VERT_SCROLLBAR, 0).map_or(149.0, |r| r.height() as f32);
            if art.button(ui, assets, cv, RESORT, art::resort_stats::SCROLLUP, (226.0, p + 6.0)) {
                w.scroll = w.scroll.saturating_sub(1);
            }
            art.draw(ui, assets, cv, RESORT, art::resort_stats::VERT_SCROLLBAR, 0, 226.0, p + 6.0 + up_h);
            let span = items.len().saturating_sub(rows).max(1) as f32;
            let nub = (w.scroll as f32 / span).min(1.0) * (bar_h - 24.0);
            art.draw(ui, assets, cv, RESORT, art::resort_stats::VERT_SCROLLNUB, 0, 226.0, p + 6.0 + up_h + nub);
            if art.button(ui, assets, cv, RESORT, art::resort_stats::SCROLLDOWN, (226.0, p + 6.0 + up_h + bar_h)) {
                w.scroll += 1;
            }
            let left_w = art.hit_rect(assets, RESORT, art::resort_stats::SCROLLLEFT, 0).map_or(36.0, |r| r.width() as f32);
            let bar_w = art.hit_rect(assets, RESORT, art::resort_stats::HORIZ_SCROLLBAR, 0).map_or(131.0, |r| r.width() as f32);
            if art.button(ui, assets, cv, RESORT, art::resort_stats::SCROLLLEFT, (22.0, p + 180.0)) {
                w.hscroll = w.hscroll.saturating_sub(1);
            }
            art.draw(ui, assets, cv, RESORT, art::resort_stats::HORIZ_SCROLLBAR, 0, 22.0 + left_w, p + 180.0);
            let hnub = (w.hscroll as f32 / 30.0).min(1.0) * (bar_w - 20.0);
            art.draw(ui, assets, cv, RESORT, art::resort_stats::HORIZ_SCROLLNUB, 0, 22.0 + left_w + hnub, p + 180.0);
            if art.button(ui, assets, cv, RESORT, art::resort_stats::SCROLLRIGHT, (22.0 + left_w + bar_w, p + 180.0)) {
                w.hscroll += 1;
            }
        }
        (StatKind::Finances, tab) => {
            let by = 89.0;
            if tab == 0 {
                let rows: [(Text, &[Ledger]); 9] = [
                    (Text::ResortFinancesConstruction, &[Ledger::Construction]),
                    (Text::ResortFinancesOperations, &[Ledger::Operations]),
                    (Text::ResortFinancesMaintenance, &[Ledger::Maintenance]),
                    (Text::ResortFinancesLandscaping, &[Ledger::Landscaping]),
                    (Text::ResortFinancesAdmissions, &[Ledger::Tickets]),
                    (
                        Text::ResortFinancesSales,
                        &[Ledger::Food, Ledger::Rentals, Ledger::Lessons, Ledger::Retail, Ledger::Entertainment, Ledger::Services],
                    ),
                    (Text::ResortFinancesWages, &[Ledger::Wages]),
                    (Text::ResortFinancesLegal, &[Ledger::Legal]),
                    (Text::ResortFinancesLodging, &[Ledger::Lodging]),
                ];
                const COLS: [f32; 6] = [136.0, 219.0, 302.0, 385.0, 468.0, 551.0];
                let week_now = g.world.day / 7;
                for (c, x) in COLS.iter().enumerate() {
                    let wk = (week_now as i64 - 5 + c as i64) + 1;
                    if wk >= 1 {
                        let label = tr(Text::ResortFinancesWeek).replace("XX", &wk.to_string());
                        text(ui, cv, x + 40.0, by + 11.0, egui::Align2::CENTER_TOP, &label, BODY);
                    }
                }
                for (r, (label, ledgers)) in rows.iter().enumerate() {
                    let y = by + 31.0 + 20.0 * r as f32;
                    text(ui, cv, 21.0, y, egui::Align2::LEFT_TOP, &tr(*label), BODY);
                    let vals = weekly(g, ledgers);
                    for (c, x) in COLS.iter().enumerate() {
                        let v = vals[c];
                        let style = TextStyle {
                            color: if v < 0.0 { Color32::from_rgb(0xb0, 0x10, 0x10) } else { BODY.color },
                            ..BODY
                        };
                        text(ui, cv, x + 76.0, y, egui::Align2::RIGHT_TOP, &money(v), style);
                    }
                }
            } else {
                let v: Vec<f32> = g
                    .world
                    .history
                    .iter()
                    .rev()
                    .take(HISTORY_DAYS)
                    .rev()
                    .map(|d| if tab == 1 { d.cash } else { d.assets })
                    .collect();
                // Scaled to the highest day plus one (0x465a90).
                let max = v.iter().cloned().fold(0.0f32, f32::max);
                let r = (20.0, by + 12.0, 630.0, by + 206.0);
                ui.painter().rect_filled(cv.rect_xywh(r.0, r.1, r.2 - r.0, r.3 - r.1), 4.0, Color32::WHITE);
                text(ui, cv, r.0 + 4.0, r.1 + 2.0, egui::Align2::LEFT_TOP, &money(max), BODY);
                graph(ui, cv, r, &v, max + 1.0, Color32::from_rgb(0x10, 0x80, 0x30));
            }
        }
        (StatKind::Guests, 0) => {
            let gs = &g.world.guests;
            let n = gs.len() as f32;
            // With nobody there, every meter shows full.
            let avg = |f: &dyn Fn(&crate::game::world::Guest) -> f32| if gs.is_empty() { 1.0 } else { gs.iter().map(f).sum::<f32>() / n };
            let rows = [
                (Text::AllGuestsFood, avg(&|x| x.need(NeedKind::Food))),
                (Text::AllGuestsEnergy, avg(&|x| x.need(NeedKind::Rest))),
                (Text::AllGuestsWarmth, avg(&|x| x.need(NeedKind::Warmth))),
                (Text::AllGuestsHealth, avg(&|x| x.need(NeedKind::Health))),
                (Text::AllGuestsEntertainment, avg(&|x| x.need(NeedKind::Entertainment))),
                (Text::AllGuestsRestroom, avg(&|x| x.need(NeedKind::Bathroom))),
                (Text::AllGuestsHappiness, avg(&|x| x.happiness)),
                // How little they will pay (0x428c9d).
                (Text::AllGuestsThrifty, avg(&|x| 1.0 - x.tolerance)),
            ];
            // Rows 31 pixels apart, each a label and a meter (0x468bd0)
            // at x 116 level with the label's box.
            for (i, (k, v)) in rows.iter().enumerate() {
                let y = p + 9.0 + 31.0 * i as f32;
                text(ui, cv, 19.0, y + 2.0, egui::Align2::LEFT_TOP, &tr(*k), BODY);
                meter(ui, art, assets, cv, 116.0, y, *v);
            }
        }
        (StatKind::Guests, tab) => {
            let ids: Vec<_> = g.world.guests.iter().map(|x| x.id).collect();
            let items: Vec<String> = g
                .world
                .guests
                .iter()
                .map(|x| {
                    if w.actions {
                        format!("{}: {}", x.name, super::state_label(strings, x, &g.world))
                    } else {
                        format!("{}: {}", x.name, x.thought.as_ref().map(|t| super::thought_text(strings, t.0, x.id)).unwrap_or_default())
                    }
                })
                .collect();
            if art.button(ui, assets, cv, GUESTS, art::guests_stats::THOUGHT, (99.0, 310.0)) {
                w.actions = false;
            }
            if art.button(ui, assets, cv, GUESTS, art::guests_stats::ACTION, (137.0, 310.0)) {
                w.actions = true;
            }
            let _ = tab;
            if let Some(i) = list(ui, cv, (25.0, p + 11.0, 216.0, p + 177.0), &items, w.scroll, "guests") {
                action = StatAction::Select(Selection::Guest(ids[i]));
            }
        }
        (StatKind::Staff, tab) => {
            let kind = STAFF_TABS[tab.min(4)];
            let staff: Vec<_> = g.world.staff.iter().filter(|x| x.kind == kind).collect();
            let ids: Vec<_> = staff.iter().map(|x| x.id).collect();
            let items: Vec<String> = staff.iter().map(|x| format!("{}: {}", x.name, super::staff_activity(strings, x, &g.world))).collect();
            if let Some(i) = list(ui, cv, (21.0, 95.0 + 12.0, 229.0, 280.0), &items, w.scroll, "staff") {
                action = StatAction::Select(Selection::Staff(ids[i]));
            }
        }
        (StatKind::Attractions, tab) => {
            let w_ = &g.world;
            let (sels, items): (Vec<Selection>, Vec<String>) = match tab {
                0 => w_
                    .structures
                    .iter()
                    .map(|st| (Selection::Structure(st.id), st.name.clone()))
                    .unzip(),
                1 => w_
                    .lifts
                    .iter()
                    .map(|l| (Selection::Lift(l.id), l.name.clone()))
                    .unzip(),
                2 => w_
                    .runs
                    .iter()
                    .map(|r| (Selection::Run(r.id), r.name.clone()))
                    .unzip(),
                _ => w_
                    .parks
                    .iter()
                    .map(|pk| (Selection::Park(pk.id), pk.name.clone()))
                    .unzip(),
            };
            if let Some(i) = list(ui, cv, (25.0, p + 12.0, 215.0, p + 176.0), &items, w.scroll, "attractions") {
                action = StatAction::Select(sels[i].clone());
            }
        }
    }
    action
}

/// The all-guests meter (0x468d10): its tray, and over it the graded
/// graph cut to 4k + 1 pixels, k being 35v − 1 truncated.
#[allow(clippy::too_many_arguments)]
fn meter(ui: &mut egui::Ui, art: &mut Art, assets: &Assets, cv: Canvas, x: f32, y: f32, v: f32) {
    use super::art::scaled_stats_bar as bar;
    art.draw(ui, assets, cv, bar::FILE, bar::TRAY, 0, x, y);
    let k = (v * 35.0 - 1.0) as i32;
    let w = (4 * k + 1) as f32;
    if w <= 0.0 {
        return;
    }
    let h = art.hit_rect(assets, bar::FILE, bar::GRAPH, 0).map_or(20.0, |r| r.height() as f32);
    let mut clipped = ui.new_child(egui::UiBuilder::new());
    clipped.set_clip_rect(cv.rect_xywh(x, y, w, h - 1.0).intersect(ui.clip_rect()));
    art.draw(&clipped, assets, cv, bar::FILE, bar::GRAPH, 0, x, y);
}
