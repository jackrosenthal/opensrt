//! Front-end screens drawn with the original menu art. Widget positions
//! come from the original executable's screen setup code (English layout).

use egui::{Color32, Sense};

use super::art::{self, ArtFile, Group};
use super::classic::{Art, BODY, Canvas, LIST};
use super::{Action, Screen, Ui, saves_dir};
use crate::assets::Assets;
use crate::game::Game;
use crate::settings::Settings;

const MAIN: ArtFile = art::main_menu::FILE;
/// Buttons the art places a pixel off from where their pictures meet the
/// background they cover: the first three main menu buttons a pixel too
/// high, Quit a pixel down and right (past the screen's edge), and Start,
/// Cancel, Save and Resume Game on other screens a pixel too low. The
/// original draws them as placed, seams and all; here they go where they
/// fit, found by matching each picture's edge against the background.
const NUDGE_ROW: (f32, f32) = (0.0, 1.0);
const NUDGE_QUIT: (f32, f32) = (-1.0, -1.0);
const NUDGE_UP: (f32, f32) = (0.0, -1.0);
const INSTANT: ArtFile = art::instant_action::FILE;
const CHALLENGE: ArtFile = art::challenge_select::FILE;
const LOAD: ArtFile = art::load_game::FILE;
const SETTINGS: ArtFile = art::settings::FILE;
const CREDITS: ArtFile = art::credits::FILE;
const ESCAPE: ArtFile = art::escape::FILE;
const SAVE: ArtFile = art::save_game::FILE;
const LEVEL_EDIT: ArtFile = art::level_edit::FILE;

/// Starting cash for the Easy/Medium/Hard choices printed on the art.
pub const DIFFICULTY_CASH: [f32; 3] = [80_000.0, 50_000.0, 30_000.0];

/// Difficulty radio buttons on the Instant Action and Challenges screens.
const DIFFICULTY_POS: [(f32, f32); 3] = [(30.0, 358.0), (30.0, 388.0), (30.0, 416.0)];
/// Mountain preview picture.
const PREVIEW_POS: (f32, f32) = (19.0, 208.0);
/// Scroll bar column shared by both list screens.
const LIST_SCROLL_POS: (f32, f32) = (587.0, 214.0);

/// A list box: rectangle (x1, y1, x2, y2) and scroll bar position.
#[derive(Clone, Copy)]
struct ListBox {
    rect: (f32, f32, f32, f32),
    scroll: (f32, f32),
}

const INSTANT_LIST: ListBox = ListBox {
    rect: (239.0, 215.0, 579.0, 449.0),
    scroll: LIST_SCROLL_POS,
};
const CHALLENGE_LIST: ListBox = ListBox {
    rect: (240.0, 216.0, 585.0, 387.0),
    scroll: LIST_SCROLL_POS,
};
/// Challenge description text box.
const CHALLENGE_TEXT: (f32, f32, f32, f32) = (241.0, 396.0, 363.0, 56.0);
const LOAD_LIST: ListBox = ListBox {
    rect: (35.0, 220.0, 452.0, 445.0),
    scroll: (552.0, 239.0),
};
const SAVE_LIST: ListBox = ListBox {
    rect: (150.0, 220.0, 537.0, 448.0),
    scroll: (569.0, 247.0),
};
const EDITOR_LIST: ListBox = ListBox {
    rect: (235.0, 218.0, 570.0, 448.0),
    scroll: LIST_SCROLL_POS,
};
/// The Map Editor's radio buttons, by MapEditorState::mode (Flat map,
/// Height map, Level map), and its Size box.
const EDITOR_MODE_POS: [(f32, f32); 3] = [(28.0, 228.0), (28.0, 352.0), (28.0, 391.0)];
const EDITOR_SIZE: (f32, f32, f32, f32) = (113.0, 270.0, 164.0, 291.0);

/// The Map Editor screen's choices.
pub struct MapEditorState {
    /// 0 Flat map, 1 Height map, 2 Level map.
    pub mode: usize,
    pub size: String,
    pub list: ListState,
}

impl Default for MapEditorState {
    /// As the original sets the screen up each time (0x456d30): Height
    /// map, the first entry, and a size of 128.
    fn default() -> Self {
        MapEditorState {
            mode: 1,
            size: "128".into(),
            list: ListState::default(),
        }
    }
}

/// Save name field.
const SAVE_NAME: (f32, f32, f32, f32) = (150.0, 160.0, 300.0, 186.0);

// Settings screen widgets.
const OPTION_X: f32 = 349.0;
const OPTION_Y: [f32; 5] = [51.0, 126.0, 205.0, 283.0, 362.0];
// The left panel's rows where the original has them. Its Renderer and
// Resolution rows stay in the picture but do nothing here (the picture is
// to be redone; `settings_art` has an attempt).
const DETAIL_Y: f32 = 204.0;
const LEVEL_OFF_X: f32 = 22.0;
const SOUND_X: [f32; 5] = [133.0, 159.0, 185.0, 210.0, 235.0];
const MUSIC_X: [f32; 5] = [133.0, 158.0, 185.0, 210.0, 235.0];
const SOUND_Y: f32 = 282.0;
const MUSIC_Y: f32 = 361.0;
const SCROLL_Y: f32 = 440.0;
const DETAIL_X: [f32; 3] = [22.0, 133.0, 235.0];

#[derive(Default)]
pub struct ListState {
    pub selected: usize,
    pub scroll: usize,
}

/// A check or radio widget at canvas (x, y). The group's elements are
/// relative to the widget: element 1 is the rollover image and element 2
/// the selected image. Returns true when clicked.
#[allow(clippy::too_many_arguments)]
fn toggle(art: &mut Art, ui: &mut egui::Ui, assets: &Assets, cv: Canvas, rel: ArtFile, group: Group, x: f32, y: f32, on: bool) -> bool {
    let r = art
        .group_element(assets, rel, group, 0)
        .map(|(_, r)| r)
        .unwrap_or(crate::formats::ui::Rect { x1: -5, y1: -5, x2: 25, y2: 25 });
    let hit = cv.rect_xywh(x + r.x1 as f32, y + r.y1 as f32, r.width() as f32, r.height() as f32);
    let id = ui.scope_id().with((rel.0, group.0, x as i32, y as i32));
    let resp = ui.interact(hit, id, Sense::click());
    if on {
        art.widget_element(ui, assets, cv, rel, group, 2, x, y);
    } else if resp.hovered() {
        art.widget_element(ui, assets, cv, rel, group, 1, x, y);
    }
    if resp.clicked() {
        crate::audio::sfx(assets, crate::audio::Sound::UiDownclick);
    }
    resp.clicked()
}

/// A scrolling list; returns the clicked row, if any.
#[allow(clippy::too_many_arguments)]
fn list(art: &mut Art, ui: &mut egui::Ui, assets: &Assets, cv: Canvas, rel: ArtFile, lb: ListBox, rows: &[String], state: &mut ListState) -> Option<usize> {
    let (x1, y1, x2, y2) = lb.rect;
    let row_h = Art::row_height(LIST);
    let visible = (((y2 - y1 + 1.0) / row_h) as usize).max(1);
    let area = cv.rect_xywh(x1, y1, x2 - x1 + 1.0, y2 - y1 + 1.0);
    if ui.rect_contains_pointer(area) {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy > 1.0 && state.scroll > 0 {
            state.scroll -= 1;
        } else if dy < -1.0 && state.scroll + visible < rows.len() {
            state.scroll += 1;
        }
    }
    // Scroll bar: up arrow, track and down arrow down the right side.
    let (sx, sy) = lb.scroll;
    let down_y = y2 - 21.0;
    art.draw(ui, assets, cv, rel, art::list_box::VERT_SCROLLBAR, 0, sx, sy + 21.0);
    if art.button(ui, assets, cv, rel, art::list_box::SCROLLUP, (sx, sy)) && state.scroll > 0 {
        state.scroll -= 1;
    }
    if art.button(ui, assets, cv, rel, art::list_box::SCROLLDOWN, (sx, down_y)) && state.scroll + visible < rows.len() {
        state.scroll += 1;
    }
    // The challenge screen spells its row highlight with a capital.
    let cursor = if art.element_count(assets, rel, art::instant_action::CURSOR) > 0 {
        art::instant_action::CURSOR
    } else {
        art::challenge_select::CURSOR
    };
    let mut clicked = None;
    for (i, row) in rows.iter().enumerate().skip(state.scroll).take(visible) {
        let ry = y1 + (i - state.scroll) as f32 * row_h;
        let r = cv.rect_xywh(x1, ry, x2 - x1 + 1.0, row_h);
        let resp = ui.interact(r, ui.scope_id().with((rel.0, "row", i)), Sense::click());
        if i == state.selected {
            art.draw(ui, assets, cv, rel, cursor, 0, x1, ry);
        }
        art.text(ui, cv, LIST, x1 + 4.0, ry + 2.0, row);
        if resp.clicked() {
            state.selected = i;
            clicked = Some(i);
            crate::audio::sfx(assets, crate::audio::Sound::UiDownclick);
        }
    }
    clicked
}

pub fn level_name(rel: &str) -> String {
    std::path::Path::new(rel)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Index of the preview picture for a level in the `mountain` group.
fn mountain_thumbnail(level: &str) -> usize {
    let n = level_name(level).to_ascii_lowercase();
    [
        ("aspen", 0),
        ("whistler", 2),
        ("vail", 3),
        ("marmot", 4),
        ("jasper", 4),
        ("moritz", 5),
        ("kitz", 6),
    ]
    .iter()
    .find(|(k, _)| n.contains(k))
    .map(|(_, i)| *i)
    .unwrap_or(1)
}

impl Ui {
    pub(super) fn classic_menus(&mut self, root: &mut egui::Ui, assets: &Assets, actions: &mut Vec<Action>) -> bool {
        let full = root.ctx().content_rect();
        root.painter().rect_filled(full, 0.0, Color32::BLACK);
        let cv = Canvas::fit(full, root.ctx().pixels_per_point());
        let mut art = std::mem::take(&mut self.art);
        let have = art.screen(assets, MAIN).is_some();
        if have {
            match self.screen {
                Screen::MainMenu => self.main_screen(&mut art, root, assets, cv, actions),
                Screen::NewGame => self.instant_screen(&mut art, root, assets, cv, actions, false),
                Screen::Challenges => self.instant_screen(&mut art, root, assets, cv, actions, true),
                Screen::LoadGame => self.load_screen(&mut art, root, assets, cv, actions),
                Screen::Settings => self.settings_screen(&mut art, root, assets, cv),
                Screen::Credits => self.credits_screen(&mut art, root, assets, cv),
                Screen::MapEditor => self.map_editor_screen(&mut art, root, assets, cv, actions),
                Screen::InGame => {}
            }
        }
        self.art = art;
        have
    }

    fn main_screen(&mut self, art: &mut Art, ui: &mut egui::Ui, assets: &Assets, cv: Canvas, actions: &mut Vec<Action>) {
        art.background(ui, assets, cv, MAIN, art::main_menu::MAIN);
        if art.button(ui, assets, cv, MAIN, art::main_menu::MAIN_INSTANT_ACTION_BUTTON, NUDGE_ROW) {
            self.screen = Screen::NewGame;
            // The original builds the screen afresh each time: Medium
            // (0x4557f0 selects radio 5) and the first level.
            self.difficulty = 1;
            self.level_list = ListState::default();
        }
        if art.button(ui, assets, cv, MAIN, art::main_menu::MAIN_CHALLENGES_BUTTON, NUDGE_ROW) {
            self.screen = Screen::Challenges;
        }
        if art.button(ui, assets, cv, MAIN, art::main_menu::MAIN_LOAD_GAME, NUDGE_ROW) {
            self.screen = Screen::LoadGame;
        }
        if art.button(ui, assets, cv, MAIN, art::main_menu::MAIN_SETTINGS, (0.0, 0.0)) {
            self.screen = Screen::Settings;
        }
        if art.button(ui, assets, cv, MAIN, art::main_menu::MAIN_MAP_EDITOR, (0.0, 0.0)) {
            self.screen = Screen::MapEditor;
            self.map_editor = MapEditorState::default();
        }
        if art.button(ui, assets, cv, MAIN, art::main_menu::MAIN_CREDITS, (0.0, 0.0)) {
            self.screen = Screen::Credits;
            self.credits_t = 0.0;
        }
        if art.button(ui, assets, cv, MAIN, art::main_menu::MAIN_QUIT, NUDGE_QUIT) {
            actions.push(Action::Exit);
        }
    }

    fn instant_screen(&mut self, art: &mut Art, ui: &mut egui::Ui, assets: &Assets, cv: Canvas, actions: &mut Vec<Action>, challenges: bool) {
        let rel = if challenges { CHALLENGE } else { INSTANT };
        art.background(ui, assets, cv, rel, art::list_screen::BACKGROUND);
        // Entries: (label, level, challenge number, description)
        let entries: Vec<(String, String, Option<usize>, String)> = if challenges {
            let strings = super::ui_strings(assets);
            super::challenges(assets, &strings)
                .into_iter()
                .map(|c| (c.title, c.level, Some(c.index), c.description))
                .collect()
        } else {
            assets
                .list("data/levels", "lev")
                .into_iter()
                .map(|(rel, _)| (super::title_case(&level_name(&rel)), rel, None, String::new()))
                .collect()
        };
        let rows: Vec<String> = entries.iter().map(|e| e.0.clone()).collect();
        let mut st = std::mem::take(if challenges { &mut self.challenge_list } else { &mut self.level_list });
        if st.selected >= rows.len() {
            st.selected = 0;
        }
        if let Some(e) = entries.get(st.selected) {
            let idx = mountain_thumbnail(&e.1);
            art.draw(ui, assets, cv, rel, art::list_screen::MOUNTAIN, idx, PREVIEW_POS.0, PREVIEW_POS.1);
        }
        for (i, (x, y)) in DIFFICULTY_POS.into_iter().enumerate() {
            if toggle(art, ui, assets, cv, rel, art::list_screen::RADIO_BUTTON, x, y, self.difficulty == i) {
                self.difficulty = i;
            }
        }
        let lb = if challenges { CHALLENGE_LIST } else { INSTANT_LIST };
        list(art, ui, assets, cv, rel, lb, &rows, &mut st);
        let sel = st.selected;
        if challenges {
            self.challenge_list = st;
            if let Some(e) = entries.get(sel) {
                art.text_box(ui, cv, BODY, CHALLENGE_TEXT, &e.3);
            }
        } else {
            self.level_list = st;
        }
        let (start, cancel) = if challenges {
            (art::challenge_select::START, art::challenge_select::CANCEL)
        } else {
            (art::instant_action::INSTANT_START, art::instant_action::INSTANT_CANCEL)
        };
        if art.button(ui, assets, cv, rel, start, NUDGE_UP) {
            if let Some(e) = entries.get(sel) {
                self.start_cash = DIFFICULTY_CASH[self.difficulty.min(2)];
                let tag = match e.2 {
                    Some(n) => format!("{}#challenge{n}", e.1),
                    None => e.1.clone(),
                };
                actions.push(Action::StartLevel(tag));
            }
        }
        if art.button(ui, assets, cv, rel, cancel, NUDGE_UP) {
            self.screen = Screen::MainMenu;
        }
    }

    /// The Map Editor screen (the original's CLevelEdit, 0x456d30 and
    /// 0x4572a0): a flat map of the size typed in, a height map picture,
    /// or a level to edit.
    fn map_editor_screen(&mut self, art: &mut Art, ui: &mut egui::Ui, assets: &Assets, cv: Canvas, actions: &mut Vec<Action>) {
        art.background(ui, assets, cv, LEVEL_EDIT, art::level_edit::BACKGROUND);
        let st = &mut self.map_editor;
        for (i, (x, y)) in EDITOR_MODE_POS.into_iter().enumerate() {
            if toggle(art, ui, assets, cv, LEVEL_EDIT, art::level_edit::RADIO_BUTTON, x, y, st.mode == i) && st.mode != i {
                st.mode = i;
                st.list = ListState::default();
            }
        }
        // The list shows file names as they are on disk, extension and all.
        let entries: Vec<(String, String)> = match st.mode {
            1 => assets.list("data/height_maps", "bmp"),
            2 => assets.list("data/levels", "lev"),
            _ => Vec::new(),
        }
        .into_iter()
        .map(|(rel, path)| {
            let shown = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| rel.clone());
            (shown, rel)
        })
        .collect();
        let rows: Vec<String> = entries.iter().map(|e| e.0.clone()).collect();
        if st.list.selected >= rows.len() {
            st.list.selected = 0;
        }
        list(art, ui, assets, cv, LEVEL_EDIT, EDITOR_LIST, &rows, &mut st.list);
        let (x1, y1, x2, y2) = EDITOR_SIZE;
        let font = egui::FontId::new(LIST.size * cv.scale, egui::FontFamily::Name(super::fonts::ARIAL_BOLD.into()));
        let edit = egui::TextEdit::singleline(&mut st.size)
            .font(font)
            .text_color(LIST.color)
            .frame(egui::Frame::NONE)
            .char_limit(3);
        ui.put(cv.rect_xywh(x1, y1, x2 - x1, y2 - y1), edit);
        st.size.retain(|c| c.is_ascii_digit());
        if art.button(ui, assets, cv, LEVEL_EDIT, art::level_edit::START_BUTTON, NUDGE_UP) {
            use crate::game::editor::{MapSource, flat_size};
            let picked = entries.get(st.list.selected).map(|e| e.1.clone());
            let source = match (st.mode, picked) {
                (0, _) => Some(MapSource::Flat(flat_size(&st.size))),
                (1, Some(rel)) => Some(MapSource::HeightMap(rel)),
                (2, Some(rel)) => Some(MapSource::Level(rel)),
                _ => None,
            };
            if let Some(source) = source {
                actions.push(Action::StartEditor(source));
            }
        }
        if art.button(ui, assets, cv, LEVEL_EDIT, art::level_edit::CANCEL_BUTTON, NUDGE_UP) {
            self.screen = Screen::MainMenu;
        }
    }

    fn load_screen(&mut self, art: &mut Art, ui: &mut egui::Ui, assets: &Assets, cv: Canvas, actions: &mut Vec<Action>) {
        art.background(ui, assets, cv, LOAD, art::load_game::BACKGROUND);
        let mut saves: Vec<std::path::PathBuf> = std::fs::read_dir(saves_dir())
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect())
            .unwrap_or_default();
        saves.sort();
        let rows: Vec<String> = saves
            .iter()
            .map(|p| p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
            .collect();
        let mut st = std::mem::take(&mut self.save_list);
        if st.selected >= rows.len() {
            st.selected = 0;
        }
        let picked = list(art, ui, assets, cv, LOAD, LOAD_LIST, &rows, &mut st).is_some();
        let dbl = picked && ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
        let sel = st.selected;
        self.save_list = st;
        if (art.button(ui, assets, cv, LOAD, art::load_game::LOAD_BUTTON, (0.0, 0.0)) || dbl) && sel < saves.len() {
            actions.push(Action::LoadSave(saves[sel].clone()));
        }
        if art.button(ui, assets, cv, LOAD, art::load_game::CANCEL_BUTTON, (0.0, 0.0)) {
            self.screen = Screen::MainMenu;
        }
    }

    fn settings_screen(&mut self, art: &mut Art, ui: &mut egui::Ui, assets: &Assets, cv: Canvas) {
        art.background(ui, assets, cv, SETTINGS, art::settings::BACKGROUND);
        let s: &mut Settings = &mut self.settings;
        let mut changed = false;
        let radio = |art: &mut Art, ui: &mut egui::Ui, x: f32, y: f32, on: bool| toggle(art, ui, assets, cv, SETTINGS, art::settings::RADIO_BUTTON, x, y, on);
        let y = DETAIL_Y;
        for (i, x) in DETAIL_X.into_iter().enumerate() {
            if radio(art, ui, x, y, s.detail == i) {
                s.detail = i;
                changed = true;
            }
        }
        for (level, xs, y) in [(&mut s.sound, SOUND_X, SOUND_Y), (&mut s.music, MUSIC_X, MUSIC_Y)] {
            if radio(art, ui, LEVEL_OFF_X, y, *level == 0) {
                *level = 0;
                changed = true;
            }
            for (i, x) in xs.into_iter().enumerate() {
                if radio(art, ui, x, y, *level == i + 1) {
                    *level = i + 1;
                    changed = true;
                }
            }
        }
        for (i, x) in MUSIC_X.into_iter().enumerate() {
            if radio(art, ui, x, SCROLL_Y, s.scroll_rate == i) {
                s.scroll_rate = i;
                changed = true;
            }
        }
        let opts: [&mut bool; 5] = [
            &mut s.all_buildings,
            &mut s.fast_maintenance,
            &mut s.reduce_garbage,
            &mut s.evil_sasquatches,
            &mut s.no_nightfall,
        ];
        for (opt, y) in opts.into_iter().zip(OPTION_Y) {
            if toggle(art, ui, assets, cv, SETTINGS, art::settings::CHECK_BUTTON, OPTION_X, y, *opt) {
                *opt = !*opt;
                changed = true;
            }
        }
        if changed {
            s.apply_audio();
            s.save();
        }
        if art.button(ui, assets, cv, SETTINGS, art::settings::OK_BUTTON, (-1.0, -1.0)) {
            self.screen = Screen::MainMenu;
        }
    }

    /// The in-game escape menu drawn over the game (the floppy log).
    pub(super) fn escape_screen(&mut self, root: &mut egui::Ui, assets: &Assets, g: &mut Game, actions: &mut Vec<Action>) {
        let full = root.ctx().content_rect();
        self.hud.blocks.push(full);
        root.painter().rect_filled(full, 0.0, Color32::BLACK);
        let cv = Canvas::fit(full, root.ctx().pixels_per_point());
        let mut art = std::mem::take(&mut self.art);
        art.sounds = super::classic::UiSounds::Menu;
        if self.save_open {
            self.save_screen(&mut art, root, assets, cv, g);
        } else {
            art.background(root, assets, cv, ESCAPE, art::escape::BACKGROUND);
            if art.button(root, assets, cv, ESCAPE, art::escape::SAVE_GAME, (0.0, 0.0)) {
                self.save_open = true;
                if self.save_name.is_empty() {
                    self.save_name = g.world.level_name.clone();
                }
            }
            if art.button(root, assets, cv, ESCAPE, art::escape::ABORT_GAME, (0.0, 0.0)) {
                self.show_menu = false;
                actions.push(Action::QuitToMenu);
            }
            if art.button(root, assets, cv, ESCAPE, art::escape::RESUME_GAME, NUDGE_UP) {
                self.close_escape(g);
            }
        }
        self.art = art;
    }

    pub(super) fn close_escape(&mut self, g: &mut Game) {
        self.show_menu = false;
        self.save_open = false;
        g.paused = self.paused_before_menu;
    }

    fn save_screen(&mut self, art: &mut Art, ui: &mut egui::Ui, assets: &Assets, cv: Canvas, g: &mut Game) {
        art.background(ui, assets, cv, SAVE, art::save_game::BACKGROUND);
        // In the map editor the screen lists and saves levels (0x468210).
        let editor = g.world.editor;
        let mut saves: Vec<std::path::PathBuf> = if editor {
            assets.list("data/levels", "lev").into_iter().map(|(_, p)| p).collect()
        } else {
            std::fs::read_dir(saves_dir())
                .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect())
                .unwrap_or_default()
        };
        saves.sort();
        let rows: Vec<String> = saves
            .iter()
            .map(|p| p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
            .collect();
        let mut st = std::mem::take(&mut self.save_list);
        if st.selected >= rows.len() {
            st.selected = 0;
        }
        if let Some(i) = list(art, ui, assets, cv, SAVE, SAVE_LIST, &rows, &mut st) {
            self.save_name = rows[i].clone();
        }
        let sel = st.selected;
        self.save_list = st;
        // Name field: plain Arial text in the art's white box.
        let (x1, y1, x2, y2) = SAVE_NAME;
        let rect = cv.rect_xywh(x1 + 4.0, y1, x2 - x1 - 8.0, y2 - y1);
        let font = egui::FontId::new(LIST.size * cv.scale, egui::FontFamily::Name(super::fonts::ARIAL_BOLD.into()));
        let edit = egui::TextEdit::singleline(&mut self.save_name)
            .font(font)
            .text_color(LIST.color)
            .frame(egui::Frame::NONE)
            .char_limit(40);
        ui.put(rect, edit);
        if art.button(ui, assets, cv, SAVE, art::save_game::SAVE_BUTTON, NUDGE_UP) {
            let name: String = self
                .save_name
                .trim()
                .chars()
                .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
                .collect();
            if !name.is_empty() {
                let saved = if editor {
                    crate::game::editor::save_level(g.world.level(), &name).map(|_| g.world.level_name = name)
                } else {
                    g.save(&saves_dir().join(format!("{name}.json")))
                };
                match saved {
                    Ok(()) => self.close_escape(g),
                    Err(e) => self.error = Some(format!("Save failed: {e:#}")),
                }
            }
        }
        if art.button(ui, assets, cv, SAVE, art::save_game::CANCEL_BUTTON, (0.0, 0.0)) {
            self.save_open = false;
        }
        if art.button(ui, assets, cv, SAVE, art::save_game::DELETE_BUTTON, (0.0, 0.0)) {
            // The shipped levels stay.
            let mine = |p: &std::path::Path| !editor || p.starts_with(crate::game::editor::user_levels_dir());
            if let Some(p) = saves.get(sel).filter(|p| mine(p)) {
                if let Err(e) = std::fs::remove_file(p) {
                    self.error = Some(format!("Delete failed: {e}"));
                }
            }
        }
    }

    fn credits_screen(&mut self, art: &mut Art, ui: &mut egui::Ui, assets: &Assets, cv: Canvas) {
        art.background(ui, assets, cv, CREDITS, art::credits::BACKGROUND);
        // The credit roll scrolls up through the window.
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        self.credits_t += dt;
        let roll = art.group_element(assets, CREDITS, art::credits::CREDITS_LIST, 0).and_then(|(s, _)| s);
        let height = roll
            .and_then(|i| art.screen(assets, CREDITS).and_then(|s| s.sprites.get(i)))
            .and_then(|sp| sp.image.as_ref().map(|im| im.height as f32))
            .unwrap_or(0.0);
        if let Some(sprite) = roll {
            let mut y = 480.0 - self.credits_t * 30.0;
            if y < -height {
                self.credits_t = 0.0;
                y = 480.0;
            }
            let clip = cv.rect_xywh(0.0, 80.0, 640.0, 340.0);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(clip));
            child.set_clip_rect(clip);
            art.sprite_at(&child, assets, cv, CREDITS, sprite, 0.0, y);
        }
        if art.button(ui, assets, cv, CREDITS, art::credits::OK_BUTTON, (0.0, 0.0)) {
            self.screen = Screen::MainMenu;
        }
        ui.ctx().request_repaint();
    }
}
