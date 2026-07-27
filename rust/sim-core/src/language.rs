use serde_json::{json, Value};

use crate::state::Individual;
use crate::types::{Genome, PhonemePalette};

/// The full set of consonant/vowel articulations available to any human
/// vocal tract -- a biological constant, the same for every simulation
/// (comparable to the fixed amino-acid alphabet the genome engine draws on).
/// This is NOT a specific language; it is the physical possibility space a
/// specific population's actual phoneme palette (`derive_phoneme_palette`)
/// is drawn from.
const CONSONANT_SUPERSET: &[char] = &['b', 'c', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 'n', 'p', 'q', 'r', 's', 't', 'v', 'w', 'x', 'y', 'z'];
/// The five cardinal vowel qualities found in nearly every human language --
/// likewise a physiological constant, not an authored choice.
const VOWEL_SUPERSET: &[char] = &['a', 'e', 'i', 'o', 'u'];

fn allele_sum(genome: &Genome, locus: &str) -> f64 {
    genome.get(locus).map(|l| l.allele1.value.unwrap_or(0.5) + l.allele2.value.unwrap_or(0.5)).unwrap_or(1.0)
}

/// Deterministic partial Fisher-Yates: picks `count` distinct entries from
/// `superset`, ordered by `seed`. Used to select which slice of the
/// articulatory superset a given population actually has access to.
fn pick_subset(superset: &[char], count: usize, seed: u64) -> Vec<char> {
    let count = count.clamp(1, superset.len());
    let mut indices: Vec<usize> = (0..superset.len()).collect();
    let mut s = seed | 1;
    for i in (1..indices.len()).rev() {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (s >> 33) as usize % (i + 1);
        indices.swap(i, j);
    }
    indices.truncate(count);
    indices.sort_unstable();
    indices.into_iter().map(|i| superset[i]).collect()
}

/// Derives this population's phoneme palette from the two founders' own
/// FOXP2_01/CNTNAP2_01 alleles -- their genetic articulatory precision sets
/// how large a sound repertoire their lineage can produce, and the literal
/// allele values (not their ids or any authored content) seed *which* slice
/// of the universal superset that repertoire draws from. Two simulations
/// with different founder genomes get different, non-authored palettes;
/// the same founders always reproduce the same palette.
pub fn derive_phoneme_palette(founder1_genome: &Genome, founder2_genome: &Genome) -> PhonemePalette {
    let precision = (allele_sum(founder1_genome, "FOXP2_01")
        + allele_sum(founder1_genome, "CNTNAP2_01")
        + allele_sum(founder2_genome, "FOXP2_01")
        + allele_sum(founder2_genome, "CNTNAP2_01"))
        / 8.0;
    let precision = precision.clamp(0.0, 1.0);
    let seed_str = format!(
        "{:.6}|{:.6}|{:.6}|{:.6}",
        allele_sum(founder1_genome, "FOXP2_01"),
        allele_sum(founder1_genome, "CNTNAP2_01"),
        allele_sum(founder2_genome, "FOXP2_01"),
        allele_sum(founder2_genome, "CNTNAP2_01"),
    );
    let seed = hash_str(&seed_str) as u64;
    let consonant_count = 6 + (precision * (CONSONANT_SUPERSET.len() - 6) as f64).round() as usize;
    let vowel_count = 3 + (precision * (VOWEL_SUPERSET.len() - 3) as f64).round() as usize;
    PhonemePalette {
        consonants: pick_subset(CONSONANT_SUPERSET, consonant_count, seed),
        vowels: pick_subset(VOWEL_SUPERSET, vowel_count, seed.wrapping_add(0x9E3779B97F4A7C15)),
    }
}

/// Same as `derive_phoneme_palette`, but reads the founders straight out of
/// a population (used by `tick::advance_one_day` to self-heal simulations
/// saved before this field existed). Falls back to an empty palette --
/// meaning no word or name can be produced yet -- if no founder is present.
pub fn derive_phoneme_palette_from_population(population: &[Individual]) -> PhonemePalette {
    let founders: Vec<&Individual> = population.iter().filter(|i| i.is_founder).take(2).collect();
    match founders.as_slice() {
        [f1, f2] => derive_phoneme_palette(&f1.genome, &f2.genome),
        [f1] => derive_phoneme_palette(&f1.genome, &f1.genome),
        _ => PhonemePalette::default(),
    }
}

/// Test/reference palette spanning the full articulatory superset -- used
/// where a test wants every generation attempt to succeed rather than
/// exercising one specific population's (possibly narrow) derived palette.
#[cfg(test)]
pub(crate) fn full_palette() -> PhonemePalette {
    PhonemePalette { consonants: CONSONANT_SUPERSET.to_vec(), vowels: VOWEL_SUPERSET.to_vec() }
}

pub const LANGUAGE_STAGES: &[(&str, f64, usize, i32)] = &[
    ("pre-linguistic", 0.0, 1, 0),
    ("gestural", 0.0, 3, 0),
    ("emotional-sounds", 0.4, 5, 1),
    ("proto-words", 0.55, 8, 4),
    ("syntax", 0.65, 15, 8),
    ("abstract", 0.72, 25, 15),
    ("writing", 0.80, 40, 25),
];

pub fn update_language_stage(individual: &mut Individual, group_size: usize, generation_count: i32) -> Value {
    let foxp2 = if individual.language.foxp2_expression > 0.0 {
        individual.language.foxp2_expression
    } else {
        individual.phenotype.language_capacity * 0.1
    };
    let current_stage = individual.language.stage.max(0) as usize;
    for (idx, (name, foxp2_min, group_min, gen_min)) in LANGUAGE_STAGES.iter().enumerate().rev() {
        if foxp2 >= *foxp2_min && group_size >= *group_min && generation_count >= *gen_min {
            if idx > current_stage {
                let next_stage = current_stage + 1;
                let next_name = LANGUAGE_STAGES[next_stage].0;
                individual.language.stage = next_stage as i32;
                individual.language.stage_name = next_name.to_string();
                if next_stage >= 4 {
                    individual.language.grammar = true;
                }
                if next_stage >= 6 {
                    individual.language.writing = true;
                }
                return json!({ "upgraded": true, "prevStage": current_stage, "newStage": next_stage, "stageName": next_name });
            }
            let _ = name;
            break;
        }
    }
    json!({ "upgraded": false })
}

pub fn update_foxp2_expression(individual: &mut Individual, group_member_count: usize) {
    let cap = individual.phenotype.language_capacity;
    let current = if individual.language.foxp2_expression > 0.0 { individual.language.foxp2_expression } else { cap * 0.1 };
    let social_gain = group_member_count.min(10) as f64 * 0.000015;
    let staging_gain = if individual.language.stage > 0 { 0.000005 } else { 0.0 };
    individual.language.foxp2_expression = (current + social_gain + staging_gain).min(cap);
}

pub fn try_acquire_word_from_environment(individual: &mut Individual, concept: &str, group_id: &str, palette: &PhonemePalette) -> bool {
    if individual.language.stage < 2 {
        return false;
    }
    if individual.language.vocabulary.contains_key(concept) {
        return false;
    }
    let foxp2 = individual.language.foxp2_expression;
    if foxp2 < 0.35 {
        return false;
    }
    let iq = individual.phenotype.fluid_intelligence;
    if rand::random::<f64>() > foxp2 * iq * 0.15 {
        return false;
    }
    let word = generate_proto_word(concept, group_id, palette);
    if word.is_empty() {
        return false;
    }
    individual.language.vocabulary.insert(concept.to_string(), word);
    true
}

pub fn learn_from_teacher(learner: &mut Individual, teacher: &Individual) {
    if teacher.language.vocabulary.is_empty() {
        return;
    }
    let foxp2 = learner.language.foxp2_expression;
    if foxp2 < 0.25 {
        return;
    }
    let max_learn = (learner.phenotype.fluid_intelligence * 3.0).floor() as usize;
    for (idx, (word, value)) in teacher.language.vocabulary.iter().enumerate() {
        if idx >= max_learn {
            break;
        }
        learner.language.vocabulary.entry(word.clone()).or_insert_with(|| value.clone());
    }
}

pub fn generate_proto_word(concept: &str, group_id: &str, palette: &PhonemePalette) -> String {
    // u64 throughout: the JS original relies on Int32 wraparound for its hash,
    // which Rust's checked-by-default i32/u32 arithmetic would panic on in
    // debug builds. Widening avoids overflow entirely without changing the
    // "deterministic per (concept, group_id)" contract callers rely on.
    let seed = hash_str(&(concept.to_string() + group_id)) as u64;
    let c = &palette.consonants;
    let v = &palette.vowels;
    if c.is_empty() || v.is_empty() {
        // This population has no sounds to draw on yet -- no word exists,
        // not even a fallback one.
        return String::new();
    }
    let len = 1 + (seed % 3) as usize;
    let mut word = String::new();
    for i in 0..len {
        word.push(c[(seed.wrapping_mul(i as u64 + 1).wrapping_mul(7) % c.len() as u64) as usize]);
        word.push(v[(seed.wrapping_mul(i as u64 + 1).wrapping_mul(13) % v.len() as u64) as usize]);
    }
    word
}

fn hash_str(str_: &str) -> u32 {
    let mut h: i32 = 0;
    for ch in str_.chars() {
        h = (h.wrapping_shl(5)).wrapping_sub(h).wrapping_add(ch as i32);
    }
    h.unsigned_abs()
}

pub const CORE_CONCEPTS: &[&str] = &[
    "danger","food","water","fire","here","there","me","you","us","them","good","bad",
    "hunt","eat","sleep","die","born","run","sun","moon","rain","dark","light","god",
    "spirit","sky","earth","time",
];

/// Per-group vocabulary snapshot (concept -> word), built from whichever
/// living member of each group happens to know the most words for that
/// concept's group -- a read-only surface of the dialect divergence that
/// `generate_proto_word`'s own group_id-seeded hashing already produces
/// (different groups deterministically coin different words for the same
/// concept), so a player can actually see two bands' words for "fire"
/// diverge after a fission, rather than that divergence being invisible
/// engine-internal state.
pub fn get_vocabulary_by_group(population: &[Individual]) -> Value {
    let mut by_group: std::collections::HashMap<&str, std::collections::HashMap<&str, &str>> = std::collections::HashMap::new();
    for ind in population {
        if !ind.alive {
            continue;
        }
        let Some(gid) = ind.group_id.as_deref() else { continue };
        let entry = by_group.entry(gid).or_default();
        for (concept, word) in ind.language.vocabulary.iter() {
            entry.entry(concept.as_str()).or_insert(word.as_str());
        }
    }
    by_group
        .into_iter()
        .map(|(gid, vocab)| (gid.to_string(), vocab.into_iter().map(|(c, w)| (c.to_string(), json!(w))).collect::<serde_json::Map<String, Value>>().into()))
        .collect::<serde_json::Map<String, Value>>()
        .into()
}

const MAX_WRITTEN_RECORDS: usize = 50;

/// Once an individual has reached the writing stage, a notable event of the
/// day can be committed to a permanent, bounded record in their own memory --
/// extending observational learning across *time*, not just across
/// individuals: a group member who reads this record later (see
/// `read_written_records`) can know about an event they never personally
/// witnessed, exactly the way a real written record works.
pub fn record_event_for_posterity(individual: &mut Individual, event: &Value, sim_day: i32) {
    if !individual.language.writing {
        return;
    }
    if !individual.memory.is_object() {
        individual.memory = json!({});
    }
    let summary = event.get("type").and_then(Value::as_str).or_else(|| event.get("description").and_then(Value::as_str)).unwrap_or("event").to_string();
    let obj = individual.memory.as_object_mut().expect("just ensured object above");
    let records = obj.entry("written_records").or_insert_with(|| json!([]));
    if let Some(arr) = records.as_array_mut() {
        if arr.len() >= MAX_WRITTEN_RECORDS {
            arr.remove(0);
        }
        arr.push(json!({ "summary": summary, "day": sim_day }));
    }
}

/// A literate individual can "read" another literate individual's written
/// records -- transmitting knowledge of past events neither of them needs to
/// have witnessed together, the writing-stage counterpart to
/// `learn_from_teacher`'s vocabulary transmission. Both parties must already
/// have writing; this never grants the writing capability itself.
pub fn read_written_records(reader: &mut Individual, source: &Individual) {
    if !reader.language.writing || !source.language.writing {
        return;
    }
    let Some(source_records) = source.memory.get("written_records").and_then(Value::as_array) else { return };
    if source_records.is_empty() {
        return;
    }
    if !reader.memory.is_object() {
        reader.memory = json!({});
    }
    let obj = reader.memory.as_object_mut().expect("just ensured object above");
    let reader_records = obj.entry("written_records").or_insert_with(|| json!([]));
    if let Some(reader_arr) = reader_records.as_array_mut() {
        for rec in source_records {
            if !reader_arr.contains(rec) {
                reader_arr.push(rec.clone());
            }
        }
        while reader_arr.len() > MAX_WRITTEN_RECORDS {
            reader_arr.remove(0);
        }
    }
}

pub fn get_language_summary(population: &[Individual]) -> Value {
    let mut map = serde_json::Map::new();
    for ind in population {
        if !ind.alive {
            continue;
        }
        let stage = ind.language.stage_name.as_str();
        let stage = if stage.is_empty() { "pre-linguistic" } else { stage };
        let count = map.get(stage).and_then(|v| v.as_i64()).unwrap_or(0) + 1;
        map.insert(stage.to_string(), json!(count));
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Language, Phenotype};

    fn make_lang(stage: i32, foxp2: f64) -> Language {
        Language { stage, stage_name: "pre-linguistic".to_string(), foxp2_expression: foxp2, ..Default::default() }
    }

    fn make_ind(language: Language) -> Individual {
        Individual {
            phenotype: Phenotype { language_capacity: 0.9, fluid_intelligence: 0.7, ..Default::default() },
            language,
            ..Default::default()
        }
    }

    // ── updateLanguageStage — FOXP2 threshold ──────────────────────────

    #[test]
    fn foxp2_below_stage2_threshold_blocks_transition() {
        let mut ind = make_ind(make_lang(1, 0.35));
        let res = update_language_stage(&mut ind, 10, 5);
        assert_eq!(res["upgraded"], false);
        assert_eq!(ind.language.stage, 1);
    }

    #[test]
    fn foxp2_group_and_gen_all_sufficient_upgrades_to_stage2() {
        let mut ind = make_ind(make_lang(1, 0.42));
        let res = update_language_stage(&mut ind, 6, 2);
        assert_eq!(res["upgraded"], true);
        assert_eq!(ind.language.stage, 2);
    }

    #[test]
    fn upgrades_to_stage3_proto_words() {
        let mut ind = make_ind(make_lang(2, 0.56));
        let res = update_language_stage(&mut ind, 10, 5);
        assert_eq!(res["upgraded"], true);
        assert_eq!(ind.language.stage, 3);
    }

    #[test]
    fn upgrades_to_stage4_and_enables_grammar() {
        let mut ind = make_ind(make_lang(3, 0.66));
        let res = update_language_stage(&mut ind, 16, 9);
        assert_eq!(res["upgraded"], true);
        assert!(ind.language.grammar);
    }

    #[test]
    fn upgrades_to_stage6_and_enables_writing() {
        let mut ind = make_ind(make_lang(5, 0.82));
        let res = update_language_stage(&mut ind, 45, 26);
        assert_eq!(res["upgraded"], true);
        assert_eq!(ind.language.stage, 6);
        assert!(ind.language.writing);
    }

    // ── group size threshold ───────────────────────────────────────────

    #[test]
    fn group_below_3_blocks_stage1_transition() {
        let mut ind = make_ind(make_lang(0, 0.0));
        let res = update_language_stage(&mut ind, 2, 0);
        assert_eq!(res["upgraded"], false);
    }

    #[test]
    fn group_exactly_at_threshold_is_inclusive() {
        let mut ind = make_ind(make_lang(3, 0.66));
        let res = update_language_stage(&mut ind, 15, 9);
        assert_eq!(res["upgraded"], true);
        assert_eq!(ind.language.stage, 4);
    }

    #[test]
    fn group_one_below_threshold_blocks_transition() {
        let mut ind = make_ind(make_lang(3, 0.66));
        let res = update_language_stage(&mut ind, 14, 9);
        assert_eq!(res["upgraded"], false);
    }

    // ── generation count threshold ─────────────────────────────────────

    #[test]
    fn generation_below_threshold_blocks_stage2() {
        let mut ind = make_ind(make_lang(1, 0.45));
        let res = update_language_stage(&mut ind, 10, 0);
        assert_eq!(res["upgraded"], false);
    }

    #[test]
    fn generation_below_threshold_blocks_stage3() {
        let mut ind = make_ind(make_lang(2, 0.60));
        let res = update_language_stage(&mut ind, 10, 3);
        assert_eq!(res["upgraded"], false);
    }

    #[test]
    fn generation_exactly_at_threshold_is_inclusive() {
        let mut ind = make_ind(make_lang(3, 0.66));
        let res = update_language_stage(&mut ind, 16, 8);
        assert_eq!(res["upgraded"], true);
    }

    // ── multiple threshold failures ────────────────────────────────────

    #[test]
    fn foxp2_sufficient_but_group_insufficient_blocks() {
        let mut ind = make_ind(make_lang(2, 0.60));
        let res = update_language_stage(&mut ind, 4, 5);
        assert_eq!(res["upgraded"], false);
    }

    #[test]
    fn group_sufficient_but_foxp2_insufficient_blocks() {
        let mut ind = make_ind(make_lang(2, 0.50));
        let res = update_language_stage(&mut ind, 10, 5);
        assert_eq!(res["upgraded"], false);
    }

    #[test]
    fn generation_sufficient_but_foxp2_insufficient_blocks() {
        let mut ind = make_ind(make_lang(3, 0.60));
        let res = update_language_stage(&mut ind, 20, 10);
        assert_eq!(res["upgraded"], false);
    }

    #[test]
    fn no_regression_when_already_past_all_satisfied_stages() {
        let mut ind = make_ind(make_lang(4, 0.70));
        let res = update_language_stage(&mut ind, 20, 10);
        assert_eq!(res["upgraded"], false);
        assert_eq!(ind.language.stage, 4);
    }

    // ── stage_name updates ─────────────────────────────────────────────

    #[test]
    fn stage_name_becomes_proto_words_on_stage3() {
        let mut ind = make_ind(make_lang(2, 0.56));
        update_language_stage(&mut ind, 10, 5);
        assert_eq!(ind.language.stage_name, "proto-words");
    }

    #[test]
    fn stage_name_becomes_writing_on_stage6() {
        let mut ind = make_ind(make_lang(5, 0.82));
        update_language_stage(&mut ind, 50, 30);
        assert_eq!(ind.language.stage_name, "writing");
    }

    // ── advances at most one stage per call ────────────────────────────

    #[test]
    fn advances_at_most_one_stage_even_when_all_higher_thresholds_met() {
        let mut ind = make_ind(make_lang(0, 0.95));
        let res = update_language_stage(&mut ind, 50, 30);
        assert_eq!(res["upgraded"], true);
        assert_eq!(ind.language.stage, 1);
    }

    #[test]
    fn requires_six_separate_calls_to_reach_stage_6_from_0() {
        let mut ind = make_ind(make_lang(0, 0.95));
        let mut upgrades = 0;
        for _ in 0..10 {
            let res = update_language_stage(&mut ind, 50, 30);
            if res["upgraded"] == true {
                upgrades += 1;
            }
            if ind.language.stage == 6 {
                break;
            }
        }
        assert!(upgrades >= 6);
        assert_eq!(ind.language.stage, 6);
    }

    #[test]
    fn stage_name_matches_actual_stage_after_each_step() {
        let names = ["pre-linguistic", "gestural", "emotional-sounds", "proto-words", "syntax", "abstract", "writing"];
        let mut ind = make_ind(make_lang(0, 0.95));
        for expected in 1..=6 {
            update_language_stage(&mut ind, 50, 30);
            assert_eq!(ind.language.stage, expected);
            assert_eq!(ind.language.stage_name, names[expected as usize]);
        }
    }

    // ── FOXP2 fallback uses cap * 0.10 ─────────────────────────────────

    #[test]
    fn missing_foxp2_falls_back_to_ten_percent_of_capacity() {
        // cap = 0.4; fallback = 0.4 * 0.10 = 0.04 -- below stage-2 foxp2_min of 0.40.
        let ind_lang = Language { stage: 1, stage_name: "gestural".to_string(), foxp2_expression: 0.0, ..Default::default() };
        let mut ind = Individual {
            phenotype: Phenotype { language_capacity: 0.4, fluid_intelligence: 0.7, ..Default::default() },
            language: ind_lang,
            ..Default::default()
        };
        let res = update_language_stage(&mut ind, 10, 2);
        assert_eq!(res["upgraded"], false);
    }

    // ── Cardinal rule: phenotype is never mutated by this engine ───────

    #[test]
    fn update_language_stage_never_touches_phenotype_fields() {
        let mut ind = make_ind(make_lang(1, 0.45));
        let before = ind.phenotype.clone();
        for _ in 0..100 {
            update_language_stage(&mut ind, 10, 5);
        }
        assert_eq!(ind.phenotype, before);
    }

    #[test]
    fn update_foxp2_expression_never_touches_phenotype_fields() {
        let mut ind = make_ind(make_lang(1, 0.3));
        let before = ind.phenotype.clone();
        for _ in 0..500 {
            update_foxp2_expression(&mut ind, 10);
        }
        assert_eq!(ind.phenotype, before);
    }

    // ── updateFoxp2Expression ──────────────────────────────────────────

    #[test]
    fn foxp2_grows_faster_with_larger_groups() {
        let mut ind1 = make_ind(make_lang(1, 0.3));
        let mut ind2 = make_ind(make_lang(1, 0.3));
        update_foxp2_expression(&mut ind1, 2);
        update_foxp2_expression(&mut ind2, 10);
        assert!(ind2.language.foxp2_expression > ind1.language.foxp2_expression);
    }

    #[test]
    fn foxp2_cannot_exceed_genetic_ceiling() {
        let cap = 0.7;
        let mut ind = Individual {
            phenotype: Phenotype { language_capacity: cap, fluid_intelligence: 0.7, ..Default::default() },
            language: make_lang(2, cap - 0.001),
            ..Default::default()
        };
        for _ in 0..1000 {
            update_foxp2_expression(&mut ind, 10);
        }
        assert!(ind.language.foxp2_expression <= cap + 1e-9);
    }

    #[test]
    fn foxp2_still_grows_via_staging_bonus_with_no_group() {
        let mut ind = make_ind(make_lang(1, 0.3));
        let before = ind.language.foxp2_expression;
        update_foxp2_expression(&mut ind, 0);
        assert!(ind.language.foxp2_expression > before);
    }

    // ── learnFromTeacher ────────────────────────────────────────────────

    #[test]
    fn learner_picks_up_words_from_teacher() {
        let mut teacher_lang = Language { stage: 3, foxp2_expression: 0.6, ..Default::default() };
        teacher_lang.vocabulary.insert("fire".to_string(), "ba".to_string());
        teacher_lang.vocabulary.insert("water".to_string(), "mo".to_string());
        let teacher = make_ind(teacher_lang);
        let mut learner = Individual {
            phenotype: Phenotype { language_capacity: 0.8, fluid_intelligence: 0.9, ..Default::default() },
            language: make_lang(3, 0.6),
            ..Default::default()
        };
        learn_from_teacher(&mut learner, &teacher);
        assert_eq!(learner.language.vocabulary.get("fire"), Some(&"ba".to_string()));
    }

    // ── BUG-15 regression — learn_from_teacher FOXP2 threshold (0.25) ──

    fn teacher_with_vocab() -> Individual {
        let mut teacher_lang = Language { stage: 3, foxp2_expression: 0.6, ..Default::default() };
        for (k, v) in [("fire", "ba"), ("water", "mo"), ("tree", "ku")] {
            teacher_lang.vocabulary.insert(k.to_string(), v.to_string());
        }
        Individual {
            phenotype: Phenotype { language_capacity: 0.8, fluid_intelligence: 0.8, ..Default::default() },
            language: teacher_lang,
            ..Default::default()
        }
    }

    fn learner_with_foxp2(foxp2: f64) -> Individual {
        Individual {
            phenotype: Phenotype { language_capacity: 0.8, fluid_intelligence: 0.8, ..Default::default() },
            language: Language { stage: 2, foxp2_expression: foxp2, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn foxp2_below_0_25_blocks_word_acquisition() {
        let mut learner = learner_with_foxp2(0.20);
        learn_from_teacher(&mut learner, &teacher_with_vocab());
        assert!(learner.language.vocabulary.is_empty());
    }

    #[test]
    fn foxp2_just_below_threshold_0_24_is_blocked() {
        let mut learner = learner_with_foxp2(0.24);
        learn_from_teacher(&mut learner, &teacher_with_vocab());
        assert!(learner.language.vocabulary.is_empty());
    }

    #[test]
    fn foxp2_at_threshold_0_25_allows_acquisition() {
        let mut learner = learner_with_foxp2(0.25);
        learn_from_teacher(&mut learner, &teacher_with_vocab());
        assert!(!learner.language.vocabulary.is_empty());
    }

    #[test]
    fn already_known_words_are_never_overwritten() {
        let mut teacher_lang = Language { stage: 3, foxp2_expression: 0.6, ..Default::default() };
        teacher_lang.vocabulary.insert("fire".to_string(), "za".to_string());
        let teacher = make_ind(teacher_lang);
        let mut learner_lang = Language { stage: 3, foxp2_expression: 0.6, ..Default::default() };
        learner_lang.vocabulary.insert("fire".to_string(), "ba".to_string());
        let mut learner = Individual {
            phenotype: Phenotype { language_capacity: 0.8, fluid_intelligence: 0.9, ..Default::default() },
            language: learner_lang,
            ..Default::default()
        };
        learn_from_teacher(&mut learner, &teacher);
        assert_eq!(learner.language.vocabulary.get("fire"), Some(&"ba".to_string()));
    }

    #[test]
    fn low_iq_learners_pick_up_fewer_words() {
        let mut teacher_lang = Language { stage: 3, foxp2_expression: 0.6, ..Default::default() };
        for (k, v) in [("a", "1"), ("b", "2"), ("c", "3"), ("d", "4")] {
            teacher_lang.vocabulary.insert(k.to_string(), v.to_string());
        }
        let teacher = make_ind(teacher_lang);
        let mut learner = Individual {
            phenotype: Phenotype { language_capacity: 0.5, fluid_intelligence: 0.4, ..Default::default() },
            language: make_lang(3, 0.6),
            ..Default::default()
        };
        learn_from_teacher(&mut learner, &teacher);
        // maxLearn = floor(0.4 * 3) = 1
        assert!(learner.language.vocabulary.len() <= 1);
    }

    // ── generateProtoWord ───────────────────────────────────────────────

    #[test]
    fn same_concept_and_group_yields_deterministic_word() {
        let p = full_palette();
        assert_eq!(generate_proto_word("fire", "g1", &p), generate_proto_word("fire", "g1", &p));
    }

    #[test]
    fn different_concept_yields_different_word() {
        let p = full_palette();
        assert_ne!(generate_proto_word("fire", "g1", &p), generate_proto_word("water", "g1", &p));
    }

    #[test]
    fn different_group_yields_a_different_dialect_word() {
        let p = full_palette();
        assert_ne!(generate_proto_word("fire", "g1", &p), generate_proto_word("fire", "g2", &p));
    }

    #[test]
    fn generated_words_contain_only_lowercase_letters() {
        let p = full_palette();
        for concept in &CORE_CONCEPTS[0..10] {
            let word = generate_proto_word(concept, "g1", &p);
            assert!(word.chars().all(|c| c.is_ascii_lowercase()), "word {word} contains non-letters");
        }
    }

    #[test]
    fn generated_word_length_is_between_2_and_6_characters() {
        let p = full_palette();
        for concept in CORE_CONCEPTS {
            let word = generate_proto_word(concept, "g1", &p);
            assert!(word.len() >= 2 && word.len() <= 6, "word {word} for {concept} out of range");
        }
    }

    #[test]
    fn empty_palette_produces_no_word() {
        let empty = PhonemePalette::default();
        assert_eq!(generate_proto_word("fire", "g1", &empty), "");
    }

    // ── derivePhonemePalette ────────────────────────────────────────────

    fn genome_with(foxp2: f64, cntnap2: f64) -> Genome {
        use crate::types::{Allele, Locus};
        let mut g = Genome::new();
        g.insert(
            "FOXP2_01".to_string(),
            Locus { allele1: Allele { value: Some(foxp2), origin: "test".into() }, allele2: Allele { value: Some(foxp2), origin: "test".into() }, ..Default::default() },
        );
        g.insert(
            "CNTNAP2_01".to_string(),
            Locus { allele1: Allele { value: Some(cntnap2), origin: "test".into() }, allele2: Allele { value: Some(cntnap2), origin: "test".into() }, ..Default::default() },
        );
        g
    }

    #[test]
    fn same_founder_genomes_always_derive_the_same_palette() {
        let g1 = genome_with(0.8, 0.7);
        let g2 = genome_with(0.6, 0.5);
        assert_eq!(derive_phoneme_palette(&g1, &g2), derive_phoneme_palette(&g1, &g2));
    }

    #[test]
    fn different_founder_genomes_derive_different_palettes() {
        let a = derive_phoneme_palette(&genome_with(0.9, 0.9), &genome_with(0.9, 0.9));
        let b = derive_phoneme_palette(&genome_with(0.2, 0.2), &genome_with(0.2, 0.2));
        assert_ne!(a, b);
    }

    #[test]
    fn higher_articulatory_precision_yields_a_larger_repertoire() {
        let low = derive_phoneme_palette(&genome_with(0.1, 0.1), &genome_with(0.1, 0.1));
        let high = derive_phoneme_palette(&genome_with(1.0, 1.0), &genome_with(1.0, 1.0));
        assert!(high.consonants.len() >= low.consonants.len());
        assert!(high.vowels.len() >= low.vowels.len());
    }

    #[test]
    fn derived_palette_never_exceeds_the_universal_superset() {
        let p = derive_phoneme_palette(&genome_with(1.0, 1.0), &genome_with(1.0, 1.0));
        assert!(p.consonants.iter().all(|c| CONSONANT_SUPERSET.contains(c)));
        assert!(p.vowels.iter().all(|c| VOWEL_SUPERSET.contains(c)));
    }

    // ── vocabulary_by_group ─────────────────────────────────────────────

    #[test]
    fn vocabulary_by_group_only_includes_grouped_living_individuals() {
        let mut a = make_ind(make_lang(3, 0.6));
        a.alive = true;
        a.group_id = Some("alpha".to_string());
        a.language.vocabulary.insert("fire".to_string(), "za".to_string());
        let mut solo = make_ind(make_lang(3, 0.6));
        solo.alive = true;
        solo.language.vocabulary.insert("fire".to_string(), "xx".to_string());
        let vocab = get_vocabulary_by_group(&[a, solo]);
        assert_eq!(vocab["alpha"]["fire"], "za");
        assert_eq!(vocab.as_object().unwrap().len(), 1);
    }

    // ── record_event_for_posterity / read_written_records ───────────────

    #[test]
    fn only_a_literate_individual_can_record_an_event() {
        let mut ind = make_ind(make_lang(3, 0.6));
        record_event_for_posterity(&mut ind, &json!({ "type": "flood" }), 10);
        assert!(ind.memory.get("written_records").is_none(), "writing is required to record anything");
    }

    #[test]
    fn a_literate_individual_records_a_bounded_history_of_notable_events() {
        let mut ind = make_ind(make_lang(6, 0.9));
        ind.language.writing = true;
        for day in 0..(MAX_WRITTEN_RECORDS + 10) {
            record_event_for_posterity(&mut ind, &json!({ "type": "flood" }), day as i32);
        }
        let records = ind.memory["written_records"].as_array().unwrap();
        assert_eq!(records.len(), MAX_WRITTEN_RECORDS);
    }

    #[test]
    fn a_literate_reader_can_read_another_literate_individuals_records_even_without_witnessing_them() {
        let mut scribe = make_ind(make_lang(6, 0.9));
        scribe.language.writing = true;
        record_event_for_posterity(&mut scribe, &json!({ "type": "eclipse_solar" }), 42);

        let mut reader = make_ind(make_lang(6, 0.9));
        reader.language.writing = true;
        read_written_records(&mut reader, &scribe);

        let records = reader.memory["written_records"].as_array().unwrap();
        assert!(records.iter().any(|r| r["summary"] == "eclipse_solar" && r["day"] == 42));
    }

    #[test]
    fn an_illiterate_reader_gains_nothing_from_a_literate_scribes_records() {
        let mut scribe = make_ind(make_lang(6, 0.9));
        scribe.language.writing = true;
        record_event_for_posterity(&mut scribe, &json!({ "type": "eclipse_solar" }), 42);

        let mut illiterate_reader = make_ind(make_lang(2, 0.5));
        read_written_records(&mut illiterate_reader, &scribe);
        assert!(illiterate_reader.memory.get("written_records").is_none());
    }

    #[test]
    fn population_with_no_founders_gets_an_empty_palette() {
        let p = derive_phoneme_palette_from_population(&[]);
        assert!(p.consonants.is_empty() && p.vowels.is_empty());
    }
}
