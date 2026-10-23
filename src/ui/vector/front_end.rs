//! The front-end menus (`main_menu`, `instant_action`,
//! `challenge_select`, `level_edit`, `load_game`, `save_game`,
//! `settings` and `credits`).
//!
//! Each screen file gets one function here, named after its `art`
//! module, that matches its groups the way `hud` in `vector.rs` does,
//! and `element` calls it for that file. The shared widgets are mapped
//! in `widgets` for every file; an arm here takes precedence, so it can
//! claim the elements of a group that are the screen's own art.
//!
//! Every screen but the main menu draws the mountain scene (the credits
//! screen's background, whose only chrome is its title tab) and then
//! its background group, which is its title tab, title, panels and
//! lettering here: `TITLE` under a drawing of the screen's own. The main
//! menu's background keeps its scene and logo, and its buttons cover
//! the ones painted into it.

use super::{Drawing, Svg, button, still, widgets};
use crate::ui::art::{self, ArtFile, Group};

/// The blue title tab the screens' drawings go over.
const TITLE: Svg = svg!("front_end/title");
const INSTANT_ACTION: Svg = svg!("front_end/instant_action" on TITLE);
const CHALLENGE_SELECT: Svg = svg!("front_end/challenge_select" on TITLE);
const LEVEL_EDIT: Svg = svg!("front_end/level_edit" on TITLE);
const LOAD_GAME: Svg = svg!("front_end/load_game" on TITLE);
const SAVE_GAME: Svg = svg!("front_end/save_game" on TITLE);
const SETTINGS: Svg = svg!("front_end/settings" on TITLE);
const CREDITS: Svg = svg!("front_end/credits" on TITLE);

/// The pill buttons, each with its label.
const START: Svg = svg!("front_end/start");
const CANCEL: Svg = svg!("front_end/cancel");
const LOAD: Svg = svg!("front_end/load");
const LOAD_CANCEL: Svg = svg!("front_end/load_cancel");
const SAVE: Svg = svg!("front_end/save");
const SAVE_CANCEL: Svg = svg!("front_end/save_cancel");
const DELETE: Svg = svg!("front_end/delete");
const QUIT: Svg = svg!("front_end/quit");
const SETTINGS_OK: Svg = svg!("front_end/settings_ok");

/// The main menu's buttons: a picture and label each over the shared
/// ring and pill.
const MAIN_BUTTON: Svg = svg!("front_end/main_button");
const INSTANT_ACTION_BUTTON: Svg = svg!("front_end/main_instant_action" on MAIN_BUTTON);
const CHALLENGES_BUTTON: Svg = svg!("front_end/main_challenges" on MAIN_BUTTON);
const LOAD_GAME_BUTTON: Svg = svg!("front_end/main_load" on MAIN_BUTTON);
const SETTINGS_BUTTON: Svg = svg!("front_end/main_settings" on MAIN_BUTTON);
const MAP_EDITOR_BUTTON: Svg = svg!("front_end/main_map_editor" on MAIN_BUTTON);
const CREDITS_BUTTON: Svg = svg!("front_end/main_credits" on MAIN_BUTTON);

/// The drawing for an element of one of these screen files, if it has
/// one of its own.
pub(super) fn element(file: ArtFile, group: Group, index: usize) -> Option<Drawing> {
    match file {
        art::main_menu::FILE => main_menu(group, index),
        art::instant_action::FILE => instant_action(group, index),
        art::challenge_select::FILE => challenge_select(group, index),
        art::level_edit::FILE => level_edit(group, index),
        art::load_game::FILE => load_game(group, index),
        art::save_game::FILE => save_game(group, index),
        art::settings::FILE => settings(group, index),
        art::credits::FILE => credits(group, index),
        _ => None,
    }
}

fn main_menu(group: Group, index: usize) -> Option<Drawing> {
    use art::main_menu::*;
    match group {
        MAIN_INSTANT_ACTION_BUTTON => button(INSTANT_ACTION_BUTTON, index),
        MAIN_CHALLENGES_BUTTON => button(CHALLENGES_BUTTON, index),
        MAIN_LOAD_GAME => button(LOAD_GAME_BUTTON, index),
        MAIN_SETTINGS => button(SETTINGS_BUTTON, index),
        MAIN_MAP_EDITOR => button(MAP_EDITOR_BUTTON, index),
        MAIN_CREDITS => button(CREDITS_BUTTON, index),
        MAIN_QUIT => button(QUIT, index),
        _ => None,
    }
}

fn instant_action(group: Group, index: usize) -> Option<Drawing> {
    use art::instant_action::*;
    match group {
        BACKGROUND => still(INSTANT_ACTION),
        INSTANT_START => button(START, index),
        INSTANT_CANCEL => button(CANCEL, index),
        _ => None,
    }
}

fn challenge_select(group: Group, index: usize) -> Option<Drawing> {
    use art::challenge_select::{BACKGROUND, CANCEL as CANCEL_GROUP, START as START_GROUP};
    match group {
        BACKGROUND => still(CHALLENGE_SELECT),
        START_GROUP => button(START, index),
        CANCEL_GROUP => button(CANCEL, index),
        _ => None,
    }
}

fn level_edit(group: Group, index: usize) -> Option<Drawing> {
    use art::level_edit::*;
    match group {
        BACKGROUND => still(LEVEL_EDIT),
        START_BUTTON => button(START, index),
        CANCEL_BUTTON => button(CANCEL, index),
        _ => None,
    }
}

fn load_game(group: Group, index: usize) -> Option<Drawing> {
    use art::load_game::*;
    match group {
        BACKGROUND => still(LOAD_GAME),
        LOAD_BUTTON => button(LOAD, index),
        CANCEL_BUTTON => button(LOAD_CANCEL, index),
        _ => None,
    }
}

fn save_game(group: Group, index: usize) -> Option<Drawing> {
    use art::save_game::*;
    match group {
        BACKGROUND => still(SAVE_GAME),
        SAVE_BUTTON => button(SAVE, index),
        CANCEL_BUTTON => button(SAVE_CANCEL, index),
        DELETE_BUTTON => button(DELETE, index),
        _ => None,
    }
}

fn settings(group: Group, index: usize) -> Option<Drawing> {
    use art::settings::*;
    match group {
        BACKGROUND => still(SETTINGS),
        // The background shows it at rest; its element 0 has no sprite.
        OK_BUTTON => button(SETTINGS_OK, index),
        _ => None,
    }
}

fn credits(group: Group, index: usize) -> Option<Drawing> {
    use art::credits::*;
    match group {
        BACKGROUND => still(CREDITS),
        OK_BUTTON => button(widgets::OK, index),
        _ => None,
    }
}
