//! Simulation: clock, weather, guests, lifts, staff, money and rating.
//!
//! The original runs at 30 ticks per second and its tuning values in
//! `skiers.ini`/`structures*.ini` are per tick; `TICK` converts.

use glam::{Vec2, Vec3};

use super::defs::{ALL_NEEDS, Category, Defs, NeedKind, Needs, StructureKind};
use super::nav::{Mode, NavGrid};
use super::thought::Thought;
use super::world::*;
use crate::formats::level::cell_flags;
use crate::text::Text;

pub const TICK: f32 = 1.0 / 30.0;
/// How steep a lift climbs (0x4a3950): its rise over its length, the
/// length taken as the longest of the three spans plus a quarter of the
/// other two.
fn lift_steepness(l: &Lift) -> f32 {
    let d = (l.end - l.start).abs();
    let mut s = [d.x, d.y, d.z];
    s.sort_by(|a, b| b.total_cmp(a));
    let len = s[0] + 0.25 * (s[1] + s[2]);
    if len > 0.0 { (l.end.z - l.start.z) / len } else { 0.0 }
}

/// Whether a skier of this level will ride the lift: its steepness within
/// the level's band (0x6520b8, 0x6520c8).
fn lift_suits(l: &Lift, skill: Skill) -> bool {
    const LOW: [f32; 4] = [0.0, 0.15, 0.3, 0.6];
    const HIGH: [f32; 4] = [0.3, 0.5, 0.6, 1.0];
    let k = lift_steepness(l);
    let i = skill.index();
    LOW[i] <= k && k <= HIGH[i]
}

/// Who a rental shop or ski school serves, by the first letters of its
/// `unique_name` (0x489a1f, 0x48999c): board rentals boarders, ski
/// rentals skiers and novice boarders; the ski school novice skiers, the
/// snowboard school novice boarders, any other school the more skilled.
fn serves(d: &super::defs::StructureDef, boarder: bool, skill: Skill) -> bool {
    let novice = skill.index() <= 1;
    let n = d.unique_name.as_bytes();
    match d.category {
        Category::Rental => rents(d, boarder, skill),
        Category::SkiSchool => match (n.first(), n.get(1)) {
            (Some(b'S'), Some(b'k')) => !boarder && novice,
            (Some(b'S'), Some(b'n')) => boarder && novice,
            _ => skill.index() >= 1,
        },
        _ => true,
    }
}

/// Whether a guest can rent here (0x4897f0 need 3): rental shops and
/// retail stores (types 7 and 0xc) whose name starts with a B rent to
/// boarders, with an S to skiers and to boarders not past intermediate.
/// So Small Retail rents skis and Large Retail nothing.
fn rents(d: &super::defs::StructureDef, boarder: bool, skill: Skill) -> bool {
    if !matches!(d.category, Category::Rental | Category::Retail) {
        return false;
    }
    match d.unique_name.as_bytes().first() {
        Some(b'B') => boarder,
        Some(b'S') => !boarder || skill.index() <= 1,
        _ => false,
    }
}

/// Each level's skiing speed (0x651f84): 3.2, 7.2, 10 and 12 of 27ths of
/// a unit a tick. Faster than this a skier may fall, and has fun.
/// The skiing table at 0x65c140, by the drop over one unit ahead: the
/// largest drop for the row (sine of 5k degrees), the speed multiplier,
/// the downhill gain a tick, and the top speed a tick (columns 0, 1, 4
/// and 2 of the original's seven).
const SPEED_TABLE: [[f32; 4]; 19] = [
    [0.0, 1.0, 0.0568, 0.3704],
    [0.0872, 0.9996, 0.0846, 0.4239],
    [0.1736, 0.9983, 0.1118, 0.4773],
    [0.2588, 0.9962, 0.1382, 0.5308],
    [0.342, 0.9932, 0.1635, 0.5843],
    [0.4226, 0.9894, 0.1876, 0.6379],
    [0.5, 0.9848, 0.2102, 0.6914],
    [0.5736, 0.9793, 0.2312, 0.7449],
    [0.6428, 0.973, 0.2505, 0.7984],
    [0.7071, 0.9659, 0.2679, 0.8519],
    [0.766, 0.958, 0.2832, 0.9053],
    [0.8191, 0.9492, 0.2964, 0.9588],
    [0.866, 0.9397, 0.3073, 1.0123],
    [0.9063, 0.9294, 0.3159, 1.0658],
    [0.9397, 0.9182, 0.322, 1.1194],
    [0.9659, 0.9063, 0.3258, 1.1729],
    [0.9848, 0.8936, 0.327, 1.2264],
    [0.9962, 0.8802, 0.3258, 1.2799],
    [1.0, 0.866, 0.322, 1.3333],
];

/// One tick of a skier's speed, units a tick (0x48ee70 states 1, 2, 4/9).
/// `drop` is how far the ground falls over the next unit, `pace` the
/// level's speed (0x651f84).
fn ski_tick(v: f32, drop: f32, turn: f32, mode: &mut SkiMode, pace: f32) -> f32 {
    const POLE_TOP: f32 = 8.0 / 27.0; // 0x65c120
    const WALK: f32 = 4.0 / 27.0; // 0x65c128
    const CLIMB: f32 = 3.2 / 27.0; // 0x65c12c
    let r = SPEED_TABLE[SPEED_TABLE.iter().position(|r| drop.abs() <= r[0]).unwrap_or(SPEED_TABLE.len() - 1)];
    let slide = |v: f32| if drop > 0.0 { ((v + r[2]) * r[1]).min(r[3]) } else { v * r[1] };
    match *mode {
        SkiMode::Pole => {
            let mut v = slide(v);
            if v > WALK * 0.7 || drop > 0.0 {
                if v > POLE_TOP {
                    if drop > 0.087_16 {
                        *mode = SkiMode::Ski;
                    }
                    v = POLE_TOP;
                }
            } else {
                *mode = SkiMode::Climb;
            }
            v
        }
        SkiMode::Climb => {
            if drop >= 0.0 {
                *mode = SkiMode::Pole;
            }
            CLIMB
        }
        SkiMode::Ski => {
            let mut v = slide(v);
            // Braking in turns, once faster than the level's pace.
            if turn.abs() > 0.0086 && v > pace {
                v -= 0.133_333_34 * turn.abs();
            }
            if v < POLE_TOP * 0.5 {
                *mode = SkiMode::Pole;
            }
            v.max(0.0)
        }
    }
}

const SKILL_SPEED: [f32; 4] = [0.118_518_52 / TICK, 0.266_666_7 / TICK, 0.370_370_36 / TICK, 0.444_444_45 / TICK];

/// What joining a ski class costs (0x4951f1).
const LESSON_FEE: f32 = 85.0;

/// The needs a guest looks after, in the order the original checks them
/// (0x488b80): the level below which they go looking, the level below
/// which going without counts four rather than one, and the thought.
const NEED_RULES: [(NeedKind, f32, f32, Thought); 5] = [
    (NeedKind::Bathroom, 0.3, 0.15, Thought::NeedToWeeWee),
    (NeedKind::Rest, 0.3, 0.15, Thought::Tired),
    (NeedKind::Warmth, 0.4, 0.2, Thought::Cold),
    (NeedKind::Food, 0.4, 0.2, Thought::Hungry),
    (NeedKind::Entertainment, 0.4, 0.2, Thought::Bored),
];
/// Walking pace on open snow and, quicker, on paths and round buildings:
/// 4/27 and 5.6/27 of a unit a tick (0x48f8xx, 0x65c128, 0x65c124).
const WALK_SPEED: f32 = 0.148_148_15 / TICK;
const PATH_SPEED: f32 = 0.207_407_4 / TICK;
/// Staff at work move at the quicker pace (0x483ffd).
const STAFF_SPEED: f32 = PATH_SPEED;

/// Gameplay switches from the settings screen.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub fast_maintenance: bool,
    pub reduce_garbage: bool,
    pub no_nightfall: bool,
}

pub struct Sim<'a> {
    pub defs: &'a Defs,
    pub w: &'a mut World,
}

fn ticks(dt: f32) -> f32 {
    dt / TICK
}

/// Where a building's takings go: lodges to lodging, the rest to sales
/// (0x494444), which the finances split by kind.
fn sales_ledger(c: Category) -> Ledger {
    match c {
        Category::Lodging => Ledger::Lodging,
        Category::Food => Ledger::Food,
        Category::Rental => Ledger::Rentals,
        Category::SkiSchool => Ledger::Lessons,
        Category::Retail => Ledger::Retail,
        Category::Entertainment => Ledger::Entertainment,
        _ => Ledger::Services,
    }
}

/// How much wear, in percent, a mechanic mends in `dt` (0x47913b): one
/// percent a tick, each costing half the `maintenance_cost`; a building
/// or lift only works again once mended fully.
fn repair_step(dt: f32) -> f32 {
    ticks(dt)
}

impl Sim<'_> {
    pub fn hours_per_second(&self) -> f32 {
        24.0 / self.defs.resort.seconds_per_day
    }

    /// Advances the world by `dt` simulated seconds.
    pub fn step(&mut self, dt: f32) {
        let prev_hour = self.w.hour;
        self.w.hour += dt * self.hours_per_second();
        if self.w.hour >= 24.0 {
            self.w.hour -= 24.0;
            self.end_of_day();
        }

        for _ in 0..ticks(dt).round().max(1.0) as u32 {
            self.w.daylight.tick(self.w.options.no_nightfall);
        }
        self.weather(dt);
        self.settle_snow();
        self.w.tick += 1;
        self.arrivals(dt);
        self.classes();
        self.lifts(dt);
        self.guests(dt);
        self.staff(dt);
        self.wear(dt);
        self.avalanches(dt);
        self.wildlife(dt);
        if (self.w.hour * 4.0).floor() != (prev_hour * 4.0).floor() {
            self.update_rating();
            self.update_capacity();
        }
    }

    // ---------------------------------------------------------------- clock

    fn end_of_day(&mut self) {
        self.w.weather.end_of_day();
        for p in &mut self.w.parks {
            p.riders_today = 0;
        }
        for g in self.w.guests.iter_mut().filter(|g| g.is_active()) {
            g.runs_today = 0;
        }

        let mut stats = std::mem::take(&mut self.w.today);
        stats.day = self.w.day;
        stats.cash = self.w.cash;
        stats.assets = self.w.structures.iter().map(|s| s.worth.now).sum::<f32>()
            + self.w.lifts.iter().map(|l| l.worth.now).sum::<f32>()
            + self.w.runs.iter().map(|r| r.worth.now).sum::<f32>();
        stats.rating = self.w.rating.stars;
        stats.guests = self.w.active_guests() as u32;
        stats.happiness = self.average_happiness();
        self.w.history.push(stats);
        for s in &mut self.w.structures {
            s.visits_today = 0;
            s.revenue_today = 0.0;
        }
        for l in &mut self.w.lifts {
            l.riders_today = 0;
        }
        for r in &mut self.w.runs {
            r.skiers_today = 0;
        }
        self.w.day += 1;
        self.w.today.day = self.w.day;
    }

    pub fn average_happiness(&self) -> f32 {
        let (sum, n) = self
            .w
            .guests
            .iter()
            .filter(|g| g.is_active())
            .fold((0.0, 0), |(s, n), g| (s + g.happiness, n + 1));
        if n == 0 { 0.5 } else { sum / n as f32 }
    }

    // -------------------------------------------------------------- weather

    // ------------------------------------------------------------- arrivals

    /// How many guests the resort holds (0x42b850): its lodges' beds and
    /// half its lifts' seats, each scaled and capped by the tuning, plus a
    /// share of the rating; then a fifth less, which each daycare wins
    /// back 30 of.
    pub fn update_capacity(&mut self) {
        let t = &self.defs.skiers;
        let seats: u32 = self
            .w
            .lifts
            .iter()
            .map(|l| (l.chairs.len() as u32 * self.defs.lifts[l.def].capacity as u32) >> 1)
            .sum();
        let count = |k: StructureKind| self.w.structures.iter().filter(|s| self.defs.structures[s.def].kind == k).count();
        let beds: f32 = self
            .w
            .structures
            .iter()
            .filter(|s| self.defs.structures[s.def].kind == StructureKind::Lodging)
            .map(|s| self.defs.structures[s.def].guest_occupancy as f32)
            .sum();
        let by_lodging = (beds * t.lodging_factor).clamp(0.0, t.max_lodging as f32);
        let by_lifts = (seats as f32 * t.lifts_factor).clamp(0.0, t.max_lifts as f32);
        let sum = by_lodging + by_lifts + t.max_skiers_5_star as f32 * self.w.rating.total * 0.001;
        let cut = (sum * 0.2 - count(StructureKind::Daycare) as f32 * 30.0).max(0.0);
        let cap = (sum - cut) as i32;
        self.w.max_guests = if cap < 10 { 10 } else { cap.min(t.max_skiers) } as u32;
    }

    fn arrival_rate(&self) -> f32 {
        let t = &self.defs.skiers;
        let max = self.w.max_guests.max(1) as f32;
        let fill = self.w.active_guests() as f32 / max;
        let base = if fill >= 1.0 {
            0.0
        } else if fill < 0.5 {
            t.per_second_empty + (t.per_second_half - t.per_second_empty) * fill * 2.0
        } else {
            t.per_second_half - t.per_second_half * (fill - 0.5) * 2.0
        };
        let m = self.marketing_count() as f32;
        base * (1.3 - 0.3 / (m * 0.5 + 1.0))
    }

    fn marketing_count(&self) -> usize {
        self.w
            .structures
            .iter()
            .filter(|s| self.defs.structures[s.def].kind == StructureKind::Marketing)
            .count()
    }

    fn entrance(&self) -> Option<&Structure> {
        self.w
            .structures
            .iter()
            .find(|s| self.defs.structures[s.def].category == Category::Entrance)
    }

    /// Guests arrive whether or not there is anything to ski (0x4289ef):
    /// each tick one may, with chance the arrival rate times the time
    /// passed (0x486ff0).
    fn arrivals(&mut self, dt: f32) {
        let Some(ent) = self.entrance() else { return };
        let door = ent.door(self.defs, self.w.level().spacing);
        let chance = self.arrival_rate() * dt;
        if self.w.rand() < chance {
            self.spawn_guest(door);
        }
    }

    /// A new arrival at `at`, as the skier constructor makes one
    /// (0x485ae0, 0x487303).
    fn new_guest(&mut self, at: Vec2) -> Guest {
        let t = &self.defs.skiers;
        let id = self.w.new_id();
        let r = self.w.rand();
        let boarder = r > t.distribution[0];
        let roll = self.w.rand();
        let mut acc = 0.0;
        let mut skill = Skill::Expert;
        for (i, p) in t.distribution[1..].iter().enumerate() {
            acc += p;
            if roll < acc {
                skill = Skill::from_index(i);
                break;
            }
        }
        // Each a mean give or take half its spread (0x487303).
        let days = t.visit_days + (self.w.rand() - 0.5) * t.variance_days;
        let cash = t.starting_cash + (self.w.rand() - 0.5) * t.variance_cash;
        let needs_skis = self.w.rand() < t.skill[skill.index()].probability_needs_skis;
        let color = (self.w.rand() * if boarder { 4.0 } else { 6.0 }) as u8;
        let jitter = Vec2::new(self.w.rand_range(-4.0, 4.0), self.w.rand_range(-4.0, 4.0));
        let pos = self.w.ground(at + jitter);
        let name = guest_name(id);
        let mut g = Guest {
            id,
            name,
            boarder,
            color,
            skill,
            pos,
            heading: 0.0,
            speed: 0.0,
            state: GuestState::Walking {
                to: at,
                purpose: Purpose::Wander,
            },
            // Arriving needs (0x492070): each within a third or so of full.
            needs: Needs {
                food: 1.0 - self.w.rand() * 0.3,
                rest: 1.0 - self.w.rand() * 0.35,
                warmth: 1.0 - self.w.rand() * 0.3,
                bathroom: 1.0 - self.w.rand() * 0.35,
                entertainment: 1.0 - self.w.rand() * 0.3,
                health: 1.0,
            },
            happiness: 0.0,
            cash,
            spent: 0.0,
            days_left: days,
            has_equipment: !needs_skis,
            rented_from: None,
            uses: Vec::new(),
            thought: None,
            arrived_day: self.w.day,
            runs_today: 0,
            injured_for: 0.0,
            anim: 0.0,
            tolerance: self.w.rand() * 0.9 + 0.1,
            waited: 0.0,
            aim: None,
            turn: 0.0,
            ski_mode: SkiMode::Pole,
            own: false,
            ctl_clip: None,
            tilt: None,
        };
        g.happiness = g.needs.happiness();
        g
    }

    /// The player's own skier (0x461fc7): a skier, never a boarder, of a
    /// skill drawn as for anyone, with 4800 to spend, held by the player.
    pub fn spawn_own(&mut self, at: Vec2) -> Id {
        let mut g = self.new_guest(at);
        g.boarder = false;
        g.color %= 6;
        g.cash = 4800.0;
        g.has_equipment = true;
        g.own = true;
        g.state = GuestState::Held;
        let id = g.id;
        self.w.guests.push(g);
        id
    }

    fn spawn_guest(&mut self, at: Vec2) {
        let g = self.new_guest(at);
        // At the gate the guest decides what to do first and where to ski
        // (0x4873f6, 0x4873ff). Unless both come to something they turn
        // away there and then, never entering; they count only in the
        // departure reports, which their destructor files (0x4861c0).
        let skill = g.skill;
        self.w.guests.push(g);
        let gi = self.w.guests.len() - 1;
        self.plan_on_ground(gi);
        let admitted = match &self.w.guests[gi].state {
            // Sent home, which has already reported them.
            GuestState::Walking { purpose: Purpose::Exit, .. } | GuestState::Leaving | GuestState::Gone => false,
            GuestState::Walking { purpose: Purpose::Lift(_), .. } => true,
            // Nowhere to ski: the choice has given its reason.
            GuestState::Walking { purpose: Purpose::Wander, .. } => {
                self.departure_report(gi);
                false
            }
            // Off to a building first; there must still be skiing.
            _ => {
                let ok = self.choose_ski(gi, skill).is_some();
                if !ok {
                    self.departure_report(gi);
                }
                ok
            }
        };
        if !admitted {
            self.w.guests.pop();
            return;
        }
        // The lift ticket, whatever the guest has (0x48740e).
        let price = self.entrance().map(|e| e.price).unwrap_or(self.w.ticket_price);
        let g = &mut self.w.guests[gi];
        g.cash -= price;
        g.spent += price;
        self.w.earn(Ledger::Tickets, price);
        if let Some(e) = self
            .w
            .structures
            .iter_mut()
            .find(|s| self.defs.structures[s.def].category == Category::Entrance)
        {
            e.revenue_today += price;
            e.revenue_total += price;
            e.visits_today += 1;
        }
        self.w.total_guests += 1;
        self.w.today.guests += 1;
    }

    // ---------------------------------------------------------------- lifts

    /// Lifts run around the clock (the original has no opening hours; a
    /// sim day is only one minute long).
    pub fn lifts_running(&self) -> bool {
        true
    }

    fn lifts(&mut self, dt: f32) {
        let defs = self.defs;
        let mut arrivals: Vec<(Id, Id)> = Vec::new();
        let mut crossings: Vec<(usize, usize, Vec3)> = Vec::new();
        let spacing = self.w.level().spacing;
        let running = self.lifts_running();
        if !running {
            let mut dispersed = Vec::new();
            for l in &mut self.w.lifts {
                dispersed.extend(l.queue.drain(..));
            }
            for g in &mut self.w.guests {
                if dispersed.contains(&g.id) {
                    g.state = GuestState::Walking {
                        to: g.pos.truncate(),
                        purpose: Purpose::Wander,
                    };
                }
            }
        }
        for li in 0..self.w.lifts.len() {
            let lift = &mut self.w.lifts[li];
            if !lift.open || lift.broken {
                continue;
            }
            // After closing, keep running empty chairs only until riders are off.
            if !running && lift.chairs.iter().all(|c| c.riders.is_empty()) {
                continue;
            }
            let d = &defs.lifts[lift.def];
            let len = lift.length();
            let total = len * 2.0;
            // speed x 0.033 a tick (0x49e98f).
            let advance = d.speed * 0.033 * ticks(dt);
            lift.anim += dt;
            let unload = lift.unload_s.unwrap_or(len);
            let board = lift.board_s.unwrap_or(0.0);
            // Whether the chair passes `at` this step, wrapping round.
            let passes = |before: f32, after: f32, at: f32| {
                (before < at && after >= at) || (before < at + total && after >= at + total)
            };
            for c in 0..lift.chairs.len() {
                let before = lift.chairs[c].s;
                let after = before + advance;
                // Unload at the top.
                if passes(before, after, unload) {
                    for r in lift.chairs[c].riders.drain(..) {
                        arrivals.push((r, lift.id));
                    }
                }
                lift.chairs[c].s = after % total.max(1e-3);
                // A drag lift's handle reaching a new cell (0x4a42a0).
                if d.show_skiers && !lift.chairs[c].riders.is_empty() {
                    let (a, _) = lift.point_at(before);
                    let (b, _) = lift.point_at(lift.chairs[c].s);
                    if (a.x / spacing).floor() != (b.x / spacing).floor() || (a.y / spacing).floor() != (b.y / spacing).floor() {
                        crossings.push((li, c, b));
                    }
                }
                // Load at the boarding point.
                if passes(before, after, board) && running && lift.chairs[c].riders.is_empty() {
                    while lift.chairs[c].riders.len() < d.capacity as usize {
                        let Some(g) = lift.queue.pop_front() else { break };
                        lift.chairs[c].riders.push(g);
                        lift.riders_today += 1;
                        lift.riders_total += 1;
                    }
                }
            }
        }
        // On steep ground a drag lift may shed its riders: a roll of up to
        // 1000 times how far the ground is from level, less 0.15, over 50;
        // then each rider lets go one time in 90, 300, 900 or 3000 by skill,
        // thinking the tow too steep (0x4a42a0).
        const HOLD: [f32; 4] = [90.0, 300.0, 900.0, 3000.0];
        for (li, c, p) in crossings {
            let n = self.w.level().normal_at(p.x, p.y).normalize_or_zero();
            let roll = (self.w.rand() * 1000.0).floor();
            if ((1.0 - n.z) - 0.15) * roll <= 50.0 {
                continue;
            }
            let riders = self.w.lifts[li].chairs.get(c).map(|ch| ch.riders.clone()).unwrap_or_default();
            for gid in riders {
                let Some(gi) = self.w.guests.iter().position(|g| g.id == gid) else { continue };
                if self.w.rand() * HOLD[self.w.guests[gi].skill.index()] >= 1.0 {
                    continue;
                }
                self.w.lifts[li].chairs[c].riders.retain(|r| *r != gid);
                let ground = self.w.ground(p.truncate());
                let g = &mut self.w.guests[gi];
                g.pos = ground;
                g.thought = Some((Thought::MadRopeTowTooSteep, 4.0));
                self.plan_on_ground(gi);
            }
        }
        for g in &mut self.w.guests {
            if let GuestState::Queued { lift } = g.state {
                if let Some(l) = self.w.lifts.iter().find(|l| l.id == lift) {
                    if l.chairs.iter().any(|c| c.riders.contains(&g.id)) {
                        g.state = GuestState::Riding { lift };
                    }
                }
            }
        }
        for (gid, lid) in arrivals {
            let exit = self.w.lift(lid).map(|l| l.exit_point()).unwrap_or_default();
            if let Some(g) = self.w.guests.iter_mut().find(|g| g.id == gid) {
                g.pos = Vec3::new(exit.x, exit.y, 0.0);
                g.state = GuestState::Walking {
                    to: exit,
                    purpose: Purpose::Wander,
                };
            }
            let p = self.w.ground(exit);
            if let Some(g) = self.w.guests.iter_mut().find(|g| g.id == gid) {
                g.pos = p;
            }
            self.plan_from_top(gid);
        }
    }

    /// Rebuilds a lift's chairs evenly along its loop.
    pub fn layout_chairs(defs: &Defs, lift: &mut Lift) {
        let d = &defs.lifts[lift.def];
        let len = lift.length();
        let spacing = (d.speed * d.secs_between_chairs).max(4.0);
        let n = ((len * 2.0) / spacing).floor().max(2.0) as usize;
        lift.chairs = (0..n)
            .map(|i| Chair {
                s: i as f32 * (len * 2.0) / n as f32,
                riders: Vec::new(),
            })
            .collect();
    }

    // --------------------------------------------------------------- guests

    fn blocked_cells(&self) -> Vec<bool> {
        self.w.occupied.iter().map(|o| o.is_some()).collect()
    }

    /// How much the route down favors each cell: along the run the guest
    /// chose, or with none chosen, runs near their level.
    fn run_bonus(&self, skill: Skill, chosen: Option<Id>) -> Vec<f32> {
        let level = self.w.level();
        let n = level.cells_per_side();
        let s = level.spacing;
        let mut bonus = vec![0.0f32; n * n];
        for run in self.w.runs.iter().filter(|r| r.open && chosen.is_none_or(|c| c == r.id)) {
            let diff = self.defs.runs.get(run.def).map(|d| d.difficulty).unwrap_or(0);
            let fit = if chosen.is_some() { 1.0 } else { 1.0 - ((diff - skill.index() as i32).abs() as f32 * 0.6) };
            if fit <= 0.0 {
                continue;
            }
            for w in run.gates.windows(2) {
                let (a, b) = (w[0].truncate(), w[1].truncate());
                let steps = (a.distance(b) / (s * 0.5)).ceil().max(1.0) as usize;
                for k in 0..=steps {
                    let p = a.lerp(b, k as f32 / steps as f32);
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let cx = (p.x / s) as i32 + dx;
                            let cy = (p.y / s) as i32 + dy;
                            if cx >= 0 && cy >= 0 && (cx as usize) < n && (cy as usize) < n {
                                let i = cy as usize * n + cx as usize;
                                bonus[i] = bonus[i].max(0.7 * fit);
                            }
                        }
                    }
                }
            }
        }
        bonus
    }

    fn ski_mode(&self, skill: Skill) -> Mode {
        let st = &self.defs.skiers.skill[skill.index()];
        Mode::Ski {
            min_slope: st.min_slope,
            max_slope: st.max_slope,
            danger: self.defs.skiers.danger_min_slope.min(55.0),
        }
    }

    /// Chooses what a guest does after getting off a lift.
    fn plan_from_top(&mut self, gid: Id) {
        let Some(gi) = self.w.guests.iter().position(|g| g.id == gid) else { return };
        let pos = self.w.guests[gi].pos.truncate();
        let skill = self.w.guests[gi].skill;
        let want = self.low_need(gi);
        let mut goals: Vec<(Vec2, Purpose)> = Vec::new();
        if let Some(k) = want {
            for s in &self.w.structures {
                let d = &self.defs.structures[s.def];
                if s.open && !s.broken && d.provides.get(k) > 0.0 {
                    goals.push((s.door(self.defs, self.w.level().spacing), Purpose::Structure(s.id)));
                }
            }
        }
        let done = self.w.guests[gi].days_left < 0.0;
        // The park chosen at the bottom, if that is what they came up for.
        let aim = self.w.guests[gi].aim.take();
        if let (Some(Aim::Park(id)), true) = (aim, goals.is_empty() && !done) {
            if let Some(p) = self.w.parks.iter().find(|p| p.id == id && p.open) {
                goals.push((p.start.truncate(), Purpose::Park(p.id)));
            }
        }
        // Otherwise down to the bottom of any lift, where they choose
        // again.
        if goals.is_empty() && !done {
            for l in &self.w.lifts {
                if l.open && !l.broken && lift_suits(l, skill) {
                    goals.push((l.base_queue_point(), Purpose::Lift(l.id)));
                }
            }
        }
        if goals.is_empty() {
            if let Some(e) = self.entrance() {
                goals.push((e.door(self.defs, self.w.level().spacing), Purpose::Exit));
            }
        }
        if goals.is_empty() {
            return;
        }
        let blocked = self.blocked_cells();
        let run = match aim {
            Some(Aim::Run(id)) => Some(id),
            _ => None,
        };
        let bonus = self.run_bonus(skill, run);
        let nav = NavGrid {
            level: self.w.level(),
            blocked: &blocked,
            bonus: Some(&bonus),
        };
        let points: Vec<Vec2> = goals.iter().map(|g| g.0).collect();
        let raw = nav.find_raw(pos, &points, self.ski_mode(skill), 40_000);
        let path = raw.map(|raw| self.s_turns(pos, raw, skill));
        let g = &mut self.w.guests[gi];
        match path {
            Some(path) => {
                let end = *path.last().unwrap();
                let goal = goals
                    .iter()
                    .min_by(|a, b| a.0.distance(end).total_cmp(&b.0.distance(end)))
                    .map(|g| g.1.clone())
                    .unwrap_or(Purpose::Wander);
                // Starting off from a standstill, poling (state 0).
                g.speed = 0.0;
                g.turn = 0.0;
                g.ski_mode = SkiMode::Pole;
                g.state = GuestState::Skiing {
                    path,
                    next: 0,
                    goal,
                };
            }
            None => {
                // No ski route: ride back down is not modeled, so walk.
                let (to, purpose) = goals[0].clone();
                g.state = GuestState::Walking { to, purpose };
                g.thought = Some((Thought::NoRunsOfMySkillLevel, 5.0));
            }
        }
    }

    /// The first need below its threshold, in the original's order.
    fn low_need(&self, gi: usize) -> Option<NeedKind> {
        let g = &self.w.guests[gi];
        NEED_RULES.iter().find(|r| g.need(r.0) < r.1).map(|r| r.0)
    }

    /// What a guest goes looking for, in the original's order (0x488b80):
    /// the first need below its threshold that some building answers, at
    /// a price the guest will pay. Needs nobody answers count against
    /// staying, one each or four when dire; the thought is the last of
    /// them.
    fn wanted(&mut self, gi: usize) -> (Option<Id>, u32, Option<Thought>) {
        let g = &self.w.guests[gi];
        let (needs, cash, tolerance) = (g.needs, g.cash, g.tolerance);
        let mut unmet = 0;
        let mut thought = None;
        for &(k, low, dire, t) in &NEED_RULES {
            let v = needs.get(k);
            if v >= low {
                continue;
            }
            thought = Some(t);
            if let Some(id) = self.find_structure(gi, tolerance, |d| d.provides.get(k) > 0.0) {
                return (Some(id), unmet, thought);
            }
            unmet += if v < dire { 4 } else { 1 };
        }
        if cash < 20.0 {
            thought = Some(Thought::OutOfCash);
            if let Some(id) = self.find_structure(gi, tolerance, |d| d.provides_cash) {
                return (Some(id), unmet, thought);
            }
            unmet += if cash <= 0.001 { 4 } else { 1 };
        }
        // Now and then a guest wants a lesson, beginners most (0x489121).
        const LESSONS: [f32; 4] = [0.5, 0.05, 0.005, 0.00001];
        let g = &self.w.guests[gi];
        let chance = LESSONS[g.skill.index()];
        let (boarder, skill) = (g.boarder, g.skill);
        if self.w.rand() < chance {
            // Only a school with one of its own instructors teaches
            // (0x488a90).
            let staffed: Vec<Id> = self.w.staff.iter().filter(|s| s.kind == StaffKind::Instructor).filter_map(|s| s.home).collect();
            // Up to ten in each instructor's class.
            let room: Vec<Id> = self
                .w
                .structures
                .iter()
                .filter(|s| s.occupants.len() < 10 * staffed.iter().filter(|&&h| h == s.id).count())
                .map(|s| s.id)
                .collect();
            // Schools that teach this guest, chosen as any other building
            // (0x4897f0 need 8), if its classes have room.
            let school = self.find_structure(gi, tolerance, |d| d.category == Category::SkiSchool && serves(d, boarder, skill));
            if let Some(id) = school.filter(|id| room.contains(id)) {
                return (Some(id), unmet, thought);
            }
            thought = Some(Thought::LookingForSkiInstruction);
            unmet += 1;
        }
        (None, unmet, thought)
    }

    /// Litter and yellow snow (0x495cc8). Guests on the move drop litter
    /// on ground the power reaches, one chance in `litter_interval` a
    /// second; and one who cannot wait any longer for a restroom goes in
    /// the snow, which turns yellow unless "Reduce skiers' garbage" is on.
    fn dirty_snow(&mut self, gi: usize, dt: f32) {
        let g = &self.w.guests[gi];
        if !matches!(g.state, GuestState::Walking { .. } | GuestState::Skiing { .. }) {
            return;
        }
        let s = self.w.level().spacing;
        let (cx, cy) = ((g.pos.x / s).floor() as i32, (g.pos.y / s).floor() as i32);
        let Some(i) = self.w.cell_index(cx, cy) else { return };
        let mid = self.w.ground(Vec2::new((cx as f32 + 0.5) * s, (cy as f32 + 0.5) * s));
        let has = |w: &World, yellow: bool| w.litter.iter().any(|l| l.yellow == yellow && l.pos.truncate().distance(mid.truncate()) < 1.0);
        let interval = self.defs.skiers.litter_interval;
        if interval >= 1.0 && self.w.powered_cells.get(i).copied().unwrap_or(false) && self.w.rand() < dt / interval && !has(self.w, false) {
            self.w.litter.push(Litter { pos: mid, yellow: false });
        }
        if self.defs.skiers.pee_on_snow && self.w.guests[gi].needs.bathroom <= 0.01 {
            self.w.guests[gi].needs.bathroom = 1.0;
            if !self.w.options.reduce_garbage && !has(self.w, true) {
                self.w.litter.push(Litter { pos: mid, yellow: true });
            }
        }
    }

    /// Ski classes (0x47e3c0, 0x47e6e0): a class sets off when it has ten
    /// students for each of the school's instructors, or 5400 ticks after
    /// its first student joined; the lesson then ends for all of them.
    fn classes(&mut self) {
        let now = self.w.hour_abs();
        let wait = 5400.0 * TICK * self.hours_per_second();
        for si in 0..self.w.structures.len() {
            let s = &self.w.structures[si];
            let Some(opened) = s.class_opened else { continue };
            let id = s.id;
            let instructors = self.w.staff.iter().filter(|x| x.kind == StaffKind::Instructor && x.home == Some(id)).count();
            if instructors > 0 && s.occupants.len() < 10 * instructors && now < opened + wait && !s.occupants.is_empty() {
                continue;
            }
            self.w.structures[si].class_opened = None;
            // With its instructor gone the class is off, and each student
            // gets the fee back (0x495120 state 0x25).
            let refund = if instructors == 0 { LESSON_FEE } else { 0.0 };
            let mut refunded = 0.0;
            for g in &mut self.w.guests {
                if let GuestState::InStructure { structure, until, .. } = &mut g.state {
                    if *structure == id {
                        *until = now;
                        g.cash += refund;
                        g.spent -= refund;
                        refunded += refund;
                    }
                }
            }
            if refunded > 0.0 {
                self.w.spend(Ledger::Lessons, refunded);
                self.w.structures[si].revenue_today -= refunded;
                self.w.structures[si].revenue_total -= refunded;
            }
        }
    }

    /// Waiting at a full building's door: in as soon as there is room
    /// and the guest is first; patience as in lift lines, except that
    /// from third in line one chance in 30 a tick of giving up (0x489fc0).
    fn at_door(&mut self, gi: usize, sid: Id, tk: f32) {
        let gid = self.w.guests[gi].id;
        let Some(si) = self.w.structures.iter().position(|s| s.id == sid) else {
            self.plan_on_ground(gi);
            return;
        };
        let s = &self.w.structures[si];
        let d = &self.defs.structures[s.def];
        let place = s.waiting.iter().position(|&w| w == gid).unwrap_or(0);
        if place == 0 && (s.occupants.len() as i32) < d.guest_occupancy && s.open && !s.broken {
            self.enter_structure(gi, sid);
            return;
        }
        let before = self.w.guests[gi].waited;
        let after = before + tk;
        self.w.guests[gi].waited = after;
        let mut leave = !s.open || s.broken;
        if !leave && place >= 2 && self.w.rand() < 1.0 - 0.967f32.powf(tk) {
            self.w.guests[gi].thought = Some((Thought::FrustratedLinesTooLong, 4.0));
            leave = true;
        }
        if !leave && after > 1800.0 && (after / 30.0).floor() > (before / 30.0).floor() && self.w.rand() < 0.2 {
            self.w.guests[gi].thought = Some((Thought::MadInlineTooLong, 4.0));
            leave = true;
        }
        if leave {
            self.w.structures[si].waiting.retain(|&w| w != gid);
            self.w.guests[gi].waited = 0.0;
            self.plan_on_ground(gi);
        }
    }

    /// Worn snow recovers: each tick every sixteenth cell, in turn, loses
    /// 1.6 of its wear for the full height of the far corner plus 1000,
    /// so high ground mends faster (0x410b50, 0x40d0c0). Only while it
    /// snows: the weather's +0x210 is the falling kind, 0 snow below
    /// 32 degrees, 1 rain (0x4100e0).
    fn settle_snow(&mut self) {
        if self.w.weather.precip != Some(super::weather::Precip::Snow) {
            return;
        }
        let lv = self.w.level();
        let n = lv.cells_per_side();
        let top = lv.height(n, n) + 1000.0;
        let start = (self.w.tick % 16) as usize;
        let mut i = start;
        while i < n * n {
            let (x, y) = (i % n, i / n);
            let z = self.w.level().height(x, y);
            let c = self.w.level_mut().cell_mut(x, y);
            if c.wear > 0.0 {
                c.wear = (c.wear - 1.6 * z / top).max(0.0);
            }
            i += 16;
        }
    }

    /// A guest coming into a cell wears its snow by one (0x40d390, for
    /// people, object type 4).
    fn tread(&mut self, from: Vec2, to: Vec2) {
        let s = self.w.level().spacing;
        let a = ((from.x / s).floor() as i32, (from.y / s).floor() as i32);
        let b = ((to.x / s).floor() as i32, (to.y / s).floor() as i32);
        if a == b {
            return;
        }
        if let Some(i) = self.w.cell_index(b.0, b.1) {
            let n = self.w.level().cells_per_side();
            let c = self.w.level_mut().cell_mut(i % n, i / n);
            // Kept to 100 (0x40d200).
            c.wear = (c.wear + 1.0).min(100.0);
        }
    }

    /// Walking up a slope steeper than one in five tires a guest
    /// (`EnergyTickUphill` a tick while the way ahead climbs more than 0.2,
    /// 0x491670).
    fn climb(&mut self, gi: usize, to: Vec3, dt: f32) {
        let d = to - self.w.guests[gi].pos;
        let len = d.length();
        if len > 1e-4 && d.z / len > 0.2 {
            let r = &mut self.w.guests[gi].needs.rest;
            *r = (*r + self.defs.skiers.energy_uphill * ticks(dt)).clamp(0.0, 1.0);
        }
    }

    /// How fast a guest walks here: quicker on a path or a building's
    /// cells than on open snow (cell flags 0x1001).
    pub(super) fn walk_speed(&self, pos: Vec2) -> f32 {
        let s = self.w.level().spacing;
        let (cx, cy) = ((pos.x / s).floor() as i32, (pos.y / s).floor() as i32);
        let Some(i) = self.w.cell_index(cx, cy) else { return WALK_SPEED };
        let n = self.w.level().cells_per_side();
        let path = self.w.level().cell(i % n, i / n).path().is_some();
        if path || self.w.occupied[i].is_some() { PATH_SPEED } else { WALK_SPEED }
    }

    /// Picks the next destination for a guest standing on the ground
    /// (0x488b80).
    fn plan_on_ground(&mut self, gi: usize) {
        self.w.guests[gi].aim = None;
        let pos = self.w.guests[gi].pos.truncate();
        let spacing = self.w.level().spacing;
        let defs = self.defs;
        let g = &self.w.guests[gi];
        if g.days_left < 0.0 {
            self.send_home(gi);
            return;
        }
        // A lift ticket dearer than the guest will stand sends them home,
        // as do three needs at their very worst.
        let dear = self.entrance().is_some_and(|e| defs.structures[e.def].dearness(e.price) > g.tolerance);
        let n = &g.needs;
        let worst = [n.bathroom <= 0.01, n.food <= 0.01, n.entertainment <= 0.01, n.rest <= 0.001, n.warmth <= 0.001, g.cash <= 0.001]
            .iter()
            .filter(|&&b| b)
            .count();
        if dear || worst > 2 {
            if dear {
                self.w.guests[gi].thought = Some((Thought::TooExpensive, 5.0));
            }
            self.send_home(gi);
            return;
        }
        // Hurt (0x488d0x): badly enough, to first aid if there is any; a
        // little, home unless the resort has a patrol or first aid.
        let health = g.needs.health;
        if health < 0.7 {
            let tolerance = g.tolerance;
            if let Some(id) = self.find_structure(gi, tolerance, |d| d.provides.health > 0.0) {
                let to = self.w.structure(id).map_or(pos, |s| s.door(defs, spacing));
                self.w.guests[gi].state = GuestState::Walking {
                    to,
                    purpose: Purpose::Structure(id),
                };
                return;
            }
        } else if health < 0.85 {
            let care = self.w.structures.iter().any(|s| {
                matches!(
                    defs.structures[s.def].kind,
                    StructureKind::Medical | StructureKind::SkiPatrol | StructureKind::ChopperPatrol
                )
            });
            if !care {
                self.send_home(gi);
                return;
            }
        }
        let g = &self.w.guests[gi];
        // Skis first if the guest has none; with no rental shop they go.
        if !g.has_equipment {
            let (boarder, skill, tolerance) = (g.boarder, g.skill, g.tolerance);
            if let Some(id) = self.find_structure(gi, tolerance, |d| rents(d, boarder, skill)) {
                let to = self.w.structure(id).map_or(pos, |s| s.door(defs, spacing));
                self.w.guests[gi].state = GuestState::Walking {
                    to,
                    purpose: Purpose::Structure(id),
                };
                return;
            }
            self.w.guests[gi].thought = Some((Thought::LookingForRentals, 5.0));
            self.send_home(gi);
            return;
        }
        let (target, unmet, thought) = self.wanted(gi);
        if let Some(t) = thought {
            self.w.guests[gi].thought = Some((t, 5.0));
        }
        if let Some(id) = target {
            let to = self.w.structure(id).map(|s| s.door(defs, spacing)).unwrap_or(pos);
            self.w.guests[gi].state = GuestState::Walking {
                to,
                purpose: Purpose::Structure(id),
            };
            return;
        }
        if unmet > 2 {
            self.send_home(gi);
            return;
        }
        // Ski: a run, a park or a lift (0x487b00).
        let skill = self.w.guests[gi].skill;
        if let Some((lift, aim)) = self.choose_ski(gi, skill) {
            let to = self.w.lifts.iter().find(|l| l.id == lift).map(|l| l.base_queue_point()).unwrap_or(pos);
            let g = &mut self.w.guests[gi];
            g.aim = aim;
            g.state = GuestState::Walking {
                to,
                purpose: Purpose::Lift(lift),
            };
        }
    }

    /// The lift that brings skiers to a run or park starting at `at`
    /// (0x42c5xx): the one whose top station is nearest, of those whose
    /// top is no more than 20 below it.
    fn feeder(&self, at: Vec3) -> Option<&Lift> {
        self.w
            .lifts
            .iter()
            .filter(|l| at.z - l.end.z < 20.0)
            .min_by(|a, b| a.end.distance_squared(at).total_cmp(&b.end.distance_squared(at)))
    }

    /// What to ski next, as 0x487b00 chooses at the bottom: the lift to
    /// ride and what for. Runs within one level of the skier's, lifts
    /// whose steepness suits them and parks they may ride are the
    /// candidates. With a park among them, three times in ten it is a
    /// park; otherwise a run if any, of the skier's own level if there is
    /// one, else the next easier, and so on; failing runs, a lift. Each is
    /// scored by the line for its lift (0x489580: per person waiting, four
    /// times the seconds between chairs per seat, but at least six; parks
    /// twenty a rider) plus a random 0 to 1, lowest taken, and 100
    /// or more never: so a run without a lift, or with a broken or closed
    /// one, is never chosen.
    fn choose_ski(&mut self, gi: usize, skill: Skill) -> Option<(Id, Option<Aim>)> {
        let boarder = self.w.guests[gi].boarder;
        let si = skill.index() as i32;
        let defs = self.defs;
        let lift_score = |l: &Lift| {
            let d = &defs.lifts[l.def];
            let each = (4.0 * d.secs_between_chairs / d.capacity.max(1) as f32).max(6.0);
            if l.open && !l.broken { l.queue.len() as f32 * each } else { 100.0 }
        };
        let mut runs: Vec<(i32, f32, Id, Id)> = Vec::new();
        for r in self.w.runs.iter().filter(|r| r.open) {
            let diff = self.defs.runs.get(r.def).map(|d| d.difficulty).unwrap_or(0);
            if (diff - si).abs() >= 2 {
                continue;
            }
            let Some(&top) = r.gates.first() else { continue };
            let (score, lift) = self.feeder(top).map_or((100.0, 0), |l| (lift_score(l), l.id));
            runs.push((diff, score, lift, r.id));
        }
        let lifts: Vec<(f32, Id)> = self.w.lifts.iter().filter(|l| lift_suits(l, skill)).map(|l| (lift_score(l), l.id)).collect();
        // The jump takes anyone, the half pipe boarders and the race
        // course skiers (0x458c95: +0xbc 0, 1 and 2). A park's line I
        // count as those on their way to it.
        let rides = |def: usize| match def {
            1 => boarder,
            2 => !boarder,
            _ => true,
        };
        let mut parks: Vec<(f32, Id, Id)> = Vec::new();
        for p in self.w.parks.iter().filter(|p| rides(p.def)) {
            let usable = p.open && self.feeder(p.start).is_some_and(|l| l.open && !l.broken);
            let line = self.w.guests.iter().filter(|g| g.aim == Some(Aim::Park(p.id))).count() as f32;
            let lift = self.feeder(p.start).map_or(0, |l| l.id);
            parks.push((if usable { line * 20.0 } else { 100.0 }, lift, p.id));
        }
        if runs.is_empty() && lifts.is_empty() && parks.is_empty() {
            self.w.guests[gi].thought = Some((Thought::NoRunsOfMySkillLevel, 5.0));
            return None;
        }
        let pick = |w: &mut World, c: &mut dyn Iterator<Item = (f32, Id, Option<Aim>)>| -> Option<(Id, Option<Aim>)> {
            let mut best: Option<(f32, Id, Option<Aim>)> = None;
            for (score, lift, aim) in c {
                let s = score + w.rand();
                if s < best.map_or(100.0, |b| b.0) {
                    best = Some((s, lift, aim));
                }
            }
            best.map(|b| (b.1, b.2))
        };
        let chosen = if !parks.is_empty() && ((runs.is_empty() && lifts.is_empty()) || self.w.rand() < 0.3) {
            pick(&mut self.w, &mut parks.iter().map(|&(s, l, p)| (s, l, Some(Aim::Park(p)))))
        } else if !runs.is_empty() {
            let level = (0..4)
                .map(|k| si - k)
                .find(|d| runs.iter().any(|r| r.0 == *d));
            pick(
                &mut self.w,
                &mut runs.iter().filter(|r| level.is_none_or(|d| r.0 == d)).map(|&(_, s, l, r)| (s, l, Some(Aim::Run(r)))),
            )
        } else {
            pick(&mut self.w, &mut lifts.iter().map(|&(s, l)| (s, l, None)))
        };
        if chosen.is_none() {
            self.w.guests[gi].thought = Some((Thought::MadLiftsTooCrowded, 5.0));
        }
        chosen
    }

    /// When guests leave, the original counts them by what was on their
    /// mind and, every so many, reports it in the message box (0x485f60;
    /// table 0x651fe0: thought category, guests per report, message).
    fn departure_report(&mut self, gi: usize) {
        // The original's table order, which `leave_counts` follows.
        const REPORTED: [Thought; 12] = [
            Thought::NeedToWeeWee,
            Thought::Hungry,
            Thought::Cold,
            Thought::Tired,
            Thought::NoRunsOfMySkillLevel,
            Thought::MadLiftsTooCrowded,
            Thought::LookingForRentals,
            Thought::LookingForSkiInstruction,
            Thought::FrustratedLinesTooLong,
            Thought::Bored,
            Thought::OutOfCash,
            Thought::TooExpensive,
        ];
        let Some((thought, _)) = self.w.guests[gi].thought else { return };
        let Some((every, msg)) = thought.departure_report() else { return };
        let Some(k) = REPORTED.iter().position(|t| *t == thought) else { return };
        if self.w.leave_counts.len() < REPORTED.len() {
            self.w.leave_counts.resize(REPORTED.len(), 0);
        }
        self.w.leave_counts[k] += 1;
        if self.w.leave_counts[k] >= every {
            self.w.leave_counts[k] = 0;
            let text = self.defs.text(msg).to_string();
            self.w.message(text, None);
        }
    }

    /// The dead skier rises as an angel, 0.33 a tick (0x48ee70 state 8),
    /// then the death is reported and, three times in ten, the family
    /// sues for a tenth of the resort's cash, at most $10,000.
    fn dying(&mut self, gi: usize, t: f32, dt: f32) {
        // The angel rises for 400 ticks (0x49698b).
        const RISE: f32 = 400.0 * TICK;
        let g = &mut self.w.guests[gi];
        g.pos.z += 0.33 * dt / TICK;
        g.state = GuestState::Dying { t: t + dt };
        if t + dt < RISE {
            return;
        }
        g.state = GuestState::Gone;
        let text = self.defs.text(Text::ReSkierDeath).to_string();
        self.w.message(text, None);
        if self.w.rand() < 0.3 {
            let fee = (self.w.cash * 0.1).clamp(0.0, 10_000.0);
            self.w.spend(Ledger::Legal, fee);
            let text = self.defs.text(Text::ReSued).to_string();
            self.w.message(text, None);
        }
    }

    fn send_home(&mut self, gi: usize) {
        self.departure_report(gi);
        let spacing = self.w.level().spacing;
        let exit = self.entrance().map(|e| e.door(self.defs, spacing));
        let g = &mut self.w.guests[gi];
        match exit {
            Some(to) => {
                g.state = GuestState::Walking {
                    to,
                    purpose: Purpose::Exit,
                }
            }
            None => g.state = GuestState::Gone,
        }
    }

    /// The nearest working building that `pred` accepts at a price no
    /// dearer than `tolerance` (0x48988b).
    /// The building a guest goes to for something (0x4897f0): of the
    /// open, working, powered ones `pred` accepts at a price no dearer
    /// than `tolerance`, the one with the shortest line, give or take:
    /// twelve a guest waiting at its door (sixty at a lodge; the list at
    /// +0xc4 that guests in line, state 6, wait on) plus a random 0 to 1,
    /// lowest taken, 100 or more never (0x489580). How far it is does not
    /// matter. With none, a guest who found one too dear thinks it too
    /// expensive.
    fn find_structure(&mut self, gi: usize, tolerance: f32, pred: impl Fn(&super::defs::StructureDef) -> bool) -> Option<Id> {
        let mut best: Option<(f32, Id)> = None;
        let mut dear = false;
        let mut lines: std::collections::HashMap<Id, f32> = std::collections::HashMap::new();
        for g in &self.w.guests {
            if let GuestState::AtDoor { structure } = g.state {
                *lines.entry(structure).or_default() += 1.0;
            }
        }
        for i in 0..self.w.structures.len() {
            let s = &self.w.structures[i];
            let d = &self.defs.structures[s.def];
            if !pred(d) {
                continue;
            }
            if d.dearness(s.price) > tolerance {
                dear = true;
                continue;
            }
            if !(s.open && !s.broken && (s.powered || !d.uses_electricity)) {
                continue;
            }
            let each = if d.category == Category::Lodging { 60.0 } else { 12.0 };
            let (busy, id) = (lines.get(&s.id).copied().unwrap_or(0.0) * each, s.id);
            let score = busy + self.w.rand();
            if score < best.map_or(100.0, |b| b.0) {
                best = Some((score, id));
            }
        }
        if best.is_none() && dear {
            self.w.guests[gi].thought = Some((Thought::TooExpensive, 5.0));
        }
        best.map(|b| b.1)
    }

    fn guests(&mut self, dt: f32) {
        let tk = ticks(dt);
        let defs = self.defs;
        // The lift ticket: the entrance's daily charge, a little each tick
        // from every guest with money left (0x496d0f).
        let daily = self.entrance().map_or(0.0, |e| e.daily_fee.unwrap_or(defs.structures[e.def].recurring_per_day));
        let fee = daily * dt / defs.resort.seconds_per_day;
        if fee > 0.0 {
            let mut paid = 0.0;
            for g in self.w.guests.iter_mut().filter(|g| g.is_active() && g.cash > 0.0) {
                g.cash -= fee;
                g.spent += fee;
                paid += fee;
            }
            if paid > 0.0 {
                self.w.earn(Ledger::Tickets, paid);
                if let Some(e) = self.w.structures.iter_mut().find(|s| defs.structures[s.def].category == Category::Entrance) {
                    e.revenue_today += paid;
                    e.revenue_total += paid;
                }
            }
        }
        // Outdoors, rain or a thaw above 32°F bores guests, and they chill
        // in proportion to how near the temperature is to the ini minimum;
        // a weather station softens both by a quarter (0x491870).
        let wx = &self.w.weather;
        let r = &defs.resort;
        let station = self.w.structures.iter().any(|st| defs.structures[st.def].kind == super::defs::StructureKind::WeatherStation);
        let soften = if station { 0.75 } else { 1.0 };
        let bad = wx.precip == Some(super::weather::Precip::Rain) || wx.temperature > 32.0;
        let span = if r.max_temperature == r.min_temperature { 1.0 } else { 1.0 / (r.max_temperature - r.min_temperature) };
        let chill = (1.0 - (wx.temperature - r.min_temperature) * span) * soften;
        for gi in 0..self.w.guests.len() {
            if !self.w.guests[gi].is_active() {
                continue;
            }
            // Needs, and the stay running out a tick at a time (0x487a1f).
            {
                let g = &mut self.w.guests[gi];
                g.days_left -= dt / r.seconds_per_day;
                let rates = match g.state {
                    GuestState::Queued { .. } | GuestState::AtDoor { .. } => defs.skiers.tick_in_line,
                    GuestState::Riding { lift } => {
                        let mut r = defs.skiers.tick_on_lift;
                        let seats = self.w.lifts.iter().find(|l| l.id == lift).map_or(0, |l| defs.lifts[l.def].capacity);
                        if seats == 1 {
                            r.rest = defs.skiers.energy_on_ropetow;
                        }
                        if seats >= 5 {
                            r.warmth = defs.skiers.warmth_on_gondola;
                        }
                        r
                    }
                    GuestState::InStructure { .. } => Needs::default(),
                    _ => defs.skiers.tick_normal,
                };
                for k in ALL_NEEDS {
                    let mut r = rates.get(k);
                    let outdoors = !matches!(g.state, GuestState::InStructure { .. });
                    if k == NeedKind::Warmth && outdoors {
                        r += defs.skiers.cold_weather_warmth * chill;
                    }
                    if k == NeedKind::Entertainment && outdoors && bad {
                        r -= defs.skiers.bad_weather_boredom * soften;
                    }
                    let v = g.needs.get_mut(k);
                    *v = (*v + r * tk).clamp(0.0, 1.0);
                }
                g.happiness = g.needs.happiness();
                if let Some((_, t)) = &mut g.thought {
                    *t -= dt;
                    if *t <= 0.0 {
                        g.thought = None;
                    }
                }
            }
            let state = self.w.guests[gi].state.clone();
            match state {
                GuestState::Walking { to, purpose } => self.walk(gi, to, purpose, dt),
                GuestState::InStructure { structure, until, need } => {
                    self.serve(gi, structure, dt);
                    let met = match need {
                        Some(k) => self.w.guests[gi].need(k) >= 1.0,
                        None => self.w.hour_abs() >= until,
                    };
                    if met {
                        self.leave_structure(gi, structure);
                    }
                }
                GuestState::Queued { lift } => {
                    // Stand in line along the lift's queue path.
                    let gid = self.w.guests[gi].id;
                    if let Some((p, dir)) = self
                        .w
                        .lift(lift)
                        .and_then(|l| l.queue.iter().position(|q| *q == gid).and_then(|k| l.queue_spot(k)))
                    {
                        let g = &mut self.w.guests[gi];
                        g.pos = p;
                        g.heading = dir.y.atan2(dir.x);
                    }
                    // Patience in line (0x489fc0): a broken lift sends
                    // everyone off; 21st or further back, one chance in 30 a
                    // tick of giving up; after a minute, one in five each
                    // second.
                    let before = self.w.guests[gi].waited;
                    let after = before + tk;
                    self.w.guests[gi].waited = after;
                    let place = self.w.lift(lift).and_then(|l| l.queue.iter().position(|q| *q == gid));
                    let mut leave = self.w.lift(lift).is_none_or(|l| !l.open || l.broken);
                    if !leave && place.is_some_and(|k| k >= 20) && self.w.rand() < 1.0 - 0.967f32.powf(tk) {
                        self.w.guests[gi].thought = Some((Thought::FrustratedLinesTooLong, 4.0));
                        leave = true;
                    }
                    if !leave && after > 1800.0 && (after / 30.0).floor() > (before / 30.0).floor() && self.w.rand() < 0.2 {
                        self.w.guests[gi].thought = Some((Thought::MadInlineTooLong, 4.0));
                        leave = true;
                    }
                    if leave {
                        if let Some(l) = self.w.lift_mut(lift) {
                            l.queue.retain(|q| *q != gid);
                        }
                        self.w.guests[gi].waited = 0.0;
                        self.plan_on_ground(gi);
                    }
                }
                GuestState::AtDoor { structure } => self.at_door(gi, structure, tk),
                GuestState::Riding { lift } => {
                    let gid = self.w.guests[gi].id;
                    let pos = self.w.lift(lift).and_then(|l| {
                        l.chairs.iter().find(|c| c.riders.contains(&gid)).map(|c| {
                            let k = c.riders.iter().position(|r| *r == gid).unwrap_or(0) as f32;
                            (l.point_at(c.s), k - (c.riders.len() as f32 - 1.0) * 0.5)
                        })
                    });
                    if let Some(((p, dir), side)) = pos {
                        // Riders of drag lifts stay on the snow under the
                        // cable, side by side.
                        let drag = self.w.lift(lift).is_some_and(|l| self.defs.lifts[l.def].show_skiers);
                        let across = Vec2::new(-dir.y, dir.x).normalize_or_zero() * side * 1.6;
                        let p = if drag { self.w.ground(p.truncate() + across) } else { p };
                        let g = &mut self.w.guests[gi];
                        g.pos = p;
                        g.heading = dir.y.atan2(dir.x);
                    }
                }
                GuestState::Skiing { path, next, goal } => self.ski(gi, path, next, goal, dt),
                GuestState::Crashed { until } => {
                    if self.w.hour_abs() >= until {
                        self.plan_from_top(self.w.guests[gi].id);
                    }
                }
                GuestState::Seized | GuestState::Held => {}
                GuestState::Injured => {
                    // Left unrescued for 5000 ticks, an injured skier dies
                    // (0x49692e); the tick rate is taken as the sim's.
                    let gid = self.w.guests[gi].id;
                    let helped = self.w.staff.iter().any(|s| s.task == StaffTask::Rescue(gid) || matches!(s.task, StaffTask::Aid { guest, .. } if guest == gid));
                    let g = &mut self.w.guests[gi];
                    if !helped {
                        g.injured_for += dt;
                        if g.injured_for > 5000.0 * TICK {
                            g.state = GuestState::Dying { t: 0.0 };
                        }
                    }
                }
                GuestState::Dying { t } => self.dying(gi, t, dt),
                GuestState::Rescued | GuestState::Controlled => {}
                GuestState::Leaving => {
                    self.w.guests[gi].state = GuestState::Gone;
                }
                GuestState::Gone => {}
            }
            self.dirty_snow(gi, dt);
        }
        // Departures free beds and drop gone guests.
        let before = self.w.guests.len();
        let gone: Vec<Id> = self
            .w
            .guests
            .iter()
            .filter(|g| matches!(g.state, GuestState::Gone))
            .map(|g| g.id)
            .collect();
        if !gone.is_empty() {
            for s in &mut self.w.structures {
                s.occupants.retain(|o| !gone.contains(o));
                s.waiting.retain(|o| !gone.contains(o));
            }
            for l in &mut self.w.lifts {
                l.queue.retain(|o| !gone.contains(o));
            }
            self.w.guests.retain(|g| !gone.contains(&g.id));
        }
        let _ = before;
    }

    fn walk(&mut self, gi: usize, to: Vec2, purpose: Purpose, dt: f32) {
        let pos = self.w.guests[gi].pos.truncate();
        // Long walks over rough ground follow a walking route: walk to the
        // next waypoint, keeping the real destination for later.
        if pos.distance(to) > 120.0 && dt > 0.0 {
            let gid = self.w.guests[gi].id;
            let cached = self.w.walk_routes.iter().position(|r| r.0 == gid && r.1.last() == Some(&to));
            if cached.is_none() {
                let blocked = self.blocked_cells();
                let nav = NavGrid {
                    level: self.w.level(),
                    blocked: &blocked,
                    bonus: None,
                };
                // Routes end at the destination itself, which also keys the
                // cache; without a route, walk straight there rather than
                // search again every tick.
                let mut path = nav.find(pos, &[to], Mode::Walk, 20_000).unwrap_or_else(|| vec![pos]);
                if path.last() != Some(&to) {
                    path.push(to);
                }
                self.w.walk_routes.retain(|r| r.0 != gid);
                self.w.walk_routes.push((gid, path));
            }
            if let Some(ri) = self.w.walk_routes.iter().position(|r| r.0 == gid) {
                let route = &mut self.w.walk_routes[ri].1;
                while route.len() > 1 && route[0].distance(pos) < 3.0 {
                    route.remove(0);
                }
                if let Some(&wp) = route.first() {
                    let d = wp - pos;
                    let step = self.walk_speed(pos) * dt;
                    let n = d.normalize_or_zero();
                    let np = if d.length() > step { pos + n * step } else { wp };
                    let p = self.w.ground(np);
                    self.climb(gi, p, dt);
                    self.tread(pos, np);
                    let g = &mut self.w.guests[gi];
                    g.pos = p;
                    g.heading = n.y.atan2(n.x);
                    g.speed = WALK_SPEED;
                    g.anim += dt;
                    return;
                }
            }
        }
        let d = to - pos;
        let dist = d.length();
        let step = self.walk_speed(pos) * dt;
        if dist > step {
            let n = d / dist;
            let np = pos + n * step;
            let p = self.w.ground(np);
            self.climb(gi, p, dt);
            self.tread(pos, np);
            let g = &mut self.w.guests[gi];
            g.pos = p;
            g.heading = n.y.atan2(n.x);
            g.speed = WALK_SPEED;
            g.anim += dt;
            return;
        }
        let p = self.w.ground(to);
        self.w.guests[gi].pos = p;
        self.w.guests[gi].speed = 0.0;
        let gid = self.w.guests[gi].id;
        self.w.walk_routes.retain(|r| r.0 != gid);
        match purpose {
            Purpose::Wander => self.plan_on_ground(gi),
            Purpose::Exit => self.w.guests[gi].state = GuestState::Leaving,
            Purpose::Lift(id) => match self.w.lifts.iter_mut().find(|l| l.id == id) {
                Some(l) if l.open && !l.broken => {
                    let gid = self.w.guests[gi].id;
                    l.queue.push_back(gid);
                    self.w.guests[gi].state = GuestState::Queued { lift: id };
                    self.w.guests[gi].waited = 0.0;
                }
                _ => self.plan_on_ground(gi),
            },
            Purpose::Structure(id) => self.enter_structure(gi, id),
            Purpose::Park(_) | Purpose::Continue => {
                let id = self.w.guests[gi].id;
                self.plan_from_top(id);
            }
        }
    }

    fn enter_structure(&mut self, gi: usize, sid: Id) {
        let defs = self.defs;
        let Some(si) = self.w.structures.iter().position(|s| s.id == sid) else {
            self.plan_on_ground(gi);
            return;
        };
        let d = &defs.structures[self.w.structures[si].def];
        let gid = self.w.guests[gi].id;
        // A building serves `guest_occupancy` at once; the rest wait at
        // the door (0x4b5653).
        {
            let s = &self.w.structures[si];
            let front = s.waiting.first().is_none_or(|&w| w == gid);
            if s.occupants.len() as i32 >= d.guest_occupancy || !front {
                let s = &mut self.w.structures[si];
                if !s.waiting.contains(&gid) {
                    s.waiting.push(gid);
                    self.w.guests[gi].waited = 0.0;
                }
                self.w.guests[gi].state = GuestState::AtDoor { structure: sid };
                return;
            }
            self.w.structures[si].waiting.retain(|&w| w != gid);
        }
        // A lesson costs a flat $85 whatever the school charges (0x495120).
        let price = if d.category == Category::SkiSchool { LESSON_FEE } else { self.w.structures[si].price };
        if price > 0.0 {
            if self.w.guests[gi].cash < price {
                self.w.guests[gi].thought = Some((Thought::OutOfCash, 4.0));
                self.plan_on_ground(gi);
                return;
            }
            // Too dear for this guest: they think so and go (0x4943d2).
            if d.dearness(self.w.structures[si].price) > self.w.guests[gi].tolerance {
                self.w.guests[gi].thought = Some((Thought::TooExpensive, 4.0));
                self.plan_on_ground(gi);
                return;
            }
            self.w.guests[gi].cash -= price;
            self.w.guests[gi].spent += price;
            self.w.earn(sales_ledger(d.category), price);
            let s = &mut self.w.structures[si];
            s.revenue_today += price;
            s.revenue_total += price;
        }
        let (boarder, skill) = (self.w.guests[gi].boarder, self.w.guests[gi].skill);
        if !self.w.guests[gi].has_equipment && rents(d, boarder, skill) {
            self.w.guests[gi].has_equipment = true;
            self.w.guests[gi].rented_from = Some(sid);
        }
        // A lesson leaves the guest's level as it was: the original only
        // adds to a progress figure nothing reads, and its level-up code
        // (0x494455, top state 8) is never reached.
        let g = &mut self.w.guests[gi];
        match g.uses.iter_mut().find(|(s, _)| *s == sid) {
            Some(u) => u.1 += 1,
            None => g.uses.push((sid, 1)),
        }
        let s = &mut self.w.structures[si];
        s.visits_today += 1;
        s.occupants.push(gid);
        // Guests stay until what brought them in is met (0x493d90): the
        // need this building answers that is lowest, as long as it does
        // answer it; otherwise, like a rental, for 80 ticks.
        let g = &self.w.guests[gi];
        let need = ALL_NEEDS
            .into_iter()
            .filter(|&k| d.provides.get(k) > 0.0 && d.ticks.get(k) > 0.0 && g.need(k) < 1.0)
            .min_by(|&a, &b| g.need(a).total_cmp(&g.need(b)));
        let mut until = self.w.hour_abs() + 80.0 * TICK * self.hours_per_second();
        let mut need = need;
        if d.category == Category::SkiSchool {
            // Students wait for the class to set off (see `classes`).
            until = f32::INFINITY;
            need = None;
            let now = self.w.hour_abs();
            self.w.structures[si].class_opened.get_or_insert(now);
        }
        self.w.guests[gi].state = GuestState::InStructure { structure: sid, until, need };
        let g = &mut self.w.guests[gi];
        g.pos.z = -1000.0;
    }

    fn serve(&mut self, gi: usize, sid: Id, dt: f32) {
        let Some(def) = self.w.structure(sid).map(|s| s.def) else { return };
        // Its daily charge, a little each tick while inside (0x49173b).
        let d = &self.defs.structures[def];
        let fee = d.recurring_per_day * dt / self.defs.resort.seconds_per_day;
        if fee > 0.0 {
            self.w.guests[gi].cash -= fee;
            self.w.guests[gi].spent += fee;
            self.w.earn(sales_ledger(d.category), fee);
            if let Some(s) = self.w.structure_mut(sid) {
                s.revenue_today += fee;
                s.revenue_total += fee;
            }
        }
        // Its `tick_` effects, each tick (0x491670).
        let ticks_ = self.defs.structures[def].ticks;
        let tk = ticks(dt);
        let g = &mut self.w.guests[gi];
        for k in ALL_NEEDS {
            let v = g.needs.get_mut(k);
            *v = (*v + ticks_.get(k) * tk).clamp(0.0, 1.0);
        }
    }

    fn leave_structure(&mut self, gi: usize, sid: Id) {
        let spacing = self.w.level().spacing;
        let gid = self.w.guests[gi].id;
        let door = match self.w.structure_mut(sid) {
            Some(s) => {
                s.occupants.retain(|o| *o != gid);
                Some(s.clone())
            }
            None => None,
        };
        if let Some(s) = door {
            let p = self.w.ground(s.door(self.defs, spacing));
            self.w.guests[gi].pos = p;
        } else {
            let p = self.w.guests[gi].pos;
            self.w.guests[gi].pos = self.w.ground(p.truncate());
        }
        self.plan_on_ground(gi);
    }

    fn ski(&mut self, gi: usize, path: Vec<Vec2>, mut next: usize, goal: Purpose, dt: f32) {
        let defs = self.defs;
        let skill = self.w.guests[gi].skill;
        let st = defs.skiers.skill[skill.index()];
        let pos = self.w.guests[gi].pos.truncate();
        let Some(&target) = path.get(next) else {
            self.arrive_after_ski(gi, goal);
            return;
        };
        let d = target - pos;
        let dist = d.length();
        let level = self.w.level.take().expect("level");
        // The original's skier (0x48ee70), by the drop over the next unit
        // and the slope table at 0x65c140.
        let dir = if dist > 0.0 { d / dist } else { Vec2::X };
        let ahead = pos + dir;
        let drop = level.height_at(pos.x, pos.y) - level.height_at(ahead.x, ahead.y);
        let g = &mut self.w.guests[gi];
        for _ in 0..ticks(dt).round().max(1.0) as usize {
            let v = ski_tick(g.speed * TICK, drop, g.turn, &mut g.ski_mode, SKILL_SPEED[skill.index()] * TICK);
            g.speed = v / TICK;
        }
        let step = g.speed * dt;
        if dist <= step.max(1.0) {
            next += 1;
            g.pos = Vec3::new(target.x, target.y, level.height_at(target.x, target.y));
            // Aiming at the next waypoint: the turn is about the sine of
            // the angle between the old way and the new (0x48e600).
            if let Some(&after) = path.get(next) {
                g.turn = dir.perp_dot((after - target).normalize_or_zero());
            }
        } else {
            let n = d / dist;
            let np = pos + n * step;
            g.pos = Vec3::new(np.x, np.y, level.height_at(np.x, np.y));
            let h = n.y.atan2(n.x);
            g.heading = h;
        }
        g.anim += dt * (g.speed / 10.0).max(0.5);
        let cell = (
            (g.pos.x / level.spacing) as i32,
            (g.pos.y / level.spacing) as i32,
        );
        let last = level.cells_per_side() as i32 - 1;
        let moguls = level.cell(cell.0.clamp(0, last) as usize, cell.1.clamp(0, last) as usize).flags & cell_flags::MOGULS != 0;
        self.w.level = Some(level);
        // Falls (0x48fb15, 0x48fd78): moguls trip skiers at many times
        // the ini chance, the less skilled the more; elsewhere only going
        // faster than the skier is used to risks it.
        const MOGULS: [f32; 4] = [50.0, 30.0, 20.0, 10.0];
        let per_tick = if moguls {
            st.probability_crash * MOGULS[skill.index()]
        } else if self.w.guests[gi].speed > SKILL_SPEED[skill.index()] {
            st.probability_crash
        } else {
            0.0
        };
        let p_crash = 1.0 - (1.0 - per_tick.min(1.0)).powf(ticks(dt));
        let crash_roll = self.w.rand();
        let injury_roll = self.w.rand();
        let injury = self.w.rand();
        let lie = self.w.rand();
        let now = self.w.guests[gi].pos.truncate();
        self.tread(pos, now);
        let g = &mut self.w.guests[gi];
        if crash_roll < p_crash {
            // A fall may hurt, anything up to fully; worse than 0.8 the
            // skier stays down for the patrol, otherwise gets up after 60
            // to 99 ticks (0x48f5xx).
            g.speed = 0.0;
            if injury_roll <= st.probability_injury {
                g.needs.health = 1.0 - injury;
            }
            if g.needs.health < 0.2 {
                g.state = GuestState::Injured;
                g.thought = Some((Thought::WoundedWaiting, 30.0));
                return;
            }
            let ticks_down = 60.0 + (lie * 40.0).floor();
            let until = self.w.hour_abs() + ticks_down * TICK * self.hours_per_second();
            let g = &mut self.w.guests[gi];
            g.state = GuestState::Crashed { until };
            g.thought = Some((Thought::Wounded, 3.0));
            return;
        }
        // Going fast for one's level is fun (`BoredomTickFast`, 0x4918b0).
        if g.speed >= SKILL_SPEED[skill.index()] {
            let e = &mut g.needs.entertainment;
            *e = (*e - defs.skiers.boredom_fast * ticks(dt)).clamp(0.0, 1.0);
        }
        g.state = GuestState::Skiing { path, next, goal };
    }

    /// Where a patroller takes those it rescues: its own base's door, or
    /// the nearest patrol building's.
    fn rescue_base(&self, si: usize) -> Option<Vec2> {
        let spacing = self.w.level().spacing;
        let defs = self.defs;
        let s = &self.w.staff[si];
        let home = s.home.and_then(|h| self.w.structure(h)).filter(|b| defs.structures[b.def].category == Category::SkiPatrol);
        let p = s.pos.truncate();
        home.map(|b| b.door(defs, spacing)).or_else(|| {
            self.w
                .structures
                .iter()
                .filter(|b| defs.structures[b.def].category == Category::SkiPatrol)
                .map(|b| b.door(defs, spacing))
                .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
        })
    }

    /// The turns a skier makes down the route (0x430ee0): every 3 to 7
    /// cells, unless the stretch crosses trees or a footpath, the way is
    /// two cubic curves, each swinging out to one side by its length
    /// times the drop over the stretch's length times a skill factor
    /// (beginners 0 to 6 at random, then 1 to 5, 4 to 5, and experts 3).
    /// The original's side-matching branch never runs (its flag stays 0),
    /// so every curve swings the same way. Five points a curve.
    fn s_turns(&mut self, start: Vec2, raw: Vec<Vec2>, skill: Skill) -> Vec<Vec2> {
        let mut pts = vec![start];
        pts.extend(raw);
        let n = pts.len();
        let mut out = Vec::new();
        let base = 3 + (self.w.rand() * 3.0) as usize % 3;
        let mut i = 0;
        while i + 1 < n {
            let j = (i + base + (self.w.rand() * 3.0) as usize % 3).min(n - 1);
            let rough = pts[i..=j].iter().any(|p| {
                let s = self.w.level().spacing;
                let (x, y) = ((p.x / s) as i32, (p.y / s) as i32);
                self.w.cell_index(x, y).is_some_and(|k| {
                    let m = self.w.level().cells_per_side();
                    let c = self.w.level().cell(k % m, k / m);
                    c.flags & cell_flags::FOREST != 0 || c.foliage != 0 || c.path().is_some()
                })
            });
            if rough {
                out.extend_from_slice(&pts[i + 1..=j]);
                i = j;
                continue;
            }
            let (a, b) = (pts[i], pts[j]);
            let (za, zb) = (self.w.ground(a).z, self.w.ground(b).z);
            // Octagonal length, as the original measures.
            let d = [(a.x - b.x).abs(), (a.y - b.y).abs(), (za - zb).abs()];
            let mut s = d;
            s.sort_by(|p, q| q.total_cmp(p));
            let len = (s[0] + (s[1] + s[2]) * 0.25).max(1.0);
            let slope = (za - zb) / len;
            for h in 0..2 {
                let r = self.w.rand();
                let swing = if slope > 0.0 {
                    let k = match skill.index() {
                        0 => r * 6.0,
                        1 => r * 4.0 + 1.0,
                        2 => r + 4.0,
                        _ => 3.0,
                    };
                    k * slope
                } else {
                    0.0
                };
                let p0 = a + (b - a) * (h as f32 * 0.5);
                let p3 = a + (b - a) * ((h + 1) as f32 * 0.5);
                let perp = Vec2::new(p0.y - p3.y, p3.x - p0.x);
                let c1 = p0 + (p3 - p0) * 0.25;
                let q3 = p0 + (p3 - p0) * 0.75;
                let (z0, z3) = (self.w.ground(p0).z, self.w.ground(p3).z);
                let c2 = self.swing_to(q3, q3 + perp * swing, z0.min(z3), z0.max(z3));
                let edge = self.w.level().cells_per_side() as f32 * self.w.level().spacing - 1.0;
                for k in 1..=5 {
                    let t = k as f32 * 0.2;
                    let u = 1.0 - t;
                    let p = p0 * (u * u * u) + c1 * (3.0 * u * u * t) + c2 * (3.0 * u * t * t) + p3 * (t * t * t);
                    out.push(p.clamp(Vec2::splat(1.0), Vec2::splat(edge)));
                }
            }
            i = j;
        }
        if out.is_empty() {
            out.push(*pts.last().unwrap());
        }
        out
    }

    /// How far a turn may swing out from `from` toward `to` (0x430c30):
    /// up to the first cell with trees or a footpath, or to where the
    /// ground leaves the heights between `lo` and `hi`.
    fn swing_to(&self, from: Vec2, to: Vec2, lo: f32, hi: f32) -> Vec2 {
        let s = self.w.level().spacing;
        let len = from.distance(to);
        let steps = (len / (s * 0.5)).ceil().max(1.0) as usize;
        let mut last = from;
        let mut last_z = self.w.ground(from).z;
        for k in 1..=steps {
            let p = from + (to - from) * (k as f32 / steps as f32);
            let (x, y) = ((p.x / s) as i32, (p.y / s) as i32);
            let Some(i) = self.w.cell_index(x, y) else { return last };
            let m = self.w.level().cells_per_side();
            let c = self.w.level().cell(i % m, i / m);
            if c.flags & cell_flags::FOREST != 0 || c.foliage != 0 || c.path().is_some() {
                return last;
            }
            let z = self.w.ground(p).z;
            if z > hi || z < lo {
                let edge = if z > hi { hi } else { lo };
                let t = ((edge - last_z) / (z - last_z + 0.1)).clamp(0.0, 1.0);
                return last + (p - last) * t;
            }
            last = p;
            last_z = z;
        }
        to
    }

    /// Hands a guest back to the AI after player control.
    pub fn release(&mut self, gid: Id) {
        if let Some(g) = self.w.guests.iter_mut().find(|g| g.id == gid) {
            g.state = GuestState::Walking {
                to: g.pos.truncate(),
                purpose: Purpose::Continue,
            };
        }
    }

    fn arrive_after_ski(&mut self, gi: usize, goal: Purpose) {
        if let Purpose::Park(id) = goal {
            if let Some(p) = self.w.parks.iter_mut().find(|p| p.id == id) {
                p.riders_today += 1;
                p.riders_total += 1;
                let end = p.end.truncate();
                let g = &mut self.w.guests[gi];
                g.thought = Some((Thought::Skiing, 4.0));
                g.state = GuestState::Skiing {
                    path: vec![end],
                    next: 0,
                    goal: Purpose::Continue,
                };
                return;
            }
        }
        if goal == Purpose::Continue {
            let id = self.w.guests[gi].id;
            self.plan_from_top(id);
            return;
        }
        let g = &mut self.w.guests[gi];
        g.runs_today += 1;
        g.speed = 0.0;
        let pos = g.pos.truncate();
        for r in &mut self.w.runs {
            if r.gates.iter().any(|p| p.truncate().distance(pos) < 200.0) {
                r.skiers_today += 1;
                break;
            }
        }
        match goal {
            Purpose::Structure(id) => self.walk(gi, pos, Purpose::Structure(id), 0.0),
            Purpose::Exit => self.w.guests[gi].state = GuestState::Leaving,
            // At the bottom they choose afresh (0x488b80).
            Purpose::Lift(_) => self.plan_on_ground(gi),
            Purpose::Wander | Purpose::Park(_) | Purpose::Continue => self.plan_on_ground(gi),
        }
    }

    // ---------------------------------------------------------------- staff

    fn staff(&mut self, dt: f32) {
        // Wages are paid a little each tick, while there is the cash for
        // it (0x477fb0 and the like).
        let days = dt / self.defs.resort.seconds_per_day;
        let mut wages = 0.0;
        for s in &self.w.staff {
            let w = s.wage * days;
            if self.w.cash - wages >= w {
                wages += w;
            }
        }
        // Staff take on work only while the resort can pay a tick's wage;
        // otherwise they drop what they were doing (each class's vtable
        // +0x70: patrol 0x4807a0, sanitation 0x47c510, avalanche control
        // 0x476b00, tour driver 0x481850, security and wildlife control
        // 0x47de70, mechanics likewise). Instructors and the machines do
        // not check.
        let cash = self.w.cash;
        if wages > 0.0 {
            self.w.spend(Ledger::Wages, wages);
        }
        for si in 0..self.w.staff.len() {
            let s = &mut self.w.staff[si];
            s.anim += dt;
            s.moving = false;
            let strike = !matches!(s.kind, StaffKind::Instructor | StaffKind::Groomer | StaffKind::Snowmaker) && cash < s.wage * days;
            if strike {
                let patrol = matches!(s.kind, StaffKind::Security | StaffKind::WildlifeControl) && matches!(s.task, StaffTask::MoveTo(_));
                if patrol || matches!(s.task, StaffTask::Repair(_) | StaffTask::RepairLift(_) | StaffTask::Rescue(_) | StaffTask::Clean(_) | StaffTask::Mortar { .. }) {
                    s.task = StaffTask::Idle;
                }
                // A tour called off lets its riders go (0x481850).
                if matches!(s.task, StaffTask::Tour { .. }) {
                    s.task = StaffTask::Idle;
                    if let Some(home) = s.home {
                        let now = self.w.hour_abs();
                        for g in &mut self.w.guests {
                            if let GuestState::InStructure { structure, until, .. } = &mut g.state {
                                if *structure == home {
                                    *until = now;
                                }
                            }
                        }
                    }
                }
                let s = &self.w.staff[si];
                if s.task == StaffTask::Idle {
                    continue;
                }
            }
            let task = self.w.staff[si].task.clone();
            match task {
                StaffTask::Idle => self.assign(si),
                StaffTask::Held => {}
                StaffTask::Tour { to, back } => self.tour(si, to, back, dt),
                StaffTask::Mortar { cell, stage } => self.mortar(si, cell, stage, dt),
                StaffTask::MoveTo(p) => {
                    // A patroller heading back answers a fresh call.
                    let called = self.w.staff[si].kind == StaffKind::Patrol
                        && self.w.guests.iter().any(|g| {
                            g.state == GuestState::Injured
                                && !self.w.staff.iter().any(|s| s.task == StaffTask::Rescue(g.id) || matches!(s.task, StaffTask::Aid { guest, .. } if guest == g.id))
                        });
                    if called || self.move_staff(si, p, dt) {
                        self.w.staff[si].task = StaffTask::Idle;
                    }
                }
                StaffTask::Repair(id) => {
                    let Some(s) = self.w.structure(id).cloned() else {
                        self.w.staff[si].task = StaffTask::Idle;
                        continue;
                    };
                    let door = s.door(self.defs, self.w.level().spacing);
                    if self.move_staff(si, door, dt) {
                        let per = self.defs.structures[s.def].maintenance_per_wear;
                        let amount = repair_step(dt).min(s.wear);
                        let done = {
                            let s = self.w.structure_mut(id).unwrap();
                            s.wear -= amount;
                            if s.wear <= 0.0 {
                                s.wear = 0.0;
                                s.broken = false;
                            }
                            s.wear <= 0.0
                        };
                        self.w.spend(Ledger::Maintenance, amount * per * 0.5);
                        if done {
                            self.w.staff[si].task = StaffTask::Idle;
                        }
                    }
                }
                StaffTask::RepairLift(id) => {
                    let Some(l) = self.w.lift(id) else {
                        self.w.staff[si].task = StaffTask::Idle;
                        continue;
                    };
                    let at = l.base_queue_point();
                    let per = self.defs.lifts[l.def].maintenance_per_wear;
                    if self.move_staff(si, at, dt) {
                        let l = self.w.lift_mut(id).unwrap();
                        let amount = repair_step(dt).min(l.wear);
                        l.wear -= amount;
                        if l.wear <= 0.0 {
                            l.wear = 0.0;
                            l.broken = false;
                        }
                        let done = l.wear <= 0.0;
                        self.w.spend(Ledger::Maintenance, amount * per * 0.5);
                        if done {
                            self.w.staff[si].task = StaffTask::Idle;
                        }
                    }
                }
                StaffTask::Rescue(gid) => {
                    let Some(g) = self.w.guest(gid) else {
                        self.w.staff[si].task = StaffTask::Idle;
                        continue;
                    };
                    let p = g.pos.truncate();
                    if self.move_staff(si, p, dt) {
                        self.w.staff[si].task = StaffTask::Aid { guest: gid, left: 33.0 * TICK };
                    }
                }
                StaffTask::Aid { guest, left } => {
                    if self.w.guest(guest).is_none_or(|g| g.state != GuestState::Injured) {
                        self.w.staff[si].task = StaffTask::Idle;
                        continue;
                    }
                    let left = left - dt;
                    self.w.staff[si].task = if left > 0.0 {
                        StaffTask::Aid { guest, left }
                    } else {
                        if let Some(g) = self.w.guests.iter_mut().find(|g| g.id == guest) {
                            g.state = GuestState::Rescued;
                            g.thought = None;
                        }
                        StaffTask::Carry(guest)
                    };
                }
                StaffTask::Carry(gid) => {
                    // To the patroller's own base (its +0x148 +0x34), or the
                    // nearest patrol building if that is gone.
                    let to = self.rescue_base(si);
                    let Some(to) = to else {
                        self.release(gid);
                        self.w.staff[si].task = StaffTask::Idle;
                        continue;
                    };
                    let there = self.move_staff(si, to, dt);
                    let snowmobile = self.base_is(si, super::defs::StructureKind::SnowmobilePatrol);
                    let s = &self.w.staff[si];
                    // The sled trails the patroller, the guest lying on it.
                    let behind = s.pos - Vec3::new(s.heading.cos(), s.heading.sin(), 0.0) * 3.5;
                    let behind = self.w.ground(behind.truncate());
                    let Some(g) = self.w.guests.iter_mut().find(|g| g.id == gid) else {
                        self.w.staff[si].task = StaffTask::Idle;
                        continue;
                    };
                    g.pos = behind;
                    g.heading = self.w.staff[si].heading;
                    // The snowmobile treats on the way (0x47ff7c: 0.0002 a
                    // millisecond).
                    if snowmobile {
                        g.needs.health = (g.needs.health + 0.0066 * ticks(dt)).min(1.0);
                    }
                    if there {
                        self.w.staff[si].task = StaffTask::Heal(gid);
                    }
                }
                StaffTask::Heal(gid) => {
                    // 0.001 a tick until the injury is gone (state 7), then
                    // the guest goes on from the door (state 8).
                    let Some(g) = self.w.guests.iter_mut().find(|g| g.id == gid) else {
                        self.w.staff[si].task = StaffTask::Idle;
                        continue;
                    };
                    g.needs.health = (g.needs.health + 0.001 * ticks(dt)).min(1.0);
                    if g.needs.health >= 1.0 {
                        self.release(gid);
                        self.w.staff[si].task = StaffTask::Rest(100.0 * TICK);
                    }
                }
                StaffTask::Rest(left) => {
                    let left = left - dt;
                    self.w.staff[si].task = if left > 0.0 { StaffTask::Rest(left) } else { StaffTask::Idle };
                }
                StaffTask::Clean(p) => {
                    if self.move_staff(si, p, dt) {
                        let at = self.w.staff[si].pos;
                        self.w.sounds.push((crate::audio::Sound::Sweeping, Some(at)));
                        self.w.litter.retain(|l| l.pos.truncate().distance(p) > 6.0);
                        self.w.staff[si].task = StaffTask::Idle;
                    }
                }
                StaffTask::Groom(p) => {
                    let (speed, _) = self.machine_rate();
                    if self.move_at(si, p, speed, dt) {
                        // The moguls on the cell are smoothed away
                        // (0x409ea0); the snow's wear stays.
                        let s = self.w.level().spacing;
                        let n = self.w.level().cells_per_side();
                        let (x, y) = ((p.x / s) as usize, (p.y / s) as usize);
                        if x < n && y < n {
                            let c = self.w.level_mut().cell_mut(x, y);
                            c.flags &= !crate::formats::level::cell_flags::MOGULS;
                            self.w.scenery_dirty = true;
                        }
                        self.w.staff[si].task = StaffTask::Idle;
                    }
                }
                StaffTask::MakeSnow(p) => {
                    let (speed, snow) = self.machine_rate();
                    if self.move_at(si, p, speed, dt) {
                        // Wear comes off a tick at a time until none is
                        // left (0x40d0c0 from 0x43c380).
                        let s = self.w.level().spacing;
                        let n = self.w.level().cells_per_side();
                        let (x, y) = ((p.x / s) as usize, (p.y / s) as usize);
                        let mended = x >= n || y >= n || {
                            let c = self.w.level_mut().cell_mut(x, y);
                            let had = c.wear > 0.0;
                            c.wear = (c.wear - snow * ticks(dt)).max(0.0);
                            !had
                        };
                        if mended {
                            self.w.staff[si].task = StaffTask::Idle;
                        }
                    }
                }
            }
        }
    }

    /// A tour under way: the guests in the tour building stay aboard
    /// until the driver is back at its door (0x480e60).
    fn tour(&mut self, si: usize, to: Vec2, back: bool, dt: f32) {
        let Some(home) = self.w.staff[si].home.and_then(|h| self.w.structure(h)).map(|h| (h.id, h.door(self.defs, self.w.level().spacing))) else {
            self.w.staff[si].task = StaffTask::Idle;
            return;
        };
        let (id, door) = home;
        let now = self.w.hour_abs();
        // 0.02 a millisecond, 0.66 a tick (0x4811e0).
        const BUS: f32 = 0.66 / TICK;
        let done = back && self.move_at(si, door, BUS, dt);
        for g in &mut self.w.guests {
            if let GuestState::InStructure { structure, until, .. } = &mut g.state {
                if *structure == id {
                    *until = if done { now } else { f32::MAX };
                }
            }
        }
        if done {
            self.w.staff[si].task = StaffTask::Idle;
        } else if !back && self.move_at(si, to, BUS, dt) {
            self.w.staff[si].task = StaffTask::Tour { to, back: true };
        }
    }

    /// How fast the grooming and snowmaking machines go, units a second,
    /// and how much wear the snowmaker takes off a cell each tick: 0.2 a
    /// tick and 1, or 0.4 and 2 with a Maintenance Garage (0x43c13c; the
    /// Service Station it also looks for is no building in the data).
    fn machine_rate(&self) -> (f32, f32) {
        let garage = self.w.structures.iter().any(|s| self.defs.structures[s.def].unique_name.starts_with("Maintenance Garage"));
        if garage { (0.4 / TICK, 2.0) } else { (0.2 / TICK, 1.0) }
    }

    pub(crate) fn move_staff(&mut self, si: usize, to: Vec2, dt: f32) -> bool {
        if self.flies(si) {
            return self.fly(si, to, dt);
        }
        // The snowmobile goes 0.01 a millisecond, 0.33 a 33 ms tick
        // (0x47fb60, called with 33 by 0x482190).
        let speed = if self.base_is(si, super::defs::StructureKind::SnowmobilePatrol) {
            0.33 / TICK
        } else if self.base_is(si, super::defs::StructureKind::SkiPatrol) {
            return self.ski_staff(si, to, dt);
        } else {
            STAFF_SPEED
        };
        self.move_at(si, to, speed, dt)
    }

    /// A ski patroller gets about as a skier does (0x4933a0 moves them
    /// with 0x48ee70): by the slope ahead, poling on the flat and
    /// sidestepping uphill. They ski at the top level's pace.
    fn ski_staff(&mut self, si: usize, to: Vec2, dt: f32) -> bool {
        let pos = self.w.staff[si].pos.truncate();
        let d = to - pos;
        let dist = d.length();
        let dir = if dist > 0.0 { d / dist } else { Vec2::X };
        let drop = self.w.ground(pos).z - self.w.ground(pos + dir).z;
        let s = &mut self.w.staff[si];
        for _ in 0..ticks(dt).round().max(1.0) as usize {
            s.speed = ski_tick(s.speed * TICK, drop, 0.0, &mut s.ski_mode, SKILL_SPEED[3] * TICK) / TICK;
        }
        let speed = s.speed;
        let arrived = self.move_at(si, to, speed, dt);
        if arrived {
            let s = &mut self.w.staff[si];
            s.speed = 0.0;
            s.ski_mode = SkiMode::Pole;
        }
        arrived
    }

    /// Whether a patroller is housed at a base of this kind.
    fn base_is(&self, si: usize, kind: super::defs::StructureKind) -> bool {
        let s = &self.w.staff[si];
        s.kind == StaffKind::Patrol && s.home.and_then(|h| self.w.structure(h)).is_some_and(|b| self.defs.structures[b.def].kind == kind)
    }

    /// Whether this is a chopper patrol, which goes by air.
    fn flies(&self, si: usize) -> bool {
        self.base_is(si, super::defs::StructureKind::ChopperPatrol)
    }

    /// The chopper's flight (0x477350): up 0.222 a tick to 24 above the
    /// ground, across at 1.333 a tick, 0.059 once within 15 of the point
    /// 24 above where it is going, there within 6.5; then down again.
    fn fly(&mut self, si: usize, to: Vec2, dt: f32) -> bool {
        const CRUISE: f32 = 24.0;
        let climb = 0.222_222_22 / TICK * dt;
        let pos = self.w.staff[si].pos;
        let below = self.w.ground(pos.truncate()).z;
        let over = self.w.ground(to).z + CRUISE;
        let flat = to - pos.truncate();
        let above = Vec3::new(to.x, to.y, over) - pos;
        let slow = 0.059_259_26 / TICK * dt;
        // Within 6.5 of the point overhead (across, once at cruise height,
        // so that coming down does not undo it).
        if flat.length() < 6.5 {
            // Landing, closing the last of the way slowly.
            let step = slow.min(flat.length());
            let p = pos.truncate() + flat.normalize_or_zero() * step;
            let ground = self.w.ground(p);
            let landed = self.w.ground(to);
            let z = pos.z - climb;
            let s = &mut self.w.staff[si];
            s.moving = true;
            if z <= ground.z && flat.length() <= step + 0.5 {
                s.pos = landed;
                return true;
            }
            s.pos = Vec3::new(p.x, p.y, z.max(ground.z));
            return false;
        }
        let s = &mut self.w.staff[si];
        s.moving = true;
        if pos.z < below + CRUISE - 0.01 {
            s.pos.z = (pos.z + climb).min(below + CRUISE);
            return false;
        }
        let step = if above.length() < 15.0 { slow } else { 1.333_333_4 / TICK * dt }.min(flat.length());
        let n = flat.normalize_or_zero();
        let p = pos.truncate() + n * step;
        let z = self.w.ground(p).z + CRUISE;
        let s = &mut self.w.staff[si];
        s.pos = Vec3::new(p.x, p.y, z);
        if n != Vec2::ZERO {
            s.heading = n.y.atan2(n.x);
        }
        false
    }

    fn move_at(&mut self, si: usize, to: Vec2, speed: f32, dt: f32) -> bool {
        let pos = self.w.staff[si].pos.truncate();
        let d = to - pos;
        let dist = d.length();
        let step = speed * dt * if self.w.options.fast_maintenance { 2.0 } else { 1.0 };
        if dist <= step.max(0.5) {
            let p = self.w.ground(to);
            self.w.staff[si].pos = p;
            return true;
        }
        let n = d / dist;
        let p = self.w.ground(pos + n * step);
        let s = &mut self.w.staff[si];
        s.pos = p;
        s.heading = n.y.atan2(n.x);
        s.moving = true;
        false
    }

    fn assign(&mut self, si: usize) {
        let kind = self.w.staff[si].kind;
        let pos = self.w.staff[si].pos.truncate();
        let taken = |w: &World, t: &StaffTask| w.staff.iter().any(|s| &s.task == t);
        let task = match kind {
            StaffKind::Maintenance => {
                // The original's choice (0x4788f9, 0x47efe9): anything
                // below 92% condition, the lowest score winning: what is
                // left of its condition (or 100 above 85%), the distance,
                // less 1000 when broken and plus 1000 when another
                // mechanic has it.
                let at = self.w.staff[si].pos;
                let score = |wear: f32, broken: bool, claimed: bool, p: Vec3| {
                    let cond = 1.0 - wear / 100.0;
                    let mut d = [(p.x - at.x).abs(), (p.y - at.y).abs(), (p.z - at.z).abs()];
                    d.sort_by(|a, b| b.total_cmp(a));
                    (if cond < 0.85 { cond } else { 100.0 })
                        + d[0]
                        + 0.25 * (d[1] + d[2])
                        + if broken { -1000.0 } else { 0.0 }
                        + if claimed { 1000.0 } else { 0.0 }
                };
                let s = self.w.structures.iter().filter(|s| s.wear > 8.0).map(|s| {
                    let t = StaffTask::Repair(s.id);
                    (score(s.wear, s.broken, taken(self.w, &t), s.pos), t)
                });
                let l = self.w.lifts.iter().filter(|l| l.wear > 8.0).map(|l| {
                    let t = StaffTask::RepairLift(l.id);
                    (score(l.wear, l.broken, taken(self.w, &t), l.start), t)
                });
                s.chain(l).min_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1)
            }
            StaffKind::Patrol => self
                .w
                .guests
                .iter()
                .filter(|g| g.state == GuestState::Injured && !taken(self.w, &StaffTask::Rescue(g.id)) && !self.w.staff.iter().any(|s| matches!(s.task, StaffTask::Aid { guest, .. } if guest == g.id)))
                .min_by(|a, b| a.pos.truncate().distance(pos).total_cmp(&b.pos.truncate().distance(pos)))
                .map(|g| StaffTask::Rescue(g.id)),
            // The nearest mess within 400 (0x47bed3: squared distance
            // under 160000).
            StaffKind::Sanitation => self
                .w
                .litter
                .iter()
                .map(|l| l.pos.truncate())
                .filter(|p| p.distance(pos) < 400.0 && !taken(self.w, &StaffTask::Clean(*p)))
                .min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos)))
                .map(StaffTask::Clean),
            // The machines work only the cells ordered groomed or snowed,
            // the nearest first in cells across plus cells down (0x43bef0).
            StaffKind::Groomer | StaffKind::Snowmaker => {
                let s = self.w.level().spacing;
                let here = ((pos.x / s) as i64, (pos.y / s) as i64);
                let jobs = if kind == StaffKind::Groomer { &mut self.w.groom_jobs } else { &mut self.w.snow_jobs };
                let near = jobs
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, c)| (c.0 as i64 - here.0).abs() + (c.1 as i64 - here.1).abs())
                    .map(|(i, _)| i);
                near.map(|i| {
                    let (x, y) = jobs.remove(i);
                    let at = Vec2::new((x as f32 + 0.5) * s, (y as f32 + 0.5) * s);
                    if kind == StaffKind::Groomer { StaffTask::Groom(at) } else { StaffTask::MakeSnow(at) }
                })
            }
            StaffKind::AvalancheControl => {
                // Waits 150 ticks between rounds (0x475536 state 0).
                self.w.staff[si].work_timer += TICK;
                if self.w.staff[si].work_timer < 150.0 * TICK {
                    return;
                }
                self.w.staff[si].work_timer = 0.0;
                self.mortar_target().map(|cell| StaffTask::Mortar { cell, stage: MortarStage::Walk })
            }
            // The tour driver goes round and round, with whoever is in
            // the tour building, out to a spot clear of trees and back;
            // not while the building is broken or closed (0x480e60 states
            // 0, 5, 3, 6, 4, 1).
            StaffKind::SnowTour => {
                let home = self.w.staff[si].home.and_then(|h| self.w.structure(h));
                match home {
                    Some(h) if h.open && !h.broken => {
                        let n = self.w.level().cells_per_side();
                        let s = self.w.level().spacing;
                        let mut to = None;
                        for _ in 0..20 {
                            let i = (self.w.rand() * (n * n) as f32) as usize % (n * n);
                            let c = self.w.level().cell(i % n, i / n);
                            if c.foliage == 0 && c.flags & cell_flags::FOREST == 0 && self.w.occupied[i].is_none() {
                                to = Some(Vec2::new(((i % n) as f32 + 0.5) * s, ((i / n) as f32 + 0.5) * s));
                                break;
                            }
                        }
                        to.map(|to| StaffTask::Tour { to, back: false })
                    }
                    Some(h) => {
                        let door = h.door(self.defs, self.w.level().spacing);
                        (door.distance(pos) > 5.0).then_some(StaffTask::MoveTo(door))
                    }
                    None => None,
                }
            }
            // Security and wildlife control patrol: off to any building
            // within 500 units but the one they are at and the entrance
            // (0x47d8a0, 0x484f40), on arrival the next.
            StaffKind::Security | StaffKind::WildlifeControl => {
                let spacing = self.w.level().spacing;
                let near: Vec<Vec2> = self
                    .w
                    .structures
                    .iter()
                    .filter(|s| self.defs.structures[s.def].category != Category::Entrance)
                    .map(|s| s.door(self.defs, spacing))
                    .filter(|d| d.distance(pos) < 500.0 && d.distance(pos) > 25.0)
                    .collect();
                if near.is_empty() {
                    None
                } else {
                    let k = (self.w.rand() * near.len() as f32) as usize;
                    Some(StaffTask::MoveTo(near[k.min(near.len() - 1)]))
                }
            }
            _ => None,
        };
        if let Some(t) = task {
            self.w.staff[si].task = t;
        } else if kind == StaffKind::Maintenance && self.w.rand() < 0.01 {
            // Mechanics with nothing to fix wander near the base area
            // (STAFF_MAINT_WANDERING); the others wait where they are
            // (the patrol is "Watching for a skier in need.").
            let around = self
                .entrance()
                .map(|e| e.pos.truncate())
                .unwrap_or(pos);
            let jitter = Vec2::new(self.w.rand_range(-60.0, 60.0), self.w.rand_range(-60.0, 60.0));
            // On the map: the entrance can sit right at its edge.
            let lv = self.w.level();
            let edge = lv.cells_per_side() as f32 * lv.spacing - 1.0;
            let to = (around + jitter).clamp(Vec2::splat(1.0), Vec2::splat(edge));
            self.w.staff[si].task = StaffTask::MoveTo(to);
        }
    }

    // ----------------------------------------------------------------- wear

    /// The power grid (0x42dc90), worked out again whenever the buildings
    /// or a plant's state change. Each working plant floods out from its
    /// own cells, nearest first: through open ground for up to four
    /// cells, and on through any building it reaches, which takes it as
    /// far again. Every building cell it reaches uses one of the plant's
    /// `utility_coverage`; when that is spent the flood stops. Buildings
    /// with a powered cell have power.
    fn update_power(&mut self) {
        let defs = self.defs;
        let mut key = self.w.structures.len() as u64 * 65_537 + self.w.occupied.iter().filter(|o| o.is_some()).count() as u64;
        for s in &self.w.structures {
            if defs.structures[s.def].kind == StructureKind::PowerPlant {
                key = key.wrapping_mul(31).wrapping_add(s.id as u64 * 4 + s.open as u64 * 2 + s.broken as u64);
            }
        }
        if self.w.power_key == Some(key) {
            return;
        }
        self.w.power_key = Some(key);
        let n = self.w.level().cells_per_side() as i32;
        let mut powered = vec![false; (n * n) as usize];
        let mut seen = vec![false; (n * n) as usize];
        let mut lit: std::collections::HashSet<Id> = std::collections::HashSet::new();
        let plants: Vec<(Id, i32)> = self
            .w
            .structures
            .iter()
            .filter(|s| defs.structures[s.def].kind == StructureKind::PowerPlant && s.open && !s.broken)
            .map(|s| (s.id, defs.structures[s.def].utility_coverage))
            .collect();
        for (plant, capacity) in plants {
            seen.iter_mut().for_each(|v| *v = false);
            let mut queue: std::collections::VecDeque<(i32, i32, u32)> = std::collections::VecDeque::new();
            for (i, o) in self.w.occupied.iter().enumerate() {
                if *o == Some(plant) {
                    let (x, y) = (i as i32 % n, i as i32 / n);
                    seen[i] = true;
                    queue.push_back((x, y, 0));
                }
            }
            let mut used = 0;
            while used < capacity {
                let Some((x, y, depth)) = queue.pop_front() else { break };
                let i = (y * n + x) as usize;
                if !powered[i] {
                    powered[i] = true;
                    if let Some(id) = self.w.occupied[i] {
                        used += 1;
                        lit.insert(id);
                    }
                }
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= n || ny >= n {
                        continue;
                    }
                    let j = (ny * n + nx) as usize;
                    if seen[j] {
                        continue;
                    }
                    if self.w.occupied[j].is_some() {
                        seen[j] = true;
                        queue.push_back((nx, ny, 0));
                    } else if depth < 4 {
                        seen[j] = true;
                        queue.push_back((nx, ny, depth + 1));
                    }
                }
            }
        }
        self.w.powered_cells = powered;
        for s in &mut self.w.structures {
            s.powered = !defs.structures[s.def].uses_electricity || lit.contains(&s.id);
        }
    }

    /// Wear, worth and running costs, every tick (0x4b55e5 for buildings,
    /// 0x49e7de for lifts, 0x4b0b80 for runs, 0x4a65d3 for parks). While open and working,
    /// condition falls by `wear_and_tear` a day and half the
    /// `operations_cost` is paid a day, a little each tick; below 30%
    /// condition it gives out at random, one chance in a thousand a tick,
    /// and always at none. Lodges lose worth more slowly with realtors
    /// about.
    fn wear(&mut self, dt: f32) {
        let days = dt / self.defs.resort.seconds_per_day;
        let ticks = dt / TICK;
        let chance = 1.0 - 0.999f32.powf(ticks);
        self.update_power();
        let realtors = self
            .w
            .structures
            .iter()
            .filter(|s| self.defs.structures[s.def].kind == StructureKind::Realtor)
            .count() as f32;
        let mut ops = 0.0;
        let mut broke = Vec::new();
        let rolls: Vec<f32> = (0..self.w.structures.len()).map(|_| self.w.rand()).collect();
        for (s, roll) in self.w.structures.iter_mut().zip(rolls) {
            let d = &self.defs.structures[s.def];
            let rate = if d.kind == StructureKind::Lodging { 0.5 + 0.5 / (realtors + 1.0) } else { 1.0 };
            s.worth.tick(ticks * rate);
            if s.open && !s.broken {
                ops += d.operations_per_day;
                s.wear = (s.wear + d.wear_per_day * days * 100.0).min(100.0);
                if s.wear >= 100.0 || (s.wear > 70.0 && roll < chance) {
                    s.broken = true;
                    broke.push((Text::ReBuildingClosed, s.pos));
                }
            }
        }
        let rolls: Vec<f32> = (0..self.w.lifts.len()).map(|_| self.w.rand()).collect();
        for (l, roll) in self.w.lifts.iter_mut().zip(rolls) {
            let d = &self.defs.lifts[l.def];
            l.worth.tick(ticks);
            if l.open && !l.broken {
                ops += d.operations_per_day;
                l.wear = (l.wear + d.wear_per_day * days * 100.0).min(100.0);
                if l.wear >= 100.0 || (l.wear > 70.0 && roll < chance) {
                    l.broken = true;
                    broke.push((Text::ReLiftBroken, l.start));
                }
            }
        }
        for r in &mut self.w.runs {
            r.worth.tick(ticks);
        }
        for p in &mut self.w.parks {
            p.worth.tick(ticks);
            if p.open {
                ops += self.defs.parks.get(p.def).map_or(0.0, |d| d.operations_per_day);
            }
        }
        if ops > 0.0 {
            self.w.spend(Ledger::Operations, ops * 0.5 * days);
        }
        for (m, p) in broke {
            let text = self.defs.text(m).to_string();
            self.w.message(text, Some(p));
        }
    }

    // --------------------------------------------------------------- rating

    /// The resort score out of 1000 (0x42b090): clean snow (cap less the
    /// litter count), population, happiness, and points from lifts, runs
    /// and some kinds of building, each category limited to its cap from
    /// `resort.ini`. Stars are the score / 200. Terrain parks ("ski toys")
    /// and snow condition are read but never counted by the original.
    pub fn update_rating(&mut self) {
        let defs = self.defs;
        let caps = &defs.resort.caps;
        let w = &self.w;
        // points × cap / 100, at most the cap.
        let part = |points: f32, cap: f32| (points * cap * 0.01).min(cap);
        let clean = (caps.clean_snow - w.litter.len() as f32).max(0.0);
        let population = (w.active_guests() as f32 * 0.0025).min(1.0) * caps.population;
        // The original averages up to three earlier days with today and
        // keeps the larger of that and the cap, so this is never below it.
        let recent: Vec<f32> = w.history.iter().rev().take(3).map(|d| d.happiness).collect();
        let mood = (recent.iter().sum::<f32>() + self.average_happiness()) / (recent.len() + 1) as f32;
        let happiness = (mood * caps.happiness * 1.3).max(caps.happiness);
        let mut kind_points = [0.0f32; 7];
        let mut kind_count = [0usize; 7];
        for st in &w.structures {
            let d = &defs.structures[st.def];
            // Type ids from 0x4b42bf, grouped as in 0x42b090.
            use super::defs::StructureKind as K;
            let k = match d.kind {
                K::Lodging => 0,
                K::Restaurant => 1,
                K::Rental => 2,
                K::SkiSchool => 3,
                K::SkiPatrol | K::ChopperPatrol | K::AvalancheControl | K::SnowmobilePatrol => 4,
                K::Retail | K::SnowmobileTour | K::HeliTour | K::Bank | K::Daycare => 5,
                K::Entertainment | K::RadioStation => 6,
                _ => continue,
            };
            kind_points[k] += d.ratings_points;
            kind_count[k] += 1;
        }
        let kind = |k: usize, cap: f32| if kind_count[k] > 0 { part(kind_points[k], cap) } else { 0.0 };
        let lift_points: f32 = w.lifts.iter().map(|l| defs.lifts[l.def].ratings_points).sum();
        let runs = |d: i32| {
            let pts: Vec<f32> = w
                .runs
                .iter()
                .filter_map(|r| defs.runs.get(r.def))
                .filter(|x| x.difficulty == d)
                .map(|x| x.ratings_points)
                .collect();
            pts.iter().sum::<f32>()
        };
        let parts = vec![
            (RatingPart::CleanSnow, clean, caps.clean_snow),
            (RatingPart::Guests, population, caps.population),
            (RatingPart::Happiness, happiness, caps.happiness),
            (RatingPart::Lifts, part(lift_points, caps.lifts), caps.lifts),
            (RatingPart::BeginnerRuns, part(runs(0), caps.beginner), caps.beginner),
            (RatingPart::IntermediateRuns, part(runs(1), caps.intermediate), caps.intermediate),
            (RatingPart::AdvancedRuns, part(runs(2), caps.advanced), caps.advanced),
            (RatingPart::ExpertRuns, part(runs(3), caps.expert), caps.expert),
            (RatingPart::Lodging, kind(0, caps.lodging), caps.lodging),
            (RatingPart::Food, kind(1, caps.food), caps.food),
            (RatingPart::Rentals, kind(2, caps.rental), caps.rental),
            (RatingPart::SkiSchool, kind(3, caps.school), caps.school),
            (RatingPart::Safety, kind(4, caps.safety), caps.safety),
            (RatingPart::Retail, kind(5, caps.retail), caps.retail),
            (RatingPart::Entertainment, kind(6, caps.entertainment), caps.entertainment),
        ];
        let total = parts.iter().map(|p| p.1).sum::<f32>().clamp(0.0, 1000.0);
        self.w.rating = Rating {
            parts,
            total,
            stars: total / 200.0,
        };
    }
}

impl World {
    /// Monotonic hour counter used for timers that may cross midnight.
    pub fn hour_abs(&self) -> f32 {
        self.day as f32 * 24.0 + self.hour
    }
}

fn guest_name(id: Id) -> String {
    const FIRST: &[&str] = &[
        "Alex", "Sam", "Jordan", "Casey", "Riley", "Morgan", "Taylor", "Jamie", "Avery", "Quinn",
        "Drew", "Robin", "Skyler", "Parker", "Reese", "Hayden", "Logan", "Emerson", "Rowan", "Kai",
    ];
    const LAST: &[&str] = &[
        "Frost", "Powell", "Alder", "Pine", "Summit", "Hill", "Brooks", "Snow", "Ridge", "Wells",
        "Stone", "Vale", "Marsh", "Crane", "Hart", "Lake", "North", "West", "Fields", "Gale",
    ];
    let a = (id as usize * 7919) % FIRST.len();
    let b = (id as usize * 104_729 / 7) % LAST.len();
    format!("{} {}", FIRST[a], LAST[b])
}
