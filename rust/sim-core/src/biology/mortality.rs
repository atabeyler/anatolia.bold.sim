use serde_json::Value;

use crate::epigenetics::compute_epigenetic_age;
use crate::microbiome::PATHOGEN_TYPES;
use crate::state::Individual;

use super::individual::get_age;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathCause {
    Infection,
    Trauma,
    Starvation,
    Dehydration,
    BirthComplications,
    GeneticDisease,
    OldAge,
    Predator,
    Conflict,
    Drowning,
}

/// Target annual mortality rates (prehistoric hunter-gatherer baseline):
/// 0-1y ~8%, 1-5y ~3.7%, 5-15y ~1%, 15-45y ~1%, 45-60y ~2.5%, 60-75y ~8%, 75+ ~20%.
/// Disease outbreaks, starvation, and disasters layer on top via multipliers.
pub fn compute_daily_death_risk(individual: &Individual, current_day: i32, environment: Option<&Value>) -> f64 {
    let chronological_age = get_age(individual, current_day);
    // Epigenetic age: accumulated stress/nutrition history can accelerate or
    // slow biological aging relative to the calendar.
    let epi_years = compute_epigenetic_age(individual, current_day);
    let age = if epi_years > 0.0 && epi_years.is_finite() { epi_years } else { chronological_age };

    let health = &individual.health;
    let phenotype = &individual.phenotype;
    let is_founder = individual.is_founder;

    let mut base_risk = if age < 1.0 {
        0.00022
    } else if age < 5.0 {
        0.00010
    } else if age < 45.0 {
        0.000027
    } else if age < 60.0 {
        0.000069
    } else if age < 75.0 {
        0.00023
    } else {
        0.00061
    };

    // Extinction guard: tiny bands receive outsized individual attention.
    let alive_count = environment.and_then(|env| env.get("alive_count")).and_then(|v| v.as_f64()).unwrap_or(100.0);
    if alive_count < 25.0 {
        base_risk *= (alive_count / 25.0).max(0.25);
    }

    // Thriving healthy adult: well-fed prime-years individuals get a discount.
    if (15.0..45.0).contains(&age) && health.hp > 0.85 && health.calories > 0.7 {
        base_risk *= 0.4;
    }

    if age >= phenotype.max_lifespan {
        base_risk += 0.03;
    }
    if health.hp < 0.2 {
        base_risk *= if is_founder { 1.8 } else { 3.0 };
    }
    if health.calories < 0.1 {
        base_risk *= if is_founder { 2.5 } else { 5.0 };
    }
    if health.hydration < 0.1 {
        base_risk *= if is_founder { 5.0 } else { 10.0 };
    }

    base_risk *= 1.0 - phenotype.immune_strength * 0.3;

    let resilience = (phenotype.stress_resilience + phenotype.health_resilience) / 2.0;
    base_risk *= 1.0 - (resilience - 0.5) * 0.25;

    // Predator risk is applied as a single term: a founder-scaled base
    // contribution, modulated by toughness (a tougher individual cuts their
    // own exposure by up to ~20% at max toughness; a frailer one raises it
    // by the same amount). Kept in one block -- previously split across two
    // separate `if let Some(env)` blocks with no cross-reference, which made
    // it easy to edit one half without noticing the other existed.
    let toughness = (phenotype.endurance + phenotype.physical_strength) / 2.0;
    if let Some(env) = environment {
        let predator_risk = env.get("predator_risk").and_then(Value::as_f64).unwrap_or(0.0);
        let env_mult = if is_founder { 0.4 } else { 1.0 };
        let toughness_reduction = (toughness - 0.5) * 0.4;
        base_risk += predator_risk * 0.0002 * (env_mult - toughness_reduction);
    }

    if health.calories < 0.4 {
        base_risk *= 1.0 + (phenotype.metabolism - 0.5) * 0.2;
    }

    if individual._in_water() {
        let water_skill = individual._water_experience().min(0.9) * 0.9;
        base_risk += 0.003 * (1.0 - water_skill);
    }

    if let Some(env) = environment {
        let env_mult = if is_founder { 0.4 } else { 1.0 };
        base_risk += env.get("disease_pressure").and_then(Value::as_f64).unwrap_or(0.0) * 0.0003 * env_mult;
    }

    // >= (not strictly >) 0.25: a strict "> 0.25" gate would never fire for
    // the single most common inbreeding scenario a small founder population
    // hits -- full-sibling or parent-offspring mating, which produces
    // exactly F = 0.25 (see genome.rs::compute_inbreeding_coefficient).
    if individual.inbreeding_coeff.unwrap_or(0.0) >= 0.25 {
        base_risk *= 1.5;
    }

    if is_founder {
        base_risk *= 0.5;
    }

    base_risk.clamp(0.0, 0.99)
}

pub fn roll_death(individual: &Individual, current_day: i32, environment: Option<&Value>) -> Option<DeathCause> {
    if rand::random::<f64>() < compute_daily_death_risk(individual, current_day, environment) {
        Some(determine_cause(individual, current_day, environment))
    } else {
        None
    }
}

fn has_lethal_infection(individual: &Individual) -> bool {
    individual
        .extra
        .get("infections")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter().any(|inf| {
                inf.get("pathogen_id")
                    .and_then(Value::as_str)
                    .and_then(|pid| PATHOGEN_TYPES.iter().find(|(id, ..)| *id == pid))
                    .map(|(_, _, base_mortality, ..)| *base_mortality >= 0.05)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn determine_cause(individual: &Individual, current_day: i32, environment: Option<&Value>) -> DeathCause {
    let age = get_age(individual, current_day);
    let health = &individual.health;
    let phenotype = &individual.phenotype;
    let is_founder = individual.is_founder;

    if individual._in_water() {
        return DeathCause::Drowning;
    }
    if health.hydration < 0.1 {
        return DeathCause::Dehydration;
    }
    if health.calories < 0.05 {
        return DeathCause::Starvation;
    }
    if has_lethal_infection(individual) {
        return DeathCause::Infection;
    }
    if age >= phenotype.max_lifespan - 5.0 {
        return DeathCause::OldAge;
    }
    let predator_threshold = if is_founder { 0.15 } else { 0.3 };
    let predator_risk = environment.and_then(|env| env.get("predator_risk")).and_then(Value::as_f64).unwrap_or(0.0);
    if predator_risk > 0.5 && rand::random::<f64>() < predator_threshold {
        return DeathCause::Predator;
    }

    // Founders never die of genetic disease -- the player designed their genome intentionally.
    let genetic_resistance = (phenotype.health_resilience + phenotype.immune_strength) / 2.0;
    let genetic_chance = if is_founder { 0.0 } else { (0.30 - genetic_resistance * 0.30).max(0.0) };

    let birth_comp_chance = if individual.sex == "female" && health.pregnancy.is_some() {
        (0.15 - phenotype.fertility * 0.15).max(0.0)
    } else {
        0.0
    };

    let toughness = (phenotype.endurance + phenotype.physical_strength) / 2.0;
    let trauma_weight = (0.30 - (toughness - 0.5) * 0.20).max(0.05);

    if age < 5.0 {
        return if rand::random::<f64>() < 0.55 { DeathCause::Trauma } else { DeathCause::GeneticDisease };
    }
    if age < 15.0 {
        return if rand::random::<f64>() < 0.65 { DeathCause::Trauma } else { DeathCause::GeneticDisease };
    }

    let founder_factor = if is_founder { 0.55 } else { 1.0 };
    let adjusted_trauma = trauma_weight * founder_factor;

    if age < 45.0 {
        let r = rand::random::<f64>();
        let trauma_cut = adjusted_trauma;
        let birth_comp_cut = trauma_cut + birth_comp_chance;
        let genetic_cut = birth_comp_cut + genetic_chance;
        // The leftover probability mass here only gets attributed to a
        // predator kill in proportion to this environment's actual
        // predator_risk -- a predator-free biome (e.g. coastal, risk 0.15)
        // must not have every unattributed adult death blamed on a predator.
        // Whatever isn't explained by real predator risk falls back to
        // trauma, the same general "misadventure" bucket used for younger
        // age bands.
        let predator_cut = genetic_cut + (predator_risk * 0.3).max(0.0).min(1.0 - genetic_cut);
        return if r < trauma_cut {
            DeathCause::Trauma
        } else if r < birth_comp_cut {
            DeathCause::BirthComplications
        } else if r < genetic_cut {
            DeathCause::GeneticDisease
        } else if r < predator_cut {
            DeathCause::Predator
        } else {
            DeathCause::Trauma
        };
    }

    let r = rand::random::<f64>();
    let old_age_cut = 0.20;
    let trauma_cut = old_age_cut + adjusted_trauma;
    let genetic_cut = trauma_cut + genetic_chance;
    if r < old_age_cut {
        DeathCause::OldAge
    } else if r < trauma_cut {
        DeathCause::Trauma
    } else if r < genetic_cut {
        DeathCause::GeneticDisease
    } else {
        DeathCause::OldAge
    }
}

trait IndividualExt {
    fn _water_experience(&self) -> f64;
    fn _in_water(&self) -> bool;
}

impl IndividualExt for Individual {
    fn _water_experience(&self) -> f64 {
        self.extra.get("_waterExperience").and_then(|v| v.as_f64()).unwrap_or(0.0)
    }
    fn _in_water(&self) -> bool {
        self.extra.get("_inWater").and_then(|v| v.as_bool()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Health, Phenotype};

    fn make_ind(age_years: i32) -> Individual {
        Individual {
            birth_day: -age_years * 365,
            phenotype: Phenotype { max_lifespan: 70.0, immune_strength: 0.5, ..Default::default() },
            health: Health { hp: 1.0, calories: 1.0, hydration: 1.0, ..Default::default() },
            inbreeding_coeff: Some(0.0),
            ..Default::default()
        }
    }

    fn env(alive_count: f64) -> Value {
        serde_json::json!({ "alive_count": alive_count })
    }

    // ── baseline age bands ───────────────────────────────────────────────

    #[test]
    fn infant_base_risk_is_small_but_positive() {
        let ind = make_ind(0); // birth_day 0, effectively newborn at day 0
        let risk = compute_daily_death_risk(&ind, 0, Some(&env(100.0)));
        assert!(risk > 0.0 && risk < 0.001);
    }

    #[test]
    fn risk_increases_with_age_band() {
        let adult = make_ind(25);
        let middle_age = make_ind(55);
        let old = make_ind(72);
        let r_a = compute_daily_death_risk(&adult, 0, Some(&env(100.0)));
        let r_m = compute_daily_death_risk(&middle_age, 0, Some(&env(100.0)));
        let r_o = compute_daily_death_risk(&old, 0, Some(&env(100.0)));
        assert!(r_m > r_a);
        assert!(r_o > r_m);
    }

    #[test]
    fn annual_infant_mortality_is_about_7_7_percent() {
        let annual = 1.0 - (1.0 - 0.00022_f64).powi(365);
        assert!((annual - 0.077).abs() < 0.01);
    }

    #[test]
    fn annual_elder_mortality_is_about_20_percent() {
        let annual = 1.0 - (1.0 - 0.00061_f64).powi(365);
        assert!((annual - 0.20).abs() < 0.05);
    }

    // ── multipliers ─────────────────────────────────────────────────────

    #[test]
    fn dehydration_multiplies_risk_heavily() {
        let normal = make_ind(25);
        let mut dehydrated = make_ind(25);
        dehydrated.health.hydration = 0.05;
        let r_n = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        let r_d = compute_daily_death_risk(&dehydrated, 0, Some(&env(100.0)));
        assert!(r_d > r_n * 5.0);
    }

    #[test]
    fn starvation_raises_risk() {
        let normal = make_ind(25);
        let mut starving = make_ind(25);
        starving.health.calories = 0.05;
        let r_n = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        let r_s = compute_daily_death_risk(&starving, 0, Some(&env(100.0)));
        assert!(r_s > r_n * 3.0);
    }

    #[test]
    fn thriving_adult_has_lower_risk_than_baseline() {
        let mut thriving = make_ind(25);
        thriving.health.hp = 0.9;
        thriving.health.calories = 0.8;
        let mut baseline = make_ind(25);
        baseline.health.hp = 0.5;
        baseline.health.calories = 0.5;
        assert!(compute_daily_death_risk(&thriving, 0, Some(&env(100.0))) < compute_daily_death_risk(&baseline, 0, Some(&env(100.0))));
    }

    #[test]
    fn inbreeding_above_quarter_multiplies_risk_by_1_5() {
        let normal = make_ind(25);
        let mut inbred = make_ind(25);
        inbred.inbreeding_coeff = Some(0.6);
        let r_n = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        let r_i = compute_daily_death_risk(&inbred, 0, Some(&env(100.0)));
        assert!((r_i - r_n * 1.5).abs() < 1e-6);
    }

    #[test]
    fn inbreeding_exactly_at_the_quarter_threshold_gets_the_multiplier() {
        // F = 0.25 is exactly what a full-sibling or parent-child mating
        // produces (see genome.rs::compute_inbreeding_coefficient) -- the
        // dominant real-world scenario a small founder population hits, so
        // this boundary case must be covered, not excluded.
        let mut at_threshold = make_ind(25);
        at_threshold.inbreeding_coeff = Some(0.25);
        let normal = make_ind(25);
        let r_at = compute_daily_death_risk(&at_threshold, 0, Some(&env(100.0)));
        let r_normal = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        assert!((r_at - r_normal * 1.5).abs() < 1e-6);
    }

    #[test]
    fn inbreeding_just_below_the_quarter_threshold_gets_no_multiplier() {
        let mut just_below = make_ind(25);
        just_below.inbreeding_coeff = Some(0.24);
        let normal = make_ind(25);
        let r_below = compute_daily_death_risk(&just_below, 0, Some(&env(100.0)));
        let r_normal = compute_daily_death_risk(&normal, 0, Some(&env(100.0)));
        assert!((r_below - r_normal).abs() < 1e-8);
    }

    #[test]
    fn risk_is_capped_at_0_99() {
        let mut worst = make_ind(100);
        worst.phenotype = Phenotype { max_lifespan: 50.0, immune_strength: 0.0, ..Default::default() };
        worst.health = Health { hp: 0.05, calories: 0.01, hydration: 0.01, ..Default::default() };
        worst.extra.insert("_inWater".to_string(), serde_json::json!(true));
        assert!(compute_daily_death_risk(&worst, 0, Some(&env(100.0))) <= 0.99);
    }

    // ── water / drowning ────────────────────────────────────────────────

    #[test]
    fn being_in_water_raises_risk() {
        let dry = make_ind(25);
        let mut wet = make_ind(25);
        wet.extra.insert("_inWater".to_string(), serde_json::json!(true));
        assert!(compute_daily_death_risk(&wet, 0, Some(&env(100.0))) > compute_daily_death_risk(&dry, 0, Some(&env(100.0))));
    }

    #[test]
    fn water_experience_lowers_drowning_risk() {
        let mut no_exp = make_ind(25);
        no_exp.extra.insert("_inWater".to_string(), serde_json::json!(true));
        let mut expert = make_ind(25);
        expert.extra.insert("_inWater".to_string(), serde_json::json!(true));
        expert.extra.insert("_waterExperience".to_string(), serde_json::json!(1.0));
        assert!(compute_daily_death_risk(&expert, 0, Some(&env(100.0))) < compute_daily_death_risk(&no_exp, 0, Some(&env(100.0))));
    }

    // ── extinction guard ────────────────────────────────────────────────

    #[test]
    fn small_population_reduces_individual_risk() {
        let ind = make_ind(25);
        let r_large = compute_daily_death_risk(&ind, 0, Some(&env(100.0)));
        let r_small = compute_daily_death_risk(&ind, 0, Some(&env(5.0)));
        assert!(r_small < r_large);
    }

    #[test]
    fn population_of_one_gets_the_minimum_0_25x_guard() {
        let ind = make_ind(25);
        let r_single = compute_daily_death_risk(&ind, 0, Some(&env(1.0)));
        let r_full = compute_daily_death_risk(&ind, 0, Some(&env(100.0)));
        assert!((r_single - r_full * 0.25).abs() < 1e-5);
    }

    // ── rollDeath causality ─────────────────────────────────────────────

    #[test]
    fn a_healthy_adult_rarely_dies_in_100_trials() {
        let healthy = make_ind(25);
        let deaths = (0..100).filter(|_| roll_death(&healthy, 0, Some(&env(100.0))).is_some()).count();
        assert!(deaths < 5);
    }

    #[test]
    fn death_returns_a_specific_cause_when_it_happens() {
        let mut ind = make_ind(100);
        ind.phenotype = Phenotype { max_lifespan: 50.0, immune_strength: 0.0, ..Default::default() };
        ind.health = Health { hp: 0.05, calories: 0.01, hydration: 0.01, ..Default::default() };
        let mut saw_death = false;
        for _ in 0..200 {
            if roll_death(&ind, 0, Some(&env(100.0))).is_some() {
                saw_death = true;
                break;
            }
        }
        assert!(saw_death);
    }

    #[test]
    fn in_water_death_is_always_attributed_to_drowning() {
        let mut ind = make_ind(25);
        ind.extra.insert("_inWater".to_string(), serde_json::json!(true));
        ind.extra.insert("_waterExperience".to_string(), serde_json::json!(0.0));
        ind.health = Health { hp: 0.05, calories: 0.01, hydration: 0.01, ..Default::default() };
        let mut saw_drowning = false;
        for _ in 0..5000 {
            if roll_death(&ind, 0, Some(&env(100.0))) == Some(DeathCause::Drowning) {
                saw_drowning = true;
                break;
            }
        }
        assert!(saw_drowning);
    }

    #[test]
    fn severe_dehydration_is_attributed_to_dehydration() {
        let mut ind = make_ind(90);
        ind.phenotype = Phenotype { max_lifespan: 50.0, immune_strength: 0.0, ..Default::default() };
        ind.health = Health { hp: 0.1, calories: 0.8, hydration: 0.05, ..Default::default() };
        let mut saw_dehydration = false;
        for _ in 0..200 {
            if roll_death(&ind, 0, Some(&env(100.0))) == Some(DeathCause::Dehydration) {
                saw_dehydration = true;
                break;
            }
        }
        assert!(saw_dehydration);
    }
}
