use crate::spatial::SpatialGrid;
use crate::state::Individual;
use rand::Rng;

use super::genome::{coefficient_of_relationship, GenealogyIndex};
use super::individual::{create_child, get_age, is_fertile};

const PREGNANCY_MIN: i32 = 266;
const MATING_RADIUS: f64 = 2.0;
// Mirrors tick.rs's own MAX_CANDIDATE_SCAN: SpatialGrid::candidates_within is
// coarse and, without a cap, costs O(cell population) per query -- a
// crowded settlement's mating search would otherwise scale with local
// density squared (every fertile female scanning every fertile male sharing
// her cell block), the exact pattern already guarded against for movement
// and observation-learning.
const MAX_MATE_CANDIDATE_SCAN: usize = 50;

// Takes references rather than owned Individuals: the caller (tick.rs) used
// to build this as a full `.cloned()` of every alive individual on every
// single tick, which per-section profiling found to be the single most
// expensive part of the whole tick once population grew into the hundreds --
// genome/epigenome/inventory/skills/beliefs/language.vocabulary/memory all
// paid for regardless of whether reproduction ever looks at them for most
// candidates. This function is read-only over `population` (only
// create_child, called on the one selected pair, needs their full data,
// which a reference already provides), so there was never a need to own
// clones of the whole scanned population just to filter/pair over it.
pub fn check_reproduction(
    population: &[&Individual],
    current_day: i32,
    simulation_id: &str,
    community_lang_stage: i32,
    genealogy: &GenealogyIndex,
) -> Vec<Individual> {
    let mut newborns = Vec::new();
    let fertile_males: Vec<&Individual> = population
        .iter()
        .copied()
        .filter(|i| i.alive && i.sex == "male" && is_fertile(i, current_day))
        .collect();
    let male_positions: Vec<(f64, f64)> = fertile_males.iter().map(|m| (m.x, m.y)).collect();
    let male_grid = SpatialGrid::build(&male_positions, MATING_RADIUS);

    for female in population.iter().copied().filter(|i| i.alive && i.sex == "female" && is_fertile(i, current_day)) {
        if female.health.pregnancy.is_some() {
            continue;
        }
        let nearby_males: Vec<&Individual> = male_grid
            .candidates_within(female.x, female.y, MATING_RADIUS, MAX_MATE_CANDIDATE_SCAN)
            .into_iter()
            .filter_map(|idx| fertile_males.get(idx).copied())
            .filter(|male| distance(male, female) < MATING_RADIUS)
            .collect();
        if nearby_males.is_empty() {
            continue;
        }
        let male = nearby_males[rand::thread_rng().gen_range(0..nearby_males.len())];
        let p = conception_probability(female, male, current_day, community_lang_stage, genealogy);
        if rand::random::<f64>() < p {
            let due_day = current_day + PREGNANCY_MIN + rand::thread_rng().gen_range(0..14);
            let child = create_child(female, male, due_day, simulation_id);
            newborns.push(child);
        }
    }

    newborns
}

/// Average of both alleles at a locus, defaulting to 0.5 (population baseline)
/// when the locus or an allele value is missing.
fn locus_average(genome: &crate::types::Genome, locus_id: &str) -> f64 {
    genome.get(locus_id).map(|l| (l.allele1.value.unwrap_or(0.5) + l.allele2.value.unwrap_or(0.5)) / 2.0).unwrap_or(0.5)
}

/// `community_lang_stage` is the highest language stage (0-6) any currently
/// living member of the population has reached -- used here to model
/// demographic transition: real-world societies with more developed
/// language/culture/technology consistently show *lower* fertility, so a
/// more linguistically advanced community mutes conception odds somewhat
/// rather than being ignored (this value used to be computed every tick in
/// tick.rs and then immediately discarded here).
fn conception_probability(female: &Individual, male: &Individual, current_day: i32, community_lang_stage: i32, genealogy: &GenealogyIndex) -> f64 {
    let age = get_age(female, current_day);
    let mut age_factor = 1.0;
    if age > 40.0 {
        age_factor = 0.2;
    } else if age > 35.0 {
        age_factor = 0.6;
    } else if age < 18.0 {
        age_factor = 0.3;
    } else if age < 20.0 {
        age_factor = 0.7;
    }
    let urge_factor = 0.6 + female.extra.get("mating_urge").and_then(|v| v.as_f64()).unwrap_or(0.5) * 0.4;
    let fertility = female.phenotype.fertility;
    // The prospective *pair's* relatedness -- what F their child would have --
    // not each parent's own historical inbreeding_coeff (which reflects their
    // OWN parents' relatedness, not each other's). Using each individual's own
    // coefficient here let full-sibling and parent-child pairs through with
    // zero penalty even though their child would be F=0.25, because two
    // non-inbred founders' children both carry inbreeding_coeff=0.0 despite
    // being full siblings of each other.
    let inbreed_penalty = coefficient_of_relationship(&female.id, &male.id, genealogy);
    // MHC (immune-locus) diversity bonus: more genetically dissimilar immune
    // alleles between the pair raise conception odds, mirroring real MHC-based
    // mate selection biology.
    let f_i1 = locus_average(&female.genome, "IMMUNE_01");
    let f_i2 = locus_average(&female.genome, "IMMUNE_02");
    let m_i1 = locus_average(&male.genome, "IMMUNE_01");
    let m_i2 = locus_average(&male.genome, "IMMUNE_02");
    let mhc_bonus = ((f_i1 - m_i1).abs() + (f_i2 - m_i2).abs()) / 2.0 * 0.2;
    // Demographic transition: at most a 30% reduction, reached only once the
    // community as a whole has reached writing (stage 6) -- a bounded, gradual
    // pull rather than something that can stall population growth on its own.
    let demographic_transition = 1.0 - (community_lang_stage.clamp(0, 6) as f64 / 6.0) * 0.3;
    ((fertility * age_factor + mhc_bonus - inbreed_penalty * 0.5) * 0.09 * urge_factor * demographic_transition).clamp(0.0, 1.0)
}

/// Daily accumulation of an individual's own mating drive, purely from their
/// own age/health/season/fertility/stress signals -- read by agent::select_action
/// (behavioral "mate" scoring) and conception_probability (urge_factor) above.
/// Was previously never called from anywhere, so mating_urge silently stayed at
/// its zero default forever and the "mate" action could never be behaviorally
/// selected (conception itself still worked via check_reproduction, which is
/// independent of the chosen action).
pub fn update_mating_urge(individual: &mut Individual, world_state: &serde_json::Value) {
    use serde_json::{json, Value};

    if individual.is_dead {
        return;
    }
    let age_years = individual.age_days.unwrap_or(0) as f64 / 365.0;
    // Deliberately wider than either sex's own is_fertile() window (female
    // 15-50, male 15-65, individual.rs) -- behavioral mate-seeking urge is
    // allowed to persist past the age conception itself becomes possible
    // (check_reproduction independently gates on is_fertile, so this never
    // lets a post-fertile individual actually conceive), the same way human
    // libido doesn't switch off the instant fertility does. MATING_URGE_MAX_AGE_YEARS
    // just needs to comfortably exceed the highest of the two is_fertile
    // caps; it isn't meant to equal either one.
    const MATING_URGE_MAX_AGE_YEARS: f64 = 72.0;
    if !(15.0..=MATING_URGE_MAX_AGE_YEARS).contains(&age_years) {
        individual.extra.insert("mating_urge".to_string(), json!(0.0));
        return;
    }
    if individual.health.pregnancy.is_some() {
        let current = individual.extra.get("mating_urge").and_then(Value::as_f64).unwrap_or(0.0);
        individual.extra.insert("mating_urge".to_string(), json!((current + 0.001).min(0.5)));
        return;
    }

    let mut urge = individual.extra.get("mating_urge").and_then(Value::as_f64).unwrap_or_else(|| rand::random::<f64>() * 0.4);

    let mut rate: f64 = 0.006; // reaches 1 in ~170 days

    if age_years < 18.0 {
        rate *= 0.55;
    } else if age_years < 35.0 {
        rate *= 1.2;
    } else if age_years > 65.0 {
        rate *= 0.25;
    } else if age_years > 55.0 {
        rate *= 0.55;
    }

    let hp = individual.health.hp;
    let calories = individual.health.calories;
    if hp < 0.35 || calories < 0.25 {
        rate *= 0.15;
    } else if hp > 0.7 && calories > 0.6 {
        rate *= 1.1;
    }

    let season = world_state.get("season").and_then(Value::as_str).unwrap_or("spring");
    if season == "spring" || season == "summer" {
        rate *= 1.15;
    }

    rate *= 0.65 + individual.phenotype.fertility * 0.7;

    if individual.psychology.stress_level > 0.7 {
        rate *= 0.4;
    }

    urge = (urge + rate).min(1.0);
    individual.extra.insert("mating_urge".to_string(), json!(urge));
}

fn distance(a: &Individual, b: &Individual) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::individual::create_founder;

    fn founder_at(sex: &str, x: f64) -> Individual {
        create_founder(&serde_json::json!({ "sex": sex, "ageYears": 25, "x": x, "y": 0 }))
    }

    fn empty_genealogy() -> GenealogyIndex {
        GenealogyIndex::new()
    }

    #[test]
    fn a_couple_in_close_proximity_eventually_conceives() {
        let male = founder_at("male", 0.0);
        let female = founder_at("female", 0.0);
        let genealogy = empty_genealogy();
        let mut conceived = false;
        for day in 0..1000 {
            let newborns = check_reproduction(&[&male, &female], day, "sim1", 0, &genealogy);
            if !newborns.is_empty() {
                conceived = true;
                break;
            }
        }
        assert!(conceived, "a close, fertile couple should eventually conceive within 1000 days");
    }

    #[test]
    fn an_already_pregnant_female_is_skipped_entirely() {
        let male = founder_at("male", 0.0);
        let mut female = founder_at("female", 0.0);
        female.health.pregnancy = Some(0);
        let genealogy = empty_genealogy();
        for day in 0..200 {
            let newborns = check_reproduction(&[&male, &female], day, "sim1", 0, &genealogy);
            assert!(newborns.is_empty(), "a pregnant female must never conceive again");
        }
    }

    #[test]
    fn a_male_beyond_mating_radius_never_produces_a_conception() {
        let male = founder_at("male", 10.0); // > MATING_RADIUS (2.0) away
        let female = founder_at("female", 0.0);
        let genealogy = empty_genealogy();
        for day in 0..500 {
            let newborns = check_reproduction(&[&male, &female], day, "sim1", 0, &genealogy);
            assert!(newborns.is_empty());
        }
    }

    #[test]
    fn an_infertile_young_female_never_conceives() {
        let male = founder_at("male", 0.0);
        let mut young_female = create_founder(&serde_json::json!({ "sex": "female", "ageYears": 10, "x": 0, "y": 0 }));
        young_female.x = 0.0;
        let genealogy = empty_genealogy();
        for day in 0..500 {
            let newborns = check_reproduction(&[&male, &young_female], day, "sim1", 0, &genealogy);
            assert!(newborns.is_empty());
        }
    }

    #[test]
    fn genetically_dissimilar_immune_alleles_raise_conception_probability() {
        // Same two individuals throughout -- only the immune loci change --
        // so fertility/age/urge (which are otherwise random per founder) can't
        // swamp the MHC signal being tested here.
        let mut female = founder_at("female", 0.0);
        let mut male = founder_at("male", 0.0);
        let genealogy = empty_genealogy();

        for locus in ["IMMUNE_01", "IMMUNE_02"] {
            female.genome.get_mut(locus).unwrap().allele1.value = Some(0.5);
            female.genome.get_mut(locus).unwrap().allele2.value = Some(0.5);
            male.genome.get_mut(locus).unwrap().allele1.value = Some(0.5);
            male.genome.get_mut(locus).unwrap().allele2.value = Some(0.5);
        }
        let similar_p = conception_probability(&female, &male, 0, 0, &genealogy);

        for locus in ["IMMUNE_01", "IMMUNE_02"] {
            female.genome.get_mut(locus).unwrap().allele1.value = Some(0.0);
            female.genome.get_mut(locus).unwrap().allele2.value = Some(0.0);
            male.genome.get_mut(locus).unwrap().allele1.value = Some(1.0);
            male.genome.get_mut(locus).unwrap().allele2.value = Some(1.0);
        }
        let diverse_p = conception_probability(&female, &male, 0, 0, &genealogy);

        assert!(diverse_p > similar_p);
    }

    #[test]
    fn a_more_linguistically_advanced_community_has_lower_conception_probability() {
        // Demographic transition: same pair, only community_lang_stage differs.
        let female = founder_at("female", 0.0);
        let male = founder_at("male", 0.0);
        let genealogy = empty_genealogy();
        let preliterate_p = conception_probability(&female, &male, 0, 0, &genealogy);
        let writing_p = conception_probability(&female, &male, 0, 6, &genealogy);
        assert!(writing_p < preliterate_p, "a writing-stage (6) community should have lower conception odds than a pre-linguistic (0) one");
        // Bounded to at most a 30% reduction -- language development nudges
        // fertility, it doesn't gate reproduction outright.
        assert!(writing_p >= preliterate_p * 0.7 - 1e-9);
    }

    #[test]
    fn community_lang_stage_beyond_six_is_clamped_not_extrapolated() {
        let female = founder_at("female", 0.0);
        let male = founder_at("male", 0.0);
        let genealogy = empty_genealogy();
        assert_eq!(
            conception_probability(&female, &male, 0, 6, &genealogy),
            conception_probability(&female, &male, 0, 99, &genealogy)
        );
    }

    // ── prospective-pair inbreeding penalty (H-08 regression) ───────────

    #[test]
    fn full_siblings_incur_a_conception_penalty_even_though_neither_parents_own_inbreeding_coeff_reflects_it() {
        // Two founders (each with inbreeding_coeff 0.0, since neither has
        // parents of their own) have two children -- full siblings of each
        // other. Each child's OWN inbreeding_coeff is also 0.0 (their
        // parents, the founders, aren't related), but the coefficient of
        // relationship BETWEEN the two siblings is 0.25 -- exactly what their
        // own child would inherit as F if they mated. conception_probability
        // must catch this from the genealogy index, not from either
        // sibling's own (irrelevant) inbreeding_coeff field.
        use super::super::genome::GenealogyEntry;

        let father = founder_at("male", 0.0);
        let mother = founder_at("female", 0.0);
        let mut sibling_a = founder_at("male", 0.0);
        let mut sibling_b = founder_at("female", 0.0);
        sibling_a.id = "sib-a".to_string();
        sibling_b.id = "sib-b".to_string();
        sibling_a.parent_1_id = Some(father.id.clone());
        sibling_a.parent_2_id = Some(mother.id.clone());
        sibling_b.parent_1_id = Some(father.id.clone());
        sibling_b.parent_2_id = Some(mother.id.clone());
        assert_eq!(sibling_a.inbreeding_coeff, Some(0.0));
        assert_eq!(sibling_b.inbreeding_coeff, Some(0.0));

        let mut genealogy = GenealogyIndex::new();
        genealogy.insert(father.id.clone(), GenealogyEntry { parent_1_id: None, parent_2_id: None, inbreeding_coeff: 0.0 });
        genealogy.insert(mother.id.clone(), GenealogyEntry { parent_1_id: None, parent_2_id: None, inbreeding_coeff: 0.0 });
        genealogy.insert(
            sibling_a.id.clone(),
            GenealogyEntry { parent_1_id: sibling_a.parent_1_id.clone(), parent_2_id: sibling_a.parent_2_id.clone(), inbreeding_coeff: 0.0 },
        );
        genealogy.insert(
            sibling_b.id.clone(),
            GenealogyEntry { parent_1_id: sibling_b.parent_1_id.clone(), parent_2_id: sibling_b.parent_2_id.clone(), inbreeding_coeff: 0.0 },
        );

        // Same exact pair, same exact (randomly generated, so otherwise
        // confounding) genomes/fertility/MHC-similarity/age throughout --
        // only the genealogy index passed in differs. This isolates the
        // inbreeding-penalty signal instead of comparing against a
        // separately-generated "unrelated" individual, whose independently
        // random fertility/MHC alleles could otherwise swamp the effect
        // being tested here (and did, intermittently, before this rewrite).
        let unrelated_p = conception_probability(&sibling_b, &sibling_a, 0, 0, &empty_genealogy());
        let sibling_p = conception_probability(&sibling_b, &sibling_a, 0, 0, &genealogy);
        assert!(sibling_p < unrelated_p, "full siblings should have a lower conception probability than the same pair evaluated as unrelated");
    }

    // ── twin-rate formula (pure math, mirrors the JS constants) ─────────

    #[test]
    fn twin_chance_for_average_fertility_is_about_1_7_percent() {
        let fshr = 0.5;
        let twin_chance = (0.003 + (fshr - 0.3) * 0.07f64).max(0.0);
        assert!((twin_chance - 0.017).abs() < 1e-3);
    }

    #[test]
    fn twin_chance_increases_with_higher_fertility() {
        let low = (0.003 + (0.3 - 0.3) * 0.07f64).max(0.0);
        let high = (0.003 + (1.0 - 0.3) * 0.07f64).max(0.0);
        assert!(high > low);
    }

    #[test]
    fn twin_chance_for_low_fertility_stays_at_or_near_the_floor() {
        let chance = (0.003 + (0.1 - 0.3) * 0.07f64).max(0.0);
        assert!((0.0..0.003).contains(&chance));
    }

    // ── H-07 regression — update_mating_urge age alignment ─────────────

    #[test]
    fn a_fourteen_year_old_has_mating_urge_reset_to_zero() {
        let mut ind = Individual {
            age_days: Some(14 * 365),
            extra: { let mut m = serde_json::Map::new(); m.insert("mating_urge".to_string(), serde_json::json!(0.9)); m },
            ..Default::default()
        };
        update_mating_urge(&mut ind, &serde_json::json!({}));
        assert_eq!(ind.extra["mating_urge"], 0.0);
    }

    #[test]
    fn a_fifteen_year_old_can_build_mating_urge() {
        let mut ind = Individual {
            age_days: Some(15 * 365 + 1),
            health: crate::types::Health { hp: 1.0, calories: 1.0, hydration: 1.0, ..Default::default() },
            extra: { let mut m = serde_json::Map::new(); m.insert("mating_urge".to_string(), serde_json::json!(0.5)); m },
            ..Default::default()
        };
        update_mating_urge(&mut ind, &serde_json::json!({}));
        assert!(ind.extra["mating_urge"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn pregnancy_caps_mating_urge_at_zero_point_five_but_still_lets_it_build() {
        let mut ind = Individual {
            age_days: Some(25 * 365),
            health: crate::types::Health { pregnancy: Some(0), ..Default::default() },
            extra: { let mut m = serde_json::Map::new(); m.insert("mating_urge".to_string(), serde_json::json!(0.499)); m },
            ..Default::default()
        };
        update_mating_urge(&mut ind, &serde_json::json!({}));
        let urge = ind.extra["mating_urge"].as_f64().unwrap();
        assert!(urge > 0.499 && urge <= 0.5 + 1e-9);
    }

    #[test]
    fn urge_never_exceeds_one() {
        let mut ind = Individual {
            age_days: Some(25 * 365),
            health: crate::types::Health { hp: 1.0, calories: 1.0, hydration: 1.0, ..Default::default() },
            phenotype: crate::types::Phenotype { fertility: 1.0, ..Default::default() },
            extra: { let mut m = serde_json::Map::new(); m.insert("mating_urge".to_string(), serde_json::json!(0.999)); m },
            ..Default::default()
        };
        for _ in 0..50 {
            update_mating_urge(&mut ind, &serde_json::json!({ "season": "summer" }));
        }
        assert!(ind.extra["mating_urge"].as_f64().unwrap() <= 1.0 + 1e-9);
    }

    #[test]
    fn a_dead_individual_is_never_touched() {
        let mut ind = Individual { is_dead: true, age_days: Some(25 * 365), ..Default::default() };
        update_mating_urge(&mut ind, &serde_json::json!({}));
        assert!(ind.extra.get("mating_urge").is_none());
    }
}
