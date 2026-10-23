//! Lift geometry from the station meshes: cable loops, queue lines and
//! the cells lifts take up.

use super::*;

impl Game {
    /// Marks the cells each lift takes up, as the original does when a lift
    /// is placed (0x4a3010): diamonds of cells |dx|+|dy| < 5 round the
    /// bottom station, < 4 round the top one and along the line. Nothing
    /// else can then be placed on them.
    /// Recomputes each lift's cable loop, queue lines, exit path and where
    /// riders get on and off, from its station meshes' attach points; and
    /// the cells lifts take up.
    pub fn refresh_lifts(&mut self) {
        let defs = self.defs.clone();
        for i in 0..self.world.lifts.len() {
            let l = &self.world.lifts[i];
            let d = &defs.lifts[l.def];
            let (lp, up, under) = cable_loop(l, d);
            let rot = Quat::from_rotation_z((l.end - l.start).truncate().to_angle());
            // Ground points stand on the station's floor (its origin), or
            // the snow where that is higher.
            let ground = |w: &World, origin: Vec3, p: Vec3| -> Vec3 {
                let q = origin + rot * p;
                let g = w.ground(q.truncate());
                Vec3::new(g.x, g.y, g.z.max(q.z))
            };
            let w = &self.world;
            let queue_lines: Vec<Vec<Vec3>> = [AttachPoint::Path as fn(u8) -> AttachPoint, AttachPoint::PathB]
                .iter()
                .map(|k| ground_path(&d.begin_pts, *k).into_iter().map(|p| ground(w, l.start, p)).collect::<Vec<_>>())
                .filter(|v: &Vec<Vec3>| !v.is_empty())
                .collect();
            let exit_line: Vec<Vec3> = ground_path(&d.end_pts, AttachPoint::Path).into_iter().map(|p| ground(w, l.end, p)).collect();
            let l = &mut self.world.lifts[i];
            l.cable_loop = lp;
            l.cable_up = up;
            l.under = under;
            l.queue_lines = queue_lines;
            l.exit_line = exit_line;
            // Riders get on where the loop passes nearest the boarding
            // point and off nearest the top's first ground point.
            l.board_s = l.queue_lines.first().and_then(|q| q.first()).and_then(|p| l.nearest_s(*p, true));
            l.unload_s = l.exit_line.first().and_then(|p| l.nearest_s(*p, false));
        }
        self.rebuild_lift_cells();
    }

    pub(super) fn rebuild_lift_cells(&mut self) {
        let lv = self.world.level();
        let (n, s) = (lv.cells_per_side() as i32, lv.spacing);
        let mut cells = vec![None; (n * n) as usize];
        let mut mark = |c: (i32, i32), r: i32, id: Id| {
            for j in -r..=r {
                for i in -r..=r {
                    let (x, y) = (c.0 + i, c.1 + j);
                    if i.abs() + j.abs() <= r && x >= 0 && y >= 0 && x < n && y < n {
                        cells[(y * n + x) as usize] = Some(id);
                    }
                }
            }
        };
        let cell = |v: Vec2| ((v.x / s).floor() as i32, (v.y / s).floor() as i32);
        for l in &self.world.lifts {
            let (a, b) = (l.start.truncate(), l.end.truncate());
            mark(cell(a), 4, l.id);
            mark(cell(b), 3, l.id);
            let steps = (a.distance(b) / (s * 0.5)).ceil().max(1.0) as usize;
            for k in 0..=steps {
                mark(cell(a.lerp(b, k as f32 / steps as f32)), 3, l.id);
            }
        }
        self.world.lift_cells = cells;
    }
}

/// A station or tower's cable points in letter order.
fn cable_chain(pts: &[(AttachPoint, Vec3)]) -> Vec<Vec3> {
    // Up to the first missing letter, as the original counts them
    // (0x49f4a0).
    (0..26).map_while(|c| attach_point(pts, AttachPoint::Cable(c))).collect()
}

/// A ground path's points (`kind(0)`, `kind(1)`, ...) in order.
fn ground_path(pts: &[(AttachPoint, Vec3)], kind: fn(u8) -> AttachPoint) -> Vec<Vec3> {
    (0..10).map_while(|i| attach_point(pts, kind(i))).collect()
}

/// The cable loop of a lift in world space, from the station meshes' cable
/// points, as the original strings it (0x49f4a0): from the bottom station's
/// last point up through the first half of each tower's points, round the
/// top station's points, back down through the rest of each tower's
/// points, and round the bottom station's other points.
/// A rope tow (no hangers or chairs: riders hold the cable itself) is
/// strung so its two strands never cross: straight up from the bottom
/// station's low wheel to the top station's low wheel, clear of the
/// towers, round the top station the other way, and back down under each
/// tower's wheel where the tower stands below the straight line between
/// its neighbors' wheels (or the stations' wheels at the ends), so the
/// wheel holds the cable down, or over it where the tower stands above
/// or in line, so the wheel holds it up.
/// Returns the loop, the length of its upward part, and which towers the
/// cable runs under.
fn cable_loop(l: &crate::game::world::Lift, d: &defs::LiftDef) -> (Vec<Vec3>, f32, Vec<bool>) {
    let dir = (l.end - l.start).truncate();
    let rot = Quat::from_rotation_z(dir.y.atan2(dir.x));
    let at = |origin: Vec3, p: Vec3| origin + rot * p;
    let start = cable_chain(&d.begin_pts);
    let mut end = cable_chain(&d.end_pts);
    let tower = cable_chain(&d.middle_pts);
    let Some(&leave) = start.last() else { return (Vec::new(), 0.0, Vec::new()) };
    let rope = d.chairs.is_empty();
    let (up, down) = if rope {
        end.reverse();
        (&[][..], &[][..])
    } else {
        tower.split_at(tower.len() / 2)
    };
    // A rope tow's wheel: its top and bottom points.
    let wheel = (
        tower.iter().copied().max_by(|a, b| a.z.total_cmp(&b.z)),
        tower.iter().copied().min_by(|a, b| a.z.total_cmp(&b.z)),
    );
    let mut pts = vec![at(l.start, leave)];
    for pole in &l.poles {
        pts.extend(up.iter().map(|p| at(*pole, *p)));
    }
    let up_len = |pts: &[Vec3], next: Vec3| -> f32 {
        pts.windows(2).map(|w| (w[1] - w[0]).length()).sum::<f32>() + pts.last().map_or(0.0, |p| (next - *p).length())
    };
    let arrive = end.first().map_or(l.end, |p| at(l.end, *p));
    let up_length = up_len(&pts, arrive);
    pts.extend(end.iter().map(|p| at(l.end, *p)));
    // The way down's supports from the top station: its wheel, each
    // tower's wheel center, and the bottom station's wheel.
    let center = |pole: Vec3| wheel.0.zip(wheel.1).map_or(pole, |(t, b)| at(pole, (t + b) * 0.5));
    let mut supports = vec![end.last().map_or(l.end, |p| at(l.end, *p))];
    supports.extend(l.poles.iter().rev().map(|p| center(*p)));
    supports.push(at(l.start, start[0]));
    let mut under = vec![false; l.poles.len()];
    for (i, pole) in l.poles.iter().rev().enumerate() {
        pts.extend(down.iter().map(|p| at(*pole, *p)));
        if let (true, Some(top), Some(bottom)) = (rope, wheel.0, wheel.1) {
            let (prev, here, next) = (supports[i], supports[i + 1], supports[i + 2]);
            let span = (next - prev).truncate();
            let t = ((here - prev).truncate().dot(span) / span.length_squared().max(1e-6)).clamp(0.0, 1.0);
            // In line counts as above: the cable rests on the wheel.
            let below = here.z < prev.z + (next.z - prev.z) * t - 0.01;
            pts.push(at(*pole, if below { bottom } else { top }));
            under[l.poles.len() - 1 - i] = below;
        }
    }
    pts.extend(start[..start.len() - 1].iter().map(|p| at(l.start, *p)));
    if pts.len() < 2 {
        return (Vec::new(), 0.0, Vec::new());
    }
    (pts, up_length, under)
}
