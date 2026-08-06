use crate::state::Individual;
use serde_json::{json, Value};

/// Candidate concepts in priority order, matching the client's own
/// `clientThoughtFromVocab` fallback (PopulationPanel.tsx) so the transition
/// from "server never sent a thought" to "server sends one" is seamless in
/// the Inner Voice panel. Concepts are only ever drawn from the 28-word
/// CORE_CONCEPTS an individual can actually acquire (see language.rs) --
/// unlike the client's list, this never references a concept ("pain") that
/// vocabulary acquisition could never produce in the first place.
fn priority_concepts(ind: &Individual, sim_day: i32) -> Vec<&'static str> {
    let hunger = 1.0 - ind.health.calories;
    let thirst = 1.0 - ind.health.hydration;
    let hp = ind.health.hp;
    let mental = ind.psychology.mental_state.as_str();
    let recent_kin_death = ind.psychology.trauma_events.iter().any(|e| e.get("type").and_then(Value::as_str) == Some("kin_death") && (sim_day as i64 - e.get("day").and_then(Value::as_i64).unwrap_or(0)) < 20);
    let recent_disaster = ind.psychology.trauma_events.iter().any(|e| {
        let ty = e.get("type").and_then(Value::as_str);
        ty.is_some() && ty != Some("kin_death") && (sim_day as i64 - e.get("day").and_then(Value::as_i64).unwrap_or(0)) < 15
    });
    let has_group = ind.group_id.is_some();
    let has_mate = ind.social.has_mate;
    let curiosity = ind.phenotype.curiosity;
    let c = ind.mind.consciousness;
    let wellbeing = ind.psychology.wellbeing;

    let mut priority = Vec::new();
    if hunger > 0.7 {
        priority.extend(["food", "hunt", "eat"]);
    }
    if thirst > 0.7 {
        priority.push("water");
    }
    if hp < 0.3 {
        priority.push("die");
    }
    if recent_disaster {
        priority.extend(["danger", "run", "fire"]);
    }
    if mental == "grieving" || recent_kin_death {
        priority.extend(["die", "you", "bad"]);
    }
    if mental == "anxious" {
        priority.extend(["danger", "bad"]);
    }
    if mental == "depressed" {
        priority.extend(["bad", "sleep"]);
    }
    if hunger > 0.45 {
        priority.push("food");
    }
    if thirst > 0.45 {
        priority.push("water");
    }
    if !has_group {
        priority.extend(["us", "here", "you"]);
    }
    if has_mate {
        priority.extend(["you", "good"]);
    }
    if mental == "excited" {
        priority.extend(["good", "us"]);
    }
    if c > 0.3 && curiosity > 0.6 {
        priority.extend(["sky", "sun", "moon", "time"]);
    }
    if c > 0.5 {
        priority.extend(["god", "spirit", "time"]);
    }
    if wellbeing > 0.7 {
        priority.extend(["good", "here"]);
    }
    priority.extend(["me", "here", "good", "bad", "sleep", "earth", "light", "dark", "rain", "sun"]);
    priority
}

/// Builds this tick's inner-thought snapshot (`proto` word sequence plus a
/// concept-glossed `annotated` form) from whichever of the priority concepts
/// above the individual actually knows a word for, or `None` before stage 2 /
/// with an empty vocabulary -- mirroring the client's own gating so a
/// present-but-empty thought never displays as if it were real speech.
fn compute_thought(ind: &Individual, sim_day: i32) -> Option<(String, String)> {
    if ind.language.stage < 2 || ind.language.vocabulary.is_empty() {
        return None;
    }
    let mut seen = std::collections::HashSet::new();
    let known: Vec<&str> = priority_concepts(ind, sim_day)
        .into_iter()
        .filter(|c| seen.insert(*c))
        .filter(|c| ind.language.vocabulary.contains_key(*c))
        .collect();
    if known.is_empty() {
        return None;
    }
    let c = ind.mind.consciousness;
    let max_words = if ind.language.stage <= 2 {
        1
    } else if ind.language.stage == 3 {
        known.len().min(2)
    } else {
        known.len().min(3 + (c * 3.0) as usize)
    };
    let span = (known.len().saturating_sub(max_words) + 1).max(1);
    let start = (sim_day as usize) % span;
    let selected = &known[start..(start + max_words).min(known.len())];
    if selected.is_empty() {
        return None;
    }
    let sep = if ind.language.stage >= 4 { "  " } else { "... " };
    let proto = selected.iter().map(|c| ind.language.vocabulary[*c].as_str()).collect::<Vec<_>>().join(sep);
    let annotated = selected.iter().map(|c| format!("{} [{c}]", ind.language.vocabulary[*c])).collect::<Vec<_>>().join(sep);
    Some((proto, annotated))
}

/// Appends a life-log entry the first time (and only the first time) each
/// milestone kind is crossed, so the client's "Life Log" tab (PopulationPanel
/// -> InnerVoiceModal) has real data instead of always reading an empty
/// array. Each kind fires at most once per individual by design -- these are
/// "first time this happened" markers, not a running feed.
const MILESTONE_KINDS: usize = 9;

fn record_life_log_milestones(ind: &mut Individual, sim_day: i32, thought: Option<&(String, String)>) {
    // Every one of the 9 possible kinds fires at most once ever (see the
    // doc comment above), so once all 9 are recorded this function has
    // permanently nothing left to do -- for any individual who has lived
    // past their last milestone (most adults, most of their life), that's
    // every remaining tick until they die. Bail out before touching JSON at
    // all rather than deserializing/reserializing both arrays for nothing,
    // every single day, for the rest of a decades-long simulation.
    if ind.mind.extra.get("_recorded_milestones").and_then(Value::as_array).is_some_and(|a| a.len() >= MILESTONE_KINDS) {
        return;
    }

    let mut recorded: Vec<String> = ind
        .mind
        .extra
        .get("_recorded_milestones")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let mut log: Vec<Value> = ind.mind.extra.get("inner_thought_log").and_then(Value::as_array).cloned().unwrap_or_default();

    let mut push = |kind: &str, condition: bool| {
        if !condition || recorded.iter().any(|r| r == kind) {
            return;
        }
        recorded.push(kind.to_string());
        let mut entry = json!({ "day": sim_day, "kind": kind });
        if let Some((proto, annotated)) = thought {
            entry["thought"] = json!({ "proto": proto, "annotated": annotated });
        }
        log.push(entry);
    };

    push("first_word", !ind.language.vocabulary.is_empty());
    push("first_thought", thought.is_some());
    push("first_abstract", ind.language.stage >= 5);
    push("consciousness_10", ind.mind.consciousness >= 0.10);
    push("consciousness_25", ind.mind.consciousness >= 0.25);
    push("consciousness_50", ind.mind.consciousness >= 0.50);
    push("consciousness_75", ind.mind.consciousness >= 0.75);
    push("death_proximity", ind.health.hp < 0.3);
    push("grief", ind.psychology.mental_state == "grieving");

    ind.mind.extra.insert("_recorded_milestones".to_string(), json!(recorded));
    ind.mind.extra.insert("inner_thought_log".to_string(), json!(log));
}

/// The six additive terms of the consciousness-growth formula, exposed as
/// their own struct (rather than inlined directly in `update_consciousness`)
/// purely so `consciousness_sensitivity` (this crate's ablation/sensitivity
/// harness -- see that module's own doc comment) can re-run a trajectory
/// with one term zeroed out and observe the *real* resulting trajectory,
/// including how the `.clamp()` in `update_consciousness` interacts with
/// that change, rather than approximating an ablation by subtracting a
/// term's contribution after the fact (invalid here specifically because
/// the clamp makes the formula non-linear across many ticks: removing a
/// positive term can change whether/when a trajectory hits its ceiling).
/// `update_consciousness` is a thin wrapper that applies this struct's sum
/// and the genetic ceiling clamp -- `consciousness_never_exceeds_its_genetic_ceiling`
/// and this module's other pre-existing tests still pass unchanged, since
/// this extraction itself doesn't alter the clamp or the sum. (The bonus
/// terms' own values *do* now additionally depend on genetics via
/// `genetic_responsiveness_multiplier` below -- see that function's doc
/// comment -- which is a deliberate behavior change, not a side effect of
/// this extraction.)
pub struct ConsciousnessDelta {
    pub base_rate: f64,
    pub lang_bonus: f64,
    pub social_bonus: f64,
    pub tom_bonus: f64,
    pub stress_penalty: f64,
    pub injury_penalty: f64,
}

impl ConsciousnessDelta {
    pub fn sum(&self) -> f64 {
        self.base_rate + self.lang_bonus + self.social_bonus + self.tom_bonus - self.stress_penalty - self.injury_penalty
    }
}

/// How strongly language stage, group membership, and theory of mind
/// translate into consciousness growth used to be three flat constants
/// (0.0005, 0.0002, 0.0003) -- identical for every individual regardless of
/// their own genetics, which is the opposite of the cardinal rule's spirit:
/// everything else in this formula (consciousness_potential itself) already
/// varies per-individual by genome, but how *effectively* an individual
/// turns language/social/empathic capacity into consciousness didn't.
///
/// This makes that responsiveness itself a heritable trait -- without
/// adding new genome loci (the 32-locus genome is a headline number
/// documented across the client UI, AGENTS.md, and this crate's own tests;
/// adding to it is a much larger, separate change). Instead each multiplier
/// reuses a phenotype trait *already* derived from an existing locus that is
/// the real biological substrate for that specific bonus:
///   - lang_bonus scales with `language_capacity` (FOXP2_01/CNTNAP2_01) --
///     the literal genes this formula's lang_bonus term is named after.
///   - social_bonus scales with `oxytocin_sensitivity` (OXTR_01) -- the real
///     receptor-sensitivity gene behind social/pair bonding in this engine's
///     own hormones.rs.
///   - tom_bonus scales with `self_awareness` (NRXN1_01/SHANK3_01/RELN_01
///     composite) -- RELN_01 is literally annotated `"theory_of_mind"` in
///     genome.rs's own locus table.
///
/// Each multiplier is bounded to 0.5x-1.5x (a below-average-genetics
/// individual is never fully zeroed out, an above-average one is never
/// unboundedly amplified), the same "small, bounded, additive/multiplicative
/// layered on top of the existing formula" pattern this codebase already
/// uses throughout (see AGENTS.md's Hormones section for numerous examples)
/// rather than replacing the hand-picked base weights outright.
fn genetic_responsiveness_multiplier(trait_value: f64) -> f64 {
    0.5 + trait_value.clamp(0.0, 1.0)
}

pub fn compute_consciousness_delta(ind: &Individual) -> ConsciousnessDelta {
    let potential = ind.phenotype.consciousness_potential;
    let hp = ind.health.hp;
    let lang_multiplier = genetic_responsiveness_multiplier(ind.phenotype.language_capacity);
    let social_multiplier = genetic_responsiveness_multiplier(ind.phenotype.oxytocin_sensitivity);
    let tom_multiplier = genetic_responsiveness_multiplier(ind.phenotype.self_awareness);
    ConsciousnessDelta {
        base_rate: (potential * 0.001).max(0.00015),
        lang_bonus: ind.language.stage as f64 / 6.0 * 0.0005 * lang_multiplier,
        social_bonus: (if ind.group_id.is_some() { 0.0002 } else { 0.0 }) * social_multiplier,
        tom_bonus: ind.psychology.theory_of_mind as f64 / 3.0 * 0.0003 * tom_multiplier,
        stress_penalty: ind.psychology.stress_level * 0.0003,
        injury_penalty: if hp < 0.3 { (0.3 - hp) * 0.002 } else { 0.0 },
    }
}

pub fn update_consciousness(ind: &mut Individual) {
    let potential = ind.phenotype.consciousness_potential;
    let delta = compute_consciousness_delta(ind);
    let current = ind.mind.consciousness;
    let ceiling = (potential * 1.2).min(1.0);
    let next = (current + delta.sum()).clamp(0.0, ceiling);
    ind.mind.consciousness = next;
}

/// Sole writer of `mind.inner_thought`/`mind.inner_thought_log`, mirroring
/// the cardinal rule already enforced for `mind.consciousness` above: no
/// other engine sets these fields. Behavioral, not scripted -- everything
/// here is derived from the individual's own vocabulary, mental state and
/// physiology, exactly like the client-side fallback it replaces.
pub fn update_inner_thought(ind: &mut Individual, sim_day: i32) {
    let thought = compute_thought(ind, sim_day);
    match &thought {
        Some((proto, annotated)) => {
            ind.mind.extra.insert("inner_thought".to_string(), json!({ "proto": proto, "annotated": annotated }));
        }
        None => {
            ind.mind.extra.remove("inner_thought");
        }
    }
    record_life_log_milestones(ind, sim_day, thought.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Health, Language, Mind, Phenotype, Psychology};

    fn base_individual(consciousness: f64, potential: f64) -> Individual {
        Individual {
            phenotype: Phenotype { consciousness_potential: potential, ..Default::default() },
            mind: Mind { consciousness, ..Default::default() },
            language: Language { stage: 0, ..Default::default() },
            psychology: Psychology { stress_level: 0.0, theory_of_mind: 0, ..Default::default() },
            health: Health { hp: 1.0, ..Default::default() },
            group_id: None,
            ..Default::default()
        }
    }

    #[test]
    fn consciousness_never_exceeds_its_genetic_ceiling() {
        let mut ind = base_individual(0.99, 0.5);
        for _ in 0..100_000 {
            update_consciousness(&mut ind);
        }
        let ceiling = (0.5_f64 * 1.2).min(1.0);
        assert!(ind.mind.consciousness <= ceiling + 1e-9, "consciousness {} exceeded genetic ceiling {ceiling}", ind.mind.consciousness);
    }

    #[test]
    fn consciousness_only_increases_when_signals_are_favourable() {
        let mut ind = base_individual(0.0, 0.6);
        update_consciousness(&mut ind);
        assert!(ind.mind.consciousness > 0.0, "baseline growth rate must be strictly positive per the cardinal formula");
    }

    #[test]
    fn low_but_nonzero_potential_grows_at_the_floor_rate_not_scaled_to_zero() {
        // baseRate = max(potential * 0.001, 0.00015): even a low-potential individual
        // grows at the floor rate rather than an arbitrarily tiny scaled value.
        let mut ind = base_individual(0.0, 0.01);
        update_consciousness(&mut ind);
        assert!(ind.mind.consciousness >= 0.00015 - 1e-9);
    }

    #[test]
    fn zero_genetic_potential_gives_a_zero_ceiling() {
        // ceiling = min(1, potential * 1.2); a true zero-potential individual can
        // never express any consciousness, by construction of the cardinal formula.
        let mut ind = base_individual(0.0, 0.0);
        update_consciousness(&mut ind);
        assert_eq!(ind.mind.consciousness, 0.0);
    }

    #[test]
    fn update_consciousness_never_touches_phenotype_fields() {
        let mut ind = base_individual(0.1, 0.5);
        let before = ind.phenotype.clone();
        for _ in 0..1000 {
            update_consciousness(&mut ind);
        }
        assert_eq!(ind.phenotype, before, "update_consciousness must never mutate phenotype");
    }

    #[test]
    fn update_consciousness_only_mutates_the_consciousness_field_of_mind() {
        let mut ind = base_individual(0.0, 0.6);
        let mut before = ind.mind.clone();
        update_consciousness(&mut ind);
        let mut after = ind.mind.clone();
        assert!(after.consciousness > 0.0);
        // Zero out the one field allowed to change and the rest must be identical.
        before.consciousness = 0.0;
        after.consciousness = 0.0;
        assert_eq!(before, after, "update_consciousness must only mutate mind.consciousness");
    }

    #[test]
    fn ceiling_saturates_at_one_when_potential_times_1_2_exceeds_one() {
        let mut ind = base_individual(0.99, 0.9); // potential*1.2 = 1.08 -> ceiling clamps to 1.0
        for _ in 0..50_000 {
            update_consciousness(&mut ind);
        }
        assert!(ind.mind.consciousness <= 1.0 + 1e-9);
    }

    #[test]
    fn higher_language_stage_grows_consciousness_faster() {
        let mut base = base_individual(0.0, 0.8);
        let mut with_lang = base_individual(0.0, 0.8);
        with_lang.language.stage = 6;
        update_consciousness(&mut base);
        update_consciousness(&mut with_lang);
        assert!(with_lang.mind.consciousness > base.mind.consciousness);
    }

    #[test]
    fn group_membership_adds_a_social_bonus() {
        let mut alone = base_individual(0.0, 0.8);
        let mut in_group = base_individual(0.0, 0.8);
        in_group.group_id = Some("g1".to_string());
        update_consciousness(&mut alone);
        update_consciousness(&mut in_group);
        assert!(in_group.mind.consciousness > alone.mind.consciousness);
    }

    #[test]
    fn heavy_injury_hp_below_0_3_slows_consciousness_growth() {
        let mut healthy = base_individual(0.1, 0.8);
        healthy.health.hp = 1.0;
        let mut injured = base_individual(0.1, 0.8);
        injured.health.hp = 0.1;
        update_consciousness(&mut healthy);
        update_consciousness(&mut injured);
        assert!(healthy.mind.consciousness > injured.mind.consciousness);
    }

    #[test]
    fn theory_of_mind_bonus_grows_consciousness_faster() {
        let mut no_tom = base_individual(0.0, 0.8);
        no_tom.psychology.theory_of_mind = 0;
        let mut with_tom = base_individual(0.0, 0.8);
        with_tom.psychology.theory_of_mind = 3;
        update_consciousness(&mut no_tom);
        update_consciousness(&mut with_tom);
        assert!(with_tom.mind.consciousness > no_tom.mind.consciousness);
    }

    #[test]
    fn consciousness_never_goes_negative_under_maximum_stress_without_injury() {
        let mut ind = base_individual(0.001, 0.5);
        ind.psychology.stress_level = 1.0;
        ind.health.hp = 1.0;
        for _ in 0..1000 {
            update_consciousness(&mut ind);
        }
        assert!(ind.mind.consciousness >= 0.0);
    }

    #[test]
    fn no_other_engine_function_mutates_mind_consciousness() {
        // Cardinal rule: update_consciousness is the sole writer of mind.consciousness.
        // Run a full slate of per-tick engine calls on the same individual and confirm
        // the field is untouched unless update_consciousness itself is invoked.
        let mut ind = base_individual(0.5, 0.5);
        let before = ind.mind.consciousness;

        crate::epigenetics::update_epigenome(&mut ind, None, 10);
        crate::psychology::update_mental_state(&mut ind, &[], &serde_json::json!({}), 10);
        crate::language::update_foxp2_expression(&mut ind, 3);
        crate::language::update_language_stage(&mut ind, 3, 0);

        assert_eq!(before, ind.mind.consciousness, "a non-consciousness engine mutated mind.consciousness");
    }

    fn individual_with_vocab(stage: i32, words: &[(&str, &str)], consciousness: f64) -> Individual {
        let mut ind = base_individual(consciousness, 0.8);
        ind.language.stage = stage;
        ind.language.vocabulary = words.iter().map(|(concept, word)| (concept.to_string(), word.to_string())).collect();
        ind
    }

    #[test]
    fn no_inner_thought_before_stage_two() {
        let ind = individual_with_vocab(1, &[("sleep", "zuk")], 0.5);
        assert!(compute_thought(&ind, 10).is_none());
    }

    #[test]
    fn no_inner_thought_with_an_empty_vocabulary() {
        let ind = individual_with_vocab(4, &[], 0.5);
        assert!(compute_thought(&ind, 10).is_none());
    }

    #[test]
    fn thought_only_ever_uses_words_the_individual_actually_knows() {
        let ind = individual_with_vocab(3, &[("sleep", "zuk")], 0.5);
        let (proto, annotated) = compute_thought(&ind, 10).expect("stage 3 with a known baseline concept should produce a thought");
        assert_eq!(proto, "zuk");
        assert!(annotated.contains("zuk") && annotated.contains("sleep"));
    }

    #[test]
    fn higher_stage_and_consciousness_select_more_words_than_stage_three() {
        let words: Vec<(&str, &str)> = vec![("me", "a"), ("here", "b"), ("good", "c"), ("bad", "d"), ("sleep", "e"), ("earth", "f")];
        let stage3 = individual_with_vocab(3, &words, 0.9);
        let stage5 = individual_with_vocab(5, &words, 0.9);
        let (proto3, _) = compute_thought(&stage3, 10).unwrap();
        let (proto5, _) = compute_thought(&stage5, 10).unwrap();
        assert!(proto5.split("  ").count() > proto3.split("... ").count());
    }

    #[test]
    fn update_inner_thought_never_touches_consciousness_or_phenotype() {
        let mut ind = individual_with_vocab(4, &[("sleep", "zuk"), ("water", "mip")], 0.5);
        let consciousness_before = ind.mind.consciousness;
        let phenotype_before = ind.phenotype.clone();
        update_inner_thought(&mut ind, 42);
        assert_eq!(ind.mind.consciousness, consciousness_before);
        assert_eq!(ind.phenotype, phenotype_before);
    }

    #[test]
    fn update_inner_thought_writes_a_thought_for_a_qualifying_individual() {
        let mut ind = individual_with_vocab(4, &[("sleep", "zuk"), ("water", "mip")], 0.5);
        update_inner_thought(&mut ind, 42);
        let thought = ind.mind.extra.get("inner_thought").expect("expected inner_thought to be set");
        assert!(!thought["annotated"].as_str().unwrap().is_empty());
    }

    #[test]
    fn update_inner_thought_clears_a_stale_thought_once_it_no_longer_qualifies() {
        let mut ind = individual_with_vocab(4, &[("sleep", "zuk")], 0.5);
        update_inner_thought(&mut ind, 1);
        assert!(ind.mind.extra.contains_key("inner_thought"));
        ind.language.vocabulary.clear();
        update_inner_thought(&mut ind, 2);
        assert!(!ind.mind.extra.contains_key("inner_thought"));
    }

    #[test]
    fn first_word_milestone_is_recorded_exactly_once() {
        let mut ind = individual_with_vocab(4, &[("sleep", "zuk")], 0.5);
        update_inner_thought(&mut ind, 1);
        update_inner_thought(&mut ind, 2);
        update_inner_thought(&mut ind, 3);
        let log = ind.mind.extra.get("inner_thought_log").unwrap().as_array().unwrap();
        assert_eq!(log.iter().filter(|e| e["kind"] == "first_word").count(), 1);
    }

    #[test]
    fn consciousness_milestones_accumulate_as_thresholds_are_crossed() {
        let mut ind = individual_with_vocab(4, &[("sleep", "zuk")], 0.05);
        update_inner_thought(&mut ind, 1);
        ind.mind.consciousness = 0.30;
        update_inner_thought(&mut ind, 2);
        let log = ind.mind.extra.get("inner_thought_log").unwrap().as_array().unwrap();
        assert!(log.iter().any(|e| e["kind"] == "consciousness_10"));
        assert!(log.iter().any(|e| e["kind"] == "consciousness_25"));
        assert!(!log.iter().any(|e| e["kind"] == "consciousness_50"));
    }

    #[test]
    fn death_proximity_milestone_fires_when_hp_is_low() {
        let mut ind = individual_with_vocab(4, &[("sleep", "zuk")], 0.5);
        ind.health.hp = 0.1;
        update_inner_thought(&mut ind, 1);
        let log = ind.mind.extra.get("inner_thought_log").unwrap().as_array().unwrap();
        assert!(log.iter().any(|e| e["kind"] == "death_proximity"));
    }

    #[test]
    fn grief_milestone_fires_when_grieving() {
        let mut ind = individual_with_vocab(4, &[("sleep", "zuk")], 0.5);
        ind.psychology.mental_state = "grieving".to_string();
        update_inner_thought(&mut ind, 1);
        let log = ind.mind.extra.get("inner_thought_log").unwrap().as_array().unwrap();
        assert!(log.iter().any(|e| e["kind"] == "grief"));
    }

    #[test]
    fn once_every_milestone_kind_is_recorded_the_log_never_grows_again() {
        let mut ind = individual_with_vocab(6, &[("sleep", "zuk"), ("water", "mip")], 0.9);
        ind.health.hp = 0.1;
        ind.psychology.mental_state = "grieving".to_string();
        for day in 1..50 {
            update_inner_thought(&mut ind, day);
        }
        let log_len_after_all_fired = ind.mind.extra.get("inner_thought_log").unwrap().as_array().unwrap().len();
        assert_eq!(log_len_after_all_fired, MILESTONE_KINDS);

        // Hundreds more days of an unchanged, fully-qualifying individual must
        // never append another entry -- the early-exit should keep this a no-op.
        for day in 50..500 {
            update_inner_thought(&mut ind, day);
        }
        let log_len_later = ind.mind.extra.get("inner_thought_log").unwrap().as_array().unwrap().len();
        assert_eq!(log_len_later, MILESTONE_KINDS);
    }
}
