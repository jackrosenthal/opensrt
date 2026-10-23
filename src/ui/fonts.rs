//! The original front end draws list and description text with Windows'
//! Arial (`CTrueTypeFont`). We load Arial from the system, or a
//! metric-compatible substitute, and register it with egui.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

pub const ARIAL: &str = "arial";
pub const ARIAL_BOLD: &str = "arial-bold";

const FAMILIES: &[&str] = &["Arial", "Liberation Sans", "Arimo", "Helvetica", "Helvetica Neue"];

fn find(db: &fontdb::Database, bold: bool) -> Option<Vec<u8>> {
    let families: Vec<fontdb::Family> = FAMILIES.iter().map(|f| fontdb::Family::Name(f)).collect();
    let query = fontdb::Query {
        families: &families,
        weight: if bold { fontdb::Weight::BOLD } else { fontdb::Weight::NORMAL },
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    };
    let id = db.query(&query)?;
    db.with_face_data(id, |data, _| data.to_vec())
}

/// Registers the Arial families with egui. Falls back to egui's default
/// font when no Arial-compatible face is installed.
pub fn install(ctx: &egui::Context) {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let mut defs = FontDefinitions::default();
    for (name, bold) in [(ARIAL, false), (ARIAL_BOLD, true)] {
        let mut family = Vec::new();
        if let Some(data) = find(&db, bold) {
            defs.font_data.insert(name.to_string(), Arc::new(FontData::from_owned(data)));
            family.push(name.to_string());
        } else {
            log::warn!("no Arial-compatible font found; menu text uses the default font");
        }
        family.extend(defs.families[&FontFamily::Proportional].iter().cloned());
        defs.families.insert(FontFamily::Name(name.into()), family);
    }
    ctx.set_fonts(defs);
}
