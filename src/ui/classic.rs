//! The original game's interface art: 640×480 screens from `data/ui`,
//! scaled to the window, with button states and the original fonts.

use std::collections::HashMap;

use egui::{Color32, FontFamily, FontId, Pos2, Rect as ERect, TextureHandle, TextureOptions, Vec2};

use super::fonts::{ARIAL, ARIAL_BOLD};
use super::art::{ArtFile, Group};
use super::vector::{self, Svg};
use crate::assets::Assets;
use crate::formats::ui::{Rect, Screen};

pub const CANVAS: Vec2 = Vec2::new(640.0, 480.0);

/// A text style from the original font table: Arial at a pixel size on the
/// 640×480 canvas, in a color.
#[derive(Clone, Copy)]
pub struct TextStyle {
    pub size: f32,
    pub bold: bool,
    pub color: Color32,
}

/// Font 3 of the original table: descriptions.
pub const BODY: TextStyle = TextStyle {
    size: 11.0,
    bold: false,
    color: Color32::from_rgb(0x00, 0x00, 0x99),
};
/// Font 4 of the original table: list entries.
pub const LIST: TextStyle = TextStyle {
    size: 16.0,
    bold: true,
    color: Color32::from_rgb(0x00, 0x00, 0x99),
};

/// Maps the 640×480 canvas onto the screen, letterboxed.
#[derive(Clone, Copy)]
pub struct Canvas {
    pub origin: Pos2,
    pub scale: f32,
    /// Physical pixels per egui point, for picking texture resolutions.
    pub ppp: f32,
}

impl Canvas {
    pub fn fit(screen: ERect, ppp: f32) -> Canvas {
        let scale = (screen.width() / CANVAS.x).min(screen.height() / CANVAS.y);
        let size = CANVAS * scale;
        Canvas {
            origin: screen.center() - size * 0.5,
            scale,
            ppp,
        }
    }
    pub fn pos(&self, x: f32, y: f32) -> Pos2 {
        self.origin + Vec2::new(x, y) * self.scale
    }
    pub fn rect(&self, r: &Rect) -> ERect {
        ERect::from_min_max(
            self.pos(r.x1 as f32, r.y1 as f32),
            self.pos(r.x2 as f32 + 1.0, r.y2 as f32 + 1.0),
        )
    }
    pub fn rect_xywh(&self, x: f32, y: f32, w: f32, h: f32) -> ERect {
        ERect::from_min_size(self.pos(x, y), Vec2::new(w, h) * self.scale)
    }
    /// Device pixels per canvas pixel.
    fn device_scale(&self) -> f32 {
        self.scale * self.ppp
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Normal = 0,
    Hover = 1,
    Pressed = 2,
}

/// Which sounds the buttons being drawn make. Each of the original's
/// screens sets its own (`sounds.ini`'s ui_* for the menus, ingame_* for
/// the game's bars, trays and building menu); the info windows set none.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum UiSounds {
    #[default]
    Menu,
    InGame,
    Silent,
}

impl UiSounds {
    pub fn rollover(self) -> Option<crate::audio::Sound> {
        match self {
            UiSounds::Menu => Some(crate::audio::Sound::UiRollover),
            UiSounds::InGame => Some(crate::audio::Sound::IngameRollover),
            UiSounds::Silent => None,
        }
    }

    pub fn click(self) -> Option<crate::audio::Sound> {
        match self {
            UiSounds::Menu => Some(crate::audio::Sound::UiDownclick),
            UiSounds::InGame => Some(crate::audio::Sound::IngameDownclick),
            UiSounds::Silent => None,
        }
    }
}

pub struct Art {
    pub sounds: UiSounds,
    screens: HashMap<String, Screen>,
    /// Textures resampled per display scale (in eighths), with the frame
    /// they were last used in.
    textures: HashMap<(String, usize, u32), (Option<(TextureHandle, Vec2)>, u64)>,
    /// Vector art rasterized per display scale (in eighths), shown
    /// state and width (in device pixels, for drawings without their own),
    /// with the frame they were last used in.
    vectors: HashMap<(Svg, Option<&'static str>, u32, Option<u32>), (Option<(TextureHandle, Vec2)>, u64)>,
}

impl Default for Art {
    fn default() -> Art {
        Art::new()
    }
}

/// Resizes art to its on-screen size with a sharp filter, so the GPU does
/// not have to stretch low-resolution textures.
fn resample(img: &crate::formats::texture::Rgba, scale: f32) -> egui::ColorImage {
    let src = image::RgbaImage::from_raw(img.width, img.height, img.pixels.clone()).unwrap();
    let w = ((img.width as f32 * scale).round() as u32).max(1);
    let h = ((img.height as f32 * scale).round() as u32).max(1);
    let out = if scale > 1.01 {
        // Premultiply so transparent edges do not pick up dark fringes.
        let mut pre = src;
        for p in pre.pixels_mut() {
            let a = p[3] as u32;
            for c in 0..3 {
                p[c] = (p[c] as u32 * a / 255) as u8;
            }
        }
        let mut up = image::imageops::resize(&pre, w, h, image::imageops::FilterType::CatmullRom);
        for p in up.pixels_mut() {
            let a = p[3] as u32;
            if a > 0 {
                for c in 0..3 {
                    p[c] = ((p[c] as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        up
    } else {
        src
    };
    egui::ColorImage::from_rgba_unmultiplied([out.width() as usize, out.height() as usize], out.as_raw())
}

impl Art {
    pub fn new() -> Art {
        Art {
            sounds: UiSounds::Menu,
            screens: HashMap::new(),
            textures: HashMap::new(),
            vectors: HashMap::new(),
        }
    }

    pub fn screen(&mut self, assets: &Assets, file: ArtFile) -> Option<&Screen> {
        if !self.screens.contains_key(file.0) {
            let s = assets
                .resolve(file.0)
                .and_then(|p| Screen::load(p).map_err(|e| log::warn!("{e:#}")).ok());
            match s {
                Some(s) => {
                    self.screens.insert(file.0.to_string(), s);
                }
                None => return None,
            }
        }
        self.screens.get(file.0)
    }

    fn texture(&mut self, ctx: &egui::Context, assets: &Assets, cv: Canvas, file: ArtFile, sprite: usize) -> Option<(TextureHandle, Vec2)> {
        // Resample once per display scale. Several scales can be in use in
        // one frame (HUD and menus), so sizes unused for a while are
        // dropped rather than everything on a change.
        let q = (cv.device_scale() * 8.0).round().max(1.0) as u32;
        let scale = q as f32 / 8.0;
        let frame = ctx.cumulative_pass_nr();
        let key = (file.0.to_string(), sprite, q);
        if let Some(t) = self.textures.get_mut(&key) {
            t.1 = frame;
            return t.0.clone();
        }
        if self.textures.len() > 512 {
            self.textures.retain(|_, t| frame.saturating_sub(t.1) < 120);
        }
        let t = self.screen(assets, file).and_then(|s| s.sprites.get(sprite)).and_then(|sp| {
            let img = sp.image.as_ref()?;
            let ci = resample(img, scale);
            let h = ctx.load_texture(format!("{}:{}", file.0, sp.name), ci, TextureOptions::LINEAR);
            Some((h, Vec2::new(img.width as f32, img.height as f32)))
        });
        self.textures.insert(key, (t.clone(), frame));
        t
    }

    fn vector_texture(&mut self, ctx: &egui::Context, cv: Canvas, svg: Svg, state: Option<&'static str>, width: Option<f32>) -> Option<(TextureHandle, Vec2)> {
        let q = (cv.device_scale() * 8.0).round().max(1.0) as u32;
        let scale = q as f32 / 8.0;
        let frame = ctx.cumulative_pass_nr();
        let key = (svg, state, q, width.map(|w| (w * scale).round() as u32));
        if let Some(t) = self.vectors.get_mut(&key) {
            t.1 = frame;
            return t.0.clone();
        }
        // A drawing made to a width is redrawn whenever the window is
        // resized, so its other widths go as soon as they are unused.
        self.vectors.retain(|k, t| k.0 != svg || k.3.is_none() || t.1 == frame);
        if self.vectors.len() > 64 {
            self.vectors.retain(|_, t| frame.saturating_sub(t.1) < 120);
        }
        let t = vector::rasterize(svg, state, width, scale).map(|(img, size)| {
            let name = format!("{}.svg:{}", svg.name, state.unwrap_or(""));
            (ctx.load_texture(name, img, TextureOptions::LINEAR), size)
        });
        self.vectors.insert(key, (t.clone(), frame));
        t
    }

    /// Paints vector art with its top-left at canvas (x, y), showing
    /// only group `state` when given.
    pub fn vector_at(&mut self, ui: &egui::Ui, cv: Canvas, svg: Svg, state: Option<&'static str>, x: f32, y: f32) {
        self.vector_paint(ui, cv, svg, state, None, x, y);
    }

    /// Paints vector art that has no width of its own `width` canvas
    /// pixels wide, with its top-left at canvas (x, y).
    pub fn vector_wide(&mut self, ui: &egui::Ui, cv: Canvas, svg: Svg, width: f32, x: f32, y: f32) {
        self.vector_paint(ui, cv, svg, None, Some(width), x, y);
    }

    #[allow(clippy::too_many_arguments)]
    fn vector_paint(&mut self, ui: &egui::Ui, cv: Canvas, svg: Svg, state: Option<&'static str>, width: Option<f32>, x: f32, y: f32) {
        let Some((tex, size)) = self.vector_texture(ui.ctx(), cv, svg, state, width) else { return };
        let uv = ERect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let o = svg.origin();
        ui.painter().image(tex.id(), cv.rect_xywh(x + o.x, y + o.y, size.x, size.y), uv, Color32::WHITE);
    }

    /// Paints a sprite with its top-left at canvas (x, y).
    pub fn sprite_at(&mut self, ui: &egui::Ui, assets: &Assets, cv: Canvas, file: ArtFile, sprite: usize, x: f32, y: f32) {
        self.sprite_part(ui, assets, cv, file, sprite, None, x, y);
    }

    /// Paints the `src` part of a sprite (inclusive rect, whole sprite when
    /// `None`), keeping the sprite's origin at canvas (x, y).
    #[allow(clippy::too_many_arguments)]
    pub fn sprite_part(&mut self, ui: &egui::Ui, assets: &Assets, cv: Canvas, file: ArtFile, sprite: usize, src: Option<Rect>, x: f32, y: f32) {
        let Some((tex, size)) = self.texture(ui.ctx(), assets, cv, file, sprite) else { return };
        // A layer rect larger than its sprite either way is not a crop
        // (the tray end cap's is 38 wide for an 18-wide sprite).
        let src = src.filter(|r| ((r.x2 + 1) as f32) <= size.x && ((r.y2 + 1) as f32) <= size.y);
        let (x1, y1, x2, y2) = match src {
            Some(r) => (
                r.x1.max(0) as f32,
                r.y1.max(0) as f32,
                ((r.x2 + 1) as f32).min(size.x),
                ((r.y2 + 1) as f32).min(size.y),
            ),
            None => (0.0, 0.0, size.x, size.y),
        };
        if x2 <= x1 || y2 <= y1 {
            return;
        }
        ui.painter().image(
            tex.id(),
            cv.rect_xywh(x + x1, y + y1, x2 - x1, y2 - y1),
            ERect::from_min_max(Pos2::new(x1 / size.x, y1 / size.y), Pos2::new(x2 / size.x, y2 / size.y)),
            Color32::WHITE,
        );
    }

    /// Paints a sprite stretched over a canvas rectangle.
    pub fn sprite_stretch(&mut self, ui: &egui::Ui, assets: &Assets, cv: Canvas, file: ArtFile, sprite: usize, src: ERect, dst: ERect) {
        let Some((tex, size)) = self.texture(ui.ctx(), assets, cv, file, sprite) else { return };
        let uv = ERect::from_min_max(
            Pos2::new(src.min.x / size.x, src.min.y / size.y),
            Pos2::new(src.max.x / size.x, src.max.y / size.y),
        );
        ui.painter().image(tex.id(), dst, uv, Color32::WHITE);
    }

    /// Like `sprite_stretch`, on another painter (for example, a tooltip
    /// layer).
    #[allow(clippy::too_many_arguments)]
    pub fn sprite_stretch_on(&mut self, painter: &egui::Painter, ctx: &egui::Context, assets: &Assets, cv: Canvas, file: ArtFile, sprite: usize, src: ERect, dst: ERect) {
        let Some((tex, size)) = self.texture(ctx, assets, cv, file, sprite) else { return };
        let uv = ERect::from_min_max(
            Pos2::new(src.min.x / size.x, src.min.y / size.y),
            Pos2::new(src.max.x / size.x, src.max.y / size.y),
        );
        painter.image(tex.id(), dst, uv, Color32::WHITE);
    }

    /// Size of a sprite in canvas pixels.
    pub fn sprite_size(&mut self, ctx: &egui::Context, assets: &Assets, cv: Canvas, file: ArtFile, sprite: usize) -> Option<Vec2> {
        self.texture(ctx, assets, cv, file, sprite).map(|t| t.1)
    }

    /// Paints every layer of element `index` of a group placed at canvas
    /// (x, y), or its vector remake when it has one. Returns false when
    /// the element does not exist.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&mut self, ui: &egui::Ui, assets: &Assets, cv: Canvas, file: ArtFile, group: Group, index: usize, x: f32, y: f32) -> bool {
        if let Some((svg, state)) = vector::element(file, group, index) {
            self.vector_at(ui, cv, svg, state, x, y);
            return true;
        }
        let Some(layers) = self
            .screen(assets, file)
            .and_then(|s| s.group(group.0))
            .and_then(|g| g.elements.get(index))
            .map(|e| e.layers.clone())
        else {
            return false;
        };
        for l in layers {
            if let Some(sprite) = l.sprite {
                self.sprite_part(ui, assets, cv, file, sprite, Some(l.src), x + l.dst.0 as f32, y + l.dst.1 as f32);
            }
        }
        true
    }

    /// Hit rectangle of an element, relative to its group position.
    pub fn hit_rect(&mut self, assets: &Assets, file: ArtFile, group: Group, index: usize) -> Option<Rect> {
        self.screen(assets, file)
            .and_then(|s| s.group(group.0))
            .and_then(|g| g.elements.get(index))
            .map(|e| e.rect)
    }

    /// Number of elements in a group.
    pub fn element_count(&mut self, assets: &Assets, file: ArtFile, group: Group) -> usize {
        self.screen(assets, file).and_then(|s| s.group(group.0)).map_or(0, |g| g.elements.len())
    }

    pub fn group_element(&mut self, assets: &Assets, file: ArtFile, group: Group, index: usize) -> Option<(Option<usize>, Rect)> {
        self.screen(assets, file)
            .and_then(|s| s.group(group.0))
            .and_then(|g| g.elements.get(index))
            .map(|e| (e.sprite, e.rect))
    }

    /// Paints one element of a group placed at the canvas origin.
    pub fn element(&mut self, ui: &egui::Ui, assets: &Assets, cv: Canvas, file: ArtFile, group: Group, index: usize) {
        self.draw(ui, assets, cv, file, group, index, 0.0, 0.0);
    }

    /// Paints element `index` of a widget group placed at canvas (x, y).
    #[allow(clippy::too_many_arguments)]
    pub fn widget_element(&mut self, ui: &egui::Ui, assets: &Assets, cv: Canvas, file: ArtFile, group: Group, index: usize, x: f32, y: f32) {
        self.draw(ui, assets, cv, file, group, index, x, y);
    }

    pub fn background(&mut self, ui: &egui::Ui, assets: &Assets, cv: Canvas, file: ArtFile, group: Group) {
        self.element(ui, assets, cv, file, group, 0);
    }

    /// A three-state button (normal/rollover/depress elements). `offset`
    /// moves the group's own layout rectangles, as the original's widget
    /// tables do. Returns true when clicked.
    pub fn button(&mut self, ui: &mut egui::Ui, assets: &Assets, cv: Canvas, file: ArtFile, group: Group, offset: (f32, f32)) -> bool {
        let Some(g) = self.screen(assets, file).and_then(|s| s.group(group.0)).cloned() else {
            return false;
        };
        let Some(first) = g.elements.first() else { return false };
        let r = Rect {
            x1: first.rect.x1 + offset.0 as i32,
            y1: first.rect.y1 + offset.1 as i32,
            x2: first.rect.x2 + offset.0 as i32,
            y2: first.rect.y2 + offset.1 as i32,
        };
        let area = cv.rect(&r);
        let id = ui.scope_id().with((file.0, group.0, r.x1, r.y1));
        let resp = ui.interact(area, id, egui::Sense::click());
        let state = if resp.is_pointer_button_down_on() {
            ButtonState::Pressed
        } else if resp.hovered() {
            ButtonState::Hover
        } else {
            ButtonState::Normal
        };
        let was = ui.ctx().memory(|m| m.data.get_temp::<bool>(id.with("h")).unwrap_or(false));
        if resp.hovered() && !was {
            if let Some(s) = self.sounds.rollover() {
                crate::audio::sfx(assets, s);
            }
        }
        ui.ctx().memory_mut(|m| m.data.insert_temp(id.with("h"), resp.hovered()));
        let idx = (state as usize).min(g.elements.len() - 1);
        self.draw(ui, assets, cv, file, group, idx, offset.0, offset.1);
        if resp.clicked() {
            if let Some(s) = self.sounds.click() {
                crate::audio::sfx(assets, s);
            }
        }
        resp.clicked()
    }

    /// Arial text with its top-left at canvas (x, y).
    pub fn text(&self, ui: &egui::Ui, cv: Canvas, style: TextStyle, x: f32, y: f32, text: &str) {
        let family = FontFamily::Name(if style.bold { ARIAL_BOLD } else { ARIAL }.into());
        ui.painter().text(
            cv.pos(x, y),
            egui::Align2::LEFT_TOP,
            text,
            FontId::new(style.size * cv.scale, family),
            style.color,
        );
    }

    /// Word-wrapped Arial text inside a canvas box (x, y, width, height).
    pub fn text_box(&self, ui: &egui::Ui, cv: Canvas, style: TextStyle, area: (f32, f32, f32, f32), text: &str) {
        let family = FontFamily::Name(if style.bold { ARIAL_BOLD } else { ARIAL }.into());
        let font = FontId::new(style.size * cv.scale, family);
        let galley = ui.painter().layout(text.to_string(), font, style.color, area.2 * cv.scale);
        let clip = cv.rect_xywh(area.0, area.1, area.2, area.3);
        ui.painter().with_clip_rect(clip).galley(cv.pos(area.0, area.1), galley, style.color);
    }

    /// Row height of a list in a text style: the font height plus 5, as
    /// the original computes it.
    pub fn row_height(style: TextStyle) -> f32 {
        style.size + 5.0
    }
}
