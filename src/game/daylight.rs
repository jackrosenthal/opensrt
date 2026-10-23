//! Day and night, as the original runs them (0x427990): not by the clock
//! but on their own tick count, 10800 ticks (six minutes) of day, 120 of
//! dusk, 1800 (one minute) of night and 120 of dawn, round and round. With
//! the "No nightfall" option day never turns to dusk.
//!
//! The lights follow: a sun of 0.8 gray by day, 0.078 at night, shining
//! along (0.5, 0.2, −1) and never moving, and an ambient of 0.2 (0x4222c0,
//! colors from 0x41e340). Dusk and dawn step between the day and night
//! colors every 15 ticks.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Phase {
    #[default]
    Day,
    Dusk,
    Night,
    Dawn,
}

const DAY: u32 = 10800;
const NIGHT: u32 = 1800;
const TWILIGHT: u32 = 120;

/// The sun's color by day and by night.
pub const SUN_DAY: f32 = 0.8;
pub const SUN_NIGHT: f32 = 0.078_431_37;
pub const AMBIENT: f32 = 0.2;
/// The direction the sunlight travels.
pub const SUN_DIR: [f32; 3] = [0.5, 0.2, -1.0];
/// The night ambient (0x41e340: 8, 25, 29 of 255).
pub const AMBIENT_NIGHT: [f32; 3] = [0.031_372_55, 0.098_039_22, 0.113_725_49];

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct DayCycle {
    pub phase: Phase,
    pub ticks: u32,
}

impl DayCycle {
    pub fn tick(&mut self, no_nightfall: bool) {
        let (len, next) = match self.phase {
            Phase::Day if no_nightfall => {
                self.ticks = 0;
                return;
            }
            Phase::Day => (DAY, Phase::Dusk),
            Phase::Dusk => (TWILIGHT, Phase::Night),
            Phase::Night => (NIGHT, Phase::Dawn),
            Phase::Dawn => (TWILIGHT, Phase::Day),
        };
        self.ticks += 1;
        if self.ticks >= len {
            self.ticks = 0;
            self.phase = next;
        }
    }

    /// Whether the night meshes, lamps and night music are on (0x42d8d0):
    /// at night and for the first 80 ticks of dawn.
    pub fn is_night(&self) -> bool {
        match self.phase {
            Phase::Night => true,
            Phase::Dawn => self.ticks < 80,
            _ => false,
        }
    }

    /// The brightness sprites are drawn at, 0..1 (0x42d920): full by day,
    /// fading over dusk to half and from nothing over dawn.
    pub fn sprite_light(&self) -> f32 {
        match self.phase {
            Phase::Day => 1.0,
            Phase::Night => 0.0,
            Phase::Dusk => 1.0 - self.ticks as f32 / 240.0,
            Phase::Dawn => self.ticks as f32 / 240.0,
        }
    }

    /// How far from night to day the lights are, 0..1, in the eight steps
    /// of 15 ticks that dusk and dawn take.
    pub fn light(&self) -> f32 {
        let step = |t: u32| (t / 15) as f32 / (TWILIGHT / 15) as f32;
        match self.phase {
            Phase::Day => 1.0,
            Phase::Night => 0.0,
            Phase::Dusk => 1.0 - step(self.ticks),
            Phase::Dawn => step(self.ticks),
        }
    }
}
