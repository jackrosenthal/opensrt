//! Sasquatches and wild rams, as the original runs them.
//!
//! A new resort gets 7 rams and, if the "Evil Sasquatches" option is on,
//! 5 sasquatches (the original's check at 0x41c1f8 is the other way
//! round, giving them only with the box unticked; fixed here), each
//! lurking unseen in a random spot of dense forest: a forest cell with more
//! than two forest neighbors (0x47d400).
//!
//! An animal stays put until its cell stops being forest (trees chopped or
//! flattened by an avalanche). Then:
//! - A sasquatch is reported seen (RE_SASQUATCH_SEEN), walks to the
//!   nearest dense forest and hides again (0x47c6e0 states 2–5).
//! - A ram goes on a rampage (RE_RAM_RAMPAGE): it runs at the nearest
//!   skier and butts them over, again and again up to one victim per ten
//!   guests, then goes back to the trees (0x479de0). Wildlife Control
//!   staff within 80 units send it straight back (0x4852f5).
//!
//! Each tick, with a chance of 1 in 18000 (1 in 300 in the sasquatch
//! challenge), a lurking sasquatch snatches a skier who has come within two
//! cells of forest: it appears in the trees beside them, walks up and
//! carries them off (RE_SKIER_ABDUCTION), and is then seen (0x4230b0).
//!
//! A lurking sasquatch near the skier the player is steering shows itself:
//! it stands idle, and yells within 25 units.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::sim::{Sim, TICK};
use super::world::*;
use crate::formats::level::cell_flags;
use crate::text::Text;

/// How close counts as arrived (0x47ae50's 6.0).
const REACH: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimalKind {
    Sasquatch,
    Ram,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AnimalState {
    /// Out of sight in the trees.
    Lurking,
    /// Walking back to the trees.
    Hiding { to: Vec2 },
    /// Going for a skier (a sasquatch to snatch, a ram to butt).
    Hunting { guest: Id },
    /// A ram butting a skier over.
    Butting { guest: Id, t: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animal {
    pub kind: AnimalKind,
    pub pos: Vec3,
    pub heading: f32,
    pub state: AnimalState,
    /// Times seen; every fifth, it moves to a fresh hiding place.
    pub sightings: u32,
    /// Skiers butted this rampage.
    pub hits: u32,
    /// Seconds into the current animation.
    pub anim: f32,
    /// The animation playing, by clip name; `None` while unseen.
    #[serde(skip)]
    pub clip: Option<&'static str>,
}

impl Animal {
    pub fn mesh(&self) -> &'static str {
        match self.kind {
            AnimalKind::Sasquatch => "data/scenery/people/sasquatch/squatch.MESH",
            AnimalKind::Ram => "data/scenery/people/ram/ram.MESH",
        }
    }
}

fn forest(w: &World, cx: i32, cy: i32) -> bool {
    let lv = w.level();
    let n = lv.cells_per_side() as i32;
    cx >= 0 && cy >= 0 && cx < n && cy < n && lv.cell(cx as usize, cy as usize).flags & cell_flags::FOREST != 0
}

fn cell_of(w: &World, p: Vec2) -> (i32, i32) {
    let s = w.level().spacing;
    ((p.x / s).floor() as i32, (p.y / s).floor() as i32)
}

fn in_forest(w: &World, p: Vec2) -> bool {
    let (x, y) = cell_of(w, p);
    forest(w, x, y)
}

/// Centers of the cells deep enough in the forest to hide in.
fn hiding_places(w: &World) -> Vec<Vec2> {
    let lv = w.level();
    let (n, s) = (lv.cells_per_side() as i32, lv.spacing);
    let mut out = Vec::new();
    for cy in 0..n {
        for cx in 0..n {
            if !forest(w, cx, cy) {
                continue;
            }
            let around = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .filter(|&(dx, dy)| (dx, dy) != (0, 0) && forest(w, cx + dx, cy + dy))
                .count();
            if around > 2 {
                out.push(Vec2::new((cx as f32 + 0.5) * s, (cy as f32 + 0.5) * s));
            }
        }
    }
    out
}

impl Sim<'_> {
    /// Puts a new resort's animals in the woods.
    pub fn spawn_wildlife(&mut self, sasquatches: bool) {
        let places = hiding_places(self.w);
        if places.is_empty() {
            return;
        }
        let kinds = std::iter::repeat_n(AnimalKind::Sasquatch, if sasquatches { 5 } else { 0 }).chain(std::iter::repeat_n(AnimalKind::Ram, 7));
        for kind in kinds {
            let p = places[(self.w.rand() * places.len() as f32) as usize % places.len()];
            let heading = self.w.rand() * std::f32::consts::TAU;
            self.w.animals.push(Animal {
                kind,
                pos: self.w.ground(p),
                heading,
                state: AnimalState::Lurking,
                sightings: 0,
                hits: 0,
                anim: 0.0,
                clip: None,
            });
        }
    }

    pub(crate) fn wildlife(&mut self, dt: f32) {
        self.abduction_roll(dt);
        for i in 0..self.w.animals.len() {
            self.w.animals[i].anim += dt;
            self.animal(i, dt);
        }
    }

    fn set_clip(&mut self, i: usize, clip: Option<&'static str>) {
        let a = &mut self.w.animals[i];
        if a.clip != clip {
            a.clip = clip;
            a.anim = 0.0;
        }
    }

    /// Moves animal `i` toward `to`; true once there.
    fn animal_walk(&mut self, i: usize, to: Vec2, dt: f32) -> bool {
        let pos = self.w.animals[i].pos.truncate();
        let d = to - pos;
        if d.length() <= REACH {
            return true;
        }
        // At a walking guest's pace, quicker on paths (0x47ae50 state 3).
        let step = (self.walk_speed(pos) * dt).min(d.length());
        let p = self.w.ground(pos + d.normalize() * step);
        let a = &mut self.w.animals[i];
        a.pos = p;
        a.heading = d.y.atan2(d.x);
        false
    }

    fn guest_pos(&self, id: Id) -> Option<Vec3> {
        self.w.guests.iter().find(|g| g.id == id && g.is_active() && g.pos.z > -100.0).map(|g| g.pos)
    }

    /// Seen out of the trees: report it and head for cover.
    fn animal_seen(&mut self, i: usize) {
        let kind = self.w.animals[i].kind;
        let at = self.w.animals[i].pos;
        let key = match kind {
            AnimalKind::Sasquatch => Text::ReSasquatchSeen,
            AnimalKind::Ram => Text::ReRamRampage,
        };
        let text = self.defs.text(key).into_owned();
        self.w.message(text, Some(at));
        self.w.animals[i].sightings += 1;
        match kind {
            AnimalKind::Sasquatch => self.animal_hide(i),
            AnimalKind::Ram => self.ram_next(i),
        }
    }

    fn animal_hide(&mut self, i: usize) {
        let pos = self.w.animals[i].pos.truncate();
        let to = hiding_places(self.w).into_iter().min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos)));
        self.w.animals[i].state = match to {
            Some(to) => AnimalState::Hiding { to },
            None => AnimalState::Lurking,
        };
    }

    /// A raging ram picks its next victim, or goes home.
    fn ram_next(&mut self, i: usize) {
        let a = &self.w.animals[i];
        let pos = a.pos.truncate();
        let limit = self.w.active_guests() as u32 / 10;
        let target = (a.hits < limit.max(1))
            .then(|| {
                self.w
                    .guests
                    .iter()
                    .filter(|g| g.is_active() && g.pos.z > -100.0 && matches!(g.state, GuestState::Skiing { .. } | GuestState::Walking { .. }))
                    .min_by(|x, y| x.pos.truncate().distance(pos).total_cmp(&y.pos.truncate().distance(pos)))
                    .map(|g| g.id)
            })
            .flatten();
        match target {
            Some(guest) => {
                self.w.animals[i].state = AnimalState::Hunting { guest };
                let at = self.w.animals[i].pos;
                self.w.sounds.push((crate::audio::Sound::Ram, Some(at)));
            }
            None => self.animal_hide(i),
        }
    }

    fn animal(&mut self, i: usize, dt: f32) {
        let a = self.w.animals[i].clone();
        match a.state {
            AnimalState::Lurking => {
                if !in_forest(self.w, a.pos.truncate()) {
                    self.animal_seen(i);
                    return;
                }
                // Only shows itself to the skier the player steers.
                let near = self.w.controlled.and_then(|id| self.guest_pos(id)).map(|p| p.distance(a.pos));
                let clip = match (a.kind, near) {
                    (AnimalKind::Sasquatch, Some(d)) if d < 25.0 => {
                        if a.clip != Some("Yell") {
                            self.w.sounds.push((crate::audio::Sound::Sasquatch, None));
                        }
                        Some("Yell")
                    }
                    (_, Some(_)) => Some("Idle"),
                    _ => None,
                };
                self.set_clip(i, clip);
            }
            AnimalState::Hiding { to } => {
                self.set_clip(i, Some("Walk"));
                if self.animal_walk(i, to, dt) {
                    let at = self.w.ground(to);
                    let a = &mut self.w.animals[i];
                    a.pos = at;
                    a.state = AnimalState::Lurking;
                    a.hits = 0;
                    a.clip = None;
                    // Every fifth sighting it moves somewhere else entirely.
                    if a.sightings % 5 == 0 {
                        let places = hiding_places(self.w);
                        if !places.is_empty() {
                            let p = places[(self.w.rand() * places.len() as f32) as usize % places.len()];
                            let g = self.w.ground(p);
                            self.w.animals[i].pos = g;
                        }
                    }
                }
            }
            AnimalState::Hunting { guest } => {
                // Wildlife Control staff scare a ram off.
                if a.kind == AnimalKind::Ram
                    && self.w.staff.iter().any(|s| s.kind == StaffKind::WildlifeControl && s.pos.distance(a.pos) < 80.0)
                {
                    self.animal_hide(i);
                    return;
                }
                let Some(target) = self.guest_pos(guest) else {
                    match a.kind {
                        AnimalKind::Sasquatch => self.animal_hide(i),
                        AnimalKind::Ram => self.ram_next(i),
                    }
                    return;
                };
                self.set_clip(i, Some(if a.kind == AnimalKind::Ram { "Run" } else { "Walk" }));
                if self.animal_walk(i, target.truncate(), dt) {
                    match a.kind {
                        AnimalKind::Sasquatch => self.snatch(i, guest),
                        AnimalKind::Ram => self.w.animals[i].state = AnimalState::Butting { guest, t: 0.0 },
                    }
                }
            }
            AnimalState::Butting { guest, t } => {
                self.set_clip(i, Some("Ram"));
                let t = t + dt;
                // The blow lands halfway through the butt.
                if t >= 0.5 {
                    let until = self.w.hour_abs() + 0.5;
                    if let Some(g) = self.w.guests.iter_mut().find(|g| g.id == guest) {
                        g.speed = 0.0;
                        g.state = GuestState::Crashed { until };
                    }
                    let at = self.w.animals[i].pos;
                    self.w.sounds.push((crate::audio::Sound::HitByRam, Some(at)));
                    self.w.animals[i].hits += 1;
                    self.ram_next(i);
                } else {
                    self.w.animals[i].state = AnimalState::Butting { guest, t };
                }
            }
        }
    }

    /// A sasquatch carries a skier off.
    fn snatch(&mut self, i: usize, guest: Id) {
        if let Some(g) = self.w.guests.iter_mut().find(|g| g.id == guest) {
            g.state = GuestState::Gone;
        }
        let at = self.w.animals[i].pos;
        let text = self.defs.text(Text::ReSkierAbduction).to_string();
        self.w.message(text, Some(at));
        self.animal_seen(i);
    }

    /// The chance each tick that a lurking sasquatch snatches a skier near
    /// the trees.
    fn abduction_roll(&mut self, dt: f32) {
        let one_in = self.w.challenge_state.as_ref().and_then(|c| c.abduction_interval()).unwrap_or(18000.0);
        if self.w.rand() >= dt / TICK / one_in {
            return;
        }
        let Some(i) = self
            .w
            .animals
            .iter()
            .position(|a| a.kind == AnimalKind::Sasquatch && a.state == AnimalState::Lurking)
        else {
            return;
        };
        let s = self.w.level().spacing;
        for g in &self.w.guests {
            if !matches!(g.state, GuestState::Skiing { .. }) {
                continue;
            }
            let (cx, cy) = cell_of(self.w, g.pos.truncate());
            let near = (-2..=2)
                .flat_map(|dy| (-2..=2).map(move |dx| (cx + dx, cy + dy)))
                .find(|&(x, y)| (x, y) != (cx, cy) && forest(self.w, x, y));
            if let Some((x, y)) = near {
                let p = Vec2::new((x as f32 + 0.5) * s, (y as f32 + 0.5) * s);
                let id = g.id;
                let pos = self.w.ground(p);
                // The skier is held where they are until it arrives.
                if let Some(g) = self.w.guests.iter_mut().find(|g| g.id == id) {
                    g.speed = 0.0;
                    g.state = GuestState::Seized;
                }
                let a = &mut self.w.animals[i];
                a.pos = pos;
                a.state = AnimalState::Hunting { guest: id };
                return;
            }
        }
    }
}
