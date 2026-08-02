use serde_json::{json, Value};

use crate::state::Individual;

pub fn initialize_psychology(individual: &mut Individual) {
    let oxt = individual.phenotype.oxytocin_sensitivity;
    let anxiety = individual.phenotype.anxiety;
    let attachment = if oxt > 0.65 {
        "secure"
    } else if oxt > 0.45 && anxiety > 0.5 {
        "anxious"
    } else if oxt < 0.35 {
        "avoidant"
    } else {
        "secure"
    };
    individual.psychology = crate::types::Psychology {
        mental_state: "content".to_string(),
        wellbeing: 0.6,
        attachment_style: attachment.to_string(),
        stress_level: 0.2,
        trauma_events: vec![],
        relationships: Default::default(),
        theory_of_mind: 0,
        self_awareness: individual.phenotype.fluid_intelligence > 0.6,
        life_satisfaction: 0.6,
        trauma_anxiety: 0.0,
        extra: Default::default(),
    };
}

pub fn update_mental_state(individual: &mut Individual, events: &[Value], world_state: &Value, sim_day: i32) {
    let p = individual.phenotype.clone();
    let ps = &mut individual.psychology;
    let mut stress = ps.stress_level * 0.95;
    let mut wellbeing = ps.wellbeing * 0.98;
    let satiation = individual.extra.get("satiation").and_then(|v| v.as_f64()).unwrap_or(0.5);
    if satiation < 0.3 {
        stress = (stress + 0.1).min(1.0);
        wellbeing = (wellbeing - 0.05).max(0.0);
    } else if satiation > 0.8 {
        wellbeing = (wellbeing + 0.02).min(1.0);
    }
    if individual.group_id.is_none() {
        stress = (stress + p.social_drive * 0.05).min(1.0);
    }
    // A learned/inherited water phobia (AGENTS.md: "avoidance activates when
    // fear > 0.05") manifests as real background anxiety, not just a stored
    // number -- otherwise fearing water would have no felt consequence.
    let water_fear = individual.extra.get("_waterFear").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if water_fear > 0.05 {
        stress = (stress + water_fear * 0.05).min(1.0);
    }
    const TRAUMA_EVENTS_CAP: usize = 50;
    let ps = &mut individual.psychology;
    if world_state.get("recent_disaster").is_some() {
        stress = (stress + 0.3).min(1.0);
        if ps.trauma_events.len() >= TRAUMA_EVENTS_CAP {
            ps.trauma_events.remove(0);
        }
        ps.trauma_events.push(json!({ "type": world_state.get("recent_disaster").cloned().unwrap_or(Value::Null), "day": sim_day }));
    }
    let mut exiled_this_tick = false;
    for ev in events {
        if ev.get("individual_id").and_then(|v| v.as_str()) != Some(individual.id.as_str()) {
            continue;
        }
        match ev.get("type").and_then(|v| v.as_str()) {
            Some("birth") => wellbeing = (wellbeing + 0.1).min(1.0),
            Some("mate_bond") => wellbeing = (wellbeing + 0.15).min(1.0),
            Some("death_of_kin") => {
                stress = (stress + 0.4).min(1.0);
                // Grief is not guaranteed -- it manifests probabilistically based on
                // the individual's own capacity for empathic response.
                let empathy_capacity = (p.oxytocin_sensitivity + p.serotonin) / 2.0;
                if rand::random::<f64>() < empathy_capacity {
                    individual.psychology.mental_state = "grieving".to_string();
                }
                if individual.psychology.trauma_events.len() >= TRAUMA_EVENTS_CAP {
                    individual.psychology.trauma_events.remove(0);
                }
                individual.psychology.trauma_events.push(json!({ "type": "kin_death", "day": sim_day }));
            }
            Some("exile") => {
                stress = (stress + 0.5).min(1.0);
                exiled_this_tick = true;
            }
            Some("discovery") => wellbeing = (wellbeing + 0.2).min(1.0),
            _ => {}
        }
    }
    // Wellbeing's stress-linked baseline term is applied last, against the
    // fully-updated `stress` (satiation/isolation/water-fear/disaster/exile
    // increments all folded in above), not the stale pre-increment value --
    // otherwise wellbeing lagged a full tick behind the stress state it's
    // supposed to be tracking.
    wellbeing = (wellbeing + (1.0 - stress) * 0.02).min(1.0);
    let ps = &individual.psychology;
    let mut mental_state = if stress > 0.7 {
        "anxious"
    } else if wellbeing < 0.2 {
        "depressed"
    } else if wellbeing > 0.8 && stress < 0.2 {
        "excited"
    } else if wellbeing > 0.6 && stress < 0.3 {
        "content"
    } else {
        "calm"
    };
    if ps.mental_state == "grieving" && stress > 0.4 {
        mental_state = "grieving";
    }
    // Exile's "depressed" consequence used to be assigned directly inside the
    // match arm above, then immediately discarded a few lines later by the
    // stress/wellbeing recomputation (exile's own +0.5 stress usually pushed
    // stress past 0.7, which the branch above maps to "anxious" instead) --
    // it almost never actually manifested. Applying it last, the same way
    // "grieving" is preserved above, makes it the one that sticks on the day
    // exile actually happens.
    if exiled_this_tick {
        mental_state = "depressed";
    }
    // Recency-weighted, not a lifetime event count that never lets go: once
    // an individual had ever accumulated more than 3 trauma events, however
    // long ago, trauma_anxiety used to ratchet toward its cap forever (the
    // stored `day` on each event went unread). Only events within the last
    // TRAUMA_RECENCY_WINDOW_DAYS count toward fresh anxiety now, closer to
    // real stress-recovery/habituation dynamics than permanent escalation.
    const TRAUMA_RECENCY_WINDOW_DAYS: i32 = 730;
    let recent_trauma_count = ps
        .trauma_events
        .iter()
        .filter(|e| {
            let event_day = e.get("day").and_then(Value::as_i64).unwrap_or(i64::MIN) as i32;
            sim_day.saturating_sub(event_day) <= TRAUMA_RECENCY_WINDOW_DAYS
        })
        .count();
    let trauma_anxiety = (ps.trauma_anxiety - 0.0005).max(0.0) + if recent_trauma_count > 3 { 0.01 } else { 0.0 };

    individual.psychology.stress_level = stress;
    individual.psychology.wellbeing = wellbeing;
    individual.psychology.mental_state = mental_state.to_string();
    individual.psychology.trauma_anxiety = trauma_anxiety.min(0.7);

    if wellbeing < 0.3 {
        individual.health.hp = (individual.health.hp - 0.003).max(0.0);
    }
    if individual.group_id.is_some() {
        let obs = individual.extra.get("_socialObservations").and_then(|v| v.as_i64()).unwrap_or(0) + 1;
        individual.extra.insert("_socialObservations".to_string(), json!(obs));
    }
    let tom = individual.psychology.theory_of_mind;
    let qi = p.fluid_intelligence;
    let emp = p.empathy;
    let ls = individual.language.stage;
    let c = individual.mind.consciousness;
    let obs = individual.extra.get("_socialObservations").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
    let tom_factor = (qi * emp).max(0.3);
    let new_tom = if tom < 1 && ls >= 1 && qi > 0.3 && obs >= 150.0 / tom_factor {
        1
    } else if tom < 2 && ls >= 2 && c > 0.02 && qi > 0.4 && obs >= 450.0 / tom_factor {
        2
    } else if tom < 3 && ls >= 3 && c > 0.1 && qi > 0.55 && obs >= 1125.0 / tom_factor {
        3
    } else {
        tom
    };
    individual.psychology.theory_of_mind = new_tom;
    individual.psychology.life_satisfaction = (wellbeing + (1.0 - stress)) / 2.0;
}

/// Real social memory is bounded -- nobody meaningfully tracks their
/// relationship with every single person they've ever crossed paths with
/// over decades. Without a cap this map grows forever (every socialize/
/// mate/trade/conflict interaction adds an entry that's never removed, not
/// even for someone who has since died), which was the dominant driver of a
/// real, measured production slowdown: every individual's ever-growing
/// relationships map gets serialized into their DB row on every tick, so
/// upsert cost climbed with elapsed sim-time even at a stable population
/// size. Evicts the weakest (closest-to-neutral) existing relationship to
/// make room for a new one -- strong bonds (positive or negative) are the
/// behaviorally/narratively significant ones, so they're what's kept.
const MAX_TRACKED_RELATIONSHIPS: usize = 40;

fn insert_relationship_bounded(relationships: &mut std::collections::HashMap<String, f64>, id: String, value: f64) {
    if relationships.len() >= MAX_TRACKED_RELATIONSHIPS && !relationships.contains_key(&id) {
        if let Some(weakest_id) = relationships.iter().min_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap_or(std::cmp::Ordering::Equal)).map(|(k, _)| k.clone()) {
            relationships.remove(&weakest_id);
        }
    }
    relationships.insert(id, value);
}

pub fn process_bonding(ind_a: &mut Individual, ind_b: &mut Individual, interaction_type: &str) {
    // Genetic oxytocin_sensitivity (receptor sensitivity, fixed at birth)
    // stays the dominant term; a small share now also comes from each
    // individual's own real, tick-by-tick circulating oxytocin level (see
    // hormones.rs) -- the same hormone/receptor split real oxytocin biology
    // has, layered on top rather than replacing the original formula.
    let genetic_bs = (ind_a.phenotype.oxytocin_sensitivity + ind_b.phenotype.oxytocin_sensitivity) / 2.0;
    let dynamic_bs = (ind_a.hormones.oxytocin + ind_b.hormones.oxytocin) / 2.0;
    let bs = (genetic_bs * 0.8 + dynamic_bs * 0.2).max(0.0);
    let d = match interaction_type {
        "mating" => 0.3,
        "cooperation" => 0.1,
        "play" => 0.08,
        "conflict" => -0.2,
        _ => 0.02,
    };
    let rel_a = ind_a.psychology.relationships.get(&ind_b.id).copied().unwrap_or(0.0);
    let rel_b = ind_b.psychology.relationships.get(&ind_a.id).copied().unwrap_or(0.0);
    insert_relationship_bounded(&mut ind_a.psychology.relationships, ind_b.id.clone(), (rel_a + d * bs).clamp(-1.0, 1.0));
    insert_relationship_bounded(&mut ind_b.psychology.relationships, ind_a.id.clone(), (rel_b + d * bs).clamp(-1.0, 1.0));
}

pub fn compute_population_psych_stats(population: &[Individual], gini: f64) -> Value {
    let living: Vec<&Individual> = population.iter().filter(|i| !i.is_dead).collect();
    if living.is_empty() {
        return json!({ "mean_wellbeing": 0.0, "mean_stress": 0.0, "happiness_index": 0.0 });
    }
    let mw = living.iter().map(|i| i.psychology.wellbeing).sum::<f64>() / living.len() as f64;
    let ms = living.iter().map(|i| i.psychology.stress_level).sum::<f64>() / living.len() as f64;
    let gini_penalty = (gini - 0.30).max(0.0) * 0.5;
    json!({
        "mean_wellbeing": mw,
        "mean_stress": ms,
        "happiness_index": ((mw + (1.0 - ms)) / 2.0 - gini_penalty).clamp(0.0, 1.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trauma_anxiety_accumulates_without_ever_mutating_the_genetic_anxiety_phenotype() {
        let mut ind = Individual {
            phenotype: crate::types::Phenotype { anxiety: 0.3, fluid_intelligence: 0.5, empathy: 0.5, social_drive: 0.5, ..Default::default() },
            language: crate::types::Language { stage: 1, ..Default::default() },
            mind: crate::types::Mind { consciousness: 0.0, ..Default::default() },
            group_id: None,
            ..Default::default()
        };
        initialize_psychology(&mut ind);
        let events = vec![json!({ "individual_id": ind.id, "type": "death_of_kin" })];
        let world = json!({});
        for day in 0..20 {
            update_mental_state(&mut ind, &events, &world, day);
        }
        assert!(ind.psychology.trauma_anxiety > 0.0, "repeated kin-death events should accumulate trauma_anxiety");
        assert_eq!(ind.phenotype.anxiety, 0.3, "phenotype.anxiety is genetic and must never be mutated by lived experience");
    }

    fn make_ind() -> Individual {
        Individual {
            id: "ind1".to_string(),
            phenotype: crate::types::Phenotype {
                fluid_intelligence: 0.6,
                anxiety: 0.3,
                curiosity: 0.5,
                social_drive: 0.5,
                oxytocin_sensitivity: 0.6,
                serotonin: 0.5,
                empathy: 0.5,
                ..Default::default()
            },
            group_id: Some("g1".to_string()),
            language: crate::types::Language { stage: 2, ..Default::default() },
            mind: crate::types::Mind { consciousness: 0.1, ..Default::default() },
            extra: {
                let mut m = serde_json::Map::new();
                m.insert("satiation".to_string(), json!(0.7));
                m
            },
            ..Default::default()
        }
    }

    // ── initializePsychology ────────────────────────────────────────────

    #[test]
    fn initialize_psychology_sets_all_fields() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        assert_eq!(ind.psychology.theory_of_mind, 0);
        assert!(ind.psychology.trauma_events.is_empty());
    }

    #[test]
    fn high_oxytocin_yields_secure_attachment() {
        let mut ind = make_ind();
        ind.phenotype.oxytocin_sensitivity = 0.8;
        ind.phenotype.anxiety = 0.2;
        initialize_psychology(&mut ind);
        assert_eq!(ind.psychology.attachment_style, "secure");
    }

    #[test]
    fn low_oxytocin_yields_avoidant_attachment() {
        let mut ind = make_ind();
        ind.phenotype.oxytocin_sensitivity = 0.2;
        ind.phenotype.anxiety = 0.3;
        initialize_psychology(&mut ind);
        assert_eq!(ind.psychology.attachment_style, "avoidant");
    }

    #[test]
    fn medium_oxytocin_and_high_anxiety_yields_anxious_attachment() {
        let mut ind = make_ind();
        ind.phenotype.oxytocin_sensitivity = 0.5;
        ind.phenotype.anxiety = 0.7;
        initialize_psychology(&mut ind);
        assert_eq!(ind.psychology.attachment_style, "anxious");
    }

    // ── updateMentalState — basic dynamics ───────────────────────────────

    #[test]
    fn stress_decays_five_percent_per_tick() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        ind.psychology.stress_level = 0.8;
        update_mental_state(&mut ind, &[], &json!({}), 1);
        assert!((ind.psychology.stress_level - 0.8 * 0.95).abs() < 1e-9);
    }

    #[test]
    fn low_satiation_raises_stress_and_lowers_wellbeing() {
        let mut ind = make_ind();
        ind.extra.insert("satiation".to_string(), json!(0.1));
        initialize_psychology(&mut ind);
        let (before_stress, before_wb) = (ind.psychology.stress_level, ind.psychology.wellbeing);
        update_mental_state(&mut ind, &[], &json!({}), 1);
        assert!(ind.psychology.stress_level > before_stress);
        assert!(ind.psychology.wellbeing < before_wb);
    }

    #[test]
    fn high_satiation_raises_wellbeing() {
        let mut ind = make_ind();
        ind.extra.insert("satiation".to_string(), json!(0.9));
        initialize_psychology(&mut ind);
        ind.psychology.wellbeing = 0.5;
        update_mental_state(&mut ind, &[], &json!({}), 1);
        assert!(ind.psychology.wellbeing > 0.5);
    }

    #[test]
    fn social_isolation_raises_stress_more_than_group_membership() {
        let mut in_group = make_ind();
        in_group.group_id = Some("g1".to_string());
        let mut alone = make_ind();
        alone.group_id = None;
        initialize_psychology(&mut in_group);
        initialize_psychology(&mut alone);
        in_group.psychology.stress_level = 0.2;
        alone.psychology.stress_level = 0.2;
        update_mental_state(&mut in_group, &[], &json!({}), 1);
        update_mental_state(&mut alone, &[], &json!({}), 1);
        assert!(alone.psychology.stress_level > in_group.psychology.stress_level);
    }

    #[test]
    fn recent_disaster_raises_stress_and_logs_trauma() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        ind.psychology.stress_level = 0.2;
        update_mental_state(&mut ind, &[], &json!({ "recent_disaster": "flood" }), 10);
        assert!(ind.psychology.stress_level > 0.2);
        assert!(ind.psychology.trauma_events.iter().any(|e| e["type"] == "flood"));
    }

    #[test]
    fn high_stress_yields_anxious_mental_state() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        ind.psychology.stress_level = 0.9;
        update_mental_state(&mut ind, &[], &json!({}), 1);
        assert_eq!(ind.psychology.mental_state, "anxious");
    }

    #[test]
    fn low_wellbeing_yields_depressed_mental_state() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        ind.psychology.stress_level = 0.3;
        ind.psychology.wellbeing = 0.1;
        update_mental_state(&mut ind, &[], &json!({}), 1);
        assert_eq!(ind.psychology.mental_state, "depressed");
    }

    #[test]
    fn high_wellbeing_and_low_stress_yields_excited_mental_state() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        ind.psychology.stress_level = 0.1;
        ind.psychology.wellbeing = 0.95;
        update_mental_state(&mut ind, &[], &json!({}), 1);
        assert_eq!(ind.psychology.mental_state, "excited");
    }

    // ── updateMentalState — events ────────────────────────────────────────

    #[test]
    fn birth_event_raises_wellbeing() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        let before = ind.psychology.wellbeing;
        update_mental_state(&mut ind, &[json!({ "type": "birth", "individual_id": "ind1" })], &json!({}), 1);
        assert!(ind.psychology.wellbeing > before);
    }

    #[test]
    fn death_of_kin_raises_stress_and_logs_trauma() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        let before = ind.psychology.stress_level;
        update_mental_state(&mut ind, &[json!({ "type": "death_of_kin", "individual_id": "ind1" })], &json!({}), 1);
        assert!(ind.psychology.stress_level > before);
        assert!(!ind.psychology.trauma_events.is_empty());
    }

    #[test]
    fn exile_event_raises_stress_substantially_and_leaves_the_individual_depressed() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        ind.psychology.stress_level = 0.1;
        update_mental_state(&mut ind, &[json!({ "type": "exile", "individual_id": "ind1" })], &json!({}), 1);
        // 0.1 * 0.95 + 0.5 ~= 0.595.
        assert!(ind.psychology.stress_level > 0.5);
        // H-05 regression: exile's "depressed" consequence used to be applied
        // then immediately overwritten by the stress/wellbeing recomputation a
        // few lines later (elevated stress alone maps to "anxious"). It's now
        // applied last, so it actually sticks on the day exile happens.
        assert_eq!(ind.psychology.mental_state, "depressed");
    }

    #[test]
    fn h06_regression_old_trauma_stops_driving_fresh_anxiety_once_it_falls_out_of_the_recency_window() {
        // Before the H-06 fix, trauma_anxiety escalated forever once an
        // individual had ever accumulated more than 3 lifetime trauma events,
        // no matter how long ago -- the stored `day` on each event went
        // unread. Rack up 4 events early (days 0-3), let a lot of calm time
        // pass, then confirm trauma_anxiety is decaying back down instead of
        // still climbing toward its cap.
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        let id = ind.id.clone();
        for day in 0..4 {
            update_mental_state(&mut ind, &[json!({ "individual_id": id, "type": "death_of_kin" })], &json!({}), day);
        }
        assert_eq!(ind.psychology.trauma_events.len(), 4);

        // While all 4 events are still within the recency window, anxiety
        // should climb toward (and cap at) 0.7 -- same escalation as before
        // the fix, just bounded by recency instead of being permanent.
        for day in 4..200 {
            update_mental_state(&mut ind, &[], &json!({}), day);
        }
        let anxiety_at_peak = ind.psychology.trauma_anxiety;
        assert!(anxiety_at_peak > 0.6, "trauma_anxiety should have climbed toward its cap while the events were still recent, got {anxiety_at_peak}");

        // Once well past the 730-day recency window, those same events must
        // no longer count as "recent trauma" -- anxiety should now be
        // decaying back down instead of staying pinned at its peak forever.
        for day in 200..2000 {
            update_mental_state(&mut ind, &[], &json!({}), day);
        }
        assert!(
            ind.psychology.trauma_anxiety < anxiety_at_peak,
            "trauma_anxiety should have decayed once those events aged out of the recency window, got {} (was {anxiety_at_peak})",
            ind.psychology.trauma_anxiety
        );
    }

    #[test]
    fn trauma_events_never_exceed_fifty_entries() {
        let mut ind = make_ind();
        initialize_psychology(&mut ind);
        for day in 0..60 {
            update_mental_state(&mut ind, &[], &json!({ "recent_disaster": "flood" }), day);
        }
        assert!(ind.psychology.trauma_events.len() <= 50);
    }

    // ── Theory of Mind ──────────────────────────────────────────────────

    #[test]
    fn social_observations_accumulate_only_within_a_group() {
        let mut ind = make_ind();
        ind.group_id = Some("g1".to_string());
        initialize_psychology(&mut ind);
        update_mental_state(&mut ind, &[], &json!({}), 1);
        assert_eq!(ind.extra.get("_socialObservations").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn social_observations_do_not_accumulate_outside_a_group() {
        let mut ind = make_ind();
        ind.group_id = None;
        initialize_psychology(&mut ind);
        update_mental_state(&mut ind, &[], &json!({}), 1);
        assert_eq!(ind.extra.get("_socialObservations"), None);
    }

    #[test]
    fn sufficient_observations_and_capability_reach_theory_of_mind_stage_1() {
        let mut ind = make_ind();
        ind.group_id = Some("g1".to_string());
        ind.language.stage = 2;
        ind.mind.consciousness = 0.05;
        ind.phenotype.fluid_intelligence = 0.8;
        ind.phenotype.empathy = 0.8;
        ind.extra.insert("_socialObservations".to_string(), json!(9999));
        initialize_psychology(&mut ind);
        ind.psychology.theory_of_mind = 0;
        update_mental_state(&mut ind, &[], &json!({}), 1);
        assert!(ind.psychology.theory_of_mind >= 1);
    }

    // ── processBonding ───────────────────────────────────────────────────

    #[test]
    fn mating_interaction_strengthens_relationship_for_both_parties() {
        let mut a = make_ind();
        a.id = "a".to_string();
        let mut b = make_ind();
        b.id = "b".to_string();
        initialize_psychology(&mut a);
        initialize_psychology(&mut b);
        process_bonding(&mut a, &mut b, "mating");
        assert!(a.psychology.relationships["b"] > 0.0);
        assert!(b.psychology.relationships["a"] > 0.0);
    }

    #[test]
    fn conflict_interaction_weakens_relationship() {
        let mut a = make_ind();
        a.id = "a".to_string();
        let mut b = make_ind();
        b.id = "b".to_string();
        initialize_psychology(&mut a);
        initialize_psychology(&mut b);
        process_bonding(&mut a, &mut b, "conflict");
        assert!(a.psychology.relationships["b"] < 0.0);
    }

    #[test]
    fn mating_forms_a_stronger_bond_than_cooperation() {
        let (mut a1, mut b1) = (make_ind(), make_ind());
        a1.id = "a1".to_string();
        b1.id = "b1".to_string();
        let (mut a2, mut b2) = (make_ind(), make_ind());
        a2.id = "a2".to_string();
        b2.id = "b2".to_string();
        initialize_psychology(&mut a1);
        initialize_psychology(&mut b1);
        initialize_psychology(&mut a2);
        initialize_psychology(&mut b2);
        process_bonding(&mut a1, &mut b1, "mating");
        process_bonding(&mut a2, &mut b2, "cooperation");
        assert!(a1.psychology.relationships["b1"] > a2.psychology.relationships["b2"]);
    }

    #[test]
    fn relationship_value_stays_within_bounds() {
        let mut a = make_ind();
        a.id = "a".to_string();
        let mut b = make_ind();
        b.id = "b".to_string();
        initialize_psychology(&mut a);
        initialize_psychology(&mut b);
        for _ in 0..100 {
            process_bonding(&mut a, &mut b, "mating");
        }
        assert!(a.psychology.relationships["b"] <= 1.0);
        assert!(a.psychology.relationships["b"] >= -1.0);
    }

    #[test]
    fn relationships_never_grow_past_the_cap() {
        let mut a = make_ind();
        a.id = "a".to_string();
        initialize_psychology(&mut a);
        for i in 0..(MAX_TRACKED_RELATIONSHIPS + 25) {
            let mut other = make_ind();
            other.id = format!("other-{i}");
            initialize_psychology(&mut other);
            process_bonding(&mut a, &mut other, "socialize");
        }
        assert!(a.psychology.relationships.len() <= MAX_TRACKED_RELATIONSHIPS);
    }

    #[test]
    fn a_strong_existing_bond_survives_eviction_pressure_from_many_weak_new_ones() {
        let mut a = make_ind();
        a.id = "a".to_string();
        initialize_psychology(&mut a);
        let mut best_friend = make_ind();
        best_friend.id = "best-friend".to_string();
        initialize_psychology(&mut best_friend);
        for _ in 0..50 {
            process_bonding(&mut a, &mut best_friend, "mating");
        }
        let strong_bond = a.psychology.relationships["best-friend"];
        assert!(strong_bond > 0.9);

        for i in 0..(MAX_TRACKED_RELATIONSHIPS + 50) {
            let mut stranger = make_ind();
            stranger.id = format!("stranger-{i}");
            initialize_psychology(&mut stranger);
            process_bonding(&mut a, &mut stranger, "socialize");
        }

        assert_eq!(a.psychology.relationships["best-friend"], strong_bond, "the strongest relationship should never be the one evicted");
        assert!(a.psychology.relationships.len() <= MAX_TRACKED_RELATIONSHIPS);
    }

    // ── computePopulationPsychStats ──────────────────────────────────────

    #[test]
    fn empty_population_yields_zero_statistics() {
        let stats = compute_population_psych_stats(&[], 0.0);
        assert_eq!(stats["happiness_index"], 0.0);
    }

    #[test]
    fn dead_individuals_are_excluded_from_stats() {
        let mut dead = make_ind();
        dead.id = "dead".to_string();
        dead.is_dead = true;
        initialize_psychology(&mut dead);
        dead.psychology.wellbeing = 0.0;

        let mut alive = make_ind();
        alive.id = "alive".to_string();
        initialize_psychology(&mut alive);
        alive.psychology.wellbeing = 1.0;

        let stats = compute_population_psych_stats(&[dead, alive], 0.0);
        assert!((stats["mean_wellbeing"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn high_gini_lowers_happiness_index() {
        let mut a = make_ind();
        a.id = "a".to_string();
        let mut b = make_ind();
        b.id = "b".to_string();
        initialize_psychology(&mut a);
        initialize_psychology(&mut b);
        let pop = [a, b];
        let low = compute_population_psych_stats(&pop, 0.10);
        let high = compute_population_psych_stats(&pop, 0.60);
        assert!(high["happiness_index"].as_f64().unwrap() < low["happiness_index"].as_f64().unwrap());
    }

    #[test]
    fn happiness_index_stays_within_zero_to_one() {
        let mut a = make_ind();
        initialize_psychology(&mut a);
        a.psychology.stress_level = 1.0;
        a.psychology.wellbeing = 0.0;
        let stats = compute_population_psych_stats(&[a], 0.9);
        let h = stats["happiness_index"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&h));
    }
}
