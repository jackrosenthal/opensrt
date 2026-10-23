//! What the pointer would build: footprint marks and the preview.

use super::*;

impl Game {
    /// Whether a cell can take a new building or lift station.
    pub(super) fn cell_mark(&self, x: i32, y: i32) -> CellMark {
        let n = self.world.level().cells_per_side() as i32;
        let blocked = x < 1
            || y < 1
            || x >= n - 1
            || y >= n - 1
            || self.world.occupied[(y * n + x) as usize].is_some()
            || self.world.lift_cells.get((y * n + x) as usize).is_some_and(|c| c.is_some());
        if blocked { CellMark::Blocked } else { CellMark::Ok }
    }

    /// The cells a lift from `st` to `p` covers while being placed.
    pub(super) fn lift_marks(&self, def: usize, st: Vec3, p: Vec3) -> Vec<((usize, usize), CellMark)> {
        let s = self.world.level().spacing;
        let n = self.world.level().cells_per_side() as i32;
        let cell = |v: Vec3| ((v.x / s).floor() as i32, (v.y / s).floor() as i32);
        let d = &self.defs.lifts[def];
        let mut out = Vec::new();
        self.mark_disk(cell(st), d.start_radius, &mut out);
        self.mark_disk(cell(p), d.end_radius, &mut out);
        // Cells whose center is within the path radius of the line.
        let (a, b) = (st.truncate(), p.truncate());
        let width = d.path_radius * s;
        let (lo, hi) = (a.min(b) - Vec2::splat(width), a.max(b) + Vec2::splat(width));
        for y in ((lo.y / s).floor() as i32).max(0)..=((hi.y / s).floor() as i32).min(n - 1) {
            for x in ((lo.x / s).floor() as i32).max(0)..=((hi.x / s).floor() as i32).min(n - 1) {
                let c = Vec2::new((x as f32 + 0.5) * s, (y as f32 + 0.5) * s);
                let ab = b - a;
                let t = if ab.length_squared() > 0.0 { ((c - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
                if c.distance(a + ab * t) < width {
                    out.push(((x as usize, y as usize), self.cell_mark(x, y)));
                }
            }
        }
        out
    }

    /// Cells within `r` cells of a center cell, as the original's lift
    /// placement visits them (0x4a0c30).
    pub(super) fn mark_disk(&self, c: (i32, i32), r: f32, out: &mut Vec<((usize, usize), CellMark)>) {
        let n = self.world.level().cells_per_side() as i32;
        let k = r.floor() as i32;
        for j in -k..=k {
            for i in -k..=k {
                let (x, y) = (c.0 + i, c.1 + j);
                if ((i * i + j * j) as f32) <= r * r && x >= 0 && y >= 0 && x < n && y < n {
                    out.push(((x as usize, y as usize), self.cell_mark(x, y)));
                }
            }
        }
    }

    /// The cells the original tints while placing (see `CellMark`).
    pub(super) fn placement_marks(&self, p: Vec3) -> Vec<((usize, usize), CellMark)> {
        let s = self.world.level().spacing;
        let n = self.world.level().cells_per_side() as i32;
        let cell = |v: Vec3| ((v.x / s).floor() as i32, (v.y / s).floor() as i32);
        let mut out = Vec::new();
        match &self.tool {
            Tool::Structure { def, rot } => {
                let d = &self.defs.structures[*def];
                let (w, h) = if rot % 2 == 0 { d.size } else { (d.size.1, d.size.0) };
                let c0 = (
                    ((p.x / s) - w as f32 * 0.5).round() as i32,
                    ((p.y / s) - h as f32 * 0.5).round() as i32,
                );
                for y in c0.1..c0.1 + h {
                    for x in c0.0..c0.0 + w {
                        if x >= 0 && y >= 0 && x < n && y < n {
                            out.push(((x as usize, y as usize), self.cell_mark(x, y)));
                        }
                    }
                }
            }
            Tool::Lift { def, start: None } => {
                self.mark_disk(cell(p), self.defs.lifts[*def].start_radius, &mut out);
            }
            Tool::Lift { def, start: Some(st) } => out = self.lift_marks(*def, *st, p),
            _ => {}
        }
        out
    }

    pub(super) fn update_placement(&mut self) {
        self.placement = None;
        self.cell_marks.clear();
        // A run being laid out shows its cells.
        if matches!(self.tool, Tool::RunPath { .. }) {
            let marks = self.run_path_marks();
            self.cell_marks.extend(marks.into_iter().map(|c| (c, CellMark::Ok)));
        }
        // The surface tools' rectangle shows as picked cells.
        if let Some(sel) = &self.terrain_sel {
            let ((x0, y0), (x1, y1)) = sel.rect();
            for y in y0.max(0)..=y1 {
                for x in x0.max(0)..=x1 {
                    self.cell_marks.push(((x as usize, y as usize), CellMark::Ok));
                }
            }
        }
        let Some(p) = self.hover else { return };
        let marks = self.placement_marks(p);
        self.cell_marks.extend(marks);
        let s = self.world.level().spacing;
        match self.tool.clone() {
            Tool::Structure { def, rot } => {
                let d = &self.defs.structures[def];
                let (w, h) = if rot % 2 == 0 { d.size } else { (d.size.1, d.size.0) };
                let cell = (
                    ((p.x / s) - w as f32 * 0.5).round() as i32,
                    ((p.y / s) - h as f32 * 0.5).round() as i32,
                );
                let (mut ok, mut reason, _) = self.footprint_ok(def, cell, rot);
                if ok && self.world.cash < d.cost {
                    ok = false;
                    reason = Some(Problem::TooExpensive);
                }
                let center = Vec3::new(
                    (cell.0 as f32 + w as f32 * 0.5) * s,
                    (cell.1 as f32 + h as f32 * 0.5) * s,
                    p.z,
                );
                self.placement = Some(Placement {
                    valid: ok,
                    reason,
                    cost: d.cost,
                    model: self.structure_model(def, center, rot),
                });
            }
            // The original shows the bottom station at the pointer before
            // the first click, without a cost (CLift state 1, 0x4a0a00).
            Tool::Lift { start: None, .. } => {
                let s = self.world.level().spacing;
                let taken = self
                    .world
                    .cell_index((p.x / s) as i32, (p.y / s) as i32)
                    .is_none_or(|i| self.world.occupied[i].is_some());
                self.placement = Some(Placement {
                    valid: !taken,
                    reason: None,
                    cost: 0.0,
                    model: Mat4::IDENTITY,
                });
            }
            Tool::Lift { def, start: Some(st) } => {
                let (_, cost, reason) = self.lift_plan(def, st, p);
                self.placement = Some(Placement {
                    valid: reason.is_none(),
                    reason,
                    cost,
                    model: Mat4::IDENTITY,
                });
            }
            Tool::Park { def, start: Some(st) } => {
                let (cost, reason) = self.park_check(def, st, p);
                self.placement = Some(Placement {
                    valid: reason.is_none(),
                    reason,
                    cost,
                    model: Mat4::IDENTITY,
                });
            }
            Tool::Run { def, gates } => {
                let cost = self.defs.runs[def].cost * (gates.len() + 1) as f32;
                self.placement = Some(Placement {
                    valid: true,
                    reason: None,
                    cost,
                    model: Mat4::from_translation(p),
                });
            }
            _ => {}
        }
    }

    /// The terrain vertex nearest a point, as the survey snaps to.
    pub fn nearest_vertex(&self, p: Vec3) -> Vec3 {
        let lv = self.world.level();
        let s = lv.spacing;
        let max = lv.size.saturating_sub(1) as f32;
        let x = (p.x / s).round().clamp(0.0, max);
        let y = (p.y / s).round().clamp(0.0, max);
        Vec3::new(x * s, y * s, lv.height(x as usize, y as usize))
    }
}
