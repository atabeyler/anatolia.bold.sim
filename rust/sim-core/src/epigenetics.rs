use serde_json::Value;

use crate::state::Individual;
use crate::types::EpigeneticLocus;

/// The five phenotype traits `apply_fx` is allowed to modulate from
/// methylation, and the `extra` key their genome-derived starting values are
/// snapshotted under at birth (see `snapshot_genetic_baseline`).
const DRIFTABLE_TRAITS: &[&str] = &["stress_reactivity", "aggression", "oxytocin_sensitivity", "learning_rate", "immune_strength"];
const BASELINE_KEY: &str = "_epi_genetic_baseline";

/// How far epigenetics may pull a trait away from its genetic baseline, as a
/// fraction of the gap to the methylation-implied target -- e.g. 0.25 means
/// the trait can move at most a quarter of the way from the genotype's own
/// value toward what current methylation alone would imply. This is
/// recomputed fresh every call from the *stored* genetic baseline (never
/// from the trait's own previous value), so daily ticks cannot compound into
/// erasing the genetic component the way a recursive `x = x*0.99 + target*0.01`
/// EMA eventually does.
const EPIGENETIC_INFLUENCE: f64 = 0.25;

/// Captures each driftable trait's genome-only value into `individual.extra`
/// once, at birth (called from `create_founder`/`create_child` right after
/// `compute_phenotype`). `apply_fx` reads this back as the fixed reference
/// point epigenetics modulates around instead of overwriting.
pub fn snapshot_genetic_baseline(individual: &mut Individual) {
    let p = &individual.phenotype;
    let values = [p.stress_reactivity, p.aggression, p.oxytocin_sensitivity, p.learning_rate, p.immune_strength];
    let obj: serde_json::Map<String, serde_json::Value> =
        DRIFTABLE_TRAITS.iter().zip(values).map(|(k, v)| ((*k).to_string(), serde_json::json!(v))).collect();
    individual.extra.insert(BASELINE_KEY.to_string(), serde_json::Value::Object(obj));
}

/// Reads back the birth-time genetic baseline for a trait, falling back to
/// the trait's current value if the snapshot is missing (simulations saved
/// before this field existed) -- self-heals going forward the same way
/// `world_state.phoneme_palette` does, per AGENTS.md.
fn genetic_baseline(individual: &Individual, trait_name: &str, current: f64) -> f64 {
    individual
        .extra
        .get(BASELINE_KEY)
        .and_then(|v| v.get(trait_name))
        .and_then(|v| v.as_f64())
        .unwrap_or(current)
}

const LOCI: &[(&str, bool, f64)] = &[
    ("HPA_AXIS", true, 0.3),
    ("BDNF_PROMOTER", true, 0.2),
    ("MAOA_REGULATION", false, 0.4),
    ("LEPTIN_RESIST", true, 0.5),
    ("INSULIN_SENS", true, 0.35),
    ("OXTR_METHYL", true, 0.45),
    ("AVP_REGULATION", true, 0.3),
    ("IMMUNE_PRIMING", false, 0.6),
];

pub fn initialize_epigenome(individual: &mut Individual) {
    individual.epigenome = LOCI
        .iter()
        .map(|(id, ..)| ((*id).to_string(), EpigeneticLocus { methylation: 0.5, last_modified: None }))
        .collect();
}

pub fn inherit_epigenome(child: &mut Individual, p1: &mut Individual, p2: &mut Individual) {
    if p1.epigenome.is_empty() {
        initialize_epigenome(p1);
    }
    if p2.epigenome.is_empty() {
        initialize_epigenome(p2);
    }
    child.epigenome = LOCI
        .iter()
        .map(|(id, _, h)| {
            let m1 = p1.epigenome.get(*id).map(|l| l.methylation).unwrap_or(0.5);
            let m2 = p2.epigenome.get(*id).map(|l| l.methylation).unwrap_or(0.5);
            let methylation = (0.5 + (((m1 + m2) / 2.0) - 0.5) * h).clamp(0.0, 1.0);
            ((*id).to_string(), EpigeneticLocus { methylation, last_modified: Some(0) })
        })
        .collect();
}

pub fn update_epigenome(individual: &mut Individual, _env: Option<&serde_json::Value>, sim_day: i32) {
    if individual.epigenome.is_empty() {
        initialize_epigenome(individual);
    }
    let stress = individual.psychology.stress_level;
    let nutrition = individual.extra.get("satiation").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let social = if individual.group_id.is_some() { 0.6 } else { 0.2 };
    // A *collective* trauma (a disaster/predator/conflict event logged today,
    // as opposed to an individual "kin_death" -- see psychology::update_mental_state)
    // hits every affected group member on the same day, not just one
    // individual's private grief. It leaves a markedly stronger HPA_AXIS
    // imprint than ordinary high stress, so descendants of disaster
    // survivors (HPA_AXIS is heritable at 0.3, see LOCI above) carry a
    // measurably stronger inherited stress-reactivity shift -- a "cultural
    // memory" of the event encoded purely through the existing epigenetic
    // inheritance pathway, not a new mechanism.
    let collective_trauma_today = individual
        .psychology
        .trauma_events
        .iter()
        .any(|e| e.get("day").and_then(Value::as_i64) == Some(sim_day as i64) && e.get("type").and_then(Value::as_str).map(|t| t != "kin_death").unwrap_or(false));
    mod_locus(individual, "HPA_AXIS", if collective_trauma_today { 0.05 } else if stress > 0.7 { 0.02 } else { -0.005 }, sim_day);
    mod_locus(individual, "LEPTIN_RESIST", if nutrition < 0.3 { 0.01 } else { -0.005 }, sim_day);
    mod_locus(individual, "OXTR_METHYL", if social < 0.3 { 0.01 } else { -0.01 }, sim_day);
    if (individual.age_days.unwrap_or(0) as f64) / 365.0 < 5.0 && stress > 0.6 {
        mod_locus(individual, "MAOA_REGULATION", 0.03, sim_day);
    }
    if individual.extra.get("infections").and_then(|v| v.as_array()).map(|a| !a.is_empty()).unwrap_or(false) {
        mod_locus(individual, "IMMUNE_PRIMING", 0.02, sim_day);
    }
    if nutrition < 0.2 {
        mod_locus(individual, "BDNF_PROMOTER", 0.015, sim_day);
    } else if nutrition > 0.7 {
        mod_locus(individual, "BDNF_PROMOTER", -0.003, sim_day);
    }
    mod_locus(individual, "INSULIN_SENS", if nutrition < 0.3 { 0.01 } else { -0.005 }, sim_day);
    let hydration = individual.health.hydration;
    let isolated = individual.group_id.is_none();
    mod_locus(individual, "AVP_REGULATION", if hydration < 0.3 || isolated { 0.01 } else { -0.005 }, sim_day);
    apply_fx(individual);
}

fn mod_locus(individual: &mut Individual, id: &str, delta: f64, sim_day: i32) {
    let Some((_, reversible, _)) = LOCI.iter().find(|(k, _, _)| *k == id) else { return };
    let locus = individual.epigenome.entry(id.to_string()).or_insert(EpigeneticLocus { methylation: 0.5, last_modified: None });
    if !*reversible && delta < 0.0 {
        return;
    }
    locus.methylation = (locus.methylation + delta).clamp(0.0, 1.0);
    locus.last_modified = Some(sim_day);
}

fn apply_fx(ind: &mut Individual) {
    if !ind.extra.contains_key(BASELINE_KEY) {
        snapshot_genetic_baseline(ind);
    }
    let get = |id: &str| ind.epigenome.get(id).map(|l| l.methylation).unwrap_or(0.5);
    let base = |ind: &Individual, trait_name: &str, current: f64| genetic_baseline(ind, trait_name, current);
    let blend = |baseline: f64, target: f64| (baseline + (target - baseline) * EPIGENETIC_INFLUENCE).clamp(0.0, 1.0);

    let stress_reactivity_base = base(ind, "stress_reactivity", ind.phenotype.stress_reactivity);
    let aggression_base = base(ind, "aggression", ind.phenotype.aggression);
    let oxytocin_base = base(ind, "oxytocin_sensitivity", ind.phenotype.oxytocin_sensitivity);
    let learning_rate_base = base(ind, "learning_rate", ind.phenotype.learning_rate);
    let immune_base = base(ind, "immune_strength", ind.phenotype.immune_strength);

    let p = &mut ind.phenotype;
    p.stress_reactivity = blend(stress_reactivity_base, get("HPA_AXIS"));
    p.aggression = blend(aggression_base, get("MAOA_REGULATION"));
    p.oxytocin_sensitivity = blend(oxytocin_base, 1.0 - get("OXTR_METHYL"));
    p.learning_rate = blend(learning_rate_base, 1.0 - get("BDNF_PROMOTER"));
    p.immune_strength = blend(immune_base, get("IMMUNE_PRIMING"));
}

pub fn compute_epigenetic_age(individual: &Individual, current_day: i32) -> f64 {
    // Founders encode birth_day as a negative "age at creation" offset, so
    // current_day - birth_day correctly recovers age-in-days for both them
    // and ordinary (positive birth_day) individuals when age_days hasn't
    // been refreshed yet -- unlike birth_day.abs(), which silently returned
    // a child's raw birth-day number (not their age) whenever age_days was
    // unset.
    let age_days = individual.age_days.unwrap_or((current_day - individual.birth_day).max(0)).max(0) as f64;
    let ca = age_days / 365.0;
    let hpa = individual.epigenome.get("HPA_AXIS").map(|l| l.methylation).unwrap_or(0.5);
    let lep = individual.epigenome.get("LEPTIN_RESIST").map(|l| l.methylation).unwrap_or(0.5);
    ca * (0.8 + ((hpa + lep) / 2.0) * 0.4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Health, Psychology};

    fn stressed_individual() -> Individual {
        Individual {
            psychology: Psychology { stress_level: 0.9, ..Default::default() },
            health: Health { hydration: 0.9, ..Default::default() },
            group_id: Some("g1".to_string()),
            age_days: Some(365 * 30),
            ..Default::default()
        }
    }

    #[test]
    fn irreversible_loci_never_decrease_even_under_relaxing_conditions() {
        // MAOA_REGULATION and IMMUNE_PRIMING are marked irreversible: once methylated,
        // no combination of low stress / good nutrition may lower them again.
        let mut ind = stressed_individual();
        ind.age_days = Some(1); // < 5 years old, high stress -> MAOA_REGULATION bump
        update_epigenome(&mut ind, None, 1);
        let maoa_after_stress = ind.epigenome["MAOA_REGULATION"].methylation;
        assert!(maoa_after_stress > 0.5, "high early-life stress should raise MAOA_REGULATION methylation");

        // Now flip to maximally relaxed conditions and tick many times.
        ind.psychology = Psychology { stress_level: 0.0, ..Default::default() };
        ind.extra.insert("satiation".to_string(), serde_json::json!(1.0));
        ind.health = Health { hydration: 1.0, ..Default::default() };
        ind.group_id = Some("g1".to_string());
        for day in 2..500 {
            update_epigenome(&mut ind, None, day);
        }
        let maoa_after_relaxation = ind.epigenome["MAOA_REGULATION"].methylation;
        assert!(
            maoa_after_relaxation >= maoa_after_stress - 1e-9,
            "irreversible locus MAOA_REGULATION decreased from {maoa_after_stress} to {maoa_after_relaxation}"
        );
    }

    #[test]
    fn reversible_loci_can_both_rise_and_fall() {
        let mut ind = stressed_individual();
        update_epigenome(&mut ind, None, 1);
        let hpa_high_stress = ind.epigenome["HPA_AXIS"].methylation;

        ind.psychology = Psychology { stress_level: 0.0, ..Default::default() };
        for day in 2..50 {
            update_epigenome(&mut ind, None, day);
        }
        let hpa_after_calm = ind.epigenome["HPA_AXIS"].methylation;
        assert!(hpa_after_calm < hpa_high_stress, "reversible HPA_AXIS should relax back down once stress drops");
    }

    #[test]
    fn methylation_is_driven_only_by_the_individuals_own_signals_not_external_injection() {
        // Cardinal rule (non-founder): epigenome may only move in response to the
        // individual's own internal state passed into update_epigenome, never by
        // another system writing the map directly. This test locks the *pathway*:
        // identical internal signals on two distinct individuals must produce
        // identical epigenetic outcomes (no hidden per-individual external inputs).
        let mut a = stressed_individual();
        let mut b = stressed_individual();
        for day in 1..30 {
            update_epigenome(&mut a, None, day);
            update_epigenome(&mut b, None, day);
        }
        assert_eq!(a.epigenome, b.epigenome);
    }

    #[test]
    fn child_starts_at_neutral_zero_point_five_when_both_parents_are_neutral() {
        let mut p1 = Individual::default();
        let mut p2 = Individual::default();
        initialize_epigenome(&mut p1);
        initialize_epigenome(&mut p2);
        let mut child = Individual::default();
        inherit_epigenome(&mut child, &mut p1, &mut p2);
        for (id, ..) in LOCI {
            assert!((child.epigenome[*id].methylation - 0.5).abs() < 1e-9, "{id} should stay neutral");
        }
    }

    #[test]
    fn immune_priming_heritability_zero_point_six_at_parents_one_gives_child_zero_point_eight() {
        let mut p1 = Individual::default();
        let mut p2 = Individual::default();
        initialize_epigenome(&mut p1);
        initialize_epigenome(&mut p2);
        for p in [&mut p1, &mut p2] {
            for locus in p.epigenome.values_mut() {
                locus.methylation = 1.0;
            }
        }
        let mut child = Individual::default();
        inherit_epigenome(&mut child, &mut p1, &mut p2);
        assert!((child.epigenome["IMMUNE_PRIMING"].methylation - 0.8).abs() < 1e-9);
    }

    #[test]
    fn hpa_axis_heritability_zero_point_three_at_parents_one_gives_child_zero_point_six_five() {
        let mut p1 = Individual::default();
        let mut p2 = Individual::default();
        initialize_epigenome(&mut p1);
        initialize_epigenome(&mut p2);
        for p in [&mut p1, &mut p2] {
            for locus in p.epigenome.values_mut() {
                locus.methylation = 1.0;
            }
        }
        let mut child = Individual::default();
        inherit_epigenome(&mut child, &mut p1, &mut p2);
        assert!((child.epigenome["HPA_AXIS"].methylation - 0.65).abs() < 1e-9);
    }

    #[test]
    fn inherited_methylation_stays_clamped_to_zero_one() {
        let mut p1 = Individual::default();
        let mut p2 = Individual::default();
        initialize_epigenome(&mut p1);
        initialize_epigenome(&mut p2);
        for p in [&mut p1, &mut p2] {
            for locus in p.epigenome.values_mut() {
                locus.methylation = 0.0;
            }
        }
        let mut child = Individual::default();
        inherit_epigenome(&mut child, &mut p1, &mut p2);
        for locus in child.epigenome.values() {
            assert!((0.0..=1.0).contains(&locus.methylation));
        }
    }

    #[test]
    fn inherit_epigenome_initializes_missing_parent_epigenomes_automatically() {
        let mut p1 = Individual::default();
        let mut p2 = Individual::default();
        let mut child = Individual::default();
        inherit_epigenome(&mut child, &mut p1, &mut p2);
        assert_eq!(child.epigenome.len(), LOCI.len());
    }

    // ── updateEpigenome ─────────────────────────────────────────────────

    #[test]
    fn update_epigenome_never_touches_phenotype_anxiety() {
        // applyFX derives stress_reactivity/aggression/oxytocin_sensitivity/learning_rate/
        // immune_strength from methylation, but phenotype.anxiety is a genetic trait --
        // no engine outside genome.rs may write it.
        let mut ind = stressed_individual();
        initialize_epigenome(&mut ind);
        let original_anxiety = ind.phenotype.anxiety;
        ind.psychology = Psychology { stress_level: 0.9, ..Default::default() };
        for day in 0..1000 {
            update_epigenome(&mut ind, None, day);
        }
        assert_eq!(ind.phenotype.anxiety, original_anxiety);
    }

    #[test]
    fn update_epigenome_initializes_epigenome_and_does_not_panic() {
        let mut ind = Individual::default();
        update_epigenome(&mut ind, None, 1);
        assert!(ind.epigenome.contains_key("HPA_AXIS"));
    }

    #[test]
    fn high_stress_raises_hpa_axis_methylation() {
        let mut ind = stressed_individual();
        initialize_epigenome(&mut ind);
        let before = ind.epigenome["HPA_AXIS"].methylation;
        for day in 0..100 {
            update_epigenome(&mut ind, None, day);
        }
        assert!(ind.epigenome["HPA_AXIS"].methylation > before);
    }

    #[test]
    fn low_stress_lowers_hpa_axis_methylation() {
        let mut ind = stressed_individual();
        ind.psychology = Psychology { stress_level: 0.1, ..Default::default() };
        initialize_epigenome(&mut ind);
        ind.epigenome.get_mut("HPA_AXIS").unwrap().methylation = 0.8;
        for day in 0..100 {
            update_epigenome(&mut ind, None, day);
        }
        assert!(ind.epigenome["HPA_AXIS"].methylation < 0.8);
    }

    #[test]
    fn starvation_raises_leptin_resist_methylation() {
        let mut ind = stressed_individual();
        ind.extra.insert("satiation".to_string(), serde_json::json!(0.1));
        initialize_epigenome(&mut ind);
        let before = ind.epigenome["LEPTIN_RESIST"].methylation;
        for day in 0..100 {
            update_epigenome(&mut ind, None, day);
        }
        assert!(ind.epigenome["LEPTIN_RESIST"].methylation > before);
    }

    #[test]
    fn social_isolation_raises_oxtr_methyl_methylation() {
        let mut ind = stressed_individual();
        ind.group_id = None;
        initialize_epigenome(&mut ind);
        let before = ind.epigenome["OXTR_METHYL"].methylation;
        for day in 0..100 {
            update_epigenome(&mut ind, None, day);
        }
        assert!(ind.epigenome["OXTR_METHYL"].methylation > before);
    }

    #[test]
    fn h02_regression_maoa_regulation_keeps_increasing_under_repeated_early_childhood_stress() {
        // Before H-02 fix: locked after first write, subsequent stress had no effect.
        // After fix: irreversible only blocks a *negative* delta -- positive keeps accumulating.
        let mut ind = Individual {
            age_days: Some(2 * 365),
            psychology: Psychology { stress_level: 0.9, ..Default::default() },
            ..Default::default()
        };
        initialize_epigenome(&mut ind);
        update_epigenome(&mut ind, None, 1);
        let after_first = ind.epigenome["MAOA_REGULATION"].methylation;
        for day in 2..200 {
            update_epigenome(&mut ind, None, day);
        }
        assert!(ind.epigenome["MAOA_REGULATION"].methylation > after_first);
    }

    #[test]
    fn h02_regression_maoa_regulation_cannot_decrease() {
        let mut ind = Individual {
            age_days: Some(2 * 365),
            psychology: Psychology { stress_level: 0.1, ..Default::default() },
            ..Default::default()
        };
        initialize_epigenome(&mut ind);
        ind.epigenome.get_mut("MAOA_REGULATION").unwrap().methylation = 0.9;
        for day in 1..200 {
            update_epigenome(&mut ind, None, day);
        }
        assert!(ind.epigenome["MAOA_REGULATION"].methylation >= 0.9);
    }

    #[test]
    fn bug01_fix_immune_priming_rises_while_infected() {
        // reversible=false only blocks negative deltas -- infection applies +0.02/day,
        // so methylation must rise toward 1 with each infected day.
        let mut ind = Individual::default();
        ind.extra.insert("infections".to_string(), serde_json::json!([{"pathogen_id": "pathogen-test"}]));
        initialize_epigenome(&mut ind);
        ind.epigenome.get_mut("IMMUNE_PRIMING").unwrap().methylation = 0.5;
        for day in 1..100 {
            update_epigenome(&mut ind, None, day);
        }
        assert!(ind.epigenome["IMMUNE_PRIMING"].methylation > 0.5);
    }

    #[test]
    fn immune_priming_stays_capped_at_one_after_many_infected_days() {
        let mut ind = Individual::default();
        ind.extra.insert("infections".to_string(), serde_json::json!([{"pathogen_id": "pathogen-test"}]));
        initialize_epigenome(&mut ind);
        for day in 1..=1000 {
            update_epigenome(&mut ind, None, day);
        }
        assert!(ind.epigenome["IMMUNE_PRIMING"].methylation <= 1.0);
    }

    #[test]
    fn uninfected_individual_does_not_gain_immune_priming_methylation() {
        let mut ind = Individual::default();
        initialize_epigenome(&mut ind);
        ind.epigenome.get_mut("IMMUNE_PRIMING").unwrap().methylation = 0.5;
        for day in 1..=50 {
            update_epigenome(&mut ind, None, day);
        }
        assert_eq!(ind.epigenome["IMMUNE_PRIMING"].methylation, 0.5);
    }

    #[test]
    fn methylation_values_stay_within_zero_one_under_extreme_conditions() {
        let mut ind = Individual {
            psychology: Psychology { stress_level: 1.0, ..Default::default() },
            ..Default::default()
        };
        ind.extra.insert("satiation".to_string(), serde_json::json!(0.0));
        initialize_epigenome(&mut ind);
        for day in 0..500 {
            update_epigenome(&mut ind, None, day);
        }
        for locus in ind.epigenome.values() {
            assert!((0.0..=1.0).contains(&locus.methylation));
        }
    }

    #[test]
    fn a_collective_disaster_leaves_a_stronger_hpa_axis_imprint_than_ordinary_stress() {
        let mut collective = stressed_individual();
        collective.psychology.trauma_events.push(serde_json::json!({ "type": "flood", "day": 5 }));
        let before = collective.epigenome.get("HPA_AXIS").map(|l| l.methylation).unwrap_or(0.5);
        update_epigenome(&mut collective, None, 5);
        let after_collective = collective.epigenome["HPA_AXIS"].methylation;

        let mut ordinary = stressed_individual();
        update_epigenome(&mut ordinary, None, 5);
        let after_ordinary = ordinary.epigenome["HPA_AXIS"].methylation;

        assert!(after_collective > after_ordinary, "collective disaster ({after_collective}) should raise HPA_AXIS more than ordinary stress ({after_ordinary})");
        let _ = before;
    }

    #[test]
    fn a_private_kin_death_does_not_count_as_collective_trauma() {
        let mut ind = stressed_individual();
        ind.psychology.trauma_events.push(serde_json::json!({ "type": "kin_death", "day": 5 }));
        update_epigenome(&mut ind, None, 5);
        let mut ordinary = stressed_individual();
        update_epigenome(&mut ordinary, None, 5);
        assert_eq!(ind.epigenome["HPA_AXIS"].methylation, ordinary.epigenome["HPA_AXIS"].methylation);
    }

    #[test]
    fn inherit_epigenome_blends_parents_weighted_by_heritability() {
        let mut p1 = Individual::default();
        let mut p2 = Individual::default();
        initialize_epigenome(&mut p1);
        initialize_epigenome(&mut p2);
        p1.epigenome.get_mut("HPA_AXIS").unwrap().methylation = 0.9;
        p2.epigenome.get_mut("HPA_AXIS").unwrap().methylation = 0.9;
        let mut child = Individual::default();
        inherit_epigenome(&mut child, &mut p1, &mut p2);
        // heritability(HPA_AXIS) = 0.3, so a child of two 0.9 parents should land
        // partway between the population baseline (0.5) and the parental average,
        // never inheriting the full parental value directly.
        let child_hpa = child.epigenome["HPA_AXIS"].methylation;
        assert!(child_hpa > 0.5 && child_hpa < 0.9, "expected partial heritability, got {child_hpa}");
        let expected = 0.5 + (0.9 - 0.5) * 0.3;
        assert!((child_hpa - expected).abs() < 1e-9);
    }
}
