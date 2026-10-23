//! Ski Resort Tycoon: Deep Powder, reimplemented.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use srt::{app, assets, formats};

/// Ski Resort Tycoon: Deep Powder, reimplemented. Reads the game data from
/// your own copy of the original CD.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Folder containing the game's DATA directory (an installed copy of
    /// the original will do, or its SkiGame.exe). Defaults to ./gamedata,
    /// then srt/gamedata in the per-user data folder, then an installed
    /// copy under Program Files (Wine's too).
    #[arg(long, global = true, env = "SRT_DATA")]
    data: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the game (the default), optionally straight into a level.
    Play {
        /// Level file, e.g. "data/levels/Aspen Colorado.lev".
        level: Option<String>,
    },
    /// Copy the game data from the original CD image (.cue/.bin or .iso).
    Extract {
        image: PathBuf,
        /// Destination folder (default: srt/gamedata in the per-user data folder).
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,srt=info")).init();
    install_crash_log();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Play { level: None }) {
        Command::Play { level } => app::App::run(assets::Assets::discover(cli.data.as_deref())?, level),
        Command::Extract { image, out } => {
            let out = out
                .or_else(|| assets::data_home().map(|d| d.join("srt/gamedata")))
                .unwrap_or_else(|| PathBuf::from("gamedata"));
            formats::extract::extract(&image, &out)
        }
    }
}

/// Writes panics, with a backtrace, to srt/crash.log in the per-user data
/// folder as well as to the terminal.
fn install_crash_log() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        let report = format!("{info}\n\n{bt}\n");
        if let Some(dir) = assets::data_home().map(|d| d.join("srt")) {
            let _ = std::fs::create_dir_all(&dir);
            let path = dir.join("crash.log");
            if std::fs::write(&path, &report).is_ok() {
                eprintln!("crash report written to {}", path.display());
            }
        }
        default(info);
    }));
}
