//! Widgets many windows and screens share (`hud/widgets/*.svg`): close
//! buttons, scroll arrows, tracks and nubs (black, and the finances
//! window's blue ones), title trays, list panels, tabs, round buttons,
//! list highlights and check buttons. The original repeats each as the
//! same sprite in every file that has it; each is drawn once here and
//! mapped for every file and group that uses it.
//!
//! Three of them are also frames for per-screen pictures: `ROUND` (the
//! round buttons), `ICON_TAB` (the windows' picture tabs) and `TAB` (the
//! pickers' picture tabs). A picture is its own drawing on the frame's
//! grid, made with `svg!("…" on widgets::ROUND)`, and is drawn over the
//! frame with the same state shown, as `FOCUS` is.

use super::{Drawing, Svg, button, shown, still};
use crate::ui::art::{self, ArtFile, Group};

/// The red cross that closes a window (SPU_Close).
pub const CLOSE: Svg = svg!("widgets/close");
/// The horizontal scroller's arrows (SPU_arrowL, SPU_arrowR).
pub const ARROW_LEFT: Svg = svg!("widgets/arrow_left");
pub const ARROW_RIGHT: Svg = svg!("widgets/arrow_right");
/// A window list's scroll arrows (ScrollUp, ScrollDown).
pub const SCROLL_UP: Svg = svg!("widgets/scroll_up");
pub const SCROLL_DOWN: Svg = svg!("widgets/scroll_down");
/// Small arrows of spinners and short lists (miniscrollUp, miniscrollDown).
pub const SPIN_UP: Svg = svg!("widgets/spin_up");
pub const SPIN_DOWN: Svg = svg!("widgets/spin_down");
/// A front-end list's scroll arrows (Scroll_Up, Scroll_Down).
pub const LIST_UP: Svg = svg!("widgets/list_up");
pub const LIST_DOWN: Svg = svg!("widgets/list_down");
/// The black disc that slides along a track (Scroll_Tab).
pub const NUB: Svg = svg!("widgets/nub");
/// Scroll tracks: a window list's (Tray_Vert, Tray_Horiz), a short one
/// (Tray_Vert_short), a stat window's (vert_text_box, horiz_text_box)
/// and a front-end list's (Scroll_Bar_Vertical).
pub const TRACK_VERT: Svg = svg!("widgets/track_vert");
pub const TRACK_VERT_SHORT: Svg = svg!("widgets/track_vert_short");
pub const TRACK_HORIZ: Svg = svg!("widgets/track_horiz");
pub const TEXT_TRACK_VERT: Svg = svg!("widgets/text_track_vert");
pub const TEXT_TRACK_HORIZ: Svg = svg!("widgets/text_track_horiz");
pub const LIST_TRACK: Svg = svg!("widgets/list_track");
/// The top of a window, under its title and tabs (Popup_tray_bkgrnd).
pub const TITLE_TRAY: Svg = svg!("widgets/title_tray");
/// A window's plain list panel (Text_list_bkgrnd, music_bkgrnd).
pub const LIST_PANEL: Svg = svg!("widgets/list_panel");
/// The blue picker tab (Tab_Normal, Tab_Depress, tab_depress_empty):
/// states `normal`, `depress` (chosen) and `dim`.
pub const TAB: Svg = svg!("widgets/tab");
/// The wooden tab (Tab_Plain_Normal, Tab_Plain_Depress): states `normal`
/// and `depress` (chosen).
pub const TAB_PLAIN: Svg = svg!("widgets/tab_plain");
/// Frame of the windows' 45x34 picture tabs (Tab_*_Normal, Tab_*_Depress):
/// states `normal` (in front) and `depress` (behind).
pub const ICON_TAB: Svg = svg!("widgets/icon_tab");
/// Frame of the 35x36 round buttons: states `normal`, `rollover` and
/// `depress`.
pub const ROUND: Svg = svg!("widgets/round");
pub const FOCUS: Svg = svg!("widgets/focus" on ROUND);
pub const RENAME: Svg = svg!("widgets/rename" on ROUND);
pub const PATSEYE: Svg = svg!("widgets/patseye" on ROUND);
pub const SELL: Svg = svg!("widgets/sell" on ROUND);
/// The chosen row of a front-end list (highlightBAR).
pub const HIGHLIGHT: Svg = svg!("widgets/highlight");
/// The finances window's blue scroller (Scroll_Up, Scroll_Down,
/// Scroll_Left, Scroll_Right, Scroll_Bar_Vertical, Scroll_Bar and its
/// Scroll_Tab); the object picker has its arrows too.
pub const BLUE_UP: Svg = svg!("widgets/blue_up");
pub const BLUE_DOWN: Svg = svg!("widgets/blue_down");
pub const BLUE_LEFT: Svg = svg!("widgets/blue_left");
pub const BLUE_RIGHT: Svg = svg!("widgets/blue_right");
pub const BLUE_TRACK_VERT: Svg = svg!("widgets/blue_track_vert");
pub const BLUE_TRACK_HORIZ: Svg = svg!("widgets/blue_track_horiz");
pub const BLUE_NUB: Svg = svg!("widgets/blue_nub");
/// Radio and check buttons (Radial_on, RADIAL_rollover): states `off`,
/// `rollover` and `on`.
pub const CHECK: Svg = svg!("widgets/check");
/// The small OK pill (the credits screen's OkButton; Bob's Ok is the
/// same art).
pub const OK: Svg = svg!("widgets/ok");

/// Files whose `close` group is the red cross.
const CLOSE_FILES: &[ArtFile] = &[
    art::rename::FILE,
    art::fluff_menu::FILE,
    art::guests_stats::FILE,
    art::lifts_toolbar::FILE,
    art::guest_window::FILE,
    art::lift_window::FILE,
    art::runmarker_window::FILE,
    art::structure_window::FILE,
    art::finances_stats::FILE,
    art::resort_stats::FILE,
    art::staff_window::FILE,
    art::attractions::FILE,
    art::structures_toolbar::FILE,
];

/// Files whose `scrollleft` and `scrollright` groups are the triangle
/// arrows.
const ARROW_FILES: &[ArtFile] = &[
    art::fluff_menu::FILE,
    art::guests_stats::FILE,
    art::lifts_toolbar::FILE,
    art::guest_window::FILE,
    art::lift_window::FILE,
    art::structure_window::FILE,
    art::resort_stats::FILE,
    art::staff_window::FILE,
    art::attractions::FILE,
    art::structures_toolbar::FILE,
];

/// Files whose `scrollup` and `scrolldown` groups are a window list's
/// arrows.
const SCROLL_FILES: &[ArtFile] = &[
    art::guests_stats::FILE,
    art::guest_window::FILE,
    art::lift_window::FILE,
    art::structure_window::FILE,
    art::resort_stats::FILE,
    art::staff_window::FILE,
    art::attractions::FILE,
];

/// Files whose `scrollup` and `scrolldown` groups are the blue arrows.
const BLUE_FILES: &[ArtFile] = &[art::finances_stats::FILE, art::fluff_menu::FILE];

/// Files with price spinners (`spinup`, `spindown`).
const SPIN_FILES: &[ArtFile] = &[art::lift_window::FILE, art::structure_window::FILE, art::resort_stats::FILE];

/// Files whose scroll nubs (`vert_scrollnub`, `horiz_scrollnub`) are the
/// black disc.
const NUB_FILES: &[ArtFile] = &[
    art::challenge_select::FILE,
    art::instant_action::FILE,
    art::level_edit::FILE,
    art::load_game::FILE,
    art::save_game::FILE,
    art::guests_stats::FILE,
    art::resort_stats::FILE,
    art::attractions::FILE,
    art::structures_toolbar::FILE,
    art::guest_window::FILE,
    art::lift_window::FILE,
    art::structure_window::FILE,
    art::staff_window::FILE,
];

/// Files whose tracks are a window list's (Tray_Vert, Tray_Horiz).
const TRACK_FILES: &[ArtFile] = &[art::guest_window::FILE, art::lift_window::FILE, art::structure_window::FILE, art::staff_window::FILE];

/// Files whose tracks are a stat window's (vert_text_box, horiz_text_box;
/// the object picker has only the horizontal one).
const TEXT_TRACK_FILES: &[ArtFile] = &[art::guests_stats::FILE, art::resort_stats::FILE, art::attractions::FILE, art::fluff_menu::FILE];

/// Files whose `title_bar` is the title tray.
const TITLE_TRAY_FILES: &[ArtFile] = &[
    art::guests_stats::FILE,
    art::guest_window::FILE,
    art::lift_window::FILE,
    art::runmarker_window::FILE,
    art::structure_window::FILE,
    art::resort_stats::FILE,
    art::staff_window::FILE,
    art::attractions::FILE,
];

/// Files whose plain `tab` group is the blue tab: the chosen tab, then
/// the others.
const TAB_FILES: &[ArtFile] = &[
    art::fluff_menu::FILE,
    art::guests_stats::FILE,
    art::lifts_toolbar::FILE,
    art::finances_stats::FILE,
    art::resort_stats::FILE,
    art::structures_toolbar::FILE,
];

/// The widget that stands in for element `index` of a group, for every
/// file that has that widget there.
pub(super) fn element(file: ArtFile, group: Group, index: usize) -> Option<Drawing> {
    use art::object_window::{CLOSE as CLOSE_GROUP, FOCUS as FOCUS_GROUP, PATSEYE as PATSEYE_GROUP, RENAME as RENAME_GROUP, TITLE_BAR};
    use art::stat_window::{BACKGROUND, HORIZ_SCROLLBAR, HORIZ_SCROLLNUB, SCROLLDOWN, SCROLLLEFT, SCROLLRIGHT, SCROLLUP, VERT_SCROLLBAR, VERT_SCROLLNUB};
    use art::structure_window::{BACKGROUND_LIFTS, BACKGROUND_NOCOST, SPINDOWN, SPINUP};
    use art::guest_window::{FIRE, TAB as TAB_GROUP};
    let has = |files: &[ArtFile]| files.contains(&file);
    let objects = has(art::object_window::FILES) && file != art::staff_window::FILE;
    match group {
        CLOSE_GROUP if has(CLOSE_FILES) => button(CLOSE, index),
        art::gen_map::CLOSE_BUTTON if file == art::gen_map::FILE => button(CLOSE, index),
        SCROLLLEFT if has(ARROW_FILES) => button(ARROW_LEFT, index),
        SCROLLRIGHT if has(ARROW_FILES) => button(ARROW_RIGHT, index),
        // The attractions window shows its arrows lit even at rest.
        SCROLLUP if file == art::attractions::FILE => button(SCROLL_UP, index.max(1)),
        SCROLLDOWN if file == art::attractions::FILE => button(SCROLL_DOWN, index.max(1)),
        SCROLLUP if has(SCROLL_FILES) => button(SCROLL_UP, index),
        SCROLLDOWN if has(SCROLL_FILES) => button(SCROLL_DOWN, index),
        SPINUP if has(SPIN_FILES) => button(SPIN_UP, index),
        SPINDOWN if has(SPIN_FILES) => button(SPIN_DOWN, index),
        // The building picker's description scrolls with the small arrows.
        SCROLLUP if file == art::structures_toolbar::FILE => button(SPIN_UP, index),
        SCROLLDOWN if file == art::structures_toolbar::FILE => button(SPIN_DOWN, index),
        SCROLLUP if has(art::list_box::FILES) => button(LIST_UP, index),
        SCROLLDOWN if has(art::list_box::FILES) => button(LIST_DOWN, index),
        SCROLLUP if has(BLUE_FILES) => button(BLUE_UP, index),
        SCROLLDOWN if has(BLUE_FILES) => button(BLUE_DOWN, index),
        SCROLLLEFT if file == art::finances_stats::FILE => button(BLUE_LEFT, index),
        SCROLLRIGHT if file == art::finances_stats::FILE => button(BLUE_RIGHT, index),
        VERT_SCROLLBAR if file == art::finances_stats::FILE => still(BLUE_TRACK_VERT),
        HORIZ_SCROLLBAR if file == art::finances_stats::FILE => still(BLUE_TRACK_HORIZ),
        VERT_SCROLLNUB | HORIZ_SCROLLNUB if file == art::finances_stats::FILE => still(BLUE_NUB),
        VERT_SCROLLNUB | HORIZ_SCROLLNUB if has(NUB_FILES) => still(NUB),
        VERT_SCROLLBAR if has(TRACK_FILES) => still(TRACK_VERT),
        HORIZ_SCROLLBAR if has(TRACK_FILES) => still(TRACK_HORIZ),
        VERT_SCROLLBAR if file == art::structures_toolbar::FILE => still(TRACK_VERT_SHORT),
        VERT_SCROLLBAR if has(TEXT_TRACK_FILES) && file != art::fluff_menu::FILE => still(TEXT_TRACK_VERT),
        HORIZ_SCROLLBAR if has(TEXT_TRACK_FILES) => still(TEXT_TRACK_HORIZ),
        VERT_SCROLLBAR if has(art::list_box::FILES) => still(LIST_TRACK),
        TITLE_BAR if has(TITLE_TRAY_FILES) => still(TITLE_TRAY),
        BACKGROUND | BACKGROUND_LIFTS | BACKGROUND_NOCOST if list_panel(file, group, index) => still(LIST_PANEL),
        TAB_GROUP if file == art::resort_stats::FILE && index == 0 => shown(TAB, "dim"),
        TAB_GROUP if has(TAB_FILES) => shown(TAB, if index == 0 { "depress" } else { "normal" }),
        TAB_GROUP if file == art::guest_window::FILE => shown(TAB_PLAIN, if index == 2 { "depress" } else { "normal" }),
        FOCUS_GROUP | FIRE if objects => button(FOCUS, index),
        art::resort_stats::GOTO if file == art::resort_stats::FILE => button(FOCUS, index),
        RENAME_GROUP if objects || file == art::resort_stats::FILE => button(RENAME, index),
        PATSEYE_GROUP if objects => button(PATSEYE, index),
        art::facility_window::SELL if has(art::facility_window::FILES) => button(SELL, index),
        art::instant_action::CURSOR | art::challenge_select::CURSOR if has(art::list_box::FILES) => still(HIGHLIGHT),
        art::list_screen::RADIO_BUTTON if has(art::list_screen::FILES) || file == art::level_edit::FILE || file == art::settings::FILE => {
            shown(CHECK, ["off", "rollover", "on"][index.min(2)])
        }
        art::settings::CHECK_BUTTON if file == art::settings::FILE => shown(CHECK, if index >= 2 { "on" } else { "off" }),
        _ => None,
    }
}

/// Whether element `index` of a window's panel group is the plain list
/// panel: the guest, lift and structure windows' list tabs, the resort
/// window's history tab, every staff window panel and the attractions
/// window's only one.
fn list_panel(file: ArtFile, group: Group, index: usize) -> bool {
    match file {
        art::guest_window::FILE | art::lift_window::FILE => index == 2,
        art::structure_window::FILE if group == art::structure_window::BACKGROUND_LIFTS => index == 2,
        // Its music tab's panel is the same art.
        art::structure_window::FILE => (2..=4).contains(&index),
        art::resort_stats::FILE => index == 4,
        art::staff_window::FILE | art::attractions::FILE => true,
        _ => false,
    }
}
