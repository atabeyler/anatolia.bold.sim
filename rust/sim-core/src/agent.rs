use serde_json::{json, Value};
use std::collections::HashSet;

use crate::state::Individual;

/// (tech_id, prereqs, skills-and-thresholds, lang_min)
///
/// Deterministic, non-probabilistic tech emergence purely from an individual's
/// own accumulated physical experience (accumulate_experience/check_tech_emergence).
/// This is a *separate* pathway from technology::learn_tech_from_observation
/// (which requires observing a peer); every tech key here must also exist in
/// technology::TECH_TREE.
#[allow(clippy::type_complexity)]
pub const TECH_SKILLS: &[(&str, &[&str], &[(&str, f64)], Option<i32>)] = &[
    ("stone_tools", &[], &[("stone_handling", 1500.0)], None),
    ("foraging", &[], &[("plant_gathering", 1200.0)], None),
    ("fire_making", &[], &[("stone_handling", 1000.0), ("wood_friction", 700.0)], None),
    ("hunting_spear", &["stone_tools"], &[("hunting_practice", 1500.0), ("wood_friction", 800.0)], None),
    ("shelter_basic", &[], &[("wood_friction", 1500.0), ("stone_handling", 500.0)], None),
    ("water_container", &["stone_tools"], &[("water_carrying", 1500.0), ("hide_working", 600.0)], None),
    ("animal_trap", &["stone_tools"], &[("animal_observation", 2000.0), ("wood_friction", 800.0)], None),
    ("clothing_basic", &["stone_tools"], &[("hide_working", 2000.0)], None),
    ("fishing", &["stone_tools"], &[("water_carrying", 1200.0), ("animal_observation", 1000.0)], None),
    ("plant_cultivation", &["foraging"], &[("farming_observation", 3000.0)], None),
    ("animal_herding", &["animal_trap"], &[("animal_observation", 4000.0)], None),
    ("food_preservation", &["fire_making"], &[("wood_friction", 1500.0), ("plant_gathering", 1500.0)], None),
    ("bow_arrow", &["hunting_spear"], &[("hunting_practice", 3000.0), ("wood_friction", 2000.0)], None),
    ("pottery", &["plant_cultivation", "fire_making"], &[("clay_working", 4000.0)], None),
    ("weaving", &["clothing_basic"], &[("fiber_working", 4000.0), ("hide_working", 1000.0)], None),
    ("metallurgy_copper", &["fire_making", "stone_tools"], &[("metal_working", 4000.0)], None),
    ("writing_system", &["pottery"], &[("social_exchange", 8000.0), ("clay_working", 3000.0)], Some(5)),
    ("calendar", &["plant_cultivation"], &[("sky_observation", 10000.0), ("farming_observation", 3000.0)], Some(4)),
    ("mathematics_basic", &["writing_system"], &[("social_exchange", 12000.0), ("sky_observation", 5000.0)], None),
    ("architecture_stone", &["metallurgy_copper"], &[("stone_handling", 10000.0), ("wood_friction", 6000.0)], None),
    ("wheel", &["metallurgy_copper"], &[("stone_handling", 8000.0), ("wood_friction", 8000.0)], None),
    ("irrigation", &["plant_cultivation", "wheel"], &[("farming_observation", 10000.0), ("water_carrying", 5000.0)], None),
    ("sailing", &["fishing", "wheel"], &[("water_carrying", 10000.0), ("wood_friction", 10000.0)], None),
    ("metallurgy_iron", &["metallurgy_copper"], &[("metal_working", 10000.0)], None),
    ("swimming", &[], &[("water_carrying", 2000.0)], None),
];

fn experience_map(individual: &Individual) -> serde_json::Map<String, Value> {
    individual.extra.get("_experience").and_then(Value::as_object).cloned().unwrap_or_default()
}

/// Gains purely from what the individual does today and what their environment
/// offers -- no economy, no inventory, no probability roll anywhere.
pub fn accumulate_experience(individual: &mut Individual, world_state: &Value) {
    let mut exp = experience_map(individual);
    let action = individual.extra.get("_currentAction").and_then(Value::as_str).unwrap_or("explore").to_string();
    let biome = world_state.get("biome").and_then(Value::as_str).unwrap_or("grassland").to_string();
    let season = world_state.get("season").and_then(Value::as_str).unwrap_or("summer").to_string();
    let fauna = world_state.get("fauna").and_then(|v| v.get("prey_density")).and_then(Value::as_f64).unwrap_or(0.0) > 0.2;
    let water = world_state.get("water_abundance").and_then(Value::as_f64).unwrap_or(0.0) > 0.2;

    let p = &individual.phenotype;
    let lr = p.fluid_intelligence * p.curiosity * (0.5 + p.learning_rate);

    let mut gain = |skill: &str, amount: f64| {
        let current = exp.get(skill).and_then(Value::as_f64).unwrap_or(0.0);
        exp.insert(skill.to_string(), json!(current + amount * lr));
    };

    let stones_present = biome != "open_ocean";
    if stones_present {
        if action == "craft" {
            gain("stone_handling", 1.0);
        } else if action == "forage" || action == "hunt" {
            gain("stone_handling", 0.7);
        } else if action == "explore" {
            gain("stone_handling", 0.3);
        }
    }

    let wood_present = ["temperate_forest", "tropical_rainforest", "mediterranean", "grassland", "tropical_savanna"].contains(&biome.as_str());
    if wood_present {
        if action == "craft" {
            gain("wood_friction", 1.0);
        } else if action == "forage" || action == "explore" {
            gain("wood_friction", 0.3);
        }
    }

    let plants_present = ["grassland", "temperate_forest", "mediterranean", "tropical_rainforest", "tropical_savanna"].contains(&biome.as_str());
    if plants_present {
        if action == "forage" {
            gain("plant_gathering", 1.0);
        } else if action == "explore" {
            gain("plant_gathering", 0.2);
        }
    }
    if plants_present && action == "forage" {
        gain("farming_observation", if season == "spring" { 0.6 } else { 0.1 });
    }
    if action == "explore" && season == "spring" {
        gain("farming_observation", 0.1);
    }

    if fauna && action == "hunt" {
        gain("hunting_practice", 1.0);
        gain("animal_observation", 0.5);
        gain("hide_working", 0.4);
    }
    if fauna && action == "explore" {
        gain("animal_observation", 0.5);
    }

    if water {
        let thirsty = individual.health.hydration < 0.5;
        gain("water_carrying", if thirsty { 1.0 } else if action == "drink" { 0.6 } else { 0.1 });
    }

    if fauna && action == "craft" {
        gain("hide_working", 0.6);
    }

    let clay_present = ["coastal", "temperate_forest", "grassland", "mediterranean"].contains(&biome.as_str());
    if clay_present && action == "craft" && individual.known_techs.iter().any(|t| t == "fire_making") {
        gain("clay_working", 1.0);
    }

    if plants_present && action == "craft" {
        gain("fiber_working", 0.8);
    }

    let ore_present = ["mountain", "mediterranean"].contains(&biome.as_str());
    if ore_present && action == "craft" && individual.known_techs.iter().any(|t| t == "fire_making") {
        gain("metal_working", 1.0);
    }

    if action == "socialize" {
        gain("social_exchange", 1.0);
    }
    if action == "explore" {
        gain("sky_observation", 0.3);
    }

    individual.extra.insert("_experience".to_string(), Value::Object(exp));
}

/// Deterministic technology emergence: no probability roll, just structural
/// thresholds crossed by the individual's own experience. Cardinal rule:
/// prerequisites must be in the individual's *own* known_techs, never the
/// global discovered-techs pool -- otherwise one person's tech would silently
/// unlock advanced tech for someone who never learned the basics themselves.
///
/// `innovation_bonus` is the civilization's `innovation_rate` astronomy bonus
/// (see `astronomy::get_astronomy_bonus`) -- a planetary model reflects a
/// generally more analytical, tech-primed culture, so it nudges everyone's
/// threshold factor rather than being gated behind any one prerequisite.
pub fn check_tech_emergence(individual: &mut Individual, discovered_techs: &mut HashSet<String>, innovation_bonus: f64) -> Vec<String> {
    let mut emerged = Vec::new();
    let exp = experience_map(individual);
    if exp.is_empty() {
        return emerged;
    }

    let iq = individual.phenotype.fluid_intelligence;
    let cur = individual.phenotype.curiosity;
    let innov = individual.phenotype.innovation;
    let factor = (iq * cur * 4.0 * (0.5 + innov + innovation_bonus)).max(0.1);

    for (tech_id, prereqs, skills, lang_min) in TECH_SKILLS {
        if discovered_techs.contains(*tech_id) || individual.known_techs.iter().any(|t| t == tech_id) {
            continue;
        }
        if prereqs.iter().any(|p| !individual.known_techs.iter().any(|t| t == p)) {
            continue;
        }
        if let Some(min) = lang_min {
            if individual.language.stage < *min {
                continue;
            }
        }
        let tree_iq_min = crate::technology::tech_index().get(*tech_id).map(|&i| crate::technology::TECH_TREE[i].4).unwrap_or(0.0);
        if iq < tree_iq_min {
            continue;
        }
        let all_met = skills.iter().all(|(skill, base)| exp.get(*skill).and_then(Value::as_f64).unwrap_or(0.0) >= base / factor);
        if all_met {
            emerged.push((*tech_id).to_string());
            discovered_techs.insert((*tech_id).to_string());
            individual.known_techs.push((*tech_id).to_string());
        }
    }
    emerged
}

pub const ACTIONS: &[&str] = &[
    "forage",
    "drink",
    "flee",
    "seek_warmth",
    "rest",
    "hunt",
    "craft",
    "socialize",
    "mate",
    "explore",
];

pub fn select_action(individual: &Individual, world_state: &Value) -> String {
    let health = &individual.health;
    let calories = health.calories;
    let hydration = health.hydration;
    let hp = health.hp;
    let ph = &individual.phenotype;
    let curiosity = ph.curiosity;
    let strength = ph.physical_strength;
    let risk_tolerance = ph.risk_tolerance;
    let stress = individual.psychology.stress_level;
    let mating = individual.extra.get("mating_urge").and_then(Value::as_f64).unwrap_or(0.0);
    let pred_fear = individual.extra.get("_fears").and_then(|v| v.get("predator")).and_then(Value::as_f64).unwrap_or(0.0);
    let dis_fear = individual.extra.get("_fears").and_then(|v| v.get("disaster")).and_then(Value::as_f64).unwrap_or(0.0);
    let temp = world_state.get("temperature").and_then(Value::as_f64).unwrap_or(20.0);
    let fauna = world_state.get("fauna").and_then(|v| v.get("prey_density")).and_then(Value::as_f64).unwrap_or(0.0);
    let age_years = individual.age_days.unwrap_or(0) as f64 / 365.0;
    let is_adult = age_years >= 13.0;

    let mut best = ("explore", -1.0);
    let scores = [
        ("flee", pred_fear.max(dis_fear) * (2.0 - risk_tolerance * 0.6)),
        ("rest", if hp < 0.25 { (1.0 - hp) * 1.8 } else { 0.0 }),
        ("drink", if hydration < 0.4 { (0.4 - hydration) * 3.5 } else { 0.0 }),
        ("forage", if calories < 0.4 { (0.4 - calories) * 3.0 } else { 0.0 }),
        ("seek_warmth", if temp < 8.0 { (8.0 - temp) / 10.0 } else { 0.0 }),
        ("mate", if is_adult && mating > 0.65 && calories > 0.4 && hydration > 0.35 { (mating - 0.65) * 2.0 } else { 0.0 }),
        ("hunt", if (calories + hydration + hp) / 3.0 > 0.55 && stress < 0.7 && fauna > 0.2 && strength > 0.3 && calories < 0.8 {
            fauna * strength * (0.8 + risk_tolerance * 0.4)
        } else { 0.0 }),
        ("craft", if (calories + hydration + hp) / 3.0 > 0.55 && stress < 0.7 { curiosity * ((calories + hydration + hp) / 3.0) * 0.6 } else { 0.0 }),
        ("socialize", if (calories + hydration + hp) / 3.0 > 0.55 && stress < 0.7 { ((calories + hydration + hp) / 3.0) * 0.25 } else { 0.0 }),
        ("explore", if (calories + hydration + hp) / 3.0 > 0.55 && stress < 0.7 { curiosity * 0.2 + risk_tolerance * 0.1 } else { 0.0 }),
    ];
    for (action, score) in scores {
        let noisy = score + (rand::random::<f64>() * 0.04);
        if noisy > best.1 {
            best = (action, noisy);
        }
    }
    best.0.to_string()
}

#[cfg(test)]
mod activity_tests {
    use super::*;
    use crate::types::{Health, Language, Phenotype};

    fn make_ind(action: &str) -> Individual {
        Individual {
            phenotype: Phenotype { fluid_intelligence: 0.7, curiosity: 0.7, ..Default::default() },
            language: Language { stage: 0, ..Default::default() },
            extra: {
                let mut m = serde_json::Map::new();
                m.insert("_currentAction".to_string(), json!(action));
                m
            },
            ..Default::default()
        }
    }

    fn make_world() -> Value {
        json!({
            "biome": "mediterranean",
            "temperature": 20.0,
            "season": "summer",
            "fauna": { "prey_density": 0.5 },
            "water_abundance": 0.5,
        })
    }

    fn exp_of(ind: &Individual, skill: &str) -> f64 {
        experience_map(ind).get(skill).and_then(Value::as_f64).unwrap_or(0.0)
    }

    #[test]
    fn craft_in_non_ocean_biome_gains_stone_handling() {
        let mut ind = make_ind("craft");
        accumulate_experience(&mut ind, &make_world());
        assert!(exp_of(&ind, "stone_handling") > 0.0);
    }

    #[test]
    fn explore_gains_less_stone_handling_than_craft() {
        let mut craft = make_ind("craft");
        let mut explore = make_ind("explore");
        accumulate_experience(&mut craft, &make_world());
        accumulate_experience(&mut explore, &make_world());
        assert!(exp_of(&craft, "stone_handling") > exp_of(&explore, "stone_handling"));
    }

    #[test]
    fn open_ocean_biome_gains_no_stone_handling() {
        let mut ind = make_ind("craft");
        let mut world = make_world();
        world["biome"] = json!("open_ocean");
        accumulate_experience(&mut ind, &world);
        assert_eq!(exp_of(&ind, "stone_handling"), 0.0);
    }

    #[test]
    fn hunt_with_fauna_gains_hunting_practice() {
        let mut ind = make_ind("hunt");
        accumulate_experience(&mut ind, &make_world());
        assert!(exp_of(&ind, "hunting_practice") > 0.0);
    }

    #[test]
    fn hunt_without_fauna_gains_no_hunting_practice() {
        let mut ind = make_ind("hunt");
        let mut world = make_world();
        world["fauna"] = json!({ "prey_density": 0.1 });
        accumulate_experience(&mut ind, &world);
        assert_eq!(exp_of(&ind, "hunting_practice"), 0.0);
    }

    #[test]
    fn higher_iq_and_curiosity_gain_more_experience_per_action() {
        let mut genius = make_ind("craft");
        genius.phenotype = Phenotype { fluid_intelligence: 0.9, curiosity: 0.9, ..Default::default() };
        let mut slow = make_ind("craft");
        slow.phenotype = Phenotype { fluid_intelligence: 0.2, curiosity: 0.2, ..Default::default() };
        accumulate_experience(&mut genius, &make_world());
        accumulate_experience(&mut slow, &make_world());
        assert!(exp_of(&genius, "stone_handling") > exp_of(&slow, "stone_handling"));
    }

    #[test]
    fn thirsty_individual_near_water_gains_more_water_carrying_than_hydrated() {
        let mut thirsty = make_ind("explore");
        thirsty.health = Health { hydration: 0.3, ..Default::default() };
        let mut fine = make_ind("explore");
        fine.health = Health { hydration: 0.9, ..Default::default() };
        accumulate_experience(&mut thirsty, &make_world());
        accumulate_experience(&mut fine, &make_world());
        assert!(exp_of(&thirsty, "water_carrying") > exp_of(&fine, "water_carrying"));
    }

    #[test]
    fn no_water_present_means_no_water_carrying_gain() {
        let mut ind = make_ind("drink");
        ind.health = Health { hydration: 0.2, ..Default::default() };
        let mut world = make_world();
        world["water_abundance"] = json!(0.1);
        accumulate_experience(&mut ind, &world);
        assert_eq!(exp_of(&ind, "water_carrying"), 0.0);
    }

    fn ind_with_experience(iq: f64, cur: f64, pairs: &[(&str, f64)]) -> Individual {
        let mut ind = Individual {
            phenotype: Phenotype { fluid_intelligence: iq, curiosity: cur, ..Default::default() },
            ..Default::default()
        };
        let mut exp = serde_json::Map::new();
        for (skill, amount) in pairs {
            exp.insert(skill.to_string(), json!(amount));
        }
        ind.extra.insert("_experience".to_string(), Value::Object(exp));
        ind
    }

    #[test]
    fn stone_tools_emerges_when_stone_handling_exceeds_threshold() {
        // factor = max(0.1, 0.7*0.7*4*0.5) = 0.98; threshold = 1500/0.98 ~ 1531 -- use a
        // generous margin so this isn't sensitive to the exact innovation default.
        let mut ind = ind_with_experience(0.7, 0.7, &[("stone_handling", 5000.0)]);
        let mut discovered = HashSet::new();
        let emerged = check_tech_emergence(&mut ind, &mut discovered, 0.0);
        assert!(emerged.contains(&"stone_tools".to_string()));
        assert!(discovered.contains("stone_tools"));
        assert!(ind.known_techs.iter().any(|t| t == "stone_tools"));
    }

    #[test]
    fn stone_tools_does_not_emerge_when_experience_is_too_low() {
        let mut ind = ind_with_experience(0.7, 0.7, &[("stone_handling", 10.0)]);
        let mut discovered = HashSet::new();
        let emerged = check_tech_emergence(&mut ind, &mut discovered, 0.0);
        assert!(!emerged.contains(&"stone_tools".to_string()));
    }

    #[test]
    fn hunting_spear_blocked_when_stone_tools_not_in_own_known_techs() {
        // Cardinal rule: prerequisite check uses the individual's own known_techs,
        // never the global discovered_techs pool.
        let mut ind = ind_with_experience(0.8, 0.8, &[("hunting_practice", 9999.0), ("wood_friction", 9999.0)]);
        let mut discovered = HashSet::new();
        discovered.insert("stone_tools".to_string());
        let emerged = check_tech_emergence(&mut ind, &mut discovered, 0.0);
        assert!(!emerged.contains(&"hunting_spear".to_string()));
    }

    #[test]
    fn hunting_spear_emerges_once_stone_tools_is_in_own_known_techs() {
        let mut ind = ind_with_experience(
            0.8,
            0.8,
            &[("stone_handling", 9999.0), ("hunting_practice", 9999.0), ("wood_friction", 9999.0)],
        );
        ind.known_techs.push("stone_tools".to_string());
        let mut discovered = HashSet::new();
        discovered.insert("stone_tools".to_string());
        let emerged = check_tech_emergence(&mut ind, &mut discovered, 0.0);
        assert!(emerged.contains(&"hunting_spear".to_string()));
    }

    #[test]
    fn astronomy_innovation_bonus_speeds_up_tech_emergence() {
        // Phenotype::default().innovation is 0.5 (its Default impl round-trips
        // through serde's own field defaults). factor at bonus=0.0 is
        // 0.7*0.7*4*(0.5+0.5) = 1.96, so the no-bonus threshold is 1500/1.96 ~
        // 765 -- 700 sits just below it. At bonus=1.0 the factor doubles to
        // 3.92 (threshold ~383), comfortably below 700.
        let ind = ind_with_experience(0.7, 0.7, &[("stone_handling", 700.0)]);
        let discovered = HashSet::new();

        let mut without_bonus = ind.clone();
        let mut discovered_a = discovered.clone();
        assert!(
            !check_tech_emergence(&mut without_bonus, &mut discovered_a, 0.0).contains(&"stone_tools".to_string()),
            "test fixture should sit below the no-bonus threshold"
        );

        let mut with_bonus = ind.clone();
        let mut discovered_b = discovered;
        assert!(
            check_tech_emergence(&mut with_bonus, &mut discovered_b, 1.0).contains(&"stone_tools".to_string()),
            "a strong innovation_rate bonus should push the same experience over the threshold"
        );
    }

    #[test]
    fn bug02_regression_prereq_in_own_known_techs_but_absent_from_global_pool_still_emerges() {
        // Individual knowledge is what matters, not the global discovered_techs pool.
        let mut ind = ind_with_experience(0.9, 0.9, &[("hunting_practice", 9999.0), ("wood_friction", 9999.0)]);
        ind.known_techs.push("stone_tools".to_string());
        let mut discovered = HashSet::new(); // empty: stone_tools not globally discovered
        let emerged = check_tech_emergence(&mut ind, &mut discovered, 0.0);
        assert!(emerged.contains(&"hunting_spear".to_string()));
    }

    #[test]
    fn writing_system_blocked_when_iq_below_minimum() {
        let mut ind = ind_with_experience(0.5, 0.9, &[("social_exchange", 9999.0), ("clay_working", 9999.0)]);
        ind.language.stage = 6;
        let mut discovered = HashSet::new();
        discovered.insert("pottery".to_string());
        let emerged = check_tech_emergence(&mut ind, &mut discovered, 0.0);
        assert!(!emerged.contains(&"writing_system".to_string()));
    }

    #[test]
    fn already_discovered_techs_never_emerge_again() {
        let mut ind = ind_with_experience(0.9, 0.9, &[("stone_handling", 9999.0), ("plant_gathering", 9999.0)]);
        let mut discovered: HashSet<String> = ["stone_tools", "foraging"].iter().map(|s| s.to_string()).collect();
        let emerged = check_tech_emergence(&mut ind, &mut discovered, 0.0);
        assert!(!emerged.contains(&"stone_tools".to_string()));
        assert!(!emerged.contains(&"foraging".to_string()));
    }

    #[test]
    fn swimming_emerges_with_sufficient_water_carrying_and_no_prereqs() {
        let mut ind = ind_with_experience(0.5, 0.5, &[("water_carrying", 9999.0)]);
        let mut discovered = HashSet::new();
        let emerged = check_tech_emergence(&mut ind, &mut discovered, 0.0);
        assert!(emerged.contains(&"swimming".to_string()));
    }

    #[test]
    fn swimming_has_no_prerequisites_in_tech_skills() {
        let (_, prereqs, ..) = TECH_SKILLS.iter().find(|(id, ..)| *id == "swimming").unwrap();
        assert!(prereqs.is_empty());
    }

    // ── ACTIONS stays in sync with what select_action can actually return ──

    #[test]
    fn select_action_never_returns_anything_outside_declared_actions() {
        for i in 0..500 {
            let seed = i as f64 / 500.0;
            let ind = Individual {
                health: crate::types::Health { hp: seed, calories: 1.0 - seed, hydration: (seed * 1.7) % 1.0, ..Default::default() },
                phenotype: Phenotype { curiosity: seed, physical_strength: 1.0 - seed, risk_tolerance: (seed * 1.3) % 1.0, ..Default::default() },
                psychology: crate::types::Psychology { stress_level: (seed * 2.1) % 1.0, ..Default::default() },
                birth_day: -365 * 25,
                age_days: Some(365 * 25),
                extra: {
                    let mut m = serde_json::Map::new();
                    m.insert("mating_urge".to_string(), json!((seed * 0.9) % 1.0));
                    m.insert("_fears".to_string(), json!({ "predator": (seed * 0.5) % 1.0, "disaster": (seed * 0.3) % 1.0 }));
                    m
                },
                ..Default::default()
            };
            let mut world = make_world();
            world["temperature"] = json!(seed * 30.0 - 5.0);
            let action = select_action(&ind, &world);
            assert!(ACTIONS.contains(&action.as_str()), "select_action returned {action:?}, missing from ACTIONS");
        }
    }
}
