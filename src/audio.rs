//! Sound effects and music, played from the original WAV files.
//!
//! Like the original's sound manager (0x5695e0), every sound is either an
//! effect or music, which picks the master volume it follows; it may loop;
//! and it has its own volume in hundredths of a decibel, 0 being full and
//! -10000 silent. Playing returns a handle for stopping it, changing its
//! volume or asking whether it is still playing.
//!
//! The output device lives on its own thread; players attach to its mixer
//! and are driven from the caller's thread.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::assets::Assets;

pub type Handle = u32;

#[derive(Clone, Copy, Debug, Default)]
pub struct Opts {
    pub music: bool,
    pub looped: bool,
    /// Hundredths of a decibel; 0 is full volume.
    pub volume: i32,
}

struct Voice {
    player: rodio::Player,
    music: bool,
    looped: bool,
    volume: i32,
}

struct Engine {
    mixer: Option<rodio::mixer::Mixer>,
    voices: HashMap<Handle, Voice>,
    next: Handle,
    fx: f32,
    music: f32,
}

static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

fn engine() -> &'static Mutex<Engine> {
    ENGINE.get_or_init(|| {
        let disabled = std::env::var_os("SRT_NO_AUDIO").is_some();
        let (tx, rx) = std::sync::mpsc::channel();
        if !disabled {
            let _ = std::thread::Builder::new().name("audio".into()).spawn(move || {
                match rodio::DeviceSinkBuilder::open_default_sink() {
                    Ok(mut sink) => {
                        sink.log_on_drop(false);
                        let _ = tx.send(Some(sink.mixer().clone()));
                        // Keep the device open for the life of the program.
                        loop {
                            std::thread::park();
                        }
                    }
                    Err(e) => {
                        log::warn!("audio disabled: {e}");
                        let _ = tx.send(None);
                    }
                }
            });
        }
        let mixer = if disabled { None } else { rx.recv().ok().flatten() };
        Mutex::new(Engine {
            mixer,
            voices: HashMap::new(),
            next: 1,
            fx: 0.8,
            music: 0.5,
        })
    })
}

fn gain(e: &Engine, v: &Voice) -> f32 {
    let master = if v.music { e.music } else { e.fx };
    master * 10f32.powf(v.volume.clamp(-10000, 0) as f32 / 2000.0)
}

/// Plays a sound by game-relative path.
pub fn play(assets: &Assets, rel: &str, opts: Opts) -> Option<Handle> {
    let path = assets.resolve(rel)?.to_path_buf();
    let mut e = engine().lock().ok()?;
    e.voices.retain(|_, v| v.looped || !v.player.empty());
    let mixer = e.mixer.clone()?;
    let file = std::fs::File::open(&path).ok()?;
    let decoder = rodio::Decoder::try_from(file).map_err(|err| log::warn!("{}: {err}", path.display())).ok()?;
    let player = rodio::Player::connect_new(&mixer);
    if opts.looped {
        use rodio::Source;
        player.append(decoder.buffered().repeat_infinite());
    } else {
        player.append(decoder);
    }
    let voice = Voice {
        player,
        music: opts.music,
        looped: opts.looped,
        volume: opts.volume,
    };
    let g = gain(&e, &voice);
    voice.player.set_volume(g);
    let h = e.next;
    e.next += 1;
    e.voices.insert(h, voice);
    Some(h)
}

/// Plays an effect once at full volume.
pub fn play_ui(assets: &Assets, rel: &str) {
    play(assets, rel, Opts::default());
}

/// The game's fixed sounds. Most are the slots of the original's sound
/// table, filled from `sounds.ini` (0x402f30); the rest it names in code.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sound {
    Placing,
    Destroy,
    PlacingSkier,
    PickupSkier,
    UiDownclick,
    UiRollover,
    AvalancheStart,
    AvalancheRumble,
    Groomer,
    ChopTrees,
    IngameRollover,
    IngameDownclick,
    TitleTune,
    Ram,
    HitByRam,
    Sasquatch,
    Sweeping,
    SkiingLoop,
    WalkInSnowLoop,
    Turn,
    Jump,
    Crash1,
    Crash2,
    ShootMortar,
    MortarExplosion,
}

impl Sound {
    /// The `sounds.ini` key of a table sound.
    fn ini_key(self) -> Option<&'static str> {
        Some(match self {
            Sound::Placing => "placing_structure",
            Sound::Destroy => "destroy_structure",
            Sound::PlacingSkier => "placing_skier",
            Sound::PickupSkier => "pickup_skier",
            Sound::UiDownclick => "ui_downclick",
            Sound::UiRollover => "ui_rollover",
            Sound::AvalancheStart => "weather_avalanche_start",
            Sound::AvalancheRumble => "weather_avalanche_continuous",
            Sound::Groomer => "machine_groomer",
            Sound::ChopTrees => "terrain_chop_trees",
            Sound::IngameRollover => "ingame_rollover",
            Sound::IngameDownclick => "ingame_downclick",
            _ => return None,
        })
    }

    /// Where the sound is when it is not in the table, or the table
    /// lacks it.
    fn default_path(self) -> &'static str {
        match self {
            Sound::Placing => "data/sounds/general/buildingplace.wav",
            Sound::Destroy => "data/sounds/general/destroy.wav",
            Sound::PlacingSkier => "data/sounds/general/placeskier.wav",
            Sound::PickupSkier => "data/sounds/general/pickupskier.wav",
            Sound::UiDownclick => "data/sounds/general/downclick.wav",
            Sound::UiRollover => "data/sounds/general/rollover.wav",
            Sound::AvalancheStart => "data/sounds/general/avalanchestart.wav",
            Sound::AvalancheRumble => "data/sounds/general/rumble.wav",
            Sound::Groomer => "data/sounds/other/groomerloop.wav",
            Sound::ChopTrees => "data/sounds/general/sawdowntrees.wav",
            Sound::IngameRollover => "data/sounds/general/ingame_rollover.wav",
            Sound::IngameDownclick => "data/sounds/general/ingame_downclick.wav",
            Sound::TitleTune => "data/sounds/skitycoonditty.wav",
            Sound::Ram => "data/sounds/other/ram.wav",
            Sound::HitByRam => "data/sounds/other/hitbyram.wav",
            Sound::Sasquatch => "data/sounds/other/sasquatch.wav",
            Sound::Sweeping => "data/sounds/other/sweeping.wav",
            Sound::SkiingLoop => "data/sounds/skiing/skiingloop.wav",
            Sound::WalkInSnowLoop => "data/sounds/skiing/walkinsnowloop.wav",
            Sound::Turn => "data/sounds/skiing/turn.wav",
            Sound::Jump => "data/sounds/skiing/jump.wav",
            Sound::Crash1 => "data/sounds/skiing/crash1.wav",
            Sound::Crash2 => "data/sounds/skiing/crash2.wav",
            Sound::ShootMortar => "data/sounds/other/shootmortor.wav",
            Sound::MortarExplosion => "data/sounds/other/mortorexplosion.wav",
        }
    }

    /// The file to play. `sounds.ini` gives bare file names, which the
    /// original finds anywhere under `data/sounds`.
    pub fn path(self, assets: &Assets) -> String {
        static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
        let table = TABLE.get_or_init(|| {
            let ini = crate::game::defs::load_ini(assets, "data/sounds/sounds.ini");
            let mut t = HashMap::new();
            if let Some(sec) = ini.section("sounds") {
                for (k, v) in &sec.entries {
                    let file = v.trim().to_ascii_lowercase();
                    let found = ["general", "other", "skiing", "music", "buildings", ""]
                        .iter()
                        .map(|d| if d.is_empty() { format!("data/sounds/{file}") } else { format!("data/sounds/{d}/{file}") })
                        .find(|p| assets.resolve(p).is_some());
                    if let Some(p) = found {
                        t.insert(k.trim().to_ascii_lowercase(), p);
                    }
                }
            }
            t
        });
        self.ini_key()
            .and_then(|k| table.get(k).cloned())
            .unwrap_or_else(|| self.default_path().to_string())
    }
}

/// Plays one of the game's fixed sounds.
pub fn play_sound(assets: &Assets, sound: Sound, opts: Opts) -> Option<Handle> {
    play(assets, &sound.path(assets), opts)
}

/// Plays one of the game's fixed sounds once at full volume.
pub fn sfx(assets: &Assets, sound: Sound) {
    play_sound(assets, sound, Opts::default());
}

pub fn stop(h: Handle) {
    if let Ok(mut e) = engine().lock() {
        if let Some(v) = e.voices.remove(&h) {
            v.player.stop();
        }
    }
}

/// Stops every sound, or every music sound.
pub fn stop_all(music_only: bool) {
    if let Ok(mut e) = engine().lock() {
        e.voices.retain(|_, v| {
            let stop = !music_only || v.music;
            if stop {
                v.player.stop();
            }
            !stop
        });
    }
}

/// How many effects are playing.
pub fn effects_playing() -> usize {
    engine().lock().map_or(0, |e| e.voices.values().filter(|v| !v.music && !v.player.empty()).count())
}

pub fn playing(h: Handle) -> bool {
    engine().lock().ok().is_some_and(|e| e.voices.get(&h).is_some_and(|v| !v.player.empty()))
}

/// Sets a sound's own volume, in hundredths of a decibel.
pub fn set_volume(h: Handle, volume: i32) {
    if let Ok(mut e) = engine().lock() {
        let Some(v) = e.voices.get_mut(&h) else { return };
        v.volume = volume;
        let v = &e.voices[&h];
        v.player.set_volume(gain(&e, v));
    }
}

/// Sets the master volumes, 0..1.
pub fn set_volumes(effects: f32, music: f32) {
    if let Ok(mut e) = engine().lock() {
        e.fx = effects;
        e.music = music;
        for v in e.voices.values() {
            v.player.set_volume(gain(&e, v));
        }
    }
}
