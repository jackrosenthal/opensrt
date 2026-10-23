//! Grid path finding over the terrain for skiing and walking.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use glam::Vec2;

use crate::formats::level::Level;

#[derive(Copy, Clone, PartialEq)]
struct Node {
    cost: f32,
    idx: usize,
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    /// Skiing down: uphill is very expensive, slopes outside the preferred
    /// band cost extra, too-steep terrain is avoided.
    Ski { min_slope: f32, max_slope: f32, danger: f32 },
    /// Walking: gentle terrain only, mild uphill penalty.
    Walk,
}

pub struct NavGrid<'a> {
    pub level: &'a Level,
    /// Cells that cannot be crossed (buildings).
    pub blocked: &'a [bool],
    /// Optional per-cell bonus (negative cost) e.g. along marked runs.
    pub bonus: Option<&'a [f32]>,
}

impl NavGrid<'_> {
    fn n(&self) -> usize {
        self.level.cells_per_side()
    }

    fn center(&self, i: usize) -> Vec2 {
        let n = self.n();
        let s = self.level.spacing;
        Vec2::new(((i % n) as f32 + 0.5) * s, ((i / n) as f32 + 0.5) * s)
    }

    pub fn cell_of(&self, p: Vec2) -> usize {
        let n = self.n() as i32;
        let s = self.level.spacing;
        let cx = ((p.x / s) as i32).clamp(0, n - 1);
        let cy = ((p.y / s) as i32).clamp(0, n - 1);
        (cy * n + cx) as usize
    }

    fn step_cost(&self, a: usize, b: usize, mode: Mode) -> Option<f32> {
        if self.blocked[b] {
            return None;
        }
        let pa = self.center(a);
        let pb = self.center(b);
        let dist = pa.distance(pb);
        let ha = self.level.height_at(pa.x, pa.y);
        let hb = self.level.height_at(pb.x, pb.y);
        let drop = ha - hb;
        let slope = (drop / dist).atan().to_degrees();
        let terrain = self.level.slope_deg_at(pb.x, pb.y);
        let bonus = self.bonus.map(|v| v[b]).unwrap_or(0.0);
        match mode {
            Mode::Ski {
                min_slope,
                max_slope,
                danger,
            } => {
                if terrain > danger {
                    return None;
                }
                let mut c = dist;
                if drop < -0.5 {
                    // Climbing on skis: allowed for short traverses only.
                    c += dist * 6.0 + (-drop) * 30.0;
                } else if slope < min_slope * 0.5 {
                    c += dist * 0.6;
                }
                if terrain > max_slope {
                    c += dist * (terrain - max_slope) * 0.35;
                } else if terrain < min_slope {
                    c += dist * (min_slope - terrain) * 0.04;
                }
                Some((c - bonus * dist).max(dist * 0.25))
            }
            Mode::Walk => {
                if terrain > 35.0 {
                    return None;
                }
                let up = (-drop).max(0.0);
                Some(dist + up * 2.0 + terrain * 0.05 * dist)
            }
        }
    }

    /// A* from `from` to whichever goal is cheapest. Returns world-space
    /// waypoints (cell centers), ending exactly at the reached goal point.
    pub fn find(&self, from: Vec2, goals: &[Vec2], mode: Mode, max_expand: usize) -> Option<Vec<Vec2>> {
        self.find_raw(from, goals, mode, max_expand).map(smooth)
    }

    /// As `find`, every cell's center kept.
    pub fn find_raw(&self, from: Vec2, goals: &[Vec2], mode: Mode, max_expand: usize) -> Option<Vec<Vec2>> {
        let n = self.n();
        let total = n * n;
        let start = self.cell_of(from);
        let goal_cells: Vec<usize> = goals.iter().map(|g| self.cell_of(*g)).collect();
        if goal_cells.contains(&start) {
            let gi = goal_cells.iter().position(|g| *g == start).unwrap();
            return Some(vec![goals[gi]]);
        }
        let h = |i: usize| {
            let p = self.center(i);
            goals
                .iter()
                .map(|g| p.distance(*g))
                .fold(f32::MAX, f32::min)
                * 0.9
        };
        let mut g = vec![f32::MAX; total];
        let mut prev = vec![usize::MAX; total];
        let mut heap = BinaryHeap::new();
        g[start] = 0.0;
        heap.push(Node { cost: h(start), idx: start });
        let mut expanded = 0;
        let mut found = None;
        while let Some(Node { idx, .. }) = heap.pop() {
            if let Some(k) = goal_cells.iter().position(|c| *c == idx) {
                found = Some((idx, k));
                break;
            }
            expanded += 1;
            if expanded > max_expand {
                break;
            }
            let (x, y) = ((idx % n) as i32, (idx / n) as i32);
            for (dx, dy) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= n as i32 || ny >= n as i32 {
                    continue;
                }
                let j = (ny as usize) * n + nx as usize;
                let Some(c) = self.step_cost(idx, j, mode) else { continue };
                let ng = g[idx] + c;
                if ng < g[j] {
                    g[j] = ng;
                    prev[j] = idx;
                    heap.push(Node { cost: ng + h(j), idx: j });
                }
            }
        }
        let (end, k) = found?;
        let mut cells = vec![end];
        let mut cur = end;
        while prev[cur] != usize::MAX {
            cur = prev[cur];
            cells.push(cur);
        }
        cells.reverse();
        let mut pts: Vec<Vec2> = cells.iter().skip(1).map(|&c| self.center(c)).collect();
        if let Some(last) = pts.last_mut() {
            *last = goals[k];
        } else {
            pts.push(goals[k]);
        }
        Some(pts)
    }
}

/// Removes redundant collinear waypoints to make movement less jagged.
fn smooth(pts: Vec<Vec2>) -> Vec<Vec2> {
    if pts.len() < 3 {
        return pts;
    }
    let mut out = vec![pts[0]];
    for i in 1..pts.len() - 1 {
        let a = *out.last().unwrap();
        let b = pts[i];
        let c = pts[i + 1];
        let d1 = (b - a).normalize_or_zero();
        let d2 = (c - b).normalize_or_zero();
        if d1.dot(d2) < 0.995 {
            out.push(b);
        }
    }
    out.push(*pts.last().unwrap());
    out
}
