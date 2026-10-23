//! Locating the original game data on disk.
//!
//! The game refers to files with Windows paths in arbitrary case
//! (`data\Scenery\objects\Lamp_A\LampA.mesh`). File systems on Linux and
//! macOS are case-sensitive, so we index the data
//! directory once and resolve references case-insensitively.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

pub struct Assets {
    pub root: PathBuf,
    index: HashMap<String, PathBuf>,
}

const LEVELS: &str = "data/levels/";

/// The map editor's saved levels, by lowercase file name.
fn user_levels() -> Vec<(String, PathBuf)> {
    let dir = crate::game::editor::user_levels_dir();
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter_map(|p| Some((p.file_name()?.to_string_lossy().to_ascii_lowercase(), p)))
        .collect()
}

fn key(rel: &str) -> String {
    rel.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

impl Assets {
    /// `root` is the directory that contains `DATA/` (the installed game
    /// folder or the extracted `Program_Executable_Files`).
    pub fn open(root: &Path) -> Result<Assets> {
        let mut index = HashMap::new();
        fn walk(base: &Path, dir: &Path, index: &mut HashMap<String, PathBuf>) {
            let Ok(rd) = std::fs::read_dir(dir) else { return };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(base, &p, index);
                } else if let Ok(rel) = p.strip_prefix(base) {
                    index.insert(key(&rel.to_string_lossy()), p.clone());
                }
            }
        }
        walk(root, root, &mut index);
        if !index.keys().any(|k| k.starts_with("data/")) {
            bail!(
                "{} does not look like a Ski Resort Tycoon data folder (no DATA directory)",
                root.display()
            );
        }
        log::info!("indexed {} game files under {}", index.len(), root.display());
        Ok(Assets {
            root: root.to_path_buf(),
            index,
        })
    }

    /// Finds the data folder from `--data` (or `$SRT_DATA`) or a few defaults.
    pub fn discover(explicit: Option<&Path>) -> Result<Assets> {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(p) = explicit {
            // The game's own SkiGame.exe stands for its folder.
            match p.parent() {
                Some(dir) if p.is_file() => candidates.push(dir.to_path_buf()),
                _ => candidates.push(p.to_path_buf()),
            }
        }
        candidates.push(PathBuf::from("gamedata"));
        if let Some(dirs) = data_home() {
            candidates.push(dirs.join("srt/gamedata"));
        }
        candidates.extend(installed_copies());
        for c in &candidates {
            if c.is_dir() {
                if let Ok(a) = Assets::open(c) {
                    return Ok(a);
                }
            }
        }
        bail!(
            "could not find the game data. Run `srt extract <path to .cue>` first, \
             or pass --data <dir containing DATA/>.\nTried: {}",
            candidates
                .iter()
                .map(|c| c.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    pub fn resolve(&self, rel: &str) -> Option<&Path> {
        let k = key(rel);
        self.index
            .get(&k)
            .or_else(|| {
                if k.starts_with("data/") {
                    None
                } else {
                    self.index.get(&format!("data/{k}"))
                }
            })
            .map(|p| p.as_path())
    }

    /// A level by relative path: one saved from the map editor, else a
    /// shipped one.
    pub fn resolve_level(&self, rel: &str) -> Option<PathBuf> {
        let k = key(rel);
        k.strip_prefix(LEVELS)
            .and_then(|name| user_levels().into_iter().find(|(n, _)| *n == name).map(|(_, p)| p))
            .or_else(|| self.resolve(rel).map(Path::to_path_buf))
    }

    /// Lists files directly inside `dir` (relative path) with extension
    /// `ext`; `data/levels` takes in the map editor's saved levels too.
    pub fn list(&self, dir: &str, ext: &str) -> Vec<(String, PathBuf)> {
        let d = key(dir);
        let d = d.trim_end_matches('/').to_string() + "/";
        let ext = format!(".{}", ext.to_ascii_lowercase());
        let mut out: Vec<(String, PathBuf)> = self
            .index
            .iter()
            .filter(|(k, _)| {
                k.starts_with(&d) && !k[d.len()..].contains('/') && k.ends_with(&ext)
            })
            .map(|(k, p)| (k.clone(), p.clone()))
            .collect();
        if d == LEVELS {
            for (name, p) in user_levels() {
                if name.ends_with(&ext) {
                    let k = format!("{LEVELS}{name}");
                    out.retain(|(o, _)| *o != k);
                    out.push((k, p));
                }
            }
        }
        out.sort();
        out
    }
}

/// Installed copies of the original game: folders holding SkiGame.exe and
/// DATA, up to two levels under Program Files (in the default Wine prefix
/// too, or `$WINEPREFIX`).
fn installed_copies() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(p) = std::env::var_os(var) {
            roots.push(PathBuf::from(p));
        }
    }
    let prefixes = std::env::var_os("WINEPREFIX").map(PathBuf::from).into_iter().chain(dirs::home_dir().map(|h| h.join(".wine")));
    for prefix in prefixes {
        for pf in ["Program Files", "Program Files (x86)"] {
            roots.push(prefix.join("drive_c").join(pf));
        }
    }
    let is_game = |d: &Path| {
        std::fs::read_dir(d).is_ok_and(|rd| {
            let names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().to_ascii_lowercase()).collect();
            names.iter().any(|n| n == "skigame.exe") && names.iter().any(|n| n == "data")
        })
    };
    let subdirs = |d: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(d).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
        v.sort();
        v
    };
    let mut out = Vec::new();
    for root in roots {
        for a in subdirs(&root) {
            if is_game(&a) {
                out.push(a.clone());
            }
            out.extend(subdirs(&a).into_iter().filter(|b| is_game(b)));
        }
    }
    out
}

/// Per-user data folder: `~/.local/share` on Linux, `~/Library/Application
/// Support` on macOS and `%APPDATA%` on Windows.
pub fn data_home() -> Option<PathBuf> {
    dirs::data_dir()
}
