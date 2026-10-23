//! Avalanches, as the original runs them.
//!
//! Every cell gathers avalanche risk, 0.000055 a tick (0x401380). When a
//! spell of rain or snow ends (the precipitation effect's countdown,
//! 0x468f20), the resort rolls for an avalanche: 0.95 in the avalanche
//! challenge, 0.3 once an Avalanche Control Building stands, otherwise
//! never. On success the riskiest prone cell (0x4010d0: open, steeper than
//! about 41°, facing the low corner, with open steep ground below) starts
//! one with a chance equal to its risk (0x401440).
//!
//! An avalanche (0x401510, 0x402230) slides down the terrain, gaining the
//! surface normal × 0.035 a tick and keeping 98.5% of its speed, until it
//! has run 300 ticks, has slowed to a stop after 75, or leaves the map. It
//! sweeps three tracks 10 units apart (0x401ea0): forest there is flattened
//! and slows it, the risk of every cell it crosses resets, skiers there are
//! knocked down and buildings and lifts break (0x40d8d0).

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::sim::{Sim, TICK};
use super::world::*;
use crate::formats::level::cell_flags;
use crate::text::Text;

const RISK_PER_TICK: f32 = 0.000055;

/// A mortar shell's burst, throwing up snow for a second (0x475ea0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blast {
    pub pos: Vec3,
    pub t: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avalanche {
    pub pos: Vec3,
    pub vel: Vec3,
    pub ticks: u32,
    /// Seconds not yet run as whole ticks.
    pub acc: f32,
}

impl Avalanche {
    /// The two side tracks, 10 units either side across the direction of
    /// travel.
    pub fn sides(&self) -> [Vec2; 2] {
        let d = self.vel.truncate().normalize_or(Vec2::X);
        let across = Vec2::new(-d.y, d.x) * 10.0;
        [self.pos.truncate() + across, self.pos.truncate() - across]
    }
}

/// Unit surface normal of the terrain at a point.
fn normal(w: &World, p: Vec2) -> Vec3 {
    let lv = w.level();
    let s = lv.spacing;
    let dx = lv.height_at(p.x + s * 0.5, p.y) - lv.height_at(p.x - s * 0.5, p.y);
    let dy = lv.height_at(p.x, p.y + s * 0.5) - lv.height_at(p.x, p.y - s * 0.5);
    Vec3::new(-dx, -dy, s).normalize()
}

/// The share of prone cells whose risk has passed 1 (0x4013d0), which
/// the weather tab shows as low, medium or high danger.
pub fn avalanche_danger(w: &World) -> f32 {
    let prone = prone_cells(w);
    if prone.is_empty() {
        return 0.0;
    }
    prone.iter().filter(|c| c.1 > 1.0).count() as f32 / prone.len() as f32
}

/// Cells an avalanche can start in, with their risk.
fn prone_cells(w: &World) -> Vec<((usize, usize), f32)> {
    let lv = w.level();
    let (n, s) = (lv.cells_per_side(), lv.spacing);
    let mut out = Vec::new();
    for cy in 0..n {
        for cx in 0..n {
            let c = Vec2::new((cx as f32 + 0.5) * s, (cy as f32 + 0.5) * s);
            let nm = normal(w, c);
            if lv.cell(cx, cy).flags & cell_flags::FOREST != 0 || nm.z >= 0.75 || nm.x > 0.05 || nm.y > 0.05 {
                continue;
            }
            // The ground below must stay open and steep for 1 to 3
            // cells, more the gentler the start.
            let steps = if nm.z < 0.4875 {
                1
            } else if nm.z < 0.6375 {
                2
            } else {
                3
            };
            let mut p = c;
            let ok = (0..steps).all(|_| {
                p += nm.truncate() * 10.0;
                let (x, y) = ((p.x / s) as usize, (p.y / s) as usize);
                x < n && y < n && lv.cell(x, y).flags & cell_flags::FOREST == 0 && normal(w, p).z <= 0.75
            });
            if ok {
                out.push(((cx, cy), risk(w, cy * n + cx)));
            }
        }
    }
    out
}


fn risk(w: &World, i: usize) -> f32 {
    let since = w.avalanche_risk.get(i).copied().unwrap_or(w.avalanche_clock);
    ((w.avalanche_clock - since) as f32) * RISK_PER_TICK
}


/// Mortar timings: setting up and firing (an estimate of the original's
/// animations), the shell's flight (11 ticks) and the wait after (120
/// ticks, 0x475536).
const MORTAR_RANGE: f32 = 100.0;
const AIM_SECS: f32 = 1.5;
const SHELL_TICKS: f32 = 11.0;
const SETTLE_TICKS: f32 = 120.0;

impl Sim<'_> {
    /// Where avalanche control should shell next (0x476df0): the first
    /// prone cell past risk 1 that nobody is shelling, else the riskiest
    /// free one above 0.25.
    pub(crate) fn mortar_target(&self) -> Option<(usize, usize)> {
        let open = self.w.structures.iter().any(|s| {
            self.defs.structures[s.def].kind == super::defs::StructureKind::AvalancheControl && s.open && !s.broken
        });
        if !open {
            return None;
        }
        let taken: Vec<(usize, usize)> = self
            .w
            .staff
            .iter()
            .filter_map(|s| match s.task {
                StaffTask::Mortar { cell, .. } => Some(cell),
                _ => None,
            })
            .collect();
        let free: Vec<((usize, usize), f32)> = prone_cells(self.w).into_iter().filter(|c| !taken.contains(&c.0)).collect();
        if let Some(c) = free.iter().find(|c| c.1 > 1.0) {
            return Some(c.0);
        }
        free.into_iter().filter(|c| c.1 > 0.25).max_by(|a, b| a.1.total_cmp(&b.1)).map(|c| c.0)
    }

    /// One avalanche control worker's shelling run (0x475520).
    pub(crate) fn mortar(&mut self, si: usize, cell: (usize, usize), stage: MortarStage, dt: f32) {
        let s = self.w.level().spacing;
        let target = Vec2::new((cell.0 as f32 + 0.5) * s, (cell.1 as f32 + 0.5) * s);
        let next = match stage {
            MortarStage::Walk => {
                let pos = self.w.staff[si].pos.truncate();
                if pos.distance(target) <= MORTAR_RANGE || self.move_staff(si, target, dt) {
                    MortarStage::Aim(0.0)
                } else {
                    MortarStage::Walk
                }
            }
            MortarStage::Aim(t) if t + dt >= AIM_SECS => {
                let at = self.w.staff[si].pos;
                self.w.sounds.push((crate::audio::Sound::ShootMortar, Some(at)));
                MortarStage::Shell(0.0)
            }
            MortarStage::Aim(t) => MortarStage::Aim(t + dt),
            MortarStage::Shell(t) if t + dt >= SHELL_TICKS * TICK => {
                // The shell clears the slope around the target: risk then
                // grows from the centre outwards (0x477030).
                let n = self.w.level().cells_per_side() as i32;
                for dy in -3..=3i32 {
                    for dx in -3..=3i32 {
                        let (x, y) = (cell.0 as i32 + dx, cell.1 as i32 + dy);
                        if x >= 0 && y >= 0 && x < n && y < n {
                            self.set_risk((y * n + x) as usize, (dx * dx + dy * dy) as f32 * 0.01);
                        }
                    }
                }
                let at = self.w.ground(target);
                self.w.sounds.push((crate::audio::Sound::MortarExplosion, Some(at)));
                self.w.blasts.push(Blast { pos: at, t: 0.0 });
                MortarStage::Settle(0.0)
            }
            MortarStage::Shell(t) => MortarStage::Shell(t + dt),
            MortarStage::Settle(t) if t + dt >= SETTLE_TICKS * TICK + AIM_SECS => {
                // On to the next slope, or home.
                self.w.staff[si].task = match self.mortar_target_except(cell) {
                    Some(c) => StaffTask::Mortar { cell: c, stage: MortarStage::Walk },
                    None => StaffTask::Idle,
                };
                return;
            }
            MortarStage::Settle(t) => MortarStage::Settle(t + dt),
        };
        self.w.staff[si].task = StaffTask::Mortar { cell, stage: next };
    }

    fn mortar_target_except(&mut self, done: (usize, usize)) -> Option<(usize, usize)> {
        // This worker's own reservation still names `done`; free it first.
        self.mortar_target().filter(|c| *c != done)
    }

    fn set_risk(&mut self, i: usize, risk: f32) {
        let since = self.w.avalanche_clock - (risk / RISK_PER_TICK) as f64;
        if let Some(r) = self.w.avalanche_risk.get_mut(i) {
            *r = since;
        }
    }

    pub(crate) fn avalanches(&mut self, dt: f32) {
        let n = self.w.level().cells_per_side();
        if self.w.avalanche_risk.len() != n * n {
            self.w.avalanche_risk = vec![self.w.avalanche_clock; n * n];
        }
        self.w.avalanche_clock += (dt / TICK) as f64;
        let mut list = std::mem::take(&mut self.w.avalanches);
        for a in &mut list {
            a.acc += dt;
            while a.acc >= TICK && a.ticks != u32::MAX {
                a.acc -= TICK;
                self.avalanche_tick(a);
            }
        }
        list.retain(|a| a.ticks != u32::MAX);
        list.append(&mut self.w.avalanches);
        self.w.avalanches = list;
    }

    /// The roll when rain or snow stops.
    pub(crate) fn avalanche_roll(&mut self) {
        let control = self
            .w
            .structures
            .iter()
            .any(|s| self.defs.structures[s.def].kind == super::defs::StructureKind::AvalancheControl);
        let challenge = self.w.challenge_state.as_ref().is_some_and(|c| c.number == 5);
        let p = if challenge {
            0.95
        } else if control {
            0.3
        } else {
            0.0
        };
        if self.w.rand() >= p {
            return;
        }
        let prone = prone_cells(self.w);
        let Some(&(cell, risk)) = prone.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else { return };
        if self.w.rand() >= risk {
            return;
        }
        self.start_avalanche(cell);
    }

    /// The original's Ctrl+Shift+X: an avalanche at the riskiest prone
    /// cell, whatever its risk (0x4270ad).
    pub fn force_avalanche(&mut self) {
        let prone = prone_cells(self.w);
        if let Some(&(cell, _)) = prone.iter().max_by(|a, b| a.1.total_cmp(&b.1)) {
            self.start_avalanche(cell);
        }
    }

    pub(crate) fn start_avalanche(&mut self, (cx, cy): (usize, usize)) {
        let n = self.w.level().cells_per_side();
        let s = self.w.level().spacing;
        self.reset_risk(cy * n + cx);
        let p = Vec2::new((cx as f32 + 0.5) * s, (cy as f32 + 0.5) * s);
        self.w.avalanches.push(Avalanche {
            pos: self.w.ground(p),
            vel: Vec3::ZERO,
            ticks: 0,
            acc: 0.0,
        });
        self.w.avalanches_started += 1;
        let text = self.defs.text(Text::ReAvalanche).to_string();
        let at = self.w.ground(p);
        self.w.message(text, Some(at));
    }

    fn avalanche_tick(&mut self, a: &mut Avalanche) {
        let nm = normal(self.w, a.pos.truncate());
        a.vel = (a.vel + nm * 0.035) * 0.985;
        a.pos += Vec3::new(a.vel.x, a.vel.y, 0.0);
        a.pos = self.w.ground(a.pos.truncate());
        a.ticks += 1;
        let ext = self.w.level().extent();
        let slow = a.vel.truncate().length_squared() < 0.1 && a.ticks > 75;
        let off = a.pos.x < 0.0 || a.pos.y < 0.0 || a.pos.x > ext || a.pos.y > ext;
        let [l, r] = a.sides();
        // Forest on the tracks is flattened and slows the slide.
        for (p, keep) in [(a.pos.truncate(), 0.85), (l, 0.95), (r, 0.95)] {
            if self.flatten_forest(p) {
                a.vel *= keep;
            }
        }
        for p in [a.pos.truncate(), l, r] {
            for dy in [-10.0, 0.0, 10.0] {
                for dx in [-10.0, 0.0, 10.0] {
                    self.avalanche_hits(p + Vec2::new(dx, dy));
                }
            }
        }
        if a.ticks > 300 || slow || off {
            a.ticks = u32::MAX;
        }
    }

    fn reset_risk(&mut self, i: usize) {
        let now = self.w.avalanche_clock;
        if let Some(r) = self.w.avalanche_risk.get_mut(i) {
            *r = now;
        }
    }

    fn cell_at(&self, p: Vec2) -> Option<(usize, usize)> {
        let lv = self.w.level();
        let (n, s) = (lv.cells_per_side() as i32, lv.spacing);
        let (x, y) = ((p.x / s).floor() as i32, (p.y / s).floor() as i32);
        (x >= 0 && y >= 0 && x < n && y < n).then_some((x as usize, y as usize))
    }

    fn flatten_forest(&mut self, p: Vec2) -> bool {
        let Some((x, y)) = self.cell_at(p) else { return false };
        let c = self.w.level_mut().cell_mut(x, y);
        if c.flags & cell_flags::FOREST == 0 {
            return false;
        }
        c.set_foliage(0);
        self.w.scenery_dirty = true;
        true
    }

    /// What an avalanche does to a cell it crosses.
    fn avalanche_hits(&mut self, p: Vec2) {
        let Some((x, y)) = self.cell_at(p) else { return };
        let lv = self.w.level();
        let (n, s) = (lv.cells_per_side(), lv.spacing);
        self.reset_risk(y * n + x);
        let in_cell = |q: Vec3| (q.x / s).floor() as usize == x && (q.y / s).floor() as usize == y;
        // Skiers on the slope are knocked down; about one in five is hurt
        // (0x48ee70 state 3 with the hit flag).
        let hit: Vec<usize> = (0..self.w.guests.len())
            .filter(|&i| {
                let g = &self.w.guests[i];
                matches!(g.state, GuestState::Skiing { .. } | GuestState::Walking { .. }) && in_cell(g.pos)
            })
            .collect();
        for i in hit {
            let hurt = self.w.rand() > 0.8;
            let until = self.w.hour_abs() + 0.5;
            let g = &mut self.w.guests[i];
            g.speed = 0.0;
            g.state = if hurt { GuestState::Injured } else { GuestState::Crashed { until } };
        }
        if let Some(id) = self.w.occupied.get(y * n + x).copied().flatten() {
            if let Some(st) = self.w.structure_mut(id) {
                st.wear = 100.0;
                st.broken = true;
            }
        }
        for l in &mut self.w.lifts {
            let hit = in_cell(l.start) || in_cell(l.end) || l.poles.iter().any(|p| in_cell(*p));
            if hit {
                l.wear = 100.0;
                l.broken = true;
            }
        }
    }
}
