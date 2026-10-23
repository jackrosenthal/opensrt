//! Falling snow drawn over the view while it snows. Measured from the
//! original at 640×480: about 140 white flakes of one to three pixels,
//! all drifting by about (10, 30) pixels a second.

use egui::{Color32, Pos2, Vec2};

use crate::game::Game;

const FLAKES_640: f32 = 140.0;
const DRIFT_480: Vec2 = Vec2::new(10.0, 30.0);

#[derive(Default)]
pub struct Snowfall {
    /// Flakes in screen fractions (0..1), with a size of one to three pixels.
    flakes: Vec<(Pos2, f32)>,
    seed: u32,
}

impl Snowfall {
    fn rand(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn show(&mut self, ui: &egui::Ui, g: &Game) {
        let heavy = g.world.weather.snowfall;
        let screen = ui.ctx().content_rect();
        let ppp = ui.ctx().pixels_per_point();
        let px = screen.size() * ppp;
        let want = (FLAKES_640 * heavy * px.x * px.y / (640.0 * 480.0)) as usize;
        while self.flakes.len() < want {
            let p = Pos2::new(self.rand(), self.rand());
            let size = 1.0 + (self.rand() * 3.0).floor().min(2.0);
            self.flakes.push((p, size));
        }
        self.flakes.truncate(want);
        if want == 0 {
            return;
        }
        if !g.paused {
            let dt = ui.input(|i| i.stable_dt).min(0.1);
            let step = DRIFT_480 * (px.y / 480.0) * dt;
            for (p, _) in &mut self.flakes {
                p.x = (p.x + step.x / px.x).rem_euclid(1.0);
                p.y = (p.y + step.y / px.y).rem_euclid(1.0);
            }
            ui.ctx().request_repaint();
        }
        let painter = ui.ctx().layer_painter(egui::LayerId::background());
        for (p, size) in &self.flakes {
            let at = screen.min + Vec2::new(p.x * screen.width(), p.y * screen.height());
            painter.rect_filled(egui::Rect::from_center_size(at, Vec2::splat(size / ppp)), 0.0, Color32::from_rgb(255, 251, 255));
        }
    }
}
