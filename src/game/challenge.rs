//! Challenges, as the original runs them (0x414960): each a numbered
//! step machine. Some steps show Bob with a message and wait for OK
//! (which moves on a step); others watch for a goal, a deadline or a
//! warning date and jump to the step for it. Waiting steps show the Bob
//! button, which brings the last message back. Most texts come from the
//! string tables; a few challenges have theirs written into the program.

use serde::{Deserialize, Serialize};

use super::defs::Defs;
use super::world::World;
use crate::formats::level::cell_flags;
use crate::text::Text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    InProgress,
    Won,
    Lost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeState {
    pub number: usize,
    #[serde(default)]
    pub step: u32,
    /// Things already reported (0xa0): Olympic courses built, warnings
    /// given.
    #[serde(default)]
    flags: u32,
    /// How many Olympic courses had been built when last reported (0xa2).
    #[serde(default)]
    courses: u8,
}

/// Bob's look, the index of his background in `bob.bin`.
pub type Mood = u8;
pub const HAPPY: Mood = 0;
pub const NORMAL: Mood = 1;
pub const POINTING: Mood = 2;
pub const UNHAPPY: Mood = 3;

/// What Bob says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Say {
    Text(Text),
    /// Text written into the program rather than the string tables.
    Builtin(&'static str),
    /// Which Olympic courses are still to build (0x4136e0 step 4).
    OlympicProgress { built: u32, count: u8 },
}

/// A message step: Bob's look and words, and whether the game waits for
/// OK (the original stops the world's tick for these).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prompt {
    pub mood: Mood,
    pub say: Say,
    pub pauses: bool,
}

const FIVE_STAR_INTRO: &str = "Aspen, Colorado. A Five Star ski resort. Well, after you have turned it into one! You need to build this resort into the epitome of ski resorts.\n\nYou must achieve a 5 star rating in less than 35 weeks.\n\nHINT: The Condomiums and the Medium Hotel can be upgraded to accomodate more guests. This saves money and space compared to building another complete hotel. To upgrade them, right-click on the building and push the 'Upgrade' button.";
const CASH_INTRO: &str = "You have to build a huge resort to gain 100,000 dollars. Make sure to set the right prices for your customers so that you are rewarded for your investments.\n\nYou have 40 weeks for this challenge.\n\nHINT: Adjust the entry price slowly as your resort grows. As soon as many people complain, you know that you have passed the critical threshold. Do the same with the other prices.";
const CASH_DONE: &str = "Magnificent! Now you are a true Tycoon.";
const CASH_WARNING: &str = "Time is running out! Remember you have 40 weeks for this challenge!";
const CASH_FAILED: &str = "You have failed... Try again!";
const PARK_INTRO: &str = "There is a nice spot up in the mountains for our new terrain park. Build a Half Pipe, Jump, and a Race course - and other buildings to make your guess happy. After you have received 50 visitors, your task is completed.\n\n You have 30 weeks for this challenge.\n\nHINT: Know your guests! Use the Finances button, then push the Guest Statistics button and choose the 'grouped Guest Status' tab. It shows you all of your guests' current demands.";
const PARK_DONE: &str = "Congratulations! More than 50 people are enjoying your new terrain park! \n\n *** Challenge Completed ***";
const PARK_WARNING: &str = "Time is running out! Remember you have 30 weeks to build the terrain park and receive 50 visitors!";
const OUT_OF_TIME: &str = "You're out of time! Looks like you need a little more practice... try this challenge again!!\n\n *** Challenge Failed ***";
const GUESTS_INTRO: &str = "Welcome to Kitzbuehel, a place that is meant to become one of Europe's most famous ski resorts.\n\nOur investors wish to see at least 500 visitors in this resort after only 45 weeks.\n\nHINT: If you build a Marketing building, many more guests will be visiting your resort.";
const GUESTS_DONE: &str = "Congratulations! I knew that you would make it. Now look at all these skiers in your beautiful resort. This must be Kitzbuehel as it was always meant to be! \n\n *** Challenge Completed ***";
const GUESTS_WARNING: &str = "Time is running out! Make sure to reach these 500 guests before 45 weeks are over!";
const BOARD_INTRO: &str = "It's up to you to turn this mountain into the place to be for snow boarders from around the world.\n\nBuild a Board Rental, a Snow Board School, and a Half Pipe, and then turn this into a 3 star resort You have 40 weeks. Go for it!\n\nHINT: Few large buildings help more than many small ones, as their relative running costs are much lower.";
const BOARD_DONE: &str = "I knew you would make it. Our investors are happy and can't wait to see you building up the next world class resort!\n\n *** Challenge Completed ***";
const BOARD_WARNING: &str = "Time is running out! Make sure to build the Board Rental, a Snow Board School, and a Half Pipe, and then turn this into a 3 star resort before the 40 weeks are over!";

/// The stars challenges and their like: one goal by a deadline, with a
/// warning on the way (steps 0 intro, 1 watch, 2 done, 4 warning, 6
/// failed).
struct Timed {
    intro: (Mood, Say),
    done: Say,
    warning: Say,
    failed: Say,
    warn_after: u32,
    fail_after: u32,
}

fn timed(number: usize) -> Option<Timed> {
    use Say::{Builtin as B, Text as T};
    let t = |intro, done, warning, failed, warn_after, fail_after| {
        Some(Timed { intro: (NORMAL, intro), done, warning, failed, warn_after, fail_after })
    };
    match number {
        3 => t(B(FIVE_STAR_INTRO), T(Text::FivestarMsgDone), T(Text::FivestarMsgWarning), T(Text::FivestarMsgFailed), 0xd1, 0xf5),
        8 => t(T(Text::TwostarMsgIntro), T(Text::TwostarMsgDone), T(Text::TwostarMsgWarning), T(Text::TwostarMsgFailed), 0x53, 0x62),
        9 => t(T(Text::ThreestarMsgIntro), T(Text::ThreestarMsgDone), T(Text::ThreestarMsgWarning), T(Text::ThreestarMsgFailed), 0x77, 0x8c),
        10 => t(B(CASH_INTRO), B(CASH_DONE), B(CASH_WARNING), B(CASH_FAILED), 0xef, 0x118),
        11 => t(B(GUESTS_INTRO), B(GUESTS_DONE), B(GUESTS_WARNING), B(OUT_OF_TIME), 0x10d, 0x13b),
        12 => t(B(PARK_INTRO), B(PARK_DONE), B(PARK_WARNING), B(OUT_OF_TIME), 0xb3, 0xd2),
        13 => t(B(BOARD_INTRO), B(BOARD_DONE), B(BOARD_WARNING), B(OUT_OF_TIME), 0xef, 0x118),
        _ => None,
    }
}

const WARNED: u32 = 8;

impl ChallengeState {
    pub fn new(number: usize) -> ChallengeState {
        ChallengeState { number, step: 0, flags: 0, courses: 0 }
    }

    /// The message for the current step, if it is a message step.
    pub fn prompt(&self) -> Option<Prompt> {
        use Say::Text as T;
        let p = |mood, say| Some(Prompt { mood, say, pauses: true });
        let warn = |mood, say| Some(Prompt { mood, say, pauses: false });
        match (self.number, self.step) {
            (1, s) => {
                let (mood, text) = match s {
                    0 => (HAPPY, Text::TutMsgIntro),
                    2 => (HAPPY, Text::TutMsgGenerator),
                    4 => (HAPPY, Text::TutMsgGrid),
                    6 => (NORMAL, Text::TutMsgTrees),
                    8 => (HAPPY, Text::TutMsgRopetow),
                    10 => (NORMAL, Text::TutMsgMarker),
                    12 => (POINTING, Text::TutMsgShack),
                    14 => (POINTING, Text::TutMsgRental),
                    16 => (NORMAL, Text::TutMsgFood),
                    18 => (POINTING, Text::TutMsgPee),
                    20 => (NORMAL, Text::TutMsgWarming),
                    22 => (NORMAL, Text::TutMsgLodge),
                    24 => (HAPPY, Text::TutMsgDone),
                    _ => return None,
                };
                p(mood, T(text))
            }
            (2, 0) => p(POINTING, T(Text::SummitMsgIntro)),
            (2, 2) => p(HAPPY, T(Text::SummitMsgDone)),
            (4, 0) => p(NORMAL, T(Text::OlympicMsgIntro)),
            (4, 2) => p(HAPPY, T(Text::OlympicMsgDone)),
            (4, 4) => warn(HAPPY, Say::OlympicProgress { built: self.flags & 7, count: self.courses }),
            (4, 6) => match self.courses {
                0 => p(UNHAPPY, T(Text::OlympicMsgWarning1)),
                1 => warn(POINTING, T(Text::OlympicMsgWarning2)),
                _ => warn(NORMAL, T(Text::OlympicMsgWarning3)),
            },
            (4, 8) => p(UNHAPPY, T(Text::OlympicMsgFailed)),
            (5, 0) => p(NORMAL, T(Text::AvaMsgIntro)),
            (5, 2) => p(HAPPY, T(Text::AvaMsgDone)),
            (5, 6) => p(UNHAPPY, T(Text::AvaMsgOneWeek)),
            (5, 8) => p(UNHAPPY, T(Text::AvaMsgLose)),
            (6, 0) => p(UNHAPPY, T(Text::SquatchMsgIntro)),
            (6, 2) => p(HAPPY, T(Text::SquatchMsgDone)),
            (7, 1) => p(UNHAPPY, T(Text::CleanupMsgIntro)),
            (7, 3) => p(HAPPY, T(Text::CleanupMsgDone)),
            (7, 6) => p(UNHAPPY, T(Text::CleanupMsgFailed)),
            (7, 8) => p(UNHAPPY, T(Text::CleanupMsgOuttacash)),
            (n, s) => {
                let t = timed(n)?;
                match s {
                    0 => p(t.intro.0, t.intro.1),
                    2 => p(HAPPY, t.done),
                    4 => warn(UNHAPPY, t.warning),
                    6 => p(UNHAPPY, t.failed),
                    _ => None,
                }
            }
        }
    }

    /// OK on the message for `step`: on to the next step.
    pub fn acknowledge(&mut self, step: u32) {
        if self.step == step {
            self.step += 1;
        }
    }

    /// Whether the challenge is watching for something, so the Bob button
    /// shows.
    pub fn waiting(&self) -> bool {
        self.outcome() == Outcome::InProgress && self.prompt().is_none() && self.watch_step()
    }

    fn watch_step(&self) -> bool {
        match self.number {
            1 => self.step % 2 == 1 && self.step < 24,
            7 => self.step == 2,
            2 | 3 | 4 | 5 | 6 | 8..=13 => self.step == 1,
            _ => false,
        }
    }

    /// The Bob button: back to the message before.
    pub fn recall(&mut self) {
        if self.waiting() && self.step > 0 {
            self.step -= 1;
        }
    }

    pub fn outcome(&self) -> Outcome {
        let (won, lost): (&[u32], &[u32]) = match self.number {
            1 => (&[25], &[]),
            2 | 6 => (&[3], &[]),
            4 | 5 => (&[3], &[9]),
            7 => (&[4], &[7, 9]),
            n if timed(n).is_some() => (&[3], &[7]),
            _ => (&[], &[]),
        };
        if won.contains(&self.step) {
            Outcome::Won
        } else if lost.contains(&self.step) {
            Outcome::Lost
        } else {
            Outcome::InProgress
        }
    }

    /// How rarely a sasquatch carries off a skier, in ticks: the
    /// sasquatch challenge sets it (0x414960 case 6).
    pub fn abduction_interval(&self) -> Option<f32> {
        match (self.number, self.step) {
            (6, 0..=2) => Some(300.0),
            (6, _) => Some(9000.0),
            _ => None,
        }
    }

    /// Watches for the current step's goal or dates. `grid` is whether
    /// the cell grid is showing, which the tutorial asks for.
    pub fn update(&mut self, w: &World, defs: &Defs, grid: bool) {
        self.pass_through();
        if !self.watch_step() {
            return;
        }
        let built = |name: &str| w.has_built(defs, &defs.requirement(name));
        let day = w.day;
        let score = w.rating.total;
        let s = self.step;
        match self.number {
            1 => self.step += self.tutorial_progress(w, grid, &built),
            2 if built("High Speed Quad") => self.step += 1,
            6 if built("Wildlife Control") => self.step += 1,
            4 => self.olympic(day, &built),
            5 => {
                if score > 700.0 {
                    self.step += 1;
                }
                if day > 0x68 && self.flags & 4 == 0 {
                    self.flags |= 4;
                    self.step = 6;
                }
                if day > 0x70 {
                    self.step = 8;
                }
            }
            7 => {
                if w.litter.len() < 5 {
                    self.step += 1;
                }
                if day > 0x70 {
                    self.step = 6;
                }
                if w.cash < 0.0 {
                    self.step = 8;
                }
            }
            n => {
                let Some(t) = timed(n) else { return };
                let met = match n {
                    3 => score > 900.0,
                    8 => score > 300.0,
                    9 => score > 500.0,
                    10 => w.cash >= 100_000.0,
                    11 => w.active_guests() > 499,
                    12 => built("HalfPipe") && built("Jump") && built("RaceCourse") && w.active_guests() > 0x31,
                    _ => built("Board Rentals") && built("Snow Board School") && built("HalfPipe") && score > 500.0,
                };
                if met {
                    self.step += 1;
                }
                if day > t.warn_after && self.flags & WARNED == 0 {
                    self.flags |= WARNED;
                    self.step = 4;
                }
                if day > t.fail_after {
                    self.step = 6;
                }
            }
        }
        if self.step != s {
            log::info!("challenge {} step {s} -> {}", self.number, self.step);
        }
    }

    /// Steps the original passes straight through: the tutorial's second,
    /// a warning's follow-up back to watching.
    fn pass_through(&mut self) {
        let to = match (self.number, self.step) {
            (1, 1) => Some(2),
            (4 | 5, 5 | 7) => Some(1),
            (n, 5) if timed(n).is_some() => Some(1),
            _ => None,
        };
        if let Some(s) = to {
            self.step = s;
        }
    }

    /// The tutorial's goals (0x4128b0). Several of a kind each count a
    /// step, as in the original.
    fn tutorial_progress(&self, w: &World, grid: bool, built: &dyn Fn(&str) -> bool) -> u32 {
        let any = |names: &[&str]| names.iter().filter(|n| built(n)).count() as u32;
        match self.step {
            3 => any(&["Generator"]),
            5 => grid as u32,
            7 => {
                // The cells from (6, 6) to (11, 11), by the entrance,
                // cleared of trees.
                let lv = w.level();
                let clear = (6..12).all(|y| (6..12).all(|x| lv.cell(x, y).flags & cell_flags::FOREST == 0));
                clear as u32
            }
            9 => any(&["Ropetow"]),
            11 => any(&["Beginner Run", "Intermediate Run", "Advanced Run", "Expert Run"]),
            13 => any(&["Maintenance Shack"]),
            15 => any(&["Ski Rentals"]),
            17 => any(&["Espresso House", "Hot Dog Stand", "Fast Food", "Snack Shack", "East Slope Bistro", "Grocery Store"]),
            19 => any(&["Portable Toilet", "Single Service Restroom", "Large Restroom"]),
            21 => any(&["Small Warming Hut"]),
            23 => any(&["Small Cabin", "Medium cabin", "Deluxe Family Cabin"]),
            _ => 0,
        }
    }

    /// The Olympic courses (0x4136e0): each new one reported, all three to
    /// win.
    fn olympic(&mut self, day: u32, built: &dyn Fn(&str) -> bool) {
        let mut count = 0u8;
        for (bit, name) in [(1, "RaceCourse"), (2, "HalfPipe"), (4, "Jump")] {
            if built(name) {
                self.flags |= bit;
                count += 1;
            }
        }
        if count == 3 {
            self.step += 1;
            self.courses = 3;
        } else if count != 0 && count != self.courses {
            self.step = 4;
            self.courses = count;
        }
        if day > 0x68 && self.flags & WARNED == 0 {
            self.flags |= WARNED;
            self.step = 6;
        }
        if day > 0x70 {
            self.step = 8;
        }
    }
}

/// Level changes when a challenge starts: the clean-up challenge strews
/// litter on one cell in ten outside the forest (0x413a90 step 0).
pub fn setup(number: usize, w: &mut World) {
    if let Some(c) = w.challenge_state.as_mut() {
        if c.number == 7 && c.step == 0 {
            c.step = 1;
        }
    }
    if number != 7 {
        return;
    }
    let n = w.level().cells_per_side();
    let s = w.level().spacing;
    for cy in 0..n {
        for cx in 0..n {
            if w.rand() >= 0.1 || w.level().cell(cx, cy).flags & cell_flags::FOREST != 0 {
                continue;
            }
            let p = w.ground(glam::Vec2::new((cx as f32 + 0.5) * s, (cy as f32 + 0.5) * s));
            w.litter.push(super::world::Litter { pos: p, yellow: false });
        }
    }
}
