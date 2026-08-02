use serde_json::{json, Map, Value};
use std::collections::HashMap;
use uuid::Uuid;

use crate::state::Individual;
use crate::types::{Epigenome, Health, Language, Mind, Psychology, Social, Volatile};

use super::genome::{combine_gametes, compute_phenotype, create_gamete, create_genome, make_x_linked_loci_hemizygous};

pub fn get_age(individual: &Individual, current_day: i32) -> f64 {
    (current_day - individual.birth_day) as f64 / 365.0
}

pub fn get_life_stage(individual: &Individual, current_day: i32) -> &'static str {
    let age = get_age(individual, current_day);
    if age < 2.0 {
        "infant"
    } else if age < 12.0 {
        "child"
    } else if age < 18.0 {
        "adolescent"
    } else if age < 45.0 {
        "adult"
    } else {
        "elder"
    }
}

pub fn is_fertile(individual: &Individual, current_day: i32) -> bool {
    let age = get_age(individual, current_day);
    match individual.sex.as_str() {
        "female" => (15.0..=50.0).contains(&age),
        "male" => (15.0..=65.0).contains(&age),
        _ => false,
    }
}

fn default_health(hp: f64, calories: f64, disease_resistance: f64) -> Health {
    Health {
        hp,
        max_hp: 1.0,
        calories,
        hydration: 1.0,
        disease: None,
        disease_resistance,
        injuries: vec![],
        pregnancy: None,
        pregnancy_day: None,
        microbiome_immunity: None,
        extra: Map::new(),
    }
}

fn default_mind(age_days: i32) -> Mind {
    Mind {
        consciousness: 0.0,
        death_awareness: false,
        emotional_state: 0.5,
        stress: 0.0,
        volatile: Volatile { satiation: 1.0, age: age_days, water_experience: 0.0, generation: 0, extra: Map::new() },
        extra: Map::new(),
    }
}

fn default_social() -> Social {
    Social { relationships: HashMap::new(), reputation: 0.5, status: 0.0, has_mate: false, mate_id: None, children_ids: vec![], extra: Map::new() }
}

fn default_language(foxp2_expression: f64) -> Language {
    Language { stage: 0, stage_name: "pre-linguistic".to_string(), vocabulary: HashMap::new(), grammar: false, writing: false, foxp2_expression, extra: Map::new() }
}

fn default_psychology() -> Psychology {
    Psychology {
        mental_state: "calm".to_string(),
        wellbeing: 0.7,
        attachment_style: String::new(),
        stress_level: 0.1,
        trauma_events: vec![],
        relationships: HashMap::new(),
        theory_of_mind: 0,
        self_awareness: false,
        life_satisfaction: 0.5,
        trauma_anxiety: 0.0,
        extra: Map::new(),
    }
}

pub fn create_founder(params: &Value) -> Individual {
    let sex = params.get("sex").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let age_years = params.get("ageYears").and_then(|v| v.as_i64()).unwrap_or(20) as i32;
    let x = params.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = params.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let genome_overrides = params.get("genome").and_then(|v| v.as_object());
    let mut genome = create_genome(genome_overrides);
    if sex == "male" {
        make_x_linked_loci_hemizygous(&mut genome);
    }
    let mut phenotype = compute_phenotype(&genome);
    if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
        phenotype.name = Some(name.trim().to_string());
    }
    let appearance = params.get("appearance").and_then(|v| v.as_object()).cloned().unwrap_or_default();
    let founder_foxp2 = phenotype.language_capacity * 0.7;
    let founder_immune_strength = phenotype.immune_strength;
    // Founders start with pre-existing adult water experience (God Mode
    // exemption, see AGENTS.md) -- stored in `extra["_waterFear"]`, the same
    // field every consumer (psychology::update_mental_state,
    // environment::apply_disaster/decay_fears, tick::witness_death) actually
    // reads, not the unrelated typed `mind.volatile` struct.
    let mut extra: Map<String, Value> = appearance.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    extra.insert("_waterFear".to_string(), json!(0.35));

    let mut individual = Individual {
        id: Uuid::new_v4().to_string(),
        simulation_id: None,
        birth_day: -age_years * 365,
        death_day: None,
        alive: true,
        is_dead: false,
        is_founder: true,
        sex,
        x,
        y,
        age_days: None,
        generation: Some(0),
        group_id: None,
        home_x: Some(x),
        home_y: Some(y),
        parent_1_id: None,
        parent_2_id: None,
        known_techs: vec![],
        genome,
        phenotype,
        epigenome: Epigenome::new(),
        // Derived from the founder's own (player-configurable, God Mode)
        // IMMUNE_01/IMMUNE_02 genotype instead of a hardcoded constant, so
        // disease_resistance is consistent with how create_child seeds it.
        health: default_health(1.0, 1.0, founder_immune_strength),
        mind: default_mind(age_years * 365),
        social: default_social(),
        skills: vec![],
        beliefs: Default::default(),
        language: default_language(founder_foxp2),
        memory: Value::Null,
        psychology: default_psychology(),
        // Overwritten immediately below by hormones::initialize_hormones,
        // which needs the fully-constructed individual (sex/phenotype) to
        // compute a real baseline -- this placeholder never survives to be
        // read.
        hormones: Default::default(),
        inventory: HashMap::new(),
        inbreeding_coeff: Some(0.0),
        extra,
    };
    // Snapshot the genome-only starting values epigenetics.rs's apply_fx
    // blends around, before any methylation has had a chance to run --
    // see epigenetics::snapshot_genetic_baseline.
    crate::epigenetics::snapshot_genetic_baseline(&mut individual);
    crate::hormones::initialize_hormones(&mut individual);
    individual
}

// (locus, a1, a2) -- intentionally two independent values per locus rather
// than a single value duplicated onto both alleles: collapsing them to one
// shared value would silently zero the within-locus variance codominant
// averaging relies on and, since a1==a2 then makes crossover deterministic
// at these loci, understate genetic diversity passed to every founder's
// children too.
pub const FOUNDER_GENOME_DEFAULTS: &[(&str, f64, f64)] = &[
    ("OXTR_01", 0.82, 0.82), ("AVPR1A_01", 0.78, 0.78), ("IMMUNE_01", 0.88, 0.85), ("IMMUNE_02", 0.85, 0.82),
    ("TERT_01", 0.85, 0.85), ("APOE_01", 0.80, 0.80), ("FOXP2_01", 0.90, 0.88), ("CNTNAP2_01", 0.82, 0.80),
    ("BDNF_01", 0.80, 0.78), ("COMT_01", 0.78, 0.76), ("DTNBP1_01", 0.80, 0.78), ("NRXN1_01", 0.82, 0.80),
    ("SHANK3_01", 0.80, 0.78), ("RELN_01", 0.80, 0.78), ("DRD4_01", 0.75, 0.75), ("DRD2_01", 0.75, 0.72),
    ("STRENGTH_01", 0.78, 0.75), ("ACTN3_01", 0.76, 0.74), ("FSHR_01", 0.70, 0.68),
];

/// Thin adapter over `create_founder` for the "seed a new simulation" call
/// site: fills in the founder-only genome defaults above (overridable by
/// `params.genome`), pins position/age/sex, and (for a non-founder, e.g. a
/// pre-placed companion individual) softens starting health/water-fear the
/// same way every other newborn does. Shared by sim-server's create_simulation
/// route and sim-wasm's create_founder_json_for_simulation so a browser-only
/// WASM run seeds identically to a server-backed one.
#[allow(clippy::too_many_arguments)]
pub fn create_founder_for_simulation(
    params: &Value,
    sex: &str,
    x: f64,
    y: f64,
    age_years: i32,
    is_founder: bool,
    simulation_id: &str,
    parent_1_id: Option<String>,
    parent_2_id: Option<String>,
) -> Individual {
    let mut genome_overrides = Map::new();
    if is_founder {
        for (key, a1, a2) in FOUNDER_GENOME_DEFAULTS {
            genome_overrides.insert((*key).to_string(), json!({ "a1": a1, "a2": a2 }));
        }
    }
    if let Some(custom) = params.get("genome").and_then(|v| v.as_object()) {
        for (k, v) in custom {
            genome_overrides.insert(k.clone(), v.clone());
        }
    }

    let founder_params = json!({
        "sex": sex,
        "ageYears": age_years,
        "x": x,
        "y": y,
        "name": params.get("name").and_then(|v| v.as_str()),
        "genome": genome_overrides,
        "appearance": params.get("appearance").cloned().unwrap_or_else(|| json!({})),
    });

    let mut individual = create_founder(&founder_params);
    individual.simulation_id = Some(simulation_id.to_string());
    individual.is_founder = is_founder;
    individual.parent_1_id = parent_1_id;
    individual.parent_2_id = parent_2_id;
    if !is_founder {
        individual.health.hp = 0.4;
        individual.extra.insert("_waterFear".to_string(), json!(0.0));
    }
    individual
}

pub fn create_child(parent1: &Individual, parent2: &Individual, birth_day: i32, simulation_id: &str) -> Individual {
    let sex = if rand::random::<f64>() < 0.5 { "male" } else { "female" }.to_string();
    let p1_stress = parent1.epigenome.get("HPA_AXIS").map(|l| l.methylation).unwrap_or(0.5);
    let p2_stress = parent2.epigenome.get("HPA_AXIS").map(|l| l.methylation).unwrap_or(0.5);
    // HPA_AXIS methylation starts at a neutral 0.5 baseline (see
    // epigenetics::initialize_epigenome), not 0.0 -- only elevation *above*
    // that neutral point represents an actual famine/stress signal. Scaling
    // directly off the raw 0.0-1.0 methylation value (as this used to) meant
    // even a never-stressed parent's neutral 0.5 contributed a permanent
    // +0.25 to stress_mult, so every gamete's effective mutation rate was at
    // least 1.25x -- i.e. ~2.5 mutations/gamete baseline, not the documented
    // ~2. Subtracting the neutral point restores a true 1.0x (2/gamete)
    // multiplier when no parent is actually stressed, rising to the same
    // 1.5x ceiling as before at maximum methylation.
    let stress_mult = 1.0 + (p1_stress.max(p2_stress) - 0.5).max(0.0);
    let genome = combine_gametes(&create_gamete(&parent1.genome, stress_mult), &create_gamete(&parent2.genome, stress_mult), &sex);
    let phenotype = compute_phenotype(&genome);
    let id = Uuid::new_v4().to_string();
    // phenotype.name stays None at birth -- naming::try_originate_name fills
    // it in later, once this individual's own lived language actually
    // supports one (see naming.rs for why).
    let child_foxp2 = phenotype.language_capacity * 0.1;
    let child_immune_strength = phenotype.immune_strength;

    let p1_water_fear = parent1.extra.get("_waterFear").and_then(Value::as_f64).unwrap_or(0.0);
    let p2_water_fear = parent2.extra.get("_waterFear").and_then(Value::as_f64).unwrap_or(0.0);
    let inherited_water_fear = (p1_water_fear + p2_water_fear) / 2.0 * 0.45;
    let inherited_group_id = parent1.group_id.clone().or_else(|| parent2.group_id.clone());
    let mut extra = Map::new();
    extra.insert("_waterFear".to_string(), json!(inherited_water_fear));

    let mut individual = Individual {
        id,
        simulation_id: Some(simulation_id.to_string()),
        birth_day,
        death_day: None,
        alive: true,
        is_dead: false,
        is_founder: false,
        sex,
        x: parent1.x,
        y: parent1.y,
        age_days: None,
        generation: Some(parent1.generation.unwrap_or(0).max(parent2.generation.unwrap_or(0)) + 1),
        group_id: inherited_group_id,
        home_x: Some(parent1.x),
        home_y: Some(parent1.y),
        parent_1_id: Some(parent1.id.clone()),
        parent_2_id: Some(parent2.id.clone()),
        known_techs: vec![],
        genome,
        phenotype,
        epigenome: Epigenome::new(),
        health: default_health(0.4, 0.8, child_immune_strength),
        mind: default_mind(0),
        social: default_social(),
        skills: vec![],
        beliefs: Default::default(),
        language: default_language(child_foxp2),
        memory: Value::Null,
        psychology: default_psychology(),
        // Overwritten immediately below by hormones::initialize_hormones,
        // which needs the fully-constructed individual (sex/phenotype) to
        // compute a real baseline -- this placeholder never survives to be
        // read.
        hormones: Default::default(),
        inventory: HashMap::new(),
        inbreeding_coeff: Some(0.0),
        extra,
    };
    crate::epigenetics::snapshot_genetic_baseline(&mut individual);
    crate::hormones::initialize_hormones(&mut individual);
    individual
}

/// Builds a new arrival individual for a *different* simulation than the one
/// `source` currently lives in -- cross-simulation migration/gene flow, an
/// explicit, rare player action (see sim-server's `god::migrate_individual`),
/// never something the tick loop triggers on its own. Every genetically
/// inherited field -- genome, phenotype, epigenome, language, skills,
/// beliefs -- carries over verbatim; that transfer is the entire point of
/// the feature. Parent ids and group membership are severed (the source
/// simulation's genealogy index and groups don't exist in the target, and
/// keeping a parent id would leave a foreign key pointing at a row in the
/// wrong simulation), and health/psychology/memory reset to a fresh-arrival
/// baseline the same way any newcomer would after a long journey. This never
/// sets `is_founder` and never grants any behavior beyond what a normal
/// adult individual already has.
pub fn migrate_individual_arrival(source: &Individual, source_current_day: i32, target_current_day: i32) -> Individual {
    let age_days_at_source = (source_current_day - source.birth_day).max(0);
    let mut individual = Individual {
        id: Uuid::new_v4().to_string(),
        simulation_id: None,
        birth_day: target_current_day - age_days_at_source,
        death_day: None,
        alive: true,
        is_dead: false,
        is_founder: false,
        sex: source.sex.clone(),
        x: source.x,
        y: source.y,
        age_days: None,
        generation: source.generation,
        group_id: None,
        home_x: Some(source.x),
        home_y: Some(source.y),
        parent_1_id: None,
        parent_2_id: None,
        known_techs: source.known_techs.clone(),
        genome: source.genome.clone(),
        phenotype: source.phenotype.clone(),
        epigenome: source.epigenome.clone(),
        health: default_health(0.6, 0.6, source.phenotype.immune_strength),
        mind: default_mind(age_days_at_source),
        social: default_social(),
        skills: source.skills.clone(),
        beliefs: source.beliefs.clone(),
        language: source.language.clone(),
        memory: Value::Null,
        psychology: default_psychology(),
        // Overwritten immediately below by hormones::initialize_hormones,
        // which needs the fully-constructed individual (sex/phenotype) to
        // compute a real baseline -- this placeholder never survives to be
        // read.
        hormones: Default::default(),
        inventory: HashMap::new(),
        inbreeding_coeff: Some(0.0),
        extra: Map::new(),
    };
    // A fresh arrival's hormones re-baseline the same way health/psychology/
    // mind do just above -- genetics/sex/age carry over, but the previous
    // simulation's per-tick circulating levels (mid-stress-response,
    // mid-pregnancy-surge, whatever they happened to be) don't mean anything
    // in a simulation this individual never lived a single tick in.
    crate::hormones::initialize_hormones(&mut individual);
    individual
}

#[cfg(test)]
mod tests {
    use super::*;

    fn founder(sex: &str, age_years: i64) -> Individual {
        create_founder(&serde_json::json!({ "sex": sex, "ageYears": age_years, "x": 0, "y": 0 }))
    }

    #[test]
    fn founders_start_with_no_known_techs() {
        assert!(founder("male", 20).known_techs.is_empty());
    }

    #[test]
    fn founder_has_generation_zero() {
        assert_eq!(founder("male", 25).generation, Some(0));
    }

    #[test]
    fn first_generation_child_has_generation_one() {
        let f1 = founder("male", 25);
        let f2 = founder("female", 25);
        let child = create_child(&f1, &f2, 0, "sim1");
        assert_eq!(child.generation, Some(1));
    }

    #[test]
    fn grandchild_has_generation_two() {
        let f1 = founder("male", 25);
        let f2 = founder("female", 25);
        let child1 = create_child(&f1, &f2, 0, "sim1");
        let child2 = create_child(&f1, &f2, 0, "sim1");
        let grandchild = create_child(&child1, &child2, 1, "sim1");
        assert_eq!(grandchild.generation, Some(2));
    }

    #[test]
    fn a_child_stays_unnamed_at_birth_until_their_own_language_originates_a_name() {
        // Cardinal rule: a name is a word, not a birth gift -- see naming.rs.
        // A newborn has no language yet, so create_child must never fill in
        // phenotype.name itself; naming::try_originate_name is the only place
        // that ever does, and only once stage/foxp2 thresholds are met.
        let f1 = founder("male", 25);
        let f2 = founder("female", 25);
        for _ in 0..20 {
            let child = create_child(&f1, &f2, 0, "sim1");
            assert!(child.phenotype.name.is_none(), "a newborn has no language yet and must not be given a name at birth");
        }
    }

    #[test]
    fn generation_is_max_of_parents_plus_one_for_mixed_generation_parents() {
        let f1 = founder("male", 25);
        let f2 = founder("female", 25);
        let child = create_child(&f1, &f2, 0, "sim1");
        let offspring = create_child(&child, &f1, 2, "sim1");
        assert_eq!(offspring.generation, Some(2));
    }

    #[test]
    fn female_between_15_and_50_is_fertile() {
        assert!(is_fertile(&founder("female", 25), 0));
    }

    #[test]
    fn female_aged_10_is_infertile() {
        assert!(!is_fertile(&founder("female", 10), 0));
    }

    #[test]
    fn female_aged_55_is_infertile() {
        assert!(!is_fertile(&founder("female", 55), 0));
    }

    #[test]
    fn male_aged_25_is_fertile() {
        assert!(is_fertile(&founder("male", 25), 0));
    }

    #[test]
    fn male_aged_70_is_infertile() {
        assert!(!is_fertile(&founder("male", 70), 0));
    }

    #[test]
    fn child_receives_parent_ids() {
        let mother = founder("female", 25);
        let father = founder("male", 25);
        let child = create_child(&mother, &father, 0, "sim1");
        assert_eq!(child.parent_1_id.as_deref(), Some(mother.id.as_str()));
        assert_eq!(child.parent_2_id.as_deref(), Some(father.id.as_str()));
    }

    #[test]
    fn child_inherits_first_available_parent_group() {
        let mut mother = founder("female", 25);
        let father = founder("male", 25);
        mother.group_id = Some("group-alpha".to_string());
        let child = create_child(&mother, &father, 0, "sim1");
        assert_eq!(child.group_id.as_deref(), Some("group-alpha"));
    }

    #[test]
    fn child_inherits_second_parent_group_when_first_parent_has_none() {
        let mother = founder("female", 25);
        let mut father = founder("male", 25);
        father.group_id = Some("group-beta".to_string());
        let child = create_child(&mother, &father, 0, "sim1");
        assert_eq!(child.group_id.as_deref(), Some("group-beta"));
    }

    #[test]
    fn child_starts_with_low_foxp2_expression() {
        let mother = founder("female", 25);
        let father = founder("male", 25);
        let child = create_child(&mother, &father, 0, "sim1");
        let cap = child.phenotype.language_capacity;
        assert!((child.language.foxp2_expression - cap * 0.1).abs() < 1e-9);
    }

    #[test]
    fn child_starts_fragile_at_hp_0_4() {
        let mother = founder("female", 25);
        let father = founder("male", 25);
        let child = create_child(&mother, &father, 0, "sim1");
        assert_eq!(child.health.hp, 0.4);
    }

    #[test]
    fn child_receives_a_real_genome_via_combine_gametes() {
        let mother = founder("female", 25);
        let father = founder("male", 25);
        let child = create_child(&mother, &father, 0, "sim1");
        assert!(child.genome.contains_key("FOXP2_01"));
        assert_eq!(child.genome.len(), mother.genome.len());
    }

    #[test]
    fn child_is_not_a_founder_and_starts_alive() {
        let mother = founder("female", 25);
        let father = founder("male", 25);
        let child = create_child(&mother, &father, 100, "sim1");
        assert!(!child.is_founder);
        assert!(child.alive);
        assert!(!child.is_dead);
        assert_eq!(child.birth_day, 100);
    }

    #[test]
    fn founder_starts_with_the_documented_pre_existing_water_fear() {
        let f = founder("male", 25);
        assert_eq!(f.extra.get("_waterFear").and_then(Value::as_f64), Some(0.35));
    }

    #[test]
    fn child_inherits_scaled_average_of_parents_water_fear() {
        let mut mother = founder("female", 25);
        let mut father = founder("male", 25);
        mother.extra.insert("_waterFear".to_string(), json!(0.5));
        father.extra.insert("_waterFear".to_string(), json!(0.3));
        let child = create_child(&mother, &father, 0, "sim1");
        let expected = (0.5 + 0.3) / 2.0 * 0.45;
        let actual = child.extra.get("_waterFear").and_then(Value::as_f64).unwrap();
        assert!((actual - expected).abs() < 1e-9);
    }

    #[test]
    fn non_founder_companion_starts_with_no_water_fear() {
        let individual = create_founder_for_simulation(
            &serde_json::json!({}),
            "male",
            0.0,
            0.0,
            20,
            false,
            "sim1",
            None,
            None,
        );
        assert_eq!(individual.extra.get("_waterFear").and_then(Value::as_f64), Some(0.0));
    }

    #[test]
    fn h03_regression_neutral_parents_produce_the_documented_four_mutations_per_child() {
        // Before the H-03 fix, stress_mult scaled directly off raw HPA_AXIS
        // methylation (neutral baseline 0.5), so even epigenetically-neutral
        // parents always contributed a phantom +0.25 "stress" bonus --
        // ~2.5 mutations/gamete (~5/child) instead of the README/AGENTS.md's
        // documented "~2 mutations per gamete (~4 per child)". Two founders
        // with untouched (never-updated) epigenomes are exactly this neutral
        // case, so their children's average mutated-allele count should land
        // close to 4, not 5.
        let mut overrides = Map::new();
        for locus_id in ["BDNF_01", "COMT_01", "DTNBP1_01", "NRG1_01", "DISC1_01", "FOXP2_01", "OXTR_01"] {
            overrides.insert(locus_id.to_string(), json!({ "a1": 0.5, "a2": 0.5 }));
        }
        let mother = create_founder(&json!({ "sex": "female", "ageYears": 25, "x": 0, "y": 0, "genome": overrides.clone() }));
        let father = create_founder(&json!({ "sex": "male", "ageYears": 25, "x": 0, "y": 0, "genome": overrides }));

        let trials = 2000;
        let mut total_mutated_alleles = 0u64;
        for _ in 0..trials {
            let child = create_child(&mother, &father, 0, "sim1");
            for locus_id in ["BDNF_01", "COMT_01", "DTNBP1_01", "NRG1_01", "DISC1_01", "FOXP2_01", "OXTR_01"] {
                let locus = &child.genome[locus_id];
                if (locus.allele1.value.unwrap() - 0.5).abs() > 1e-9 {
                    total_mutated_alleles += 1;
                }
                if (locus.allele2.value.unwrap_or(0.5) - 0.5).abs() > 1e-9 {
                    total_mutated_alleles += 1;
                }
            }
        }
        let observed_rate = total_mutated_alleles as f64 / trials as f64 / (7.0 * 2.0);
        let expected_rate = 2.0 / 32.0; // baseline (2/gamete) mutation probability per allele draw
        assert!(
            (observed_rate - expected_rate).abs() < 0.01,
            "expected per-allele mutation rate close to the documented baseline {expected_rate}, got {observed_rate}"
        );
    }
}
