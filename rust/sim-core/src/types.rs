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
fn tsh_default() -> f64 {
    0.5
}
fn thyroid_default() -> f64 {
    0.5
}
fn insulin_default() -> f64 {
    0.4
}
fn glucagon_default() -> f64 {
    0.3
}
fn leptin_default() -> f64 {
    0.3
}
fn ghrelin_default() -> f64 {
    0.4
}
fn growth_hormone_default() -> f64 {
    0.3
}
fn dhea_default() -> f64 {
    0.2
}
fn norepinephrine_default() -> f64 {
    0.1
}
// Shared low/mid default helpers for the Group A/B hormones below, whose
// individual resting baselines don't need their own distinctly-named
// constant the way the original six/fourteen's did.
fn d01() -> f64 {
    0.1
}
fn d02() -> f64 {
    0.2
}
fn d03() -> f64 {
    0.3
}
fn d04() -> f64 {
    0.4
}
fn d05() -> f64 {
    0.5
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
    /// Adrenocorticotropic hormone -- the pituitary signal upstream of
    /// cortisol in the real HPA axis. Tracks `stress_level` directly and
    /// faster than cortisol; cortisol's own target is now driven by this
    /// (not stress_level directly), a genuine two-stage cascade.
    #[serde(default = "cortisol_default")]
    pub acth: f64,
    /// Thyroid-stimulating hormone -- rises under real negative feedback
    /// when `thyroid` runs low (the actual HPT-axis feedback loop), driving
    /// it back up the following tick.
    #[serde(default = "tsh_default")]
    pub tsh: f64,
    /// Luteinizing hormone -- the gonadotropin pulse upstream of
    /// testosterone/estrogen production. Follows the same puberty ramp,
    /// with a same-tick surge on mating (the actual physiological trigger
    /// for a mating-linked sex-hormone surge, not the sex hormones
    /// themselves).
    #[serde(default = "sex_hormone_default")]
    pub lh: f64,
    /// Combined T3/T4 thyroid hormone -- drives basal metabolic tempo.
    /// Falls under sustained undernourishment (real "sick euthyroid"
    /// energy-conservation adaptation), rises under TSH stimulation.
    #[serde(default = "thyroid_default")]
    pub thyroid: f64,
    /// Rises with a well-fed satiation swing (glucose uptake/storage
    /// signal); the fast-acting complement to `glucagon`.
    #[serde(default = "insulin_default")]
    pub insulin: f64,
    /// Rises when hungry (energy-mobilization signal); the fast-acting
    /// complement to `insulin`. Sustained elevation reflects a genuine
    /// starvation-adaptation response -- see its bounded mortality-risk
    /// discount in `mortality.rs`.
    #[serde(default = "glucagon_default")]
    pub glucagon: f64,
    /// Slow-moving (weeks-scale) average of nutritional state -- a real
    /// long-term energy-reserve signal, distinct from insulin's fast
    /// same-tick response.
    #[serde(default = "leptin_default")]
    pub leptin: f64,
    /// Fast-moving acute hunger signal -- the inverse-and-quicker
    /// complement to leptin's slow trend.
    #[serde(default = "ghrelin_default")]
    pub ghrelin: f64,
    /// Growth hormone -- high through childhood/adolescence, declining in
    /// adulthood. Exposed for now with no direct feedback hook (see
    /// hormones.rs's own doc comment on why a real growth-curve hook is
    /// deferred).
    #[serde(default = "growth_hormone_default")]
    pub growth_hormone: f64,
    /// Adrenal androgen precursor -- peaks in young adulthood, declines
    /// with age in both sexes ("adrenopause"), independently of the
    /// sex-specific senescence curve. A shared secondary modulator on both
    /// testosterone and estrogen's own age-driven baseline.
    #[serde(default = "dhea_default")]
    pub dhea: f64,
    /// Surges after giving birth (lactation), decays slowly over
    /// subsequent ticks. See `apply_birth_surge`.
    #[serde(default = "adrenaline_default")]
    pub prolactin: f64,
    /// Pairs with estrogen but with its own, pregnancy-specific dynamic
    /// (luteal-phase pregnancy maintenance) rather than tracking the same
    /// puberty/senescence curve.
    #[serde(default = "sex_hormone_default")]
    pub progesterone: f64,
    /// Sustained (weeks-scale) vigilance/arousal tone -- slower than
    /// adrenaline's acute spike, faster than cortisol's own return to
    /// baseline. Sets adrenaline's own resting floor (the real locus
    /// coeruleus -> adrenal coupling).
    #[serde(default = "norepinephrine_default")]
    pub norepinephrine: f64,
    /// Approximated from `parental_care`/`cooperation` (both partly
    /// AVPR1A_01-driven already, see genome.rs) since raw locus values
    /// aren't exposed through the phenotype API. Oxytocin's male-leaning
    /// counterpart in real bonding/mate-guarding physiology -- see its
    /// extra weight in `psychology::process_bonding` for males.
    #[serde(default = "oxytocin_default")]
    pub vasopressin: f64,

    // ---- Group A: further pituitary/adrenal/immune hormones, each tied to
    // an existing system (HPG, POMC precursor pathway, microbiome/infection,
    // GH cascade, the metabolic pair) ----
    /// Follicle-stimulating hormone -- the other real gonadotropin, distinct
    /// from `lh`. Tracks the same puberty curve at a slower response rate.
    #[serde(default = "d01")]
    pub fsh: f64,
    /// Corticotropin-releasing hormone -- the hypothalamic signal upstream
    /// of even `acth` in the real HPA axis; now the actual driver of
    /// `acth`'s own target.
    #[serde(default = "cortisol_default")]
    pub crh: f64,
    /// Melanocyte-stimulating hormone -- shares POMC as a common precursor
    /// with ACTH/endorphin (real biology), so tracks `acth` directly.
    #[serde(default = "d03")]
    pub msh: f64,
    /// Beta-endorphin -- the third POMC-derived hormone; rises with acute
    /// pain (low HP) and reward (satiation swing), with a small bounded
    /// wellbeing boost when elevated (real endogenous analgesia/euphoria).
    #[serde(default = "d03")]
    pub endorphin: f64,
    /// Interleukin-6 -- inflammatory cytokine, real trigger is an active
    /// infection (`health.disease`); contributes to thyroid suppression
    /// during illness (the real cytokine-driven component of "sick
    /// euthyroid", alongside undernutrition).
    #[serde(default = "d01")]
    pub il6: f64,
    /// TNF-alpha -- inflammatory cytokine; real HPA-axis activator, adds to
    /// `acth`'s target during active infection.
    #[serde(default = "d01")]
    pub tnf_alpha: f64,
    /// Type-I interferon -- antiviral response tied to active infection;
    /// small bounded infection-severity discount in `microbiome.rs`.
    #[serde(default = "d01")]
    pub interferon: f64,
    /// Insulin-like growth factor 1 -- the real downstream liver signal GH
    /// actually acts through; tracks `growth_hormone` directly (cascade).
    #[serde(default = "d03")]
    pub igf1: f64,
    /// Fat-tissue hormone, real-world inversely related to `leptin`/body
    /// fat; nudges `insulin`'s own target down slightly when high (real
    /// insulin-sensitizing effect).
    #[serde(default = "d05")]
    pub adiponectin: f64,
    /// Neuropeptide Y -- potent hypothalamic appetite driver, real trigger
    /// is low `leptin`; amplifies `ghrelin`'s own target slightly (real
    /// NPY-ghrelin interaction).
    #[serde(default = "d03")]
    pub npy: f64,

    // ---- Group B1: digestive-hormone timescale layer over the existing
    // satiation signal (no literal stomach-contents state exists in this
    // simulation -- see hormones.rs's own doc comment for why these are
    // still real, distinctly-timed formulas rather than one signal repeated
    // eight times) ----
    /// Rises immediately on a same-tick "just ate" swing (real: released on
    /// food's arrival).
    #[serde(default = "d03")]
    pub gastrin: f64,
    /// Follows gastrin with a short lag (real: pH-regulation response to
    /// gastrin).
    #[serde(default = "d03")]
    pub secretin: f64,
    /// Slower satiety signal from a sustained well-fed swing (real:
    /// fat/protein-triggered, longer-acting than gastrin).
    #[serde(default = "d03")]
    pub cck: f64,
    /// Cyclic, rises specifically *between* meals (real: inter-meal gut
    /// motility driver) -- the inverse-timing complement to gastrin.
    #[serde(default = "d04")]
    pub motilin: f64,
    /// Rises with insulin (real: the incretin that potentiates
    /// insulin release after eating).
    #[serde(default = "d03")]
    pub gip: f64,
    /// Real broad GI-hormone inhibitor -- negative feedback against
    /// gastrin's own level, the same negative-feedback pattern as TSH/thyroid.
    #[serde(default = "d03")]
    pub somatostatin: f64,
    /// Long-acting satiety signal, slower than CCK (real: released after
    /// CCK, sustains fullness longer).
    #[serde(default = "d03")]
    pub pyy: f64,
    /// Self-regulates digestive-hormone secretion broadly; tracks
    /// somatostatin loosely (real: both part of the same inhibitory
    /// feedback family).
    #[serde(default = "d03")]
    pub pancreatic_polypeptide: f64,

    // ---- Group B2: cardiovascular/renal, proxied through the existing
    // hydration (blood-volume proxy) and hp (blood-loss/injury proxy)
    // signals -- no literal blood-pressure state exists; see hormones.rs ----
    /// Rises when `health.hydration` is low (real: low blood volume/pressure
    /// triggers renin release).
    #[serde(default = "d03")]
    pub renin: f64,
    /// Downstream of renin (real cascade).
    #[serde(default = "d03")]
    pub angiotensin_ii: f64,
    /// Downstream of angiotensin II; promotes water/salt retention -- small
    /// bounded discount to `mortality.rs`'s dehydration risk term (real
    /// adaptive water retention).
    #[serde(default = "d03")]
    pub aldosterone: f64,
    /// Atrial natriuretic peptide -- the real counter-regulatory signal to
    /// renin/aldosterone, rises when hydration is high rather than low.
    #[serde(default = "d03")]
    pub anp: f64,
    /// Brain natriuretic peptide -- tracks ANP (real: released alongside it
    /// under the same volume-overload trigger).
    #[serde(default = "d02")]
    pub bnp: f64,
    /// Erythropoietin -- real trigger is low oxygen/blood loss, proxied here
    /// by low `health.hp`; small bounded recovery-adjacent mortality
    /// discount when elevated (real: stimulates red-cell production).
    #[serde(default = "d03")]
    pub epo: f64,

    // ---- Group B3: bone/calcium, proxied through age and (for PTH/
    // calcitonin) the real estrogen-bone protective link -- no literal bone-
    // density state exists; see hormones.rs ----
    /// Parathyroid hormone -- real age-related rise (compensating for
    /// natural bone/calcium loss), sharply amplified in post-fertile females
    /// by low estrogen (real: estrogen protects bone; its decline drives
    /// PTH-mediated loss, i.e. osteoporosis). Small bounded elder-female
    /// mortality term in `mortality.rs`.
    #[serde(default = "d03")]
    pub pth: f64,
    /// Opposes PTH; real age-related decline.
    #[serde(default = "d04")]
    pub calcitonin: f64,
    /// Calcitriol (active vitamin D) -- real age-related production
    /// decline, modulated by genetic `health_resilience` (synthesis
    /// efficiency). No sunlight/UV signal exists in this simulation to
    /// drive it further.
    #[serde(default = "d05")]
    pub vitamin_d: f64,
    /// Bone-formation marker; tracks `growth_hormone` (real: bone formation
    /// is active during growth).
    #[serde(default = "d03")]
    pub osteocalcin: f64,

    // ---- Group B4: circadian. This simulation has no day/night cycle at
    // its one-tick-per-day resolution, so melatonin's real light-triggered
    // dynamic can't be modeled -- only its real age-related decline and its
    // real reciprocal coupling with cortisol (see hormones.rs) ----
    /// Real age-related production decline; normally suppresses cortisol,
    /// so its own decline (age, or chronic stress-driven suppression) feeds
    /// a small bounded rise into `acth`'s own target.
    #[serde(default = "d04")]
    pub melatonin: f64,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Hormones {
    fn default() -> Self {
        serde_json::from_value(Value::Object(Map::new())).expect("all Hormones fields have defaults")
    }
}
