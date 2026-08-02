//! Strongly-typed shapes for the parts of an `Individual` that were previously
//! held as loose `serde_json::Value` blobs. Each struct keeps a flattened
//! `extra` catch-all (matching the rest of this codebase's convention, e.g.
//! `Individual`/`WorldState`) so unknown/experimental JSON fields still
//! round-trip instead of being silently dropped -- but every field an engine
//! actually reads or writes is now a real, compiler-checked field instead of
//! a stringly-keyed `.get("...").and_then(Value::as_f64())` lookup.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

fn half() -> f64 {
    0.5
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Allele {
    /// `None` only for the absent second allele of a hemizygous (X-linked, male) locus.
    #[serde(default)]
    pub value: Option<f64>,
    #[serde(default)]
    pub origin: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Locus {
    #[serde(rename = "locusId", default)]
    pub locus_id: String,
    #[serde(default)]
    pub chromosome: Option<String>,
    #[serde(default)]
    pub allele1: Allele,
    #[serde(default)]
    pub allele2: Allele,
    #[serde(rename = "expressionType", default)]
    pub expression_type: String,
    #[serde(rename = "trait", default)]
    pub trait_name: String,
}

pub type Genome = HashMap<String, Locus>;

/// The specific subset of the universal human articulatory repertoire this
/// simulation's population can actually produce, derived once from the two
/// founders' own FOXP2/CNTNAP2 alleles (see `language::derive_phoneme_palette`)
/// -- not a fixed, developer-picked list shared by every simulation. Every
/// procedurally generated word (vocabulary, personal names) is built from
/// this palette, never from a hardcoded consonant/vowel string.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PhonemePalette {
    #[serde(default)]
    pub consonants: Vec<char>,
    #[serde(default)]
    pub vowels: Vec<char>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EpigeneticLocus {
    #[serde(default = "half")]
    pub methylation: f64,
    #[serde(default)]
    pub last_modified: Option<i32>,
}

pub type Epigenome = HashMap<String, EpigeneticLocus>;

/// The ~50 genome-derived traits computed once by `compute_phenotype`, plus a
/// handful (`stress_reactivity`, `aggression`, `oxytocin_sensitivity`,
/// `learning_rate`, `immune_strength`) that epigenetics.rs is allowed to drift
/// slightly over a lifetime in response to methylation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Phenotype {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default = "half")]
    pub height_factor: f64,
    #[serde(default = "half")]
    pub physical_strength: f64,
    #[serde(default = "half")]
    pub physical_endurance: f64,
    #[serde(default = "half")]
    pub endurance: f64,
    #[serde(default = "half")]
    pub fluid_intelligence: f64,
    #[serde(default = "half")]
    pub working_memory: f64,
    #[serde(default = "half")]
    pub conscientiousness: f64,
    #[serde(default = "half")]
    pub learning_rate: f64,
    #[serde(default = "half")]
    pub language_capacity: f64,
    #[serde(default = "half")]
    pub language_learning: f64,
    #[serde(default = "half")]
    pub social_bonding: f64,
    #[serde(default = "half")]
    pub social_drive: f64,
    #[serde(default = "half")]
    pub oxytocin_sensitivity: f64,
    #[serde(default = "half")]
    pub empathy: f64,
    #[serde(default = "half")]
    pub cooperation: f64,
    #[serde(default = "half")]
    pub altruism: f64,
    #[serde(default = "half")]
    pub parental_care: f64,
    #[serde(default = "half")]
    pub aggression: f64,
    #[serde(default = "half")]
    pub dominance: f64,
    #[serde(default = "half")]
    pub curiosity: f64,
    #[serde(default = "half")]
    pub risk_tolerance: f64,
    #[serde(default = "half")]
    pub innovation: f64,
    #[serde(default = "half")]
    pub artistic_sense: f64,
    #[serde(default = "half")]
    pub serotonin: f64,
    #[serde(default = "half")]
    pub stress_resilience: f64,
    #[serde(default = "half")]
    pub stress_reactivity: f64,
    #[serde(default = "half")]
    pub health_resilience: f64,
    #[serde(default = "half")]
    pub anxiety: f64,
    #[serde(default = "half")]
    pub independence: f64,
    #[serde(default = "half")]
    pub xenophobia: f64,
    #[serde(default = "half")]
    pub metabolism: f64,
    #[serde(default = "half")]
    pub immune_strength: f64,
    #[serde(default)]
    pub max_lifespan: f64,
    #[serde(default = "half")]
    pub fertility: f64,
    #[serde(default = "half")]
    pub consciousness_potential: f64,
    #[serde(default)]
    pub belief_capacity: f64,
    #[serde(default = "half")]
    pub religiosity: f64,
    #[serde(default = "half")]
    pub self_awareness: f64,
    #[serde(default)]
    pub eye_color: String,
    #[serde(default)]
    pub hair_color: String,
    #[serde(default = "half")]
    pub skin_tone: f64,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Phenotype {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).expect("all Phenotype fields have defaults")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Health {
    #[serde(default = "one")]
    pub hp: f64,
    #[serde(default = "one")]
    pub max_hp: f64,
    #[serde(default = "one")]
    pub calories: f64,
    #[serde(default = "one")]
    pub hydration: f64,
    #[serde(default)]
    pub disease: Option<String>,
    #[serde(default = "half")]
    pub disease_resistance: f64,
    #[serde(default)]
    pub injuries: Vec<Value>,
    /// The day conception happened; `None` means not currently pregnant.
    /// (A JSON `null` deserializes to `None`, unlike the old `Value` field
    /// where the mere presence of the "pregnancy" key was mistaken for
    /// "is pregnant" -- see the reproduction.rs bugfix this replaces.)
    #[serde(default)]
    pub pregnancy: Option<i32>,
    #[serde(default)]
    pub pregnancy_day: Option<i32>,
    #[serde(default)]
    pub microbiome_immunity: Option<f64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn one() -> f64 {
    1.0
}

impl Default for Health {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).expect("all Health fields have defaults")
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Volatile {
    #[serde(default)]
    pub satiation: f64,
    #[serde(default)]
    pub age: i32,
    #[serde(rename = "_waterExperience", default)]
    pub water_experience: f64,
    #[serde(default)]
    pub generation: i32,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Mind {
    /// Cardinal rule: only `consciousness::update_consciousness` may write this field.
    #[serde(default)]
    pub consciousness: f64,
    #[serde(default)]
    pub death_awareness: bool,
    #[serde(default = "half")]
    pub emotional_state: f64,
    #[serde(default)]
    pub stress: f64,
    #[serde(rename = "_volatile", default)]
    pub volatile: Volatile,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Mind {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).expect("all Mind fields have defaults")
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Social {
    #[serde(default)]
    pub relationships: HashMap<String, f64>,
    #[serde(default)]
    pub reputation: f64,
    #[serde(default)]
    pub status: f64,
    #[serde(default)]
    pub has_mate: bool,
    #[serde(default)]
    pub mate_id: Option<String>,
    #[serde(default)]
    pub children_ids: Vec<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Language {
    #[serde(default)]
    pub stage: i32,
    #[serde(default)]
    pub stage_name: String,
    #[serde(default)]
    pub vocabulary: HashMap<String, String>,
    #[serde(default)]
    pub grammar: bool,
    #[serde(default)]
    pub writing: bool,
    #[serde(default)]
    pub foxp2_expression: f64,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Language {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).expect("all Language fields have defaults")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Psychology {
    #[serde(default = "calm")]
    pub mental_state: String,
    #[serde(default = "half")]
    pub wellbeing: f64,
    #[serde(default)]
    pub attachment_style: String,
    #[serde(default = "half")]
    pub stress_level: f64,
    #[serde(default)]
    pub trauma_events: Vec<Value>,
    #[serde(default)]
    pub relationships: HashMap<String, f64>,
    #[serde(default)]
    pub theory_of_mind: i32,
    #[serde(default)]
    pub self_awareness: bool,
    #[serde(default = "half")]
    pub life_satisfaction: f64,
    #[serde(default)]
    pub trauma_anxiety: f64,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn calm() -> String {
    "calm".to_string()
}

impl Default for Psychology {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).expect("all Psychology fields have defaults")
    }
}

fn cortisol_default() -> f64 {
    0.3
}
fn dopamine_default() -> f64 {
    0.35
}
fn adrenaline_default() -> f64 {
    0.05
}
fn sex_hormone_default() -> f64 {
    0.1
}
fn oxytocin_default() -> f64 {
    0.15
}

/// Dynamic, tick-by-tick circulating hormone levels -- distinct from the
/// static genome-derived phenotype traits (`oxytocin_sensitivity`,
/// `serotonin`, `aggression`, `dominance`, ...) which represent *receptor
/// sensitivity*/genetic predisposition, not an actual secreted amount. See
/// `hormones.rs`, the sole writer of every field here (cardinal rule, same
/// as `mind.consciousness`): every value is a formula over genetics
/// (phenotype/sex/age) and this tick's real, already-tracked state
/// (stress_level, hp, pregnancy, group membership) -- never scripted per
/// individual.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hormones {
    /// HPA-axis stress hormone. Tracks `psychology.stress_level`, scaled by
    /// the individual's own genetic `stress_reactivity`.
    #[serde(default = "cortisol_default")]
    pub cortisol: f64,
    /// Acute fight-or-flight response. Near-zero at rest; spikes fast on a
    /// same-tick acute threat (disaster, exile, critical HP) and clears fast
    /// once the threat passes -- much quicker dynamics than cortisol.
    #[serde(default = "adrenaline_default")]
    pub adrenaline: f64,
    /// Sex-differentiated; follows a real puberty ramp (childhood -> ~17y)
    /// and senescence decline (andropause), modulated by genetic dominance.
    #[serde(default = "sex_hormone_default")]
    pub testosterone: f64,
    /// Sex-differentiated; follows a real puberty ramp, cycles with
    /// pregnancy, and declines after the fertile window (menopause-like),
    /// modulated by genetic fertility.
    #[serde(default = "sex_hormone_default")]
    pub estrogen: f64,
    /// Reward/motivation. Rises with a same-tick positive nutritional swing
    /// (successful foraging after hunger), decays otherwise toward a
    /// genetic baseline (curiosity/risk_tolerance).
    #[serde(default = "dopamine_default")]
    pub dopamine: f64,
    /// Dynamic circulating bonding hormone -- distinct from the static
    /// genetic `oxytocin_sensitivity` (receptor sensitivity) it's scaled
    /// by. Rises with group presence and surges on mating.
    #[serde(default = "oxytocin_default")]
    pub oxytocin: f64,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Hormones {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).expect("all Hormones fields have defaults")
    }
}
