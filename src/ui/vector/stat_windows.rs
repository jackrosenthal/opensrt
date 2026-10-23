//! The stat windows (`resort_stats`, `finances_stats`, `guests_stats`
//! and `attractions`), the high-level statistics (`highlevel_stats`) and
//! the graph bar (`scaled_stats_bar`).
//!
//! Each screen file gets one function here, named after its `art`
//! module, that matches its groups the way `hud` in `vector.rs` does,
//! and `element` calls it for that file. The shared widgets are mapped
//! in `widgets` for every file; an arm here takes precedence, so it can
//! claim the elements of a group that are the screen's own art.
//!
//! The windows' picture tabs are pictures over `widgets::ICON_TAB`: a
//! tab's `..._Depress` group is the tab behind (`depress`, its picture
//! dimmed) and its `..._Normal` (`_ani`) group the tab in front
//! (`normal`). The graph bar's only sprite and the guests and
//! attractions windows' other tabs are drawn by the object windows.

use super::widgets::{ICON_TAB, ROUND};
use super::{Drawing, Svg, button, shown, still};
use crate::ui::art::{self, ArtFile, Group};

/// The drawing for an element of one of these screen files, if it has
/// one of its own.
pub(super) fn element(file: ArtFile, group: Group, index: usize) -> Option<Drawing> {
    match file {
        art::resort_stats::FILE => resort_stats(group, index),
        art::finances_stats::FILE => finances_stats(group, index),
        art::guests_stats::FILE => guests_stats(group, index),
        art::attractions::FILE => attractions(group),
        art::highlevel_stats::FILE => highlevel_stats(group, index),
        _ => None,
    }
}

/// A picture tab: behind the others, or in front.
fn tab(svg: Svg, front: bool) -> Option<Drawing> {
    shown(svg, if front { "normal" } else { "depress" })
}

fn resort_stats(group: Group, index: usize) -> Option<Drawing> {
    use art::resort_stats::*;
    // The wind dial's sixteen elements show its eight headings in pairs.
    const HEADINGS: [&str; 8] = ["n", "ne", "e", "se", "s", "sw", "w", "nw"];
    match group {
        // The history tab's panel is the plain list panel, in `widgets`.
        BACKGROUND => match index {
            0 => still(svg!("stat_windows/overview")),
            1 => still(svg!("stat_windows/admissions")),
            2 => still(svg!("stat_windows/rating")),
            3 => still(svg!("stat_windows/weather")),
            _ => None,
        },
        TAB_MAIN | TAB_MAIN_ANI => tab(svg!("stat_windows/tabs/overview" on ICON_TAB), group == TAB_MAIN_ANI),
        TAB_GUESTS | TAB_GUESTS_ANI => tab(svg!("stat_windows/tabs/admissions" on ICON_TAB), group == TAB_GUESTS_ANI),
        TAB_RATE | TAB_RATE_ANI => tab(svg!("stat_windows/tabs/rating" on ICON_TAB), group == TAB_RATE_ANI),
        TAB_WEATHER | TAB_WEATHER_ANI => tab(svg!("stat_windows/tabs/weather" on ICON_TAB), group == TAB_WEATHER_ANI),
        TAB_HISTORY | TAB_HISTORY_ANI => tab(svg!("stat_windows/tabs/history" on ICON_TAB), group == TAB_HISTORY_ANI),
        WIND => shown(svg!("stat_windows/wind"), HEADINGS[(index / 2).min(7)]),
        _ => None,
    }
}

fn finances_stats(group: Group, index: usize) -> Option<Drawing> {
    use art::finances_stats::*;
    match group {
        TITLE_BAR => still(svg!("stat_windows/finances_tray")),
        BACKGROUND => match index {
            0 => still(svg!("stat_windows/income")),
            1 => still(svg!("stat_windows/cash")),
            _ => still(svg!("stat_windows/assets")),
        },
        CHART_TAB | CHART_TAB_ANI => tab(svg!("stat_windows/tabs/chart" on ICON_TAB), group == CHART_TAB_ANI),
        CASH_TAB | CASH_TAB_ANI => tab(svg!("stat_windows/tabs/cash" on ICON_TAB), group == CASH_TAB_ANI),
        ASSET_TAB | ASSET_TAB_ANI => tab(svg!("stat_windows/tabs/assets" on ICON_TAB), group == ASSET_TAB_ANI),
        _ => None,
    }
}

fn guests_stats(group: Group, index: usize) -> Option<Drawing> {
    use art::guests_stats::*;
    match group {
        // The average needs, then the lists' taller list panel.
        BACKGROUND if index == 0 => still(svg!("stat_windows/guest_needs")),
        BACKGROUND => still(svg!("stat_windows/guest_list")),
        THOUGHT => button(svg!("stat_windows/thoughts" on ROUND), index),
        ACTION => button(svg!("stat_windows/actions" on ROUND), index),
        _ => None,
    }
}

fn attractions(group: Group) -> Option<Drawing> {
    use art::attractions::*;
    match group {
        TAB_LIFT | TAB_LIFT_ANI => tab(svg!("stat_windows/tabs/lifts" on ICON_TAB), group == TAB_LIFT_ANI),
        _ => None,
    }
}

fn highlevel_stats(group: Group, index: usize) -> Option<Drawing> {
    use art::highlevel_stats::*;
    const FRAMES: [&str; 10] = ["frame0", "frame1", "frame2", "frame3", "frame4", "frame5", "frame6", "frame7", "frame8", "frame9"];
    let frame = FRAMES[index.min(9)];
    match group {
        // The broken building marker, in four sizes.
        WRENCH_ZOOM1 => shown(svg!("stat_windows/wrench_1"), frame),
        WRENCH_ZOOM2 => shown(svg!("stat_windows/wrench_2"), frame),
        WRENCH_ZOOM3 => shown(svg!("stat_windows/wrench_3"), frame),
        WRENCH_ZOOM4 => shown(svg!("stat_windows/wrench_4"), frame),
        _ => None,
    }
}
