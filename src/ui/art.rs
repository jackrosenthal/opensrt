//! The original's UI art files and the groups in them, named once.
//!
//! Generated from the game's files: each module is one `.bin` screen
//! file with its path and a constant for every group in it. The family
//! modules list the groups every file of a kind has, for code that
//! draws whichever of them is showing.

/// A UI screen file, by game-relative path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArtFile(pub &'static str);

/// A named group of elements in a UI screen file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Group(pub &'static str);

pub mod escape {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/EscapeMenu/escape.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const ABORT_GAME: Group = Group("AbortGame");
    pub const RESUME_GAME: Group = Group("ResumeGame");
    pub const SAVE_GAME: Group = Group("SaveGame");
}

pub mod fog {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/bob/fog.bin");
    pub const BACKGROUND: Group = Group("Background");
    pub const FOG100: Group = Group("Fog100");
    pub const FOG250: Group = Group("Fog250");
    pub const FOG500: Group = Group("Fog500");
    pub const NO_FOG: Group = Group("NoFog");
}

pub mod bob {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/bob/bob.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const OK: Group = Group("Ok");
    pub const BOB_BUTTON: Group = Group("BobButton");
    pub const BOB_BUTTON_ROLLOVER: Group = Group("BobButtonRollover");
    pub const BOB_BUTTON_CLICKED: Group = Group("BobButtonClicked");
    pub const BOB_BUTTON_NORMAL: Group = Group("BobButtonNormal");
}

pub mod challenge_select {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/challengeselect/ChallengeSelect.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const CURSOR: Group = Group("Cursor");
    pub const RADIO_BUTTON: Group = Group("RadioButton");
    pub const CANCEL: Group = Group("Cancel");
    pub const START: Group = Group("Start");
    pub const MOUNTAIN: Group = Group("mountain");
}

pub mod construction_ext {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/construction_extension/construction_extension.bin");
    pub const LOG_BUTTON: Group = Group("log_button");
    pub const ENDCAP: Group = Group("endcap");
    pub const LOG_STRUCTURES: Group = Group("log_structures");
    pub const STRUCTURES_OVERLAY: Group = Group("structures_overlay");
    pub const LOG_LIFTS: Group = Group("log_lifts");
    pub const LOG_OBJECTS: Group = Group("log_objects");
    pub const LIFTS_OVERLAY: Group = Group("lifts_overlay");
    pub const OBJECTS_OVERLAY: Group = Group("objects_overlay");
}

pub mod rename {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/element_rename/element_rename.bin");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const CLOSE: Group = Group("close");
    pub const BACKGROUND: Group = Group("background");
    pub const OK: Group = Group("OK");
    pub const CANCEL: Group = Group("cancel");
}

pub mod build_sub {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/extending_submenus/build_extension_sub/build_extension_sub.bin");
    pub const ENDCAP: Group = Group("endcap");
    pub const TRAY_BUTTON: Group = Group("tray_button");
    pub const TRAY_LODGING: Group = Group("tray_lodging");
    pub const TRAY_FOOD: Group = Group("tray_food");
    pub const TRAY_UTILITIES: Group = Group("tray_utilities");
    pub const TRAY_MAINTENANCE: Group = Group("tray_maintenance");
    pub const TRAY_RENTALS: Group = Group("tray_rentals");
    pub const TRAY_COMFORT: Group = Group("tray_comfort");
    pub const TRAY_ENTERTAINMENT: Group = Group("tray_entertainment");
    pub const TRAY_SERVICES: Group = Group("tray_services");
}

pub mod fluff_sub {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/extending_submenus/fluff_extension_sub/fluff_extension_sub.bin");
    pub const TRAY_BUTTON: Group = Group("tray_button");
    pub const ENDCAP: Group = Group("endcap");
    pub const TRAY_OBJECTS: Group = Group("tray_objects");
    pub const TRAY_PATHS: Group = Group("tray_paths");
    pub const TRAY_LANDSCAPE: Group = Group("tray_landscape");
}

pub mod lifts_sub {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/extending_submenus/lifts_extension_sub/lifts_extension_sub.bin");
    pub const TRAY_BUTTON: Group = Group("tray_button");
    pub const ENDCAP: Group = Group("endcap");
    pub const TRAY_LIFTS: Group = Group("tray_lifts");
    pub const TRAY_JUMPS: Group = Group("tray_jumps");
    pub const TRAY_MARKERS: Group = Group("tray_markers");
}

pub mod skirun_sub {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/extending_submenus/skirun_extension_sub/skirun_extension_sub.bin");
    pub const TRAY_BUTTON: Group = Group("tray_button");
    pub const ENDCAP: Group = Group("endcap");
    pub const TRAY_RUNADD: Group = Group("tray_runadd");
    pub const TRAY_RUNSUB: Group = Group("tray_runsub");
    pub const TRAY_RUNFOLLOW: Group = Group("tray_runfollow");
}

pub mod surface_sub {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/extending_submenus/surfacemod_extension_sub/surfacemod_extension_sub.bin");
    pub const TRAY_BUTTON: Group = Group("tray_button");
    pub const ENDCAP: Group = Group("endcap");
    pub const TRAY_SMOOTH: Group = Group("tray_smooth");
    pub const TRAY_FLATRAISE: Group = Group("tray_flatraise");
    pub const TRAY_RAISEPOINT: Group = Group("tray_raisepoint");
    pub const TRAY_HILL: Group = Group("tray_hill");
    pub const TRAY_PEAK: Group = Group("tray_peak");
    pub const TRAY_MOGULS: Group = Group("tray_moguls");
    pub const TRAY_SURVEY: Group = Group("tray_survey");
    pub const TRAY_GROOMER: Group = Group("tray_groomer");
    pub const TRAY_SNOW: Group = Group("tray_snow");
}

pub mod trees_sub {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/extending_submenus/trees_extension_sub/trees_extension_sub.bin");
    pub const TRAY_BUTTON: Group = Group("tray_button");
    pub const ENDCAP: Group = Group("endcap");
    pub const TRAY_TREEDENSE: Group = Group("tray_treedense");
    pub const TRAY_TREEMEDIUM: Group = Group("tray_treemedium");
    pub const TRAY_TREESPARSE: Group = Group("tray_treesparse");
    pub const TRAY_CHAINSAW: Group = Group("tray_chainsaw");
}

pub mod finance_ext {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/finance_extension/finance_extension.bin");
    pub const LOG_BUTTON: Group = Group("log_button");
    pub const ENDCAP: Group = Group("endcap");
    pub const LOG_RESORTFINANCE: Group = Group("log_resortfinance");
    pub const LOG_GUESTSTATS: Group = Group("log_gueststats");
    pub const LOG_RESORTSTATS: Group = Group("log_resortstats");
    pub const LOG_ATTRACTION: Group = Group("log_attraction");
    pub const LOG_STAFFSTATS: Group = Group("log_staffstats");
}

pub mod fluff_menu {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/fluff_menu/fluff_menu.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const NEW: Group = Group("new");
    pub const ROTATE: Group = Group("rotate");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const CLOSE: Group = Group("close");
    pub const TAB: Group = Group("tab");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const PATH: Group = Group("path");
    pub const HALFPATH: Group = Group("halfpath");
    pub const TAB_TREES_ANI: Group = Group("tab_trees_ani");
    pub const TAB_OBJECTS: Group = Group("tab_objects");
    pub const TAB_OBJECTS_ANI: Group = Group("tab_objects_ani");
    pub const TAB_PATHS_ANI: Group = Group("tab_paths_ani");
    pub const TAB_PATHS: Group = Group("tab_paths");
    pub const TAB_TREES: Group = Group("tab_trees");
    pub const DELETE: Group = Group("delete");
    pub const GOTO: Group = Group("goto");
    pub const RENAME: Group = Group("rename");
    pub const BACKGROUNDBIG: Group = Group("backgroundbig");
    pub const WD_TILE_B_PATH: Group = Group("wd_tileB_path");
    pub const ROCK_PATH: Group = Group("rock_path");
    pub const STONE_PATH: Group = Group("Stone_path");
    pub const WD_TILE_A_PATH: Group = Group("wd_tileA_path");
    pub const ERASER_HALF: Group = Group("eraser_half");
    pub const ERASER: Group = Group("eraser");
    pub const RENAME_DEPRESS: Group = Group("rename_depress");
    pub const RENAME_NORMAL: Group = Group("rename_normal");
    pub const RENAME_ROLLOVER: Group = Group("rename_rollover");
    pub const GOTO_DEPRESS: Group = Group("goto_depress");
    pub const GOTO_NORMAL: Group = Group("goto_normal");
    pub const GOTO_ROLLOVER: Group = Group("goto_rollover");
    pub const DELETE_DEPRESS: Group = Group("Delete_Depress");
    pub const DELETE_NORMAL: Group = Group("Delete_Normal");
    pub const DELETE_ROLLOVER: Group = Group("Delete_Rollover");
    pub const TREE_DEPRESS: Group = Group("tree_depress");
    pub const TREE_NORMAL_S: Group = Group("tree_normal_S");
    pub const TREE_NORMAL_R: Group = Group("tree_normal_R");
    pub const TREE_NORMAL_Q: Group = Group("tree_normal_Q");
    pub const TREE_NORMAL_P: Group = Group("tree_normal_P");
    pub const TREE_NORMAL_O: Group = Group("tree_normal_O");
    pub const TREE_NORMAL_N: Group = Group("tree_normal_N");
    pub const TREE_NORMAL_M: Group = Group("tree_normal_M");
    pub const TREE_NORMAL_L: Group = Group("tree_normal_L");
    pub const TREE_NORMAL_K: Group = Group("tree_normal_K");
    pub const TREE_NORMAL_J: Group = Group("tree_normal_J");
    pub const TREE_NORMAL_I: Group = Group("tree_normal_I");
    pub const TREE_NORMAL_H: Group = Group("tree_normal_H");
    pub const TREE_NORMAL_G: Group = Group("tree_normal_G");
    pub const TREE_NORMAL_F: Group = Group("tree_normal_F");
    pub const TREE_NORMAL_E: Group = Group("tree_normal_E");
    pub const TREE_NORMAL_D: Group = Group("tree_normal_D");
    pub const TREE_NORMAL_C: Group = Group("tree_normal_C");
    pub const TREE_NORMAL_B: Group = Group("tree_normal_B");
    pub const TREE_NORMAL_A: Group = Group("tree_normal_A");
    pub const TREE_NORMAL_T: Group = Group("tree_normal_T");
    pub const OBJECTS_DEPRESS: Group = Group("Objects_depress");
    pub const PATHS_NORMAL_G: Group = Group("Paths_normal_g");
    pub const PATHS_NORMAL_F: Group = Group("Paths_normal_f");
    pub const PATHS_NORMAL_E: Group = Group("Paths_normal_e");
    pub const PATHS_NORMAL_D: Group = Group("Paths_normal_d");
    pub const PATHS_NORMAL_C: Group = Group("Paths_normal_c");
    pub const PATHS_NORMAL_B: Group = Group("Paths_normal_b");
    pub const PATHS_NORMAL_A: Group = Group("Paths_normal_a");
    pub const PATHS_DEPRESS: Group = Group("Paths_depress");
    pub const OPEN_ROLLOVER: Group = Group("Open_rollover");
    pub const OPEN_NORMAL: Group = Group("Open_normal");
    pub const OPEN_DEPRESS: Group = Group("Open_depress");
    pub const OBJECTS_NORMAL_I: Group = Group("Objects_normal_I");
    pub const OBJECTS_NORMAL_H: Group = Group("Objects_normal_H");
    pub const OBJECTS_NORMAL_G: Group = Group("Objects_normal_G");
    pub const OBJECTS_NORMAL_F: Group = Group("Objects_normal_F");
    pub const OBJECTS_NORMAL_E: Group = Group("Objects_normal_E");
    pub const OBJECTS_NORMAL_D: Group = Group("Objects_normal_D");
    pub const OBJECTS_NORMAL_C: Group = Group("Objects_normal_C");
    pub const OBJECTS_NORMAL_B: Group = Group("Objects_normal_B");
    pub const OBJECTS_NORMAL_A: Group = Group("Objects_normal_A");
    pub const PATHS_NORMAL_H: Group = Group("Paths_normal_h");
    pub const BRICK: Group = Group("Brick");
    pub const CEMENT: Group = Group("Cement");
    pub const COBBLE: Group = Group("Cobble");
    pub const TITLE_BAR_NORMAL: Group = Group("Title_Bar_Normal");
    pub const TAB_NORMAL: Group = Group("Tab_Normal");
    pub const TAB_DEPRESS: Group = Group("Tab_Depress");
    pub const CLOSE_ROLLOVER: Group = Group("Close_Rollover");
    pub const CLOSE_NORMAL: Group = Group("Close_Normal");
    pub const CLOSE_DEPRESS: Group = Group("Close_Depress");
    pub const SCROLL_BAR: Group = Group("Scroll_Bar");
    pub const SCROLL_BAR_VERTICAL: Group = Group("Scroll_Bar_Vertical");
    pub const SCROLL_RIGHT_ROLLOVER: Group = Group("Scroll_Right_Rollover");
    pub const SCROLL_RIGHT_NORMAL: Group = Group("Scroll_Right_Normal");
    pub const SCROLL_RIGHT_DEPRESS: Group = Group("Scroll_Right_Depress");
    pub const SCROLL_LEFT_ROLLOVER: Group = Group("Scroll_Left_Rollover");
    pub const SCROLL_LEFT_NORMAL: Group = Group("Scroll_Left_Normal");
    pub const SCROLL_LEFT_DEPRESS: Group = Group("Scroll_Left_Depress");
    pub const SCROLL_DOWN_ROLLOVER: Group = Group("Scroll_Down_Rollover");
    pub const SCROLL_DOWN_NORMAL: Group = Group("Scroll_Down_Normal");
    pub const SCROLL_DOWN_DEPRESS: Group = Group("Scroll_Down_Depress");
    pub const SCROLL_UP_ROLLOVER: Group = Group("Scroll_Up_Rollover");
    pub const SCROLL_UP_NORMAL: Group = Group("Scroll_Up_Normal");
    pub const SCROLL_UP_DEPRESS: Group = Group("Scroll_Up_Depress");
    pub const NEW_DEPRESS: Group = Group("new_depress");
    pub const NEW_ROLL_OVER: Group = Group("new_roll_over");
    pub const NEW_NORMAL: Group = Group("new_normal");
    pub const ROTATE_NORMAL: Group = Group("rotate_normal");
    pub const ROTATE_DEPRESS: Group = Group("rotate_depress");
    pub const ROTATE_ROLL_OVER: Group = Group("rotate_roll_over");
    pub const DIALOG_BACKGROUND: Group = Group("Dialog_Background");
    pub const DIALOG_BACKGROUND2: Group = Group("Dialog_Background2");
    pub const SCROLL_TAB: Group = Group("Scroll_Tab");
}

pub mod scaled_stats_bar {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/scaledstatsbar/scaledstatsbar.bin");
    pub const TRAY: Group = Group("tray");
    pub const GRAPH: Group = Group("graph");
}

pub mod guests_stats {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/guests_toolbar/guests_toolbar.bin");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const CLOSE: Group = Group("close");
    pub const THOUGHT: Group = Group("thought");
    pub const BACKGROUND: Group = Group("background");
    pub const TAB: Group = Group("tab");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const ACTION: Group = Group("action");
    pub const AVE: Group = Group("ave");
    pub const AVE_ANI: Group = Group("ave_ani");
    pub const INDIVIDUAL_ANI: Group = Group("individual_ani");
    pub const INDIVIDUAL: Group = Group("individual");
    pub const GROUP: Group = Group("group");
    pub const GROUP_ANI: Group = Group("group_ani");
}

pub mod highlevel_stats {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/highlevel_statistics/highlevel_statistics.bin");
    pub const MESSAGE: Group = Group("message");
    pub const MONEY: Group = Group("money");
    pub const TIME: Group = Group("time");
    pub const LIGHT: Group = Group("light");
    pub const GOTO: Group = Group("goto");
    pub const DELETE: Group = Group("delete");
    pub const ROTATE: Group = Group("rotate");
    pub const STAR: Group = Group("star");
    pub const PLACEMENT: Group = Group("placement");
    pub const SPACE_BAR: Group = Group("SpaceBar");
    pub const WRENCH_ZOOM1: Group = Group("WrenchZoom1");
    pub const WRENCH_ZOOM2: Group = Group("WrenchZoom2");
    pub const WRENCH_ZOOM3: Group = Group("WrenchZoom3");
    pub const WRENCH_ZOOM4: Group = Group("WrenchZoom4");
}

pub mod instant_action {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/instant_action/instant_action.bin");
    pub const CURSOR: Group = Group("cursor");
    pub const RADIO_BUTTON: Group = Group("RadioButton");
    pub const BACKGROUND: Group = Group("background");
    pub const INSTANT_START: Group = Group("INSTANT_Start");
    pub const INSTANT_CANCEL: Group = Group("INSTANT_Cancel");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const MOUNTAIN: Group = Group("mountain");
}

pub mod lifts_toolbar {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/lifts_toolbar/lifts_toolbar.bin");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const CLOSE: Group = Group("close");
    pub const BACKGROUND: Group = Group("background");
    pub const LIFT: Group = Group("lift");
    pub const ROPE_TOW: Group = Group("rope_tow");
    pub const DELETE: Group = Group("delete");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const QUESTION_MARK_LEFT: Group = Group("QuestionMarkLeft");
    pub const QUESTION_MARK: Group = Group("QuestionMark");
    pub const QUESTION_MARK_RIGHT: Group = Group("QuestionMarkRight");
}

pub mod gen_map {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/generate_random_map/generate_random_map.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const GENERATE_BUTTON: Group = Group("generate_button");
    pub const CLOSE_BUTTON: Group = Group("close_button");
    pub const RAND_WIREFRAME_BUTTON: Group = Group("rand_wireframe_button");
    pub const ANCHOR_POINT: Group = Group("anchor_point");
    pub const GEN_TREES: Group = Group("Gen_Trees");
    pub const FLATTEN_ENTRANCE: Group = Group("flatten_entrance");
    pub const SCROLL_TAB: Group = Group("scroll_tab");
}

pub mod level_edit {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/level_edit/level_edit.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const START_BUTTON: Group = Group("start_button");
    pub const CANCEL_BUTTON: Group = Group("cancel_button");
    pub const CURSOR: Group = Group("cursor");
    pub const RADIO_BUTTON: Group = Group("RadioButton");
}

pub mod load_game {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/load_game/load_game.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const CURSOR: Group = Group("cursor");
    pub const LOAD_BUTTON: Group = Group("load_button");
    pub const CANCEL_BUTTON: Group = Group("cancel_button");
}

pub mod credits {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/main/credits.bin");
    pub const OK_BUTTON: Group = Group("OkButton");
    pub const BACKGROUND: Group = Group("Background");
    pub const CREDITS_LIST: Group = Group("CreditsList");
}

pub mod main_menu {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/main/main.bin");
    pub const MAIN: Group = Group("main");
    pub const MAIN_INSTANT_ACTION_BUTTON: Group = Group("MAIN_InstantActionButton");
    pub const MAIN_CHALLENGES_BUTTON: Group = Group("MAIN_ChallengesButton");
    pub const MAIN_LOAD_GAME: Group = Group("MAIN_LoadGame");
    pub const MAIN_SETTINGS: Group = Group("MAIN_Settings");
    pub const MAIN_CREDITS: Group = Group("MAIN_Credits");
    pub const MAIN_MAP_EDITOR: Group = Group("MAIN_MapEditor");
    pub const MAIN_QUIT: Group = Group("MAIN_Quit");
}

pub mod mini_map {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/mini_map_window/mini_map_window.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const RANDOM_GEN: Group = Group("random_gen");
    pub const FILTER_ALL: Group = Group("filter_all");
    pub const FILTER_STRUCTS: Group = Group("filter_structs");
    pub const FILTER_SKIERS: Group = Group("filter_skiers");
    pub const FILTER_LIFTS: Group = Group("filter_lifts");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const BACKGROUND_MINIMIZED: Group = Group("background_minimized");
    pub const CLOSE: Group = Group("close");
}

pub mod nav_bar {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/navigation_bar/navigation_bar.bin");
    pub const MAIN: Group = Group("main");
    pub const PAUSE: Group = Group("pause");
    pub const PAUSE_OFF: Group = Group("pause_off");
    pub const BUILD: Group = Group("build");
    pub const TERRAIN: Group = Group("terrain");
    pub const FINANCES: Group = Group("finances");
    pub const BACKGROUND_640: Group = Group("640background");
    pub const ZOOM_IN: Group = Group("zoom_in");
    pub const ZOOM_OUT: Group = Group("zoom_out");
    pub const ROTATE_CW: Group = Group("rotate_cw");
    pub const ROTATE_CCW: Group = Group("rotate_ccw");
    pub const FIRST_PERSON: Group = Group("first_person");
    pub const CONSTRUCTION_OVERLAY: Group = Group("construction_overlay");
    pub const TERRAIN_OVERLAY: Group = Group("terrain_overlay");
    pub const FINANCE_OVERLAY: Group = Group("finance_overlay");
    pub const LOG_GRID: Group = Group("log_grid");
    pub const MAP_VIEW: Group = Group("map_view");
    pub const STAR: Group = Group("star");
    pub const SPACE_BAR: Group = Group("SpaceBar");
    pub const BACKGROUND_800: Group = Group("800background");
    pub const BACKGROUND_1024: Group = Group("1024background");
    pub const TE_BACKGROUND_640: Group = Group("640TEBackground");
    pub const TE_BACKGROUND_800: Group = Group("800TEBackground");
    pub const TE_BACKGROUND_1024: Group = Group("1024TEBackground");
    pub const LOG_SURVEY: Group = Group("log_survey");
}

pub mod no_power {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/no_power/no_power.bin");
    pub const ZOOM1: Group = Group("Zoom1");
    pub const ZOOM3: Group = Group("Zoom3");
    pub const ZOOM2: Group = Group("Zoom2");
    pub const ZOOM4: Group = Group("Zoom4");
}

pub mod guest_window {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/one_guest_window/one_guest_window.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const CLOSE: Group = Group("close");
    pub const FOCUS: Group = Group("focus");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const PATSEYE: Group = Group("patseye");
    pub const RENAME: Group = Group("rename");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const TAB: Group = Group("tab");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const HAND_OF_GOD: Group = Group("HandOfGod");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const SKILL: Group = Group("skill");
    pub const ONE_GUEST_MAIN: Group = Group("OneGuestMain");
    pub const GUEST_STATS_TAB: Group = Group("GuestStatsTab");
    pub const GUEST_THOUGHTS_TAB: Group = Group("GuestThoughtsTab");
    pub const EMP_AVALANCHE_TAB: Group = Group("EmpAvalancheTab");
    pub const EMP_CATERING_TAB: Group = Group("EmpCateringTab");
    pub const GUEST_GROUP_TAB: Group = Group("GuestGroupTab");
    pub const EMP_MAINTENANCE_TAB: Group = Group("EmpMaintenanceTab");
    pub const EMP_SANITATION_TAB: Group = Group("EmpSanitationTab");
    pub const EMP_SECURITY_TAB: Group = Group("EmpSecurityTab");
    pub const EMP_INSTRUCTOR_TAB: Group = Group("EmpInstructorTab");
    pub const FIRE: Group = Group("fire");
    pub const EMP_SKI_PATROL_TAB: Group = Group("EmpSkiPatrolTab");
}

pub mod lift_window {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/one_lift_window/one_lift_window.bin");
    pub const STATS_TAB: Group = Group("stats_tab");
    pub const CLOSE: Group = Group("close");
    pub const FOCUS: Group = Group("focus");
    pub const GUEST_TAB: Group = Group("guest_tab");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const MAIN_TAB: Group = Group("main_tab");
    pub const OPEN_BUILDING: Group = Group("OpenBuilding");
    pub const PATSEYE: Group = Group("patseye");
    pub const RENAME: Group = Group("rename");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const SPINUP: Group = Group("spinup");
    pub const SPINDOWN: Group = Group("spindown");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const CLOSE_BUILDING: Group = Group("CloseBuilding");
    pub const SELL: Group = Group("sell");
    pub const BACKGROUND: Group = Group("background");
}

pub mod runmarker_window {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/one_runmarker_window/one_runmarker_window.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const MARKER_TAB: Group = Group("marker_tab");
    pub const CLOSE: Group = Group("close");
    pub const CLOSE_BUILDING: Group = Group("CloseBuilding");
    pub const OPEN_BUILDING: Group = Group("OpenBuilding");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const FOCUS: Group = Group("focus");
    pub const PATSEYE: Group = Group("patseye");
    pub const RENAME: Group = Group("rename");
    pub const SELL: Group = Group("sell");
    pub const JUMP_TAB: Group = Group("jump_tab");
}

pub mod structure_window {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/one_structure_window/one_structure_window.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const BACKGROUND_LIFTS: Group = Group("background_lifts");
    pub const STATS_TAB: Group = Group("stats_tab");
    pub const CLOSE: Group = Group("close");
    pub const FOCUS: Group = Group("focus");
    pub const GUEST_TAB: Group = Group("guest_tab");
    pub const HIRE: Group = Group("hire");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const MAIN_TAB: Group = Group("main_tab");
    pub const OPEN_BUILDING: Group = Group("OpenBuilding");
    pub const PATSEYE: Group = Group("patseye");
    pub const RENAME: Group = Group("rename");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const SPINUP: Group = Group("spinup");
    pub const SPINDOWN: Group = Group("spindown");
    pub const STAFF_TAB: Group = Group("staff_tab");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const CLOSE_BUILDING: Group = Group("CloseBuilding");
    pub const SELL: Group = Group("sell");
    pub const UPGRADE: Group = Group("upgrade");
    pub const MUSIC_TAB: Group = Group("music_tab");
    pub const PLAY: Group = Group("play");
    pub const STOP: Group = Group("stop");
    pub const HIGHLIGHT: Group = Group("highlight");
    pub const BACKGROUND_NOCOST: Group = Group("background_nocost");
}

pub mod finances_stats {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/resort_expenses_toolbar/resort_expenses_toolbar.bin");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const CLOSE: Group = Group("close");
    pub const TAB: Group = Group("tab");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const ASSET_TAB: Group = Group("asset_tab");
    pub const CASH_TAB: Group = Group("cash_tab");
    pub const CHART_TAB: Group = Group("chart_tab");
    pub const ASSET_TAB_ANI: Group = Group("asset_tab_ani");
    pub const CASH_TAB_ANI: Group = Group("cash_tab_ani");
    pub const CHART_TAB_ANI: Group = Group("chart_tab_ani");
    pub const BACKGROUND: Group = Group("background");
}

pub mod resort_stats {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/restort_stats_toolbar/resort_stats_toolbar.bin");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const CLOSE: Group = Group("close");
    pub const BACKGROUND: Group = Group("background");
    pub const TAB: Group = Group("tab");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const CLOSE_S: Group = Group("close_s");
    pub const OPEN: Group = Group("open");
    pub const GOTO: Group = Group("goto");
    pub const WEATHER: Group = Group("weather");
    pub const WIND: Group = Group("wind");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const TAB_MAIN: Group = Group("tab_main");
    pub const TAB_MAIN_ANI: Group = Group("tab_main_ani");
    pub const TAB_GUESTS: Group = Group("tab_guests");
    pub const TAB_GUESTS_ANI: Group = Group("tab_guests_ani");
    pub const TAB_RATE: Group = Group("tab_rate");
    pub const TAB_RATE_ANI: Group = Group("tab_rate_ani");
    pub const TAB_WEATHER: Group = Group("tab_weather");
    pub const TAB_HISTORY: Group = Group("tab_history");
    pub const SPINDOWN: Group = Group("spindown");
    pub const SPINBACK: Group = Group("spinback");
    pub const TAB_WEATHER_ANI: Group = Group("tab_weather_ani");
    pub const TAB_HISTORY_ANI: Group = Group("tab_history_ani");
    pub const RENAME: Group = Group("rename");
    pub const SPINUP: Group = Group("spinup");
}

pub mod save_game {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/save_game/save_game.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const CURSOR: Group = Group("cursor");
    pub const DELETE_BUTTON: Group = Group("DeleteButton");
    pub const SAVE_BUTTON: Group = Group("save_button");
    pub const CANCEL_BUTTON: Group = Group("cancel_button");
}

pub mod settings {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/settings/settings.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const RADIO_BUTTON: Group = Group("RadioButton");
    pub const OK_BUTTON: Group = Group("OkButton");
    pub const LEFT_REZ: Group = Group("LeftRez");
    pub const RIGHT_REZ: Group = Group("RightRez");
    pub const REZS: Group = Group("Rezs");
    pub const CHECK_BUTTON: Group = Group("CheckButton");
}

pub mod staff_window {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/staff_window/staff_window.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const CLOSE: Group = Group("close");
    pub const FOCUS: Group = Group("focus");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const PATSEYE: Group = Group("patseye");
    pub const RENAME: Group = Group("rename");
    pub const SELL: Group = Group("sell");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const TAB_AVALANCHE: Group = Group("tab_avalanche");
    pub const TAB_MAINTENANCE: Group = Group("tab_maintenance");
    pub const TAB_SECURITY: Group = Group("tab_security");
    pub const TAB_SKIPATROL: Group = Group("tab_skipatrol");
    pub const TAB_SANITATION: Group = Group("tab_sanitation");
    pub const TAB_INSTRUCTOR: Group = Group("tab_instructor");
    pub const TAB_CATERING: Group = Group("tab_catering");
}

pub mod attractions {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/structure_list_menu/structure_list_toolbar.bin");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const CLOSE: Group = Group("close");
    pub const BACKGROUND: Group = Group("background");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const TAB_JUMP_ANI: Group = Group("tab_jump_ani");
    pub const TAB_JUMP: Group = Group("tab_jump");
    pub const TAB_LIFT: Group = Group("tab_lift");
    pub const TAB_LIFT_ANI: Group = Group("tab_lift_ani");
    pub const TAB_RUN: Group = Group("tab_run");
    pub const TAB_RUN_ANI: Group = Group("tab_run_ani");
    pub const TAB_STRUCTURE: Group = Group("tab_structure");
    pub const TAB_STRUCTURE_ANI: Group = Group("tab_structure_ani");
}

pub mod structures_toolbar {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/structures_toolbar/structures_toolbar.bin");
    pub const BACKGROUND: Group = Group("background");
    pub const NEW: Group = Group("new");
    pub const ROTATE: Group = Group("rotate");
    pub const DELETE: Group = Group("delete");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const CLOSE: Group = Group("close");
    pub const TAB: Group = Group("tab");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
    pub const QUESTION_MARK: Group = Group("QuestionMark");
    pub const QUESTION_MARK_LEFT: Group = Group("QuestionMarkLeft");
    pub const QUESTION_MARK_RIGHT: Group = Group("QuestionMarkRight");
    pub const BUILD: Group = Group("build");
}

pub mod terrain_ext {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/terrain_extension/terrain_extension.bin");
    pub const LOG_BUTTON: Group = Group("log_button");
    pub const ENDCAP: Group = Group("endcap");
    pub const SURFACEMOD_OVERLAY: Group = Group("surfacemod_overlay");
    pub const SKIRUN_OVERLAY: Group = Group("skirun_overlay");
    pub const TREES_OVERLAY: Group = Group("trees_overlay");
    pub const LOG_SURFACEMOD: Group = Group("log_surfacemod");
    pub const LOG_SKIRUN: Group = Group("log_skirun");
    pub const LOG_TREES: Group = Group("log_trees");
}

pub mod thoughts {
    use super::{ArtFile, Group};
    pub const FILE: ArtFile = ArtFile("data/ui/thoughts/thoughts.bin");
    pub const HUNGER: Group = Group("hunger");
    pub const ANGER: Group = Group("anger");
    pub const COLD: Group = Group("cold");
    pub const PEE: Group = Group("pee");
    pub const HAPPINESS: Group = Group("happiness");
    pub const ALERT: Group = Group("alert");
    pub const THIRSTY: Group = Group("thirsty");
    pub const TIRED: Group = Group("tired");
}

/// Groups in every file of: structure_window, lift_window, runmarker_window, guest_window, staff_window.
pub mod object_window {
    use super::{ArtFile, Group};
    pub const FILES: &[ArtFile] = &[super::structure_window::FILE, super::lift_window::FILE, super::runmarker_window::FILE, super::guest_window::FILE, super::staff_window::FILE];
    pub const BACKGROUND: Group = Group("background");
    pub const CLOSE: Group = Group("close");
    pub const FOCUS: Group = Group("focus");
    pub const PATSEYE: Group = Group("patseye");
    pub const RENAME: Group = Group("rename");
    pub const TITLE_BAR: Group = Group("title_bar");
}

/// Groups in every file of: resort_stats, finances_stats, guests_stats, staff_window, attractions.
pub mod stat_window {
    use super::{ArtFile, Group};
    pub const FILES: &[ArtFile] = &[super::resort_stats::FILE, super::finances_stats::FILE, super::guests_stats::FILE, super::staff_window::FILE, super::attractions::FILE];
    pub const BACKGROUND: Group = Group("background");
    pub const CLOSE: Group = Group("close");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const TITLE_BAR: Group = Group("title_bar");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
}

/// Groups in every file of: structures_toolbar, lifts_toolbar, fluff_menu.
pub mod picker {
    use super::{ArtFile, Group};
    pub const FILES: &[ArtFile] = &[super::structures_toolbar::FILE, super::lifts_toolbar::FILE, super::fluff_menu::FILE];
    pub const BACKGROUND: Group = Group("background");
    pub const CLOSE: Group = Group("close");
    pub const DELETE: Group = Group("delete");
    pub const HORIZ_SCROLLBAR: Group = Group("horiz_scrollbar");
    pub const HORIZ_SCROLLNUB: Group = Group("horiz_scrollnub");
    pub const SCROLLLEFT: Group = Group("scrollleft");
    pub const SCROLLRIGHT: Group = Group("scrollright");
    pub const TITLE_BAR: Group = Group("title_bar");
}

/// Groups in every file of: build_sub, fluff_sub, lifts_sub, skirun_sub, surface_sub, trees_sub.
pub mod tray {
    use super::{ArtFile, Group};
    pub const FILES: &[ArtFile] = &[super::build_sub::FILE, super::fluff_sub::FILE, super::lifts_sub::FILE, super::skirun_sub::FILE, super::surface_sub::FILE, super::trees_sub::FILE];
    pub const ENDCAP: Group = Group("endcap");
    pub const TRAY_BUTTON: Group = Group("tray_button");
}

/// Groups in every file of: construction_ext, terrain_ext, finance_ext.
pub mod log_menu {
    use super::{ArtFile, Group};
    pub const FILES: &[ArtFile] = &[super::construction_ext::FILE, super::terrain_ext::FILE, super::finance_ext::FILE];
    pub const ENDCAP: Group = Group("endcap");
    pub const LOG_BUTTON: Group = Group("log_button");
}

/// Groups in every file of: instant_action, challenge_select.
pub mod list_screen {
    use super::{ArtFile, Group};
    pub const FILES: &[ArtFile] = &[super::instant_action::FILE, super::challenge_select::FILE];
    pub const BACKGROUND: Group = Group("background");
    pub const MOUNTAIN: Group = Group("mountain");
    pub const RADIO_BUTTON: Group = Group("RadioButton");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
}

/// Groups in every extending menu and tray file.
pub mod extension {
    use super::{ArtFile, Group};
    pub const FILES: &[ArtFile] = &[
        super::construction_ext::FILE,
        super::terrain_ext::FILE,
        super::finance_ext::FILE,
        super::build_sub::FILE,
        super::fluff_sub::FILE,
        super::lifts_sub::FILE,
        super::skirun_sub::FILE,
        super::surface_sub::FILE,
        super::trees_sub::FILE,
    ];
    pub const ENDCAP: Group = Group("endcap");
}

/// Groups in every window for something that opens and closes: structure_window, lift_window, runmarker_window.
pub mod facility_window {
    use super::{ArtFile, Group};
    pub const FILES: &[ArtFile] = &[super::structure_window::FILE, super::lift_window::FILE, super::runmarker_window::FILE];
    pub const BACKGROUND: Group = Group("background");
    pub const CLOSE: Group = Group("close");
    pub const CLOSE_BUILDING: Group = Group("CloseBuilding");
    pub const FOCUS: Group = Group("focus");
    pub const OPEN_BUILDING: Group = Group("OpenBuilding");
    pub const PATSEYE: Group = Group("patseye");
    pub const RENAME: Group = Group("rename");
    pub const SELL: Group = Group("sell");
    pub const TITLE_BAR: Group = Group("title_bar");
}

/// Groups in every screen with a scrolling list: instant_action, challenge_select, level_edit, load_game, save_game.
pub mod list_box {
    use super::{ArtFile, Group};
    pub const FILES: &[ArtFile] = &[
        super::instant_action::FILE,
        super::challenge_select::FILE,
        super::level_edit::FILE,
        super::load_game::FILE,
        super::save_game::FILE,
    ];
    pub const BACKGROUND: Group = Group("background");
    pub const SCROLLDOWN: Group = Group("scrolldown");
    pub const SCROLLUP: Group = Group("scrollup");
    pub const VERT_SCROLLBAR: Group = Group("vert_scrollbar");
    pub const VERT_SCROLLNUB: Group = Group("vert_scrollnub");
}
