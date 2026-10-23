//! Building, upgrading, hiring and demolishing.

use super::*;

impl Game {
    pub fn footprint_ok(&self, def: usize, cell: (i32, i32), rot: u8) -> (bool, Option<Problem>, f32) {
        let (w, h) = {
            let (a, b) = self.defs.structures[def].size;
            if rot % 2 == 0 { (a, b) } else { (b, a) }
        };
        let level = self.world.level();
        let n = level.cells_per_side() as i32;
        if cell.0 < 1 || cell.1 < 1 || cell.0 + w >= n || cell.1 + h >= n {
            return (false, Some(Problem::OffMap), 0.0);
        }
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for y in cell.1..=cell.1 + h {
            for x in cell.0..=cell.0 + w {
                let z = level.height(x as usize, y as usize);
                lo = lo.min(z);
                hi = hi.max(z);
            }
        }
        for y in cell.1..cell.1 + h {
            for x in cell.0..cell.0 + w {
                let i = (y * n + x) as usize;
                if self.world.occupied[i].is_some() || self.world.lift_cells.get(i).is_some_and(|c| c.is_some()) {
                    return (false, Some(Problem::Occupied), 0.0);
                }
            }
        }
        let span = (w.max(h) as f32) * level.spacing;
        let slope = ((hi - lo) / span).atan().to_degrees();
        if slope > 30.0 {
            return (false, Some(Problem::TooSteep), slope);
        }
        (true, None, slope)
    }

    /// Whether everything in a `predecessors` list has been built.
    pub fn unlocked(&self, reqs: &[Requirement]) -> bool {
        self.settings.all_buildings || reqs.iter().all(|r| self.built(r))
    }

    pub(super) fn built(&self, r: &Requirement) -> bool {
        self.world.has_built(&self.defs, r)
    }

    pub fn build_structure(&mut self, def: usize, cell: (i32, i32), rot: u8, free: bool) -> bool {
        let (ok, reason, _) = self.footprint_ok(def, cell, rot);
        let d = self.defs.structures[def].clone();
        if !ok {
            if let Some(r) = reason {
                self.error(r);
            }
            return false;
        }
        if !free && self.world.cash < d.cost {
            self.error(Problem::TooExpensive);
            return false;
        }
        self.place_structure(def, cell, rot, free);
        true
    }

    /// Upgrades a building into the one its definition names (0x46ed20):
    /// it costs that building's price, and the building keeps its place,
    /// name and state. Returns false when it can't be done or afforded.
    pub fn upgrade_structure(&mut self, id: Id) -> bool {
        let Some(st) = self.world.structure(id).cloned() else { return false };
        let Some(up) = self.defs.structures[st.def].upgrade else { return false };
        let d = &self.defs.structures[up];
        if self.world.cash < d.cost {
            self.error(Problem::TooExpensive);
            return false;
        }
        self.world.spend(Ledger::Construction, d.cost);
        // Re-centered on the old footprint: the upgrades can be smaller.
        let (w, h) = if st.rot % 2 == 0 { d.size } else { (d.size.1, d.size.0) };
        let s = self.world.level().spacing;
        let cell = (
            (st.pos.x / s - w as f32 * 0.5).round() as i32,
            (st.pos.y / s - h as f32 * 0.5).round() as i32,
        );
        let n = self.world.level().cells_per_side() as i32;
        for o in self.world.occupied.iter_mut().filter(|o| **o == Some(id)) {
            *o = None;
        }
        for y in cell.1.max(0)..(cell.1 + h).min(n) {
            for x in cell.0.max(0)..(cell.0 + w).min(n) {
                self.world.occupied[(y * n + x) as usize] = Some(id);
            }
        }
        let pos = Vec3::new((cell.0 as f32 + w as f32 * 0.5) * s, (cell.1 as f32 + h as f32 * 0.5) * s, st.pos.z);
        if let Some(m) = self.world.structure_mut(id) {
            m.def = up;
            m.cell = cell;
            m.pos = pos;
            m.worth.add(d.cost);
            m.wear = 0.0;
        }
        self.sfx(Sound::Placing);
        true
    }

    /// Puts a structure down without checking the site.
    pub(super) fn place_structure(&mut self, def: usize, cell: (i32, i32), rot: u8, free: bool) {
        let d = self.defs.structures[def].clone();
        // The first groomer or snowmaker shop opens up the snow tools
        // (0x4b5f14).
        let snow_shop = |k: StructureKind| matches!(k, StructureKind::Groomer | StructureKind::Snowmaker);
        if snow_shop(d.kind) && !self.world.structures.iter().any(|st| snow_shop(self.defs.structures[st.def].kind)) {
            let text = self.defs.text(Text::ReNewTerrainOptions).to_string();
            self.world.message(text, None);
        }
        let (w, h) = if rot % 2 == 0 { d.size } else { (d.size.1, d.size.0) };
        let s = self.world.level().spacing;
        // Flatten the footprint to its average height.
        let mut sum = 0.0;
        let mut cnt = 0.0;
        for y in cell.1..=cell.1 + h {
            for x in cell.0..=cell.0 + w {
                sum += self.world.level().height(x as usize, y as usize);
                cnt += 1.0;
            }
        }
        let z = sum / cnt;
        for y in cell.1..=cell.1 + h {
            for x in cell.0..=cell.0 + w {
                self.world.level_mut().set_height(x as usize, y as usize, z);
            }
        }
        let id = self.world.new_id();
        let n = self.world.level().cells_per_side() as i32;
        for y in cell.1..cell.1 + h {
            for x in cell.0..cell.0 + w {
                self.world.occupied[(y * n + x) as usize] = Some(id);
                let c = self.world.level_mut().cell_mut(x as usize, y as usize);
                c.set_foliage(0);
            }
        }
        let pos = Vec3::new(
            (cell.0 as f32 + w as f32 * 0.5) * s,
            (cell.1 as f32 + h as f32 * 0.5) * s,
            z,
        );
        let count = self.world.structures.iter().filter(|x| x.def == def).count();
        let base_name = self.defs.structure_name(&d).to_string();
        // The entrance carries the resort's name ("Ski Resort" until
        // renamed), shown as the resort window's title.
        let name = if d.category == Category::Entrance {
            self.defs.text(Text::ResortName).to_string()
        } else {
            format!("{base_name} {}", count + 1)
        };
        // The price starts at `item_cost` (0x4b4ca0 copies it to +0x8f0).
        let price = d.item_cost.max(d.item_cost_min).max(0.0);
        self.world.structures.push(Structure {
            id,
            def,
            cell,
            rot,
            pos,
            name,
            wear: 0.0,
            open: true,
            price,
            powered: true,
            occupants: Vec::new(),
            visits_today: 0,
            revenue_today: 0.0,
            revenue_total: 0.0,
            built_day: self.world.day,
            broken: false,
            worth: Worth::new(d.cost),
            daily_fee: None,
            class_opened: None,
            waiting: Vec::new(),
        });
        if !free {
            self.world.spend(Ledger::Construction, d.cost);
        }
        // A building that houses staff comes with one of them (0x482dc0
        // from 0x4b5f06); more are hired from its window, up to its
        // staff count.
        if let Some(k) = d.kind.staff() {
            if !free && d.staff_count > 0 {
                self.hire(k, Some(pos), Some(id));
            }
        }
        // The groomer shop brings a grooming and a snowmaking machine
        // (0x4b5fe1, 0x4b601a).
        if d.kind == StructureKind::Groomer && !free {
            self.hire(StaffKind::Groomer, Some(pos), Some(id));
            self.hire(StaffKind::Snowmaker, Some(pos), Some(id));
        }
        self.terrain_dirty = true;
        self.foliage_dirty = true;
    }

    /// Who is paid what: each staff class's wage method (vtable +0x5c)
    /// pays one of the five wages. Mechanics, security guards and
    /// wildlife control share the maintenance wage (0x4794e0); avalanche
    /// control (0x4814e0) and the chopper patrol (0x477fb0) the chopper
    /// wage; ski and snowmobile patrols the patrol wage (0x47f3a0).
    /// Groomers are vehicles, never paid. `home` is the staff member's
    /// building.
    pub(super) fn wage_of(&self, kind: StaffKind, home: Option<Id>) -> f32 {
        let t = &self.defs.staff;
        let chopper = home
            .and_then(|h| self.world.structure(h))
            .is_some_and(|s| self.defs.structures[s.def].kind == StructureKind::ChopperPatrol);
        match kind {
            StaffKind::Patrol if chopper => t.chopper_wages,
            StaffKind::Patrol => t.patrol_wages,
            StaffKind::Maintenance | StaffKind::Security | StaffKind::WildlifeControl => t.maintenance_wages,
            StaffKind::Sanitation => t.sanitation_wages,
            StaffKind::Instructor => t.instructor_wages,
            StaffKind::AvalancheControl | StaffKind::SnowTour => t.chopper_wages,
            StaffKind::Groomer | StaffKind::Snowmaker => 0.0,
        }
    }

    /// Whether a building may take on another of its staff: fewer than
    /// its staff count, plus one for every staff housing (0x46fee2:
    /// +0xf8 plus the staff housing count, 0x69fd60 by type).
    pub fn can_hire(&self, building: Id) -> bool {
        let Some(st) = self.world.structure(building) else { return false };
        let d = &self.defs.structures[st.def];
        let Some(kind) = d.kind.staff() else { return false };
        let housing = self.world.structures.iter().filter(|s| self.defs.structures[s.def].kind == StructureKind::StaffHousing).count() as i32;
        let housed = self.world.staff.iter().filter(|s| s.home == Some(building) && s.kind == kind).count() as i32;
        d.staff_count > 0 && housed < d.staff_count + housing
    }

    pub fn hire(&mut self, kind: StaffKind, at: Option<Vec3>, home: Option<Id>) {
        let wage = self.wage_of(kind, home);
        let pos = at
            .or_else(|| {
                self.world
                    .structures
                    .iter()
                    .find(|s| self.defs.structures[s.def].category == Category::Entrance)
                    .map(|s| s.pos)
            })
            .unwrap_or(self.camera.target);
        let id = self.world.new_id();
        let n = self.world.staff.iter().filter(|s| s.kind == kind).count() + 1;
        self.world.staff.push(Staff {
            id,
            kind,
            name: match kind.name_text() {
                Some(t) => {
                    let t = self.defs.text(t);
                    // The machines go by their text alone (0x43ba90).
                    if matches!(kind, StaffKind::Groomer | StaffKind::Snowmaker) {
                        t.to_string()
                    } else if t.contains("%d") { t.replacen("%d", &n.to_string(), 1) } else { format!("{t}{n}") }
                }
                None => format!("{} #{n}", kind.label()),
            },
            pos,
            heading: 0.0,
            task: StaffTask::Idle,
            wage,
            work_timer: 0.0,
            carrying: None,
            home,
            anim: 0.0,
            moving: false,
            speed: 0.0,
            ski_mode: SkiMode::Pole,
        });
    }

    pub fn fire(&mut self, id: Id) {
        self.world.staff.retain(|s| s.id != id);
        if self.selection == Some(Selection::Staff(id)) {
            self.selection = None;
        }
    }

    pub(super) fn lift_plan(&self, def: usize, start: Vec3, end: Vec3) -> (Vec<Vec3>, f32, Option<Problem>) {
        let d = &self.defs.lifts[def];
        let level = self.world.level();
        let horiz = start.truncate().distance(end.truncate());
        let segments = (horiz / d.pole_spacing.max(10.0)).ceil().max(1.0) as usize;
        let poles: Vec<Vec3> = (1..segments)
            .map(|i| {
                let p = start.truncate().lerp(end.truncate(), i as f32 / segments as f32);
                Vec3::new(p.x, p.y, level.height_at(p.x, p.y))
            })
            .collect();
        let cost = d.base_cost + d.cost_per_pole * poles.len() as f32;
        let mut reason = None;
        if horiz < 60.0 {
            reason = Some(Problem::TooShort);
        } else if poles.len() as i32 > d.max_poles {
            reason = Some(Problem::TooLong);
        } else if end.z <= start.z + 5.0 {
            reason = Some(Problem::WrongWay);
        } else if self.world.cash < cost {
            reason = Some(Problem::TooExpensive);
        }
        if self.lift_marks(def, start, end).iter().any(|(_, m)| *m == CellMark::Blocked) {
            reason = Some(Problem::Occupied);
        }
        (poles, cost, reason)
    }

    pub(super) fn build_lift(&mut self, def: usize, start: Vec3, end: Vec3) -> bool {
        let (poles, cost, reason) = self.lift_plan(def, start, end);
        if let Some(r) = reason {
            self.error(r);
            return false;
        }
        let d = &self.defs.lifts[def];
        let id = self.world.new_id();
        let count = self.world.lifts.iter().filter(|l| l.def == def).count();
        let name = {
            let base = match self.defs.strings.get(&d.name_key) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => d.unique_name.clone(),
            };
            // Numbered from 1, as the original does ("Ropetow 1").
            format!("{base} {}", count + 1)
        };
        let mut lift = Lift {
            id,
            def,
            name,
            start,
            end,
            poles,
            chairs: Vec::new(),
            queue: Default::default(),
            wear: 0.0,
            open: true,
            broken: false,
            riders_today: 0,
            riders_total: 0,
            built_day: self.world.day,
            worth: Worth::new(cost),
            cable_loop: Vec::new(),
            cable_up: 0.0,
            anim: 0.0,
            queue_lines: Vec::new(),
            exit_line: Vec::new(),
            board_s: None,
            unload_s: None,
        };
        Sim::layout_chairs(&self.defs, &mut lift);
        // Stations block their cells and clear trees around the line.
        let s = self.world.level().spacing;
        for p in [start, end] {
            if let Some(i) = self.world.cell_index((p.x / s) as i32, (p.y / s) as i32) {
                self.world.occupied[i] = Some(id);
            }
        }
        let steps = (start.truncate().distance(end.truncate()) / (s * 0.5)) as usize + 1;
        let n = self.world.level().cells_per_side();
        for k in 0..=steps {
            let p = start.truncate().lerp(end.truncate(), k as f32 / steps as f32);
            let (cx, cy) = ((p.x / s) as usize, (p.y / s) as usize);
            if cx < n && cy < n {
                self.world.level_mut().cell_mut(cx, cy).set_foliage(0);
            }
        }
        self.world.lifts.push(lift);
        self.refresh_lifts();
        self.world.spend(Ledger::Construction, cost);
        self.foliage_dirty = true;
        self.sim().update_capacity();
        true
    }

    /// What the next run of a kind is called, "Beginner Run 1" and so on,
    /// numbered within its kind as the original does.
    pub(super) fn next_run_name(&self, def: usize) -> String {
        let n = self.world.runs.iter().filter(|r| r.def == def).count() + 1;
        let label = match self.defs.strings.get(&self.defs.runs[def].name_key) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => self.defs.runs[def].unique_name.clone(),
        };
        format!("{label} {n}")
    }

    pub(super) fn build_run(&mut self, def: usize, gates: Vec<Vec3>) -> bool {
        if gates.len() < 2 {
            self.error(Problem::TooFewGates);
            return false;
        }
        let cost = self.defs.runs[def].cost * gates.len() as f32;
        if self.world.cash < cost {
            self.error(Problem::TooExpensive);
            return false;
        }
        let id = self.world.new_id();
        let name = self.next_run_name(def);
        self.world.runs.push(Run {
            id,
            def,
            name,
            gates,
            skiers_today: 0,
            open: true,
            worth: Worth::new(cost),
        });
        self.world.spend(Ledger::Construction, cost);
        true
    }

    pub(super) fn park_check(&self, def: usize, start: Vec3, end: Vec3) -> (f32, Option<Problem>) {
        let d = &self.defs.parks[def];
        let len = start.truncate().distance(end.truncate());
        let reason = if len < 40.0 {
            Some(Problem::TooShort)
        } else if len > 260.0 {
            Some(Problem::TooLong)
        } else if end.z > start.z - 3.0 {
            Some(Problem::WrongWay)
        } else if self.world.cash < d.cost {
            Some(Problem::TooExpensive)
        } else {
            None
        };
        (d.cost, reason)
    }

    pub(super) fn build_park(&mut self, def: usize, start: Vec3, end: Vec3) -> bool {
        let (cost, reason) = self.park_check(def, start, end);
        if let Some(r) = reason {
            self.error(r);
            return false;
        }
        let d = &self.defs.parks[def];
        let name = match self.defs.strings.get(&d.name_key) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => d.unique_name.clone(),
        };
        let id = self.world.new_id();
        self.world.parks.push(Park {
            id,
            def,
            name,
            start,
            end,
            riders_today: 0,
            riders_total: 0,
            open: true,
            worth: Worth::new(cost),
        });
        // Clear trees along the course.
        let s = self.world.level().spacing;
        let n = self.world.level().cells_per_side();
        let steps = (start.truncate().distance(end.truncate()) / (s * 0.5)) as usize + 1;
        for k in 0..=steps {
            let p = start.truncate().lerp(end.truncate(), k as f32 / steps as f32);
            for dx in -1..=1i32 {
                for dy in -1..=1i32 {
                    let (cx, cy) = ((p.x / s) as i32 + dx, (p.y / s) as i32 + dy);
                    if cx >= 0 && cy >= 0 && (cx as usize) < n && (cy as usize) < n {
                        self.world.level_mut().cell_mut(cx as usize, cy as usize).set_foliage(0);
                    }
                }
            }
        }
        self.world.spend(Ledger::Construction, cost);
        self.foliage_dirty = true;
        true
    }

    /// Lays out the pieces of a park course along its line.
    pub fn park_layout(&mut self, gpu: &Gpu, scene: &SceneRenderer, def: usize, start: Vec3, end: Vec3, snow: u8) -> Vec<(MeshId, Mat4)> {
        let d = self.defs.parks[def].clone();
        let assets = self.assets.clone();
        let dir = (end - start).truncate();
        let len = dir.length().max(1.0);
        let yaw = dir.y.atan2(dir.x) - std::f32::consts::FRAC_PI_2;
        let rot = Quat::from_rotation_z(yaw);
        let mut out = Vec::new();
        let mut place = |cache: &mut MeshCache, k: usize, t: f32| {
            let id = cache.get(gpu, &scene.meshes, &assets, d.piece(snow, k), 0);
            let p = start.truncate().lerp(end.truncate(), t);
            let z = self.world.level().height_at(p.x, p.y);
            out.push((id, Mat4::from_rotation_translation(rot, Vec3::new(p.x, p.y, z))));
        };
        place(&mut self.meshes, 0, 0.0);
        place(&mut self.meshes, 1, 12.0 / len);
        let tiles = ((len - 50.0) / 14.0).max(0.0) as usize;
        for i in 0..tiles {
            place(&mut self.meshes, 2, (24.0 + i as f32 * 14.0) / len);
        }
        place(&mut self.meshes, 3, ((len - 22.0) / len).max(0.2));
        place(&mut self.meshes, 4, 1.0);
        out
    }

    /// Gives a selected object a new name.
    pub fn rename(&mut self, sel: &Selection, name: String) {
        // The name box of the guest and resort windows (0x4652c0) has
        // three names that move money, as operations, floating off the
        // renamed thing.
        let resort = match sel {
            Selection::Structure(id) => self
                .world
                .structure(*id)
                .is_some_and(|s| self.defs.structures[s.def].category == Category::Entrance),
            _ => false,
        };
        if resort || matches!(sel, Selection::Guest(_)) {
            let at = match sel {
                Selection::Guest(id) => self.world.guest(*id).map(|g| g.pos),
                Selection::Structure(id) => self.world.structure(*id).map(|s| s.pos),
                _ => None,
            };
            let money = match name.as_str() {
                "Brad" => 100.0,
                "Melissa Ann" => -1000.0,
                "Ryan" => 630.0,
                _ => 0.0,
            };
            if money != 0.0 {
                if money > 0.0 {
                    self.world.earn(Ledger::Operations, money);
                } else {
                    self.world.spend(Ledger::Operations, -money);
                }
                if let Some(at) = at {
                    let sign = if money > 0.0 { "+" } else { "-" };
                    self.floaters.push(Floater {
                        at,
                        text: format!("{sign}{}", crate::ui::style::money(money.abs())),
                        age: 0.0,
                    });
                }
            }
        }
        let w = &mut self.world;
        match sel {
            Selection::Structure(id) => {
                if let Some(s) = w.structure_mut(*id) {
                    s.name = name;
                }
            }
            Selection::Lift(id) => {
                if let Some(l) = w.lift_mut(*id) {
                    l.name = name;
                }
            }
            Selection::Run(id) => {
                if let Some(r) = w.runs.iter_mut().find(|r| r.id == *id) {
                    r.name = name;
                }
            }
            Selection::Park(id) => {
                if let Some(p) = w.parks.iter_mut().find(|p| p.id == *id) {
                    p.name = name;
                }
            }
            Selection::Guest(id) => {
                if let Some(x) = w.guests.iter_mut().find(|x| x.id == *id) {
                    x.name = name;
                }
            }
            Selection::Staff(id) => {
                if let Some(s) = w.staff.iter_mut().find(|s| s.id == *id) {
                    s.name = name;
                }
            }
        }
    }

    pub fn demolish(&mut self, sel: &Selection) {
        let before = (self.world.parks.len(), self.world.structures.len(), self.world.lifts.len(), self.world.runs.len());
        self.demolish_inner(sel);
        if before != (self.world.parks.len(), self.world.structures.len(), self.world.lifts.len(), self.world.runs.len()) {
            self.sfx(Sound::Destroy);
        }
    }

    /// Pulls something down, paying back what it is worth (0x4b2cae for
    /// buildings, 0x4a3743 for lifts).
    pub(super) fn demolish_inner(&mut self, sel: &Selection) {
        match sel {
            Selection::Park(id) => {
                if let Some(i) = self.world.parks.iter().position(|p| p.id == *id) {
                    let p = self.world.parks.remove(i);
                    self.world.earn(Ledger::Construction, p.worth.now);
                }
            }
            Selection::Structure(id) => {
                let Some(i) = self.world.structures.iter().position(|s| s.id == *id) else { return };
                if self.defs.structures[self.world.structures[i].def].category == Category::Entrance {
                    self.error(Problem::Entrance);
                    return;
                }
                let s = self.world.structures.remove(i);
                for o in self.world.occupied.iter_mut() {
                    if *o == Some(*id) {
                        *o = None;
                    }
                }
                for g in &mut self.world.guests {
                    if let GuestState::InStructure { structure, .. } = g.state {
                        if structure == *id {
                            g.state = GuestState::Walking {
                                to: s.pos.truncate(),
                                purpose: Purpose::Wander,
                            };
                            g.pos = s.pos;
                        }
                    }
                }
                self.world.earn(Ledger::Construction, s.worth.now);
            }
            Selection::Lift(id) => {
                let Some(i) = self.world.lifts.iter().position(|l| l.id == *id) else { return };
                let l = self.world.lifts.remove(i);
                for o in self.world.occupied.iter_mut() {
                    if *o == Some(*id) {
                        *o = None;
                    }
                }
                self.refresh_lifts();
                let riders: Vec<Id> = l
                    .queue
                    .iter()
                    .copied()
                    .chain(l.chairs.iter().flat_map(|c| c.riders.iter().copied()))
                    .collect();
                for g in &mut self.world.guests {
                    if riders.contains(&g.id) {
                        g.pos = self.world.level.as_ref().map(|lv| {
                            Vec3::new(l.start.x, l.start.y, lv.height_at(l.start.x, l.start.y))
                        }).unwrap_or(l.start);
                        g.state = GuestState::Walking {
                            to: l.start.truncate(),
                            purpose: Purpose::Wander,
                        };
                    }
                }
                self.world.earn(Ledger::Construction, l.worth.now);
            }
            Selection::Run(id) => {
                if let Some(i) = self.world.runs.iter().position(|r| r.id == *id) {
                    let r = self.world.runs.remove(i);
                    self.world.earn(Ledger::Construction, r.worth.now);
                }
            }
            Selection::Staff(id) => self.fire(*id),
            Selection::Guest(_) => {}
        }
        self.selection = None;
        self.terrain_dirty = true;
        self.foliage_dirty = true;
    }
}
