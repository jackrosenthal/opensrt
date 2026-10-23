//! Music, as the original's music manager runs it (0x404b90, 0x404ea0).
//!
//! - The menus loop `skitycoonditty.wav` (0x474cb0).
//! - In the game a stream plays the songs in `data/music/`: one song over
//!   and over, or with the radio tower set to random a different song each
//!   time one ends. At night it plays `music.ini`'s night ambience instead,
//!   looped.
//! - When rain or snow starts, one of `music.ini`'s rain or snow songs
//!   plays once, and the stream fades to -40 dB over two seconds until it
//!   has finished, then fades back (0x4052d0, 0x405340).
//! - The escape menu stops those songs and fades the stream out, and plays
//!   the title tune only if there are fewer than two songs (0x448240).

use crate::assets::Assets;
use crate::audio::{self, Handle, Opts};

/// How fast the stream fades, in hundredths of a decibel a second.
const FADE: f32 = 2000.0;
const QUIET: f32 = -4000.0;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Fade {
    In,
    Out,
    Full,
    Silent,
}

pub struct Music {
    songs: Vec<String>,
    snow: Vec<String>,
    rain: Vec<String>,
    night_track: Option<String>,
    /// The song playing or to play; `None` once the radio stops the music.
    index: Option<usize>,
    pub random: bool,
    stream: Option<Handle>,
    night: bool,
    weather: Option<Handle>,
    fade: Fade,
    volume: f32,
    /// The title tune, in the menus or the escape menu.
    tune: Option<Handle>,
    in_game: Option<bool>,
    escape: bool,
    seed: u32,
}

/// Song lists from `music.ini`; bare names live in `data/sounds/music`.
fn ini_songs(assets: &Assets, section: &str) -> Vec<String> {
    let ini = crate::game::defs::load_ini(assets, "data/sounds/music.ini");
    let Some(s) = ini.section(section) else { return Vec::new() };
    let n = s.f32_or("num_songs", 0.0) as usize;
    (1..=n).filter_map(|i| s.get(&format!("song{i}"))).map(full_path).collect()
}

fn full_path(name: &str) -> String {
    if name.contains('/') || name.contains('\\') {
        name.replace('\\', "/")
    } else {
        format!("data/sounds/music/{name}")
    }
}

impl Music {
    pub fn new(assets: &Assets) -> Music {
        let ini = crate::game::defs::load_ini(assets, "data/sounds/music.ini");
        let night_track = ini.section("night").and_then(|s| s.get("source")).map(full_path);
        // `default.wav` comes first, the rest in the order they are found.
        let mut songs: Vec<String> = assets.list("data/music", "wav").into_iter().map(|(k, _)| k).collect();
        if let Some(i) = songs.iter().position(|s| s.ends_with("/default.wav")) {
            let d = songs.remove(i);
            songs.insert(0, d);
        }
        Music {
            index: (!songs.is_empty()).then_some(0),
            songs,
            snow: ini_songs(assets, "snow"),
            rain: ini_songs(assets, "rain"),
            night_track,
            random: false,
            stream: None,
            night: false,
            weather: None,
            fade: Fade::Full,
            volume: 0.0,
            tune: None,
            in_game: None,
            escape: false,
            seed: 0x1234_5678,
        }
    }

    fn rand(&mut self, n: usize) -> usize {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.seed >> 8) as usize % n.max(1)
    }

    pub fn song_names(&self) -> Vec<String> {
        self.songs.iter().map(|s| s.rsplit('/').next().unwrap_or(s).to_string()).collect()
    }

    pub fn current(&self) -> Option<usize> {
        self.index
    }

    fn stop_stream(&mut self) {
        if let Some(h) = self.stream.take() {
            audio::stop(h);
        }
    }

    fn stop_weather(&mut self) {
        if let Some(h) = self.weather.take() {
            audio::stop(h);
        }
    }

    fn play_song(&mut self, assets: &Assets) {
        let Some(i) = self.index else { return };
        self.stop_stream();
        let looped = !self.random;
        self.stream = audio::play(
            assets,
            &self.songs[i],
            Opts { music: true, looped, volume: self.volume as i32 },
        );
    }

    /// The radio tower picks a song (0x405280). A new stream starts at
    /// full volume (0x5695a0).
    pub fn choose(&mut self, assets: &Assets, i: usize) {
        if i < self.songs.len() {
            self.random = false;
            self.index = Some(i);
            self.volume = 0.0;
            self.fade = Fade::Full;
            self.play_song(assets);
        }
    }

    /// The radio tower's "Random" (0x4053b0).
    pub fn shuffle(&mut self, assets: &Assets) {
        // After Stop it does nothing, as in the original.
        self.random = true;
        let Some(i) = self.index else { return };
        let i = (i + 1) % self.songs.len().max(1);
        self.index = Some(i);
        self.volume = 0.0;
        self.fade = Fade::Full;
        self.play_song(assets);
    }

    /// Ctrl+Right and Ctrl+Left step through the songs (0x404ea0).
    pub fn step(&mut self, assets: &Assets, forward: bool) {
        let Some(i) = self.index else { return };
        let n = self.songs.len();
        let next = if forward {
            (i + 1) % n.max(1)
        } else if i == 0 {
            return;
        } else {
            i - 1
        };
        self.index = Some(next);
        self.play_song(assets);
    }

    /// The radio tower's "Stop" (0x405240).
    pub fn stop(&mut self) {
        self.index = None;
        self.stop_weather();
        self.fade = Fade::Out;
    }

    /// Rain or snow has started (0x410a50).
    pub fn weather_started(&mut self, assets: &Assets, rain: bool) {
        if self.in_game != Some(true) {
            return;
        }
        self.stop_weather();
        let list = if rain { self.rain.clone() } else { self.snow.clone() };
        if list.is_empty() {
            return;
        }
        let pick = list[self.rand(list.len())].clone();
        self.weather = audio::play(assets, &pick, Opts { music: true, looped: false, volume: 0 });
        self.fade = Fade::Out;
    }

    /// Call every frame. `game` is `Some(night)` while a game is shown,
    /// `escape` while its escape menu is open.
    pub fn update(&mut self, assets: &Assets, dt: f32, game: Option<bool>, escape: bool) {
        let in_game = game.is_some();
        if self.in_game != Some(in_game) {
            // Switching between the menus and a game stops everything.
            self.in_game = Some(in_game);
            audio::stop_all(true);
            self.stream = None;
            self.weather = None;
            self.tune = None;
            self.night = false;
            self.escape = false;
            self.volume = 0.0;
            self.fade = Fade::Full;
            if !in_game {
                self.tune = audio::play_sound(assets, audio::Sound::TitleTune, Opts { music: true, looped: true, volume: 0 });
            }
        }
        let Some(night) = game else { return };

        if escape != self.escape {
            self.escape = escape;
            if escape {
                self.stop_weather();
                self.fade = Fade::Out;
                if self.songs.len() < 2 {
                    self.tune = audio::play_sound(assets, audio::Sound::TitleTune, Opts { music: true, looped: true, volume: 0 });
                }
            } else {
                if let Some(h) = self.tune.take() {
                    audio::stop(h);
                }
                self.fade = Fade::In;
            }
        }

        // Night swaps the songs for the night ambience and back.
        if night != self.night {
            self.night = night;
            audio::stop_all(true);
            self.stream = None;
            // A weather song cut short ends like one that finished.
            if self.weather.take().is_some() {
                self.fade = Fade::In;
            }
            if let Some(h) = self.tune.take() {
                audio::stop(h);
            }
            if night {
                if let Some(t) = self.night_track.clone() {
                    self.stream = audio::play(assets, &t, Opts { music: true, looped: true, volume: self.volume as i32 });
                }
            }
        }

        if let Some(w) = self.weather {
            if !audio::playing(w) {
                self.weather = None;
                self.fade = Fade::In;
            }
        }

        let streaming = self.stream.is_some_and(audio::playing);
        if !streaming && self.weather.is_none() && self.index.is_some() {
            if self.night {
                if let Some(t) = self.night_track.clone() {
                    self.stream = audio::play(assets, &t, Opts { music: true, looped: true, volume: self.volume as i32 });
                }
            } else {
                if self.random && self.songs.len() > 1 {
                    let cur = self.index.unwrap_or(0);
                    let mut i = self.rand(self.songs.len());
                    if i == cur {
                        i = (i + 1) % self.songs.len();
                    }
                    self.index = Some(i);
                }
                self.play_song(assets);
            }
        }

        match self.fade {
            Fade::In => {
                self.volume += FADE * dt;
                if self.volume >= 0.0 {
                    self.volume = 0.0;
                    self.fade = Fade::Full;
                }
            }
            Fade::Out => {
                self.volume -= FADE * dt;
                if self.volume <= QUIET {
                    self.volume = QUIET;
                    self.fade = Fade::Silent;
                }
            }
            Fade::Full | Fade::Silent => {}
        }
        if let Some(h) = self.stream {
            audio::set_volume(h, self.volume as i32);
        }
    }
}
