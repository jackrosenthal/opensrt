//! `SRT_HUD_OPEN`: what to set up when a game starts, for taking
//! screenshots of particular screens. Parsed once from the variable.

use crate::ui::hud::Nav;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestHook {
    /// `build`, `terrain` or `finances`, optionally `/<tray>`.
    Menu(Nav, Option<usize>),
    /// `pick/<category>`: the building picker.
    Picker(usize),
    /// `stats/<window>[/<tab>[/weather]]`; `weather` builds a weather
    /// station first.
    Stats { window: usize, tab: Option<usize>, weather: bool },
    /// Hires a maintenance worker and selects them.
    Staff,
    /// Builds a radio tower and selects it.
    Radio,
    /// Builds the first upgradeable building and selects it.
    Upgrade,
    /// Night; `night/built` lays out a starter resort first.
    Night(bool),
    Minimap,
    /// Selects the entrance.
    Select,
    /// Breaks the entrance down, so it shows the wrench.
    Broken,
    /// Strews litter and yellow snow in front of the entrance.
    Dirt,
    /// Builds a snowmobile patrol, a chopper patrol base, a snowmobile
    /// tour, a groomer shop, both schools, and avalanche control.
    Vehicles,
    /// Builds a starter resort, runs it a day, lays the guest nearest the
    /// ski patrol up hurt, and looks at them up close.
    Rescue,
    /// Builds a starter resort, runs it until someone skis, and follows
    /// them up close.
    Follow,
    /// The player's own skier, put down on the starter resort's slope and
    /// ridden in the follow view.
    Ride,
    /// Lays a few footpaths (a line, a corner, a T and a cross) in front
    /// of the entrance.
    Paths,
    /// `surface/<n>`: takes up a surface tool, 0 hill, 1 peak, 2 flat
    /// raise, 3 single point, 4 smooth, 5 moguls, 6 snow, 7 groomer.
    Surface(usize),
    /// `runpath/<mode>`: takes up a Path Skirun tool (0 follow, 1 cut,
    /// 2 cut and fill).
    RunPath(u8),
    Escape,
    Save,
    /// Auto-builds a starter resort and selects its first run (`run`),
    /// lift (`lift`) or park, or runs it until a guest arrives and
    /// selects them (`guest`).
    Starter(StarterPick),
}

impl TestHook {
    pub fn from_env() -> Option<TestHook> {
        let v = std::env::var("SRT_HUD_OPEN").ok()?;
        let hook = TestHook::parse(&v);
        if hook.is_none() {
            log::warn!("SRT_HUD_OPEN={v:?} not understood");
        }
        hook
    }

    fn parse(s: &str) -> Option<TestHook> {
        let mut parts = s.split('/');
        let head = parts.next()?;
        let num = |p: Option<&str>| p.and_then(|t| t.parse::<usize>().ok());
        Some(match head {
            "build" => TestHook::Menu(Nav::Build, num(parts.next())),
            "terrain" => TestHook::Menu(Nav::Terrain, num(parts.next())),
            "finances" => TestHook::Menu(Nav::Finances, num(parts.next())),
            "pick" => TestHook::Picker(num(parts.next())?),
            "stats" => TestHook::Stats {
                window: num(parts.next())?,
                tab: num(parts.next()),
                weather: parts.next() == Some("weather"),
            },
            "staff" => TestHook::Staff,
            "radio" => TestHook::Radio,
            "upgrade" => TestHook::Upgrade,
            "night" => TestHook::Night(parts.next() == Some("built")),
            "minimap" => TestHook::Minimap,
            "select" => TestHook::Select,
            "broken" => TestHook::Broken,
            "dirt" => TestHook::Dirt,
            "paths" => TestHook::Paths,
            "vehicles" => TestHook::Vehicles,
            "rescue" => TestHook::Rescue,
            "follow" => TestHook::Follow,
            "ride" => TestHook::Ride,
            "surface" => TestHook::Surface(num(parts.next())?),
            "runpath" => TestHook::RunPath(num(parts.next())? as u8),
            "escape" => TestHook::Escape,
            "save" => TestHook::Save,
            "run" => TestHook::Starter(StarterPick::Run),
            "lift" => TestHook::Starter(StarterPick::Lift),
            "guest" => TestHook::Starter(StarterPick::Guest),
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterPick {
    Run,
    Lift,
    Guest,
}
