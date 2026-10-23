//! Snow puffs and building sounds.

use super::*;

impl Game {
    /// Avalanche snow clouds and sound. The original throws three puffs of
    /// `snowcloud.tga` a tick, from the slide and its two side tracks,
    /// moving with it plus a random scatter (0x401830), with the start sound
    /// and a looping rumble (sounds.ini weather_avalanche_*). Puff size,
    /// growth and life are estimates.
    pub fn puffs(&mut self, dt: f32) {
        let live = if self.paused { 0.0 } else { dt * self.speed };
        for p in &mut self.puffs {
            p.age += live;
            p.pos += p.vel * live * 30.0;
            p.vel *= 0.95f32.powf(live * 30.0);
        }
        self.puffs.retain(|p| p.age < PUFF_LIFE);
        let mut seed = (self.world.avalanche_clock as u32).wrapping_mul(2654435761);
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        let n = (live / sim::TICK).round().min(8.0) as usize;
        for a in &self.world.avalanches {
            for _ in 0..n {
                let [l, r] = a.sides();
                for q in [a.pos.truncate(), l, r] {
                    let pos = self.world.ground(q) + Vec3::new(0.0, 0.0, 2.0);
                    let scatter = Vec3::new(rnd() - 0.5, rnd() - 0.5, rnd() * 0.5) * 0.3;
                    self.puffs.push(Puff {
                        pos,
                        vel: a.vel * 0.6 + scatter,
                        age: 0.0,
                        grow: 2.0 + rnd() * 2.0,
                    });
                }
            }
        }
        // Mortar bursts: snow from the cells within three steps of the
        // target, for 30 ticks (0x475ea0).
        let spacing = self.world.level().spacing;
        let mut blasts = std::mem::take(&mut self.world.blasts);
        for b in &mut blasts {
            b.t += live;
            for _ in 0..n {
                let (dx, dy) = ((rnd() * 7.0) as i32 - 3, (rnd() * 7.0) as i32 - 3);
                if dx.abs() + dy.abs() >= 4 {
                    continue;
                }
                let q = b.pos.truncate() + Vec2::new(dx as f32, dy as f32) * spacing + Vec2::new(rnd() - 0.5, rnd() - 0.5) * 10.0;
                let pos = self.world.ground(q) + Vec3::new(0.0, 0.0, 2.0);
                self.puffs.push(Puff {
                    pos,
                    vel: Vec3::new(0.0, 0.0, 0.1),
                    age: 0.0,
                    grow: 1.0 + rnd() * 2.0,
                });
            }
        }
        blasts.retain(|b| b.t < 30.0 * sim::TICK);
        self.world.blasts = blasts;
        if self.world.avalanches_started != self.seen_avalanches {
            self.seen_avalanches = self.world.avalanches_started;
            self.sfx(Sound::AvalancheStart);
        }
        // The rumble loops while a slide runs, louder the faster it goes:
        // (speed per tick + 1) × 5000 − 10000 hundredths of a dB (0x402418).
        match self.world.avalanches.iter().map(|a| a.vel.length()).reduce(f32::max) {
            Some(speed) => {
                let volume = (((speed + 1.0) * 5000.0 - 10000.0) as i32).min(0);
                match self.rumble {
                    Some(h) => crate::audio::set_volume(h, volume),
                    None => {
                        self.rumble = crate::audio::play_sound(
                            &self.assets,
                            Sound::AvalancheRumble,
                            crate::audio::Opts { music: false, looped: true, volume },
                        );
                    }
                }
            }
            None => {
                if let Some(h) = self.rumble.take() {
                    crate::audio::stop(h);
                }
            }
        }
    }

    /// Buildings on screen now and then make their sound (0x4b8010): after
    /// 300 quiet ticks, with a chance of 1 in 1000 a tick, while fewer than
    /// six effects play. The original sets the volume from its three zoom
    /// steps, 0, -6.7 and -13.3 dB.
    pub(super) fn on_screen(&self, p: Vec3, size: Vec2) -> bool {
        self.camera.project(p, size).is_some_and(|q| q.x >= 0.0 && q.y >= 0.0 && q.x <= size.x && q.y <= size.y)
    }

    pub(super) fn building_sounds(&mut self, dt: f32, size: Vec2) {
        if self.paused {
            return;
        }
        let ticks = dt * self.speed / sim::TICK;
        let zoom = self.camera.step as f32;
        let volume = ((15.0 - zoom) * 666.667 - 10000.0) as i32;
        let ids: Vec<(Id, Vec3, usize)> = self.world.structures.iter().map(|s| (s.id, s.pos, s.def)).collect();
        self.building_quiet.retain(|id, _| ids.iter().any(|x| x.0 == *id));
        for (id, pos, def) in ids {
            // The count runs all the time; a new building starts at 600
            // (0x4b2b4e), one just placed at 0 as it has just sounded.
            let quiet = self.building_quiet.entry(id).or_insert(600.0);
            *quiet += ticks;
            if *quiet <= 300.0 {
                continue;
            }
            if !self.on_screen(pos, size) {
                continue;
            }
            let chance = 1.0 - 0.999f32.powf(ticks);
            if self.world.rand() >= chance || crate::audio::effects_playing() >= 6 {
                continue;
            }
            let rel = sound_path(&self.defs.structures[def].menu_sound);
            if !rel.is_empty() {
                crate::audio::play(&self.assets, &rel, crate::audio::Opts { music: false, looped: false, volume });
                self.building_quiet.insert(id, 0.0);
            }
        }
    }
}
