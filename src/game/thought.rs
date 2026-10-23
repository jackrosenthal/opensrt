//! What guests think: the bubble over their heads and the line in their
//! window. Most thoughts are a category of the string tables (`ST_*`)
//! with numbered variants, `ST_HUNGRY_01`, `ST_HUNGRY_02`, ...; the
//! wounded ones are single texts.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::text::{Text, TextKey};

/// Saved by the original key, as earlier saves stored it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Thought {
    #[serde(rename = "ST_HUNGRY")]
    Hungry,
    #[serde(rename = "ST_COLD")]
    Cold,
    #[serde(rename = "ST_NEED_TO_WEE_WEE")]
    NeedToWeeWee,
    #[serde(rename = "ST_TIRED")]
    Tired,
    #[serde(rename = "ST_BORED")]
    Bored,
    #[serde(rename = "ST_SKIING")]
    Skiing,
    #[serde(rename = "ST_GOING_TO_SKI_LIFT")]
    GoingToSkiLift,
    #[serde(rename = "ST_NO_RUNS_OF_MY_SKILL_LEVEL")]
    NoRunsOfMySkillLevel,
    #[serde(rename = "ST_MAD_LIFTS_TOO_CROWDED")]
    MadLiftsTooCrowded,
    #[serde(rename = "ST_MAD_INLINE_TOO_LONG")]
    MadInlineTooLong,
    #[serde(rename = "ST_MAD_ROPE_TOW_TOO_STEEP")]
    MadRopeTowTooSteep,
    #[serde(rename = "ST_FRUSTRATED_LINES_TOO_LONG")]
    FrustratedLinesTooLong,
    #[serde(rename = "ST_LOOKING_FOR_RENTALS")]
    LookingForRentals,
    #[serde(rename = "ST_LOOKING_FOR_SKI_INSTRUCTION")]
    LookingForSkiInstruction,
    #[serde(rename = "ST_OUT_OF_CASH")]
    OutOfCash,
    #[serde(rename = "ST_TOO_EXPENSIVE")]
    TooExpensive,
    #[serde(rename = "SKIER_THOUGHT_WOUNDED")]
    Wounded,
    #[serde(rename = "SKIER_THOUGHT_WOUNDED_WAITING")]
    WoundedWaiting,
}

/// The `n`th variant (from 1) of a thought category.
pub struct Variant(pub Thought, pub u8);

impl TextKey for Variant {
    fn key(&self) -> Cow<'static, str> {
        Cow::Owned(format!("{}_{:02}", self.0.category(), self.1))
    }
}

impl Thought {
    /// The string tables' category, `ST_HUNGRY` and so on.
    fn category(self) -> &'static str {
        use Thought::*;
        match self {
            Hungry => "ST_HUNGRY",
            Cold => "ST_COLD",
            NeedToWeeWee => "ST_NEED_TO_WEE_WEE",
            Tired => "ST_TIRED",
            Bored => "ST_BORED",
            Skiing => "ST_SKIING",
            GoingToSkiLift => "ST_GOING_TO_SKI_LIFT",
            NoRunsOfMySkillLevel => "ST_NO_RUNS_OF_MY_SKILL_LEVEL",
            MadLiftsTooCrowded => "ST_MAD_LIFTS_TOO_CROWDED",
            MadInlineTooLong => "ST_MAD_INLINE_TOO_LONG",
            MadRopeTowTooSteep => "ST_MAD_ROPE_TOW_TOO_STEEP",
            FrustratedLinesTooLong => "ST_FRUSTRATED_LINES_TOO_LONG",
            LookingForRentals => "ST_LOOKING_FOR_RENTALS",
            LookingForSkiInstruction => "ST_LOOKING_FOR_SKI_INSTRUCTION",
            OutOfCash => "ST_OUT_OF_CASH",
            TooExpensive => "ST_TOO_EXPENSIVE",
            Wounded => Text::SkierThoughtWounded.key(),
            WoundedWaiting => Text::SkierThoughtWoundedWaiting.key(),
        }
    }

    /// A single text rather than a category of variants.
    pub fn text(self) -> Option<Text> {
        match self {
            Thought::Wounded => Some(Text::SkierThoughtWounded),
            Thought::WoundedWaiting => Some(Text::SkierThoughtWoundedWaiting),
            _ => None,
        }
    }

    /// When guests leave thinking this, the original reports it once so
    /// many have (0x485f60; table 0x651fe0): how many, and the message.
    pub fn departure_report(self) -> Option<(u32, Text)> {
        use Thought::*;
        Some(match self {
            NeedToWeeWee => (5, Text::ReSkierNeedPee),
            Hungry => (5, Text::ReSkierHungry),
            Cold => (5, Text::ReSkierCold),
            Tired => (5, Text::ReSkierTired),
            NoRunsOfMySkillLevel => (20, Text::ReMoreRunsNeeded),
            MadLiftsTooCrowded => (20, Text::ReSkierLinesTooLong),
            LookingForRentals => (20, Text::ReNeedRentals),
            LookingForSkiInstruction => (10, Text::ReSkiSchoolsWanted),
            FrustratedLinesTooLong => (20, Text::ReSkierLinesTooLong),
            Bored => (5, Text::ReSkierBored),
            OutOfCash => (30, Text::ReSkierOutOfCash),
            TooExpensive => (30, Text::ReSkierTooExpensive),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_by_original_key() {
        assert_eq!(serde_json::to_string(&Thought::NeedToWeeWee).unwrap(), "\"ST_NEED_TO_WEE_WEE\"");
        assert_eq!(serde_json::from_str::<Thought>("\"SKIER_THOUGHT_WOUNDED\"").unwrap(), Thought::Wounded);
        assert_eq!(Variant(Thought::Hungry, 3).key(), "ST_HUNGRY_03");
    }
}
