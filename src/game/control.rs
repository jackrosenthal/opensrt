//! Riding along with someone (the original's follow view, 0x433430 and
//! 0x432aa0), steering a skier (0x4971b0, 0x498910, 0x498ec0), and
//! picking people up and putting them down.

use std::collections::HashMap;

use super::*;

/// The skiing states of a steered skier (+0x104): pushing (walking up a
/// rise on skis), gliding, snowplowing, and down after a crash.
const PUSH: usize = 0;
const GLIDE: usize = 1;
const PLOW: usize = 2;
const CRASHED: usize = 3;

/// Each state's numbers (0x69f4e0): push, drag on the ground (times the
/// velocity), drag in the air, top push speed, -, turn acceleration, top
/// turn rate, and how fast forward and sideways speed bleed away on the
/// ground.
const PARAMS: [[f32; 9]; 4] = [
    [40.0, 0.0, 0.0, 5.0, 1.0, 8.0, 1.5, 5.0, 5.0],
    [5.0, -0.2, -0.4, 8.0, 3.0, 8.0, 2.5, 0.1, 5.0],
    [0.0, -1.5, -1.5, 4.0, 2.0, 2.0, 2.0, 2.0, 4.0],
    [0.0, -4.0, -1.0, 5.0, 0.0, 0.0, 0.0, 3.0, 3.0],
];
const GRAVITY: f32 = -12.0;
/// Above this speed the skier turns to face the way they go and stand
/// square to the snow, this fast (radians a second), less in the air,
/// where the ground is no steeper than these (0x4984f0).
const FACE_SPEED: f32 = 0.3;
const FACE_RATE: f32 = 2.0;
const FACE_AIR: f32 = 0.4;
const FACE_MAX_TILT: f32 = 1.047;
const FACE_FLAT: f32 = 0.707;

/// The skier clips by the original's numbers (0x656820).
const CLIPS: [&str; 61] = [
    "GlideSlow", "GlideMedium", "GlideFast", "DoublePole", "SkiWalk", "SnowPlow", "Crashed", "InLineIdle",
    "InLineIdle2", "InLineStepForward", "Flail", "FlailNoSkis", "WalkNoSkis", "WalkCarrySkis", "flail", "flail",
    "flail", "ADV_Mogul", "SP_MogulRight", "SP_MogulLeft", "SP_MogulBack", "SP_MogulForward", "FacePlant",
    "FacePlantStill", "Death", "S_Turns", "Crash2", "Crash2Still", "Crash3", "Crash3Still", "Daffy", "SpreadEagle",
    "BackScratcher", "ThreeSixty", "flail", "SP_WobbleRight", "SP_WobbleLeft", "SP_WobbleBack", "SP_WobbleForward",
    "SP_Glide", "SP_LeanInRight", "SP_LeanRight", "SP_LeanOutRight", "SP_LeanInLeft", "SP_LeanLeft",
    "SP_LeanOutLeft", "ADV_LeanInLeft", "ADV_LeanLeft", "ADV_LeanOutLeft", "ADV_LeanInRight", "ADV_LeanRight",
    "ADV_LeanOutRight", "Land", "YardSale", "YardSaleStill", "IdleCarrySkis", "IdleNoSkis", "Ride", "Yell",
    "GlideFast", "S_Turn_hold",
];
const POLE: u8 = 3;
const SKATE: u8 = 4;
const SNOWPLOW: u8 = 5;
const LEAN_IN_LEFT: u8 = 0x2e;
const LEAN_LEFT: u8 = 0x2f;
const LEAN_OUT_LEFT: u8 = 0x30;
const LEAN_IN_RIGHT: u8 = 0x31;
const LEAN_RIGHT: u8 = 0x32;
const LEAN_OUT_RIGHT: u8 = 0x33;
const LAND: u8 = 0x34;
const AIR: u8 = 0x3b;
const TURN_HOLD: u8 = 0x3c;
/// The tricks Ctrl does in the air: Daffy, Spread Eagle, Back Scratcher
/// and 360. Landing in one is a crash.
const TRICKS: std::ops::RangeInclusive<u8> = 0x1e..=0x21;
/// The falls, the last only above 10 a second.
const FALLS: [u8; 4] = [0x16, 0x1a, 0x1c, 0x35];

/// The follow view: the camera rides with someone, or looks out from a
/// building, with nothing on screen but "Press space".
#[derive(Debug, Clone)]
pub struct Eye {
    pub of: Selection,
    /// How far the fog lets one see: 100, 200, 350 or 50000 (the fog
    /// dialog, 0x44af30).
    pub sight: f32,
    /// The eased camera position (0x69e828).
    pos: Vec3,
    /// Where the camera looks, while nobody is steered.
    look: Vec3,
    /// The overview to go back to.
    overview: crate::camera::Camera,
}

/// A steered skier (+0x15c), and the state of their skiing.
#[derive(Debug, Clone)]
pub struct Control {
    pub guest: Id,
    /// Made by the first-person button (+0x13e): Space takes them away.
    pub own: bool,
    vel: Vec3,
    /// The body's whole orientation (+0x160): +x ahead, +z up.
    rot: Quat,
    /// Turn rate (+0x19c), radians a second, positive to the right.
    yaw: f32,
    mode: usize,
    /// Seconds in the state (+0x1bc).
    timer: f32,
    airborne: bool,
    /// Seconds since a turn ended (+0x1cc).
    glide: f32,
    clip: u8,
    clip_t: f32,
    /// Each clip's length and whether it loops, from the body's mesh.
    clips: Arc<HashMap<String, (f32, bool)>>,
}

/// Octagonal speed, as the original measures it.
fn oct(v: Vec3) -> f32 {
    let a = v.abs();
    let big = a.max_element();
    big + (a.x + a.y + a.z - big) * 0.25
}

impl Game {
    /// Which of the original cursors fits the current tool and hover.
    pub fn cursor_kind(&self, input: &Input) -> crate::cursor::CursorKind {
        use crate::cursor::CursorKind as C;
        if input.middle_down || (input.right_down && input.held(KeyCode::ShiftLeft)) {
            return C::Rotate;
        }
        if input.right_down {
            return C::Pan;
        }
        if self.carrying.is_some() {
            return C::Hand;
        }
        if self.hover.is_none() {
            return C::Arrow;
        }
        match self.placement.as_ref().and_then(|p| p.reason) {
            Some(Problem::TooExpensive) => return C::NoCash,
            Some(Problem::TooLong) => return C::TooLong,
            _ => {}
        }
        // The original resets to the arrow every frame and only switches
        // for terrain editing, clearing and refused placements.
        if let (Tool::Trees { density: 0 }, Some(r)) = (&self.tool, self.tree_rect()) {
            if self.chop_cost(r) > self.world.cash && !self.world.sandbox {
                return C::NoCash;
            }
        }
        match &self.tool {
            Tool::Terrain(_) | Tool::Survey { .. } => C::Survey,
            Tool::Trees { density: 0 } | Tool::RunPath { .. } | Tool::Bulldoze => C::Saw,
            _ => C::Arrow,
        }
    }

    /// The first-person button (0x461fc7): a skier of the player's own,
    /// with 4800 to spend, picked up to be put down on the mountain.
    pub fn spawn_own_skier(&mut self) {
        if self.carrying.is_some() || self.eye.is_some() {
            return;
        }
        let at = Vec2::new(2.0 * self.world.level().spacing, 2.0 * self.world.level().spacing);
        let id = self.sim().spawn_own(at);
        self.carrying = Some(Selection::Guest(id));
        self.fog_request = None;
        self.sfx(Sound::PickupSkier);
    }

    /// Where someone or something is seen from, and which way it faces.
    fn eye_subject(&self, of: &Selection) -> Option<(Vec3, Vec3, bool)> {
        let dir = |h: f32| Vec3::new(h.cos(), h.sin(), 0.0);
        match *of {
            Selection::Guest(id) => {
                let g = self.world.guest(id).filter(|g| g.pos.z > -100.0 && g.is_active())?;
                let f = match &self.control {
                    Some(c) if c.guest == id => c.rot * Vec3::X,
                    _ => dir(g.heading),
                };
                Some((g.pos, f, true))
            }
            Selection::Staff(id) => {
                let s = self.world.staff.iter().find(|s| s.id == id)?;
                Some((s.pos, dir(s.heading), true))
            }
            Selection::Structure(id) => {
                let s = self.world.structure(id)?;
                Some((s.pos, dir(s.rot as f32 * std::f32::consts::FRAC_PI_2), false))
            }
            Selection::Lift(id) => {
                let l = self.world.lifts.iter().find(|l| l.id == id)?;
                Some((l.start, (l.end - l.start).normalize_or(Vec3::X), false))
            }
            Selection::Park(id) => {
                let p = self.world.parks.iter().find(|p| p.id == id)?;
                Some((p.start, (p.end - p.start).normalize_or(Vec3::X), false))
            }
            Selection::Run(_) => None,
        }
    }

    /// Into the follow view (0x433430), once a fog setting is chosen: the
    /// game is saved as AutoSave first, and the camera put four units
    /// behind and six up, looking ahead; a building is seen from 24 up.
    pub fn enter_eye(&mut self, of: Selection, sight: f32) {
        let Some((p, f, person)) = self.eye_subject(&of) else { return };
        let _ = self.save(&crate::ui::saves_dir().join("AutoSave.json"));
        let (pos, look) = if person {
            (p - f * 4.0 + Vec3::Z * 6.0, p + f + Vec3::Z * 6.0)
        } else {
            (p + Vec3::Z * 24.0, p + f + Vec3::Z * 24.0)
        };
        self.eye = Some(Eye {
            of,
            sight,
            pos,
            look: look - pos,
            overview: self.camera.clone(),
        });
        self.selection = None;
        self.tool = Tool::Inspect;
        self.camera.ortho = false;
        // The original's perspective camera sees 90 degrees (0x4ceb80).
        self.camera.fov_y = std::f32::consts::FRAC_PI_2;
        self.camera.look_from(pos, look);
    }

    /// Space (0x432aa0): back to the overview. A steered guest goes back
    /// to their own devices, and the player's own skier is gone.
    pub fn leave_eye(&mut self) {
        if let Some(c) = self.control.take() {
            if c.own {
                self.world.guests.retain(|g| g.id != c.guest);
            } else {
                self.sim().release(c.guest);
            }
        }
        if let Some(e) = self.eye.take() {
            self.camera = e.overview;
        }
        self.stop_ski_sound();
    }

    /// Starts steering a guest (state 0x1a, 0x496a9b): still, gliding,
    /// out of their group and lodging.
    fn take_control(&mut self, id: Id, own: bool) {
        let Some(g) = self.world.guests.iter_mut().find(|g| g.id == id) else { return };
        if g.pos.z < -100.0 {
            return;
        }
        // Lift riders get off at the top first.
        if let GuestState::Riding { lift } | GuestState::Queued { lift } = g.state {
            if let Some(l) = self.world.lifts.iter_mut().find(|l| l.id == lift) {
                l.queue.retain(|q| *q != id);
                for c in &mut l.chairs {
                    c.riders.retain(|r| *r != id);
                }
                let top = l.exit_point();
                let z = self.world.level.as_ref().map(|lv| lv.height_at(top.x, top.y)).unwrap_or(0.0);
                g.pos = Vec3::new(top.x, top.y, z);
            }
        }
        g.state = GuestState::Controlled;
        g.speed = 0.0;
        let g = g.clone();
        let clips = self.clip_lengths(&g);
        let g = self.world.guests.iter_mut().find(|g| g.id == id).unwrap();
        g.ctl_clip = Some(CLIPS[SNOWPLOW as usize]);
        g.anim = 0.0;
        self.control = Some(Control {
            guest: id,
            own,
            vel: Vec3::ZERO,
            rot: Quat::from_rotation_z(g.heading),
            yaw: 0.0,
            mode: GLIDE,
            timer: 0.0,
            airborne: false,
            glide: 0.0,
            clip: SNOWPLOW,
            clip_t: 0.0,
            clips,
        });
    }

    /// The lengths of the body's clips, by the names drawn.
    fn clip_lengths(&self, g: &Guest) -> Arc<HashMap<String, (f32, bool)>> {
        let mut out = HashMap::new();
        let mesh = self.assets.resolve(&guest_mesh(g)).and_then(|p| crate::formats::mesh::Mesh::load(p).ok());
        if let Some(m) = mesh {
            for c in &m.clips {
                out.insert(c.name.to_ascii_lowercase(), (c.frames.max(1) as f32 / c.fps.max(1.0), c.looping));
            }
        }
        Arc::new(out)
    }

    /// The follow view, each frame (0x432aa0): Space leaves, C takes or
    /// gives back control of the guest followed, and the camera eases to
    /// four behind and three up, looking at a steered skier.
    pub(super) fn eye_update(&mut self, dt: f32, input: &Input, keys: bool) {
        let Some(mut e) = self.eye.take() else { return };
        if keys && input.pressed.contains(&KeyCode::Space) {
            self.eye = Some(e);
            self.leave_eye();
            return;
        }
        // The original acts as C comes back up (0x69e878); a tap can come
        // and go between frames here, so it acts on the press.
        let c_tap = keys && input.pressed.contains(&KeyCode::KeyC);
        if let Selection::Guest(id) = e.of {
            let own = self.control.as_ref().is_some_and(|c| c.own);
            if c_tap && !own {
                match self.control.take() {
                    Some(c) => {
                        self.sim().release(c.guest);
                        self.stop_ski_sound();
                    }
                    None => self.take_control(id, false),
                }
            }
        }
        if self.control.is_some() {
            self.drive(dt, input, keys);
        }
        let Some((p, f, person)) = self.eye_subject(&e.of) else {
            self.eye = Some(e);
            self.leave_eye();
            return;
        };
        if person {
            let want = p - f * 4.0 + Vec3::Z * 3.0;
            e.pos += (want - e.pos) * (1.0 - 0.9f32.powf(dt * 30.0));
        }
        let steered = matches!((&e.of, &self.control), (Selection::Guest(id), Some(c)) if c.guest == *id);
        let look = if steered { p + Vec3::Z * 2.0 } else { e.pos + e.look };
        self.camera.look_from(e.pos, look);
        self.eye = Some(e);
    }

    fn set_clip(&mut self, clip: u8, restart: bool, keep_time: bool) {
        let Some(c) = self.control.as_mut() else { return };
        if restart || c.clip != clip {
            c.clip = clip;
            if !keep_time {
                c.clip_t = 0.0;
            }
        }
    }

    /// The clip has played through (0x48be80).
    fn clip_done(&self) -> bool {
        let Some(c) = &self.control else { return true };
        let name = CLIPS[c.clip as usize].to_ascii_lowercase();
        c.clips.get(&name).is_none_or(|&(len, looping)| !looping && c.clip_t >= len)
    }

    pub(super) fn stop_ski_sound(&mut self) {
        if let Some((_, h)) = self.ski_loop.take() {
            crate::audio::stop(h);
        }
    }

    /// A state's sound: walking in snow while pushing, the skiing loop
    /// otherwise, louder with speed and scraping louder still when
    /// snowplowing or down; silent in the air (0x497c14).
    fn ski_sound(&mut self, mode: usize, vel: Vec3, airborne: bool) {
        let sound = if mode == PUSH { Sound::WalkInSnowLoop } else { Sound::SkiingLoop };
        if self.ski_loop.is_some_and(|(s, _)| s != sound) {
            self.stop_ski_sound();
        }
        let a = vel.abs();
        let big = a.max_element();
        let rest = a.x + a.y + a.z - big;
        let f = match mode {
            PUSH => 1.0,
            GLIDE => big * 0.05 + rest * 0.0125,
            _ => big * 0.2 + rest * 0.05,
        }
        .min(1.0);
        let volume = if airborne || f <= 0.0 { -10000 } else { (f * 3000.0 - 3000.0) as i32 };
        match self.ski_loop {
            Some((_, h)) => crate::audio::set_volume(h, volume),
            None => {
                self.ski_loop = crate::audio::play_sound(&self.assets, sound, crate::audio::Opts { music: false, looped: true, volume })
                    .map(|h| (sound, h));
            }
        }
    }

    /// One frame of steering (0x4971b0, 0x498910, 0x498ec0). The arrow
    /// keys turn, push and snowplow; Ctrl does tricks in the air.
    pub(super) fn drive(&mut self, dt: f32, input: &Input, keys: bool) {
        let mut left = dt;
        while left > 0.0 {
            let step = left.min(1.0 / 30.0);
            left -= step;
            self.drive_step(step, input, keys);
        }
    }

    fn drive_step(&mut self, dt: f32, input: &Input, keys: bool) {
        let Some(mut c) = self.control.clone() else { return };
        let Some(gi) = self.world.guests.iter().position(|g| g.id == c.guest) else {
            self.control = None;
            return;
        };
        let k = |code: KeyCode| keys && input.held(code);
        let turn = k(KeyCode::ArrowRight) as i32 as f32 - k(KeyCode::ArrowLeft) as i32 as f32;
        let push = k(KeyCode::ArrowUp) as i32 as f32 - k(KeyCode::ArrowDown) as i32 as f32;
        let trick = k(KeyCode::ControlLeft) || k(KeyCode::ControlRight);
        let pos = self.world.guests[gi].pos;
        // The ground under where the skier was, and a tree in their cell.
        let (n, ground, tree, ext) = {
            let level = self.world.level();
            let s = level.spacing;
            let n_cells = level.cells_per_side();
            let (cx, cy) = ((pos.x / s).max(0.0) as usize, (pos.y / s).max(0.0) as usize);
            let tree = (cx < n_cells && cy < n_cells && level.cell(cx, cy).foliage != 0)
                .then(|| Vec2::new((cx as f32 + 0.5) * s, (cy as f32 + 0.5) * s));
            (level.normal_at(pos.x, pos.y), Vec3::new(pos.x, pos.y, level.height_at(pos.x, pos.y)), tree, level.extent())
        };
        // Ahead along the snow; the body's own forward where that is far
        // from it (0x4971b0).
        let body = c.rot * Vec3::X;
        let fwd = {
            let along = (c.rot * Vec3::Y).cross(n).normalize_or(body);
            let along = if along.dot(body) < 0.0 { -along } else { along };
            if along.dot(body).abs() < 0.9 { body } else { along }
        };
        c.timer += dt;
        c.clip_t += dt;

        // The state (0x4971b0).
        let m = oct(c.vel);
        let mut accel = !c.airborne;
        let to = |c: &mut Control, mode: usize| {
            c.mode = mode;
            c.timer = 0.0;
        };
        match c.mode {
            PUSH => {
                if push > 0.0 || c.airborne {
                    if !(m <= PARAMS[PUSH][3] && fwd.z >= 0.1 && !c.airborne) {
                        to(&mut c, GLIDE);
                    }
                } else {
                    to(&mut c, PLOW);
                }
            }
            GLIDE => {
                if push < 0.0 && !c.airborne {
                    to(&mut c, PLOW);
                } else if push > 0.0 && !c.airborne && m < PARAMS[PUSH][3] && fwd.z > 0.2 {
                    to(&mut c, PUSH);
                } else if let Some(t) = tree {
                    // Into a tree.
                    if m > 5.0 && t.distance(pos.truncate()) < 2.0 {
                        self.crash_sound();
                        to(&mut c, CRASHED);
                    }
                }
            }
            PLOW => {
                if push >= 0.0 || c.airborne {
                    c.mode = GLIDE;
                }
            }
            _ => {
                if c.timer > 3.0 {
                    to(&mut c, GLIDE);
                }
            }
        }
        let p = PARAMS[c.mode];
        if c.airborne {
            accel = false;
        }
        if accel {
            // Pushing along the skis, never backwards, and only up to the
            // state's top speed.
            let along = fwd.dot(c.vel);
            let mut amount = (p[0] * push * dt).max(-along);
            if amount > 0.0 && p[3] < amount + along {
                amount = (p[3] - along).max(0.0);
            }
            c.vel += fwd * amount;
        }
        if turn == 0.0 {
            c.yaw = if c.yaw > 0.0 { (c.yaw - p[5] * dt).max(0.0) } else { (c.yaw + p[5] * dt).min(0.0) };
        } else {
            c.yaw += p[5] * turn * dt;
        }
        c.yaw = c.yaw.clamp(-p[6], p[6]);
        if !c.airborne {
            // The skis and the way they go turn together, about the body's
            // up.
            let q = Quat::from_axis_angle((c.rot * Vec3::Z).normalize_or(Vec3::Z), -c.yaw * dt);
            c.rot = (q * c.rot).normalize();
            c.vel = q * c.vel;
        }

        // Moving (0x498910).
        c.vel.z += GRAVITY * dt;
        if !c.airborne {
            let drag = p[1] * if tree.is_some() { 3.0 } else { 1.0 } * dt;
            c.vel += c.vel * drag;
        } else {
            c.vel += c.vel * p[2] * dt;
        }
        let next = pos + c.vel * dt;
        if n.dot(next - ground) <= 0.0 {
            // On the snow: speed bleeds off along and across the skis, and
            // what goes into the slope turns along it.
            let fwd = (fwd - n * fwd.dot(n)).normalize_or(fwd);
            let side = n.cross(fwd);
            for (axis, grip) in [(fwd, p[7]), (side, p[8])] {
                let v = axis.dot(c.vel);
                let after = if v > 0.0 { (v - grip * dt).max(0.0) } else { (v + grip * dt).min(0.0) };
                c.vel += axis * (after - v);
            }
            let speed = c.vel.length();
            c.vel += n * c.vel.dot(n).abs();
            c.vel = c.vel.normalize_or_zero() * speed;
        }
        let mut np = pos + c.vel * dt;
        let above = n.dot(np - ground);
        if above < 0.0 {
            np -= n * above;
        }
        if above >= 1.0 {
            if !c.airborne {
                self.jump_sound();
            }
            c.airborne = true;
        } else {
            if c.airborne && TRICKS.contains(&c.clip) {
                // Landed mid-trick.
                self.crash_sound();
                c.mode = CRASHED;
                c.timer = 0.0;
            }
            c.airborne = false;
        }
        // Facing the way they go, square to the snow (0x4984f0): up
        // toward the ground's normal (in the air, as it is), ahead along
        // the velocity, turned toward that by a step at a time, and not
        // at all once closer than a step.
        if c.vel.length() > FACE_SPEED {
            let body_up = c.rot * Vec3::Z;
            let up = if c.airborne { body_up } else { n };
            if body_up.angle_between(up) < FACE_MAX_TILT || up.z > FACE_FLAT {
                let side = up.cross(c.vel).normalize_or_zero();
                if side != Vec3::ZERO {
                    let ahead = side.cross(up);
                    let want = Quat::from_mat3(&glam::Mat3::from_cols(ahead, up.cross(ahead), up)).normalize();
                    let angle = c.rot.angle_between(want);
                    let step = FACE_RATE * if c.airborne { FACE_AIR } else { 1.0 } * dt;
                    if step <= angle {
                        c.rot = c.rot.slerp(want, (step / angle).clamp(0.0, 1.0)).normalize();
                    }
                }
            }
        }
        let ahead = c.rot * Vec3::X;
        let heading = ahead.y.atan2(ahead.x);
        // Off the map: back to the corner (0x496e49).
        if np.x < 1.0 || np.y < 1.0 || np.x > ext - 1.0 || np.y > ext - 1.0 {
            let corner = Vec2::splat(20.0);
            np = self.world.ground(corner);
            c.vel = Vec3::ZERO;
        }
        {
            let g = &mut self.world.guests[gi];
            g.pos = np;
            g.heading = heading;
            g.tilt = Some(c.rot);
            g.speed = c.vel.length();
        }
        self.control = Some(c.clone());
        self.ski_sound(c.mode, c.vel, c.airborne);
        self.pick_clip(dt, trick, push);
        if let Some(c) = &self.control {
            let g = &mut self.world.guests[gi];
            g.ctl_clip = Some(CLIPS[c.clip as usize]);
            g.anim = c.clip_t;
        }
    }

    /// The steered skier's animation (0x498ec0).
    fn pick_clip(&mut self, dt: f32, trick: bool, push: f32) {
        let Some(c) = self.control.clone() else { return };
        let turn_sound = |g: &Game, volume: i32| {
            crate::audio::play_sound(&g.assets, Sound::Turn, crate::audio::Opts { music: false, looped: false, volume });
        };
        match c.mode {
            PUSH => self.set_clip(SKATE, false, false),
            PLOW => self.set_clip(SNOWPLOW, false, false),
            CRASHED => {
                if !FALLS.contains(&c.clip) {
                    let pick = if oct(c.vel) <= 10.0 { (self.world.rand() * 3.0) as usize } else { (self.world.rand() * 4.0) as usize };
                    self.set_clip(FALLS[pick.min(3)], true, false);
                }
            }
            _ if c.airborne => {
                if TRICKS.contains(&c.clip) {
                    if self.clip_done() {
                        self.set_clip(AIR, false, false);
                    }
                } else if c.clip == AIR {
                    if trick {
                        let t = 0x1e + (self.world.rand() * 4.0) as u8;
                        self.set_clip(t.min(0x21), false, false);
                    }
                } else {
                    self.set_clip(AIR, false, false);
                }
            }
            _ => {
                if let Some(cc) = self.control.as_mut() {
                    cc.glide += dt;
                }
                let m = oct(c.vel);
                match c.clip {
                    0 | 1 | 2 | TURN_HOLD => {
                        // Up held while slow: a pole push, when climbing
                        // or now and then (0x498fd6).
                        let poling = push > 0.0 && ((m < 10.0 && (c.vel.z > 0.0 || self.world.rand() < 0.022)) || m < 6.0);
                        if poling {
                            self.set_clip(POLE, false, false);
                            turn_sound(self, -1500);
                        } else if c.yaw > 0.0 {
                            self.set_clip(LEAN_IN_RIGHT, false, false);
                        } else if c.yaw < 0.0 {
                            self.set_clip(LEAN_IN_LEFT, false, false);
                        } else if c.glide > 1.0 {
                            let clip = if m >= 18.0 {
                                2
                            } else if m >= 12.0 {
                                1
                            } else {
                                0
                            };
                            self.set_clip(clip, false, false);
                        }
                    }
                    POLE | LAND => {
                        if self.clip_done() {
                            self.set_clip(1, false, false);
                        }
                    }
                    LEAN_IN_LEFT => {
                        if c.yaw >= 0.0 {
                            self.set_clip(LEAN_OUT_LEFT, true, true);
                        } else if self.clip_done() {
                            self.set_clip(LEAN_LEFT, false, false);
                            turn_sound(self, 0);
                        }
                    }
                    LEAN_LEFT => {
                        if c.yaw >= 0.0 {
                            self.set_clip(LEAN_OUT_LEFT, false, false);
                        }
                    }
                    LEAN_OUT_LEFT => {
                        if c.yaw < 0.0 {
                            self.set_clip(LEAN_IN_LEFT, true, true);
                        } else if self.clip_done() {
                            self.set_clip(TURN_HOLD, false, false);
                            if let Some(cc) = self.control.as_mut() {
                                cc.glide = 0.0;
                            }
                        }
                    }
                    LEAN_IN_RIGHT => {
                        if c.yaw <= 0.0 {
                            self.set_clip(LEAN_OUT_RIGHT, true, true);
                        } else if self.clip_done() {
                            self.set_clip(LEAN_RIGHT, false, false);
                            turn_sound(self, 0);
                        }
                    }
                    LEAN_RIGHT => {
                        if c.yaw <= 0.0 {
                            self.set_clip(LEAN_OUT_RIGHT, false, false);
                        }
                    }
                    LEAN_OUT_RIGHT => {
                        if c.yaw > 0.0 {
                            self.set_clip(LEAN_IN_RIGHT, true, true);
                        } else if self.clip_done() {
                            self.set_clip(TURN_HOLD, false, false);
                            if let Some(cc) = self.control.as_mut() {
                                cc.glide = 0.0;
                            }
                        }
                    }
                    AIR => self.set_clip(LAND, false, false),
                    _ => self.set_clip(1, false, false),
                }
            }
        }
    }

    /// Picks a guest up (the guest window's Hand of God, 0x452f48): they
    /// leave whatever they were doing and hang from the pointer.
    pub fn pick_up(&mut self, id: Id) {
        self.drop_carried();
        if let Some(s) = self.world.staff.iter_mut().find(|s| s.id == id) {
            // Employees too, from their window (0x46bd20).
            s.task = StaffTask::Held;
            self.carrying = Some(Selection::Staff(id));
            self.sfx(Sound::PickupSkier);
            return;
        }
        let Some(g) = self.world.guests.iter_mut().find(|g| g.id == id) else { return };
        if g.pos.z < -100.0 {
            self.error(Problem::Indoors);
            return;
        }
        if let GuestState::Riding { lift } | GuestState::Queued { lift } = g.state {
            if let Some(l) = self.world.lifts.iter_mut().find(|l| l.id == lift) {
                l.queue.retain(|q| *q != id);
                for c in &mut l.chairs {
                    c.riders.retain(|r| *r != id);
                }
            }
        }
        g.state = GuestState::Held;
        g.speed = 0.0;
        g.anim = 0.0;
        self.carrying = Some(Selection::Guest(id));
        self.sfx(Sound::PickupSkier);
    }

    /// Puts the carried guest down where they hang (0x48a2aa); they then
    /// decide afresh what to do. The player's own skier is taken in hand
    /// instead, and the fog dialog asked for (0x432f10).
    pub(super) fn drop_carried(&mut self) {
        let Some(sel) = self.carrying.take() else { return };
        if let Selection::Staff(id) = sel {
            let Some(s) = self.world.staff.iter().find(|s| s.id == id) else { return };
            let p = s.pos.truncate();
            let at = self.world.ground(p);
            if let Some(s) = self.world.staff.iter_mut().find(|s| s.id == id) {
                s.pos = at;
                s.task = StaffTask::Idle;
            }
            self.sfx(Sound::PlacingSkier);
            return;
        }
        let Selection::Guest(id) = sel else { return };
        let Some(g) = self.world.guests.iter_mut().find(|g| g.id == id) else { return };
        let p = g.pos.truncate();
        let ground = self.world.level.as_ref().map(|lv| lv.height_at(p.x, p.y)).unwrap_or(g.pos.z);
        let g = self.world.guests.iter_mut().find(|g| g.id == id).unwrap();
        g.pos = Vec3::new(p.x, p.y, ground);
        g.state = GuestState::Walking { to: p, purpose: Purpose::Wander };
        let own = g.own;
        self.sfx(Sound::PlacingSkier);
        if own {
            self.take_control(id, true);
            self.fog_request = Some(Selection::Guest(id));
        }
    }

    /// One of the crash sounds at random (`crash*.wav`).
    fn crash_sound(&mut self) {
        let crash = if self.world.rand() < 0.5 { Sound::Crash1 } else { Sound::Crash2 };
        self.sfx(crash);
    }

    /// Puts whoever is carried down at `at` (for test hooks).
    pub fn put_down_at(&mut self, at: Vec3) {
        if let Some(Selection::Guest(id)) = self.carrying.clone() {
            if let Some(g) = self.world.guests.iter_mut().find(|g| g.id == id) {
                g.pos = at;
            }
        }
        self.drop_carried();
    }

    /// The jump sound, at -15 dB (0x498e51).
    pub(super) fn jump_sound(&self) {
        crate::audio::play_sound(&self.assets, Sound::Jump, crate::audio::Opts { music: false, looped: false, volume: -1500 });
    }
}
