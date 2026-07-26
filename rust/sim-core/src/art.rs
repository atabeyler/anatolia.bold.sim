use std::collections::HashSet;

use serde_json::{json, Value};

#[allow(clippy::type_complexity)]
pub const ART_FORMS: &[(&str, &str, f64, f64, &[&str], f64, Option<f64>)] = &[
    ("cave_painting", "visual", 0.35, 0.3, &[], 0.3, None),
    ("sculpture", "visual", 0.45, 0.4, &["stone_tools"], 0.4, None),
    ("pottery_decoration", "visual", 0.5, 0.4, &["pottery"], 0.3, None),
    ("textile_pattern", "visual", 0.5, 0.45, &["weaving"], 0.35, None),
    ("architecture_art", "visual", 0.6, 0.5, &["architecture_stone"], 0.5, None),
    ("rhythmic_percussion", "music", 0.25, 0.2, &[], 0.2, None),
    ("vocal_melody", "music", 0.3, 0.25, &[], 0.2, None),
    ("flute_bone", "music", 0.4, 0.35, &["stone_tools"], 0.3, None),
    ("string_instrument", "music", 0.5, 0.4, &["hunting_spear"], 0.4, None),
    ("oral_story", "narrative", 0.4, 0.3, &[], 0.25, Some(0.45)),
    ("epic_poem", "narrative", 0.55, 0.5, &[], 0.4, Some(0.6)),
    ("written_story", "narrative", 0.65, 0.5, &["writing_system"], 0.5, Some(0.65)),
];

fn art_description(art_id: &str) -> &str {
    match art_id {
        "cave_painting" => "Pigments applied to rock surfaces depict animals and figures",
        "sculpture" => "Three-dimensional forms carved from stone or bone",
        "pottery_decoration" => "Geometric and figurative patterns adorn ceramic surfaces",
        "textile_pattern" => "Woven cloth bears complex repeating patterns",
        "architecture_art" => "Buildings are decorated with carved reliefs and motifs",
        "rhythmic_percussion" => "Stones and bones struck together in rhythmic patterns",
        "vocal_melody" => "Sustained pitched vocalizations form melodic sequences",
        "flute_bone" => "A hollow bone with finger holes produces musical tones",
        "string_instrument" => "A taut cord vibrates to produce musical notes",
        "oral_story" => "Narrative accounts passed between individuals by spoken word",
        "epic_poem" => "Long rhythmic verse recounts heroic deeds and origins",
        "written_story" => "Narrative accounts preserved in written symbols",
        other => other,
    }
}

pub fn process_art_tick(
    population: &[crate::state::Individual],
    discovered_arts: &mut HashSet<String>,
    discovered_techs: &HashSet<String>,
    world_state: &Value,
    sim_day: i32,
) -> Vec<Value> {
    let mut events = Vec::new();
    let surplus = world_state.get("food_abundance").and_then(Value::as_f64).unwrap_or(0.5);
    for individual in population.iter().filter(|i| !i.is_dead) {
        let life_stage = crate::biology::individual::get_life_stage(individual, sim_day);
        if life_stage == "infant" || life_stage == "child" {
            continue;
        }
        let p = &individual.phenotype;
        let artistic = p.artistic_sense;
        let foxp2 = individual.language.foxp2_expression;
        let action = individual.extra.get("_currentAction").and_then(Value::as_str).unwrap_or("");
        for (art_id, medium, iq_min, artistic_min, requires_tech, surplus_min, foxp2_min) in ART_FORMS {
            if discovered_arts.contains(*art_id)
                || p.fluid_intelligence < *iq_min
                || artistic < *artistic_min
                || surplus < *surplus_min
                || requires_tech.iter().any(|t| !discovered_techs.contains(*t))
                || foxp2_min.is_some_and(|min| foxp2 < min)
                || action != match *medium { "visual" => "craft", "music" => "socialize", _ => "socialize" }
            {
                continue;
            }
            if rand::random::<f64>() < artistic * p.fluid_intelligence * surplus / 5000.0 {
                discovered_arts.insert((*art_id).to_string());
                events.push(json!({
                    "type": "art_created",
                    "art_id": art_id,
                    "medium": medium,
                    "creator_id": individual.id,
                    "day": sim_day,
                    "importance": if *iq_min > 0.5 { "high" } else { "medium" },
                    "description": art_description(art_id),
                }));
            }
        }
    }
    events
}

pub fn apply_art_effects(individual: &mut crate::state::Individual, group: Option<&mut Value>, discovered_arts: &HashSet<String>) {
    if discovered_arts.is_empty() {
        return;
    }
    // The ambient wellbeing bonus is scaled by the population's total art
    // discoveries (a global, not per-group, tally -- art forms aren't
    // re-invented independently the way tech/belief/culture propagation
    // requires observation of a specific holder), but it's still gated on
    // actually belonging to a group: a truly solitary individual who has
    // never been part of any society has nothing to have observed this
    // civilizational richness through, so they shouldn't get the same boost
    // as everyone actually living in it.
    if individual.group_id.is_some() {
        individual.psychology.wellbeing = (individual.psychology.wellbeing + discovered_arts.len() as f64 * 0.00005).min(1.0);
    }
    if discovered_arts.len() > 3 {
        if let Some(group) = group {
            let tension = group.get("internal_tension").and_then(Value::as_f64).unwrap_or(0.5);
            if let Some(obj) = group.as_object_mut() {
                obj.insert("internal_tension".to_string(), json!((tension - 0.01).max(0.0)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Individual;

    fn artistic_individual(action: &str) -> Individual {
        Individual {
            birth_day: -25 * 365, // adult; life-stage gating is a separate test below
            group_id: Some("g1".to_string()),
            phenotype: crate::types::Phenotype { fluid_intelligence: 0.9, artistic_sense: 0.9, ..Default::default() },
            language: crate::types::Language { foxp2_expression: 0.9, ..Default::default() },
            extra: {
                let mut m = serde_json::Map::new();
                m.insert("_currentAction".to_string(), json!(action));
                m
            },
            ..Default::default()
        }
    }

    #[test]
    fn no_visual_art_emerges_without_a_matching_craft_activity() {
        // Cardinal rule: art must arise from what the individual actually does
        // (a "craft" action for visual art), never from a trait scan alone.
        let population = vec![artistic_individual("socialize")]; // high artistic_sense, wrong action
        let mut discovered = HashSet::new();
        let techs = HashSet::new();
        let world = json!({ "food_abundance": 1.0 });
        for day in 0..3000 {
            let evs = process_art_tick(&population, &mut discovered, &techs, &world, day);
            assert!(evs.is_empty() || evs.iter().all(|e| e["medium"] != "visual"));
        }
        assert!(!discovered.contains("cave_painting"));
    }

    #[test]
    fn visual_art_can_emerge_once_the_individual_is_actually_crafting() {
        let population = vec![artistic_individual("craft")];
        let mut discovered = HashSet::new();
        let techs = HashSet::new();
        let world = json!({ "food_abundance": 1.0 });
        let mut emerged = false;
        for day in 0..200_000 {
            let evs = process_art_tick(&population, &mut discovered, &techs, &world, day);
            if evs.iter().any(|e| e["art_id"] == "cave_painting") {
                emerged = true;
                break;
            }
        }
        assert!(emerged, "a highly artistic individual actually crafting should eventually produce cave_painting");
    }

    // ── ART_FORMS definitions ────────────────────────────────────────────

    #[test]
    fn defines_twelve_art_forms() {
        assert_eq!(ART_FORMS.len(), 12);
    }

    #[test]
    fn rhythmic_percussion_has_the_lowest_iq_requirement() {
        let min = ART_FORMS.iter().map(|(_, _, iq, ..)| *iq).fold(f64::INFINITY, f64::min);
        let percussion = ART_FORMS.iter().find(|(id, ..)| *id == "rhythmic_percussion").unwrap().2;
        assert_eq!(percussion, min);
    }

    #[test]
    fn written_story_requires_writing_system() {
        let (_, _, _, _, tech, ..) = ART_FORMS.iter().find(|(id, ..)| *id == "written_story").unwrap();
        assert!(tech.contains(&"writing_system"));
    }

    #[test]
    fn at_least_one_visual_art_form_exists() {
        assert!(ART_FORMS.iter().any(|(_, medium, ..)| *medium == "visual"));
    }

    #[test]
    fn narrative_forms_all_define_a_positive_foxp2_min() {
        for (_, medium, _, _, _, _, foxp2_min) in ART_FORMS {
            if *medium == "narrative" {
                assert!(foxp2_min.is_some_and(|m| m > 0.0));
            }
        }
    }

    // ── processArtTick discovery gating ─────────────────────────────────

    #[test]
    fn nothing_new_emerges_once_every_form_is_already_discovered() {
        let ind = artistic_individual("craft");
        let all: HashSet<String> = ART_FORMS.iter().map(|(id, ..)| id.to_string()).collect();
        let mut discovered = all;
        let events = process_art_tick(&[ind], &mut discovered, &HashSet::new(), &json!({ "food_abundance": 0.9 }), 1);
        assert!(events.is_empty());
    }

    #[test]
    fn infants_and_children_never_create_art() {
        let infant = Individual { birth_day: 0, ..artistic_individual("craft") };
        let child = Individual { birth_day: -5 * 365, ..artistic_individual("craft") };
        let events = process_art_tick(&[infant, child], &mut HashSet::new(), &HashSet::new(), &json!({ "food_abundance": 0.9 }), 1);
        assert!(events.is_empty());
    }

    #[test]
    fn low_food_surplus_prevents_any_art_discovery() {
        let ind = artistic_individual("craft");
        let mut found = false;
        for day in 0..1000 {
            let evs = process_art_tick(std::slice::from_ref(&ind), &mut HashSet::new(), &HashSet::new(), &json!({ "food_abundance": 0.1 }), day);
            if !evs.is_empty() {
                found = true;
                break;
            }
        }
        assert!(!found);
    }

    #[test]
    fn art_requiring_a_tech_is_never_discovered_without_it() {
        let ind = artistic_individual("craft");
        let mut discovered = HashSet::new();
        for day in 0..5000 {
            process_art_tick(std::slice::from_ref(&ind), &mut discovered, &HashSet::new(), &json!({ "food_abundance": 0.9 }), day);
        }
        assert!(!discovered.contains("sculpture")); // requires stone_tools
    }

    #[test]
    fn sculpture_is_discoverable_once_stone_tools_is_known() {
        let ind = artistic_individual("craft");
        let mut techs = HashSet::new();
        techs.insert("stone_tools".to_string());
        let mut discovered = HashSet::new();
        let mut found = false;
        for day in 0..100_000 {
            process_art_tick(std::slice::from_ref(&ind), &mut discovered, &techs, &json!({ "food_abundance": 0.9 }), day);
            if discovered.contains("sculpture") {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn art_created_event_has_expected_shape() {
        let ind = Individual {
            id: "artist-1".to_string(),
            ..artistic_individual("socialize")
        };
        let mut event = None;
        for day in 0..100_000 {
            let mut discovered = HashSet::new();
            let evs = process_art_tick(std::slice::from_ref(&ind), &mut discovered, &HashSet::new(), &json!({ "food_abundance": 0.9 }), day);
            if let Some(ev) = evs.into_iter().next() {
                event = Some(ev);
                break;
            }
        }
        let ev = event.expect("expected an art_created event within 100000 days");
        assert_eq!(ev["type"], "art_created");
        assert_eq!(ev["creator_id"], "artist-1");
        assert!(ev["art_id"].is_string());
        assert!(ev["medium"].is_string());
    }

    // ── applyArtEffects ──────────────────────────────────────────────────

    #[test]
    fn no_wellbeing_change_when_nothing_is_discovered() {
        let mut ind = artistic_individual("craft");
        let before = ind.psychology.wellbeing;
        apply_art_effects(&mut ind, None, &HashSet::new());
        assert_eq!(ind.psychology.wellbeing, before);
    }

    #[test]
    fn known_art_forms_raise_wellbeing() {
        let mut ind = artistic_individual("craft");
        let before = ind.psychology.wellbeing;
        let known: HashSet<String> = ["rhythmic_percussion", "vocal_melody", "cave_painting"].iter().map(|s| s.to_string()).collect();
        apply_art_effects(&mut ind, None, &known);
        assert!(ind.psychology.wellbeing > before);
    }

    #[test]
    fn wellbeing_never_exceeds_one() {
        let mut ind = artistic_individual("craft");
        ind.psychology.wellbeing = 0.9999;
        let all: HashSet<String> = ART_FORMS.iter().map(|(id, ..)| id.to_string()).collect();
        for _ in 0..1000 {
            apply_art_effects(&mut ind, None, &all);
        }
        assert!(ind.psychology.wellbeing <= 1.0);
    }

    #[test]
    fn a_solitary_individual_with_no_group_gets_no_ambient_wellbeing_bonus() {
        // A truly solitary individual has never been part of any society and
        // has nothing to have observed this civilizational richness through.
        let mut ind = Individual { group_id: None, ..artistic_individual("craft") };
        let before = ind.psychology.wellbeing;
        let known: HashSet<String> = ["rhythmic_percussion", "vocal_melody", "cave_painting"].iter().map(|s| s.to_string()).collect();
        apply_art_effects(&mut ind, None, &known);
        assert_eq!(ind.psychology.wellbeing, before);
    }

    #[test]
    fn four_or_more_art_forms_reduce_group_tension() {
        let mut ind = artistic_individual("craft");
        let mut group = json!({ "internal_tension": 0.5 });
        let four: HashSet<String> = ["rhythmic_percussion", "vocal_melody", "cave_painting", "oral_story"].iter().map(|s| s.to_string()).collect();
        apply_art_effects(&mut ind, Some(&mut group), &four);
        assert!(group["internal_tension"].as_f64().unwrap() < 0.5);
    }

    #[test]
    fn fewer_than_four_art_forms_do_not_reduce_group_tension() {
        let mut ind = artistic_individual("craft");
        let mut group = json!({ "internal_tension": 0.5 });
        let mut one = HashSet::new();
        one.insert("rhythmic_percussion".to_string());
        apply_art_effects(&mut ind, Some(&mut group), &one);
        assert_eq!(group["internal_tension"], 0.5);
    }
}
