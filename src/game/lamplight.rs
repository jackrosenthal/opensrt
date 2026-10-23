//! Lamp light on the ground at night. The original lights its terrain a
//! vertex at a time (0x42a4f0) and its light evaluator traces a line from
//! each point to the lamp, dropping the lamp when anything is in the way
//! (0x51a3ef, 0x508c40). So a lamp under a roof lights the floor beneath
//! it but not the snow outside the walls.

use std::sync::Arc;

use glam::{Mat4, Vec3};

use crate::formats::level::Level;
use crate::render::Lamp;

/// The line test leaves this much of the line at its start alone
/// (0x51a3d8 passes 0.5), so a surface does not shadow itself. None is
/// left at the lamp's end, so a lamp inside its fitting lights nothing
/// beyond it, as with the entrance's pillar lamps.
const CLEAR: f32 = 0.5;

/// A mesh's triangles in its own space, grouped in a bounding volume
/// hierarchy for line tests.
pub struct Solid {
    tris: Vec<[Vec3; 3]>,
    nodes: Vec<Node>,
}

struct Node {
    min: Vec3,
    max: Vec3,
    /// Leaves: the triangles `first..first + count`; inner nodes: count 0
    /// and the children at `first` and `first + 1`.
    first: usize,
    count: usize,
}

const LEAF: usize = 8;

impl Solid {
    pub fn new(mut tris: Vec<[Vec3; 3]>) -> Solid {
        let mut nodes = Vec::new();
        if !tris.is_empty() {
            let n = tris.len();
            nodes.push(Node { min: Vec3::ZERO, max: Vec3::ZERO, first: 0, count: n });
            Self::split(&mut tris, &mut nodes, 0);
        }
        Solid { tris, nodes }
    }

    fn bounds(tris: &[[Vec3; 3]]) -> (Vec3, Vec3) {
        tris.iter().flatten().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(a, b), p| (a.min(*p), b.max(*p)))
    }

    fn split(tris: &mut [[Vec3; 3]], nodes: &mut Vec<Node>, i: usize) {
        let (first, count) = (nodes[i].first, nodes[i].count);
        let (min, max) = Self::bounds(&tris[first..first + count]);
        nodes[i].min = min;
        nodes[i].max = max;
        if count <= LEAF {
            return;
        }
        // Halve along the longest side by triangle centers.
        let axis = (max - min).max_position();
        let part = &mut tris[first..first + count];
        let center = |t: &[Vec3; 3]| (t[0][axis] + t[1][axis] + t[2][axis]) / 3.0;
        part.sort_by(|a, b| center(a).total_cmp(&center(b)));
        let half = count / 2;
        let left = nodes.len();
        nodes[i].first = left;
        nodes[i].count = 0;
        nodes.push(Node { min, max, first, count: half });
        nodes.push(Node { min, max, first: first + half, count: count - half });
        Self::split(tris, nodes, left);
        Self::split(tris, nodes, left + 1);
    }

    /// Whether the segment from `a` along `dir` between `t0` and `t1`
    /// passes through any triangle.
    fn hits(&self, a: Vec3, dir: Vec3, t0: f32, t1: f32) -> bool {
        if self.nodes.is_empty() {
            return false;
        }
        let inv = dir.recip();
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i];
            if !slab(a, inv, n.min, n.max, t0, t1) {
                continue;
            }
            if n.count == 0 {
                stack.push(n.first);
                stack.push(n.first + 1);
                continue;
            }
            for t in &self.tris[n.first..n.first + n.count] {
                if crossing(a, dir, t).is_some_and(|s| s > t0 && s < t1) {
                    return true;
                }
            }
        }
        false
    }
}

/// Whether the segment meets the box (ray-slab test).
fn slab(a: Vec3, inv: Vec3, min: Vec3, max: Vec3, t0: f32, t1: f32) -> bool {
    let p = (min - a) * inv;
    let q = (max - a) * inv;
    let near = p.min(q).max_element().max(t0);
    let far = p.max(q).min_element().min(t1);
    near <= far
}

/// Where along the ray it crosses a triangle, either face.
fn crossing(a: Vec3, dir: Vec3, t: &[Vec3; 3]) -> Option<f32> {
    let e1 = t[1] - t[0];
    let e2 = t[2] - t[0];
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-8 {
        return None;
    }
    let inv = 1.0 / det;
    let s = a - t[0];
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    Some(e2.dot(q) * inv)
}

/// A solid placed in the world.
pub struct Occluder {
    pub solid: Arc<Solid>,
    pub model: Mat4,
}

/// Lamp light at each terrain height sample (`level.size` squared, rows of
/// x), with the original's falloff between a lamp's inner and outer
/// radius, its spot cone, and the ground's facing, and none where a
/// building stands between.
pub fn ground_light(level: &Level, lamps: &[Lamp], occluders: &[Occluder]) -> Vec<[f32; 4]> {
    let n = level.size;
    let s = level.spacing;
    let mut out = vec![[0.0f32; 4]; n * n];
    if lamps.is_empty() || n == 0 {
        return out;
    }
    // Each occluder's world bounds and the inverse of its placement.
    let placed: Vec<(Vec3, Vec3, Mat4, &Solid)> = occluders
        .iter()
        .filter(|o| !o.solid.nodes.is_empty())
        .map(|o| {
            let r = &o.solid.nodes[0];
            let corners = (0..8).map(|k| {
                let c = Vec3::new(
                    if k & 1 == 0 { r.min.x } else { r.max.x },
                    if k & 2 == 0 { r.min.y } else { r.max.y },
                    if k & 4 == 0 { r.min.z } else { r.max.z },
                );
                o.model.transform_point3(c)
            });
            let (min, max) = corners.fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(a, b), p| (a.min(p), b.max(p)));
            (min, max, o.model.inverse(), o.solid.as_ref())
        })
        .collect();
    let blocked = |p: Vec3, lamp: Vec3| -> bool {
        let len = (lamp - p).length();
        if len <= CLEAR {
            return false;
        }
        let (lo, hi) = (p.min(lamp), p.max(lamp));
        placed.iter().any(|(min, max, inv, solid)| {
            if hi.cmplt(*min).any() || lo.cmpgt(*max).any() {
                return false;
            }
            // In the solid's own space; its placement has no scale, so
            // distances along the line carry over.
            let a = inv.transform_point3(p);
            let b = inv.transform_point3(lamp);
            let dir = (b - a) / len;
            solid.hits(a, dir, CLEAR, len)
        })
    };
    let normal = |x: usize, y: usize| {
        let hl = level.height(x.saturating_sub(1), y);
        let hr = level.height((x + 1).min(n - 1), y);
        let hd = level.height(x, y.saturating_sub(1));
        let hu = level.height(x, (y + 1).min(n - 1));
        Vec3::new(hl - hr, hd - hu, 2.0 * s).normalize()
    };
    for l in lamps {
        let lp = Vec3::new(l.pos_outer[0], l.pos_outer[1], l.pos_outer[2]);
        let (outer, inner) = (l.pos_outer[3], l.color_inner[3]);
        let color = Vec3::new(l.color_inner[0], l.color_inner[1], l.color_inner[2]);
        let (outer2, inner2) = (outer * outer, inner * inner);
        let span = |c: f32| (((c - outer) / s).floor().max(0.0) as usize, (((c + outer) / s).ceil().max(0.0) as usize).min(n - 1));
        let (x0, x1) = span(lp.x);
        let (y0, y1) = span(lp.y);
        for gy in y0..=y1 {
            for gx in x0..=x1 {
                let p = Vec3::new(gx as f32 * s, gy as f32 * s, level.height(gx, gy));
                let d = lp - p;
                let d2 = d.length_squared();
                if d2 >= outer2 {
                    continue;
                }
                let fall = ((outer2 - d2) / (outer2 - inner2).max(1e-4)).clamp(0.0, 1.0);
                let to = d.normalize_or_zero();
                let facing = normal(gx, gy).dot(to).max(0.0);
                let mut cone = 1.0;
                if l.spot[3] > 0.5 {
                    // Full inside the inner cone, fading by angle to none at
                    // the outer one (0x51ac20).
                    let a = (-to).dot(Vec3::new(l.spot[0], l.spot[1], l.spot[2])).clamp(-1.0, 1.0).acos();
                    cone = ((l.cones[0] - a) / (l.cones[0] - l.cones[1]).max(1e-4)).clamp(0.0, 1.0);
                }
                let k = fall * facing * cone;
                if k <= 0.0 || blocked(p, lp) {
                    continue;
                }
                let o = &mut out[gy * n + gx];
                for c in 0..3 {
                    o[c] += color[c] * k;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_through_a_wall_is_blocked() {
        // A wall in the x = 0 plane, 10 square.
        let wall = Solid::new(vec![
            [Vec3::new(0.0, -5.0, 0.0), Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, 5.0, 10.0)],
            [Vec3::new(0.0, -5.0, 0.0), Vec3::new(0.0, 5.0, 10.0), Vec3::new(0.0, -5.0, 10.0)],
        ]);
        let a = Vec3::new(-3.0, 0.0, 2.0);
        let b = Vec3::new(3.0, 0.0, 2.0);
        let len = (b - a).length();
        assert!(wall.hits(a, (b - a) / len, CLEAR, len - CLEAR));
        let c = Vec3::new(3.0, 0.0, 22.0);
        let len = (c - a).length();
        assert!(!wall.hits(a, (c - a) / len, CLEAR, len - CLEAR));
    }
}
