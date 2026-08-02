use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::types::{Allele, Epigenome, Genome, Health, Hormones, Language, Mind, Phenotype, PhonemePalette, Psychology, Social};

/// `Locus::locus_id`/`chromosome`/`expression_type`/`trait_name` are constant
/// per locus (defined once by `biology::genome::LOCI`), yet were being
/// serialized on every single individual's `data_json` row -- for a 32-locus
/// genome that's the single biggest contributor to per-individual payload
/// size (measured ~7KB of a ~14KB row), rewritten to Postgres on every tick
/// batch regardless of whether anything about that individual actually
/// changed. Only `allele1`/`allele2` are genuinely per-individual, so only
/// those go over the wire; `hydrate_genome_metadata` reconstructs the rest
/// deterministically from the locus_id (the map key) on the way back in.
fn serialize_slim_genome<S>(genome: &Genome, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    #[derive(Serialize)]
    struct SlimLocus<'a> {
        allele1: &'a Allele,
        allele2: &'a Allele,
    }
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(Some(genome.len()))?;
    for (locus_id, locus) in genome {
        map.serialize_entry(locus_id, &SlimLocus { allele1: &locus.allele1, allele2: &locus.allele2 })?;
    }
    map.end()
}

fn deserialize_hydrated_genome<'de, D>(deserializer: D) -> Result<Genome, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut genome: Genome = Deserialize::deserialize(deserializer)?;
    crate::biology::genome::hydrate_genome_metadata(&mut genome);
    Ok(genome)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorldState {
    /// This simulation's derived sound palette (see `language::derive_phoneme_palette`).
    /// `None` only for states saved before this field existed; `tick::advance_one_day`
    /// self-heals it from the population's founders on the next tick.
    #[serde(default)]
    pub phoneme_palette: Option<PhonemePalette>,
    #[serde(default)]
    pub biome: Option<String>,
    #[serde(default)]
    pub season: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub food_abundance: Option<f64>,
    #[serde(default)]
    pub water_abundance: Option<f64>,
    #[serde(default)]
    pub alive_count: Option<usize>,
    #[serde(default)]
    pub current_day: Option<i32>,
    #[serde(default)]
    pub current_year: Option<i32>,
    /// Baseline centroid the band's last logged migration was measured from
    /// (see `tick::track_migration`). `None` until the first tick establishes
    /// an initial baseline; no migration is ever logged against a missing one.
    #[serde(default)]
    pub last_migration_x: Option<f64>,
    #[serde(default)]
    pub last_migration_y: Option<f64>,
    #[serde(default)]
    pub last_migration_day: Option<i32>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Individual {
    pub id: String,
    #[serde(default)]
    pub simulation_id: Option<String>,
    pub birth_day: i32,
    #[serde(default)]
    pub death_day: Option<i32>,
    #[serde(default)]
    pub alive: bool,
    #[serde(default)]
    pub is_dead: bool,
    #[serde(default)]
    pub is_founder: bool,
    #[serde(default)]
    pub sex: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub age_days: Option<i32>,
    #[serde(default)]
    pub generation: Option<i32>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub home_x: Option<f64>,
    #[serde(default)]
    pub home_y: Option<f64>,
    #[serde(default)]
    pub parent_1_id: Option<String>,
    #[serde(default)]
    pub parent_2_id: Option<String>,
    #[serde(default)]
    pub known_techs: Vec<String>,
    #[serde(default, serialize_with = "serialize_slim_genome", deserialize_with = "deserialize_hydrated_genome")]
    pub genome: Genome,
    #[serde(default)]
    pub phenotype: Phenotype,
    #[serde(default)]
    pub epigenome: Epigenome,
    #[serde(default)]
    pub health: Health,
    #[serde(default)]
    pub mind: Mind,
    #[serde(default)]
    pub social: Social,
    #[serde(default)]
    pub skills: Vec<Value>,
    #[serde(default)]
    pub beliefs: HashSet<String>,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub memory: Value,
    #[serde(default)]
    pub psychology: Psychology,
    #[serde(default)]
    pub hormones: Hormones,
    #[serde(default)]
    pub inventory: HashMap<String, f64>,
    #[serde(default)]
    pub inbreeding_coeff: Option<f64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SimulationState {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub start_latitude: Option<f64>,
    #[serde(default)]
    pub start_longitude: Option<f64>,
    #[serde(default)]
    pub current_day: i32,
    #[serde(default)]
    pub current_year: i32,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub speed_multiplier: Option<i32>,
    #[serde(default)]
    pub world_state: WorldState,
    #[serde(default)]
    pub individuals: Vec<Individual>,
    #[serde(default)]
    pub founder_1: Option<Value>,
    #[serde(default)]
    pub founder_2: Option<Value>,
    #[serde(default)]
    pub discovered_techs: Vec<String>,
    #[serde(default)]
    pub discovered_beliefs: Vec<String>,
    /// Procedurally generated label per discovered belief_id, filled in by
    /// `belief::try_label_belief` once the population's own language can
    /// actually express it (proto-words, stage >= 3) -- see belief.rs.
    /// Absent for a discovered-but-not-yet-nameable belief; the belief_id
    /// itself is an internal bucketing key only, never shown to the player.
    #[serde(default)]
    pub belief_labels: HashMap<String, String>,
    /// This simulation's civilization-level name, set once by `tick.rs` when
    /// most of the living population belongs to a group that has already
    /// named itself (see culture.rs's group naming, gated on the group's own
    /// `naming_ceremony` cultural meme) -- never forced, never set before
    /// that threshold, and never re-set once chosen.
    #[serde(default)]
    pub civilization_name: Option<String>,
    #[serde(default)]
    pub discovered_arts: Vec<String>,
    #[serde(default)]
    pub astronomy_knowledge: Vec<String>,
    #[serde(default)]
    pub celestial_observations: Vec<String>,
    #[serde(default)]
    pub groups: Vec<Value>,
    #[serde(default)]
    pub settlements: Vec<Value>,
    /// Conceived individuals whose `birth_day` is still in the future. They are
    /// spliced into `individuals` (and only then count toward population/events)
    /// once `current_day >= birth_day`, so a pregnancy is not a phantom living member.
    #[serde(default)]
    pub pending_births: Vec<Individual>,
    #[serde(default)]
    pub events: Vec<Value>,
    /// Keys of civilization-milestone events already fired (population/tech/belief/
    /// art/language-stage/longevity thresholds), so each only ever emits once.
    #[serde(default)]
    pub milestones: Vec<String>,
    /// Total individuals ever born (unlike `individuals.len()`, this stays
    /// correct even when the tick loop's in-memory `individuals` is bounded
    /// to alive+recently-dead only -- see runtime.rs's bounded-load path).
    /// Sourced from the DB's dedicated `population_count` column on load
    /// (same "dedicated column is truth" pattern as `status`/
    /// `speed_multiplier`), then incremented in-memory on every birth.
    #[serde(default)]
    pub total_ever_born: i32,
    /// Total individuals ever died -- the same "dedicated, monotonic
    /// counter" fix as `total_ever_born` above, for the exact same reason:
    /// `derive_stats`'s own `deaths` field used to be
    /// `individuals.iter().filter(is_dead).count()`, which silently
    /// undercounted (often down to near-zero on a long-running simulation)
    /// once ws.rs's periodic tick broadcast started sourcing it from
    /// `load_bounded_tick_state_no_genealogy` (db.rs), which deliberately
    /// excludes anyone dead more than `DEAD_FIELD_STRIP_GRACE_DAYS` ago from
    /// the in-memory `individuals` set entirely -- the Population panel's
    /// own deceased list (an unbounded DB query) kept showing every death
    /// while the live stats HUD's death counter kept dropping back toward
    /// zero as deaths aged out of the bounded window. Sourced from the DB's
    /// dedicated `death_count` column on load, then incremented in-memory
    /// on every death.
    #[serde(default)]
    pub total_ever_died: i32,
    /// Everyone-ever-born's parent ids + inbreeding coefficient, always kept
    /// fully populated regardless of how `individuals` itself is bounded --
    /// see `biology::genome::GenealogyIndex`. Deliberately not persisted in
    /// state_json: it's sourced fresh from the `individuals` table's
    /// dedicated columns on every load (see db.rs's `load_genealogy_index`),
    /// which is already the single source of truth for this data.
    #[serde(skip)]
    pub genealogy: crate::biology::genome::GenealogyIndex,
    /// Names from `TOGGLEABLE_ENGINES` currently skipped by `advance_one_day`
    /// -- a diagnostic-only escape hatch (see the Performance panel's
    /// per-engine on/off buttons) for isolating which engine a slowdown
    /// actually comes from. Deliberately never persisted: it resets to
    /// "everything on" on every fresh load, since a temporary diagnostic
    /// toggle should never silently outlive the session that set it, and a
    /// simulation run with engines disabled is expected to end up in an
    /// inconsistent state (nobody ages, nobody dies, whatever's off implies)
    /// that has no business being saved as the simulation's real history.
    #[serde(skip)]
    pub disabled_engines: HashSet<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Every engine `advance_one_day` can individually skip via
/// `SimulationState.disabled_engines`, in the order they run each tick.
/// `setup` (world state/disasters/phoneme palette/aging/group formation/
/// death witnessing) is deliberately not included -- it does foundational
/// bookkeeping (e.g. incrementing `current_day` itself) that every other
/// engine depends on, so disabling it wouldn't isolate anything, it would
/// just break everything downstream in a way that looks like every other
/// engine failed too.
pub const TOGGLEABLE_ENGINES: &[&str] = &[
    "economy",
    "consciousness_psychology",
    "language_naming",
    "microbiome_agent",
    "movement",
    "observation_learning",
    "tech_emergence",
    "reproduction",
    "mortality_roll",
    "microbiome_outbreak",
    "group_pruning",
    "belief",
    "culture_art",
    "social",
    "law",
    "architecture_conflict",
    "astronomy",
    "trade_disease",
];

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TickReport {
    pub current_day: i32,
    pub alive_count: usize,
    pub updated_age_count: usize,
}

/// Per-phase wall-clock breakdown of a single `advance_one_day` call, in
/// milliseconds -- lets the Performance panel's "MODULE / PERFORMANCE" block
/// attribute a slow tick to a specific engine group instead of one opaque
/// "Compute" total. Grouped by contiguous position in `advance_one_day`
/// (see that function's own numbered-step comments), not by a hard
/// module boundary. The old per-individual step (economy/epigenetics/
/// consciousness/psychology/language/microbiome/decision-making) used to be
/// timed as a single "economy_ms" bucket covering all of it; it's now split
/// into several sequential `par_iter_mut` passes -- one per engine group --
/// each with its own timer, so this field list reflects real per-engine cost
/// rather than one mislabeled catch-all.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PhaseTimings {
    /// World state, natural disasters, phoneme palette, aging/generation
    /// count, group formation, death witnessing. Not individually toggleable
    /// -- see `TOGGLEABLE_ENGINES`'s own doc comment for why.
    pub setup_ms: f64,
    /// Economy only: inventory init, gather/consume/produce, starvation HP loss.
    pub economy_ms: f64,
    /// Epigenetics, consciousness (cardinal rule: only this pass may touch
    /// mind.consciousness), psychology/mental state, water state.
    pub consciousness_psychology_ms: f64,
    /// FOXP2 expression, language stage growth, vocabulary/name origination.
    pub language_naming_ms: f64,
    /// Gut microbiome, action selection + experience accrual, mating urge.
    pub microbiome_agent_ms: f64,
    /// Band cohesion / mating drive / persisted wander heading.
    pub movement_ms: f64,
    /// Spatial-grid-bounded observation of nearby individuals: technology
    /// picked up by watching others, vocabulary picked up from a teacher.
    pub observation_learning_ms: f64,
    /// Technology discovered from an individual's own accumulated experience
    /// (independent of observation_learning's copy-from-others pathway).
    pub tech_emergence_ms: f64,
    /// Reproduction: conception, pregnancy terms, birth processing.
    pub reproduction_ms: f64,
    /// Death risk rolls for the living.
    pub mortality_roll_ms: f64,
    /// Population-wide microbiome/pathogen contagion (can itself kill).
    pub microbiome_outbreak_ms: f64,
    /// Removing today's dead from their group's member_ids.
    pub group_pruning_ms: f64,
    /// Belief formation, spread, ritual emergence, belief labeling.
    pub belief_ms: f64,
    /// Culture, civilization naming, art.
    pub culture_art_ms: f64,
    /// Leadership, group roles, fission signalling.
    pub social_ms: f64,
    /// Norm emergence, violation checks, exile enforcement.
    pub law_ms: f64,
    /// Settlement formation/construction/overcrowding, intergroup conflict.
    pub architecture_conflict_ms: f64,
    /// Celestial observation/knowledge accumulation.
    pub astronomy_ms: f64,
    /// Trade between adjacent living individuals + disease spread on contact.
    pub trade_disease_ms: f64,
}

impl PhaseTimings {
    pub fn accumulate(&mut self, other: &PhaseTimings) {
        self.setup_ms += other.setup_ms;
        self.economy_ms += other.economy_ms;
        self.consciousness_psychology_ms += other.consciousness_psychology_ms;
        self.language_naming_ms += other.language_naming_ms;
        self.microbiome_agent_ms += other.microbiome_agent_ms;
        self.movement_ms += other.movement_ms;
        self.observation_learning_ms += other.observation_learning_ms;
        self.tech_emergence_ms += other.tech_emergence_ms;
        self.reproduction_ms += other.reproduction_ms;
        self.mortality_roll_ms += other.mortality_roll_ms;
        self.microbiome_outbreak_ms += other.microbiome_outbreak_ms;
        self.group_pruning_ms += other.group_pruning_ms;
        self.belief_ms += other.belief_ms;
        self.culture_art_ms += other.culture_art_ms;
        self.social_ms += other.social_ms;
        self.law_ms += other.law_ms;
        self.architecture_conflict_ms += other.architecture_conflict_ms;
        self.astronomy_ms += other.astronomy_ms;
        self.trade_disease_ms += other.trade_disease_ms;
    }
}

impl SimulationState {
    pub fn alive_count(&self) -> usize {
        self.individuals
            .iter()
            .filter(|individual| individual.alive && !individual.is_dead)
            .count()
    }
}
