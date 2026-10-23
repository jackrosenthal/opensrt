//! egui settings for the few native widgets left (text entry), and money
//! formatting.

use egui::{Color32, Visuals};

pub fn apply(ctx: &egui::Context) {
    let mut v = Visuals::light();
    v.selection.bg_fill = Color32::from_rgb(0x99, 0xb0, 0xe0);
    v.text_cursor.stroke.color = Color32::from_rgb(0x00, 0x00, 0x99);
    ctx.set_visuals(v);
}

pub fn money(v: f32) -> String {
    let neg = v < 0.0;
    let n = v.abs().round() as i64;
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if neg { format!("-${out}") } else { format!("${out}") }
}
