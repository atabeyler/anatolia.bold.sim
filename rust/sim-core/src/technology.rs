use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::state::Individual;

use serde_json::json;

#[allow(clippy::type_complexity)]
pub const TECH_TREE: &[(&str, i32, &[&str], f64, f64, &str, Option<i32>)] = &[
    ("fire_making", 0, &[], 0.8, 0.3, "any", None),
    ("stone_tools", 0, &[], 0.6, 0.25, "any", None),
    ("foraging", 0, &[], 0.3, 0.2, "any", None),
    ("hunting_spear", 1, &["stone_tools"], 1.2, 0.35, "fauna_present", None),
    ("shelter_basic", 1, &[], 0.9, 0.3, "cold_or_rain", None),
    ("water_container", 1, &["stone_tools"], 1.0, 0.3, "water_need", None),
    ("animal_trap", 1, &["stone_tools"], 1.3, 0.4, "fauna_present", None),
    // requires stone_tools to match agent.rs's TECH_SKILLS entry for the
    // same tech (direct invention already required it there) -- without
    // this, learn_tech_from_observation let an individual pick up
    // clothing_basic from a nearby peer without ever knowing stone_tools
    // themselves, bypassing a prerequisite direct invention enforced.
    ("clothing_basic", 1, &["stone_tools"], 1.1, 0.3, "cold", None),
    ("swimming", 1, &[], 1.0, 0.25, "water_nearby", None),
    ("fishing", 2, &["stone_tools"], 1.4, 0.4, "water_nearby", None),
    ("plant_cultivation", 2, &["foraging"], 2.0, 0.5, "seasonal_plants", None),
    ("animal_herding", 2, &["animal_trap"], 2.5, 0.55, "herdable_animals", None),
    ("food_preservation", 2, &["fire_making"], 1.8, 0.45, "any", None),
    ("bow_arrow", 2, &["hunting_spear"], 2.2, 0.5, "any", None),
    ("pottery", 3, &["plant_cultivation", "fire_making"], 3.0, 0.55, "clay_nearby", None),
    ("weaving", 3, &["clothing_basic"], 2.8, 0.5, "plant_fibers", None),
    ("metallurgy_copper", 3, &["fire_making", "stone_tools"], 4.0, 0.6, "copper_ore", None),
    ("writing_system", 3, &["pottery"], 5.0, 0.7, "trade_need", Some(5)),
    ("calendar", 3, &["plant_cultivation"], 3.5, 0.6, "any", Some(4)),
    ("mathematics_basic", 3, &["writing_system"], 4.5, 0.65, "any", None),
    ("architecture_stone", 4, &["metallurgy_copper"], 5.5, 0.65, "stone_available", None),
    ("wheel", 4, &["metallurgy_copper"], 5.0, 0.65, "any", None),
    ("irrigation", 4, &["plant_cultivation", "wheel"], 5.5, 0.65, "river_nearby", None),
    ("sailing", 4, &["fishing", "wheel"], 5.5, 0.65, "coastal_or_river", None),
    ("metallurgy_iron", 4, &["metallurgy_copper"], 6.0, 0.7, "iron_ore", None),
];

// Maps tech_id -> its index in TECH_TREE, built once and reused for the
// lifetime of the process. learn_tech_from_observation below runs this
// lookup for every (nearby teacher, their known tech) pair for every living
// individual every tick -- a linear TECH_TREE.iter().find() there meant
// O(nearby * their_known_techs * TECH_TREE.len()) string comparisons per
// tick, compounding as known_techs grows toward the full tree over a long
// simulation.
pub(crate) fn tech_index() -> &'static HashMap<&'static str, usize> {
    static INDEX: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();
    INDEX.get_or_init(|| TECH_TREE.iter().enumerate().map(|(i, entry)| (entry.0, i)).collect())
}

// Takes references, not owned Individuals: this is read-only over `nearby`
// (only `individual` is ever mutated), so the caller (tick.rs) collecting
// full clones into `nearby` for every alive individual every tick -- up to
// MAX_NEARBY_SAMPLE clones each, of an observation_stub whose known_techs/
// vocabulary still grow across the run -- was pure waste on top of the
// O(n) scans/allocations fixed above.
pub fn learn_tech_from_observation(individual: &mut Individual, nearby: &[&Individual], discovered_techs: &mut HashSet<String>) {
    let index = tech_index();
    // Built once per learner rather than doing an O(known_techs.len()) Vec
    // scan (`.contains()`, plus a per-check String allocation for every
    // prerequisite) for every single candidate tech from every nearby
    // teacher. Updated below on an actual learn so same-tick prerequisite
    // chaining still works exactly as it did when this read the live Vec.
    let mut known: HashSet<String> = individual.known_techs.iter().cloned().collect();
    for other in nearby.iter().copied() {
        if other.id == individual.id {
            continue;
        }
        for tech_id in &other.known_techs {
            if known.contains(tech_id) {
                continue;
            }
            let Some(&i) = index.get(tech_id.as_str()) else { continue };
            let (_, _, requires, difficulty, iq_min, _, lang_min) = &TECH_TREE[i];
            if individual.phenotype.fluid_intelligence < *iq_min {
                continue;
            }
            // Mirrors agent.rs::check_tech_emergence's own lang_min gate for
            // direct invention -- without it, a pre-linguistic (stage 0)
            // individual could pick up writing_system (which required
            // lang_stage >= 5 for whoever actually invented it) purely by
            // observing a teacher, bypassing the language-stage prerequisite
            // entirely.
            if let Some(min) = lang_min {
                if individual.language.stage < *min {
                    continue;
                }
            }
            if !requires.iter().all(|r| known.contains(*r)) {
                continue;
            }
            let rate = (individual.phenotype.curiosity * individual.phenotype.fluid_intelligence * (0.5 + individual.phenotype.learning_rate * 0.5)) / (difficulty * 2000.0);
            if rand::random::<f64>() < rate {
                individual.known_techs.push(tech_id.clone());
                discovered_techs.insert(tech_id.clone());
                known.insert(tech_id.clone());
            }
        }
    }
}

pub fn known_techs_json(individual: &Individual) -> serde_json::Value {
    json!(individual.known_techs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capable_individual(id: &str, techs: &[&str]) -> Individual {
        Individual {
            id: id.to_string(),
            known_techs: techs.iter().map(|s| s.to_string()).collect(),
            phenotype: crate::types::Phenotype { fluid_intelligence: 0.9, curiosity: 0.9, learning_rate: 0.9, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn an_individual_never_learns_a_tech_that_no_nearby_peer_knows() {
        // Cardinal rule: tech can only be learned by observing a nearby peer who
        // personally knows it. With no nearby individuals at all, nothing may appear.
        let mut learner = capable_individual("solo", &[]);
        let mut discovered = HashSet::new();
        for _ in 0..1000 {
            learn_tech_from_observation(&mut learner, &[], &mut discovered);
        }
        assert!(learner.known_techs.is_empty());
        assert!(discovered.is_empty());
    }

    #[test]
    fn cannot_learn_technology_if_iq_is_insufficient() {
        // metallurgy_copper iq_min=0.6; learner IQ=0.3
        let mut learner = Individual {
            id: "learner".to_string(),
            known_techs: vec!["fire_making".to_string(), "stone_tools".to_string()],
            phenotype: crate::types::Phenotype { fluid_intelligence: 0.3, curiosity: 1.0, ..Default::default() },
            ..Default::default()
        };
        let teacher = capable_individual("teacher", &["metallurgy_copper"]);
        let mut discovered = HashSet::new();
        for _ in 0..500 {
            learn_tech_from_observation(&mut learner, &[&teacher], &mut discovered);
        }
        assert!(!learner.known_techs.contains(&"metallurgy_copper".to_string()));
    }

    #[test]
    fn does_not_add_already_known_technology_again() {
        let teacher = capable_individual("teacher", &["foraging"]);
        let mut learner = capable_individual("learner", &["foraging"]);
        let before = learner.known_techs.len();
        let mut discovered = HashSet::new();
        for _ in 0..100 {
            learn_tech_from_observation(&mut learner, &[&teacher], &mut discovered);
        }
        assert_eq!(learner.known_techs.len(), before);
    }

    #[test]
    fn a_prerequisite_learned_this_same_call_unlocks_its_dependent_tech() {
        // Regression test for the `known` HashSet optimization in
        // learn_tech_from_observation: it must stay in sync with
        // individual.known_techs within a single call, the same way the
        // original per-check Vec scan naturally did, so learning a
        // prerequisite from one teacher can still unlock a tech that
        // requires it from another teacher within the very same call.
        let mut learner = Individual {
            id: "learner".to_string(),
            known_techs: vec![],
            phenotype: crate::types::Phenotype { fluid_intelligence: 0.9, curiosity: 1.0, learning_rate: 1.0, ..Default::default() },
            ..Default::default()
        };
        let stone_tools_teacher = capable_individual("teacher_a", &["stone_tools"]);
        // hunting_spear requires stone_tools.
        let hunting_spear_teacher = capable_individual("teacher_b", &["hunting_spear"]);
        let nearby = [&stone_tools_teacher, &hunting_spear_teacher];
        let mut discovered = HashSet::new();
        for _ in 0..30_000 {
            learn_tech_from_observation(&mut learner, &nearby, &mut discovered);
            if learner.known_techs.contains(&"hunting_spear".to_string()) {
                break;
            }
        }
        assert!(learner.known_techs.contains(&"stone_tools".to_string()));
        assert!(learner.known_techs.contains(&"hunting_spear".to_string()), "hunting_spear should eventually unlock once stone_tools is learned, chained within a call if needed");
    }

    #[test]
    fn learns_nothing_when_no_nearby_peer_exists() {
        let mut learner = capable_individual("learner", &[]);
        let mut discovered = HashSet::new();
        learn_tech_from_observation(&mut learner, &[], &mut discovered);
        assert!(learner.known_techs.is_empty());
    }

    #[test]
    fn cannot_learn_a_language_gated_technology_via_observation_below_its_language_stage() {
        // writing_system requires lang_stage >= 5 (see TECH_TREE). Direct
        // invention (agent::check_tech_emergence) already enforces this;
        // observation learning previously discarded lang_min entirely,
        // letting even a pre-linguistic (stage 0) individual pick up
        // writing_system purely by watching a teacher who invented it.
        let mut learner = Individual {
            id: "learner".to_string(),
            known_techs: vec!["pottery".to_string()],
            phenotype: crate::types::Phenotype { fluid_intelligence: 0.95, curiosity: 1.0, learning_rate: 1.0, ..Default::default() },
            ..Default::default()
        };
        assert_eq!(learner.language.stage, 0, "sanity check: a default individual is pre-linguistic");
        let teacher = capable_individual("teacher", &["writing_system"]);
        let mut discovered = HashSet::new();
        for _ in 0..5000 {
            learn_tech_from_observation(&mut learner, &[&teacher], &mut discovered);
        }
        assert!(!learner.known_techs.contains(&"writing_system".to_string()), "a pre-linguistic individual must never learn writing_system by observation alone");
    }

    #[test]
    fn can_still_learn_a_language_gated_technology_via_observation_once_the_stage_is_met() {
        let mut learner = Individual {
            id: "learner".to_string(),
            known_techs: vec!["pottery".to_string()],
            phenotype: crate::types::Phenotype { fluid_intelligence: 0.95, curiosity: 1.0, learning_rate: 1.0, ..Default::default() },
            language: crate::types::Language { stage: 5, ..Default::default() },
            ..Default::default()
        };
        let teacher = capable_individual("teacher", &["writing_system"]);
        let mut discovered = HashSet::new();
        // writing_system's difficulty (5.0) makes the per-tick learn rate tiny
        // even at max curiosity/IQ/learning_rate (~0.0000475 with these
        // stats), so this needs many more trials than the cheaper techs
        // exercised elsewhere in this file to reliably observe a success.
        for _ in 0..200_000 {
            learn_tech_from_observation(&mut learner, &[&teacher], &mut discovered);
            if learner.known_techs.contains(&"writing_system".to_string()) {
                break;
            }
        }
        assert!(
            learner.known_techs.contains(&"writing_system".to_string()),
            "once language stage >= 5 is met, observation learning should still be able to pick up writing_system"
        );
    }

    #[test]
    fn individual_is_ignored_if_they_appear_in_their_own_peer_list() {
        let mut learner = capable_individual("solo", &[]);
        let self_ref = capable_individual("solo", &["foraging"]);
        let mut discovered = HashSet::new();
        for _ in 0..200 {
            learn_tech_from_observation(&mut learner, &[&self_ref], &mut discovered);
        }
        assert!(!learner.known_techs.contains(&"foraging".to_string()));
    }

    #[test]
    fn higher_difficulty_technology_is_learned_more_slowly() {
        // foraging difficulty=0.3 vs food_preservation difficulty=1.8 -- the easier
        // tech should be picked up far more often over the same number of trials.
        let n = 50_000;
        let mut easy_learned = 0;
        let mut hard_learned = 0;
        for _ in 0..n {
            let mut le = capable_individual("le", &[]);
            let te = capable_individual("te", &["foraging"]);
            let mut d1 = HashSet::new();
            learn_tech_from_observation(&mut le, &[&te], &mut d1);
            if le.known_techs.contains(&"foraging".to_string()) {
                easy_learned += 1;
            }

            let mut lh = capable_individual("lh", &["fire_making"]);
            let th = capable_individual("th", &["food_preservation"]);
            let mut d2 = HashSet::new();
            learn_tech_from_observation(&mut lh, &[&th], &mut d2);
            if lh.known_techs.contains(&"food_preservation".to_string()) {
                hard_learned += 1;
            }
        }
        assert!(easy_learned > hard_learned);
    }

    #[test]
    fn learning_a_tech_requires_its_prerequisites_to_already_be_known() {
        // hunting_spear requires stone_tools; observing someone who has hunting_spear
        // but not stone_tools should never transmit it.
        let teacher = capable_individual("teacher", &["hunting_spear"]);
        let mut learner = capable_individual("learner", &[]);
        let mut discovered = HashSet::new();
        for _ in 0..2000 {
            learn_tech_from_observation(&mut learner, &[&teacher], &mut discovered);
        }
        assert!(!learner.known_techs.contains(&"hunting_spear".to_string()));
    }

    #[test]
    fn learning_becomes_possible_once_the_prerequisite_is_completed() {
        let teacher = capable_individual("teacher", &["hunting_spear"]);
        let mut learner = capable_individual("learner", &["stone_tools"]);
        let mut discovered = HashSet::new();
        let mut learned = false;
        for _ in 0..100_000 {
            learn_tech_from_observation(&mut learner, &[&teacher], &mut discovered);
            if learner.known_techs.contains(&"hunting_spear".to_string()) {
                learned = true;
                break;
            }
        }
        assert!(learned, "learner who already knows the prerequisite should eventually pick up hunting_spear");
    }

    #[test]
    fn a_known_tech_can_eventually_be_learned_from_a_nearby_peer_who_knows_it() {
        let teacher = capable_individual("teacher", &["stone_tools"]);
        let mut learner = capable_individual("learner", &[]);
        let mut discovered = HashSet::new();
        let mut learned = false;
        for _ in 0..50_000 {
            learn_tech_from_observation(&mut learner, &[&teacher], &mut discovered);
            if learner.known_techs.contains(&"stone_tools".to_string()) {
                learned = true;
                break;
            }
        }
        assert!(learned, "a capable learner near a knowledgeable peer should eventually pick up stone_tools");
        assert!(discovered.contains("stone_tools"));
    }

    #[test]
    fn own_known_techs_are_never_relearned_or_duplicated() {
        let teacher = capable_individual("teacher", &["stone_tools"]);
        let mut learner = capable_individual("learner", &["stone_tools"]);
        let mut discovered = HashSet::new();
        for _ in 0..200 {
            learn_tech_from_observation(&mut learner, &[&teacher], &mut discovered);
        }
        assert_eq!(learner.known_techs.iter().filter(|t| *t == "stone_tools").count(), 1);
    }

    // ── TECH_TREE static structure ──────────────────────────────────────

    #[test]
    fn all_tier_zero_technologies_have_no_prerequisites() {
        for (id, tier, requires, ..) in TECH_TREE {
            if *tier == 0 {
                assert!(requires.is_empty(), "{id} is tier 0 but has prerequisites");
            }
        }
    }

    #[test]
    fn hunting_spear_requires_stone_tools() {
        let (.., requires, _, _, _, _) = TECH_TREE.iter().find(|(id, ..)| *id == "hunting_spear").unwrap();
        assert!(requires.contains(&"stone_tools"));
    }

    #[test]
    fn pottery_requires_plant_cultivation_and_fire_making() {
        let (.., requires, _, _, _, _) = TECH_TREE.iter().find(|(id, ..)| *id == "pottery").unwrap();
        assert!(requires.contains(&"plant_cultivation"));
        assert!(requires.contains(&"fire_making"));
    }

    #[test]
    fn writing_system_requires_pottery_and_language_stage_five() {
        let (_, _, requires, _, _, _, lang_min) = TECH_TREE.iter().find(|(id, ..)| *id == "writing_system").unwrap();
        assert!(requires.contains(&"pottery"));
        assert_eq!(*lang_min, Some(5));
    }

    #[test]
    fn mathematics_basic_requires_writing_system() {
        let (.., requires, _, _, _, _) = TECH_TREE.iter().find(|(id, ..)| *id == "mathematics_basic").unwrap();
        assert!(requires.contains(&"writing_system"));
    }

    #[test]
    fn all_prerequisites_are_defined_in_tech_tree() {
        let all_ids: HashSet<&str> = TECH_TREE.iter().map(|(id, ..)| *id).collect();
        for (id, _, requires, ..) in TECH_TREE {
            for req in *requires {
                assert!(all_ids.contains(req), "{id} prerequisite {req} is not defined");
            }
        }
    }

    #[test]
    fn every_technology_has_positive_difficulty_and_iq_min_within_zero_one() {
        for (_, _, _, difficulty, iq_min, ..) in TECH_TREE {
            assert!(*difficulty > 0.0);
            assert!(*iq_min >= 0.0);
            assert!(*iq_min <= 1.0);
        }
    }

    // H-16 regression: clothing_basic's prerequisite used to differ between
    // this table (empty -- learn_tech_from_observation's own gate) and
    // agent.rs's TECH_SKILLS (required stone_tools -- check_tech_emergence's
    // own gate), letting an individual pick up the tech from a nearby peer
    // without ever knowing stone_tools themselves. Every tech's prerequisite
    // set must agree between the two pathways.
    #[test]
    fn every_techs_prerequisites_agree_between_tech_tree_and_agent_tech_skills() {
        use std::collections::BTreeSet;
        for (id, _, requires, ..) in TECH_TREE {
            let (_, agent_requires, ..) = crate::agent::TECH_SKILLS.iter().find(|(aid, ..)| aid == id).unwrap_or_else(|| panic!("{id} missing from agent::TECH_SKILLS"));
            let tree_set: BTreeSet<&str> = requires.iter().copied().collect();
            let agent_set: BTreeSet<&str> = agent_requires.iter().copied().collect();
            assert_eq!(tree_set, agent_set, "{id}'s prerequisites differ between TECH_TREE {tree_set:?} and agent::TECH_SKILLS {agent_set:?}");
        }
    }
}
