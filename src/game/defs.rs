//! Data-driven definitions loaded from the original `data/scenery/*.ini`.
//!
//! The structs mirror the full ini schema, including fields the simulation
//! does not use yet.
#![allow(dead_code)]

use anyhow::Result;

use crate::assets::Assets;
use crate::formats::eng::Strings;
use crate::formats::ini::{Ini, Section};
use crate::formats::mesh::AttachPoint;

/// Mesh variants for weather/time of day.
#[derive(Debug, Clone, Default)]
pub struct MeshSet {
    pub day: String,
    pub light_snow: String,
    pub heavy_snow: String,
    pub night: String,
    pub night_light_snow: String,
    pub night_heavy_snow: String,
}

impl MeshSet {
    fn from_section(s: &Section) -> MeshSet {
        let g = |k: &str| s.get(k).unwrap_or("").to_string();
        MeshSet {
            day: g("mesh"),
            light_snow: g("lightsnowmesh"),
            heavy_snow: g("snowmesh"),
            night: g("nightmesh"),
            night_light_snow: g("nightlightsnowmesh"),
            night_heavy_snow: g("nightsnowmesh"),
        }
    }

    /// Picks the best variant for the conditions, falling back to day.
    pub fn pick(&self, night: bool, snow: u8) -> &str {
        let pick = match (night, snow) {
            (false, 0) => &self.day,
            (false, 1) => &self.light_snow,
            (false, _) => &self.heavy_snow,
            (true, 0) => &self.night,
            (true, 1) => &self.night_light_snow,
            (true, _) => &self.night_heavy_snow,
        };
        if pick.is_empty() { &self.day } else { pick }
    }
}

/// What a skier can satisfy at a structure.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Needs {
    pub food: f32,
    pub warmth: f32,
    pub bathroom: f32,
    pub entertainment: f32,
    pub rest: f32,
    pub health: f32,
}

impl Needs {
    /// How happy these needs make a guest (0x492140): a weighted mean,
    /// health counting most.
    pub fn happiness(&self) -> f32 {
        (2.0 * self.rest + 3.0 * self.warmth + self.bathroom + 2.0 * self.food + 3.0 * self.entertainment + 8.0 * self.health) / 19.0
    }

    pub fn get(&self, k: NeedKind) -> f32 {
        match k {
            NeedKind::Food => self.food,
            NeedKind::Warmth => self.warmth,
            NeedKind::Bathroom => self.bathroom,
            NeedKind::Entertainment => self.entertainment,
            NeedKind::Rest => self.rest,
            NeedKind::Health => self.health,
        }
    }
    pub fn get_mut(&mut self, k: NeedKind) -> &mut f32 {
        match k {
            NeedKind::Food => &mut self.food,
            NeedKind::Warmth => &mut self.warmth,
            NeedKind::Bathroom => &mut self.bathroom,
            NeedKind::Entertainment => &mut self.entertainment,
            NeedKind::Rest => &mut self.rest,
            NeedKind::Health => &mut self.health,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum NeedKind {
    Food,
    Warmth,
    Bathroom,
    Entertainment,
    Rest,
    Health,
}

pub const ALL_NEEDS: [NeedKind; 6] = [
    NeedKind::Food,
    NeedKind::Warmth,
    NeedKind::Bathroom,
    NeedKind::Entertainment,
    NeedKind::Rest,
    NeedKind::Health,
];

/// A structure's type: the `[Structures]` keys of the ini files, which the
/// original maps to type ids in one table (0x66dc5c on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructureKind {
    Entrance,
    Restaurant,
    Bathroom,
    WarmupHut,
    Rental,
    SkiLocker,
    SkiSchool,
    SkiPatrol,
    Medical,
    ChopperPatrol,
    SnowmobilePatrol,
    Retail,
    Entertainment,
    Lodging,
    StaffHousing,
    PowerPlant,
    Sanitation,
    WeatherStation,
    RadioStation,
    Maintenance,
    Groomer,
    Snowmaker,
    AvalancheControl,
    SnowmobileTour,
    HeliTour,
    Realtor,
    Marketing,
    Security,
    Bank,
    Daycare,
    WildlifeControl,
    Other,
}

impl StructureKind {
    pub fn parse(key: &str) -> StructureKind {
        use StructureKind::*;
        match key.trim() {
            "entrance" => Entrance,
            "restaurant" => Restaurant,
            "bathroom" => Bathroom,
            "warmup_hut" => WarmupHut,
            "rental" => Rental,
            "ski_locker" => SkiLocker,
            "ski_school" => SkiSchool,
            "ski_patrol" => SkiPatrol,
            "medical" => Medical,
            "chopper_patrol" => ChopperPatrol,
            "snowmobile_patrol" => SnowmobilePatrol,
            "retail" => Retail,
            "entertainment" => Entertainment,
            "lodging" => Lodging,
            "staff_housing" => StaffHousing,
            "power_plant" => PowerPlant,
            "sanitation" => Sanitation,
            "weather_station" => WeatherStation,
            "radio_station" => RadioStation,
            "maintenance" => Maintenance,
            "groomer" => Groomer,
            "snowmaker" => Snowmaker,
            "avalanche_control" => AvalancheControl,
            "snowmobile_tour" => SnowmobileTour,
            "heli_tour" => HeliTour,
            "realtor" => Realtor,
            "marketing" => Marketing,
            "security" => Security,
            "bank" => Bank,
            "daycare" => Daycare,
            "wildlife_control" => WildlifeControl,
            _ => Other,
        }
    }

    pub fn category(self) -> Category {
        use StructureKind::*;
        match self {
            Entrance => Category::Entrance,
            Restaurant => Category::Food,
            Bathroom => Category::Bathroom,
            WarmupHut => Category::WarmingHut,
            Rental => Category::Rental,
            SkiSchool => Category::SkiSchool,
            SkiPatrol | SnowmobilePatrol | ChopperPatrol | Medical => Category::SkiPatrol,
            Retail => Category::Retail,
            Entertainment => Category::Entertainment,
            Lodging => Category::Lodging,
            StaffHousing => Category::StaffHousing,
            PowerPlant => Category::PowerPlant,
            Sanitation | WeatherStation | RadioStation => Category::Utility,
            Maintenance | Groomer | AvalancheControl => Category::Maintenance,
            SnowmobileTour | HeliTour | Realtor | Marketing | Security | Bank | Daycare | WildlifeControl => Category::Service,
            SkiLocker | Snowmaker | Other => Category::Other,
        }
    }

    /// The staff a building of this type employs and can hire.
    pub fn staff(self) -> Option<super::world::StaffKind> {
        use super::world::StaffKind as S;
        use StructureKind::*;
        Some(match self {
            SkiPatrol | SnowmobilePatrol | ChopperPatrol | Medical => S::Patrol,
            // The groomer shop's staff is a mechanic (0x482dc0 case
            // 0x1a); its machines come besides.
            Maintenance | Groomer => S::Maintenance,
            Sanitation => S::Sanitation,
            SkiSchool => S::Instructor,
            AvalancheControl => S::AvalancheControl,
            Security => S::Security,
            WildlifeControl => S::WildlifeControl,
            SnowmobileTour => S::SnowTour,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Entrance,
    Food,
    Bathroom,
    WarmingHut,
    Rental,
    SkiSchool,
    SkiPatrol,
    Retail,
    Entertainment,
    Lodging,
    StaffHousing,
    Service,
    Utility,
    PowerPlant,
    Maintenance,
    Other,
}

impl Category {
    /// Build-menu group, matching the original toolbar layout.
    pub fn group(self) -> BuildGroup {
        match self {
            Category::Lodging | Category::StaffHousing => BuildGroup::Lodging,
            Category::Food | Category::Retail | Category::Entertainment => BuildGroup::Rentals,
            Category::Bathroom | Category::WarmingHut | Category::Rental | Category::SkiSchool => {
                BuildGroup::Rentals
            }
            Category::SkiPatrol | Category::Maintenance => BuildGroup::Maintenance,
            Category::Service => BuildGroup::Services,
            Category::Utility | Category::PowerPlant => BuildGroup::Utilities,
            Category::Entrance | Category::Other => BuildGroup::Services,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildGroup {
    Lodging,
    Rentals,
    Maintenance,
    Services,
    Utilities,
}

#[derive(Debug, Clone)]
pub struct StructureDef {
    pub key: String,
    pub kind: StructureKind,
    pub category: Category,
    pub meshes: MeshSet,
    pub size: (i32, i32),
    pub name_key: String,
    pub unique_name: String,
    pub desc_key: String,
    pub guest_occupancy: i32,
    pub go_inside: bool,
    pub uses_electricity: bool,
    /// For a power plant, how many building cells it can supply.
    pub utility_coverage: i32,
    pub menu_sound: String,
    pub predecessors: Vec<Requirement>,
    pub cost: f32,
    pub wear_per_day: f32,
    pub operations_per_day: f32,
    pub maintenance_per_wear: f32,
    pub provides: Needs,
    /// Banks: guests short of money go in and come out with some, at
    /// `tick_cash` a tick (0x491670).
    pub provides_cash: bool,
    pub tick_cash: f32,
    pub item_cost: f32,
    pub item_cost_min: f32,
    pub item_cost_max: f32,
    pub recurring_per_day: f32,
    pub ticks: Needs,
    pub ratings_points: f32,
    pub staff_count: i32,
    /// What this building upgrades to (`upgradeable`, `upgrade_name`).
    pub upgrade: Option<usize>,
    /// Reached only by upgrading, so left out of the build menu
    /// (`is_an_upgrade`).
    pub is_upgrade: bool,
    /// Which zoom level its window's preview shows it at (0 nearest).
    pub preview_zoom: u8,
}

impl StructureDef {
    /// How far a click of the price spinners moves the price (0x46d8c1):
    /// a twentieth of the range, whole, and at least 1.
    pub fn price_step(&self) -> f32 {
        (((self.item_cost_max - self.item_cost_min) * 0.05) as i32).max(1) as f32
    }

    /// How dear a price seems, 0 at `item_cost_min` to 1 at
    /// `item_cost_max` (0x4b4ef0); a guest whose tolerance is lower walks
    /// away.
    pub fn dearness(&self, price: f32) -> f32 {
        (price - self.item_cost_min) / (self.item_cost_max - self.item_cost_min).max(1.0)
    }

    /// The price range as the original settles it on loading: without
    /// limits it is from half to one and a half times `item_cost` (plus
    /// one), and always spans `item_cost`.
    fn settle_prices(&mut self) {
        if self.item_cost_max == 10000.0 {
            self.item_cost_max = self.item_cost * 1.5 + 1.0;
        }
        if self.item_cost_min == 0.0 {
            self.item_cost_min = self.item_cost * 0.5;
        }
        if self.item_cost < self.item_cost_min {
            self.item_cost_min = 0.0;
        }
        if self.item_cost_max < self.item_cost {
            self.item_cost_max = self.item_cost + 1.0;
        }
    }

    fn from_section(category_key: &str, s: &Section) -> StructureDef {
        let mut d = Self::read_section(category_key, s);
        d.settle_prices();
        d
    }

    fn read_section(category_key: &str, s: &Section) -> StructureDef {
        StructureDef {
            key: s.name.clone(),
            kind: StructureKind::parse(category_key),
            category: StructureKind::parse(category_key).category(),
            meshes: MeshSet::from_section(s),
            size: s.pair_or("size", (1, 1)),
            name_key: s.str_or("name", "").to_string(),
            unique_name: s.str_or("unique_name", &s.name).to_string(),
            desc_key: s.str_or("description", "").to_string(),
            guest_occupancy: s.i32_or("guest_occupancy", 0),
            go_inside: s.bool_or("go_inside", false),
            uses_electricity: s.bool_or("uses_electricity", false),
            utility_coverage: s.i32_or("utility_coverage", 0),
            menu_sound: s.str_or("menu_sound", "").to_string(),
            predecessors: requirements(s.get("predecessors")),
            cost: s.f32_or("structure_cost", s.f32_or("cost", 0.0)),
            wear_per_day: s.f32_or("wear_and_tear", 0.0),
            operations_per_day: s.f32_or("operations_cost", 0.0),
            maintenance_per_wear: s.f32_or("maintenance_cost", 0.0),
            provides_cash: s.f32_or("provides_cash", 0.0) != 0.0,
            tick_cash: s.f32_or("tick_cash", 0.0),
            provides: Needs {
                food: s.f32_or("provides_food", 0.0),
                warmth: s.f32_or("provides_warmth", 0.0),
                bathroom: s.f32_or("provides_bathroom", 0.0),
                entertainment: s.f32_or("provides_entertainment", 0.0),
                rest: s.f32_or("provides_rest", 0.0),
                health: s.f32_or("provides_health", 0.0),
            },
            item_cost: s.f32_or("item_cost", 0.0),
            item_cost_min: s.f32_or("item_cost_min", 0.0),
            item_cost_max: s.f32_or("item_cost_max", 10000.0),
            recurring_per_day: s.f32_or("reacuring_cost_per_day", 0.0),
            ticks: Needs {
                food: s.f32_or("tick_food", 0.0),
                warmth: s.f32_or("tick_warmth", 0.0),
                bathroom: s.f32_or("tick_bathroom", 0.0),
                entertainment: s.f32_or("tick_entertainment", 0.0),
                rest: s.f32_or("tick_rest", 0.0),
                health: s.f32_or("tick_health", 0.0),
            },
            ratings_points: s.f32_or("ratings_points", 0.0),
            staff_count: s.i32_or("staff_count", 0),
            upgrade: None,
            is_upgrade: s.bool_or("is_an_upgrade", false),
            preview_zoom: s.f32_or("preview_zoom", 1.0).clamp(0.0, 2.0) as u8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiftDef {
    pub index: usize,
    pub name_key: String,
    pub unique_name: String,
    pub desc_key: String,
    pub begin: MeshSet,
    pub middle: MeshSet,
    pub end: MeshSet,
    /// Chair meshes: 0 empty, 1.. occupied variants.
    pub chairs: Vec<String>,
    /// Riders are drawn as skiers (rope tow, T-bar); otherwise the chair
    /// meshes show them (`<chair>_<riders><A-D>`).
    pub show_skiers: bool,
    /// Attach points of the bottom station, towers and top station in their
    /// own frames (`Mesh::attach`): cable loops `cableA`.. and the ground
    /// paths `pt0`.., `ptb0`.. where guests queue and get off.
    pub begin_pts: Vec<(AttachPoint, glam::Vec3)>,
    pub middle_pts: Vec<(AttachPoint, glam::Vec3)>,
    pub end_pts: Vec<(AttachPoint, glam::Vec3)>,
    pub base_cost: f32,
    pub cost_per_pole: f32,
    pub ratings_points: f32,
    pub max_poles: i32,
    pub pole_spacing: f32,
    pub predecessors: Vec<Requirement>,
    pub speed: f32,
    pub capacity: i32,
    pub secs_between_chairs: f32,
    pub start_radius: f32,
    pub end_radius: f32,
    pub path_radius: f32,
    pub wear_per_day: f32,
    pub operations_per_day: f32,
    pub maintenance_per_wear: f32,
    pub menu_sound: String,
}

fn lift_meshes(s: &Section, prefix: &str) -> MeshSet {
    let g = |suffix: &str| {
        s.get(&format!("{prefix}{suffix}"))
            .map(|v| v.split(',').next().unwrap_or("").trim().to_string())
            .unwrap_or_default()
    };
    MeshSet {
        day: g(""),
        light_snow: g("LS"),
        heavy_snow: g("HS"),
        night: g("N"),
        night_light_snow: g("NLS"),
        night_heavy_snow: g("NHS"),
    }
}

#[derive(Debug, Clone)]
pub struct RunMarkerDef {
    pub key: String,
    /// The gate model; the ini names it without its `.MESH`.
    pub mesh: String,
    pub name_key: String,
    pub unique_name: String,
    pub desc_key: String,
    pub cost: f32,
    /// 0 beginner, 1 intermediate, 2 advanced, 3 expert
    pub difficulty: i32,
    pub predecessors: Vec<Requirement>,
    pub ratings_points: f32,
}

/// Terrain park course (jump, half pipe, race course) from `jumps.ini`.
#[derive(Debug, Clone)]
pub struct ParkDef {
    pub name_key: String,
    pub unique_name: String,
    pub desc_key: String,
    pub width: i32,
    /// start, gate, tile, ramp, end — for no snow / some snow / lots of snow.
    pub pieces: [[String; 5]; 3],
    pub cost: f32,
    pub ratings_points: f32,
    pub operations_per_day: f32,
    pub predecessors: Vec<Requirement>,
}

impl ParkDef {
    pub fn piece(&self, snow: u8, k: usize) -> &str {
        let set = &self.pieces[(snow as usize).min(2)];
        if set[k].is_empty() { &self.pieces[0][k] } else { &set[k] }
    }
}

#[derive(Debug, Clone)]
pub struct DecoDef {
    pub key: String,
    pub meshes: MeshSet,
    pub name_key: String,
    pub unique_name: String,
    pub cost: f32,
    /// Which zoom level the objects menu shows it at (0 nearest).
    pub preview_zoom: u8,
}

/// Tunables from `resort.ini`.
#[derive(Debug, Clone)]
pub struct ResortTuning {
    pub seconds_per_day: f32,
    pub weeks_per_season: i32,
    pub probability_snow: f32,
    pub probability_rain: f32,
    pub probability_hail: f32,
    pub precipitation_interval: f32,
    pub probability_fog: f32,
    pub min_wind: f32,
    pub max_wind: f32,
    pub min_temperature: f32,
    pub max_temperature: f32,
    pub caps: AttractCaps,
    pub initial_cash: f32,
}

#[derive(Debug, Clone, Default)]
pub struct AttractCaps {
    pub population: f32,
    pub happiness: f32,
    pub lifts: f32,
    pub beginner: f32,
    pub intermediate: f32,
    pub advanced: f32,
    pub expert: f32,
    pub lodging: f32,
    pub food: f32,
    pub school: f32,
    pub rental: f32,
    pub entertainment: f32,
    pub retail: f32,
    pub ski_toys: f32,
    pub safety: f32,
    pub clean_snow: f32,
}

/// Skier tuning from `skiers.ini`.
#[derive(Debug, Clone)]
pub struct SkierTuning {
    pub danger_min_slope: f32,
    pub danger_max_slope: f32,
    pub max_skiers: i32,
    pub max_skiers_5_star: i32,
    pub max_lodging: i32,
    pub lodging_factor: f32,
    pub max_lifts: i32,
    pub lifts_factor: f32,
    pub per_second_empty: f32,
    pub per_second_half: f32,
    pub tick_in_line: Needs,
    pub tick_on_lift: Needs,
    /// Riding a one-seat lift tires instead (`EnergyTickOnRopetow`); one
    /// seating five or more warms (`WarmthTickOnGondola`) (0x491670).
    pub energy_on_ropetow: f32,
    pub warmth_on_gondola: f32,
    pub tick_normal: Needs,
    pub energy_uphill: f32,
    pub boredom_fast: f32,
    pub bad_weather_boredom: f32,
    pub cold_weather_warmth: f32,
    pub maintenance_wages: f32,
    pub patrol_wages: f32,
    /// Every guest's stay and money: the mean and the full spread
    /// (0x487303). The `lodging_` ones are read but never used.
    pub visit_days: f32,
    pub variance_days: f32,
    pub starting_cash: f32,
    pub variance_cash: f32,
    pub distribution: [f32; 5],
    pub skill: [SkillTuning; 4],
    pub litter_interval: f32,
    /// Guests who cannot hold on any longer go in the snow.
    pub pee_on_snow: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SkillTuning {
    pub probability_crash: f32,
    pub probability_injury: f32,
    pub probability_needs_skis: f32,
    pub min_slope: f32,
    pub max_slope: f32,
}

#[derive(Debug, Clone)]
pub struct StaffTuning {
    pub patrol_wages: f32,
    pub maintenance_wages: f32,
    pub chopper_wages: f32,
    pub sanitation_wages: f32,
    pub instructor_wages: f32,
}

pub struct Defs {
    pub structures: Vec<StructureDef>,
    pub lifts: Vec<LiftDef>,
    pub runs: Vec<RunMarkerDef>,
    pub decos: Vec<DecoDef>,
    pub parks: Vec<ParkDef>,
    pub resort: ResortTuning,
    pub skiers: SkierTuning,
    pub staff: StaffTuning,
    pub strings: Strings,
    /// The language's `MoneyStyle`: 1 shows the barometer in hPa rather
    /// than inHg.
    pub money_style: u32,
}

/// Something that must be built before something else can be (the
/// `predecessors` lists, checked in 0x470460).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requirement {
    /// "Any Powerplant".
    AnyPowerPlant,
    Structure(usize),
    Lift(usize),
    Run(usize),
    Park(usize),
    /// A name nothing in the game has; as in the original, never met.
    Unknown(String),
}

const ANY_POWER_PLANT: &str = "Any Powerplant";

/// A `predecessors` list, by name until `resolve_requirements` matches
/// the names to definitions.
fn requirements(v: Option<&str>) -> Vec<Requirement> {
    split_list(v).into_iter().map(Requirement::Unknown).collect()
}

impl Requirement {
    /// The name the data gives it, for the build menu's list of what is
    /// still needed.
    pub fn name<'a>(&'a self, defs: &'a Defs) -> &'a str {
        match self {
            Requirement::AnyPowerPlant => ANY_POWER_PLANT,
            Requirement::Structure(i) => &defs.structures[*i].unique_name,
            Requirement::Lift(i) => &defs.lifts[*i].unique_name,
            Requirement::Run(i) => &defs.runs[*i].unique_name,
            Requirement::Park(i) => &defs.parks[*i].unique_name,
            Requirement::Unknown(n) => n,
        }
    }
}

fn split_list(v: Option<&str>) -> Vec<String> {
    v.map(|s| {
        s.split(',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

pub(crate) fn load_ini(assets: &Assets, rel: &str) -> Ini {
    match assets.resolve(rel) {
        Some(p) => Ini::load(p).unwrap_or_default(),
        None => {
            log::warn!("missing {rel}");
            Ini::default()
        }
    }
}

/// Loads an ini's structures, with the names of what upgradeable ones
/// upgrade to, for resolving once all are loaded.
fn structures_from(ini: &Ini, out: &mut Vec<StructureDef>, upgrades: &mut Vec<Option<String>>) {
    let Some(list) = ini.section("Structures") else { return };
    for (cat, count) in &list.entries {
        let n = crate::formats::ini::parse_f32(count).unwrap_or(0.0) as i32;
        for i in 1..=n {
            if let Some(s) = ini.section(&format!("{cat}{i}")) {
                out.push(StructureDef::from_section(cat, s));
                upgrades.push(s.get("upgrade_name").filter(|_| s.bool_or("upgradeable", false)).map(|n| n.trim().to_string()));
            }
        }
    }
}

impl Defs {
    pub fn load(assets: &Assets) -> Result<Defs> {
        let mut structures = Vec::new();
        let mut upgrades = Vec::new();
        for f in [
            "structures",
            "structures_service",
            "structures_utility",
            "structures_maintenance",
            "lodges",
        ] {
            structures_from(&load_ini(assets, &format!("data/scenery/{f}.ini")), &mut structures, &mut upgrades);
        }
        for (i, name) in upgrades.iter().enumerate() {
            structures[i].upgrade = name.as_ref().and_then(|n| structures.iter().position(|d| d.unique_name.eq_ignore_ascii_case(n)));
        }

        let li = load_ini(assets, "data/scenery/lifts.ini");
        let mut lifts = Vec::new();
        if let Some(s) = li.section("Lift") {
            let count = s.i32_or("count", 0);
            for i in 0..count {
                let p = format!("lift{i}");
                let head: Vec<String> = s
                    .get(&p)
                    .unwrap_or("")
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .collect();
                let num = |k: &str, d: f32| s.f32_or(&format!("{p}_{k}"), d);
                let mut chairs = Vec::new();
                for c in 0..8 {
                    if let Some(m) = s.get(&format!("{p}_chair{c}")) {
                        chairs.push(m.to_string());
                    }
                }
                lifts.push(LiftDef {
                    index: i as usize,
                    name_key: head.first().cloned().unwrap_or_default(),
                    unique_name: head.get(1).cloned().unwrap_or_default(),
                    desc_key: s.str_or(&format!("{p}_description"), "").to_string(),
                    begin: lift_meshes(s, &format!("{p}_begin")),
                    middle: lift_meshes(s, &format!("{p}_middle")),
                    end: lift_meshes(s, &format!("{p}_end")),
                    chairs,
                    show_skiers: num("show_skiers", 1.0) != 0.0,
                    base_cost: num("basic_construction_cost", 1000.0),
                    cost_per_pole: num("construction_cost_per_pole", 100.0),
                    ratings_points: num("ratings_points", 5.0),
                    max_poles: num("max_poles", 10.0) as i32,
                    pole_spacing: num("pole_spacing", 50.0),
                    predecessors: requirements(s.get(&format!("{p}_predecessors"))),
                    speed: num("speed", 8.0),
                    capacity: num("capacity", 1.0) as i32,
                    secs_between_chairs: num("secs_between_chairs", 5.0),
                    start_radius: num("startradius", 3.0),
                    end_radius: num("endradius", 2.0),
                    path_radius: num("pathradius", 1.0),
                    wear_per_day: num("wear_and_tear", 0.05),
                    operations_per_day: num("operations_cost", 50.0),
                    maintenance_per_wear: num("maintenance_cost", 10.0),
                    menu_sound: s.str_or(&format!("{p}_menu_sound"), "").to_string(),
                    begin_pts: Vec::new(),
                    middle_pts: Vec::new(),
                    end_pts: Vec::new(),
                });
            }
        }

        let pts = |rel: &str| -> Vec<(AttachPoint, glam::Vec3)> {
            assets
                .resolve(rel)
                .and_then(|p| crate::formats::mesh::Mesh::load(p).ok())
                .map_or(Vec::new(), |m| m.attach)
        };
        for l in &mut lifts {
            l.begin_pts = pts(&l.begin.day);
            l.middle_pts = pts(&l.middle.day);
            l.end_pts = pts(&l.end.day);
        }

        let ri = load_ini(assets, "data/scenery/runmarker.ini");
        let mut runs = Vec::new();
        if let Some(list) = ri.section("Structures") {
            let n = list.i32_or("run_marker", 0);
            for i in 1..=n {
                if let Some(s) = ri.section(&format!("run_marker{i}")) {
                    runs.push(RunMarkerDef {
                        key: s.name.clone(),
                        mesh: {
                            let m = s.str_or("mesh", "");
                            if m.is_empty() || m.contains('.') { m.to_string() } else { format!("{m}.MESH") }
                        },
                        name_key: s.str_or("name", "").to_string(),
                        unique_name: s.str_or("unique_name", "").to_string(),
                        desc_key: s
                            .get("description")
                            .or(s.get("desciption"))
                            .unwrap_or("")
                            .to_string(),
                        cost: s.f32_or("cost", 150.0),
                        difficulty: s.i32_or("type", 0),
                        predecessors: requirements(s.get("predecessors")),
                        ratings_points: s.f32_or("ratings_points", 50.0),
                    });
                }
            }
        }

        let oi = load_ini(assets, "data/scenery/objects.ini");
        let mut decos = Vec::new();
        if let Some(list) = oi.section("Objects") {
            for i in 1..=list.i32_or("count", 0) {
                if let Some(s) = oi.section(&format!("Object{i}")) {
                    let mut meshes = MeshSet::from_section(s);
                    for m in [
                        &mut meshes.day,
                        &mut meshes.light_snow,
                        &mut meshes.heavy_snow,
                        &mut meshes.night,
                        &mut meshes.night_light_snow,
                        &mut meshes.night_heavy_snow,
                    ] {
                        if !m.is_empty() && !m.to_ascii_lowercase().starts_with("data") {
                            *m = format!("data/scenery/{m}");
                        }
                    }
                    decos.push(DecoDef {
                        key: s.name.clone(),
                        meshes,
                        name_key: s.str_or("name", "").to_string(),
                        unique_name: s.str_or("unique_name", "").to_string(),
                        cost: s.f32_or("cost", 0.0),
                        preview_zoom: s.f32_or("preview_zoom", 1.0).clamp(0.0, 2.0) as u8,
                    });
                }
            }
        }

        let ji = load_ini(assets, "data/scenery/jumps.ini");
        let mut parks = Vec::new();
        if let Some(s) = ji.section("Jump") {
            for i in 0..s.i32_or("count", 0) {
                let p = format!("jump{i}");
                let head: Vec<String> = s
                    .get(&p)
                    .unwrap_or("")
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .collect();
                let set = |prefix: &str| -> [String; 5] {
                    ["start", "gate", "tile", "ramp", "end"]
                        .map(|k| s.str_or(&format!("{p}_{prefix}{k}"), "").to_string())
                };
                parks.push(ParkDef {
                    name_key: head.first().cloned().unwrap_or_default(),
                    unique_name: head.get(1).cloned().unwrap_or_default(),
                    desc_key: s.str_or(&format!("{p}_description"), "").to_string(),
                    width: s.i32_or(&format!("{p}_width"), 2),
                    pieces: [set(""), set("S_"), set("SM_")],
                    cost: s.f32_or(&format!("{p}_basic_construction_cost"), 20000.0),
                    ratings_points: s.f32_or(&format!("{p}_ratings_points"), 25.0),
                    operations_per_day: s.f32_or(&format!("{p}_operations_cost"), 0.0),
                    predecessors: requirements(s.get(&format!("{p}_predecessors"))),
                });
            }
        }

        let rs = load_ini(assets, "data/scenery/resort.ini");
        let init = rs.section("init").cloned().unwrap_or_default();
        let lvl = rs.section("Level").cloned().unwrap_or_default();
        let resort = ResortTuning {
            seconds_per_day: init.f32_or("seconds_per_sim_day", 60.0),
            weeks_per_season: init.i32_or("sim_weeks_per_sim_season", 20),
            probability_snow: init.f32_or("probability_snow", 0.6),
            probability_rain: init.f32_or("probability_rain", 0.0),
            probability_hail: init.f32_or("probability_hail", 0.0),
            precipitation_interval: init.f32_or("precipitation_interval", 120.0),
            probability_fog: init.f32_or("probability_fog", 0.0),
            min_wind: init.f32_or("min_wind_speed", 0.0),
            max_wind: init.f32_or("max_wind_speed", 45.0),
            min_temperature: init.f32_or("min_temperature", -20.0),
            max_temperature: init.f32_or("max_temperature", 35.0),
            caps: AttractCaps {
                population: init.f32_or("population_attract_cap", 50.0),
                happiness: init.f32_or("happiness_attract_cap", 50.0),
                lifts: init.f32_or("lift_attract_cap", 160.0),
                beginner: init.f32_or("beginner_run_attract_cap", 30.0),
                intermediate: init.f32_or("intermediate_run_attract_cap", 30.0),
                advanced: init.f32_or("advanced_run_attract_cap", 30.0),
                expert: init.f32_or("expert_run_attract_cap", 60.0),
                lodging: init.f32_or("lodging_attract_cap", 210.0),
                food: init.f32_or("food_attract_cap", 100.0),
                school: init.f32_or("school_attract_cap", 50.0),
                rental: init.f32_or("rental_attract_cap", 50.0),
                entertainment: init.f32_or("entertain_attract_cap", 120.0),
                retail: init.f32_or("retail_attract_cap", 50.0),
                ski_toys: init.f32_or("skitoys_attract_cap", 50.0),
                safety: init.f32_or("safety_attract_cap", 60.0),
                clean_snow: init.f32_or("clean_snow_attract_cap", 30.0),
            },
            initial_cash: lvl.f32_or("inital_cash", 10000.0),
        };

        let si = load_ini(assets, "data/scenery/skiers.ini");
        let ctl = si.section("control").cloned().unwrap_or_default();
        let dist = si.section("distribution").cloned().unwrap_or_default();
        let skill = |name: &str| {
            let s = si.section(name).cloned().unwrap_or_default();
            SkillTuning {
                probability_crash: s.f32_or("probability_crash", 0.001),
                probability_injury: s.f32_or("probability_injury", 0.001),
                probability_needs_skis: s.f32_or("probability_need_skies", 0.5),
                min_slope: s.f32_or("target_min_slope", 5.0),
                max_slope: s.f32_or("target_max_slope", 20.0),
            }
        };
        let needs3 = |p: &str| Needs {
            warmth: ctl.f32_or(&format!("WarmthTick{p}"), 0.0),
            entertainment: -ctl.f32_or(&format!("BoredomTick{p}"), 0.0),
            food: -ctl.f32_or(&format!("HungerTick{p}"), 0.0),
            rest: ctl.f32_or(&format!("EnergyTick{p}"), 0.0),
            bathroom: -ctl.f32_or(&format!("WeeWeeTick{p}"), 0.0),
            health: 0.0,
        };
        let skiers = SkierTuning {
            danger_min_slope: ctl.f32_or("danger_min_slope", 60.0),
            danger_max_slope: ctl.f32_or("danger_max_slope", 70.0),
            max_skiers: ctl.i32_or("max_skiers", 100),
            max_skiers_5_star: ctl.i32_or("max_skiers_by_rating", 200),
            max_lodging: ctl.i32_or("max_skiers_lodging", 50),
            lodging_factor: ctl.f32_or("max_skiers_lodging_factor", 2.0),
            max_lifts: ctl.i32_or("max_skiers_lifts", 70),
            lifts_factor: ctl.f32_or("max_skiers_lifts_factor", 1.2),
            per_second_empty: ctl.f32_or("skiers_per_second_empty", 0.77),
            per_second_half: ctl.f32_or("skiers_per_second_half", 0.4),
            tick_in_line: needs3("InLine"),
            tick_on_lift: needs3("OnLift"),
            energy_on_ropetow: ctl.f32_or("EnergyTickOnRopetow", 0.0),
            warmth_on_gondola: ctl.f32_or("WarmthTickOnGondola", 0.0),
            tick_normal: needs3("Normal"),
            energy_uphill: ctl.f32_or("EnergyTickUphill", -0.0008),
            boredom_fast: ctl.f32_or("BoredomTickFast", 0.0),
            bad_weather_boredom: ctl.f32_or("BadWeatherBoredomTick", 0.0),
            cold_weather_warmth: ctl.f32_or("ColdWeatherWarmthTick", -0.0007),
            maintenance_wages: ctl.f32_or("maintennance_wages_per_day", 40.0),
            patrol_wages: ctl.f32_or("patrol_wages_per_day", 40.0),
            visit_days: ctl.f32_or("normal_visit_days", 10.0),
            variance_days: ctl.f32_or("normal_variance_days", 5.0),
            starting_cash: ctl.f32_or("normal_starting_cash", 1000.0),
            variance_cash: ctl.f32_or("normal_variance_cash", 500.0),
            distribution: [
                dist.f32_or("percent_skiers", 0.5),
                dist.f32_or("percent_beginner", 0.4),
                dist.f32_or("percent_intermediate", 0.25),
                dist.f32_or("percent_advanced", 0.2),
                dist.f32_or("percent_expert", 0.15),
            ],
            skill: [
                skill("skier_beginner"),
                skill("skier_intermediate"),
                skill("skier_advanced"),
                skill("skier_expert"),
            ],
            litter_interval: si
                .section("behavior")
                .map(|s| s.f32_or("litter_interval", 1000.0))
                .unwrap_or(1000.0),
            pee_on_snow: si.section("behavior").is_some_and(|s| s.bool_or("pee_on_snow", false)),
        };

        let sti = load_ini(assets, "data/scenery/staff.ini");
        let st = sti.section("STATS").cloned().unwrap_or_default();
        let staff = StaffTuning {
            patrol_wages: st.f32_or("patrol_wages_per_day", 20.0),
            maintenance_wages: st.f32_or("maintenance_wages_per_day", 30.0),
            chopper_wages: st.f32_or("chopperpatrol_wages_per_day", 40.0),
            sanitation_wages: st.f32_or("sanitation_wages_per_day", 30.0),
            instructor_wages: st.f32_or("instructor_wages_per_day", 35.0),
        };

        let lang = load_ini(assets, "data/scenery/English.ini");
        let ls = lang.section("SETTINGS").cloned().unwrap_or_default();
        let ext = ls.str_or("LanguageExtension", "eng").to_string();
        let strings = assets
            .resolve("data/ui/shell.eng")
            .and_then(|p| p.parent())
            .and_then(|dir| Strings::load_dir(dir, &ext).ok())
            .unwrap_or_default();
        log::info!(
            "defs: {} structures, {} lifts, {} run markers, {} objects, {} strings",
            structures.len(),
            lifts.len(),
            runs.len(),
            decos.len(),
            strings.len()
        );
        let mut defs = Defs {
            structures,
            lifts,
            runs,
            decos,
            parks,
            resort,
            skiers,
            staff,
            strings,
            money_style: ls.f32_or("MoneyStyle", 0.0) as u32,
        };
        defs.resolve_requirements();
        Ok(defs)
    }

    /// What a unique name names (see `Requirement`).
    pub fn requirement(&self, name: &str) -> Requirement {
        let eq = |n: &str| n.eq_ignore_ascii_case(name);
        if eq(ANY_POWER_PLANT) {
            Requirement::AnyPowerPlant
        } else if let Some(i) = self.structures.iter().position(|d| eq(&d.unique_name)) {
            Requirement::Structure(i)
        } else if let Some(i) = self.lifts.iter().position(|d| eq(&d.unique_name)) {
            Requirement::Lift(i)
        } else if let Some(i) = self.runs.iter().position(|d| eq(&d.unique_name)) {
            Requirement::Run(i)
        } else if let Some(i) = self.parks.iter().position(|d| eq(&d.unique_name)) {
            Requirement::Park(i)
        } else {
            log::warn!("requirement {name:?} names nothing");
            Requirement::Unknown(name.to_string())
        }
    }

    /// Matches the `predecessors` names to definitions, by unique name.
    fn resolve_requirements(&mut self) {
        let resolve = |defs: &Defs, list: &[Requirement]| -> Vec<Requirement> {
            list.iter()
                .map(|r| match r {
                    Requirement::Unknown(n) => defs.requirement(n),
                    r => r.clone(),
                })
                .collect()
        };
        for i in 0..self.structures.len() {
            self.structures[i].predecessors = resolve(self, &self.structures[i].predecessors);
        }
        for i in 0..self.lifts.len() {
            self.lifts[i].predecessors = resolve(self, &self.lifts[i].predecessors);
        }
        for i in 0..self.runs.len() {
            self.runs[i].predecessors = resolve(self, &self.runs[i].predecessors);
        }
        for i in 0..self.parks.len() {
            self.parks[i].predecessors = resolve(self, &self.parks[i].predecessors);
        }
    }

    /// A key from the game's data files (see `Strings::lookup`).
    pub fn lookup<'a>(&'a self, key: &'a str) -> &'a str {
        if key.is_empty() {
            return key;
        }
        self.strings.lookup(key)
    }

    /// A text the code names (see `Strings::text`).
    pub fn text(&self, k: impl crate::text::TextKey) -> std::borrow::Cow<'_, str> {
        self.strings.text(k)
    }

    pub fn structure_name<'a>(&'a self, d: &'a StructureDef) -> &'a str {
        match self.strings.get(&d.name_key) {
            Some(s) if !s.is_empty() => s,
            _ => &d.unique_name,
        }
    }
}
