//! The pickers (`structures_toolbar`, `lifts_toolbar` and `fluff_menu`)
//! and the in-game dialogs (`rename`, `gen_map`, `no_power`, `thoughts`,
//! `mini_map`, `escape`, `bob` and `fog`).
//!
//! Each screen file gets one function here, named after its `art`
//! module, that matches its groups the way `hud` in `vector.rs` does,
//! and `element` calls it for that file. The shared widgets are mapped
//! in `widgets` for every file; an arm here takes precedence, so it can
//! claim the elements of a group that are the screen's own art.

use super::{Drawing, Svg, button, shown, still, widgets};
use crate::ui::art::{self, ArtFile, Group};

/// The building and lift pickers' panel (the same art in both).
const PANEL: Svg = svg!("windows/pickers/panel");
/// The question mark over a locked entry: `whole` in the middle box,
/// `left` and `right` its halves in the side boxes.
const QUESTION: Svg = svg!("windows/pickers/question");
/// The round Build and Rotate buttons. The original shows them white
/// under the pointer as at rest, so their rollover is their normal look.
const BUILD: Svg = svg!("windows/pickers/build" on widgets::ROUND);
const ROTATE: Svg = svg!("windows/pickers/rotate" on widgets::ROUND);

/// A picker's round button, white under the pointer as at rest.
fn white_button(svg: Svg, index: usize) -> Option<Drawing> {
    button(svg, if index == 1 { 0 } else { index })
}

/// The drawing for an element of one of these screen files, if it has
/// one of its own.
pub(super) fn element(file: ArtFile, group: Group, index: usize) -> Option<Drawing> {
    match file {
        art::structures_toolbar::FILE => structures_toolbar(group, index),
        art::lifts_toolbar::FILE => lifts_toolbar(group, index),
        art::fluff_menu::FILE => fluff_menu(group, index),
        art::rename::FILE => rename(group, index),
        art::gen_map::FILE => gen_map(group, index),
        art::no_power::FILE => no_power(group, index),
        art::thoughts::FILE => thoughts(group),
        art::mini_map::FILE => mini_map(group, index),
        art::escape::FILE => escape(group, index),
        art::bob::FILE => bob(group, index),
        art::fog::FILE => fog(group, index),
        _ => None,
    }
}

fn structures_toolbar(group: Group, index: usize) -> Option<Drawing> {
    use art::structures_toolbar::*;
    match group {
        BACKGROUND => still(PANEL),
        QUESTION_MARK => shown(QUESTION, "whole"),
        QUESTION_MARK_LEFT => shown(QUESTION, "left"),
        QUESTION_MARK_RIGHT => shown(QUESTION, "right"),
        HORIZ_SCROLLBAR => still(svg!("windows/pickers/scrollbox")),
        BUILD => white_button(self::BUILD, index),
        ROTATE => white_button(self::ROTATE, index),
        _ => None,
    }
}

fn lifts_toolbar(group: Group, index: usize) -> Option<Drawing> {
    use art::lifts_toolbar::*;
    match group {
        BACKGROUND => still(PANEL),
        QUESTION_MARK => shown(QUESTION, "whole"),
        QUESTION_MARK_LEFT => shown(QUESTION, "left"),
        QUESTION_MARK_RIGHT => shown(QUESTION, "right"),
        HORIZ_SCROLLBAR => still(svg!("windows/pickers/scrollbox_wide")),
        LIFT => white_button(BUILD, index),
        _ => None,
    }
}

fn fluff_menu(group: Group, index: usize) -> Option<Drawing> {
    use art::fluff_menu::*;
    // The paths entries: the eraser, then the four paths, whole in the
    // middle box and their left part in the side boxes.
    let sample = |part| {
        let svg = match index {
            0 => svg!("windows/pickers/eraser"),
            1 => svg!("windows/pickers/path_rock"),
            2 => svg!("windows/pickers/path_cobble"),
            3 => svg!("windows/pickers/path_brick"),
            _ => svg!("windows/pickers/path_herringbone"),
        };
        shown(svg, part)
    };
    match group {
        BACKGROUND => still(svg!("windows/pickers/objects_panel")),
        PATH => sample("whole"),
        HALFPATH => sample("half"),
        HORIZ_SCROLLNUB => still(svg!("windows/pickers/objects_nub")),
        ROTATE => white_button(self::ROTATE, index),
        // Not shown by the game here; `highlevel_stats` has the same
        // button.
        GOTO => button(svg!("windows/widgets/goto"), index),
        _ => None,
    }
}

fn rename(group: Group, index: usize) -> Option<Drawing> {
    use art::rename::*;
    match group {
        BACKGROUND => still(svg!("dialogs/rename/window")),
        OK => button(svg!("dialogs/rename/ok" on widgets::ROUND), index),
        CANCEL => button(svg!("dialogs/rename/cancel" on widgets::ROUND), index),
        _ => None,
    }
}

fn gen_map(group: Group, index: usize) -> Option<Drawing> {
    use art::gen_map::*;
    match group {
        BACKGROUND => still(svg!("dialogs/generate_map/window")),
        GENERATE_BUTTON => button(svg!("dialogs/generate_map/terrain"), index),
        GEN_TREES => button(svg!("dialogs/generate_map/trees"), index),
        RAND_WIREFRAME_BUTTON => button(svg!("dialogs/generate_map/go"), index),
        ANCHOR_POINT if index == 0 => still(svg!("dialogs/generate_map/anchor")),
        SCROLL_TAB => still(svg!("dialogs/generate_map/nub")),
        FLATTEN_ENTRANCE => shown(svg!("dialogs/generate_map/check"), if index == 0 { "off" } else { "on" }),
        _ => None,
    }
}

/// The ten frames of the spinning bolt, as state names.
const FRAMES: [&str; 10] = ["f0", "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9"];

fn no_power(group: Group, index: usize) -> Option<Drawing> {
    use art::no_power::*;
    let svg = match group {
        ZOOM1 => svg!("dialogs/no_power/zoom1"),
        ZOOM2 => svg!("dialogs/no_power/zoom2"),
        ZOOM3 => svg!("dialogs/no_power/zoom3"),
        ZOOM4 => svg!("dialogs/no_power/zoom4"),
        _ => return None,
    };
    shown(svg, FRAMES[index % 10])
}

fn thoughts(group: Group) -> Option<Drawing> {
    use art::thoughts::*;
    match group {
        HUNGER => still(svg!("hud/thoughts/hunger")),
        ANGER => still(svg!("hud/thoughts/anger")),
        COLD => still(svg!("hud/thoughts/cold")),
        PEE => still(svg!("hud/thoughts/pee")),
        HAPPINESS => still(svg!("hud/thoughts/happiness")),
        ALERT => still(svg!("hud/thoughts/alert")),
        THIRSTY => still(svg!("hud/thoughts/thirsty")),
        TIRED => still(svg!("hud/thoughts/tired")),
        _ => None,
    }
}

/// The mini-map's filter buttons' frame.
const FILTER: Svg = svg!("hud/minimap/filter");

fn mini_map(group: Group, index: usize) -> Option<Drawing> {
    use art::mini_map::*;
    match group {
        BACKGROUND => still(svg!("hud/minimap/window")),
        RANDOM_GEN => button(svg!("hud/minimap/generate"), index),
        FILTER_STRUCTS => button(svg!("hud/minimap/filter_structures" on FILTER), index),
        FILTER_SKIERS => button(svg!("hud/minimap/filter_skiers" on FILTER), index),
        FILTER_LIFTS => button(svg!("hud/minimap/filter_lifts" on FILTER), index),
        FILTER_ALL => button(svg!("hud/minimap/filter_terrain" on FILTER), index),
        _ => None,
    }
}

/// The escape menu's buttons, drawn as the main menu's are: a picture and
/// label over its ring and pill. Its background is the front end's
/// mountain scene.
fn escape(group: Group, index: usize) -> Option<Drawing> {
    use art::escape::*;
    use super::front_end::MAIN_BUTTON;
    match group {
        SAVE_GAME => button(svg!("menus/escape_menu/save" on MAIN_BUTTON), index),
        ABORT_GAME => button(svg!("menus/escape_menu/abort" on MAIN_BUTTON), index),
        RESUME_GAME => button(svg!("menus/escape_menu/resume" on MAIN_BUTTON), index),
        _ => None,
    }
}

/// Bob's window and button. The portrait in the window is the original
/// painting, which `ui::bob` draws over the window; the OK button is
/// the front end's.
fn bob(group: Group, index: usize) -> Option<Drawing> {
    use art::bob::*;
    match group {
        BACKGROUND => still(svg!("dialogs/bob/window")),
        OK => button(widgets::OK, index),
        BOB_BUTTON => button(svg!("dialogs/bob/button"), index),
        BOB_BUTTON_NORMAL => button(svg!("dialogs/bob/button"), index),
        BOB_BUTTON_ROLLOVER => shown(svg!("dialogs/bob/button"), "rollover"),
        BOB_BUTTON_CLICKED => shown(svg!("dialogs/bob/button"), "depress"),
        _ => None,
    }
}

fn fog(group: Group, index: usize) -> Option<Drawing> {
    use art::fog::*;
    match group {
        BACKGROUND => still(svg!("dialogs/fog/window")),
        FOG100 => button(svg!("dialogs/fog/dense"), index),
        FOG250 => button(svg!("dialogs/fog/medium"), index),
        FOG500 => button(svg!("dialogs/fog/sparse"), index),
        NO_FOG => button(svg!("dialogs/fog/none"), index),
        _ => None,
    }
}
