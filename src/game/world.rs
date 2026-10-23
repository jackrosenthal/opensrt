//! World state: everything that is simulated and saved.

use std::collections::VecDeque;

use glam::{Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::defs::{Defs, NeedKind, Needs};
use crate::formats::level::Level;

pub type Id = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Structure {
    pub id: Id,
    /// Index into `Defs::structures`.
    pub def: usize,
    /// Lowest cell corner of the footprint.
    pub cell: (i32, i32),
    /// Quarter turns.
    pub rot: u8,
    pub pos: Vec3,
    pub name: String,
    pub wear: f32,
    pub open: bool,
    pub price: f32,
    pub powered: bool,
    pub occupants: Vec<Id>,
    pub visits_today: u32,
    pub revenue_today: f32,
    pub revenue_total: f32,
    pub built_day: u32,
    pub broken: bool,
    #[serde(default)]
    pub worth: Worth,
    /// The daily charge once the player has set the lift ticket price: a
    /// quarter of it (0x466307). Otherwise `reacuring_cost_per_day`.
    #[serde(default)]
    pub daily_fee: Option<f32>,
    /// A ski school's class: when its first student joined (absolute
    /// hours).
    #[serde(default)]
    pub class_opened: Option<f32>,
    /// Guests in line at the door, first in front.
    #[serde(default)]
    pub waiting: Vec<Id>,
}

/// What a building, lift or park is worth (0x4b0b80, 0x49e7de): what it
/// cost, falling by a hundred-thousandth of that a tick until it is worth
/// half. An upgrade adds its price and starts the fall again from there
/// (0x4b4ca0).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Worth {
    pub now: f32,
    pub base: f32,
}

impl Worth {
    pub fn new(cost: f32) -> Worth {
        Worth { now: cost, base: cost }
    }

    pub fn tick(&mut self, ticks: f32) {
        self.now = (self.now - self.base * 1e-5 * ticks).max(self.base * 0.5);
    }

    pub fn add(&mut self, cost: f32) {
        self.now += cost;
        self.base = self.now;
    }
}

impl Structure {
    /// Footprint in cells after rotation.
    pub fn footprint(&self, defs: &Defs) -> (i32, i32) {
        let (w, h) = defs.structures[self.def].size;
        if self.rot % 2 == 0 { (w, h) } else { (h, w) }
    }

    /// Point where guests enter, on the ground in front of the building.
    pub fn door(&self, defs: &Defs, spacing: f32) -> Vec2 {
        let (w, h) = self.footprint(defs);
        let c = Vec2::new(self.pos.x, self.pos.y);
        let half = Vec2::new(w as f32, h as f32) * spacing * 0.5;
        let dir = match self.rot % 4 {
            0 => Vec2::new(0.0, -1.0),
            1 => Vec2::new(1.0, 0.0),
            2 => Vec2::new(0.0, 1.0),
            _ => Vec2::new(-1.0, 0.0),
        };
        c + dir * (half.abs().dot(dir.abs()) + spacing * 0.6)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chair {
    /// Distance traveled along the loop (0..2*length).
    pub s: f32,
    pub riders: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lift {
    pub id: Id,
    pub def: usize,
    pub name: String,
    pub start: Vec3,
    pub end: Vec3,
    pub poles: Vec<Vec3>,
    pub chairs: Vec<Chair>,
    pub queue: VecDeque<Id>,
    pub wear: f32,
    pub open: bool,
    pub broken: bool,
    pub riders_today: u32,
    pub riders_total: u32,
    pub built_day: u32,
    #[serde(default)]
    pub worth: Worth,
    /// The cable loop from the meshes' attach points, set by the renderer:
    /// up from the start station to the end station, then back.
    #[serde(skip)]
    pub cable_loop: Vec<Vec3>,
    /// Length of the upward half of `cable_loop`.
    #[serde(skip)]
    pub cable_up: f32,
    /// Seconds the machinery has been running, for the mesh animation.
    #[serde(skip)]
    pub anim: f32,
    /// Queue lines on the ground from the boarding point outwards (one, or
    /// two for chairlifts), and the path off the top station.
    #[serde(skip)]
    pub queue_lines: Vec<Vec<Vec3>>,
    #[serde(skip)]
    pub exit_line: Vec<Vec3>,
    /// Where on the loop (in `point_at` units) riders get on and off.
    #[serde(skip)]
    pub board_s: Option<f32>,
    #[serde(skip)]
    pub unload_s: Option<f32>,
    /// Per tower: the cable runs under its wheel, which then turns the
    /// other way.
    #[serde(skip)]
    pub under: Vec<bool>,
}

/// Height of the cable above the stations and poles.
pub const CABLE_HEIGHT: f32 = 22.0;

impl Lift {
    pub fn length(&self) -> f32 {
        let mut pts = vec![self.start];
        pts.extend(self.poles.iter().copied());
        pts.push(self.end);
        pts.windows(2).map(|w| (w[1] - w[0]).length()).sum()
    }

    /// Cable anchor points from base to top.
    pub fn cable(&self) -> Vec<Vec3> {
        let mut pts = vec![self.start + Vec3::Z * CABLE_HEIGHT * 0.6];
        pts.extend(self.poles.iter().map(|p| *p + Vec3::Z * CABLE_HEIGHT));
        pts.push(self.end + Vec3::Z * CABLE_HEIGHT * 0.6);
        pts
    }

    /// Position on the loop: 0..len goes up, len..2len comes back down.
    pub fn point_at(&self, s: f32) -> (Vec3, Vec3) {
        if self.cable_loop.len() >= 2 {
            return self.point_on_loop(s);
        }
        let pts = self.cable();
        let len: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
        let total = len * 2.0;
        let s = s.rem_euclid(total.max(1e-3));
        let (mut d, up) = if s <= len { (s, true) } else { (total - s, false) };
        let side = {
            let dir = (self.end - self.start).truncate().normalize_or_zero();
            Vec3::new(-dir.y, dir.x, 0.0) * if up { 2.2 } else { -2.2 }
        };
        for w in pts.windows(2) {
            let seg = (w[1] - w[0]).length();
            if d <= seg || seg == 0.0 {
                let t = if seg > 0.0 { d / seg } else { 0.0 };
                let dir = (w[1] - w[0]).normalize_or_zero();
                let p = w[0].lerp(w[1], t) + side - Vec3::Z * 2.5;
                return (p, if up { dir } else { -dir });
            }
            d -= seg;
        }
        (self.end + side, Vec3::X)
    }

    /// `point_at` along the cable drawn from the meshes. The parameter
    /// keeps the simulation's scale: 0..len is the way up.
    fn point_on_loop(&self, s: f32) -> (Vec3, Vec3) {
        let pts = &self.cable_loop;
        let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum::<f32>() + (pts[0] - pts[pts.len() - 1]).length();
        let sim_len: f32 = self.cable().windows(2).map(|w| (w[1] - w[0]).length()).sum();
        // Map the simulation's up/down halves onto the loop's halves.
        let s = s.rem_euclid((sim_len * 2.0).max(1e-3));
        let mut d = if s <= sim_len {
            s / sim_len.max(1e-3) * self.cable_up
        } else {
            self.cable_up + (s - sim_len) / sim_len.max(1e-3) * (total - self.cable_up)
        };
        let n = pts.len();
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let seg = (b - a).length();
            if d <= seg {
                let t = if seg > 0.0 { d / seg } else { 0.0 };
                return (a.lerp(b, t), (b - a).normalize_or_zero());
            }
            d -= seg;
        }
        (pts[0], Vec3::X)
    }

    /// The position on the loop (in `point_at` units) that passes closest
    /// to a ground point. Boarding points can lie just before the loop's
    /// start (round the bottom wheel), so the whole loop is searched; for
    /// getting off, the half round the top.
    pub fn nearest_s(&self, p: Vec3, boarding: bool) -> Option<f32> {
        if self.cable_loop.len() < 2 {
            return None;
        }
        let len = self.length();
        let (a, b) = if boarding { (0.0, len * 2.0) } else { (len * 0.5, len * 1.5) };
        let steps = 400;
        (0..=steps)
            .map(|i| a + (b - a) * i as f32 / steps as f32)
            .min_by(|x, y| {
                let dx = (self.point_at(*x).0.truncate() - p.truncate()).length();
                let dy = (self.point_at(*y).0.truncate() - p.truncate()).length();
                dx.total_cmp(&dy)
            })
    }

    /// Where guests join the queue: the far end of the queue line.
    pub fn base_queue_point(&self) -> Vec2 {
        if let Some(p) = self.queue_lines.first().and_then(|l| l.last()) {
            return p.truncate();
        }
        let dir = (self.end - self.start).truncate().normalize_or_zero();
        self.start.truncate() - dir * 8.0
    }

    /// Where the `k`-th guest in the queue stands and which way they face:
    /// along the queue lines back from the boarding point, alternating
    /// between lines when there are two.
    pub fn queue_spot(&self, k: usize) -> Option<(Vec3, Vec2)> {
        const GAP: f32 = 3.0;
        let lines = self.queue_lines.len().max(1);
        let line = self.queue_lines.get(k % lines)?;
        let mut d = (k / lines) as f32 * GAP;
        for w in line.windows(2) {
            let seg = (w[1] - w[0]).truncate().length();
            if d <= seg {
                let dir = (w[0] - w[1]).truncate().normalize_or_zero();
                return Some((w[0].lerp(w[1], d / seg.max(1e-3)), dir));
            }
            d -= seg;
        }
        let n = line.len();
        Some((*line.last()?, (line[n.saturating_sub(2)] - line[n - 1]).truncate().normalize_or_zero()))
    }

    pub fn exit_point(&self) -> Vec2 {
        if let Some(p) = self.exit_line.last() {
            return p.truncate();
        }
        let dir = (self.end - self.start).truncate().normalize_or_zero();
        self.end.truncate() + dir * 10.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: Id,
    /// Index into `Defs::runs` (difficulty).
    pub def: usize,
    pub name: String,
    /// Gate positions from top to bottom.
    pub gates: Vec<Vec3>,
    pub skiers_today: u32,
    /// Closed runs are left alone (`RMTB_CLOSE`).
    #[serde(default = "yes")]
    pub open: bool,
    #[serde(default)]
    pub worth: Worth,
}

fn yes() -> bool {
    true
}

/// A terrain park course laid from `start` (top) to `end` (bottom).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Park {
    pub id: Id,
    pub def: usize,
    pub name: String,
    pub start: Vec3,
    pub end: Vec3,
    pub riders_today: u32,
    pub riders_total: u32,
    #[serde(default = "yes")]
    pub open: bool,
    #[serde(default)]
    pub worth: Worth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deco {
    pub id: Id,
    pub def: usize,
    pub pos: Vec3,
    pub rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Skill {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

impl Skill {
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(i: usize) -> Skill {
        match i {
            0 => Skill::Beginner,
            1 => Skill::Intermediate,
            2 => Skill::Advanced,
            _ => Skill::Expert,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GuestState {
    /// Walking on the ground to a point.
    Walking { to: Vec2, purpose: Purpose },
    /// Inside a building until `need` is met, or with none until `until`
    /// (0x493d90).
    InStructure {
        structure: Id,
        until: f32,
        #[serde(default)]
        need: Option<NeedKind>,
    },
    Queued { lift: Id },
    /// In line at a full building's door (0x493d90 state 7).
    AtDoor { structure: Id },
    Riding { lift: Id },
    Skiing { path: Vec<Vec2>, next: usize, goal: Purpose },
    Crashed { until: f32 },
    /// Steered by the player.
    Controlled,
    Injured,
    /// Held by a sasquatch that is coming for them.
    Seized,
    /// Picked up by the player ("Hand of God").
    Held,
    /// Died of hypothermia: floats up as an angel, then is gone.
    Dying { t: f32 },
    Rescued,
    Leaving,
    Gone,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Purpose {
    Wander,
    Lift(Id),
    Structure(Id),
    Park(Id),
    /// Keep skiing downhill from wherever the guest ends up.
    Continue,
    Exit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guest {
    pub id: Id,
    pub name: String,
    pub boarder: bool,
    pub color: u8,
    pub skill: Skill,
    pub pos: Vec3,
    pub heading: f32,
    pub speed: f32,
    pub state: GuestState,
    pub needs: Needs,
    pub happiness: f32,
    pub cash: f32,
    pub spent: f32,
    pub days_left: f32,
    pub has_equipment: bool,
    /// Rental shop the guest rents equipment from (charged daily).
    #[serde(default)]
    pub rented_from: Option<Id>,
    pub uses: Vec<(Id, u32)>,
    /// Current thought and how long it lasts.
    pub thought: Option<(super::thought::Thought, f32)>,
    pub arrived_day: u32,
    pub runs_today: u32,
    pub anim: f32,
    /// Seconds spent injured without a patrol on the way.
    #[serde(default)]
    pub injured_for: f32,
    /// How dear a price the guest will pay, 0.1 to 1 (0x486ff0); see
    /// `StructureDef::dearness`.
    #[serde(default = "middling")]
    pub tolerance: f32,
    /// Ticks spent in the current line (0x489fc0).
    #[serde(default)]
    pub waited: f32,
    /// The run or park the guest chose at the bottom and is riding up to
    /// (guest +0xa4, 0x487b00).
    #[serde(default)]
    pub aim: Option<Aim>,
    /// How sharply the skier last turned, about the sine of the angle
    /// (+0xb4, set on aiming at each waypoint by 0x48e600).
    #[serde(default)]
    pub turn: f32,
    /// Poling, skiing or climbing (0x48ee70 states 1, 9 and 2).
    #[serde(default)]
    pub ski_mode: SkiMode,
    /// The player's own skier, from the first-person button (+0x13e).
    #[serde(default)]
    pub own: bool,
    /// While steered, the clip being played and the body's whole
    /// orientation (see `control`).
    #[serde(skip)]
    pub ctl_clip: Option<&'static str>,
    #[serde(skip)]
    pub tilt: Option<Quat>,
}

/// What a guest means to ski once up the lift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Aim {
    Run(Id),
    Park(Id),
}

fn middling() -> f32 {
    0.55
}

impl Guest {
    pub fn need(&self, k: NeedKind) -> f32 {
        self.needs.get(k)
    }
    pub fn is_active(&self) -> bool {
        !matches!(self.state, GuestState::Gone)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum StaffKind {
    Patrol,
    Maintenance,
    Sanitation,
    Instructor,
    Groomer,
    AvalancheControl,
    Security,
    WildlifeControl,
    /// The snowmobile tour's driver (STAFF_SNOW_TOUR).
    SnowTour,
    /// The snowmaking machine that comes with the groomer shop.
    Snowmaker,
}

impl StaffKind {
    pub const ALL: [StaffKind; 10] = [
        StaffKind::Patrol,
        StaffKind::Maintenance,
        StaffKind::Sanitation,
        StaffKind::Instructor,
        StaffKind::Groomer,
        StaffKind::AvalancheControl,
        StaffKind::Security,
        StaffKind::WildlifeControl,
        StaffKind::SnowTour,
        StaffKind::Snowmaker,
    ];
    /// The text naming one of these (`Maintenance #%d`); the groomer has
    /// none, the avalanche control's has no number in it.
    pub fn name_text(self) -> Option<crate::text::Text> {
        use crate::text::Text;
        Some(match self {
            StaffKind::Patrol => Text::StaffSkiPatrolName,
            StaffKind::Maintenance => Text::StaffMaintName,
            StaffKind::Sanitation => Text::StaffSanitationName,
            StaffKind::Instructor => Text::StaffSkiInstructorName,
            StaffKind::Groomer => Text::TeGroomerName,
            StaffKind::Snowmaker => Text::TeSnowmakerName,
            StaffKind::AvalancheControl => Text::StaffAcgName,
            StaffKind::Security => Text::StaffSecurityName,
            StaffKind::WildlifeControl => Text::StaffWildlifecontrolName,
            StaffKind::SnowTour => Text::StaffSnowTourName,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            StaffKind::Patrol => "Ski Patrol",
            StaffKind::Maintenance => "Mechanic",
            StaffKind::Sanitation => "Sanitation",
            StaffKind::Instructor => "Instructor",
            StaffKind::Groomer => "Groomer",
            StaffKind::AvalancheControl => "Avalanche Control",
            StaffKind::Security => "Security",
            StaffKind::WildlifeControl => "Wildlife Control",
            StaffKind::SnowTour => "Snowmobile Tour",
            StaffKind::Snowmaker => "Snowmaker",
        }
    }
    pub fn mesh(self) -> &'static str {
        match self {
            StaffKind::Patrol => "data/scenery/people/ski_patrol/skipatrol.MESH",
            StaffKind::Maintenance => "data/scenery/people/mechanic/mechanic.mesh",
            StaffKind::Sanitation => "data/scenery/people/sanitation/sanitation.MESH",
            // At a ski school; a board school's are drawn as boarders.
            StaffKind::Instructor => "data/scenery/people/ski_patrol/skipatrol.MESH",
            StaffKind::Groomer => "data/scenery/people/groomer/groomer.mesh",
            StaffKind::AvalancheControl => "data/scenery/people/avicontrol/avicontrol.MESH",
            StaffKind::Security => "data/scenery/people/security/security.MESH",
            StaffKind::WildlifeControl => "data/scenery/people/wildlifecontrol/wildlifecontrol.MESH",
            StaffKind::SnowTour => "data/scenery/people/tourbus/tourbus.mesh",
            StaffKind::Snowmaker => "data/scenery/people/snowmaker/blower.mesh",
        }
    }
}

/// How a skier is getting along (0x48ee70).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SkiMode {
    /// Poling or skating, up to 8/27 a tick (state 1).
    #[default]
    Pole,
    /// Skiing down by the slope table (states 4 and 9).
    Ski,
    /// Sidestepping up at 3.2/27 a tick (state 2).
    Climb,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaffTask {
    Idle,
    MoveTo(Vec2),
    Repair(Id),
    RepairLift(Id),
    /// Going to an injured guest.
    Rescue(Id),
    /// First aid where they lie: 33 ticks (0x4933a0 state 2).
    Aid { guest: Id, left: f32 },
    /// Taking them on the sled (or snowmobile, or chopper) to base
    /// (states 3-5).
    Carry(Id),
    /// Treating them at base until well (state 7).
    Heal(Id),
    /// Resting after a rescue, 100 ticks (state 8).
    Rest(f32),
    Clean(Vec2),
    Groom(Vec2),
    /// Making snow on a worn cell until it is mended.
    MakeSnow(Vec2),
    /// A snowmobile tour: off with the guests from the tour building to
    /// `to` and back (0x480e60).
    Tour { to: Vec2, back: bool },
    /// Picked up by the player.
    Held,
    /// Avalanche control shelling a risky slope (see `avalanche`).
    Mortar { cell: (usize, usize), stage: MortarStage },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MortarStage {
    /// Walking to within range.
    Walk,
    /// Setting up and firing, for the given seconds so far.
    Aim(f32),
    /// The shell in the air.
    Shell(f32),
    /// Watching the slope settle, then packing up.
    Settle(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Staff {
    pub id: Id,
    pub kind: StaffKind,
    pub name: String,
    pub pos: Vec3,
    pub heading: f32,
    pub task: StaffTask,
    pub wage: f32,
    pub work_timer: f32,
    pub carrying: Option<Id>,
    /// The building that took them on; instructors teach at theirs.
    #[serde(default)]
    pub home: Option<Id>,
    /// Seconds into the current animation.
    #[serde(skip)]
    pub anim: f32,
    /// Moved this tick.
    #[serde(skip)]
    pub moving: bool,
    /// A ski patroller's speed (units a second) and how they are skiing.
    #[serde(skip)]
    pub speed: f32,
    #[serde(skip)]
    pub ski_mode: SkiMode,
}

impl Staff {
    /// The animation clip for what the employee is doing. Ski patrollers
    /// and instructors are skiers; the groomer's mesh has no clips.
    pub fn clip(&self) -> &'static str {
        let skier = matches!(self.kind, StaffKind::Patrol | StaffKind::Instructor);
        match (&self.task, self.moving) {
            (StaffTask::Held, _) => {
                if skier {
                    "FlailNoSkis"
                } else {
                    "Flail"
                }
            }
            (_, true) => {
                if self.kind == StaffKind::Patrol && self.speed > 0.0 {
                    match self.ski_mode {
                        SkiMode::Pole => "DoublePole",
                        SkiMode::Climb => "SkiWalk",
                        SkiMode::Ski => match self.speed / 30.0 {
                            v if v > 0.444_444_45 => "GlideFast",
                            v if v > 0.296_296_3 => "GlideMedium",
                            _ => "GlideSlow",
                        },
                    }
                } else if skier {
                    "SkiWalk"
                } else {
                    "Walk"
                }
            }
            (StaffTask::Clean(_), false) => "Clean",
            (StaffTask::Repair(_) | StaffTask::RepairLift(_), false) => "Fix",
            (StaffTask::Mortar { stage: MortarStage::Aim(_), .. }, false) => "ShootCannon",
            _ => {
                if skier {
                    "IdleCarrySkies"
                } else {
                    "Idle"
                }
            }
        }
    }
}

/// Dirt on a cell, drawn on the snow (0x40bce0): litter, or where a
/// guest relieved themselves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Litter {
    /// The cell's middle, on the ground.
    pub pos: Vec3,
    #[serde(default)]
    pub yellow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Sky {
    #[default]
    Clear,
    Snow,
    Rain,
    Fog,
}

pub use super::weather::Weather;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Ledger {
    Tickets,
    Lodging,
    Food,
    Rentals,
    Lessons,
    Retail,
    Entertainment,
    Services,
    Construction,
    Wages,
    Operations,
    Maintenance,
    /// Lawsuits.
    Legal,
    /// Trees, snow surfaces and footpaths (the original's category 4).
    Landscaping,
}

impl Ledger {
    pub const ALL: [Ledger; 14] = [
        Ledger::Tickets,
        Ledger::Lodging,
        Ledger::Food,
        Ledger::Rentals,
        Ledger::Lessons,
        Ledger::Retail,
        Ledger::Entertainment,
        Ledger::Services,
        Ledger::Construction,
        Ledger::Wages,
        Ledger::Operations,
        Ledger::Maintenance,
        Ledger::Legal,
        Ledger::Landscaping,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Ledger::Tickets => "Lift tickets",
            Ledger::Lodging => "Lodging",
            Ledger::Food => "Food",
            Ledger::Rentals => "Rentals",
            Ledger::Lessons => "Lessons",
            Ledger::Retail => "Retail",
            Ledger::Entertainment => "Entertainment",
            Ledger::Services => "Services",
            Ledger::Construction => "Construction",
            Ledger::Wages => "Wages",
            Ledger::Operations => "Operations",
            Ledger::Maintenance => "Maintenance",
            Ledger::Legal => "Legal fees",
            Ledger::Landscaping => "Landscaping",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DayStats {
    pub day: u32,
    pub income: Vec<(Ledger, f32)>,
    pub expense: Vec<(Ledger, f32)>,
    pub guests: u32,
    pub cash: f32,
    /// What the buildings, lifts and runs are worth (0x428820).
    #[serde(default)]
    pub assets: f32,
    pub rating: f32,
    pub happiness: f32,
}

impl DayStats {
    pub fn add(&mut self, l: Ledger, amount: f32) {
        let list = if amount >= 0.0 { &mut self.income } else { &mut self.expense };
        match list.iter_mut().find(|(k, _)| *k == l) {
            Some((_, v)) => *v += amount,
            None => list.push((l, amount)),
        }
    }
    pub fn total_income(&self) -> f32 {
        self.income.iter().map(|(_, v)| v).sum()
    }
    pub fn total_expense(&self) -> f32 {
        self.expense.iter().map(|(_, v)| v).sum()
    }
}

/// What the resort's rating is made of (`resort.ini`'s caps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatingPart {
    CleanSnow,
    Guests,
    Happiness,
    Lifts,
    BeginnerRuns,
    IntermediateRuns,
    AdvancedRuns,
    ExpertRuns,
    Lodging,
    Food,
    Rentals,
    SkiSchool,
    Safety,
    Retail,
    Entertainment,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rating {
    /// Each part's points and its cap, rebuilt with the rating.
    #[serde(skip)]
    pub parts: Vec<(RatingPart, f32, f32)>,
    pub total: f32,
    pub stars: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub text: String,
    pub day: u32,
    pub hour: f32,
    pub focus: Option<Vec3>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub level_name: String,
    pub level_file: String,
    #[serde(skip)]
    pub level: Option<Level>,
    pub level_bytes: Vec<u8>,
    pub next_id: Id,
    pub structures: Vec<Structure>,
    pub lifts: Vec<Lift>,
    pub runs: Vec<Run>,
    #[serde(default)]
    pub parks: Vec<Park>,
    pub decos: Vec<Deco>,
    pub guests: Vec<Guest>,
    pub staff: Vec<Staff>,
    pub litter: Vec<Litter>,
    /// Ticks run, for work spread over ticks.
    #[serde(default)]
    pub tick: u64,
    /// Cells the player has ordered groomed, waiting for a groomer
    /// (the job lists 0x43be40 fills).
    #[serde(default)]
    pub groom_jobs: Vec<(usize, usize)>,
    /// Cells ordered snowed, waiting for a snowmaking machine.
    #[serde(default)]
    pub snow_jobs: Vec<(usize, usize)>,
    /// Waypoints for guests on long walks (guest, remaining route).
    #[serde(default)]
    pub walk_routes: Vec<(Id, Vec<Vec2>)>,
    /// Cells covered by a structure/lift station (structure or lift id).
    pub occupied: Vec<Option<Id>>,
    /// Guests who have left, per reason, since that reason's last report
    /// (see `Sim::departure_report`).
    #[serde(default)]
    pub leave_counts: Vec<u32>,
    /// Ticks run, and per cell the tick its avalanche risk was last
    /// reset (see `avalanche`).
    #[serde(default)]
    pub avalanche_clock: f64,
    #[serde(default)]
    pub avalanche_risk: Vec<f64>,
    #[serde(default)]
    pub avalanches: Vec<super::avalanche::Avalanche>,
    #[serde(skip)]
    pub blasts: Vec<super::avalanche::Blast>,
    #[serde(default)]
    pub animals: Vec<super::wildlife::Animal>,
    #[serde(default)]
    pub wildlife_spawned: bool,
    /// The guest the player is steering, if any.
    #[serde(skip)]
    pub controlled: Option<Id>,
    /// Sounds the simulation wants played, with where they come from if
    /// they are only heard while that spot is on screen (0x41bbe0).
    #[serde(skip)]
    pub sounds: Vec<(crate::audio::Sound, Option<Vec3>)>,
    /// Day, dusk, night and dawn.
    #[serde(default)]
    pub daylight: super::daylight::DayCycle,
    /// Avalanches started, for the start sound.
    #[serde(skip)]
    pub avalanches_started: u32,
    /// Trees or snow changed outside the editing tools (avalanches,
    /// groomers).
    #[serde(skip)]
    pub scenery_dirty: bool,
    /// Cells a lift's stations and line take up (see `Game::rebuild_lift_cells`).
    #[serde(skip)]
    pub lift_cells: Vec<Option<Id>>,
    /// Cells the power reaches (0x40000 in the original), and what the
    /// grid was worked out for.
    #[serde(skip)]
    pub powered_cells: Vec<bool>,
    #[serde(skip)]
    pub power_key: Option<u64>,
    pub cash: f32,
    pub day: u32,
    /// Hours since midnight.
    pub hour: f32,
    pub weather: Weather,
    pub today: DayStats,
    pub history: Vec<DayStats>,
    pub rating: Rating,
    pub max_guests: u32,
    pub messages: Vec<Message>,
    pub ticket_price: f32,
    pub challenge: Option<usize>,
    #[serde(default)]
    pub challenge_state: Option<super::challenge::ChallengeState>,
    pub total_guests: u32,
    pub rng: u64,
    /// Landscaping is free (the map editor).
    pub sandbox: bool,
    /// A map editor session: Save writes the map as a level.
    #[serde(default)]
    pub editor: bool,
    /// The map editor's Generate Random Map numbers.
    #[serde(default)]
    pub map_gen: super::editor::GenSettings,
    #[serde(skip)]
    pub options: super::sim::Options,
}

impl World {
    pub fn level(&self) -> &Level {
        self.level.as_ref().expect("level loaded")
    }
    pub fn level_mut(&mut self) -> &mut Level {
        self.level.as_mut().expect("level loaded")
    }

    pub fn new_id(&mut self) -> Id {
        self.next_id += 1;
        self.next_id
    }

    pub fn rand(&mut self) -> f32 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32) / (1u64 << 24) as f32
    }

    pub fn rand_range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.rand()
    }

    pub fn structure(&self, id: Id) -> Option<&Structure> {
        self.structures.iter().find(|s| s.id == id)
    }
    pub fn structure_mut(&mut self, id: Id) -> Option<&mut Structure> {
        self.structures.iter_mut().find(|s| s.id == id)
    }
    pub fn lift(&self, id: Id) -> Option<&Lift> {
        self.lifts.iter().find(|l| l.id == id)
    }
    pub fn lift_mut(&mut self, id: Id) -> Option<&mut Lift> {
        self.lifts.iter_mut().find(|l| l.id == id)
    }
    pub fn guest(&self, id: Id) -> Option<&Guest> {
        self.guests.iter().find(|g| g.id == id)
    }

    pub fn cell_index(&self, cx: i32, cy: i32) -> Option<usize> {
        let n = self.level().cells_per_side() as i32;
        (cx >= 0 && cy >= 0 && cx < n && cy < n).then(|| (cy * n + cx) as usize)
    }

    pub fn ground(&self, p: Vec2) -> Vec3 {
        Vec3::new(p.x, p.y, self.level().height_at(p.x, p.y))
    }

    pub fn message(&mut self, text: impl Into<String>, focus: Option<Vec3>) {
        let m = Message {
            text: text.into(),
            day: self.day,
            hour: self.hour,
            focus,
        };
        log::info!("day {}: {}", m.day, m.text);
        self.messages.push(m);
        if self.messages.len() > 200 {
            self.messages.remove(0);
        }
    }

    pub fn is_night(&self) -> bool {
        self.daylight.is_night()
    }

    /// The snow variant of something standing at height `z`.
    pub fn snow_variant(&self, z: f32) -> u8 {
        self.weather.snow_variant(z, self.level().height_range())
    }

    pub fn active_guests(&self) -> usize {
        self.guests.iter().filter(|g| g.is_active()).count()
    }

    /// Whether something meeting a requirement stands in the resort
    /// (0x42bf30 looks things up by name).
    pub fn has_built(&self, defs: &Defs, r: &super::defs::Requirement) -> bool {
        use super::defs::{Category, Requirement};
        match r {
            Requirement::AnyPowerPlant => self.structures.iter().any(|s| defs.structures[s.def].category == Category::PowerPlant),
            Requirement::Structure(i) => self.structures.iter().any(|s| s.def == *i),
            Requirement::Lift(i) => self.lifts.iter().any(|l| l.def == *i),
            Requirement::Run(i) => self.runs.iter().any(|r| r.def == *i),
            Requirement::Park(i) => self.parks.iter().any(|p| p.def == *i),
            Requirement::Unknown(_) => false,
        }
    }

    pub fn spend(&mut self, ledger: Ledger, amount: f32) {
        self.cash -= amount;
        self.today.add(ledger, -amount);
    }

    pub fn earn(&mut self, ledger: Ledger, amount: f32) {
        self.cash += amount;
        self.today.add(ledger, amount);
    }
}
