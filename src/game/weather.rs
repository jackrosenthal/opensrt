//! Weather, as the original runs it each tick (0x4100e0).
//!
//! - Temperature (°F) swings along a sine around 25°: each half swing gets
//!   a random height (0 to 25°) and length (60 to 180 seconds).
//! - The barometer (inHg) runs through four 45-second phases: falling to
//!   29–30, low and steady, rising to 30–31, high and steady.
//! - Rain or snow starts at random, `dt / precipitation_interval` a tick,
//!   doubled while the barometer is low and never while it is high; snow
//!   below 32°F, rain above (0x410a50). A spell lasts 500–1200 ticks,
//!   half that for rain, and its end may bring an avalanche (0x468f20).
//! - Fog comes with `probability_fog` a tick, 0.1–0.6 thick for 500–2500
//!   ticks.
//! - The wind turns 0.005 rad a tick toward a heading up to 90° either
//!   side of the last, and its speed wanders 0.001 a tick between
//!   `min_wind_speed` and `max_wind_speed`.
//! - Snow depth runs from 20 to 96: snowfall adds 0.05 a tick, rain takes
//!   0.05 and clear weather 0.007, the last two only below
//!   `max_temperature`.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::sim::{Sim, TICK};
use super::world::Sky;
use crate::text::TextKey;

pub const DEPTH_MIN: f32 = 20.0;
pub const DEPTH_MAX: f32 = 96.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Mildly,
    Plain,
    Very,
}

/// The forecast's half of the report (`WEATHER_REPORT_[VERY_|MILDLY_]COLD`
/// or `..._WARM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempReport {
    pub strength: Strength,
    pub warm: bool,
}

impl TextKey for TempReport {
    fn key(&self) -> Cow<'static, str> {
        let prefix = match self.strength {
            Strength::Mildly => "MILDLY_",
            Strength::Plain => "",
            Strength::Very => "VERY_",
        };
        Cow::Owned(format!("WEATHER_REPORT_{prefix}{}", if self.warm { "WARM" } else { "COLD" }))
    }
}

/// The barometer's half of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaroReport {
    SteadyHighSun,
    SteadyHighRain,
    Falling,
    Low,
    Rising,
}

impl TextKey for BaroReport {
    fn key(&self) -> Cow<'static, str> {
        Cow::Borrowed(match self {
            BaroReport::SteadyHighSun => "WEATHER_REPORT_BAROMETER_STEADY_HIGH_SUN",
            BaroReport::SteadyHighRain => "WEATHER_REPORT_BAROMETER_STEADY_HIGH_RAIN",
            BaroReport::Falling => "WEATHER_REPORT_BAROMETER_FALLING",
            BaroReport::Low => "WEATHER_REPORT_BAROMETER_LOW",
            BaroReport::Rising => "WEATHER_REPORT_BAROMETER_RISING",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Precip {
    Snow,
    Rain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Weather {
    /// What the sky shows, from the fields below.
    #[serde(skip)]
    pub sky: Sky,
    /// Degrees Fahrenheit.
    pub temperature: f32,
    temp_phase: f32,
    temp_swing: f32,
    temp_period: f32,
    temp_half: bool,
    /// Inches of mercury.
    pub pressure: f32,
    pressure_t: f32,
    pressure_from: f32,
    pressure_to: f32,
    pressure_phase: u8,
    /// 0..1.
    pub humidity: f32,
    pub wind: f32,
    /// Radians.
    pub wind_dir: f32,
    wind_target: f32,
    /// 20..96.
    pub snow_depth: f32,
    /// Cloud cover 0..1 (0.5 snowing, 0.8 raining or foggy).
    pub cloud: f32,
    /// Snowfall strength 0..1, easing in and out.
    pub snowfall: f32,
    pub fog: f32,
    fog_ticks: i32,
    pub precip: Option<Precip>,
    precip_ticks: i32,
    /// Seconds run, for the humidity.
    clock: f32,
    /// Set when rain or snow starts, for the music.
    #[serde(skip)]
    pub started: Option<Precip>,
    /// Snow depth at the end of each day, the last 30 (0x428ed4).
    pub depth_history: Vec<f32>,
}

impl Default for Weather {
    /// The original's starting weather (0x40ff00).
    fn default() -> Weather {
        Weather {
            sky: Sky::Clear,
            temperature: 25.0,
            // Past a full swing, so the first tick picks a new one.
            temp_phase: 9999.0,
            temp_swing: 0.0,
            temp_period: 120.0,
            temp_half: false,
            pressure: 29.0,
            pressure_t: 0.0,
            pressure_from: 29.0,
            pressure_to: 29.0,
            pressure_phase: 2,
            humidity: 0.5,
            wind: 0.0,
            wind_dir: 0.0,
            wind_target: 0.0,
            snow_depth: 30.0,
            cloud: 0.0,
            snowfall: 0.0,
            fog: 0.0,
            fog_ticks: 0,
            precip: None,
            precip_ticks: 0,
            clock: 0.0,
            depth_history: Vec::new(),
            started: None,
        }
    }
}

impl Weather {
    pub fn end_of_day(&mut self) {
        if self.depth_history.len() >= 30 {
            self.depth_history.remove(0);
        }
        self.depth_history.push(self.snow_depth);
    }

    /// The weather station's report (0x410800): a temperature forecast
    /// from where the swing stands, and the barometer's phase.
    pub fn report(&self) -> (TempReport, BaroReport) {
        let c = self.temp_phase / (2.0 * std::f32::consts::PI);
        let warm = !self.temp_half;
        let temp = if c - c.trunc() >= 0.75 {
            TempReport { strength: Strength::Plain, warm: !warm }
        } else {
            // The original compares the swing (0 to 25°) against 0.66 and
            // 0.33, so it is nearly always "very".
            let strength = if self.temp_swing > 0.66 {
                Strength::Very
            } else if self.temp_swing < 0.33 {
                Strength::Mildly
            } else {
                Strength::Plain
            };
            TempReport { strength, warm }
        };
        let baro = match self.pressure_phase {
            0 if self.pressure_t / 45.0 < 0.1 => BaroReport::SteadyHighSun,
            0 => BaroReport::SteadyHighRain,
            1 => BaroReport::Falling,
            2 => BaroReport::Low,
            _ => BaroReport::Rising,
        };
        (temp, baro)
    }

    /// Snow depth as 0..1 of its range.
    pub fn depth_frac(&self) -> f32 {
        (self.snow_depth - DEPTH_MIN) / (DEPTH_MAX - DEPTH_MIN)
    }

    /// The snow variant (0 bare to 2 deep) of something at height `z`
    /// on a map spanning `zmin..zmax`: deeper snow reaches further down
    /// (0x40aa40).
    pub fn snow_variant(&self, z: f32, (zmin, zmax): (f32, f32)) -> u8 {
        let h = (z - zmin) / (zmax - zmin + 1.0);
        let d = self.depth_frac() * 2.0;
        if 2.0 - d < h {
            2
        } else if 1.0 - d < h {
            1
        } else {
            0
        }
    }
}

impl Sim<'_> {
    pub(crate) fn weather(&mut self, dt: f32) {
        let r = &self.defs.resort;
        let rolls: [f32; 8] = std::array::from_fn(|_| self.w.rand());
        let wx = &mut self.w.weather;
        let pi = std::f32::consts::PI;
        wx.clock += dt;

        // Temperature.
        if wx.temp_phase >= 2.0 * pi {
            wx.temp_phase = 0.0;
            wx.temp_half = false;
            wx.temp_swing = rolls[0] * 25.0;
            wx.temp_period = (rolls[1] + 0.5) * 120.0;
        } else if wx.temp_phase >= pi && !wx.temp_half {
            wx.temp_half = true;
            wx.temp_swing = rolls[0] * 25.0;
            wx.temp_period = (rolls[1] + 0.5) * 120.0;
        }
        wx.temp_phase += dt / wx.temp_period * pi;
        wx.temperature = wx.temp_phase.sin() * wx.temp_swing + 25.0;

        // Barometer.
        wx.pressure_t += dt;
        wx.pressure = (wx.pressure_to - wx.pressure_from) * wx.pressure_t / 45.0 + wx.pressure_from;
        if wx.pressure_t >= 45.0 {
            wx.pressure_t = 0.0;
            wx.pressure_phase = (wx.pressure_phase + 1) & 3;
            wx.pressure_from = wx.pressure_to;
            match wx.pressure_phase {
                1 => wx.pressure_to = rolls[2] + 29.0,
                3 => wx.pressure_to = rolls[2] + 30.0,
                _ => {}
            }
        }

        // Humidity: the original takes it from the wall clock, changing
        // every 1000 seconds; this uses game time instead.
        let k = (wx.clock / 1000.0).floor() * 2.0;
        wx.humidity = (k * 628.318_5).sin() * 0.3 + 0.4;

        // Rain or snow.
        let chance = match wx.pressure_phase {
            2 => 1.0,
            0 => 0.0,
            _ => 0.5,
        };
        let challenge = self.w.challenge_state.as_ref().is_some_and(|c| c.number == 5);
        // The avalanche challenge makes it snow eight times as often
        // (0x4130d0).
        let factor = if challenge { 8.0 } else { 1.0 };
        let wx = &mut self.w.weather;
        if wx.precip.is_none() && rolls[3] <= dt / r.precipitation_interval.max(1.0) * factor * chance {
            let rain = wx.temperature >= 32.0;
            let ticks = rolls[4] * 700.0 + 500.0;
            wx.precip = Some(if rain { Precip::Rain } else { Precip::Snow });
            wx.started = wx.precip;
            wx.precip_ticks = if rain { ticks * 0.5 } else { ticks } as i32;
        }

        // Fog.
        if rolls[5] < r.probability_fog && wx.fog_ticks < 1 {
            wx.fog = rolls[6] * 0.5 + 0.1;
            wx.fog_ticks = (rolls[7] * 2000.0 + 500.0) as i32;
        }
        let mut ended = false;
        if wx.precip.is_some() {
            wx.precip_ticks -= (dt / TICK).round().max(1.0) as i32;
            if wx.precip_ticks <= 0 {
                wx.precip = None;
                ended = true;
            }
        }
        if wx.fog_ticks == 0 {
            wx.fog = 0.0;
        }
        wx.fog_ticks -= 1;

        // Wind.
        let turn = 0.005;
        wx.wind_dir += if wx.wind_target <= wx.wind_dir { -turn } else { turn };
        if (wx.wind_target - wx.wind_dir).abs() < 0.02 {
            wx.wind_dir = wx.wind_dir.rem_euclid(2.0 * pi);
            let roll = self.w.rand();
            let wx = &mut self.w.weather;
            wx.wind_target = roll * pi - pi / 2.0 + wx.wind_dir;
        }
        let up = self.w.rand() < 0.5;
        let wx = &mut self.w.weather;
        if up && wx.wind < r.max_wind {
            wx.wind += 0.001;
        } else if !up && r.min_wind < wx.wind {
            wx.wind -= 0.001;
        }

        // Snow depth, cloud and snowfall.
        let (lo, hi) = (r.min_temperature, r.max_temperature);
        match wx.precip {
            Some(Precip::Snow) => {
                if lo < wx.temperature && wx.snow_depth < DEPTH_MAX {
                    wx.snow_depth += 0.05;
                }
                wx.cloud = 0.5;
                wx.snowfall = (wx.snowfall + 0.0015).min(1.0);
            }
            Some(Precip::Rain) => {
                if wx.temperature < hi && DEPTH_MIN < wx.snow_depth {
                    wx.snow_depth -= 0.05;
                }
                wx.snowfall = (wx.snowfall - 0.001).max(0.0);
                wx.cloud = 0.8;
            }
            None if wx.fog == 0.0 => {
                if wx.temperature < hi && DEPTH_MIN < wx.snow_depth {
                    wx.snow_depth -= 0.007;
                }
                wx.snowfall = (wx.snowfall - 0.0005).max(0.0);
                wx.cloud = 0.0;
            }
            None => {
                if lo < wx.temperature {
                    wx.cloud = 0.8;
                }
            }
        }
        wx.sky = match wx.precip {
            Some(Precip::Snow) => Sky::Snow,
            Some(Precip::Rain) => Sky::Rain,
            None if wx.fog > 0.0 => Sky::Fog,
            None => Sky::Clear,
        };
        if ended {
            self.avalanche_roll();
        }
    }
}
