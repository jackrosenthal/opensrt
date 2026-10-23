# srt — Ski Resort Tycoon: Deep Powder

A from-scratch reimplementation of *Ski Resort Tycoon: Deep Powder* (2003) in
Rust. It reads the game data from your own copy of the original CD. No
original code or assets are included in this repository.

The original was a Windows/DirectDraw game with a software renderer and
pre-rendered sprites. This version runs on Linux, Windows and macOS. It
renders the original 3D models directly with wgpu (Vulkan, Metal, DirectX 12
or OpenGL) at any resolution, draws the original menu art and cursors, and
uses rodio for sound.

## Install the game data

You need the original CD image (`.cue` + `.bin`) or an `.iso`:

```sh
cargo build --release
./target/release/srt extract /path/to/dev-skrt.cue
```

`extract` reads the CD image directly. It walks the ISO 9660 file system,
unpacks the InstallShield 5 cabinet (`data1.hdr`/`data1.cab`) and copies the
`DATA/` and `GENERATEDSPRITES/` folders to `srt/gamedata` inside your
per-user data folder (change this with `--out DIR`):

| OS | Data folder |
| --- | --- |
| Linux | `~/.local/share/srt/` |
| macOS | `~/Library/Application Support/srt/` |
| Windows | `%APPDATA%\srt\` | Nothing from the original installer is run.

You can also point the game at an existing Windows installation with
`--data DIR` or `$SRT_DATA` (the folder containing `DATA/`).

## Play

```sh
./target/release/srt                 # main menu
./target/release/srt play "data/levels/Aspen Colorado.lev"
```

| Input | Action |
| --- | --- |
| WASD / arrows, right-drag | pan |
| Q / E, middle-drag (or Shift + right-drag) | rotate |
| mouse wheel, + / − | zoom |
| PgUp / PgDn | tilt |
| Space / P | pause |
| 1–4 | game speed |
| R | rotate the building being placed |
| Enter | finish a ski run |
| right-click | undo last gate / cancel lift start |
| Delete | demolish the selection |
| Esc | cancel tool / release a controlled skier |
| F11 | fullscreen |

**Controlling a skier:** select a guest and click *Take control*. Steer with
A/D or the arrow keys, tuck with W, brake with S and jump with Space. C
switches between third-person and first-person views, and Esc hands the
guest back to the AI.

Saves go to `saves/` in the same folder as JSON, and options to `settings.json`.

## What is implemented

- **Mountains:** the six original mountains and the original challenges,
  with goals and time limits taken from the original game logic.
- **Construction:**
  - 58 structures from `structures*.ini` and `lodges.ini`, with prices,
    running costs, wear, power requirements and prerequisites.
  - 7 lift types: chairs loop on the cable, load at the base and unload at
    the top, with capacity and speed from `lifts.ini`.
  - Marked runs at four difficulties.
  - Terrain-park courses (jump, half pipe, race course).
  - Decorations, tree planting and clearing, and terrain sculpting.
- **Guests:**
  - Skiers and snowboarders at four skill levels, with needs (hunger,
    warmth, bathroom, fun, energy).
  - Money, visit length, lodging and equipment rental.
  - Lift queues, slope-aware routing down the mountain, crashes and
    injuries.
  - Thoughts, and happiness that feeds into the resort rating.
- **Staff:** ski patrol, mechanics, sanitation, groomers and instructors.
- **Economy:** daily lift tickets, rentals, lodging, per-visit prices,
  wages, running costs, loans, advertising and a daily ledger.
- **Rating:** a star rating built from the attraction caps in
  `resort.ini`. The guest arrival rate follows the original formula.
- **Weather and time:** weather, snow depth (switches building models
  between light and heavy snow) and a day/night cycle with the original
  night models.

## Layout

- `src/formats/` contains the readers for the original data:
  - `texture.rs`: `.map`, `.bin` and `.mob` texture records (JPEG mips plus
    an RLE alpha channel).
  - `mesh.rs`: `.MESH` models.
  - `level.rs`: `.lev` terrain.
  - `ini.rs` and `eng.rs`: configuration files and UTF-16 string tables.
  - `extract.rs`: reading the CD image and InstallShield cabinet.
- `src/render/` contains the wgpu renderer: the terrain texture array,
  instanced meshes and headless capture.
- `src/game/` contains the definitions, world state, simulation, path
  finding, challenges and player tools.
- `src/ui/` contains the egui interface.

## Diagnostics

Developer tools live in a separate binary, `srt-tools` (see `srt-tools --help`):

```sh
srt-tools simulate "data/levels/Aspen Colorado.lev" 6 out.png   # headless run with an auto-built resort
srt-tools shot-level "data/levels/Kitzbuehel Austria.lev" out.png
srt-tools render-mesh path/to/Model.MESH out.png
srt-tools check-textures DATA_DIR
srt-tools dump-ui path/to/screen.bin [SPRITE_DIR]
```

Both binaries take `--data DIR` (or `$SRT_DATA`). `SRT_NO_AUDIO=1` disables
sound, and `RUST_LOG=srt=debug` enables more logging.
