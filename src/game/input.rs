//! The per-frame update: keys, mouse, clicks and the simulation clock.

use super::*;

impl Game {
    pub fn update(&mut self, dt: f32, input: &Input, wants_pointer: bool, wants_keys: bool, size: Vec2, _ui: &crate::ui::Ui) {
        self.camera.aspect = size.x / size.y.max(1.0);
        for f in &mut self.floaters {
            f.age += dt;
        }
        self.floaters.retain(|f| f.age < FLOATER_SECS);
        if self.eye.is_some() {
            // The follow view takes only its own keys (0x432aa0).
            self.eye_update(dt, input, !wants_keys);
            self.hover = None;
            self.placement = None;
        } else {
            if !wants_keys {
                self.keyboard(dt, input);
            }
            self.edge_scroll(dt, input, size);
            if !wants_pointer {
                self.mouse(input, size);
            } else {
                self.hover = None;
                self.placement = None;
            }
            self.camera.animate(dt);
        }
        let ortho = self.eye.is_none();
        if ortho && !self.camera.ortho {
            // Back from the skier's view to the overview's zoom.
            self.camera.show_zoom();
        }
        self.camera.ortho = ortho;
        if let Some((_, t)) = &mut self.last_error {
            *t -= dt;
            if *t <= 0.0 {
                self.last_error = None;
            }
        }
        // Keep the camera over the map and above ground.
        let ext = self.world.level().extent();
        if self.eye.is_none() {
            self.camera.target.x = self.camera.target.x.clamp(0.0, ext);
            self.camera.target.y = self.camera.target.y.clamp(0.0, ext);
            let gz = self.world.level().height_at(self.camera.target.x, self.camera.target.y);
            self.camera.target.z += (gz - self.camera.target.z) * (dt * 6.0).min(1.0);
        }
        let eye = self.camera.eye();
        let ez = self.world.level().height_at(eye.x, eye.y) + 8.0;
        let inside = eye.x > 0.0 && eye.y > 0.0 && eye.x < ext && eye.y < ext;
        if inside && eye.z < ez && !self.camera.ortho && self.eye.is_none() {
            self.camera.pitch = (self.camera.pitch + 0.02).min(1.45);
        }

        self.puffs(dt);
        self.building_sounds(dt, size);
        if self.control.is_none() {
            self.stop_ski_sound();
        }
        if !self.world.wildlife_spawned {
            self.world.wildlife_spawned = true;
            // The original spawns them only with the box unticked
            // (0x41c1f8); here the box turns them on, as its label says.
            // The sasquatch challenge (number 6) always has them, as it
            // did with the original's default settings.
            let challenge = self.world.challenge_state.as_ref().is_some_and(|c| c.number == 6);
            let sasquatches = self.settings.evil_sasquatches || challenge;
            self.sim().spawn_wildlife(sasquatches);
        }
        self.world.controlled = self.control.as_ref().map(|c| c.guest);
        for (s, at) in std::mem::take(&mut self.world.sounds) {
            if at.is_none_or(|p| self.on_screen(p, size)) {
                self.sfx(s);
            }
        }
        // Groomers and snow makers drone while on screen (0x43c3d0).
        let groomers: Vec<(Id, Vec3)> = self.world.staff.iter().filter(|s| matches!(s.kind, StaffKind::Groomer | StaffKind::Snowmaker)).map(|s| (s.id, s.pos)).collect();
        self.groomer_loops.retain(|id, h| {
            let keep = groomers.iter().any(|g| g.0 == *id);
            if !keep {
                crate::audio::stop(*h);
            }
            keep
        });
        for (id, pos) in groomers {
            let seen = self.on_screen(pos, size);
            match (seen, self.groomer_loops.get(&id).copied()) {
                (true, None) => {
                    if let Some(h) = crate::audio::play_sound(
                        &self.assets,
                        Sound::Groomer,
                        crate::audio::Opts { music: false, looped: true, volume: 0 },
                    ) {
                        self.groomer_loops.insert(id, h);
                    }
                }
                (false, Some(h)) => {
                    crate::audio::stop(h);
                    self.groomer_loops.remove(&id);
                }
                _ => {}
            }
        }
        // A challenge message waiting for OK stops the world (0x414960's
        // callers skip the tick).
        let held = self.world.challenge_state.as_ref().and_then(|c| c.prompt()).is_some_and(|p| p.pauses);
        if !self.paused && !held {
            self.sim_accum += dt * self.speed;
            let mut steps = 0;
            while self.sim_accum >= sim::TICK && steps < 240 {
                self.sim_accum -= sim::TICK;
                self.world.options = sim::Options {
                    fast_maintenance: self.settings.fast_maintenance,
                    reduce_garbage: self.settings.reduce_garbage,
                    no_nightfall: self.settings.no_nightfall,
                };
                self.sim().step(sim::TICK);
                steps += 1;
            }
            if steps >= 240 {
                self.sim_accum = 0.0;
            }
            if let Some(mut c) = self.world.challenge_state.take() {
                c.update(&self.world, &self.defs, self.show_grid);
                self.world.challenge_state = Some(c);
            }
        }
    }

    /// Scrolls the map when the pointer rests at a window edge, at the
    /// speed set by the "Scroll rate" option.
    pub(super) fn edge_scroll(&mut self, dt: f32, input: &Input, size: Vec2) {
        const EDGE: f32 = 6.0;
        if !input.inside || input.right_down || input.middle_down {
            return;
        }
        let m = input.mouse;
        let mut f = 0.0;
        let mut r = 0.0;
        if m.x <= EDGE {
            r -= 1.0;
        } else if m.x >= size.x - 1.0 - EDGE {
            r += 1.0;
        }
        if m.y <= EDGE {
            f += 1.0;
        } else if m.y >= size.y - 1.0 - EDGE {
            f -= 1.0;
        }
        if f != 0.0 || r != 0.0 {
            let k = self.settings.pan_speed();
            self.camera.pan(f * dt * 600.0 * k, r * dt * 600.0 * k);
        }
    }

    pub(super) fn keyboard(&mut self, dt: f32, input: &Input) {
        let k = |c: KeyCode| input.held(c);
        // The original's debug keys: Ctrl+Shift+4 adds $10,000 a frame,
        // Ctrl+Shift+X sets off an avalanche (0x427048).
        let cheat = (k(KeyCode::ControlLeft) || k(KeyCode::ControlRight)) && (k(KeyCode::ShiftLeft) || k(KeyCode::ShiftRight));
        if cheat && k(KeyCode::Digit4) {
            self.world.cash += 10_000.0;
        }
        if cheat && input.pressed.contains(&KeyCode::KeyX) {
            self.sim().force_avalanche();
        }
        if std::mem::take(&mut self.world.scenery_dirty) {
            self.foliage_dirty = true;
            self.terrain_dirty = true;
        }
        let mut f = 0.0;
        let mut r = 0.0;
        if k(KeyCode::KeyW) {
            f += 1.0;
        }
        if k(KeyCode::KeyS) {
            f -= 1.0;
        }
        if k(KeyCode::KeyD) {
            r += 1.0;
        }
        if k(KeyCode::KeyA) {
            r -= 1.0;
        }
        if f != 0.0 || r != 0.0 {
            let k = self.settings.pan_speed();
            self.camera.pan(f * dt * 600.0 * k, r * dt * 600.0 * k);
        }
        if k(KeyCode::KeyQ) {
            self.camera.rotate(dt * 1.5, 0.0);
        }
        if k(KeyCode::KeyE) {
            self.camera.rotate(-dt * 1.5, 0.0);
        }
        if k(KeyCode::PageUp) {
            self.camera.rotate(0.0, dt);
        }
        if k(KeyCode::PageDown) {
            self.camera.rotate(0.0, -dt);
        }
        for key in &input.pressed {
            match key {
                // Nothing runs in the map editor (0x418df0 has no world
                // update).
                KeyCode::Space | KeyCode::KeyP if !self.world.editor => self.paused = !self.paused,
                KeyCode::Equal | KeyCode::NumpadAdd => self.camera.zoom(1),
                KeyCode::Minus | KeyCode::NumpadSubtract => self.camera.zoom(-1),
                KeyCode::Comma => self.camera.quarter_turn(1.0),
                KeyCode::Period => self.camera.quarter_turn(-1.0),
                KeyCode::Digit1 => self.speed = 1.0,
                KeyCode::Digit2 => self.speed = 2.0,
                KeyCode::Digit3 => self.speed = 4.0,
                KeyCode::Digit4 => self.speed = 8.0,
                KeyCode::KeyR => match &mut self.tool {
                    // The same way as the picker's rotate button turns the
                    // preview.
                    Tool::Structure { rot, .. } => *rot = (*rot + 3) % 4,
                    Tool::Deco { rot, .. } => *rot += std::f32::consts::FRAC_PI_4,
                    _ => {}
                },
                KeyCode::Escape if matches!(self.tool, Tool::RunPath { .. }) && self.drop_run_layout() => {}
                // A picked rectangle goes first; with none, the tool.
                KeyCode::Escape if matches!(self.tool, Tool::Terrain(_)) && self.terrain_sel.take().is_some() => {}
                KeyCode::Escape => {
                    if let Tool::Run { def, gates } = &self.tool {
                        if gates.len() >= 2 {
                            let (d, g) = (*def, gates.clone());
                            self.build_run(d, g);
                        }
                    }
                    self.tool = Tool::Inspect;
                }
                KeyCode::Enter => {
                    if let Tool::Run { def, gates } = &self.tool {
                        let (d, g) = (*def, gates.clone());
                        if self.build_run(d, g) {
                            self.tool = Tool::Run { def: d, gates: Vec::new() };
                        }
                    }
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    if let Some(sel) = self.selection.clone() {
                        self.demolish(&sel);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn mouse(&mut self, input: &Input, size: Vec2) {
        if input.scroll != 0.0 {
            self.camera.wheel(input.scroll);
        }
        if input.middle_down || (input.right_down && input.held(KeyCode::ShiftLeft)) {
            self.camera.rotate(-input.mouse_delta.x * 0.006, input.mouse_delta.y * 0.004);
        } else if input.right_down && input.mouse_delta != Vec2::ZERO {
            self.camera.pan(input.mouse_delta.y * 0.9, -input.mouse_delta.x * 0.9);
        }
        let ndc = screen_to_ndc(input.mouse, size);
        self.hover = ground_hit(&self.camera, self.world.level(), ndc);
        self.update_placement();
        if let Some(sel) = self.carrying.clone() {
            // The person dangles over the pointer until dropped with a click.
            if let Some(p) = self.hover {
                let up = p + Vec3::new(0.0, 0.0, 6.0);
                match sel {
                    Selection::Guest(id) => {
                        if let Some(g) = self.world.guests.iter_mut().find(|g| g.id == id) {
                            g.pos = up;
                        }
                    }
                    Selection::Staff(id) => {
                        if let Some(s) = self.world.staff.iter_mut().find(|s| s.id == id) {
                            s.pos = up;
                        }
                    }
                    _ => {}
                }
            }
            if input.left_clicked {
                self.drop_carried();
            }
        } else if input.left_clicked {
            self.click(input, size);
        }
        if let Tool::Terrain(t) = self.tool {
            self.surface_input(t, input, size);
        } else {
            self.terrain_sel = None;
        }
        if let Tool::RunPath { mode } = self.tool {
            self.run_path_input(mode, input);
        } else {
            self.run_layout = RunLayout::default();
        }
        if input.left_down {
            if let (Tool::Path { kind }, Some(p)) = (self.tool.clone(), self.hover) {
                self.paint_path(kind, p);
            }
        }
        // Tree tools: press to start a rectangle, release to apply it.
        if let Tool::Trees { density } = self.tool {
            if input.left_clicked {
                self.tree_drag = self.hover.map(|p| self.cell_of(p));
            }
            if !input.left_down {
                if let Some(rect) = self.tree_rect() {
                    self.apply_trees(density, rect);
                }
                self.tree_drag = None;
            }
        } else {
            self.tree_drag = None;
        }
        // A right click (without dragging the view) steps back: it undoes
        // the last point of a lift, run or course, else leaves the tool.
        if input.right_tap {
            match &mut self.tool {
                Tool::Run { gates, .. } if !gates.is_empty() => {
                    gates.pop();
                }
                Tool::Lift { start, .. } if start.is_some() => *start = None,
                Tool::Park { start, .. } if start.is_some() => *start = None,
                Tool::Survey { start } if start.is_some() => *start = None,
                // Right-clicking skiers or buildings shows their details
                // (TUT_MSG_INTRO).
                Tool::Inspect => {
                    if let Some(p) = self.hover {
                        self.select_at(p);
                    }
                }
                _ => self.tool = Tool::Inspect,
            }
        }
    }

    pub(super) fn click(&mut self, _input: &Input, _size: Vec2) {
        let Some(p) = self.hover else { return };
        match self.tool.clone() {
            Tool::Inspect => self.select_at(p),
            Tool::Survey { start: None } => {
                self.tool = Tool::Survey {
                    start: Some(self.nearest_vertex(p)),
                }
            }
            Tool::Survey { start: Some(_) } => self.tool = Tool::Inspect,
            Tool::Structure { def, rot } => {
                let s = self.world.level().spacing;
                let d = &self.defs.structures[def];
                let (w, h) = if rot % 2 == 0 { d.size } else { (d.size.1, d.size.0) };
                let cell = (
                    ((p.x / s) - w as f32 * 0.5).round() as i32,
                    ((p.y / s) - h as f32 * 0.5).round() as i32,
                );
                let before = self.world.cash;
                if self.build_structure(def, cell, rot, false) {
                    // The placing sound, then the building's own (0x4b5be3).
                    self.sfx(Sound::Placing);
                    let own = sound_path(&self.defs.structures[def].menu_sound);
                    self.sfx_file(&own);
                    if let Some(st) = self.world.structures.last() {
                        self.building_quiet.insert(st.id, 0.0);
                    }
                    // One building per pick, as in the original.
                    self.tool = Tool::Inspect;
                    if let Some(st) = self.world.structures.last() {
                        let (id, pos) = (st.id, st.pos);
                        self.after_build(Selection::Structure(id), pos, before);
                    }
                }
            }
            Tool::Park { def, start: None } => self.tool = Tool::Park { def, start: Some(p) },
            Tool::Park { def, start: Some(st) } => {
                let before = self.world.cash;
                if self.build_park(def, st, p) {
                    self.sfx(Sound::Placing);
                    self.tool = Tool::Inspect;
                    if let Some(pk) = self.world.parks.last() {
                        let (id, pos) = (pk.id, pk.start.lerp(pk.end, 0.5));
                        self.after_build(Selection::Park(id), pos, before);
                    }
                }
            }
            Tool::Lift { def, start: None } => self.tool = Tool::Lift { def, start: Some(p) },
            Tool::Lift { def, start: Some(st) } => {
                let before = self.world.cash;
                if self.build_lift(def, st, p) {
                    self.sfx(Sound::Placing);
                    let own = sound_path(&self.defs.lifts[def].menu_sound);
                    self.sfx_file(&own);
                    self.tool = Tool::Inspect;
                    if let Some(l) = self.world.lifts.last() {
                        let (id, pos) = (l.id, l.start.lerp(l.end, 0.5));
                        self.after_build(Selection::Lift(id), pos, before);
                    }
                }
            }
            Tool::Run { def, mut gates } => {
                if let Some(last) = gates.last() {
                    if p.z > last.z + 2.0 {
                        self.error(Problem::GateNotLower);
                        return;
                    }
                }
                gates.push(p);
                self.sfx(Sound::Placing);
                self.tool = Tool::Run { def, gates };
            }
            Tool::Deco { def, rot } => {
                let cost = self.defs.decos[def].cost;
                if self.world.cash < cost {
                    self.error(Problem::TooExpensive);
                    return;
                }
                let id = self.world.new_id();
                self.world.decos.push(Deco { id, def, pos: p, rot });
                self.world.spend(Ledger::Construction, cost);
                self.sfx(Sound::Placing);
            }
            Tool::Bulldoze => {
                self.select_at(p);
                if let Some(sel) = self.selection.clone() {
                    self.demolish(&sel);
                } else if let Some(i) = self
                    .world
                    .decos
                    .iter()
                    .position(|d| d.pos.truncate().distance(p.truncate()) < 6.0)
                {
                    self.world.decos.remove(i);
                    self.sfx(Sound::Destroy);
                }
            }
            Tool::Terrain(_) | Tool::Trees { .. } | Tool::RunPath { .. } | Tool::Path { .. } => {}
        }
    }

    /// What is under a ground point: people first (small targets), then
    /// buildings, lifts, parks and runs.
    pub fn pick_at(&self, p: Vec3) -> Option<Selection> {
        let s = self.world.level().spacing;
        // People first (small targets), then buildings, lifts, runs.
        let near = |q: Vec3, r: f32| q.distance(p) < r;
        if let Some(g) = self
            .world
            .guests
            .iter()
            .filter(|g| g.pos.z > -100.0 && near(g.pos, 6.0))
            .min_by(|a, b| a.pos.distance(p).total_cmp(&b.pos.distance(p)))
        {
            return Some(Selection::Guest(g.id));
        }
        if let Some(st) = self.world.staff.iter().find(|x| near(x.pos, 7.0)) {
            return Some(Selection::Staff(st.id));
        }
        if let Some(i) = self.world.cell_index((p.x / s) as i32, (p.y / s) as i32) {
            if let Some(id) = self.world.occupied[i] {
                if self.world.structure(id).is_some() {
                    return Some(Selection::Structure(id));
                }
                if self.world.lift(id).is_some() {
                    return Some(Selection::Lift(id));
                }
            }
        }
        for l in &self.world.lifts {
            let a = l.start.truncate();
            let b = l.end.truncate();
            let q = p.truncate();
            let t = ((q - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
            if (a + (b - a) * t).distance(q) < 8.0 {
                return Some(Selection::Lift(l.id));
            }
        }
        for pk in &self.world.parks {
            let a = pk.start.truncate();
            let b = pk.end.truncate();
            let q = p.truncate();
            let t = ((q - a).dot(b - a) / (b - a).length_squared().max(1.0)).clamp(0.0, 1.0);
            if (a + (b - a) * t).distance(q) < 12.0 {
                return Some(Selection::Park(pk.id));
            }
        }
        for r in &self.world.runs {
            if r.gates.iter().any(|g| near(*g, 10.0)) {
                return Some(Selection::Run(r.id));
            }
        }
        None
    }

    /// After a placement the original opens the new object's window and
    /// sends the cost from it toward the bottom-left corner.
    pub(super) fn after_build(&mut self, sel: Selection, pos: Vec3, cash_before: f32) {
        self.selection = Some(sel);
        let spent = cash_before - self.world.cash;
        if spent > 0.0 {
            self.floaters.push(Floater {
                at: pos,
                text: format!("-{}", crate::ui::style::money(spent)),
                age: 0.0,
            });
        }
    }

    pub fn select_at(&mut self, p: Vec3) {
        self.selection = self.pick_at(p);
    }

    /// The tooltip the original shows over the world: the name of what is
    /// under the pointer (nothing over open ground); while placing a lift,
    /// its name, then its cost, or what is wrong ("Not Long Enough").
    pub fn hover_label(&self) -> Option<String> {
        let p = self.hover?;
        match &self.tool {
            Tool::Lift { def, start: None } => {
                let d = &self.defs.lifts[*def];
                let base = match self.defs.strings.get(&d.name_key) {
                    Some(s) if !s.is_empty() => s.to_string(),
                    _ => d.unique_name.clone(),
                };
                let n = self.world.lifts.iter().filter(|l| l.def == *def).count() + 1;
                return Some(format!("{base} {n}"));
            }
            // Placing run markers shows the run's name to be (seen in the
            // original).
            Tool::Run { def, .. } => return Some(self.next_run_name(*def)),
            Tool::Lift { start: Some(_), .. } => {
                return self.placement.as_ref().map(|pl| match pl.reason.and_then(|r| r.text()) {
                    Some(t) => self.defs.text(t).into_owned(),
                    None => format!("-{}", crate::ui::style::money(pl.cost)),
                });
            }
            // While a rectangle is dragged out, what it will cost
            // (0x423b40 state 1); while one is reshaped over buildings,
            // that it cannot be (GM_NOT_CHANGEABLE).
            Tool::Trees { density: 0 } => {
                if let Some(rect) = self.tree_rect() {
                    return Some(format!("-{}", crate::ui::style::money(self.chop_cost(rect))));
                }
            }
            Tool::RunPath { .. } => return Some(self.defs.text(self.run_path_label()).into_owned()),
            Tool::Terrain(t) => match &self.terrain_sel {
                Some(sel) if sel.phase == SelPhase::Dragging && matches!(t, TerrainTool::Moguls | TerrainTool::MakeSnow | TerrainTool::Groom) => {
                    let cost = self.surface_cells(*t, sel.rect()).len() as f32 * 5.0;
                    return Some(format!("-{}", crate::ui::style::money(cost)));
                }
                Some(sel) if matches!(sel.phase, SelPhase::Shaping { .. }) && !self.changeable(sel.rect()) => {
                    return Some(self.defs.text(Text::GmNotChangeable).into_owned());
                }
                _ => {}
            },
            _ => {}
        }
        let name = match self.pick_at(p) {
            Some(Selection::Guest(id)) => self.world.guests.iter().find(|g| g.id == id).map(|g| g.name.clone()),
            Some(Selection::Staff(id)) => self.world.staff.iter().find(|x| x.id == id).map(|x| x.name.clone()),
            Some(Selection::Structure(id)) => self.world.structure(id).map(|x| x.name.clone()),
            Some(Selection::Lift(id)) => self.world.lift(id).map(|x| x.name.clone()),
            Some(Selection::Park(id)) => self.world.parks.iter().find(|x| x.id == id).map(|x| x.name.clone()),
            Some(Selection::Run(id)) => self.world.runs.iter().find(|x| x.id == id).map(|x| x.name.clone()),
            _ => None,
        };
        name
    }
}
