//! egui front end: menus and in-game panels.

pub mod art;
pub mod bob;
pub mod classic;
use crate::game::defs::StructureKind;
pub mod fonts;
pub mod hud;
pub mod fog;
pub mod genmap;
pub mod menus;
pub mod minimap;
pub mod picker;
pub mod settings_art;
pub mod snowfall;
pub mod statwin;
pub mod windows;
pub mod style;

use std::path::PathBuf;

use egui::{Align2, Color32};

use crate::assets::Assets;
use crate::formats::ini::Ini;
use crate::game::thought::{Thought, Variant};
use crate::game::world::*;
use crate::game::{Game, Tool};
use crate::text::{Text, TextKey};
use art::{ArtFile, Group};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    MainMenu,
    NewGame,
    Challenges,
    LoadGame,
    Settings,
    Credits,
    MapEditor,
    InGame,
}

pub enum Action {
    StartLevel(String),
    StartEditor(crate::game::editor::MapSource),
    LoadSave(PathBuf),
    QuitToMenu,
    ToggleFullscreen,
    Exit,
}

/// Title and structure types of each build tray category, from the
/// original building picker.
pub const BUILD_CATEGORIES: [(Text, &[StructureKind]); 8] = {
    use StructureKind::*;
    [
        (Text::ScmLodging, &[Lodging]),
        (Text::ScmFood, &[Restaurant]),
        (Text::ScmEntertainment, &[Entertainment, RadioStation, SnowmobileTour, HeliTour]),
        (Text::ScmGuestComfort, &[Bathroom, WarmupHut]),
        (Text::ScmRentals, &[Rental, Retail]),
        (Text::ScmServices, &[SkiPatrol, SnowmobilePatrol, ChopperPatrol, SkiSchool, Bank, Daycare]),
        (Text::ScmUtilities, &[PowerPlant, WeatherStation]),
        (Text::ScmMaintenance, &[Maintenance, Sanitation, StaffHousing, HeliTour, Realtor, Marketing, Security, WildlifeControl, Groomer]),
    ]
};


#[derive(Default)]
pub struct Ui {
    pub screen: Screen,
    pub error: Option<String>,
    pub show_menu: bool,
    pub save_name: String,
    pub fx_volume: f32,
    pub music_volume: f32,
    pub music_started: bool,
    pub music: Option<crate::music::Music>,
    pub art: classic::Art,
    pub settings: crate::settings::Settings,
    pub difficulty: usize,
    pub start_cash: f32,
    pub map_editor: menus::MapEditorState,
    /// The fog dialog, for what is to be followed (see `fog`).
    pub fog_for: Option<crate::game::Selection>,
    /// The map editor's Generate Random Map window, when open.
    pub gen_window: Option<genmap::GenWindow>,
    pub level_list: menus::ListState,
    pub challenge_list: menus::ListState,
    pub save_list: menus::ListState,
    pub credits_t: f32,
    pub hud: hud::Hud,
    pub strings: Option<crate::formats::eng::Strings>,
    pub show_minimap: bool,
    pub minimap: minimap::Minimap,
    pub snowfall: snowfall::Snowfall,
    /// Object windows opened so far, for the original's cascade.
    pub windows_opened: u32,
    /// The save screen of the escape menu is showing.
    pub save_open: bool,
    pub paused_before_menu: bool,
    pub picker: Option<picker::Picker>,
    pub previews: picker::Previews,
    pub obj_window: Option<windows::ObjWindow>,
    pub stat_windows: Vec<statwin::StatWindow>,
    pub rename: Option<windows::Rename>,
    /// Bob dialogs waiting to be shown, oldest first.
    pub bob: Vec<bob::BobMessage>,
    /// The challenge step whose message is queued or showing.
    pub challenge_shown: Option<u32>,
}

pub fn saves_dir() -> PathBuf {
    crate::assets::data_home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("srt/saves")
}

pub(crate) struct Challenge {
    pub index: usize,
    pub title: String,
    pub level: String,
    pub description: String,
}

pub(crate) fn challenges(assets: &Assets, game_strings: &crate::formats::eng::Strings) -> Vec<Challenge> {
    let Some(p) = assets.resolve("data/challenges/challenges.ini") else { return Vec::new() };
    let Ok(ini) = Ini::load(p) else { return Vec::new() };
    let mut out: Vec<(i32, Challenge)> = Vec::new();
    for s in &ini.sections {
        let Some(num) = s.name.strip_prefix("Challenge_").and_then(|n| n.parse::<usize>().ok()) else {
            continue;
        };
        if !s.bool_or("available", false) {
            continue;
        }
        let level = s.str_or("Level", "").to_string();
        if assets.resolve(&level).is_none() {
            continue;
        }
        out.push((
            s.i32_or("listbox_position", num as i32),
            Challenge {
                index: num,
                title: game_strings.lookup(s.str_or("Title", "")).to_string(),
                level,
                description: game_strings.lookup(s.str_or("Description", "")).replace("\\n", "\n"),
            },
        ));
    }
    out.sort_by_key(|c| c.0);
    out.into_iter().map(|c| c.1).collect()
}

pub(crate) fn ui_strings(assets: &Assets) -> crate::formats::eng::Strings {
    assets
        .resolve("data/ui/shell.eng")
        .and_then(|p| p.parent())
        .and_then(|d| crate::formats::eng::Strings::load_dir(d, "eng").ok())
        .unwrap_or_default()
}

impl Ui {
    pub fn show(&mut self, root: &mut egui::Ui, assets: &Assets, game: Option<&mut Game>) -> Vec<Action> {
        let mut actions = Vec::new();
        if !self.music_started {
            self.music_started = true;
            self.fx_volume = self.settings.sound as f32 / 5.0;
            self.music_volume = self.settings.music as f32 / 5.0;
            self.settings.apply_audio();
        }
        let mut game = game;
        let music = self.music.get_or_insert_with(|| crate::music::Music::new(assets));
        let dt = root.ctx().input(|i| i.stable_dt).min(0.1);
        match (&self.screen, game.as_deref_mut()) {
            (Screen::InGame, Some(g)) => {
                if let Some(p) = g.world.weather.started.take() {
                    music.weather_started(assets, p == crate::game::weather::Precip::Rain);
                }
                music.update(assets, dt, Some(g.world.is_night()), self.show_menu);
                let (ctrl, right, left) = root.ctx().input(|i| {
                    (i.modifiers.ctrl, i.key_pressed(egui::Key::ArrowRight), i.key_pressed(egui::Key::ArrowLeft))
                });
                if ctrl && (right || left) {
                    music.step(assets, right);
                }
            }
            _ => music.update(assets, dt, None, false),
        }
        if let Some(err) = self.error.take() {
            self.bob.push(bob::BobMessage {
                mood: bob::Mood::Unhappy,
                text: err,
                step: None,
            });
        }
        match (self.screen, game.as_deref_mut()) {
            (Screen::InGame, Some(g)) => self.in_game(root, assets, g, &mut actions),
            (Screen::InGame, None) => self.screen = Screen::MainMenu,
            _ => self.menus(root, assets, &mut actions),
        }
        if let Some(m) = self.bob.first() {
            let mut art = std::mem::take(&mut self.art);
            art.sounds = classic::UiSounds::Menu;
            if bob::show(m, root, &mut art, assets, &mut self.hud.blocks) {
                let m = self.bob.remove(0);
                if let (Some(step), Some(c)) = (m.step, game.as_deref_mut().and_then(|g| g.world.challenge_state.as_mut())) {
                    c.acknowledge(step);
                    self.challenge_shown = None;
                }
            }
            self.art = art;
        }
        actions
    }

    // --------------------------------------------------------------- menus

    fn menus(&mut self, root: &mut egui::Ui, assets: &Assets, actions: &mut Vec<Action>) {
        self.art.sounds = classic::UiSounds::Menu;
        if self.classic_menus(root, assets, actions) {
            return;
        }
        // Without the original art there is nothing to show but how to
        // install it.
        let full = root.ctx().content_rect();
        root.painter().rect_filled(full, 0.0, Color32::BLACK);
        root.painter().text(
            full.center(),
            Align2::CENTER_CENTER,
            "Game data not found.\nRun `srt extract <CD image>` or pass --data DIR.",
            egui::FontId::proportional(20.0),
            Color32::WHITE,
        );
    }

    // ------------------------------------------------------------- in game

    fn in_game(&mut self, root: &mut egui::Ui, assets: &Assets, g: &mut Game, actions: &mut Vec<Action>) {
        let ctx = &root.ctx().clone();
        let strings = self.strings.take().unwrap_or_else(|| ui_strings(assets));
        if let Some(sel) = g.fog_request.take() {
            self.fog_for = Some(sel);
        }
        // The follow view hides everything but "Press space" (0x4619b0).
        if g.eye.is_some() {
            self.strings = Some(strings);
            self.hud.blocks.clear();
            self.snowfall.show(root, g);
            self.press_space(root, assets);
            return;
        }
        // Markers over people and buildings belong to the scene, under
        // the falling snow, the HUD and every window.
        self.thought_icons(root, assets, g);
        self.power_icons(root, assets, g);
        self.snowfall.show(root, g);
        let mut art = std::mem::take(&mut self.art);
        art.sounds = classic::UiSounds::InGame;
        let events = self.hud.show(root, &mut art, assets, &strings, g);
        self.art = art;
        self.strings = Some(strings);
        let mut events = events;
        // Esc opens the menu when nothing else wants it, as in the original.
        if !self.show_menu
            && g.tool == Tool::Inspect
            && g.eye.is_none()
            && self.fog_for.is_none()
            && g.selection.is_none()
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            events.push(hud::HudEvent::MainMenu);
        }
        for e in events {
            match e {
                hud::HudEvent::MainMenu => {
                    self.paused_before_menu = g.paused;
                    g.paused = true;
                    self.show_menu = true;
                    self.hud.close_menus();
                }
                hud::HudEvent::ToggleMinimap => self.show_minimap = !self.show_minimap,
                hud::HudEvent::Command(c) => self.command(c, g),
            }
        }
        // The pickers cover the mini-map, as the objects menu does in the
        // bottom right corner.
        if self.show_minimap {
            let mut art = std::mem::take(&mut self.art);
            art.sounds = classic::UiSounds::InGame;
            if minimap::show(&mut self.minimap, root, &mut art, assets, g, &mut self.hud.blocks) && self.gen_window.is_none() {
                self.gen_window = Some(genmap::GenWindow::open(g));
            }
            self.art = art;
        }
        if let Some(mut w) = self.gen_window.take() {
            let mut art = std::mem::take(&mut self.art);
            art.sounds = classic::UiSounds::InGame;
            if w.show(root, &mut art, assets, g, &mut self.hud.blocks) {
                self.gen_window = Some(w);
            }
            self.art = art;
        }
        if let Some(mut p) = self.picker.take() {
            let strings = self.strings.take().unwrap_or_default();
            let mut art = std::mem::take(&mut self.art);
            art.sounds = classic::UiSounds::InGame;
            let r = picker::show(&mut p, root, &mut art, assets, &strings, &mut self.previews, g, &mut self.hud.blocks);
            self.art = art;
            self.strings = Some(strings);
            if matches!(r, picker::PickerResult::Open) {
                self.picker = Some(p);
            }
        }
        self.stat_windows(root, assets, g);
        self.object_window(root, assets, g);
        self.challenge_messages(root, assets, g);
        self.floaters(root, g);
        if self.show_menu {
            self.escape_screen(root, assets, g, actions);
        }
        if let Some(sel) = self.fog_for.clone() {
            let mut art = std::mem::take(&mut self.art);
            art.sounds = classic::UiSounds::InGame;
            if let Some(sight) = fog::show(root, &mut art, assets, &mut self.hud.blocks) {
                self.fog_for = None;
                g.enter_eye(sel, sight);
            }
            self.art = art;
        }
    }

    /// "Press space" (`SpaceBar`), centered 80 above the screen's foot
    /// (0x460b00).
    fn press_space(&mut self, root: &mut egui::Ui, assets: &Assets) {
        let screen = root.ctx().content_rect();
        let s = (screen.height() / 768.0).min(screen.width() / 1024.0);
        let Some((_, r)) = self.art.group_element(assets, art::nav_bar::FILE, art::nav_bar::SPACE_BAR, 0) else { return };
        let w = if r.width() > 1 { r.width() as f32 } else { 257.0 };
        let cv = classic::Canvas {
            origin: egui::Pos2::new(screen.center().x - w * 0.5 * s, screen.max.y - 80.0 * s),
            scale: s,
            ppp: root.ctx().pixels_per_point(),
        };
        self.art.draw(root, assets, cv, art::nav_bar::FILE, art::nav_bar::SPACE_BAR, 0, 0.0, 0.0);
    }

    fn toggle_stats(&mut self, kind: statwin::StatKind) {
        if let Some(i) = self.stat_windows.iter().position(|w| w.kind == kind) {
            self.stat_windows.remove(i);
        } else {
            self.stat_windows.push(statwin::StatWindow::new(kind));
        }
    }

    fn stat_windows(&mut self, root: &mut egui::Ui, assets: &Assets, g: &mut Game) {
        let strings = self.strings.take().unwrap_or_default();
        let mut art = std::mem::take(&mut self.art);
        art.sounds = classic::UiSounds::Silent;
        let mut keep = Vec::new();
        for mut w in std::mem::take(&mut self.stat_windows) {
            match statwin::show(&mut w, root, &mut art, assets, &strings, &mut self.previews, g, &mut self.hud.blocks) {
                statwin::StatAction::Close => {
                    // The entrance's window is the resort window: closing
                    // it lets go of the entrance, which would reopen it.
                    if w.kind == statwin::StatKind::Resort && entrance_selected(g) {
                        g.selection = None;
                    }
                }
                statwin::StatAction::Select(sel) => {
                    g.selection = Some(sel);
                    keep.push(w);
                }
                statwin::StatAction::Rename(sel, text) => {
                    self.rename = Some(windows::Rename { sel, text });
                    keep.push(w);
                }
                statwin::StatAction::None => keep.push(w),
            }
        }
        self.stat_windows = keep;
        self.art = art;
        self.strings = Some(strings);
    }

    fn open_picker(&mut self, kind: picker::PickKind) {
        match &mut self.picker {
            Some(p) if p.kind == kind => self.picker = None,
            Some(p) => {
                let pos = p.pos;
                let mut n = picker::Picker::new(kind);
                n.pos = pos;
                self.picker = Some(n);
            }
            None => self.picker = Some(picker::Picker::new(kind)),
        }
    }

    fn command(&mut self, c: hud::Command, g: &mut Game) {
        use hud::Command as C;
        match c {
            C::Buildings(k) => self.open_picker(picker::PickKind::Buildings(k)),
            C::Lifts => self.open_picker(picker::PickKind::Lifts),
            C::Jumps => self.open_picker(picker::PickKind::Jumps),
            C::RunMarkers => self.open_picker(picker::PickKind::Runs),
            C::Objects => self.open_picker(picker::PickKind::Objects),
            C::Paths => self.open_picker(picker::PickKind::Paths),
            C::RunPath(mode) => g.tool = Tool::RunPath { mode },
            C::Surface(t) => g.tool = Tool::Terrain(t),
            C::Trees(d) => g.tool = Tool::Trees { density: d },
            C::ResortStats => self.toggle_stats(statwin::StatKind::Resort),
            C::AttractionStats => self.toggle_stats(statwin::StatKind::Attractions),
            C::Finances => self.toggle_stats(statwin::StatKind::Finances),
            C::GuestStats => self.toggle_stats(statwin::StatKind::Guests),
            C::StaffStats => self.toggle_stats(statwin::StatKind::Staff),
        }
    }

    fn object_window(&mut self, root: &mut egui::Ui, assets: &Assets, g: &mut Game) {
        // The entrance opens the resort window, as in the original.
        if entrance_selected(g) {
            if !self.stat_windows.iter().any(|w| w.kind == statwin::StatKind::Resort) {
                self.stat_windows.push(statwin::StatWindow::new(statwin::StatKind::Resort));
            }
            self.obj_window = None;
            return self.rename_box(root, assets, g);
        }
        match (&g.selection, &mut self.obj_window) {
            (None, _) => self.obj_window = None,
            (Some(sel), Some(w)) if w.sel == *sel => {}
            (Some(sel), w) => {
                let pos = w.as_ref().map(|w| w.pos);
                let mut n = windows::ObjWindow::new(sel.clone());
                match pos {
                    Some(p) => n.pos = p,
                    None => {
                        // The original opens them at the top left under the
                        // bar, each one 30 pixels further down and right.
                        // Where the cascade wraps is not known.
                        let k = (self.windows_opened % 8) as f32;
                        n.pos = egui::Vec2::new(30.0 * k, 42.0 + 30.0 * k);
                        self.windows_opened += 1;
                    }
                }
                self.obj_window = Some(n);
            }
        }
        let strings = self.strings.take().unwrap_or_default();
        let mut art = std::mem::take(&mut self.art);
        art.sounds = classic::UiSounds::Silent;
        if let Some(mut w) = self.obj_window.take() {
            let songs = self.music.as_ref().map(|m| m.song_names()).unwrap_or_default();
            match windows::show(&mut w, root, &mut art, assets, &strings, &mut self.previews, g, &mut self.hud.blocks, &songs) {
                windows::WindowAction::Close => g.selection = None,
                windows::WindowAction::Rename(sel, text) => {
                    self.rename = Some(windows::Rename { sel, text });
                    self.obj_window = Some(w);
                }
                windows::WindowAction::None => self.obj_window = Some(w),
                windows::WindowAction::Music(pick) => {
                    if let Some(m) = self.music.as_mut() {
                        match pick {
                            Some(i) if i < songs.len() => m.choose(assets, i),
                            Some(_) => m.shuffle(assets),
                            None => m.stop(),
                        }
                    }
                    self.obj_window = Some(w);
                }
            }
        }
        self.art = art;
        self.strings = Some(strings);
        self.rename_box(root, assets, g);
    }

    fn rename_box(&mut self, root: &mut egui::Ui, assets: &Assets, g: &mut Game) {
        let mut art = std::mem::take(&mut self.art);
        if let Some(mut r) = self.rename.take() {
            match windows::rename(&mut r, root, &mut art, assets, &mut self.hud.blocks) {
                Some(Some(name)) if !name.is_empty() => g.rename(&r.sel, name),
                Some(_) => {}
                None => self.rename = Some(r),
            }
        }
        self.art = art;
    }

    /// The original's thought icons (`thoughts.bin`) over guests that have
    /// something on their mind.
    fn thought_icons(&mut self, root: &egui::Ui, assets: &Assets, g: &Game) {
        const THOUGHTS: ArtFile = art::thoughts::FILE;
        let rect = root.ctx().content_rect();
        let ppp = root.ctx().pixels_per_point();
        let s = (rect.height() / 768.0).min(rect.width() / 1024.0);
        let size = glam::Vec2::new(rect.width() * ppp, rect.height() * ppp);
        let mut art = std::mem::take(&mut self.art);
        for x in &g.world.guests {
            let Some((thought, _)) = x.thought else { continue };
            if x.pos.z < -100.0 || x.pos.distance(g.camera.eye()) > 900.0 {
                continue;
            }
            let Some(p) = g.camera.project(x.pos + glam::Vec3::Z * 5.0, size) else { continue };
            let cv = classic::Canvas {
                origin: egui::pos2(p.x / ppp, p.y / ppp),
                scale: s,
                ppp,
            };
            art.draw(root, assets, cv, THOUGHTS, thought_icon(thought), 0, -7.0, -14.0);
        }
        self.art = art;
    }

    /// Animated markers over buildings: the electric ring when without
    /// power (`no_power.bin`) and the wrench when broken
    /// (`highlevel_statistics.bin`). Each comes in four sizes, 11 to 95
    /// pixels; the one closest to the building's width on screen is used.
    /// Frame rate not known.
    fn power_icons(&mut self, root: &egui::Ui, assets: &Assets, g: &Game) {
        const NO_POWER: ArtFile = art::no_power::FILE;
        const WRENCH: ArtFile = art::highlevel_stats::FILE;
        // The four sizes' widths, and each file's groups for them.
        const SIZES: [f32; 4] = [11.0, 23.0, 47.0, 95.0];
        const RINGS: [Group; 4] = [art::no_power::ZOOM1, art::no_power::ZOOM2, art::no_power::ZOOM3, art::no_power::ZOOM4];
        const WRENCHES: [Group; 4] = [
            art::highlevel_stats::WRENCH_ZOOM1,
            art::highlevel_stats::WRENCH_ZOOM2,
            art::highlevel_stats::WRENCH_ZOOM3,
            art::highlevel_stats::WRENCH_ZOOM4,
        ];
        let rect = root.ctx().content_rect();
        let ppp = root.ctx().pixels_per_point();
        let s = (rect.height() / 768.0).min(rect.width() / 1024.0);
        let size = glam::Vec2::new(rect.width() * ppp, rect.height() * ppp);
        let frame = (root.input(|i| i.time) * 10.0) as usize % 10;
        let spacing = g.world.level().spacing;
        let mut art = std::mem::take(&mut self.art);
        for st in &g.world.structures {
            let d = &g.defs.structures[st.def];
            let (file, groups) = if st.broken {
                (WRENCH, WRENCHES)
            } else if !st.powered && d.uses_electricity {
                (NO_POWER, RINGS)
            } else {
                continue;
            };
            let w = d.size.0.max(d.size.1) as f32 * spacing;
            let (Some(a), Some(b)) = (g.camera.project(st.pos, size), g.camera.project(st.pos + glam::Vec3::X * w, size)) else { continue };
            let width = a.distance(b) / ppp / s;
            let k = (0..4).min_by(|&x, &y| (SIZES[x] - width).abs().total_cmp(&(SIZES[y] - width).abs())).unwrap_or(2);
            let (group, px) = (groups[k], SIZES[k]);
            let Some(c) = g.camera.project(st.pos + glam::Vec3::Z * w * 0.3, size) else { continue };
            let cv = classic::Canvas {
                origin: egui::pos2(c.x / ppp, c.y / ppp),
                scale: s,
                ppp,
            };
            art.draw(root, assets, cv, file, group, frame, -px * 0.5, -px * 0.5);
        }
        self.art = art;
        root.ctx().request_repaint();
    }

    /// Money spent flies from the new object to the bottom-left corner,
    /// easing in (see `game::FLOATER_SECS`): bold, measured 0x00009c.
    fn floaters(&mut self, root: &egui::Ui, g: &Game) {
        if g.floaters.is_empty() {
            return;
        }
        let rect = root.ctx().content_rect();
        let ppp = root.ctx().pixels_per_point();
        let s = (rect.height() / 768.0).min(rect.width() / 1024.0);
        let size = glam::Vec2::new(rect.width() * ppp, rect.height() * ppp);
        let target = egui::pos2(rect.min.x, rect.max.y + rect.height() * 0.08);
        let font = egui::FontId::new(14.0 * s, egui::FontFamily::Name(fonts::ARIAL_BOLD.into()));
        let painter = root.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("floaters")));
        for f in &g.floaters {
            let Some(p) = g.camera.project(f.at, size) else { continue };
            let start = egui::pos2(p.x / ppp, p.y / ppp);
            let u = (f.age / crate::game::FLOATER_SECS).clamp(0.0, 1.0);
            let at = start + (target - start) * (u * u);
            painter.text(at, egui::Align2::CENTER_CENTER, &f.text, font.clone(), egui::Color32::from_rgb(0, 0, 0x9c));
        }
        root.ctx().request_repaint();
    }

    /// The challenge's messages in Bob's dialog, one per message step,
    /// and the Bob button while it watches for its goal.
    fn challenge_messages(&mut self, root: &mut egui::Ui, assets: &Assets, g: &mut Game) {
        use crate::game::challenge::Say;
        let Some(c) = g.world.challenge_state.as_mut() else { return };
        let strings = self.strings.get_or_insert_with(Default::default);
        if let Some(p) = c.prompt() {
            if self.challenge_shown != Some(c.step) {
                self.challenge_shown = Some(c.step);
                let text = match p.say {
                    Say::Text(t) => strings.text(t).into_owned(),
                    Say::Builtin(s) => s.to_string(),
                    Say::OlympicProgress { built, count } => {
                        // What is still to build, by what is built
                        // (0x4136e0 step 4).
                        let rest = match built {
                            1 => Some(Text::OlymicMsgStructures1),
                            2 => Some(Text::OlymicMsgStructures2),
                            3 => Some(Text::OlymicMsgStructures3),
                            4 => Some(Text::OlymicMsgStructures4),
                            5 => Some(Text::OlymicMsgStructures5),
                            6 => Some(Text::OlymicMsgStructures6),
                            _ => None,
                        };
                        let rest = rest.map(|t| strings.text(t).into_owned()).unwrap_or_default();
                        let frame = if count == 2 { Text::OlympicMsgOneStructure } else { Text::OlympicMsgTwoStructures };
                        strings.text(frame).replacen("%s", &rest, 1)
                    }
                };
                self.bob.push(bob::BobMessage { mood: bob::Mood::from_index(p.mood), text, step: Some(c.step) });
            }
        } else if c.waiting() && self.bob.is_empty() {
            let mut art = std::mem::take(&mut self.art);
            art.sounds = classic::UiSounds::InGame;
            if bob::button(root, &mut art, assets, &mut self.hud.blocks) {
                c.recall();
            }
            self.art = art;
        }
    }

}

fn entrance_selected(g: &Game) -> bool {
    match g.selection {
        Some(crate::game::Selection::Structure(id)) => g
            .world
            .structure(id)
            .is_some_and(|st| g.defs.structures[st.def].category == crate::game::defs::Category::Entrance),
        _ => false,
    }
}

/// What a guest is doing, in the original's words (`SKIER_THOUGHT_*`).
/// On the way up, the name is of the run or park they chose, as the
/// original names the guest's target (+0xa4, 0x4908xx).
pub(crate) fn state_label(strings: &crate::formats::eng::Strings, guest: &crate::game::world::Guest, w: &World) -> String {
    use crate::game::world::Aim;
    let with = |key: Text, name: &str| strings.text(key).replace("%s", name);
    let aimed = guest.aim.and_then(|a| match a {
        Aim::Run(id) => w.runs.iter().find(|r| r.id == id).map(|r| r.name.as_str()),
        Aim::Park(id) => w.parks.iter().find(|p| p.id == id).map(|p| p.name.as_str()),
    });
    let lift = |id: Id| aimed.or_else(|| w.lift(id).map(|l| l.name.as_str())).unwrap_or("");
    match &guest.state {
        GuestState::Walking { purpose, .. } => match purpose {
            Purpose::Lift(id) => with(Text::SkierThoughtGoingTo, lift(*id)),
            Purpose::Structure(id) => with(Text::SkierThoughtGoingTo, w.structure(*id).map(|s| s.name.as_str()).unwrap_or("")),
            Purpose::Exit => strings.text(Text::SkierThoughtLeaving).into_owned(),
            Purpose::Park(id) => with(Text::SkierThoughtGoingTo, w.parks.iter().find(|p| p.id == *id).map(|p| p.name.as_str()).unwrap_or("")),
            Purpose::Continue => strings.text(Text::SkierThoughtSkiing).into_owned(),
            Purpose::Wander => strings.text(Text::SkierThoughtWandering).into_owned(),
        },
        GuestState::InStructure { structure, .. } => with(Text::SkierThoughtInside, w.structure(*structure).map(|s| s.name.as_str()).unwrap_or("")),
        GuestState::Queued { lift: id } => with(Text::SkierThoughtInLine, lift(*id)),
        GuestState::AtDoor { structure } => with(Text::SkierThoughtInLine, w.structure(*structure).map(|s| s.name.as_str()).unwrap_or("")),
        GuestState::Riding { lift: id } => with(Text::SkierThoughtRiding, lift(*id)),
        GuestState::Skiing { .. } => strings.text(Text::SkierThoughtSkiing).into_owned(),
        GuestState::Crashed { .. } => strings.text(Text::SkierThoughtWounded).into_owned(),
        GuestState::Controlled | GuestState::Held => strings.text(Text::SkierThoughtHandOfGod).into_owned(),
        GuestState::Seized => strings.text(Text::SkierThoughtThinking).into_owned(),
        GuestState::Injured | GuestState::Dying { .. } => strings.text(Text::SkierThoughtWoundedWaiting).into_owned(),
        GuestState::Rescued => strings.text(Text::SkierThoughtWounded).into_owned(),
        GuestState::Leaving | GuestState::Gone => strings.text(Text::SkierThoughtLeaving).into_owned(),
    }
}



/// What an employee is doing, in the original's words (`Staff.eng`).
pub(crate) fn staff_activity(strings: &crate::formats::eng::Strings, s: &crate::game::world::Staff, w: &World) -> String {
    use crate::game::world::{MortarStage, StaffKind, StaffTask};
    let t = |k: Text| strings.text(k).into_owned();
    let named = |k: Text, name: Option<&str>| format!("{} {}", strings.text(k).trim_end(), name.unwrap_or("")).trim_end().to_string();
    // Unpaid staff sulk (state 3, STAFF_FIRED, "Sulking. . ."); a tick's
    // wage is a few cents, so that is when the money has run out.
    // Instructors keep teaching regardless.
    let unpaid = s.wage > 0.0 && w.cash <= 0.0 && s.kind != StaffKind::Instructor;
    match (&s.task, s.kind) {
        (StaffTask::Held, _) => t(Text::StaffHandOfGod),
        _ if unpaid => t(Text::StaffFired),
        (StaffTask::Repair(id), _) => {
            let name = w.structure(*id).map(|x| x.name.as_str());
            named(if s.moving { Text::StaffMaintWalking } else { Text::StaffMaintFixing }, name)
        }
        (StaffTask::RepairLift(id), _) => {
            let name = w.lift(*id).map(|x| x.name.as_str());
            named(if s.moving { Text::StaffMaintWalking } else { Text::StaffMaintFixing }, name)
        }
        (StaffTask::Clean(_), _) => t(Text::StaffSanitationCleaning),
        (StaffTask::Rescue(id) | StaffTask::Aid { guest: id, .. } | StaffTask::Carry(id) | StaffTask::Heal(id), _) => {
            let name = w.guest(*id).map(|g| g.name.clone()).unwrap_or_default();
            strings.text(Text::StaffSnoMoRescue).replacen("%s", &name, 1)
        }
        (StaffTask::Mortar { stage: MortarStage::Settle(_), .. }, _) => t(Text::StaffAcgWaitingForResult),
        (StaffTask::Mortar { .. }, _) => t(Text::StaffAcgDoingJob),
        (StaffTask::Tour { .. }, _) => t(Text::StaffSnowTourDoingJob),
        (_, StaffKind::SnowTour) => t(Text::StaffSnowTourPickup),
        (StaffTask::MoveTo(_), StaffKind::Maintenance) => t(Text::StaffMaintWandering),
        (_, StaffKind::Maintenance) => t(Text::StaffMaintWaiting),
        (_, StaffKind::Sanitation) => t(Text::StaffSanitationWaiting),
        // Rescue states 8 and 9 (0x480550).
        (StaffTask::MoveTo(_) | StaffTask::Rest(_), StaffKind::Patrol) => t(Text::StaffSnoMoReturnHome),
        (_, StaffKind::Patrol) => t(Text::StaffSnoMoIdle),
        (_, StaffKind::AvalancheControl) => t(Text::StaffAcgWaiting),
        (_, StaffKind::Security) => t(Text::StaffSecurityPatrol),
        (_, StaffKind::WildlifeControl) => t(Text::StaffWildlifecontrolIdle),
        _ => t(Text::StaffThinking),
    }
}

pub(crate) fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The icon over a guest's head for a thought (`thoughts.bin`).
fn thought_icon(t: Thought) -> Group {
    use crate::game::thought::Thought::*;
    use art::thoughts::*;
    match t {
        Hungry => HUNGER,
        Cold => COLD,
        NeedToWeeWee => PEE,
        Tired => TIRED,
        Skiing => HAPPINESS,
        Wounded | WoundedWaiting | GoingToSkiLift => ALERT,
        _ => ANGER,
    }
}

/// Text of a guest thought. Categories have numbered variants, many of
/// them empty in the original; one is picked per guest.
pub(crate) fn thought_text(strings: &crate::formats::eng::Strings, thought: Thought, guest: u32) -> String {
    if let Some(t) = thought.text() {
        return strings.text(t).replace("%s", "").trim().to_string();
    }
    let variants: Vec<String> = (1..=25)
        .map(|i| Variant(thought, i))
        .filter_map(|v| {
            let t = strings.text(&v);
            (!t.is_empty() && t != v.key()).then(|| t.into_owned())
        })
        .collect();
    if variants.is_empty() {
        return String::new();
    }
    variants[guest as usize % variants.len()].clone()
}
