//! Dynamic endocrine system. Distinct from the static, genome-derived
//! phenotype traits (`oxytocin_sensitivity`, `serotonin`, `aggression`,
//! `dominance`, `stress_reactivity`, ...) which model receptor
//! sensitivity/predisposition and never change after birth -- everything in
//! this module models an actual circulating hormone *level* that rises and
//! falls tick by tick.
//!
//! Cardinal rule: `individual.hormones` may only ever be written here (same
//! contract as `mind.consciousness` in consciousness.rs). Every value is a
//! deterministic formula over the individual's own genetics (phenotype/sex/
//! age) and this tick's already-tracked, real state -- `psychology.stress_level`
//! (itself already rolled up from disaster/isolation/exile/trauma/water-fear
//! in psychology.rs), `health.hp`, `health.pregnancy`, `group_id`, and the
//! satiation swing economy.rs already writes to `extra["satiation"]` earlier
//! in the same tick. Nothing here is scripted per individual, and nothing
//! reads an event type that isn't actually reachable in the live tick loop.

use serde_json::{json, Map, Value};

use crate::state::Individual;
use crate::types::Hormones;

use super::biology::individual::get_age;

/// Population-wide hormone averages, mirroring `psychology::compute_population_psych_stats`'s
/// own shape/rounding convention -- surfaced by client_view::derive_stats as
/// `stats.mean_hormones` for the client (PsychologyPanel's "Hormonal System"
/// section) to render without shipping every individual's full hormone
/// struct on every stats poll.
pub fn compute_population_hormone_stats(population: &[Individual]) -> Value {
    let living: Vec<&Individual> = population.iter().filter(|i| i.alive && !i.is_dead).collect();
    if living.is_empty() {
        return json!({ "cortisol": 0.0, "adrenaline": 0.0, "testosterone": 0.0, "estrogen": 0.0, "dopamine": 0.0, "oxytocin": 0.0 });
    }
    let n = living.len() as f64;
    let avg = |f: fn(&Hormones) -> f64| living.iter().map(|i| f(&i.hormones)).sum::<f64>() / n;
    json!({
        "cortisol": avg(|h| h.cortisol),
        "adrenaline": avg(|h| h.adrenaline),
        "testosterone": avg(|h| h.testosterone),
        "estrogen": avg(|h| h.estrogen),
        "dopamine": avg(|h| h.dopamine),
        "oxytocin": avg(|h| h.oxytocin),
    })
}

/// Real puberty timeline: negligible before 9, ramping through adolescence,
/// reaching the adult plateau by 17 -- matches `get_life_stage`'s own
/// "adolescent" band (12-18y) in biology/individual.rs.
fn puberty_curve(age_years: f64) -> f64 {
    if age_years < 9.0 {
        0.0
    } else if age_years < 17.0 {
        (age_years - 9.0) / 8.0
    } else {
        1.0
    }
}

/// Gradual male decline (andropause: slow, partial) vs. a sharper female
/// decline (menopause: steeper, over a shorter window) -- both real,
/// well-documented asymmetries in reproductive-hormone senescence.
fn senescence_curve(age_years: f64, sex: &str) -> f64 {
    if sex == "female" {
        if age_years < 45.0 {
            1.0
        } else if age_years < 55.0 {
            (1.0 - (age_years - 45.0) / 10.0 * 0.85).max(0.15)
        } else {
            0.15
        }
    } else if age_years < 50.0 {
        1.0
    } else {
        (1.0 - (age_years - 50.0) * 0.01).max(0.4)
    }
}

/// Baseline (pre-event-modulation) testosterone/estrogen for this
/// individual's current age, sex, and genetics. Both sexes carry some of
/// each (biologically accurate), with the sex-typical hormone dominant.
fn sex_hormone_baselines(sex: &str, age_years: f64, dominance: f64, fertility: f64) -> (f64, f64) {
    let puberty = puberty_curve(age_years);
    match sex {
        "male" => {
            let senescence = senescence_curve(age_years, sex);
            let dominance_mod = 0.7 + dominance * 0.3;
            let testosterone = (0.15 + 0.55 * puberty * dominance_mod * senescence).clamp(0.0, 1.0);
            (testosterone, 0.06)
        }
        "female" => {
            let senescence = senescence_curve(age_years, sex);
            let fertility_mod = 0.7 + fertility * 0.3;
            let estrogen = (0.12 + 0.55 * puberty * fertility_mod * senescence).clamp(0.0, 1.0);
            (0.08, estrogen)
        }
        _ => (0.1, 0.1),
    }
}

fn dopamine_baseline(curiosity: f64, risk_tolerance: f64) -> f64 {
    (0.35 + curiosity * 0.1 + risk_tolerance * 0.1).clamp(0.0, 1.0)
}

fn oxytocin_baseline(oxytocin_sensitivity: f64) -> f64 {
    (oxytocin_sensitivity * 0.3).clamp(0.0, 1.0)
}

/// Genetic/age baseline at birth (day 0 of this individual's life) -- called
/// once by every individual-creation path (`create_founder`, `create_child`,
/// `migrate_individual_arrival`), the same way `epigenetics::snapshot_genetic_baseline`
/// is. Nothing here reads any per-tick state; it exists purely so a
/// newborn's very first tick already has a sane, genetically-grounded
/// starting point instead of a flat default.
pub fn initialize_hormones(individual: &mut Individual) {
    let p = &individual.phenotype;
    let cortisol = (0.25 + p.stress_reactivity * 0.25).clamp(0.0, 1.0);
    let adrenaline = 0.05;
    let (testosterone, estrogen) = sex_hormone_baselines(&individual.sex, 0.0, p.dominance, p.fertility);
    let dopamine = dopamine_baseline(p.curiosity, p.risk_tolerance);
    let oxytocin = oxytocin_baseline(p.oxytocin_sensitivity);
    individual.hormones = Hormones { cortisol, adrenaline, testosterone, estrogen, dopamine, oxytocin, extra: Map::new() };
}

/// Daily update -- called once per living individual per tick, after
/// `psychology::update_mental_state` (needs this tick's fresh `stress_level`)
/// and after the economy phase (needs this tick's fresh `satiation`).
/// Every target is blended toward, not snapped to, at a hormone-specific
/// rate reflecting real clearance speed (adrenaline fastest, cortisol
/// moderate, testosterone/estrogen slowest).
pub fn update_hormones(individual: &mut Individual, current_day: i32) {
    let age_years = get_age(individual, current_day);
    let p = individual.phenotype.clone();
    let sex = individual.sex.clone();
    let h = individual.hormones.clone();

    // ---- Cortisol (HPA axis) ----
    // Driven by this tick's already-fully-rolled-up stress_level (disaster/
    // isolation/exile/trauma/water-fear all folded in there by
    // psychology::update_mental_state), scaled by genetic reactivity: two
    // individuals under identical circumstances don't secrete identical
    // cortisol.
    let stress = individual.psychology.stress_level;
    let cortisol_target = (stress * (0.4 + p.stress_reactivity * 0.6)).clamp(0.0, 1.0);
    let cortisol = h.cortisol + (cortisol_target - h.cortisol) * 0.2;

    // ---- Adrenaline (acute fight-or-flight) ----
    // Only a real, this-instant threat spikes it -- critically low HP is the
    // one signal available here that's unambiguously "immediate danger" on
    // every backend (WASM-local included) without depending on an event feed.
    let acute_threat = individual.health.hp < 0.25;
    let adrenaline_target = if acute_threat { (0.6 + p.risk_tolerance * 0.4).clamp(0.0, 1.0) } else { 0.05 };
    let adrenaline_rate = if acute_threat { 0.8 } else { 0.5 };
    let adrenaline = h.adrenaline + (adrenaline_target - h.adrenaline) * adrenaline_rate;

    // ---- Testosterone / Estrogen ----
    let (base_t, base_e) = sex_hormone_baselines(&sex, age_years, p.dominance, p.fertility);
    let pregnant = individual.health.pregnancy.is_some();
    let testosterone_target = base_t;
    let testosterone = h.testosterone + (testosterone_target - h.testosterone) * 0.1;
    // Pregnancy is the one estrogen swing genuinely tracked in per-tick state
    // (health.pregnancy) -- a real ~1.6x elevation, not a fixed replacement,
    // so it still respects the age/senescence baseline underneath it.
    let estrogen_target = if pregnant { (base_e * 1.6).min(1.0) } else { base_e };
    let estrogen = h.estrogen + (estrogen_target - h.estrogen) * 0.1;

    // ---- Dopamine (reward/motivation) ----
    // A same-tick nutritional swing is the one genuinely live, universally
    // available "did something good just happen" signal: satiation is
    // written fresh every tick by economy::consume_resources, before this
    // phase runs (see tick.rs's economy phase). Well-fed relative to a
    // recently-hungry baseline reads as a real reward; going hungry reads as
    // the opposite.
    let satiation = individual.extra.get("satiation").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let baseline_dopamine = dopamine_baseline(p.curiosity, p.risk_tolerance);
    let dopamine_target = if satiation > 0.75 {
        (baseline_dopamine + 0.3 * (satiation - 0.75) / 0.25).min(1.0)
    } else if satiation < 0.3 {
        (baseline_dopamine - 0.2 * (0.3 - satiation) / 0.3).max(0.05)
    } else {
        baseline_dopamine
    };
    let dopamine_rate = if satiation > 0.75 { 0.4 } else { 0.25 };
    let dopamine = h.dopamine + (dopamine_target - h.dopamine) * dopamine_rate;

    // ---- Oxytocin (dynamic circulating bonding hormone) ----
    // Ambient elevation from real, live group membership (physical/social
    // proximity to kin/allies); mating gives a direct one-tick surge applied
    // separately at its own call site (see `apply_mating_surge` below,
    // called from tick.rs right where conception is rolled) since that is a
    // discrete event, not something this per-tick baseline pass can see.
    let baseline_oxytocin = oxytocin_baseline(p.oxytocin_sensitivity);
    let oxytocin_target = if individual.group_id.is_some() { (baseline_oxytocin + p.oxytocin_sensitivity * 0.15).min(1.0) } else { baseline_oxytocin };
    let oxytocin = h.oxytocin + (oxytocin_target - h.oxytocin) * 0.15;

    individual.hormones = Hormones { cortisol, adrenaline, testosterone, estrogen, dopamine, oxytocin, extra: h.extra };
}

/// A conception event is a real, discrete, this-instant reproductive/social
/// signal -- called directly from tick.rs's reproduction phase, at the exact
/// point mating is rolled (the one live call site `psychology::process_bonding`
/// itself is invoked from), rather than inferred from an event log. Testosterone
/// and estrogen both get a modest one-tick surge in either parent (real mate-
/// competition/consolidation and reproductive-state physiology), and oxytocin
/// surges in proportion to each individual's own receptor sensitivity --
/// exactly the same asymmetry `psychology::process_bonding`'s own bond-strength
/// formula already models.
pub fn apply_mating_surge(mother: &mut Individual, father: &mut Individual) {
    for ind in [mother, father] {
        let sensitivity = ind.phenotype.oxytocin_sensitivity;
        ind.hormones.testosterone = (ind.hormones.testosterone + 0.1).min(1.0);
        ind.hormones.estrogen = (ind.hormones.estrogen + 0.1).min(1.0);
        ind.hormones.oxytocin = (ind.hormones.oxytocin + sensitivity * 0.4).min(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Health, Phenotype, Psychology};

    fn base_individual(sex: &str) -> Individual {
        Individual {
            sex: sex.to_string(),
            birth_day: 0,
            phenotype: Phenotype { stress_reactivity: 0.5, dominance: 0.5, fertility: 0.5, curiosity: 0.5, risk_tolerance: 0.5, oxytocin_sensitivity: 0.5, ..Default::default() },
            health: Health { hp: 1.0, ..Default::default() },
            psychology: Psychology { stress_level: 0.2, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn initialize_seeds_a_prepubertal_baseline_not_a_flat_default() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        // Age 0 -> puberty_curve(0) == 0.0, so testosterone should sit at its
        // prepubertal floor, not the adult plateau.
        assert!(ind.hormones.testosterone < 0.2, "expected a prepubertal testosterone floor, got {}", ind.hormones.testosterone);
    }

    #[test]
    fn male_testosterone_rises_through_puberty_and_plateaus_in_adulthood() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let adult_t = ind.hormones.testosterone;
        assert!(adult_t > 0.4, "expected an adult male to have a substantial testosterone plateau, got {adult_t}");
    }

    #[test]
    fn female_estrogen_rises_through_puberty_and_plateaus_in_adulthood() {
        let mut ind = base_individual("female");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let adult_e = ind.hormones.estrogen;
        assert!(adult_e > 0.4, "expected an adult female to have a substantial estrogen plateau, got {adult_e}");
    }

    #[test]
    fn pregnancy_elevates_estrogen_above_the_non_pregnant_baseline() {
        let mut ind = base_individual("female");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let baseline = ind.hormones.estrogen;
        ind.health.pregnancy = Some(25 * 365);
        for day in (25 * 365 + 1)..=(25 * 365 + 30) {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.estrogen > baseline, "pregnancy should elevate estrogen above baseline ({} vs {baseline})", ind.hormones.estrogen);
    }

    #[test]
    fn male_testosterone_declines_past_fifty_but_never_collapses() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=(30 * 365) {
            update_hormones(&mut ind, day);
        }
        let prime = ind.hormones.testosterone;
        for day in (30 * 365 + 1)..=(70 * 365) {
            update_hormones(&mut ind, day);
        }
        let elder = ind.hormones.testosterone;
        assert!(elder < prime, "andropause should lower testosterone below the prime-age level ({elder} vs {prime})");
        assert!(elder > 0.15, "andropause should be a partial decline, not a collapse to near-zero, got {elder}");
    }

    #[test]
    fn critically_low_hp_spikes_adrenaline() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.health.hp = 0.1;
        update_hormones(&mut ind, 1);
        assert!(ind.hormones.adrenaline > 0.3, "critical HP should spike adrenaline, got {}", ind.hormones.adrenaline);
    }

    #[test]
    fn adrenaline_clears_fast_once_the_threat_passes() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.health.hp = 0.1;
        update_hormones(&mut ind, 1);
        let spiked = ind.hormones.adrenaline;
        ind.health.hp = 1.0;
        update_hormones(&mut ind, 2);
        assert!(ind.hormones.adrenaline < spiked, "adrenaline should start clearing the tick after the threat ends");
    }

    #[test]
    fn high_stress_raises_cortisol_toward_a_reactivity_scaled_target() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.psychology.stress_level = 0.9;
        for day in 1..=10 {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.cortisol > 0.5, "sustained high stress should elevate cortisol, got {}", ind.hormones.cortisol);
    }

    #[test]
    fn cortisol_never_leaves_the_unit_interval_under_sustained_maximum_stress() {
        let mut ind = base_individual("male");
        ind.phenotype.stress_reactivity = 1.0;
        initialize_hormones(&mut ind);
        ind.psychology.stress_level = 1.0;
        for day in 1..=3650 {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.cortisol <= 1.0 && ind.hormones.cortisol >= 0.0);
    }

    #[test]
    fn a_recent_meal_after_hunger_gives_a_dopamine_reward_bump() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.extra.insert("satiation".to_string(), serde_json::json!(0.2));
        update_hormones(&mut ind, 1);
        let hungry_dopamine = ind.hormones.dopamine;
        ind.extra.insert("satiation".to_string(), serde_json::json!(0.95));
        update_hormones(&mut ind, 2);
        assert!(ind.hormones.dopamine > hungry_dopamine, "a well-fed swing should raise dopamine above the hungry level");
    }

    #[test]
    fn group_membership_elevates_ambient_oxytocin_over_isolation() {
        let mut solo = base_individual("male");
        initialize_hormones(&mut solo);
        solo.group_id = None;
        let mut grouped = base_individual("male");
        initialize_hormones(&mut grouped);
        grouped.group_id = Some("g1".to_string());
        for day in 1..=30 {
            update_hormones(&mut solo, day);
            update_hormones(&mut grouped, day);
        }
        assert!(grouped.hormones.oxytocin > solo.hormones.oxytocin);
    }

    #[test]
    fn mating_surges_oxytocin_testosterone_and_estrogen_in_both_parents() {
        let mut mother = base_individual("female");
        initialize_hormones(&mut mother);
        let mut father = base_individual("male");
        initialize_hormones(&mut father);
        let (mo_t, mo_e, mo_ox) = (mother.hormones.testosterone, mother.hormones.estrogen, mother.hormones.oxytocin);
        let (fa_t, fa_e, fa_ox) = (father.hormones.testosterone, father.hormones.estrogen, father.hormones.oxytocin);
        apply_mating_surge(&mut mother, &mut father);
        assert!(mother.hormones.testosterone > mo_t && mother.hormones.estrogen > mo_e && mother.hormones.oxytocin > mo_ox);
        assert!(father.hormones.testosterone > fa_t && father.hormones.estrogen > fa_e && father.hormones.oxytocin > fa_ox);
    }

    #[test]
    fn every_hormone_stays_within_the_unit_interval_over_a_long_run() {
        let mut ind = base_individual("female");
        initialize_hormones(&mut ind);
        for day in 1..=(90 * 365) {
            ind.health.pregnancy = if day % 270 < 30 { Some(day) } else { None };
            ind.psychology.stress_level = ((day % 100) as f64) / 100.0;
            update_hormones(&mut ind, day);
            let h = &ind.hormones;
            for v in [h.cortisol, h.adrenaline, h.testosterone, h.estrogen, h.dopamine, h.oxytocin] {
                assert!((0.0..=1.0).contains(&v), "hormone left [0,1] on day {day}: {v}");
            }
        }
    }
}
