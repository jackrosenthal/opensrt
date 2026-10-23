//! The pickers (`structures_toolbar`, `lifts_toolbar` and `fluff_menu`)
//! and the in-game dialogs (`rename`, `gen_map`, `no_power`, `thoughts`,
//! `mini_map`, `escape`, `bob` and `fog`).
//!
//! Each screen file gets one function here, named after its `art`
//! module, that matches its groups the way `hud` in `vector.rs` does,
//! and `element` calls it for that file. The shared widgets are mapped
//! in `widgets` for every file; an arm here takes precedence, so it can
//! claim the elements of a group that are the screen's own art.

use super::Drawing;
use crate::ui::art::{ArtFile, Group};

/// The drawing for an element of one of these screen files, if it has
/// one of its own.
pub(super) fn element(_file: ArtFile, _group: Group, _index: usize) -> Option<Drawing> {
    None
}
