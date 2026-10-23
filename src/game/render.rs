//! Gathering the world into draw batches, and the lights and sky.

use super::*;

impl Game {
    pub(super) fn structure_model(&self, def: usize, center: Vec3, rot: u8) -> Mat4 {
        let d = &self.defs.structures[def];
        let (w, h) = d.size;
        let s = self.world.level().spacing;
        // Meshes are authored from the footprint corner toward +x/+y.
        let to_origin = Mat4::from_translation(Vec3::new(-(w as f32) * s * 0.5, -(h as f32) * s * 0.5, 0.0));
        Mat4::from_translation(center)
            * Mat4::from_rotation_z(rot as f32 * std::f32::consts::FRAC_PI_2)
            * to_origin
    }

    /// A digest of every cell's worn-look step.
    fn wear_key(&self) -> u64 {
        self.world.level().cells.iter().fold(0xcbf2_9ce4_8422_2325u64, |k, c| {
            let step = if c.wear > 20.0 { 1 + ((c.wear - 20.0) / 20.0).min(3.0) as u64 } else { 0 };
            (k ^ step).wrapping_mul(0x0100_0000_01b3)
        })
    }

    pub fn prepare_render(&mut self, gpu: &Gpu, scene: &mut SceneRenderer) {
        // Footpaths turn snowy and back as the snow deepens and melts.
        self.snow_check = self.snow_check.saturating_sub(1);
        if !self.terrain_dirty && self.snow_check == 0 {
            self.snow_check = 30;
            if self.find_snowy_paths() != self.snowy_paths {
                self.terrain_dirty = true;
            }
            // Skiers wear the snow and it mends: the worn look steps at
            // wear 20, 40, 60 and 80.
            let key = self.wear_key();
            if key != self.worn {
                self.worn = key;
                self.terrain_dirty = true;
            }
        }
        if self.terrain_dirty {
            scene.terrain.rebuild(gpu, self.world.level(), &self.deep_snow());
            self.snowy_paths = self.find_snowy_paths();
            self.terrain_dirty = false;
            // The ground moved: its lamp light is redone.
            self.ground_lamps_key = 0;
        }
        let assets = self.assets.clone();
        let defs = self.defs.clone();
        let night = self.world.is_night();
        // Snow variants go by height: deeper snow reaches further down.
        let (wx, zr) = (self.world.weather.clone(), self.world.level().height_range());
        let snow = move |z: f32| wx.snow_variant(z, zr);
        let get = |cache: &mut MeshCache, rel: &str| cache.get(gpu, &scene.meshes, &assets, rel, 0);

        if self.foliage_dirty {
            self.foliage.clear();
            let level = self.world.level();
            let n = level.cells_per_side();
            for cy in 0..n {
                for cx in 0..n {
                    let c = level.cell(cx, cy);
                    let Some(path) = FOLIAGE_MESHES.get(c.foliage as usize) else { continue };
                    if path.is_empty() {
                        continue;
                    }
                    let id = get(&mut self.meshes, path);
                    let x = (cx as f32 + 0.5) * level.spacing;
                    let y = (cy as f32 + 0.5) * level.spacing;
                    let z = level.height_at(x, y);
                    let rot = ((cx * 7 + cy * 13) % 16) as f32 * 0.39;
                    self.foliage
                        .push((id, Mat4::from_rotation_translation(Quat::from_rotation_z(rot), Vec3::new(x, y, z))));
                }
            }
            self.foliage_dirty = false;
        }
        self.batches.clear();
        self.highlights.clear();
        // Litter and yellow snow are drawn on their cells (0x40bce0).
        let spacing = self.world.level().spacing;
        let dirt: Vec<((usize, usize), u8)> = self
            .world
            .litter
            .iter()
            .map(|l| (((l.pos.x / spacing) as usize, (l.pos.y / spacing) as usize), if l.yellow { 2 } else { 1 }))
            .collect();
        scene.terrain.set_marks(gpu, &self.cell_marks, &dirt);
        scene.terrain.plain = self.settings.detail == 0;
        for (id, m) in &self.foliage {
            self.batches.push_sprite(*id, *m, Vec4::ONE);
        }

        let mut occluders: Vec<(MeshId, Mat4)> = Vec::new();
        for s in &self.world.structures {
            let d = &defs.structures[s.def];
            let id = get(&mut self.meshes, d.meshes.pick(night, snow(s.pos.z)));
            let tint = Vec4::ONE;
            let model = {
                let (w, h) = d.size;
                let sp = self.world.level().spacing;
                Mat4::from_translation(s.pos)
                    * Mat4::from_rotation_z(s.rot as f32 * std::f32::consts::FRAC_PI_2)
                    * Mat4::from_translation(Vec3::new(-(w as f32) * sp * 0.5, -(h as f32) * sp * 0.5, 0.0))
            };
            self.batches.push_sprite(id, model, tint);
            occluders.push((id, model));
            // The selected object gets the green box (0x439630).
            if self.selection == Some(Selection::Structure(s.id)) {
                self.highlights.extend(highlight(&self.meshes, id, model, false));
            }
            if d.category == Category::Entrance {
                if let Some(sign) = self.meshes.entrance_sign(gpu, &scene.meshes, &assets) {
                    self.batches.push_prelit(sign, model, tint);
                }
            }
        }
        if let (Tool::Structure { def, .. }, Some(pl)) = (&self.tool, &self.placement) {
            let id = get(&mut self.meshes, defs.structures[*def].meshes.pick(night, snow(pl.model.w_axis.z)));
            self.batches.push_sprite(id, pl.model, Vec4::ONE);
        }

        for l in &self.world.lifts {
            let d = &defs.lifts[l.def];
            let dir = (l.end - l.start).truncate();
            // Lift meshes point along +x, toward the top station.
            let yaw = dir.y.atan2(dir.x);
            let rot = Quat::from_rotation_z(yaw);
            let sel = self.selection == Some(Selection::Lift(l.id));
            let tint = Vec4::ONE;
            // Stations and towers carry vertex-animation frames (turning
            // wheels), played while the lift runs. The rate is not known
            // from the original; LIFT_FPS is a guess.
            let frame = (l.anim * LIFT_FPS) as usize;
            let animated = |cache: &mut MeshCache, rel: &str| {
                let first = get(cache, rel);
                let n = cache.frames(first);
                cache.get(gpu, &scene.meshes, &assets, rel, frame % n.max(1))
            };
            let b = animated(&mut self.meshes, d.begin.pick(night, snow(l.start.z)));
            let e = animated(&mut self.meshes, d.end.pick(night, snow(l.end.z)));
            let mut pieces: Vec<(MeshId, Mat4)> = vec![
                (b, Mat4::from_rotation_translation(rot, l.start)),
                (e, Mat4::from_rotation_translation(rot, l.end)),
            ];
            for p in &l.poles {
                let m = animated(&mut self.meshes, d.middle.pick(night, snow(p.z)));
                pieces.push((m, Mat4::from_rotation_translation(rot, *p)));
            }
            for (id, model) in pieces {
                self.batches.lift_meshes.insert(id);
                self.batches.push_sprite(id, model, tint);
                if sel {
                    self.highlights.extend(highlight(&self.meshes, id, model, false));
                }
            }
            if l.cable_loop.len() >= 2 {
                let mut pts = l.cable_loop.clone();
                pts.push(pts[0]);
                self.batches.line_strip(&pts, CABLE);
            }
            if let Some(chair) = d.chairs.first() {
                let empty = get(&mut self.meshes, chair);
                let full = d
                    .chairs
                    .get(1)
                    .map(|c| get(&mut self.meshes, c))
                    .unwrap_or(empty);
                for (ci, c) in l.chairs.iter().enumerate() {
                    let (p, fwd) = l.point_at(c.s);
                    let rot = Quat::from_rotation_z(fwd.y.atan2(fwd.x));
                    let rider = c.riders.first().and_then(|id| self.world.guests.iter().find(|g| g.id == *id));
                    if !d.show_skiers {
                        // Chairs carry their riders in the mesh:
                        // `<chair>_<riders><A-D>`, the letter picked per chair.
                        let mesh = if c.riders.is_empty() {
                            empty
                        } else {
                            let n = c.riders.len().min(2);
                            let letter = (b'A' + (ci % 4) as u8) as char;
                            let (stem, ext) = chair.rsplit_once('.').unwrap_or((chair, "MESH"));
                            let rel = format!("{stem}_{n}{letter}.{ext}");
                            if assets.resolve(&rel).is_some() { get(&mut self.meshes, &rel) } else { full }
                        };
                        self.batches.push(mesh, Mat4::from_rotation_translation(rot, p), Vec4::ONE);
                    } else if let Some(g) = rider {
                        // A loaded drag lift hanger: its `tbar` point in the
                        // skier's hands (`tongs`), its top on a line up to
                        // the cable.
                        let body = guest_mesh(g);
                        let tongs = self
                            .meshes
                            .source(gpu, &assets, &body)
                            .and_then(|m| attach_point(&m.mesh.attach, AttachPoint::Tongs))
                            .unwrap_or(Vec3::new(-0.3, 0.0, 3.15));
                        let grip = g.pos + Quat::from_rotation_z(g.heading) * tongs;
                        let tbar = self
                            .meshes
                            .source(gpu, &assets, d.chairs.get(1).unwrap_or(chair))
                            .and_then(|m| attach_point(&m.mesh.attach, AttachPoint::Tbar))
                            .unwrap_or(Vec3::ZERO);
                        let top = grip - rot * tbar;
                        self.batches.push(full, Mat4::from_rotation_translation(rot, top), Vec4::ONE);
                        self.batches.line_strip(&[p, top], CABLE);
                    } else {
                        self.batches.push(empty, Mat4::from_rotation_translation(rot, p), Vec4::ONE);
                    }
                }
            }
        }
        if let (Tool::Lift { def, start: None }, Some(h)) = (&self.tool, self.hover) {
            let d = &defs.lifts[*def];
            let id = get(&mut self.meshes, d.begin.pick(night, snow(h.z)));
            let ok = self.placement.as_ref().map(|p| p.valid).unwrap_or(false);
            self.batches.push_sprite(id, Mat4::from_translation(h), Vec4::ONE);
            self.highlights.extend(highlight(&self.meshes, id, Mat4::from_translation(h), !ok));
        }
        if let (Tool::Lift { def, start: Some(st) }, Some(h)) = (&self.tool, self.hover) {
            let d = &defs.lifts[*def];
            let id = get(&mut self.meshes, d.middle.pick(night, snow(h.z)));
            let begin = get(&mut self.meshes, d.begin.pick(night, snow(st.z)));
            let end = get(&mut self.meshes, d.end.pick(night, snow(h.z)));
            let ok = self.placement.as_ref().map(|p| p.valid).unwrap_or(false);
            let (poles, _, _) = self.lift_plan(*def, *st, h);
            let dir = (h - *st).truncate();
            let rot = Quat::from_rotation_z(dir.y.atan2(dir.x));
            let mut pieces: Vec<(MeshId, Mat4)> =
                poles.iter().map(|p| (id, Mat4::from_rotation_translation(rot, *p))).collect();
            pieces.push((begin, Mat4::from_rotation_translation(rot, *st)));
            pieces.push((end, Mat4::from_rotation_translation(rot, h)));
            for (mesh, m) in pieces {
                self.batches.push_sprite(mesh, m, Vec4::ONE);
                self.highlights.extend(highlight(&self.meshes, mesh, m, !ok));
            }
            // The planned cable, drawn like a built one (0x49e030).
            let mut line = vec![*st + Vec3::Z * 12.0];
            line.extend(poles.iter().map(|p| *p + Vec3::Z * 20.0));
            line.push(h + Vec3::Z * 12.0);
            self.batches.line_strip(&line, CABLE);
        }

        for r in &self.world.runs {
            let Some(d) = defs.runs.get(r.def) else { continue };
            let id = get(&mut self.meshes, &d.mesh);
            let sel = self.selection == Some(Selection::Run(r.id));
            for (i, g) in r.gates.iter().enumerate() {
                let next = r.gates.get(i + 1).or(r.gates.get(i.wrapping_sub(1))).copied().unwrap_or(*g);
                let dir = (next - *g).truncate();
                let yaw = dir.y.atan2(dir.x);
                let tint = if sel { Vec4::new(1.4, 1.4, 0.6, 1.0) } else { Vec4::ONE };
                self.batches
                    .push_sprite(id, Mat4::from_rotation_translation(Quat::from_rotation_z(yaw), *g), tint);
            }
        }
        if let Tool::Run { def, gates } = &self.tool
            && let Some(d) = defs.runs.get(*def)
        {
            let id = get(&mut self.meshes, &d.mesh);
            for g in gates.iter().chain(self.hover.iter()) {
                self.batches.push_sprite(id, Mat4::from_translation(*g), Vec4::new(0.8, 1.3, 0.8, 1.0));
            }
        }
        let parks: Vec<(Id, usize, Vec3, Vec3)> = self.world.parks.iter().map(|p| (p.id, p.def, p.start, p.end)).collect();
        for (pid, def, a, b) in parks {
            let sel = self.selection == Some(Selection::Park(pid));
            let tint = if sel { Vec4::new(1.3, 1.3, 0.7, 1.0) } else { Vec4::ONE };
            for (id, m) in self.park_layout(gpu, scene, def, a, b, snow(a.z)) {
                self.batches.push_sprite(id, m, tint);
            }
        }
        if let (Tool::Park { def, start: Some(st) }, Some(h)) = (self.tool.clone(), self.hover) {
            for (id, m) in self.park_layout(gpu, scene, def, st, h, snow(st.z)) {
                self.batches.push_sprite(id, m, Vec4::ONE);
            }
        }
        let get = |cache: &mut MeshCache, rel: &str| cache.get(gpu, &scene.meshes, &assets, rel, 0);
        for d in &self.world.decos {
            let def = &defs.decos[d.def];
            let id = get(&mut self.meshes, def.meshes.pick(night, snow(d.pos.z)));
            self.batches
                .push_sprite(id, Mat4::from_rotation_translation(Quat::from_rotation_z(d.rot), d.pos), Vec4::ONE);
        }

        // People.
        for g in &self.world.guests {
            if g.pos.z < -100.0 || !g.is_active() {
                continue;
            }
            // Chairlift riders are part of the chair meshes.
            if let GuestState::Riding { lift } = g.state {
                if self.world.lift(lift).is_some_and(|l| !defs.lifts[l.def].show_skiers) {
                    continue;
                }
            }
            // A rescued guest rides the patroller's sled, lying down, or
            // is out of sight in the snowmobile, the chopper, or the base.
            if g.state == GuestState::Rescued {
                let carrier = self.world.staff.iter().find(|s| s.task == crate::game::world::StaffTask::Carry(g.id));
                let Some(c) = carrier else { continue };
                let vehicle = c.home.and_then(|h| self.world.structure(h)).is_some_and(|b| {
                    matches!(defs.structures[b.def].kind, StructureKind::SnowmobilePatrol | StructureKind::ChopperPatrol)
                });
                if vehicle {
                    continue;
                }
                // The sled hangs from the patroller, reaching 6.8 back,
                // its runners 1.64 below its origin.
                let m = Mat4::from_rotation_translation(Quat::from_rotation_z(c.heading), c.pos + Vec3::Z * 1.64);
                let sled = get(&mut self.meshes, "data/scenery/people/ski_patrol/sled/sled.mesh");
                self.batches.push(sled, m, Vec4::ONE);
            }
            if let GuestState::Dying { t } = g.state {
                // The original swaps the skier for its angel (0x49693e).
                let rel = "data/scenery/people/angel/angel.MESH";
                let first = get(&mut self.meshes, rel);
                let n = self.meshes.frames(first).max(1);
                let id = self.meshes.get(gpu, &scene.meshes, &assets, rel, (t * 10.0) as usize % n);
                self.batches.push(id, Mat4::from_rotation_translation(Quat::from_rotation_z(g.heading), g.pos), Vec4::ONE);
                continue;
            }
            let base = guest_mesh(g);
            // Bodies face +x. Skinned people are baked per timeline frame
            // with their parts (poles, skis, boards) attached.
            // A steered skier leans with the snow (see `control`).
            let rot = match g.tilt {
                Some(q) if g.state == GuestState::Controlled => q,
                _ => Quat::from_rotation_z(g.heading),
            };
            let model = Mat4::from_scale_rotation_translation(Vec3::ONE, rot, g.pos);
            let sel = self.selection == Some(Selection::Guest(g.id));
            let tint = Vec4::ONE;
            let name = guest_clip(g);
            let clip_name = if g.boarder { boarder_clip(name) } else { name.to_string() };
            let frame = self
                .meshes
                .clip(gpu, &assets, &base, &clip_name)
                .map_or(0, |c| {
                    // Moving clips repeat for as long as the state lasts; a
                    // steered skier's play as the original plays them.
                    let steered = g.state == GuestState::Controlled;
                    let c = crate::formats::mesh::Clip { looping: c.looping || (name != "YardSale" && !steered), ..c };
                    c.frame_at(g.anim)
                });
            let body = self.meshes.get(gpu, &scene.meshes, &assets, &base, frame as usize);
            self.batches.push(body, model, tint);
            if sel {
                self.highlights.extend(highlight(&self.meshes, body, model, false));
            }
        }
        for s in &self.world.staff {
            let sel = self.selection == Some(Selection::Staff(s.id));
            let m = Mat4::from_scale_rotation_translation(
                Vec3::ONE,
                Quat::from_rotation_z(s.heading),
                s.pos,
            );
            // Patrols from a snowmobile or chopper base go in their
            // vehicle (0x47f890 sits the patroller on the snowmobile's
            // `sit` point; 0x477240's class flies the helicopter).
            let base = s.home.and_then(|h| self.world.structure(h)).map(|b| defs.structures[b.def].kind);
            // Avalanche control rides its own snowmobile too (0x475260; drawn
            // by 0x476860 like the patrol's), getting off to work the
            // mortar. Held up by the player, anyone is just the body.
            let vehicle = match (s.kind, base) {
                _ if s.task == crate::game::world::StaffTask::Held => None,
                (StaffKind::Patrol, Some(StructureKind::SnowmobilePatrol)) => Some(("data/scenery/people/sp_snowmobile/sp_snowmobile.mesh", true)),
                (StaffKind::Patrol, Some(StructureKind::ChopperPatrol)) => Some(("data/scenery/people/helicopter/sp_helicopter.mesh", false)),
                (StaffKind::AvalancheControl, _) => {
                    let off = matches!(s.task, crate::game::world::StaffTask::Mortar { stage, .. } if !matches!(stage, crate::game::world::MortarStage::Walk));
                    Some(("data/scenery/people/ac_snowmobile/ac_snowmobile.mesh", !off))
                }
                _ => None,
            };
            let mut rider = m;
            let mut clip = s.clip().to_string();
            // Instructors dress as the patrol at a ski school, as a blue
            // boarder at any other (0x482dc0 picks class 6 or 7 by the
            // school's name; 0x47eaf0).
            let mut rel = s.kind.mesh();
            if s.kind == StaffKind::Instructor {
                let board = s.home.and_then(|h| self.world.structure(h)).is_some_and(|b| !defs.structures[b.def].unique_name.eq_ignore_ascii_case("Ski School"));
                if board {
                    rel = "data/scenery/people/boarders/boarder_blue.mesh";
                    clip = super::boarder_clip(&clip);
                } else {
                    rel = StaffKind::Patrol.mesh();
                }
            }
            if let Some((vrel, seated)) = vehicle {
                let id = get(&mut self.meshes, vrel);
                let dismounted = !seated && s.kind == StaffKind::AvalancheControl;
                // Parked a few steps behind its rider.
                let vm = if dismounted { m * Mat4::from_translation(Vec3::new(-6.0, 0.0, 0.0)) } else { m };
                self.batches.push(id, vm, Vec4::ONE);
                if sel {
                    self.highlights.extend(highlight(&self.meshes, id, vm, false));
                }
                if dismounted {
                    // The operator stands on the ground by it.
                } else if !seated {
                    continue;
                } else {
                    let sit = self
                        .meshes
                        .source(gpu, &assets, vrel)
                        .and_then(|src| attach_point(&src.mesh.attach, AttachPoint::Sit))
                        .unwrap_or(Vec3::ZERO);
                    rider = m * Mat4::from_translation(sit);
                    clip = "Ride".into();
                }
            }
            let frame = self.meshes.clip(gpu, &assets, rel, &clip).map_or(0, |c| {
                let c = crate::formats::mesh::Clip { looping: true, ..c };
                c.frame_at(s.anim)
            });
            let id = self.meshes.get(gpu, &scene.meshes, &assets, rel, frame as usize);
            self.batches.push(id, rider, Vec4::ONE);
            if sel && vehicle.is_none() {
                self.highlights.extend(highlight(&self.meshes, id, m, false));
            }
        }
        if let Some(cloud) = self.meshes.billboard(gpu, &scene.meshes, &assets, "data/textures/particles/snowcloud.tga") {
            // Face the camera: the quad's x/z plane spans the view's right
            // and up directions.
            let eye = self.camera.eye();
            for p in &self.puffs {
                let toward = (eye - p.pos).normalize_or(Vec3::Y);
                let right = toward.cross(Vec3::Z).normalize_or(Vec3::X);
                let up = right.cross(toward);
                let s = p.size();
                let m = Mat4::from_cols((right * s).extend(0.0), toward.extend(0.0), (up * s).extend(0.0), p.pos.extend(1.0));
                self.batches.push(cloud, m, Vec4::ONE);
            }
        }
        // Sasquatches and rams, when they show themselves.
        for a in &self.world.animals {
            let Some(clip) = a.clip else { continue };
            let frame = self.meshes.clip(gpu, &assets, a.mesh(), clip).map_or(0, |c| {
                let c = crate::formats::mesh::Clip { looping: true, ..c };
                c.frame_at(a.anim)
            });
            let id = self.meshes.get(gpu, &scene.meshes, &assets, a.mesh(), frame as usize);
            self.batches.push(id, Mat4::from_rotation_translation(Quat::from_rotation_z(a.heading), a.pos), Vec4::ONE);
        }
        let all = self.collect_lamps();
        self.light_ground(gpu, scene, &all, &occluders);
    }

    /// Works out the lamp light on the ground, shadowed by the buildings,
    /// when the lamps or buildings have changed.
    fn light_ground(&mut self, gpu: &Gpu, scene: &mut SceneRenderer, lamps: &[crate::render::Lamp], occluders: &[(MeshId, Mat4)]) {
        // FNV-1a over everything the result depends on; the lamps come
        // nearest the view first, so theirs are summed to ignore order.
        let fnv = |xs: &mut dyn Iterator<Item = u32>| {
            xs.fold(0xcbf2_9ce4_8422_2325u64, |k, x| (k ^ x as u64).wrapping_mul(0x0100_0000_01b3))
        };
        let mut key = lamps.iter().fold(lamps.len() as u64 + 1, |k, l| {
            let mut v = l.pos_outer.iter().chain(&l.color_inner).chain(&l.spot).chain(&l.cones).map(|v| v.to_bits());
            k.wrapping_add(fnv(&mut v))
        });
        for (id, m) in occluders {
            let mut v = std::iter::once(*id as u32).chain(m.to_cols_array().into_iter().map(f32::to_bits));
            key = key.rotate_left(5) ^ fnv(&mut v);
        }
        if key == self.ground_lamps_key {
            return;
        }
        self.ground_lamps_key = key;
        let solids: Vec<crate::game::lamplight::Occluder> = occluders
            .iter()
            .filter_map(|(id, model)| {
                Some(crate::game::lamplight::Occluder {
                    solid: self.meshes.solid(*id)?,
                    model: *model,
                })
            })
            .collect();
        let t = std::time::Instant::now();
        let light = crate::game::lamplight::ground_light(self.world.level(), lamps, &solids);
        log::debug!("ground lamp light: {} lamps, {} solids, {:?}", lamps.len(), solids.len(), t.elapsed());
        scene.terrain.set_ground_lamps(gpu, &light);
    }

    /// The lights of the night meshes being drawn, at an eighth of their
    /// color as the original adds them (0x49daa0), nearest the view first.
    /// The nearest few shade meshes; all of them are returned.
    pub(super) fn collect_lamps(&mut self) -> Vec<crate::render::Lamp> {
        self.lamps.clear();
        if !self.world.is_night() {
            return Vec::new();
        }
        let focus = self.camera.target;
        let mut all: Vec<(f32, crate::render::Lamp)> = Vec::new();
        for (id, list) in &self.batches.lists {
            let Some(lights) = self.meshes.lights.get(id) else { continue };
            for inst in list {
                let model = Mat4::from_cols_array_2d(&inst.model);
                let scale = if self.batches.lift_meshes.contains(id) { 0.125 } else { 1.0 };
                for l in lights.iter() {
                    let p = model.transform_point3(l.pos);
                    let c = l.color * scale;
                    let (spot, cones) = match l.spot {
                        Some(s) => (model.transform_vector3(s.dir).normalize_or_zero().extend(1.0), [s.outer, s.inner, 0.0, 0.0]),
                        None => (Vec4::ZERO, [0.0; 4]),
                    };
                    all.push((
                        p.distance_squared(focus),
                        crate::render::Lamp {
                            pos_outer: p.extend(l.outer).into(),
                            color_inner: c.extend(l.inner).into(),
                            spot: spot.into(),
                            cones,
                        },
                    ));
                }
            }
        }
        all.sort_by(|a, b| a.0.total_cmp(&b.0));
        self.lamps.extend(all.iter().take(crate::render::MAX_LAMPS).map(|x| x.1));
        all.into_iter().map(|x| x.1).collect()
    }

    /// Sky, sun and fog for the current time of day and weather.
    pub fn environment(&self, g: &mut Globals) {
        use daylight::*;
        let h = self.world.hour;
        let cycle = &self.world.daylight;
        let l = cycle.light();
        // The scene lights (0x427990): a fixed sun and an ambient, stepping
        // between their day and night values over dusk and dawn.
        let to_sun = -Vec3::from(SUN_DIR).normalize();
        g.sun_dir = to_sun.extend(0.0).into();
        g.sun_color = Vec3::splat(SUN_NIGHT + (SUN_DAY - SUN_NIGHT) * l).extend(1.0).into();
        let ambient = Vec3::from(AMBIENT_NIGHT).lerp(Vec3::splat(AMBIENT), l);
        g.ambient = ambient.extend(1.0).into();
        // Sprites carry the day lighting, dimmed as the original dims them
        // over dusk and dawn. At night the original draws the night sprites
        // at their brightest palette (its night entrance matches exactly);
        // those were rendered under the night lights and the models' own
        // lamps, which leaves lit surfaces at about 0.36 of the day ones
        // (measured on the entrance) and the pre-lit windows bright.
        const NIGHT_SPRITE: f32 = 0.36;
        let s = cycle.sprite_light();
        let (bright, prelit) = if s > 0.0 { (s, s) } else { (NIGHT_SPRITE, 1.0) };
        // w: the brightness pre-lit sprite pieces are drawn at.
        g.sprite_sun_dir = to_sun.extend(prelit).into();
        g.sprite_sun_color = Vec3::splat(SUN_DAY * bright).extend(1.0).into();
        g.sprite_ambient = Vec3::splat(AMBIENT * bright).extend(1.0).into();
        // The background (0x433dc0): sRGB (161, 179, 244) by day, going
        // toward (28, 39, 80) as the sprites darken. The original has no
        // distance fog outside its first-person view.
        // In the follow view it fogs from 150 to 2000 units, in a color
        // only half as far toward night (0x432f10), closer in as the fog
        // dialog asks: the original fogs by distance over 100, 200 or 350
        // (0x44af30), seen here as fog ending at three times that.
        let first_person = self.eye.is_some();
        let sight = self.eye.as_ref().map_or(1.0e6, |e| e.sight);
        let k = if first_person { (1.0 - s) * 0.5 } else { 1.0 - s };
        let day = Vec3::new(161.0, 179.0, 244.0);
        let night = Vec3::new(28.0, 39.0, 80.0);
        let bg = day.lerp(night, k) / 255.0;
        let linear = |c: f32| if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
        g.fog_color = Vec3::new(linear(bg.x), linear(bg.y), linear(bg.z)).extend(1.0).into();
        let grid = if self.show_grid { self.world.level().spacing } else { 0.0 };
        g.params = if first_person {
            let end = (sight * 3.0).min(2000.0);
            [(150.0f32).min(end * 0.5), end, h, grid]
        } else {
            [1.0e6, 2.0e6, h, grid]
        };
        let plain = if self.settings.detail == 0 { 1.0 } else { 0.0 };
        g.lamp_count = [self.lamps.len() as f32, plain, 0.0, 0.0];
        g.lamps[..self.lamps.len()].copy_from_slice(&self.lamps);
    }
}
