//! Terrain editing: sculpting, snow tools and trees.

use super::*;

/// What a cell of footpath costs.
const PATH_COST: f32 = 1.0;

impl Game {
    /// The surface tools, as the original runs them (0x423b40): press and
    /// drag to pick a rectangle of cells. Smoothing, moguls, snowmaking
    /// and grooming act on it when the button comes up. The height tools
    /// keep it picked; pressing on one of its corner points and dragging
    /// up or down then reshapes it about that point, a quarter unit for
    /// every pixel the pointer moves. Pressing anywhere else starts over.
    pub(super) fn surface_input(&mut self, t: TerrainTool, input: &Input, size: Vec2) {
        // The original's pixels, at 768 lines.
        let px = 768.0 / size.y.max(1.0);
        let hover_cell = self.hover.map(|p| self.cell_of(p));
        if input.left_clicked {
            let corner = self.hover.and_then(|p| {
                let sel = self.terrain_sel.as_ref().filter(|s| s.phase == SelPhase::Picked && t.shapes())?;
                let s = self.world.level().spacing;
                let point = ((p.x / s).round() as i32, (p.y / s).round() as i32);
                let ((x0, y0), (x1, y1)) = sel.rect();
                let inside = (x0..=x1 + 1).contains(&point.0) && (y0..=y1 + 1).contains(&point.1);
                // Where in the rectangle the point lies, kept off its edges
                // (0x4228b0).
                let frac = |v: i32, lo: i32, hi: i32| ((v - lo) as f32 / (hi + 1 - lo).max(1) as f32).clamp(0.01, 0.99);
                inside.then(|| (point, Vec2::new(frac(point.0, x0, x1), frac(point.1, y0, y1))))
            });
            self.terrain_sel = match (corner, hover_cell) {
                (Some((point, center)), _) => self.terrain_sel.clone().map(|s| TerrainSel {
                    phase: SelPhase::Shaping {
                        point: (point.0 as usize, point.1 as usize),
                        center,
                        last_y: input.mouse.y,
                    },
                    ..s
                }),
                (None, Some(c)) => Some(TerrainSel {
                    from: c,
                    to: c,
                    phase: SelPhase::Dragging,
                }),
                (None, None) => None,
            };
            return;
        }
        let Some(mut sel) = self.terrain_sel.clone() else { return };
        match sel.phase {
            SelPhase::Dragging if input.left_down => {
                if let Some(c) = hover_cell {
                    sel.to = c;
                }
            }
            SelPhase::Dragging => {
                sel.phase = SelPhase::Picked;
                if !t.shapes() {
                    // The one-off tools act and let go of the rectangle.
                    self.surface_rect(t, sel.rect());
                    self.terrain_sel = None;
                    return;
                }
            }
            SelPhase::Shaping { point, center, last_y } if input.left_down => {
                let d = (last_y - input.mouse.y) * px * 0.25;
                if d != 0.0 {
                    self.shape(t, sel.rect(), point, center, d);
                }
                sel.phase = SelPhase::Shaping {
                    point,
                    center,
                    last_y: input.mouse.y,
                };
            }
            SelPhase::Shaping { .. } => sel.phase = SelPhase::Picked,
            SelPhase::Picked => {}
        }
        self.terrain_sel = Some(sel);
    }

    /// Whether a height point may move: one of the four cells round it
    /// must be free of buildings and lifts (the 0x41 cell flags).
    pub(super) fn movable(&self, x: i32, y: i32) -> bool {
        let n = self.world.level().cells_per_side() as i32;
        [(x, y), (x - 1, y), (x, y - 1), (x - 1, y - 1)].iter().any(|&(a, b)| {
            a >= 0 && b >= 0 && a < n && b < n && {
                let i = (b * n + a) as usize;
                self.world.occupied[i].is_none() && self.world.lift_cells.get(i).is_none_or(|c| c.is_none())
            }
        })
    }

    /// Raises (or with `d` below zero, lowers) the picked cells' corners:
    /// a rounded hill (0x420070) or a pointed peak (0x41f7b0) rising most
    /// at `center` across the rectangle, everything to the corners'
    /// average plus `d` (0x41fd30), or the one point pressed, at most ten
    /// a step (0x41f670).
    fn shape(&mut self, t: TerrainTool, rect: ((i32, i32), (i32, i32)), point: (usize, usize), center: Vec2, d: f32) {
        let ((x0, y0), (x1, y1)) = rect;
        // The rectangle's corner points.
        let (px0, py0, px1, py1) = (x0, y0, x1 + 1, y1 + 1);
        let size = self.world.level().size as i32;
        let mut changed = false;
        match t {
            TerrainTool::Point => {
                let (x, y) = (point.0 as i32, point.1 as i32);
                if x < size && y < size && self.movable(x, y) {
                    let h = self.world.level().height(point.0, point.1);
                    self.world.level_mut().set_height(point.0, point.1, h + d.clamp(-10.0, 10.0));
                    changed = true;
                }
            }
            TerrainTool::Flatten => {
                let lv = self.world.level();
                let mut sum = 0.0;
                let mut count = 0.0;
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        for (a, b) in [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)] {
                            sum += lv.height(a as usize, b as usize);
                            count += 1.0;
                        }
                    }
                }
                let to = sum / f32::max(count, 1.0) + d;
                for y in py0..=py1.min(size - 1) {
                    for x in px0..=px1.min(size - 1) {
                        if self.movable(x, y) {
                            self.world.level_mut().set_height(x as usize, y as usize, to);
                            changed = true;
                        }
                    }
                }
            }
            TerrainTool::Hill | TerrainTool::Peak => {
                let (w, h) = ((px1 - px0) as f32, (py1 - py0) as f32);
                let (cx, cy) = (w * center.x, h * center.y);
                let hill = t == TerrainTool::Hill;
                // Each axis rises from nothing at the edges to one at the
                // center: along a quarter cosine for a hill, straight for
                // a peak.
                let rise = |i: f32, c: f32, len: f32| -> f32 {
                    let f = if i <= c { i / c.max(1e-3) } else { 1.0 - (i - c) / (len - c).max(1e-3) };
                    if hill {
                        ((1.0 - f) * std::f32::consts::FRAC_PI_2).cos()
                    } else {
                        f
                    }
                };
                let mut weights = Vec::new();
                let mut top = 0.0f32;
                for y in py0..=py1 {
                    for x in px0..=px1 {
                        let wgt = rise((x - px0) as f32, cx, w) * rise((y - py0) as f32, cy, h);
                        top = top.max(wgt);
                        weights.push((x, y, wgt));
                    }
                }
                for (x, y, wgt) in weights {
                    if x < size && y < size && top > 0.0 && self.movable(x, y) {
                        let z = self.world.level().height(x as usize, y as usize);
                        self.world.level_mut().set_height(x as usize, y as usize, z + d * wgt / top);
                        changed = true;
                    }
                }
            }
            _ => {}
        }
        // Reshaping the ground costs nothing: the original's height tools
        // charge no money.
        if changed {
            self.terrain_dirty = true;
            self.foliage_dirty = true;
        }
    }

    /// Smoothing (0x41f230): each height point of a rectangle of cells,
    /// that may move, becomes the average of itself and its eight
    /// neighbors, once.
    pub(super) fn smooth_rect(&mut self, rect: ((i32, i32), (i32, i32))) {
        let ((x0, y0), (x1, y1)) = rect;
        let size = self.world.level().size as i32;
        let (lo_x, lo_y) = (x0.max(1), y0.max(1));
        let (hi_x, hi_y) = ((x1 + 1).min(size - 2), (y1 + 1).min(size - 2));
        let lv = self.world.level();
        let mut new = Vec::new();
        for y in lo_y..=hi_y {
            for x in lo_x..=hi_x {
                if !self.movable(x, y) {
                    continue;
                }
                let mut sum = 0.0;
                for b in y - 1..=y + 1 {
                    for a in x - 1..=x + 1 {
                        sum += lv.height(a as usize, b as usize);
                    }
                }
                new.push((x as usize, y as usize, sum / 9.0));
            }
        }
        if !new.is_empty() {
            for (x, y, z) in new {
                self.world.level_mut().set_height(x, y, z);
            }
            self.terrain_dirty = true;
            self.foliage_dirty = true;
        }
    }

    /// The cells in a rectangle a snow tool would work on.
    pub(super) fn surface_cells(&self, t: TerrainTool, rect: ((i32, i32), (i32, i32))) -> Vec<(usize, usize)> {
        use crate::formats::level::cell_flags::MOGULS;
        let ((x0, y0), (x1, y1)) = rect;
        let n = self.world.level().cells_per_side() as i32;
        let lv = self.world.level();
        (y0.max(0)..=y1.min(n - 1))
            .flat_map(|y| (x0.max(0)..=x1.min(n - 1)).map(move |x| (x as usize, y as usize)))
            .filter(|&(x, y)| {
                let c = lv.cell(x, y);
                match t {
                    // Not on trees, paths, buildings or lifts (0x409e30:
                    // none of the 0x1047 flags).
                    TerrainTool::Moguls => {
                        let i = y * n as usize + x;
                        c.flags & (MOGULS | crate::formats::level::cell_flags::FOREST) == 0
                            && c.foliage == 0
                            && c.path().is_none()
                            && self.world.occupied[i].is_none()
                            && self.world.lift_cells.get(i).is_none_or(|l| l.is_none())
                    }
                    // Not already waiting for a machine (flags 0x204 and
                    // 0x500 at 0x423b40: moguls or worn, and not queued).
                    TerrainTool::Groom => c.flags & MOGULS != 0 && !self.world.groom_jobs.contains(&(x, y)),
                    TerrainTool::MakeSnow => c.wear > 20.0 && !self.world.snow_jobs.contains(&(x, y)),
                    _ => false,
                }
            })
            .collect()
    }

    /// Whether any height point of a rectangle may move.
    pub(super) fn changeable(&self, rect: ((i32, i32), (i32, i32))) -> bool {
        let ((x0, y0), (x1, y1)) = rect;
        (y0..=y1 + 1).any(|y| (x0..=x1 + 1).any(|x| self.movable(x, y)))
    }

    /// Smoothing, moguls, snowmaking and grooming on a picked rectangle.
    /// Smoothing is free. The snow tools cost $5 for each cell
    /// they work on, as landscaping (0x423b40 tools 6 to 8), and only if
    /// the resort can afford it all.
    fn surface_rect(&mut self, t: TerrainTool, rect: ((i32, i32), (i32, i32))) {
        use crate::formats::level::cell_flags::MOGULS;
        if t == TerrainTool::Smooth {
            self.smooth_rect(rect);
            return;
        }
        let todo = self.surface_cells(t, rect);
        let cost = todo.len() as f32 * 5.0;
        if cost <= 0.0 {
            return;
        }
        if cost > self.world.cash && !self.world.sandbox {
            self.error(Problem::TooExpensive);
            return;
        }
        // Grooming and snowmaking are work for the resort's grooming and
        // snowmaking machines (0x41ef50, 0x41f0c0): the cells go on their
        // list, or with none of them nothing comes of it, though it is
        // paid for. Each machine drives out to the nearest in turn.
        match t {
            TerrainTool::Groom | TerrainTool::MakeSnow => {
                let (kind, jobs) = if t == TerrainTool::Groom {
                    (StaffKind::Groomer, &mut self.world.groom_jobs)
                } else {
                    (StaffKind::Snowmaker, &mut self.world.snow_jobs)
                };
                if self.world.staff.iter().any(|s| s.kind == kind) {
                    jobs.extend(todo);
                }
            }
            _ => {
                for &(x, y) in &todo {
                    self.world.level_mut().cell_mut(x, y).flags |= MOGULS;
                }
                self.terrain_dirty = true;
            }
        }
        let at = self.rect_center(rect);
        self.landscape(cost, at);
    }

    /// Pays for landscaping, the cost floating off from where it was done
    /// as the original shows it (0x42ac90 with its show flag).
    fn landscape(&mut self, cost: f32, at: Vec3) {
        self.world.spend(Ledger::Landscaping, cost);
        self.floaters.push(Floater {
            at,
            text: format!("-{}", crate::ui::style::money(cost)),
            age: 0.0,
        });
    }

    /// The ground at the middle of a rectangle of cells.
    fn rect_center(&self, rect: ((i32, i32), (i32, i32))) -> Vec3 {
        let s = self.world.level().spacing;
        let ((x1, y1), (x2, y2)) = rect;
        self.world.ground(Vec2::new((x1 + x2 + 1) as f32 * 0.5 * s, (y1 + y2 + 1) as f32 * 0.5 * s))
    }

    /// Grid cell under a world point.
    pub fn cell_of(&self, p: Vec3) -> (i32, i32) {
        let s = self.world.level().spacing;
        let n = self.world.level().cells_per_side() as i32 - 1;
        (((p.x / s) as i32).clamp(0, n), ((p.y / s) as i32).clamp(0, n))
    }

    /// The tree rectangle being dragged (inclusive cells), if any.
    pub fn tree_rect(&self) -> Option<((i32, i32), (i32, i32))> {
        let a = self.tree_drag?;
        let b = self.cell_of(self.hover?);
        Some(((a.0.min(b.0), a.1.min(b.1)), (a.0.max(b.0), a.1.max(b.1))))
    }

    /// What the chainsaw charges for a rectangle: $10 per cell with trees,
    /// as in the original.
    pub fn chop_cost(&self, rect: ((i32, i32), (i32, i32))) -> f32 {
        let ((x1, y1), (x2, y2)) = rect;
        let lv = self.world.level();
        let mut n = 0;
        for y in y1..=y2 {
            for x in x1..=x2 {
                if lv.cell(x as usize, y as usize).foliage != 0 {
                    n += 1;
                }
            }
        }
        n as f32 * 10.0
    }

    pub(super) fn apply_trees(&mut self, density: u8, rect: ((i32, i32), (i32, i32))) {
        let ((x1, y1), (x2, y2)) = rect;
        if density == 0 {
            let cost = self.chop_cost(rect);
            if cost <= 0.0 {
                return;
            }
            if self.world.cash < cost && !self.world.sandbox {
                self.error(Problem::TooExpensive);
                return;
            }
            for y in y1..=y2 {
                for x in x1..=x2 {
                    if self.world.occupied[(y * self.world.level().cells_per_side() as i32 + x) as usize].is_none() {
                        self.world.level_mut().cell_mut(x as usize, y as usize).set_foliage(0);
                    }
                }
            }
            let at = self.rect_center(rect);
            self.landscape(cost, at);
            self.sfx(Sound::ChopTrees);
            self.foliage_dirty = true;
            return;
        }
        // Sparse, medium and dense: one cell in ten, one in five, or every
        // one (0x41eae0 with 2, 1 and 0).
        let chance = [0.0, 0.1, 0.2, 1.0][density.min(3) as usize];
        let n = self.world.level().cells_per_side() as i32;
        // Planting an area is free (0x423b40 tools 13 to 15 charge
        // nothing); single trees cost $10 (see `paint_trees`).
        let mut planted = false;
        for y in y1..=y2 {
            for x in x1..=x2 {
                if self.world.occupied[(y * n + x) as usize].is_some() {
                    continue;
                }
                let roll = self.world.rand();
                let c = self.world.level_mut().cell_mut(x as usize, y as usize);
                if c.foliage != 0 || c.path().is_some() {
                    continue;
                }
                if roll < chance {
                    c.set_foliage(13 + (roll * 1000.0) as u8 % 6);
                    planted = true;
                }
            }
        }
        if planted {
            self.foliage_dirty = true;
        }
    }

    // ----------------------------------------------------------- rendering

    /// Lays or takes up the footpath in the cell under the pointer, a
    /// dollar a cell either way; buildings and lifts keep theirs clear.
    pub(super) fn paint_path(&mut self, kind: Option<u8>, p: Vec3) {
        let (cx, cy) = self.cell_of(p);
        let n = self.world.level().cells_per_side() as i32;
        if cx < 0 || cy < 0 || cx >= n || cy >= n {
            return;
        }
        let i = (cy * n + cx) as usize;
        if self.world.occupied[i].is_some() || self.world.lift_cells.get(i).is_some_and(|c| c.is_some()) {
            return;
        }
        let cell = self.world.level().cell(cx as usize, cy as usize);
        // Not through trees (0x423630: none of the 0x6003 flags).
        if cell.path() == kind || (kind.is_some() && (cell.foliage != 0 || cell.flags & crate::formats::level::cell_flags::FOREST != 0)) {
            return;
        }
        // Laying a cell or taking one up costs the same dollar (0x423630
        // mode 2: the eraser acts only on paths, and charges when it does).
        if self.world.cash < PATH_COST && !self.world.sandbox {
            self.error(Problem::TooExpensive);
            return;
        }
        self.landscape(PATH_COST, p);
        self.world.level_mut().cell_mut(cx as usize, cy as usize).set_path(kind);
        self.terrain_dirty = true;
    }
}
