//! The windows for one structure, lift, run marker, guest or employee
//! (`structure_window`, `lift_window`, `runmarker_window`, `guest_window`
//! and `staff_window`; the staff window is also the employees list),
//! drawn from `art/windows/objects/`: their overview and stats panels,
//! picture tabs, open and closed buttons, the buttons of their own, and a
//! guest's skill sign.
//!
//! Each screen file gets one function here, named after its `art`
//! module, that matches its groups the way `hud` in `vector.rs` does,
//! and `element` calls it for that file. The shared widgets are mapped
//! in `widgets` for every file; an arm here takes precedence, so it can
//! claim the elements of a group that are the screen's own art.
//!
//! Some pictures here are shared with other windows (the buildings, runs,
//! jumps and guest tabs in `art/windows/tabs/`, and the guests window's
//! graph bar in `art/windows/widgets/`), so they are public to the other
//! families.

use super::{Drawing, Svg, button, shown, still, widgets};
use crate::ui::art::{self, ArtFile, Group};

/// Overview panels: a building's (state `priced` with the price box,
/// `plain` without), a lift's, run's or terrain park's, and a guest's.
const STRUCTURE_OVERVIEW: Svg = svg!("windows/objects/structure_overview");
const LIFT_OVERVIEW: Svg = svg!("windows/objects/lift_overview");
const GUEST_OVERVIEW: Svg = svg!("windows/objects/guest_overview");
/// Stats panels: label and value boxes in rows.
const STRUCTURE_STATS: Svg = svg!("windows/objects/structure_stats");
const LIFT_STATS: Svg = svg!("windows/objects/lift_stats");
const GUEST_STATS: Svg = svg!("windows/objects/guest_stats");
/// A building's, lift's or run's open and closed buttons (Bldg_Open,
/// Bldg_close).
const OPEN: Svg = svg!("windows/objects/open");
const CLOSED: Svg = svg!("windows/objects/closed");
/// A condominium's or hotel's upgrade button.
const UPGRADE: Svg = svg!("windows/objects/upgrade");
/// A radio tower's chosen song (Highlight).
const HIGHLIGHT: Svg = svg!("windows/objects/highlight");
/// A guest's skill sign: states `beginner`, `intermediate`, `advanced`
/// and `expert`.
const SKILL: Svg = svg!("windows/objects/skill");
/// Round buttons of these windows' own.
const HIRE: Svg = svg!("windows/objects/hire" on widgets::ROUND);
const PLAY: Svg = svg!("windows/objects/play" on widgets::ROUND);
const STOP: Svg = svg!("windows/objects/stop" on widgets::ROUND);
const HAND_OF_GOD: Svg = svg!("windows/objects/hand_of_god" on widgets::ROUND);
/// The employee window's wooden buttons, with the round buttons'
/// pictures.
const STAFF_BUTTON: Svg = svg!("windows/objects/staff_button");
const STAFF_RENAME: Svg = svg!("windows/widgets/rename" on STAFF_BUTTON);
const STAFF_FOCUS: Svg = svg!("windows/widgets/focus" on STAFF_BUTTON);
const STAFF_PATSEYE: Svg = svg!("windows/widgets/patseye" on STAFF_BUTTON);
const STAFF_SELL: Svg = svg!("windows/widgets/sell" on STAFF_BUTTON);

/// Picture tabs shared with other windows: buildings (Tab_Buildings),
/// runs (Tab_Runs), jumps (Tab_Jumps), a guest group (Tab_GuestGroup), a
/// guest's or the guests' stats (Tab_GuestStats, Tab_GuestAvg) and a
/// guest's or one guest's history (Tab_GuestHistory, Tab_GuestIndivid).
pub(super) const BUILDINGS: Svg = svg!("windows/tabs/buildings" on widgets::ICON_TAB);
pub(super) const RUNS: Svg = svg!("windows/tabs/runs" on widgets::ICON_TAB);
pub(super) const JUMPS: Svg = svg!("windows/tabs/jumps" on widgets::ICON_TAB);
pub(super) const GUEST_GROUP: Svg = svg!("windows/tabs/guest_group" on widgets::ICON_TAB);
pub(super) const GUEST_STATS_TAB: Svg = svg!("windows/tabs/guest_stats" on widgets::ICON_TAB);
pub(super) const GUEST_HISTORY: Svg = svg!("windows/tabs/guest_history" on widgets::ICON_TAB);
/// The guests window's graph bar (Graph_Bar).
pub(super) const GRAPH_BAR: Svg = svg!("windows/widgets/graph_bar");
/// These windows' own picture tabs.
const TAB_GUEST: Svg = svg!("windows/objects/tabs/guest" on widgets::ICON_TAB);
const TAB_BUILDING_STATS: Svg = svg!("windows/objects/tabs/building_stats" on widgets::ICON_TAB);
const TAB_BUILDING_GUESTS: Svg = svg!("windows/objects/tabs/building_guests" on widgets::ICON_TAB);
const TAB_BUILDING_STAFF: Svg = svg!("windows/objects/tabs/building_staff" on widgets::ICON_TAB);
const TAB_MUSIC: Svg = svg!("windows/objects/tabs/music" on widgets::ICON_TAB);
const TAB_SKI_PATROL: Svg = svg!("windows/objects/tabs/ski_patrol" on widgets::ICON_TAB);
const TAB_AVALANCHE: Svg = svg!("windows/objects/tabs/avalanche" on widgets::ICON_TAB);
const TAB_INSTRUCTOR: Svg = svg!("windows/objects/tabs/instructor" on widgets::ICON_TAB);
const TAB_MAINTENANCE: Svg = svg!("windows/objects/tabs/maintenance" on widgets::ICON_TAB);
const TAB_CATERING: Svg = svg!("windows/objects/tabs/catering" on widgets::ICON_TAB);
const TAB_SANITATION: Svg = svg!("windows/objects/tabs/sanitation" on widgets::ICON_TAB);
const TAB_SECURITY: Svg = svg!("windows/objects/tabs/security" on widgets::ICON_TAB);

/// The drawing for an element of one of these screen files, if it has
/// one of its own.
pub(super) fn element(file: ArtFile, group: Group, index: usize) -> Option<Drawing> {
    match file {
        art::structure_window::FILE => structure_window(group, index),
        art::lift_window::FILE => lift_window(group, index),
        art::runmarker_window::FILE => runmarker_window(group, index),
        art::guest_window::FILE => guest_window(group, index),
        art::staff_window::FILE => staff_window(group, index),
        art::scaled_stats_bar::FILE if group == art::scaled_stats_bar::GRAPH => still(GRAPH_BAR),
        _ => None,
    }
}

/// A picture tab: the group's third element (`..._Normal`) is the tab in
/// front, the first two (`..._Depress`) one behind.
fn tab(svg: Svg, index: usize) -> Option<Drawing> {
    shown(svg, if index >= 2 { "normal" } else { "depress" })
}

fn structure_window(group: Group, index: usize) -> Option<Drawing> {
    use art::structure_window::*;
    match group {
        // The overview with the price box, then the stats; the list tabs'
        // panels are the shared list panel.
        BACKGROUND => match index {
            0 => shown(STRUCTURE_OVERVIEW, "priced"),
            1 => still(STRUCTURE_STATS),
            _ => None,
        },
        // Buildings that sell nothing: the overview without it. The lift
        // panels' first is the same art; their stats (a wooden panel) are
        // never shown.
        BACKGROUND_NOCOST | BACKGROUND_LIFTS if index == 0 => shown(STRUCTURE_OVERVIEW, "plain"),
        BACKGROUND_NOCOST if index == 1 => still(STRUCTURE_STATS),
        MAIN_TAB => tab(BUILDINGS, index),
        STATS_TAB => tab(TAB_BUILDING_STATS, index),
        GUEST_TAB => tab(TAB_BUILDING_GUESTS, index),
        STAFF_TAB => tab(TAB_BUILDING_STAFF, index),
        MUSIC_TAB => tab(TAB_MUSIC, index),
        OPEN_BUILDING => button(OPEN, index),
        CLOSE_BUILDING => button(CLOSED, index),
        UPGRADE => button(self::UPGRADE, index),
        HIRE => button(self::HIRE, index),
        PLAY => button(self::PLAY, index),
        STOP => button(self::STOP, index),
        HIGHLIGHT => still(self::HIGHLIGHT),
        _ => None,
    }
}

fn lift_window(group: Group, index: usize) -> Option<Drawing> {
    use art::lift_window::*;
    match group {
        BACKGROUND => match index {
            0 => still(LIFT_OVERVIEW),
            1 => still(LIFT_STATS),
            _ => None,
        },
        MAIN_TAB => tab(BUILDINGS, index),
        STATS_TAB => tab(TAB_BUILDING_STATS, index),
        GUEST_TAB => tab(TAB_BUILDING_GUESTS, index),
        OPEN_BUILDING => button(OPEN, index),
        CLOSE_BUILDING => button(CLOSED, index),
        _ => None,
    }
}

fn runmarker_window(group: Group, index: usize) -> Option<Drawing> {
    use art::runmarker_window::*;
    match group {
        BACKGROUND => still(LIFT_OVERVIEW),
        MARKER_TAB => tab(RUNS, index),
        JUMP_TAB => tab(JUMPS, index),
        OPEN_BUILDING => button(OPEN, index),
        CLOSE_BUILDING => button(CLOSED, index),
        _ => None,
    }
}

fn guest_window(group: Group, index: usize) -> Option<Drawing> {
    use art::guest_window::*;
    match group {
        BACKGROUND => match index {
            0 => still(GUEST_OVERVIEW),
            1 => still(self::GUEST_STATS),
            _ => None,
        },
        ONE_GUEST_MAIN => tab(TAB_GUEST, index),
        GUEST_STATS_TAB => tab(self::GUEST_STATS_TAB, index),
        GUEST_THOUGHTS_TAB => tab(GUEST_HISTORY, index),
        GUEST_GROUP_TAB => tab(GUEST_GROUP, index),
        EMP_SKI_PATROL_TAB => tab(TAB_SKI_PATROL, index),
        EMP_AVALANCHE_TAB => tab(TAB_AVALANCHE, index),
        EMP_INSTRUCTOR_TAB => tab(TAB_INSTRUCTOR, index),
        EMP_MAINTENANCE_TAB => tab(TAB_MAINTENANCE, index),
        EMP_CATERING_TAB => tab(TAB_CATERING, index),
        EMP_SANITATION_TAB => tab(TAB_SANITATION, index),
        EMP_SECURITY_TAB => tab(TAB_SECURITY, index),
        // The employee window moves its employee with this one too.
        HAND_OF_GOD => button(self::HAND_OF_GOD, index),
        SKILL => shown(self::SKILL, ["beginner", "intermediate", "advanced", "expert"][index.min(3)]),
        _ => None,
    }
}

fn staff_window(group: Group, index: usize) -> Option<Drawing> {
    use art::staff_window::*;
    match group {
        TAB_SKIPATROL => tab(TAB_SKI_PATROL, index),
        TAB_AVALANCHE => tab(self::TAB_AVALANCHE, index),
        TAB_INSTRUCTOR => tab(self::TAB_INSTRUCTOR, index),
        TAB_MAINTENANCE => tab(self::TAB_MAINTENANCE, index),
        TAB_CATERING => tab(self::TAB_CATERING, index),
        TAB_SANITATION => tab(self::TAB_SANITATION, index),
        TAB_SECURITY => tab(self::TAB_SECURITY, index),
        // The employee window's buttons: its sell button fires.
        RENAME => button(STAFF_RENAME, index),
        FOCUS => button(STAFF_FOCUS, index),
        PATSEYE => button(STAFF_PATSEYE, index),
        SELL => button(STAFF_SELL, index),
        _ => None,
    }
}
