//! Lays out a small starter resort automatically. Used by the headless
//! `simulate` command and by the "quick start" option.

use glam::{Vec2, Vec3};

use super::defs::Category;
use super::world::Id;
use super::{Game, Tool};

impl Game {
    fn find_def(&self, pred: impl Fn(&super::defs::StructureDef) -> bool) -> Option<usize> {
        self.defs.structures.iter().position(pred)
    }

    /// Tries to place a structure near `around`, spiralling outwards.
    pub fn place_near(&mut self, def: usize, around: Vec2) -> Option<Id> {
        let s = self.world.level().spacing;
        let (w, h) = self.defs.structures[def].size;
        let c0 = ((around.x / s) as i32 - w / 2, (around.y / s) as i32 - h / 2);
        for r in 0..30i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let cell = (c0.0 + dx * 2, c0.1 + dy * 2);
                    let before = self.world.structures.len();
                    let saved = self.last_error.take();
                    if self.try_build_quiet(def, cell) {
                        return self.world.structures.get(before).map(|s| s.id);
                    }
                    self.last_error = saved;
                }
            }
        }
        None
    }

    fn try_build_quiet(&mut self, def: usize, cell: (i32, i32)) -> bool {
        let ok = self.build_structure(def, cell, 0, false);
        self.last_error = None;
        ok
    }

    /// Builds a lift from near the entrance up the fall line, a beginner and
    /// an intermediate run back down, and the basic services.
    pub fn autobuild_starter(&mut self) -> bool {
        let Some(ent) = self
            .world
            .structures
            .iter()
            .find(|s| self.defs.structures[s.def].category == Category::Entrance)
            .map(|s| s.pos)
        else {
            return false;
        };
        let level = self.world.level();
        let ext = level.extent();
        // Walk uphill from the entrance to find a lift top.
        // Start just outside the entrance footprint, on the uphill side.
        let n0 = level.normal_at(ent.x, ent.y);
        let up0 = -Vec2::new(n0.x, n0.y);
        let away = if up0.length() > 0.01 { up0.normalize() } else { Vec2::new(0.0, 1.0) };
        let mut base = ent.truncate() + away * 70.0;
        // On a wide flat valley floor, walk toward the summit until the
        // ground ahead rises.
        let n = level.cells_per_side();
        let summit = (0..n * n)
            .map(|i| (i % n, i / n))
            .max_by(|a, b| level.height(a.0, a.1).total_cmp(&level.height(b.0, b.1)))
            .map(|(x, y)| Vec2::new(x as f32, y as f32) * level.spacing)
            .unwrap_or(base);
        let to_summit = (summit - base).normalize_or_zero();
        while base.distance(summit) > 400.0 {
            let ahead = base + to_summit * 300.0;
            if level.height_at(ahead.x, ahead.y) - level.height_at(base.x, base.y) > 30.0 {
                break;
            }
            base += to_summit * 50.0;
        }
        // Lift top: the highest point 250-550 units away.
        let mut top = base;
        let mut best = f32::MIN;
        for i in 0..72 {
            let ang = i as f32 / 72.0 * std::f32::consts::TAU;
            for r in [250.0f32, 330.0, 420.0, 520.0] {
                let q = base + Vec2::new(ang.cos(), ang.sin()) * r;
                if q.x < 30.0 || q.y < 30.0 || q.x > ext - 30.0 || q.y > ext - 30.0 {
                    continue;
                }
                let z = level.height_at(q.x, q.y) - r * 0.02;
                if z > best {
                    best = z;
                    top = q;
                }
            }
        }
        let start = Vec3::new(base.x, base.y, level.height_at(base.x, base.y));
        let end = Vec3::new(top.x, top.y, level.height_at(top.x, top.y));
        // Use the best unlocked lift that can span the distance, shortening
        // the line if nothing reaches.
        let horiz = start.truncate().distance(end.truncate());
        let mut lift_def = 0;
        let mut end = end;
        for (i, d) in self.defs.lifts.iter().enumerate() {
            if self.unlocked(&d.predecessors) || i == 0 {
                lift_def = i;
            }
        }
        // Diagnostics can ask for a particular lift type.
        if let Some(i) = std::env::var("SRT_AUTOBUILD_LIFT").ok().and_then(|v| v.parse::<usize>().ok()) {
            lift_def = i.min(self.defs.lifts.len() - 1);
        }
        let d = &self.defs.lifts[lift_def];
        let reach = d.pole_spacing * (d.max_poles as f32 + 1.0) * 0.95;
        if horiz > reach {
            let t = reach / horiz;
            let q = start.truncate().lerp(end.truncate(), t);
            end = Vec3::new(q.x, q.y, self.world.level().height_at(q.x, q.y));
        }
        let top = end.truncate();
        let mut built_lift = self.build_lift(lift_def, start, end);
        // Bigger stations may not fit beside the entrance: move the base
        // up the line.
        let mut start = start;
        for _ in 0..4 {
            if built_lift || self.last_error.map(|e| e.0) != Some(super::Problem::Occupied) {
                break;
            }
            let q = start.truncate().lerp(end.truncate(), 0.15);
            start = Vec3::new(q.x, q.y, self.world.level().height_at(q.x, q.y));
            built_lift = self.build_lift(lift_def, start, end);
        }
        if !built_lift {
            log::warn!("autobuild lift failed: {:?} ({start} -> {end})", self.last_error);
            built_lift = self.build_lift(0, start, start.lerp(end, 0.6));
            if !built_lift {
                log::warn!("autobuild fallback lift failed: {:?}", self.last_error);
            }
        }
        self.last_error = None;
        // Runs: gates down either side of the lift line.
        let dir = (top - base).normalize_or_zero();
        let side = Vec2::new(-dir.y, dir.x);
        for (k, offset) in [(0usize, 70.0f32), (1, -70.0)] {
            if k >= self.defs.runs.len() {
                continue;
            }
            let gates: Vec<Vec3> = (0..6)
                .map(|i| {
                    let t = 1.0 - i as f32 / 5.0;
                    let q = base.lerp(top, t) + side * offset * (0.4 + 0.6 * (t * 3.0).sin().abs());
                    let q = q.clamp(Vec2::splat(20.0), Vec2::splat(ext - 20.0));
                    Vec3::new(q.x, q.y, self.world.level().height_at(q.x, q.y))
                })
                .collect();
            self.build_run(k, gates);
        }
        let near = ent.truncate();
        let picks: Vec<Option<usize>> = vec![
            self.find_def(|d| d.category == Category::PowerPlant),
            self.find_def(|d| d.category == Category::Food),
            self.find_def(|d| d.category == Category::Bathroom),
            self.find_def(|d| d.category == Category::Rental),
            self.find_def(|d| d.category == Category::WarmingHut),
            self.find_def(|d| d.category == Category::Lodging),
            self.find_def(|d| d.category == Category::SkiPatrol),
            self.find_def(|d| d.kind == super::defs::StructureKind::Maintenance),
            self.find_def(|d| d.category == Category::SkiSchool),
        ];
        for d in picks.into_iter().flatten() {
            self.place_near(d, near + Vec2::new(-40.0, -40.0));
        }
        self.tool = Tool::Inspect;
        built_lift
    }
}
