//! Orbiting strategy-game camera over a z-up world.

use glam::{Mat4, Vec2, Vec3, Vec4};

/// The overview's elevation. The original's camera (0x4339d0) sits 800
/// units out and sin(0.5) × 800 up, so atan(sin 0.5) ≈ 25.6° above the
/// horizon.
pub const OVERVIEW_PITCH: f32 = 0.447_5;
/// The world units across the screen at the original's three zoom steps
/// (0x432518), closest first. It opens at the middle one and steps
/// between them, with no zoom in between (0x433be0, 0x433c50).
pub const ZOOM_WIDTHS: [f32; 3] = [90.0, 300.0, 1100.0];
pub const OPENING_STEP: usize = 1;
const ORTHO_ASPECT: f32 = 4.0 / 3.0;

#[derive(Debug, Clone)]
pub struct Camera {
    pub target: Vec3,
    /// Rotation around z, radians.
    pub yaw: f32,
    /// Elevation angle above the horizon, radians.
    pub pitch: f32,
    /// How far off the eye is; in the orthographic overview, the view is
    /// as wide as a 4:3 perspective one would be at the target from here,
    /// whatever the window's shape, as the original fixes its width.
    pub distance: f32,
    pub fov_y: f32,
    /// The original's overview is orthographic (0x4fc210); the steered
    /// skier's view is a perspective one.
    pub ortho: bool,
    pub aspect: f32,
    /// Remaining angle of an animated quarter turn.
    pub turning: f32,
    /// Which of `ZOOM_WIDTHS` the overview shows.
    pub step: usize,
    /// Mouse wheel movement not yet taken as a zoom step.
    wheel: f32,
}

impl Camera {
    /// An overview of `target` at the opening zoom step.
    pub fn new(target: Vec3) -> Camera {
        let mut c = Camera {
            turning: 0.0,
            step: OPENING_STEP,
            wheel: 0.0,
            target,
            yaw: -2.2,
            pitch: OVERVIEW_PITCH,
            distance: 0.0,
            fov_y: 0.8,
            ortho: true,
            aspect: 16.0 / 9.0,
        };
        c.show_step();
        c
    }

    pub fn eye(&self) -> Vec3 {
        let dir = Vec3::new(
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
        );
        self.target + dir * self.distance
    }

    /// Puts the eye at `eye`, looking at `at`.
    pub fn look_from(&mut self, eye: Vec3, at: Vec3) {
        let d = eye - at;
        let len = d.length().max(1e-3);
        self.target = at;
        self.distance = len;
        self.yaw = d.y.atan2(d.x);
        self.pitch = (d.z / len).clamp(-1.0, 1.0).asin();
    }

    pub fn view(&self) -> Mat4 {
        crate::render::look_at(self.eye(), self.target, Vec3::Z)
    }

    pub fn proj(&self) -> Mat4 {
        if self.ortho {
            let w = self.distance * (self.fov_y * 0.5).tan() * ORTHO_ASPECT;
            let h = w / self.aspect.max(0.1);
            // Depth reaches behind the eye too, so the eye's position
            // never clips the world.
            return glam::camera::rh::proj::directx::orthographic(-w, w, -h, h, -20000.0, 20000.0);
        }
        crate::render::perspective(
            self.fov_y,
            self.aspect,
            (self.distance * 0.01).clamp(0.5, 20.0),
            self.distance * 20.0 + 5000.0,
        )
    }

    pub fn view_proj(&self) -> Mat4 {
        self.proj() * self.view()
    }

    /// Ground-plane basis vectors (forward, right) for panning.
    pub fn ground_axes(&self) -> (Vec3, Vec3) {
        let fwd = -Vec3::new(self.yaw.cos(), self.yaw.sin(), 0.0);
        let right = Vec3::new(-fwd.y, fwd.x, 0.0) * -1.0;
        (fwd, right)
    }

    pub fn pan(&mut self, forward: f32, right: f32) {
        let (f, r) = self.ground_axes();
        let k = self.distance * 0.0015 + 0.05;
        self.target += (f * forward + r * right) * k;
    }

    /// Zooms so the view is `width` world units across at the target.
    pub fn set_view_width(&mut self, width: f32) {
        self.distance = width / (2.0 * (self.fov_y * 0.5).tan() * ORTHO_ASPECT);
    }

    /// Steps the zoom in (`dir` > 0) or out (`dir` < 0), as the original's
    /// zoom buttons do.
    pub fn zoom(&mut self, dir: i32) {
        self.step = (self.step as i32 - dir.signum()).clamp(0, ZOOM_WIDTHS.len() as i32 - 1) as usize;
        self.show_step();
    }

    /// Sets the view to the current zoom step's width.
    pub fn show_step(&mut self) {
        self.set_view_width(ZOOM_WIDTHS[self.step]);
    }

    /// Zooms a step for each whole notch the wheel has turned.
    pub fn wheel(&mut self, notches: f32) {
        self.wheel += notches;
        while self.wheel.abs() >= 1.0 {
            let dir = self.wheel.signum();
            self.zoom(dir as i32);
            self.wheel -= dir;
        }
    }

    /// Starts an animated 90° turn about the view center, as the original's
    /// rotate buttons do; `dir` is +1 or -1.
    pub fn quarter_turn(&mut self, dir: f32) {
        if self.turning.abs() < 1e-3 {
            self.turning = dir * std::f32::consts::FRAC_PI_2;
        }
    }

    /// Advances a quarter turn: the original steps π/100 per tick.
    pub fn animate(&mut self, dt: f32) {
        if self.turning == 0.0 {
            return;
        }
        let step = (std::f32::consts::PI / 100.0 * 60.0 * dt).min(self.turning.abs());
        let d = step * self.turning.signum();
        self.yaw += d;
        self.turning -= d;
        if self.turning.abs() < 1e-4 {
            self.turning = 0.0;
        }
    }

    pub fn rotate(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw += dyaw;
        self.pitch = (self.pitch + dpitch).clamp(0.12, 1.45);
    }

    /// Ray through a pixel (`ndc` in [-1, 1], y up).
    pub fn ray(&self, ndc: Vec2) -> (Vec3, Vec3) {
        let inv = self.view_proj().inverse();
        let near = inv * Vec4::new(ndc.x, ndc.y, 0.0, 1.0);
        let far = inv * Vec4::new(ndc.x, ndc.y, 1.0, 1.0);
        let near = near.truncate() / near.w;
        let far = far.truncate() / far.w;
        (near, (far - near).normalize())
    }

    /// Projects a world point to pixel coordinates, if it is in front.
    pub fn project(&self, p: Vec3, size: Vec2) -> Option<Vec2> {
        let c = self.view_proj() * p.extend(1.0);
        if c.w <= 0.0 {
            return None;
        }
        let n = c.truncate() / c.w;
        Some(Vec2::new((n.x * 0.5 + 0.5) * size.x, (1.0 - (n.y * 0.5 + 0.5)) * size.y))
    }
}
