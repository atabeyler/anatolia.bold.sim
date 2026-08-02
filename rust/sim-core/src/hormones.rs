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
//!
//! Twenty hormones total, organized as a genuine cascade rather than twenty
//! independent flat values -- matching how the real endocrine system is
//! actually organized around hypothalamic-pituitary-target-gland axes:
//! - **HPA axis**: `acth` (pituitary signal) drives `cortisol` (adrenal
//!   output), not stress_level directly.
//! - **HPT axis**: `tsh` rises under real negative feedback when `thyroid`
//!   (the prior tick's value) runs low, and drives it back up.
//! - **HPG axis**: `lh` (the gonadotropin pulse) modulates `testosterone`/
//!   `estrogen` production on top of their own age/sex baseline; `dhea` (an
//!   independent adrenal precursor) modulates both a second way.
//! - **Metabolic pair**: `insulin`/`glucagon` (fast, same-tick) and
//!   `leptin`/`ghrelin` (slow long-term trend vs. fast acute hunger) both
//!   track nutritional state but at genuinely different timescales.
//! - **Arousal cascade**: `norepinephrine` (sustained, weeks-scale
//!   vigilance) sets `adrenaline`'s own resting floor, the real locus
//!   coeruleus -> adrenal coupling.
//! - **Reproductive/bonding**: `oxytocin`/`vasopressin` (bonding, the
//!   latter more male-leaning), `prolactin` (post-birth), `progesterone`
//!   (pregnancy-specific, distinct from estrogen's own pregnancy bump).
//! - `growth_hormone` (age-curve only, deliberately left with no direct
//!   feedback hook -- see its own doc comment on why).
//!
//! **Deliberately not modeled**: hormones with no corresponding subsystem in
//! this simulation at all (digestive-tract hormones -- gastrin, secretin,
//! CCK, motilin -- since there is no real digestion subsystem, only abstract
//! calories/hydration; cardiovascular -- ANP, BNP, renin, erythropoietin --
//! no blood-pressure/blood-volume subsystem; bone/calcium -- PTH, calcitonin
//! -- no skeletal subsystem; melatonin -- no day/night cycle at this
//! simulation's one-tick-per-day resolution). Adding these as flat, causally
//! inert fields would violate the same cardinal-rule spirit this module
//! otherwise upholds: every value here is real and load-bearing, not
//! decorative. They can be added once/if their own underlying subsystem
//! exists to actually drive and be driven by them.

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
        return json!({
            "cortisol": 0.0, "adrenaline": 0.0, "testosterone": 0.0, "estrogen": 0.0, "dopamine": 0.0, "oxytocin": 0.0,
            "acth": 0.0, "tsh": 0.0, "lh": 0.0, "thyroid": 0.0, "insulin": 0.0, "glucagon": 0.0, "leptin": 0.0,
            "ghrelin": 0.0, "growth_hormone": 0.0, "dhea": 0.0, "prolactin": 0.0, "progesterone": 0.0,
            "norepinephrine": 0.0, "vasopressin": 0.0,
        });
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
        "acth": avg(|h| h.acth),
        "tsh": avg(|h| h.tsh),
        "lh": avg(|h| h.lh),
        "thyroid": avg(|h| h.thyroid),
        "insulin": avg(|h| h.insulin),
        "glucagon": avg(|h| h.glucagon),
        "leptin": avg(|h| h.leptin),
        "ghrelin": avg(|h| h.ghrelin),
        "growth_hormone": avg(|h| h.growth_hormone),
        "dhea": avg(|h| h.dhea),
        "prolactin": avg(|h| h.prolactin),
        "progesterone": avg(|h| h.progesterone),
        "norepinephrine": avg(|h| h.norepinephrine),
        "vasopressin": avg(|h| h.vasopressin),
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

/// DHEA-S's real lifetime curve: rises through childhood/puberty, peaks
/// around 25, then declines steadily ("adrenopause") -- independent of, and
/// in addition to, the sex-specific senescence curve above (DHEA is an
/// adrenal precursor shared by both sexes, not a gonadal hormone).
fn dhea_curve(age_years: f64) -> f64 {
    if age_years < 25.0 {
        (age_years / 25.0).clamp(0.0, 1.0)
    } else {
        (1.0 - (age_years - 25.0) * 0.012).max(0.1)
    }
}

/// Real GH pulsatility peaks around puberty, then declines through
/// adulthood ("somatopause").
fn growth_hormone_curve(age_years: f64) -> f64 {
    if age_years < 17.0 {
        (0.4 + puberty_curve(age_years) * 0.6).min(1.0)
    } else {
        (1.0 - (age_years - 17.0) * 0.012).max(0.15)
    }
}

/// Baseline (pre-LH/DHEA-modulation) testosterone/estrogen for this
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
    let acth = cortisol;
    let adrenaline = 0.05;
    let norepinephrine = 0.1;
    let (testosterone, estrogen) = sex_hormone_baselines(&individual.sex, 0.0, p.dominance, p.fertility);
    let dopamine = dopamine_baseline(p.curiosity, p.risk_tolerance);
    let oxytocin = oxytocin_baseline(p.oxytocin_sensitivity);
    let vasopressin_sensitivity = (p.parental_care * 0.5 + p.cooperation * 0.5).clamp(0.0, 1.0);
    let vasopressin = (vasopressin_sensitivity * 0.3).clamp(0.0, 1.0);
    let tsh = 0.5;
    let thyroid = 0.5;
    let lh = puberty_curve(0.0);
    let dhea = dhea_curve(0.0);
    let growth_hormone = growth_hormone_curve(0.0);
    let progesterone = if individual.sex == "female" { 0.15 } else { 0.05 };
    individual.hormones = Hormones {
        cortisol,
        adrenaline,
        testosterone,
        estrogen,
        dopamine,
        oxytocin,
        acth,
        tsh,
        lh,
        thyroid,
        insulin: 0.4,
        glucagon: 0.3,
        leptin: 0.3,
        ghrelin: 0.4,
        growth_hormone,
        dhea,
        prolactin: 0.05,
        progesterone,
        norepinephrine,
        vasopressin,
        extra: Map::new(),
    };
}

/// Daily update -- called once per living individual per tick, after
/// `psychology::update_mental_state` (needs this tick's fresh `stress_level`)
/// and after the economy phase (needs this tick's fresh `satiation`). Every
/// target is blended toward, not snapped to, at a hormone-specific rate
/// reflecting real clearance/gland-response speed. Several hormones form a
/// genuine same-tick cascade (computed in dependency order below: ACTH
/// before cortisol, LH/DHEA before testosterone/estrogen, TSH before
/// thyroid, norepinephrine before adrenaline); the negative-feedback pairs
/// (TSH<->thyroid) intentionally read the *previous* tick's value on one
/// side to avoid a same-tick circular dependency -- itself realistic, since
/// real feedback loops operate with a delay.
pub fn update_hormones(individual: &mut Individual, current_day: i32) {
    let age_years = get_age(individual, current_day);
    let p = individual.phenotype.clone();
    let sex = individual.sex.clone();
    let h = individual.hormones.clone();

    let satiation = individual.extra.get("satiation").and_then(|v| v.as_f64()).unwrap_or(0.5);

    // ---- Metabolic pair 1: insulin / glucagon (fast, same-tick) ----
    let insulin = h.insulin + (satiation - h.insulin) * 0.35;
    let glucagon = h.glucagon + ((1.0 - satiation) - h.glucagon) * 0.35;

    // ---- Metabolic pair 2: leptin (slow trend) / ghrelin (fast, acute) ----
    let leptin = h.leptin + (satiation - h.leptin) * 0.03;
    let ghrelin = h.ghrelin + ((1.0 - satiation) - h.ghrelin) * 0.4;

    // ---- HPA axis: ACTH (pituitary) drives cortisol (adrenal) ----
    let stress = individual.psychology.stress_level;
    let acth = h.acth + (stress - h.acth) * 0.3;
    let cortisol_target = (acth * (0.4 + p.stress_reactivity * 0.6)).clamp(0.0, 1.0);
    let cortisol = h.cortisol + (cortisol_target - h.cortisol) * 0.2;

    // ---- Arousal cascade: norepinephrine (sustained) sets adrenaline's floor ----
    // Only a real, this-instant threat spikes adrenaline hard -- critically
    // low HP or severe acute hunger (ghrelin) are the two signals available
    // here that are unambiguously "urgent" on every backend (WASM-local
    // included) without depending on an event feed.
    let acute_threat = individual.health.hp < 0.25 || ghrelin > 0.8;
    let norepinephrine_target = if acute_threat { 0.6 } else { (0.15 + stress * 0.2).clamp(0.0, 1.0) };
    let norepinephrine = h.norepinephrine + (norepinephrine_target - h.norepinephrine) * 0.15;
    let adrenaline_target = if acute_threat { (0.6 + p.risk_tolerance * 0.4).clamp(0.0, 1.0) } else { (norepinephrine * 0.3 + 0.05).clamp(0.0, 1.0) };
    let adrenaline_rate = if acute_threat { 0.8 } else { 0.5 };
    let adrenaline = h.adrenaline + (adrenaline_target - h.adrenaline) * adrenaline_rate;

    // ---- HPT axis: TSH <-> thyroid negative feedback ----
    // TSH rises when the *previous* tick's thyroid output was low (the real
    // pituitary negative-feedback signal); thyroid then responds to this
    // tick's fresh TSH plus nutritional state (sustained undernourishment
    // suppresses thyroid output -- the real "sick euthyroid" energy-
    // conservation adaptation).
    let tsh_target = (1.0 - h.thyroid).clamp(0.0, 1.0) * 0.7 + 0.15;
    let tsh = h.tsh + (tsh_target - h.tsh) * 0.15;
    let thyroid_target = (0.25 + satiation * 0.35 + tsh * 0.3).clamp(0.0, 1.0);
    let thyroid = h.thyroid + (thyroid_target - h.thyroid) * 0.08;

    // ---- HPG axis: LH (gonadotropin pulse) + DHEA modulate testosterone/estrogen ----
    let lh_target = puberty_curve(age_years);
    let lh = h.lh + (lh_target - h.lh) * 0.12;
    let dhea_target = dhea_curve(age_years);
    let dhea = h.dhea + (dhea_target - h.dhea) * 0.05;
    let (base_t, base_e) = sex_hormone_baselines(&sex, age_years, p.dominance, p.fertility);
    let pregnant = individual.health.pregnancy.is_some();
    let lh_gain = 0.7 + 0.3 * lh;
    let dhea_gain = 0.85 + 0.15 * dhea;
    let testosterone_target = (base_t * lh_gain * dhea_gain).clamp(0.0, 1.0);
    let testosterone = h.testosterone + (testosterone_target - h.testosterone) * 0.1;
    let estrogen_base = if pregnant { (base_e * 1.6).min(1.0) } else { base_e };
    let estrogen_target = (estrogen_base * lh_gain * dhea_gain).clamp(0.0, 1.0);
    let estrogen = h.estrogen + (estrogen_target - h.estrogen) * 0.1;

    // ---- Progesterone: pregnancy-specific, distinct from estrogen's own dynamic ----
    let progesterone_target = if pregnant {
        0.85
    } else if sex == "female" {
        (0.1 + p.fertility * 0.1 * puberty_curve(age_years)).clamp(0.0, 1.0)
    } else {
        0.05
    };
    let progesterone = h.progesterone + (progesterone_target - h.progesterone) * 0.12;

    // ---- Growth hormone: age-curve only (see module doc comment) ----
    let growth_hormone_target = growth_hormone_curve(age_years);
    let growth_hormone = h.growth_hormone + (growth_hormone_target - h.growth_hormone) * 0.05;

    // ---- Dopamine (reward/motivation) ----
    // A same-tick nutritional swing is the primary reward signal; chronic
    // leptin depletion (long-term energy-reserve deficit) also blunts reward
    // sensitivity by a small, bounded amount -- a real, well-documented
    // leptin-dopamine coupling.
    let baseline_dopamine = dopamine_baseline(p.curiosity, p.risk_tolerance);
    let dopamine_target_raw = if satiation > 0.75 {
        (baseline_dopamine + 0.3 * (satiation - 0.75) / 0.25).min(1.0)
    } else if satiation < 0.3 {
        (baseline_dopamine - 0.2 * (0.3 - satiation) / 0.3).max(0.05)
    } else {
        baseline_dopamine
    };
    let dopamine_target = (dopamine_target_raw * (0.9 + 0.1 * leptin)).clamp(0.0, 1.0);
    let dopamine_rate = if satiation > 0.75 { 0.4 } else { 0.25 };
    let dopamine = h.dopamine + (dopamine_target - h.dopamine) * dopamine_rate;

    // ---- Oxytocin / vasopressin (bonding) ----
    let baseline_oxytocin = oxytocin_baseline(p.oxytocin_sensitivity);
    let oxytocin_target = if individual.group_id.is_some() { (baseline_oxytocin + p.oxytocin_sensitivity * 0.15).min(1.0) } else { baseline_oxytocin };
    let oxytocin = h.oxytocin + (oxytocin_target - h.oxytocin) * 0.15;
    let vasopressin_sensitivity = (p.parental_care * 0.5 + p.cooperation * 0.5).clamp(0.0, 1.0);
    let baseline_vasopressin = (vasopressin_sensitivity * 0.3).clamp(0.0, 1.0);
    let vasopressin_target = if individual.group_id.is_some() { (baseline_vasopressin + vasopressin_sensitivity * 0.15).min(1.0) } else { baseline_vasopressin };
    let vasopressin = h.vasopressin + (vasopressin_target - h.vasopressin) * 0.15;

    // ---- Prolactin: decays slowly from whatever apply_birth_surge set ----
    let prolactin_target = 0.05;
    let prolactin = h.prolactin + (prolactin_target - h.prolactin) * 0.02;

    individual.hormones = Hormones {
        cortisol,
        adrenaline,
        testosterone,
        estrogen,
        dopamine,
        oxytocin,
        acth,
        tsh,
        lh,
        thyroid,
        insulin,
        glucagon,
        leptin,
        ghrelin,
        growth_hormone,
        dhea,
        prolactin,
        progesterone,
        norepinephrine,
        vasopressin,
        extra: h.extra,
    };
}

/// A conception event is a real, discrete, this-instant reproductive/social
/// signal -- called directly from tick.rs's reproduction phase, at the exact
/// point mating is rolled (the one live call site `psychology::process_bonding`
/// itself is invoked from), rather than inferred from an event log. LH (the
/// actual physiological gonadotropin trigger) surges first, testosterone and
/// estrogen follow, and oxytocin/vasopressin surge in proportion to each
/// individual's own receptor sensitivity -- exactly the same asymmetry
/// `psychology::process_bonding`'s own bond-strength formula already models.
pub fn apply_mating_surge(mother: &mut Individual, father: &mut Individual) {
    for ind in [mother, father] {
        let sensitivity = ind.phenotype.oxytocin_sensitivity;
        let vasopressin_sensitivity = (ind.phenotype.parental_care * 0.5 + ind.phenotype.cooperation * 0.5).clamp(0.0, 1.0);
        ind.hormones.lh = (ind.hormones.lh + 0.15).min(1.0);
        ind.hormones.testosterone = (ind.hormones.testosterone + 0.1).min(1.0);
        ind.hormones.estrogen = (ind.hormones.estrogen + 0.1).min(1.0);
        ind.hormones.oxytocin = (ind.hormones.oxytocin + sensitivity * 0.4).min(1.0);
        ind.hormones.vasopressin = (ind.hormones.vasopressin + vasopressin_sensitivity * 0.4).min(1.0);
    }
}

/// A real, discrete, this-instant event (unlike prolactin's own otherwise-flat
/// per-tick target) -- called directly from tick.rs at the exact point a
/// birth is resolved, mirroring `apply_mating_surge`'s own pattern. Prolactin
/// then decays slowly back toward baseline over subsequent ticks (see
/// `update_hormones`), matching real lactation's weeks-scale elevation.
pub fn apply_birth_surge(mother: &mut Individual) {
    mother.hormones.prolactin = (mother.hormones.prolactin + 0.7).min(1.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Health, Phenotype, Psychology};

    fn base_individual(sex: &str) -> Individual {
        Individual {
            sex: sex.to_string(),
            birth_day: 0,
            phenotype: Phenotype {
                stress_reactivity: 0.5,
                dominance: 0.5,
                fertility: 0.5,
                curiosity: 0.5,
                risk_tolerance: 0.5,
                oxytocin_sensitivity: 0.5,
                parental_care: 0.5,
                cooperation: 0.5,
                ..Default::default()
            },
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
    fn pregnancy_elevates_progesterone_far_above_the_non_pregnant_baseline() {
        let mut ind = base_individual("female");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let baseline = ind.hormones.progesterone;
        ind.health.pregnancy = Some(25 * 365);
        for day in (25 * 365 + 1)..=(25 * 365 + 60) {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.progesterone > baseline + 0.3, "pregnancy should sharply elevate progesterone ({} vs baseline {baseline})", ind.hormones.progesterone);
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
        assert!(elder > 0.1, "andropause should be a partial decline, not a collapse to near-zero, got {elder}");
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
        ind.extra.insert("satiation".to_string(), serde_json::json!(0.5));
        update_hormones(&mut ind, 2);
        assert!(ind.hormones.adrenaline < spiked, "adrenaline should start clearing the tick after the threat ends");
    }

    #[test]
    fn severe_acute_hunger_also_spikes_adrenaline_via_ghrelin() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=5 {
            ind.extra.insert("satiation".to_string(), serde_json::json!(0.02));
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.ghrelin > 0.8, "sustained near-zero satiation should push ghrelin high, got {}", ind.hormones.ghrelin);
        assert!(ind.hormones.adrenaline > 0.3, "severe acute hunger should spike adrenaline via ghrelin, got {}", ind.hormones.adrenaline);
    }

    #[test]
    fn high_stress_raises_cortisol_toward_a_reactivity_scaled_target_via_acth() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.psychology.stress_level = 0.9;
        for day in 1..=10 {
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.acth > 0.7, "sustained high stress should elevate ACTH first, got {}", ind.hormones.acth);
        assert!(ind.hormones.cortisol > 0.5, "sustained high stress should elevate cortisol via ACTH, got {}", ind.hormones.cortisol);
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
    fn thyroid_falls_under_sustained_undernourishment() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=60 {
            ind.extra.insert("satiation".to_string(), serde_json::json!(0.9));
            update_hormones(&mut ind, day);
        }
        let well_fed_thyroid = ind.hormones.thyroid;
        for day in 61..=200 {
            ind.extra.insert("satiation".to_string(), serde_json::json!(0.1));
            update_hormones(&mut ind, day);
        }
        assert!(ind.hormones.thyroid < well_fed_thyroid, "sustained undernourishment should lower thyroid output ({} vs well-fed {well_fed_thyroid})", ind.hormones.thyroid);
    }

    #[test]
    fn tsh_rises_when_thyroid_output_is_low_real_negative_feedback() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        ind.hormones.thyroid = 0.1;
        update_hormones(&mut ind, 1);
        assert!(ind.hormones.tsh > 0.5, "low thyroid should trigger a TSH negative-feedback rise, got {}", ind.hormones.tsh);
    }

    #[test]
    fn dhea_peaks_in_young_adulthood_and_declines_with_age() {
        let mut ind = base_individual("male");
        initialize_hormones(&mut ind);
        for day in 1..=(25 * 365) {
            update_hormones(&mut ind, day);
        }
        let young_adult = ind.hormones.dhea;
        for day in (25 * 365 + 1)..=(70 * 365) {
            update_hormones(&mut ind, day);
        }
        let elder = ind.hormones.dhea;
        assert!(elder < young_adult, "DHEA should decline past young adulthood ({elder} vs {young_adult})");
    }

    #[test]
    fn group_membership_elevates_ambient_oxytocin_and_vasopressin_over_isolation() {
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
        assert!(grouped.hormones.vasopressin > solo.hormones.vasopressin);
    }

    #[test]
    fn mating_surges_lh_oxytocin_vasopressin_testosterone_and_estrogen_in_both_parents() {
        let mut mother = base_individual("female");
        initialize_hormones(&mut mother);
        let mut father = base_individual("male");
        initialize_hormones(&mut father);
        let (mo_t, mo_e, mo_ox, mo_avp, mo_lh) = (mother.hormones.testosterone, mother.hormones.estrogen, mother.hormones.oxytocin, mother.hormones.vasopressin, mother.hormones.lh);
        let (fa_t, fa_e, fa_ox, fa_avp, fa_lh) = (father.hormones.testosterone, father.hormones.estrogen, father.hormones.oxytocin, father.hormones.vasopressin, father.hormones.lh);
        apply_mating_surge(&mut mother, &mut father);
        assert!(mother.hormones.testosterone > mo_t && mother.hormones.estrogen > mo_e && mother.hormones.oxytocin > mo_ox && mother.hormones.vasopressin > mo_avp && mother.hormones.lh > mo_lh);
        assert!(father.hormones.testosterone > fa_t && father.hormones.estrogen > fa_e && father.hormones.oxytocin > fa_ox && father.hormones.vasopressin > fa_avp && father.hormones.lh > fa_lh);
    }

    #[test]
    fn birth_surges_prolactin_which_then_decays_slowly() {
        let mut mother = base_individual("female");
        initialize_hormones(&mut mother);
        let baseline = mother.hormones.prolactin;
        apply_birth_surge(&mut mother);
        let surged = mother.hormones.prolactin;
        assert!(surged > baseline + 0.3, "birth should sharply surge prolactin ({surged} vs baseline {baseline})");
        update_hormones(&mut mother, 1);
        assert!(mother.hormones.prolactin < surged, "prolactin should start decaying the tick after birth");
        assert!(mother.hormones.prolactin > baseline, "prolactin should still be well above baseline immediately after birth");
    }

    #[test]
    fn every_hormone_stays_within_the_unit_interval_over_a_long_run() {
        let mut ind = base_individual("female");
        initialize_hormones(&mut ind);
        for day in 1..=(90 * 365) {
            ind.health.pregnancy = if day % 270 < 30 { Some(day) } else { None };
            ind.psychology.stress_level = ((day % 100) as f64) / 100.0;
            ind.extra.insert("satiation".to_string(), serde_json::json!(((day % 100) as f64) / 100.0));
            update_hormones(&mut ind, day);
            let h = &ind.hormones;
            for v in [
                h.cortisol, h.adrenaline, h.testosterone, h.estrogen, h.dopamine, h.oxytocin, h.acth, h.tsh, h.lh, h.thyroid, h.insulin, h.glucagon, h.leptin, h.ghrelin,
                h.growth_hormone, h.dhea, h.prolactin, h.progesterone, h.norepinephrine, h.vasopressin,
            ] {
                assert!((0.0..=1.0).contains(&v), "hormone left [0,1] on day {day}: {v}");
            }
        }
    }
}
