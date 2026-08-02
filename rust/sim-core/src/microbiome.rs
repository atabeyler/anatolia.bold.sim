use serde_json::{json, Value};

#[allow(clippy::type_complexity)]
pub const PATHOGEN_TYPES: &[(&str, &str, f64, i32, i32, i32, i32, Option<&[&str]>)] = &[
    ("wound_infection", "contact", 0.12, 14, 60, 2, 180, None),
    ("intestinal_parasite", "fecal_oral", 0.05, 30, 365 * 3, 5, 365, None),
    ("respiratory_common", "airborne", 0.02, 14, 180, 4, 365, None),
    ("fungal_skin", "contact", 0.01, 30, 180, 4, 500, None),
    ("fever_tick", "vector", 0.08, 10, 365 * 2, 3, 730, Some(&["grassland", "temperate_forest"])),
    ("malaria_like", "vector", 0.10, 14, 365, 3, 730, Some(&["tropical_rainforest", "tropical_savanna", "coastal"])),
    ("pneumonia_like", "airborne", 0.15, 21, 365 * 2, 8, 1000, None),
    ("cholera_like", "water", 0.30, 7, 365 * 5, 20, 1500, None),
    ("plague_like", "airborne", 0.40, 10, 365 * 10, 30, 2000, None),
];

pub fn process_microbiome_tick(population: &mut [crate::state::Individual], world_state: &Value, sim_day: i32) -> Vec<Value> {
    let mut events = Vec::new();
    let alive_count = population.iter().filter(|i| !i.is_dead).count();
    for ind in population.iter_mut().filter(|i| !i.is_dead) {
        dedupe_infections(ind);
    }
    let season = world_state.get("season").and_then(Value::as_str).unwrap_or("");
    // Grace period: no NEW outbreaks before day 180 or below 8 people. Existing
    // infections are always processed (cleared + mortality) below regardless of density.
    if sim_day >= 180 && alive_count >= 8 {
        for (pathogen_id, transmission, base_mortality, duration_days, _immunity_duration, density_threshold, min_day, biomes) in PATHOGEN_TYPES {
            if alive_count < *density_threshold as usize || sim_day < *min_day {
                continue;
            }
            if let Some(biomes) = biomes {
                if !biomes.contains(&world_state.get("biome").and_then(Value::as_str).unwrap_or("")) {
                    continue;
                }
            }
            let seasonal_multiplier = if season == "summer" && *transmission == "vector" {
                2.0
            } else if season == "winter" && *transmission == "airborne" {
                1.5
            } else {
                1.0
            };
            let mut new_cases = 0;
            for individual in population.iter_mut().filter(|i| !i.is_dead) {
                if has_pathogen(individual, pathogen_id) || immune_until(individual, pathogen_id) > sim_day {
                    continue;
                }
                if rand::random::<f64>() < exposure_probability(individual, transmission, seasonal_multiplier, alive_count) {
                    let infections = individual
                        .extra
                        .entry("infections".to_string())
                        .or_insert_with(|| json!([]));
                    if let Some(arr) = infections.as_array_mut() {
                        arr.push(json!({ "pathogen_id": pathogen_id, "days_remaining": duration_days, "infected_day": sim_day }));
                    }
                    new_cases += 1;
                }
            }
            if new_cases > 0 {
                events.push(json!({
                    "type": "epidemic_outbreak",
                    "pathogen_id": pathogen_id,
                    "initial_cases": new_cases,
                    "day": sim_day,
                    "importance": if *base_mortality > 0.2 { "high" } else { "medium" }
                }));
            }
        }
    }

    // Always process existing infections: decrement duration, apply mortality,
    // clear when done and grant temporary immunity to survivors.
    for individual in population.iter_mut() {
        if individual.is_dead {
            continue;
        }
        let Some(infections) = individual.extra.get_mut("infections").and_then(Value::as_array_mut) else { continue };
        if infections.is_empty() {
            continue;
        }
        let hp = individual.health.hp;
        let immune_strength = individual.phenotype.immune_strength;
        let microbiome_immunity = individual.health.microbiome_immunity.unwrap_or(0.0);
        let total_immunity = (immune_strength * 0.7 + microbiome_immunity).min(0.95);

        let mut newly_dead = false;
        let mut resolved: Vec<(String, i32)> = Vec::new();
        for infection in infections.iter_mut() {
            let Some(obj) = infection.as_object_mut() else { continue };
            // An infection acquired THIS tick (by the outbreak/exposure pass
            // above, which stamps infected_day) must not also be decremented
            // and mortality-rolled the same tick -- without this, day 0 of an
            // infection was silently already "day 1", so a brand-new case
            // could (rarely) kill on the very day of exposure.
            if obj.get("infected_day").and_then(Value::as_i64) == Some(sim_day as i64) {
                continue;
            }
            let days_remaining = obj.get("days_remaining").and_then(Value::as_i64).unwrap_or(0) as i32 - 1;
            obj.insert("days_remaining".to_string(), json!(days_remaining));
            let Some(pathogen_id) = obj.get("pathogen_id").and_then(Value::as_str).map(str::to_string) else { continue };
            let Some((_, _, base_mortality, duration_days, immunity_duration, ..)) = PATHOGEN_TYPES.iter().find(|(id, ..)| *id == pathogen_id) else {
                if days_remaining <= 0 {
                    resolved.push((pathogen_id, 0));
                }
                continue;
            };
            if !newly_dead {
                // Founders get the same protection here as everywhere else in
                // the mortality model (0.5x on process_disaster/background
                // risk) -- previously disease was the one death path that
                // could still wipe out a founder despite them being
                // deliberately shielded from every other cause.
                let founder_factor = if individual.is_founder { 0.5 } else { 1.0 };
                // Interferon (see hormones.rs) rises with this same active
                // infection and reflects the real antiviral response -- a
                // small, bounded discount on top of the existing immunity/hp
                // terms, not a replacement for them.
                let interferon_factor = 1.0 - (individual.hormones.interferon - 0.3).max(0.0) * 0.2;
                let daily_mortality = base_mortality * (1.0 - total_immunity) * (1.0 - hp * 0.3) * founder_factor * interferon_factor / *duration_days as f64;
                if rand::random::<f64>() < daily_mortality {
                    newly_dead = true;
                }
            }
            if days_remaining <= 0 || newly_dead {
                resolved.push((pathogen_id, *immunity_duration));
            }
        }
        if newly_dead {
            individual.alive = false;
            individual.is_dead = true;
            individual.death_day = Some(sim_day);
            individual.extra.insert("death_cause".to_string(), json!("infection"));
            events.push(json!({ "type": "death", "individual_id": individual.id, "cause": "infection", "day": sim_day, "importance": "medium", "is_founder": individual.is_founder }));
        }
        if !resolved.is_empty() {
            let resolved_ids: std::collections::HashSet<&str> = resolved.iter().map(|(id, _)| id.as_str()).collect();
            if let Some(infections) = individual.extra.get_mut("infections").and_then(Value::as_array_mut) {
                infections.retain(|inf| inf.get("pathogen_id").and_then(Value::as_str).map(|id| !resolved_ids.contains(id)).unwrap_or(true));
            }
            if !individual.is_dead {
                let immunities = individual.extra.entry("immunities".to_string()).or_insert_with(|| json!({}));
                if let Some(obj) = immunities.as_object_mut() {
                    for (pathogen_id, immunity_duration) in resolved {
                        if immunity_duration > 0 {
                            obj.insert(pathogen_id, json!(sim_day + immunity_duration));
                        }
                    }
                }
            }
        }
    }

    events
}

fn has_pathogen(individual: &crate::state::Individual, pathogen_id: &str) -> bool {
    individual
        .extra
        .get("infections")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().any(|inf| inf.get("pathogen_id").and_then(Value::as_str) == Some(pathogen_id)))
        .unwrap_or(false)
}

fn immune_until(individual: &crate::state::Individual, pathogen_id: &str) -> i32 {
    individual
        .extra
        .get("immunities")
        .and_then(|v| v.get(pathogen_id))
        .and_then(Value::as_i64)
        .unwrap_or(0) as i32
}

fn exposure_probability(individual: &crate::state::Individual, transmission: &str, seasonal_multiplier: f64, alive_count: usize) -> f64 {
    let population_scale = (alive_count as f64 / 25.0).clamp(0.2, 1.0);
    let base = 0.00008 * seasonal_multiplier * population_scale;
    match transmission {
        "water" => base * (1.0 + (1.0 - individual.health.hydration).max(0.0) * 2.0),
        "fecal_oral" => base * if individual.group_id.is_some() { 1.5 } else { 0.6 },
        "airborne" => base * if individual.group_id.is_some() { 2.0 } else { 0.4 },
        "vector" => base,
        "contact" => base * (1.0 + (0.5 - individual.health.hp).max(0.0) * 3.0),
        _ => base,
    }
}

fn dedupe_infections(individual: &mut crate::state::Individual) {
    if let Some(infections) = individual.extra.get_mut("infections").and_then(Value::as_array_mut) {
        if infections.len() < 2 {
            return;
        }
        let mut by_path: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
        for infection in infections.drain(..) {
            let Some(pathogen_id) = infection.get("pathogen_id").and_then(Value::as_str).map(str::to_string) else { continue };
            let remaining = infection.get("days_remaining").and_then(Value::as_i64).unwrap_or(0);
            let keep_new = by_path.get(&pathogen_id).and_then(|prev| prev.get("days_remaining")).and_then(Value::as_i64).map(|prev| remaining > prev).unwrap_or(true);
            if keep_new {
                by_path.insert(pathogen_id, infection);
            }
        }
        *infections = by_path.into_values().collect();
    }
}

/// Inter-individual pathogen transmission: an already-infected individual can pass a
/// pathogen they carry to a nearby susceptible one. Distinct from the environmental
/// exposure rolled in process_microbiome_tick above.
pub fn spread_infection(infected: &crate::state::Individual, susceptible: &mut crate::state::Individual, pathogen_id: &str, sim_day: i32, alive_count: usize) -> bool {
    if !has_pathogen(infected, pathogen_id) || immune_until(susceptible, pathogen_id) > sim_day {
        return false;
    }
    if has_pathogen(susceptible, pathogen_id) {
        return false;
    }
    let Some((_, transmission, _, duration_days, ..)) = PATHOGEN_TYPES.iter().find(|(id, ..)| *id == pathogen_id) else {
        return false;
    };
    let group_scale = (alive_count as f64 / 30.0).clamp(0.3, 1.0);
    let base_rate = match *transmission {
        "airborne" => 0.3,
        "contact" => 0.2,
        _ => 0.15,
    };
    let rate = base_rate * group_scale;
    if rand::random::<f64>() < rate * (1.0 - susceptible.phenotype.immune_strength * 0.5) {
        let infections = susceptible.extra.entry("infections".to_string()).or_insert_with(|| json!([]));
        if let Some(arr) = infections.as_array_mut() {
            arr.push(json!({ "pathogen_id": pathogen_id, "days_remaining": duration_days, "infected_day": sim_day }));
        }
        true
    } else {
        false
    }
}

pub fn update_gut_microbiome(individual: &mut crate::state::Individual, world_state: &Value) {
    let diversity = individual
        .extra
        .get("microbiome")
        .and_then(|v| v.get("diversity"))
        .and_then(Value::as_f64)
        .unwrap_or(0.5);
    let food = world_state.get("food_abundance").and_then(Value::as_f64).unwrap_or(0.5);
    let immune_boost = individual.phenotype.immune_strength * 0.15;
    let new_diversity = (diversity * 0.95 + (food * 0.5 + immune_boost).min(1.0) * 0.05).min(1.0);
    individual
        .extra
        .insert("microbiome".to_string(), json!({ "diversity": new_diversity, "composition": {} }));
    individual.health.microbiome_immunity = Some((new_diversity * 0.3).min(0.3));
}

pub fn compute_health_stats(population: &[crate::state::Individual]) -> Value {
    let living: Vec<_> = population.iter().filter(|i| !i.is_dead).collect();
    if living.is_empty() {
        return json!({ "sick_count": 0, "sick_rate": 0.0, "pathogen_diversity": 0 });
    }
    let sick: Vec<_> = living
        .iter()
        .filter(|i| i.extra.get("infections").and_then(Value::as_array).map(|a| !a.is_empty()).unwrap_or(false))
        .collect();
    let mut pathogens = std::collections::HashSet::new();
    for individual in &sick {
        if let Some(arr) = individual.extra.get("infections").and_then(Value::as_array) {
            for infection in arr {
                if let Some(pid) = infection.get("pathogen_id").and_then(Value::as_str) {
                    pathogens.insert(pid.to_string());
                }
            }
        }
    }
    json!({
        "sick_count": sick.len(),
        "sick_rate": sick.len() as f64 / living.len() as f64,
        "pathogen_diversity": pathogens.len()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Individual;
    use crate::types::{Health, Phenotype};

    fn infected(pathogen: &str, days_remaining: i64) -> Individual {
        let mut ind = Individual::default();
        ind.extra.insert("infections".to_string(), json!([{ "pathogen_id": pathogen, "days_remaining": days_remaining }]));
        ind
    }

    // ── microbiome infection dedupe ──────────────────────────────────────

    #[test]
    fn spread_infection_does_not_duplicate_an_already_present_infection() {
        let infected_ind = infected("respiratory_common", 10);
        let mut susceptible = Individual {
            phenotype: Phenotype { immune_strength: 0.0, ..Default::default() },
            ..infected("respiratory_common", 5)
        };
        let spread = spread_infection(&infected_ind, &mut susceptible, "respiratory_common", 10, 20);
        assert!(!spread);
        assert_eq!(susceptible.extra["infections"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn dedupe_collapses_duplicate_persisted_infections_before_a_tick_and_decrements_the_survivor() {
        let mut ind = Individual {
            alive: true,
            is_dead: false,
            phenotype: Phenotype { immune_strength: 1.0, ..Default::default() },
            health: Health { hp: 1.0, ..Default::default() },
            ..Default::default()
        };
        ind.extra.insert(
            "infections".to_string(),
            json!([
                { "pathogen_id": "respiratory_common", "days_remaining": 2 },
                { "pathogen_id": "respiratory_common", "days_remaining": 9 },
                { "pathogen_id": "fungal_skin", "days_remaining": 5 },
            ]),
        );
        let mut population = vec![ind];
        process_microbiome_tick(&mut population, &json!({ "biome": "mediterranean", "season": "spring" }), 200);

        let mut ids: Vec<String> = population[0].extra["infections"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|i| i.get("pathogen_id").and_then(Value::as_str).map(String::from))
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["fungal_skin".to_string(), "respiratory_common".to_string()]);

        let respiratory = population[0].extra["infections"].as_array().unwrap().iter().find(|i| i["pathogen_id"] == "respiratory_common").unwrap();
        // After dedupe (keeps higher days_remaining=9) + one tick decrement -> 8.
        assert_eq!(respiratory["days_remaining"], 8);
    }

    // ── infection resolution (previously entirely missing) ──────────────

    #[test]
    fn an_infection_is_cleared_once_days_remaining_reaches_zero() {
        let ind = Individual {
            phenotype: Phenotype { immune_strength: 1.0, ..Default::default() },
            health: Health { hp: 1.0, ..Default::default() },
            ..infected("fungal_skin", 1)
        };
        let mut population = vec![ind];
        process_microbiome_tick(&mut population, &json!({}), 1);
        assert!(population[0].extra.get("infections").and_then(Value::as_array).map(|a| a.is_empty()).unwrap_or(true));
    }

    #[test]
    fn surviving_a_resolved_infection_grants_temporary_immunity() {
        let ind = Individual {
            phenotype: Phenotype { immune_strength: 1.0, ..Default::default() },
            health: Health { hp: 1.0, ..Default::default() },
            ..infected("fungal_skin", 1)
        };
        let mut population = vec![ind];
        process_microbiome_tick(&mut population, &json!({}), 100);
        assert!(!population[0].is_dead);
        let immune_until_day = population[0].extra["immunities"]["fungal_skin"].as_i64().unwrap();
        assert_eq!(immune_until_day, 100 + 180); // fungal_skin immunity_duration = 180
    }

    #[test]
    fn an_infection_acquired_this_tick_is_not_also_decremented_this_tick() {
        // V-20 regression: an infection whose infected_day matches the
        // current sim_day must be left untouched this tick -- its own
        // duration countdown and mortality roll should only start the
        // following tick, not the same day it was contracted.
        let mut ind = Individual {
            phenotype: Phenotype { immune_strength: 1.0, ..Default::default() },
            health: Health { hp: 1.0, ..Default::default() },
            ..Default::default()
        };
        ind.extra.insert("infections".to_string(), json!([{ "pathogen_id": "fungal_skin", "days_remaining": 5, "infected_day": 50 }]));
        let mut population = vec![ind];
        process_microbiome_tick(&mut population, &json!({}), 50);
        let infection = &population[0].extra["infections"].as_array().unwrap()[0];
        assert_eq!(infection["days_remaining"], 5, "an infection acquired this same tick must not be decremented yet");
    }

    #[test]
    fn an_infection_from_a_prior_tick_is_decremented_normally() {
        let mut ind = Individual {
            phenotype: Phenotype { immune_strength: 1.0, ..Default::default() },
            health: Health { hp: 1.0, ..Default::default() },
            ..Default::default()
        };
        ind.extra.insert("infections".to_string(), json!([{ "pathogen_id": "fungal_skin", "days_remaining": 5, "infected_day": 49 }]));
        let mut population = vec![ind];
        process_microbiome_tick(&mut population, &json!({}), 50);
        let infection = &population[0].extra["infections"].as_array().unwrap()[0];
        assert_eq!(infection["days_remaining"], 4);
    }

    #[test]
    fn infection_death_event_carries_is_founder_matching_the_actual_victim() {
        // The frontend plays a distinct founder-death alarm keyed off
        // data.is_founder -- this must reflect who actually died.
        let founder = Individual {
            id: "founder-1".to_string(),
            is_founder: true,
            phenotype: Phenotype { immune_strength: 0.0, ..Default::default() },
            health: Health { hp: 0.0, ..Default::default() },
            ..infected("plague_like", 500)
        };
        let mut population = vec![founder];
        let mut death_event = None;
        for day in 0..500 {
            let events = process_microbiome_tick(&mut population, &json!({}), day);
            if let Some(ev) = events.into_iter().find(|e| e["type"] == "death") {
                death_event = Some(ev);
                break;
            }
        }
        let ev = death_event.expect("expected the founder to eventually die to plague_like at 0 immune_strength/hp");
        assert_eq!(ev["is_founder"], true);
    }

    #[test]
    fn founders_are_meaningfully_more_likely_to_survive_the_same_infection_than_non_founders() {
        const TRIALS: usize = 1000;
        let mut founder_deaths = 0;
        let mut non_founder_deaths = 0;
        for _ in 0..TRIALS {
            let mut founder = Individual { is_founder: true, phenotype: Phenotype { immune_strength: 0.0, ..Default::default() }, health: Health { hp: 0.0, ..Default::default() }, ..infected("plague_like", 10) };
            let mut non_founder = Individual { is_founder: false, phenotype: Phenotype { immune_strength: 0.0, ..Default::default() }, health: Health { hp: 0.0, ..Default::default() }, ..infected("plague_like", 10) };
            let mut founder_pop = vec![founder.clone()];
            let mut non_founder_pop = vec![non_founder.clone()];
            for day in 0..10 {
                process_microbiome_tick(&mut founder_pop, &json!({}), day);
                process_microbiome_tick(&mut non_founder_pop, &json!({}), day);
            }
            founder = founder_pop.into_iter().next().unwrap();
            non_founder = non_founder_pop.into_iter().next().unwrap();
            if founder.is_dead {
                founder_deaths += 1;
            }
            if non_founder.is_dead {
                non_founder_deaths += 1;
            }
        }
        assert!(
            founder_deaths < non_founder_deaths,
            "founders ({founder_deaths}/{TRIALS}) should die less often than non-founders ({non_founder_deaths}/{TRIALS}) to the same infection"
        );
    }

    #[test]
    fn a_lethal_infection_can_kill_a_maximally_vulnerable_individual() {
        // plague_like: base_mortality 0.40, duration 10 days -> daily mortality with
        // zero immunity and zero hp is 0.40 * 1.0 * 1.0 / 10 = 0.04/day.
        let mut deaths = 0;
        for _ in 0..300 {
            let ind = Individual {
                phenotype: Phenotype { immune_strength: 0.0, ..Default::default() },
                health: Health { hp: 0.0, ..Default::default() },
                ..infected("plague_like", 10)
            };
            let mut population = vec![ind];
            for day in 0..10 {
                process_microbiome_tick(&mut population, &json!({}), day);
                if population[0].is_dead {
                    break;
                }
            }
            if population[0].is_dead {
                deaths += 1;
                assert_eq!(population[0].extra["death_cause"], "infection");
            }
        }
        assert!(deaths > 0, "a maximally vulnerable individual should die to plague_like at least once in 300 trials");
    }
}
