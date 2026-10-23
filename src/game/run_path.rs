//! The Path Skirun tools, as the original runs them (0x4248a0): a ski
//! run is laid out node by node and cut into the ground when Enter is
//! pressed. Each node is a cell, with a width (how far the pointer is
//! from it, state 1) and a depth (two for every cell the pointer is
//! below it, state 2); then the next leg's end is placed (state 3), and
//! so on. Esc throws the layout away. Cutting is free.

use super::*;

/// A node of a run being laid out: its cell, the ground height there
/// when it was placed, and the half width (cells) and depth of the cut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunNode {
    pub cell: (i32, i32),
    pub height: f32,
    pub width: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunPhase {
    /// Placing the first node (SKIRUN_NODE_PLACING_START).
    #[default]
    Start,
    /// Setting the node's width (SKIRUN_NODE_SETTING_WIDTH).
    Width,
    /// Setting its depth (SKIRUN_NODE_SETTING_HEIGHT).
    Depth,
    /// Placing the next leg's end (SKIRUN_NODE_PLACING_LEG).
    Leg,
}

/// A run being laid out with the Path Skirun tools.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunLayout {
    pub nodes: Vec<RunNode>,
    /// The node being placed, before it joins `nodes`.
    pub current: Option<RunNode>,
    pub phase: RunPhase,
}

impl Game {
    /// Mouse and keys for the Path Skirun tools. `mode` 0 follows the
    /// existing ground, 1 only cuts, 2 cuts and fills.
    pub(super) fn run_path_input(&mut self, mode: u8, input: &Input) {
        let cell = self.hover.map(|p| self.cell_of(p));
        if input.pressed.contains(&KeyCode::Enter) && self.run_layout.nodes.len() >= 2 {
            self.cut_run(mode);
            self.run_layout = RunLayout::default();
            return;
        }
        let Some(cell) = cell else { return };
        let lay = &mut self.run_layout;
        if input.left_clicked {
            match lay.phase {
                RunPhase::Start | RunPhase::Leg => {
                    let height = self.world.level().height(cell.0 as usize, cell.1 as usize);
                    lay.current = Some(RunNode { cell, height, width: 0.0, depth: 0.0 });
                    lay.phase = RunPhase::Width;
                }
                RunPhase::Width => lay.phase = RunPhase::Depth,
                RunPhase::Depth => {
                    if let Some(n) = lay.current.take() {
                        lay.nodes.push(n);
                    }
                    lay.phase = RunPhase::Leg;
                }
            }
            return;
        }
        let Some(n) = lay.current.as_mut() else { return };
        match lay.phase {
            // As many cells as the pointer is from the node (the original
            // counts the cells along the line through the node to the
            // pointer and its mirror image, and halves that).
            RunPhase::Width => {
                let d = Vec2::new((cell.0 - n.cell.0) as f32, (cell.1 - n.cell.1) as f32);
                n.width = d.length().floor();
            }
            RunPhase::Depth => n.depth = 2.0 * (n.cell.1 - cell.1).max(0) as f32,
            _ => {}
        }
    }

    /// Esc while laying out a run: the layout goes. Whether there was one.
    pub(super) fn drop_run_layout(&mut self) -> bool {
        let had = self.run_layout != RunLayout::default();
        self.run_layout = RunLayout::default();
        had
    }

    /// What the tool says next to the pointer: the step, and once two
    /// nodes are down, that Enter cuts the run.
    pub(super) fn run_path_label(&self) -> Text {
        let ready = self.run_layout.nodes.len() >= 2;
        match (self.run_layout.phase, ready) {
            (RunPhase::Start, _) => Text::SkirunNodePlacingStart,
            (RunPhase::Width, false) => Text::SkirunNodeSettingWidth,
            (RunPhase::Width, true) => Text::SkirunNodeSettingWidthConfirm,
            (RunPhase::Depth, false) => Text::SkirunNodeSettingHeight,
            (RunPhase::Depth, true) => Text::SkirunNodeSettingHeightConfirm,
            (RunPhase::Leg, false) => Text::SkirunNodePlacingLeg,
            (RunPhase::Leg, true) => Text::SkirunNodePlacingLegConfirm,
        }
    }

    /// The cells to show while laying out: every leg's corridor, the
    /// one to the pointer, and the width or depth being set.
    pub(super) fn run_path_marks(&self) -> Vec<(usize, usize)> {
        let lay = &self.run_layout;
        let n = self.world.level().cells_per_side() as i32;
        let mut out = Vec::new();
        let mut push = |x: i32, y: i32| {
            if x >= 0 && y >= 0 && x < n && y < n {
                out.push((x as usize, y as usize));
            }
        };
        let legs = lay.nodes.windows(2).map(|w| (w[0], w[1]));
        let hover = self.hover.map(|p| self.cell_of(p));
        let pending = match (lay.phase, lay.nodes.last(), hover) {
            (RunPhase::Leg, Some(a), Some(c)) => Some((*a, RunNode { cell: c, ..*a })),
            _ => None,
        };
        for (a, b) in legs.chain(pending) {
            for (x, y, _) in corridor(a, b) {
                push(x, y);
            }
        }
        if let Some(c) = lay.current {
            match lay.phase {
                RunPhase::Width => {
                    if let Some(h) = hover {
                        let (dx, dy) = (h.0 - c.cell.0, h.1 - c.cell.1);
                        let steps = dx.abs().max(dy.abs());
                        for k in -steps..=steps {
                            let t = k as f32 / steps.max(1) as f32;
                            push(c.cell.0 + (dx as f32 * t).round() as i32, c.cell.1 + (dy as f32 * t).round() as i32);
                        }
                    }
                }
                RunPhase::Depth => {
                    for k in 0..=(c.depth / 2.0) as i32 {
                        push(c.cell.0, c.cell.1 - k);
                    }
                }
                _ => push(c.cell.0, c.cell.1),
            }
        }
        out
    }

    /// Cuts the run along its legs (0x4248a0 state 4). Every height point
    /// within a leg's width, the width going evenly from one node's to
    /// the next, is set to the straight grade between the nodes' heights
    /// (cut and fill, mode 2), to the lower of that and the ground (cut
    /// only, mode 1), or left at the ground (follow, mode 0), and then
    /// sunk by the depth there times cos(pi/2 x (distance / width)^2),
    /// and never below zero. A point is cut by the first leg that reaches
    /// it. The trees in the cut go, and it is smoothed, at no cost.
    fn cut_run(&mut self, mode: u8) {
        let nodes = self.run_layout.nodes.clone();
        let size = self.world.level().size as i32;
        let mut done = std::collections::HashSet::new();
        for w in nodes.windows(2) {
            let (a, b) = (w[0], w[1]);
            for (x, y, along) in corridor(a, b) {
                if x < 0 || y < 0 || x >= size || y >= size || !done.insert((x, y)) {
                    continue;
                }
                // Buildings and lifts keep their ground (my choice; the
                // original cuts under them too).
                if !self.movable(x, y) {
                    continue;
                }
                let Along { t, dist2, width, at } = along;
                let ground = self.world.level().height(x as usize, y as usize);
                let lerp = |p: f32, q: f32| match at {
                    End::A => p,
                    End::B => q,
                    End::Between => p + (q - p) * t,
                };
                let mut h = if mode == 0 { ground } else { lerp(a.height, b.height) };
                let depth = lerp(a.depth, b.depth);
                h -= depth * (dist2 / (width * width) * std::f32::consts::FRAC_PI_2).cos();
                h = h.max(0.0);
                if mode == 1 {
                    h = h.min(ground);
                }
                self.world.level_mut().set_height(x as usize, y as usize, h);
            }
        }
        if done.is_empty() {
            return;
        }
        // Trees off the cut cells, then the cut's box smoothed (the vtable
        // +0x40 and +0x44 calls).
        let n = self.world.level().cells_per_side() as i32;
        for &(x, y) in &done {
            if x < n && y < n && self.world.occupied[(y * n + x) as usize].is_none() {
                self.world.level_mut().cell_mut(x as usize, y as usize).set_foliage(0);
            }
        }
        let (x0, x1) = (done.iter().map(|c| c.0).min().unwrap_or(0), done.iter().map(|c| c.0).max().unwrap_or(0));
        let (y0, y1) = (done.iter().map(|c| c.1).min().unwrap_or(0), done.iter().map(|c| c.1).max().unwrap_or(0));
        self.smooth_rect(((x0, y0), (x1.min(n - 1), y1.min(n - 1))));
        self.terrain_dirty = true;
        self.foliage_dirty = true;
    }
}

/// Which end of a leg is nearest a point, if either.
#[derive(Debug, Clone, Copy, PartialEq)]
enum End {
    A,
    B,
    Between,
}

/// Where a point lies against a leg: how far along (0..1), its squared
/// distance from the leg, the leg's width there, and which end it is
/// nearest when past one.
#[derive(Debug, Clone, Copy)]
struct Along {
    t: f32,
    dist2: f32,
    width: f32,
    at: End,
}

/// The height points within a leg's width, with where they lie.
fn corridor(a: RunNode, b: RunNode) -> Vec<(i32, i32, Along)> {
    let pa = Vec2::new(a.cell.0 as f32, a.cell.1 as f32);
    let pb = Vec2::new(b.cell.0 as f32, b.cell.1 as f32);
    let ab = pb - pa;
    let len2 = ab.length_squared();
    let reach = a.width.max(b.width).ceil() as i32 + 1;
    let (x0, x1) = (a.cell.0.min(b.cell.0) - reach, a.cell.0.max(b.cell.0) + reach);
    let (y0, y1) = (a.cell.1.min(b.cell.1) - reach, a.cell.1.max(b.cell.1) + reach);
    let mut out = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = Vec2::new(x as f32, y as f32);
            let s = if len2 > 0.0 { (p - pa).dot(ab) / len2 } else { 0.0 };
            let (q, at) = if s <= 0.0 {
                (pa, End::A)
            } else if s >= 1.0 {
                (pb, End::B)
            } else {
                (pa + ab * s, End::Between)
            };
            let t = s.clamp(0.0, 1.0);
            let width = match at {
                End::A => a.width,
                End::B => b.width,
                End::Between => a.width + (b.width - a.width) * t,
            };
            let dist2 = q.distance_squared(p);
            if dist2 < width * width {
                out.push((x, y, Along { t, dist2, width, at }));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corridor_follows_the_leg() {
        let node = |x: i32, w: f32| RunNode { cell: (x, 0), height: 0.0, width: w, depth: 0.0 };
        let c = corridor(node(0, 2.0), node(10, 2.0));
        let find = |x: i32, y: i32| c.iter().find(|p| p.0 == x && p.1 == y).map(|p| p.2);
        let mid = find(5, 0).expect("on the line");
        assert!((mid.t - 0.5).abs() < 1e-6 && mid.at == End::Between);
        assert!(find(5, 1).is_some());
        assert!(find(5, 2).is_none(), "the edge itself is outside");
        assert_eq!(find(-1, 0).map(|a| a.at), Some(End::A));
        assert!(find(13, 0).is_none());
    }
}
