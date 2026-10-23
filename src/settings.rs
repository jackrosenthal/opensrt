//! Player options from the settings screen, saved between sessions.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Whether the game starts fullscreen, as F11 last left it.
    pub fullscreen: bool,
    /// The window's size (logical pixels) when last windowed.
    pub window: Option<(u32, u32)>,
    /// Scenery detail: 0 low, 1 medium, 2 high.
    pub detail: usize,
    /// 0 off, 1..=5 volume steps.
    pub sound: usize,
    pub music: usize,
    /// 0 slow .. 4 fast.
    pub scroll_rate: usize,
    pub all_buildings: bool,
    pub fast_maintenance: bool,
    pub reduce_garbage: bool,
    pub evil_sasquatches: bool,
    pub no_nightfall: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            fullscreen: true,
            window: None,
            detail: 2,
            sound: 4,
            music: 3,
            scroll_rate: 2,
            all_buildings: false,
            fast_maintenance: false,
            reduce_garbage: false,
            evil_sasquatches: true,
            no_nightfall: false,
        }
    }
}

fn path() -> Option<PathBuf> {
    crate::assets::data_home().map(|d| d.join("srt/settings.json"))
}

impl Settings {
    pub fn load() -> Settings {
        path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = path() {
            if let Some(d) = p.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            if let Ok(b) = serde_json::to_vec_pretty(self) {
                let _ = std::fs::write(p, b);
            }
        }
    }

    pub fn apply_audio(&self) {
        // The settings screen's levels, Off then Quiet to Loud (0x44c280).
        const LEVELS: [f32; 6] = [0.0, 0.1, 0.4, 0.7, 0.9, 1.0];
        let level = |i: usize| LEVELS[i.min(5)];
        crate::audio::set_volumes(level(self.sound), level(self.music));
    }

    /// The scroll rate's steps, relative to the middle one: the original
    /// stores 512 to 1536 in steps of 256 (0x44c0f6).
    pub fn pan_speed(&self) -> f32 {
        [0.5, 0.75, 1.0, 1.25, 1.5][self.scroll_rate.min(4)]
    }

}
