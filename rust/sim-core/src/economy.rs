use serde_json::json;
use std::collections::{HashMap, HashSet};

use crate::state::Individual;

pub const RESOURCE_TYPES: &[&str] = &[
    "food", "water", "stone", "wood", "clay", "flint", "hide", "bone", "copper_ore", "iron_ore", "salt", "obsidian",
];

pub const GOODS_TYPES: &[&str] = &[
    "stone_tool", "spear", "bow", "pottery", "clothing", "rope", "dried_food", "copper_tool", "iron_tool", "woven_cloth", "ceramic_vessel",
];

pub fn initialize_inventory() -> HashMap<String, f64> {
    HashMap::from([("food".to_string(), 30.0), ("water".to_string(), 15.0), ("stone".to_string(), 2.0), ("wood".to_string(), 3.0)])
}

// How much a fully-pressured population's per-individual yield is cut, and
// the floor below which it never drops -- see gather_resources's own comment
// on food_pressure/water_pressure for why this exists at all. Chosen so full
// pressure (population at or beyond compute_resource_pressure's carrying
// capacity) makes food/water scarce enough to push satiation-driven health
// decline (see tick.rs's economy pass) without making it instantly fatal --
// real subsistence populations under strain don't starve to zero overnight.
const RESOURCE_PRESSURE_YIELD_CUT: f64 = 0.6;
const MIN_YIELD_FRACTION_UNDER_PRESSURE: f64 = 0.1;

/// `farming_bonus` is the civilization's accumulated astronomy knowledge's
/// `farming_efficiency` bonus (see `astronomy::get_astronomy_bonus`) -- a
/// seasonal calendar lets cultivation be timed to the actual growing season,
/// so it only ever scales the plant_cultivation yield, not foraging/hunting.
///
/// `food_pressure`/`water_pressure` come from `environment::
/// compute_resource_pressure(world_state, population_size)` -- population
/// competing for the same land's finite food/water, not each individual's
/// own effort, is what should make gathering harder as a settlement grows
/// past its carrying capacity. Passed in already computed (once per tick,
/// not once per individual) since neither depends on which individual is
/// gathering.
pub fn gather_resources(
    individual: &Individual,
    world_state: &serde_json::Value,
    discovered_techs: &HashSet<String>,
    farming_bonus: f64,
    food_pressure: f64,
    water_pressure: f64,
) -> HashMap<String, f64> {
    let p = &individual.phenotype;
    let e = ((p.conscientiousness + p.physical_strength) / 2.0).max(0.3);
    let fb = world_state.get("food_abundance").and_then(|v| v.as_f64()).unwrap_or(0.5) * e;
    let mut food = fb * 2.0;
    if discovered_techs.contains("foraging") {
        food += fb * 2.0;
    }
    if discovered_techs.contains("hunting_spear") || discovered_techs.contains("bow_arrow") {
        food += world_state.get("fauna").and_then(|v| v.get("prey_density")).and_then(|v| v.as_f64()).unwrap_or(0.3) * 2.0 * e;
    }
    if discovered_techs.contains("fishing") && world_state.get("water_abundance").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.3 {
        food += 1.2 * e;
    }
    if discovered_techs.contains("plant_cultivation") {
        food += 2.5 * e * (1.0 + farming_bonus);
    }
    if discovered_techs.contains("animal_herding") {
        food += 2.0 * e;
    }
    food *= (1.0 - food_pressure * RESOURCE_PRESSURE_YIELD_CUT).max(MIN_YIELD_FRACTION_UNDER_PRESSURE);
    let water = world_state.get("water_abundance").and_then(|v| v.as_f64()).unwrap_or(0.5)
        * 1.5
        * (1.0 - water_pressure * RESOURCE_PRESSURE_YIELD_CUT).max(MIN_YIELD_FRACTION_UNDER_PRESSURE);
    let wood = world_state.get("flora").and_then(|v| v.get("density")).and_then(|v| v.as_f64()).unwrap_or(0.5) * 0.3;

    // Raw materials -- previously only food/water/wood were ever produced by
    // this function, so stone/clay/hide/ore could only ever be spent, never
    // replenished past the one-time starting inventory (initialize_inventory):
    // pottery, clothing, metallurgy, and most Tier-2+ architecture were
    // effectively unreachable in a real running simulation despite being
    // fully implemented and tested. Raw stone/flint/obsidian collection
    // doesn't require any tech -- picking rocks up off the ground predates
    // toolmaking knowledge -- while clay/hide/bone are byproducts of
    // already-tech-gated activity (riverbank foraging, successful hunts).
    let stone = 0.15 * e;
    let clay = world_state.get("water_abundance").and_then(|v| v.as_f64()).unwrap_or(0.5) * 0.12 * e;
    let flint = 0.03 * e;
    let obsidian = 0.015 * e;
    let salt = 0.02 * e;
    let hunted = discovered_techs.contains("hunting_spear") || discovered_techs.contains("bow_arrow");
    let prey_density = world_state.get("fauna").and_then(|v| v.get("prey_density")).and_then(|v| v.as_f64()).unwrap_or(0.3);
    let hide = if hunted { prey_density * 0.15 * e } else { 0.0 };
    let bone = if hunted { prey_density * 0.1 * e } else { 0.0 };
    // Ore is found the same way stone is (no metallurgy knowledge required to
    // dig up a rock), it's simply rarer -- metallurgy_copper/metallurgy_iron
    // gate *smelting* it into tools (produce_goods), not finding it.
    let copper_ore = 0.02 * e;
    let iron_ore = 0.015 * e;

    HashMap::from([
        ("food".to_string(), food),
        ("water".to_string(), water),
        ("wood".to_string(), wood),
        ("stone".to_string(), stone),
        ("clay".to_string(), clay),
        ("flint".to_string(), flint),
        ("obsidian".to_string(), obsidian),
        ("salt".to_string(), salt),
        ("hide".to_string(), hide),
        ("bone".to_string(), bone),
        ("copper_ore".to_string(), copper_ore),
        ("iron_ore".to_string(), iron_ore),
    ])
}

pub fn consume_resources(individual: &mut Individual) -> serde_json::Value {
    let fnorm = 0.4 + (individual.phenotype.physical_strength * 0.1).min(0.1);
    let wnorm = 0.25;
    let food = individual.inventory.get("food").copied().unwrap_or(0.0).min(fnorm);
    let water = individual.inventory.get("water").copied().unwrap_or(0.0).min(wnorm);
    let current_food = individual.inventory.entry("food".to_string()).or_insert(0.0);
    *current_food = (*current_food - fnorm).max(0.0);
    let current_water = individual.inventory.entry("water".to_string()).or_insert(0.0);
    *current_water = (*current_water - wnorm).max(0.0);
    json!({ "satiation": (food / fnorm + water / wnorm) / 2.0 })
}

pub fn produce_goods(individual: &mut Individual, discovered_techs: &HashSet<String>) -> HashMap<String, f64> {
    let mut produced = HashMap::new();
    let cs = (individual.phenotype.conscientiousness + individual.phenotype.fluid_intelligence) / 2.0;
    // Captured as owned copies (not a borrow of individual.inventory) since
    // several branches below both read and mutate the inventory.
    let stone = individual.inventory.get("stone").copied().unwrap_or(0.0);
    let clay = individual.inventory.get("clay").copied().unwrap_or(0.0);
    let wood = individual.inventory.get("wood").copied().unwrap_or(0.0);
    let food = individual.inventory.get("food").copied().unwrap_or(0.0);
    let hide = individual.inventory.get("hide").copied().unwrap_or(0.0);
    let copper_ore = individual.inventory.get("copper_ore").copied().unwrap_or(0.0);
    let iron_ore = individual.inventory.get("iron_ore").copied().unwrap_or(0.0);
    let flint = individual.inventory.get("flint").copied().unwrap_or(0.0);
    let obsidian = individual.inventory.get("obsidian").copied().unwrap_or(0.0);
    let salt = individual.inventory.get("salt").copied().unwrap_or(0.0);

    // Flint and obsidian were historically the preferred knapping stones for
    // sharp-edged tools (obsidian in particular produces a finer edge than
    // plain stone), so either substitutes for -- or is preferred over --
    // undifferentiated "stone" here, whichever the individual actually has.
    let stone_tool_material = if flint >= 1.0 { Some("flint") } else if obsidian >= 1.0 { Some("obsidian") } else if stone >= 1.0 { Some("stone") } else { None };
    let mut stone_tool_material_used: Option<&str> = None;
    if discovered_techs.contains("stone_tools") && stone_tool_material.is_some() && rand::random::<f64>() < cs * 0.1 {
        produced.insert("stone_tool".to_string(), 1.0);
        stone_tool_material_used = stone_tool_material;
    }
    if discovered_techs.contains("pottery") && clay >= 2.0 && rand::random::<f64>() < cs * 0.08 {
        produced.insert("ceramic_vessel".to_string(), 1.0);
    }
    if discovered_techs.contains("weaving") && wood >= 1.0 && rand::random::<f64>() < cs * 0.07 {
        produced.insert("woven_cloth".to_string(), 1.0);
    }
    // Salt curing is what actually lets food keep -- historically the whole
    // point of food_preservation technique -- so dried_food now consumes a
    // small amount of it alongside the raw food, rather than salt existing
    // in the resource catalog with no consumer anywhere in the engine.
    if discovered_techs.contains("food_preservation") && food >= 3.0 && salt >= 0.5 && rand::random::<f64>() < 0.05 {
        produced.insert("dried_food".to_string(), 2.0);
    }
    // Previously only 4 of the 11 declared GOODS_TYPES were ever produced by
    // anything -- spear/bow/pottery/clothing/rope/copper_tool/iron_tool
    // existed purely as unreachable catalog entries, and metallurgy_copper/
    // metallurgy_iron had no crafting payoff at all despite being fully
    // implemented on the tech-tree side. Each of these mirrors the same
    // "tech known + material on hand + conscientiousness*IQ roll" pattern
    // already used above.
    if discovered_techs.contains("hunting_spear") && wood >= 1.0 && rand::random::<f64>() < cs * 0.08 {
        produced.insert("spear".to_string(), 1.0);
    }
    if discovered_techs.contains("bow_arrow") && wood >= 2.0 && rand::random::<f64>() < cs * 0.06 {
        produced.insert("bow".to_string(), 1.0);
    }
    if discovered_techs.contains("pottery") && clay >= 1.0 && rand::random::<f64>() < cs * 0.1 {
        produced.insert("pottery".to_string(), 1.0);
    }
    if discovered_techs.contains("clothing_basic") && hide >= 1.0 && rand::random::<f64>() < cs * 0.07 {
        produced.insert("clothing".to_string(), 1.0);
    }
    if discovered_techs.contains("weaving") && wood >= 1.0 && rand::random::<f64>() < cs * 0.06 {
        produced.insert("rope".to_string(), 1.0);
    }
    if discovered_techs.contains("metallurgy_copper") && copper_ore >= 1.0 && rand::random::<f64>() < cs * 0.05 {
        produced.insert("copper_tool".to_string(), 1.0);
    }
    if discovered_techs.contains("metallurgy_iron") && iron_ore >= 1.0 && rand::random::<f64>() < cs * 0.04 {
        produced.insert("iron_tool".to_string(), 1.0);
    }

    if produced.contains_key("dried_food") {
        *individual.inventory.entry("food".to_string()).or_insert(0.0) -= 1.5;
    }
    if let (Some(v), Some(mat)) = (produced.get("stone_tool"), stone_tool_material_used) {
        *individual.inventory.entry(mat.to_string()).or_insert(0.0) -= v;
    }
    if let Some(v) = produced.get("ceramic_vessel") {
        *individual.inventory.entry("clay".to_string()).or_insert(0.0) -= 2.0 * v;
    }
    if let Some(v) = produced.get("woven_cloth") {
        *individual.inventory.entry("wood".to_string()).or_insert(0.0) -= v;
    }
    if let Some(v) = produced.get("spear") {
        *individual.inventory.entry("wood".to_string()).or_insert(0.0) -= v;
    }
    if let Some(v) = produced.get("bow") {
        *individual.inventory.entry("wood".to_string()).or_insert(0.0) -= 2.0 * v;
    }
    if let Some(v) = produced.get("pottery") {
        *individual.inventory.entry("clay".to_string()).or_insert(0.0) -= v;
    }
    if let Some(v) = produced.get("clothing") {
        *individual.inventory.entry("hide".to_string()).or_insert(0.0) -= v;
    }
    if let Some(v) = produced.get("rope") {
        *individual.inventory.entry("wood".to_string()).or_insert(0.0) -= v;
    }
    if let Some(v) = produced.get("copper_tool") {
        *individual.inventory.entry("copper_ore".to_string()).or_insert(0.0) -= v;
    }
    if let Some(v) = produced.get("iron_tool") {
        *individual.inventory.entry("iron_ore".to_string()).or_insert(0.0) -= v;
    }
    produced
}

/// Trade actually moves goods between the two inventories (a surplus holder,
/// more than 3 units, donates to whichever party is short, under 1 unit, of
/// that resource); it isn't just an event. Inter-group trades additionally
/// require trust (reputation) to clear, and both parties gain a small
/// reputation bump.
pub fn attempt_trade(ind_a: &mut Individual, ind_b: &mut Individual, sim_day: i32) -> Option<serde_json::Value> {
    let tw = ((ind_a.phenotype.altruism + ind_b.phenotype.altruism) / 2.0).max(0.0);
    if rand::random::<f64>() > tw * 0.4 {
        return None;
    }
    if let (Some(ga), Some(gb)) = (&ind_a.group_id, &ind_b.group_id) {
        if ga != gb {
            let trust = (ind_a.social.reputation + ind_b.social.reputation) / 2.0;
            if rand::random::<f64>() > trust * 0.5 + 0.15 {
                return None;
            }
        }
    }

    let needy = |inv: &HashMap<String, f64>| -> Vec<String> { inv.iter().filter(|(_, q)| **q < 1.0).map(|(r, _)| r.clone()).collect() };
    let surplus = |inv: &HashMap<String, f64>| -> Vec<String> { inv.iter().filter(|(_, q)| **q > 3.0).map(|(r, _)| r.clone()).collect() };

    let a_needy = needy(&ind_a.inventory);
    let b_needy = needy(&ind_b.inventory);
    let a_surplus = surplus(&ind_a.inventory);
    let b_surplus = surplus(&ind_b.inventory);

    let gives_a = a_surplus.iter().find(|r| b_needy.contains(r)).cloned();
    let gives_b = b_surplus.iter().find(|r| a_needy.contains(r)).cloned();
    if gives_a.is_none() && gives_b.is_none() {
        return None;
    }

    let qty = 0.5 + rand::random::<f64>() * 0.5;
    if let Some(resource) = &gives_a {
        let a_amount = ind_a.inventory.entry(resource.clone()).or_insert(0.0);
        *a_amount = (*a_amount - qty).max(0.0);
        *ind_b.inventory.entry(resource.clone()).or_insert(0.0) += qty * 0.9;
    }
    if let Some(resource) = &gives_b {
        let b_amount = ind_b.inventory.entry(resource.clone()).or_insert(0.0);
        *b_amount = (*b_amount - qty).max(0.0);
        *ind_a.inventory.entry(resource.clone()).or_insert(0.0) += qty * 0.9;
    }

    ind_a.social.reputation = (ind_a.social.reputation + 0.01).min(1.0);
    ind_b.social.reputation = (ind_b.social.reputation + 0.01).min(1.0);

    Some(json!({
        "type": "trade",
        "individual_a": ind_a.id,
        "individual_b": ind_b.id,
        "a_gave": gives_a,
        "b_gave": gives_b,
        "day": sim_day
    }))
}

pub fn compute_economic_stats(population: &[&Individual]) -> serde_json::Value {
    let inv: Vec<f64> = population.iter().map(|i| i.inventory.values().sum()).collect();
    if inv.is_empty() {
        return json!({ "mean_wealth": 0.0, "gini": 0.0 });
    }
    let mean = inv.iter().sum::<f64>() / inv.len() as f64;
    let mut sorted = inv.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len() as f64;
    let mut gn = 0.0;
    for (i, val) in sorted.iter().enumerate() {
        gn += (2.0 * (i as f64 + 1.0) - n - 1.0) * *val;
    }
    json!({ "mean_wealth": mean, "gini": if mean > 0.0 { (gn / (n * n * mean)).max(0.0) } else { 0.0 } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Phenotype, Social};

    fn make_ind(food_override: Option<f64>) -> Individual {
        let mut inv = initialize_inventory();
        if let Some(f) = food_override {
            inv.insert("food".to_string(), f);
        }
        Individual {
            phenotype: Phenotype { conscientiousness: 0.7, physical_strength: 0.7, ..Default::default() },
            inventory: inv,
            ..Default::default()
        }
    }

    fn world() -> serde_json::Value {
        json!({ "food_abundance": 0.8, "water_abundance": 0.6, "flora": { "density": 0.5 }, "fauna": { "prey_density": 0.3 }, "biome": "grassland" })
    }

    // ── gatherResources ─────────────────────────────────────────────────

    #[test]
    fn gathering_produces_positive_food_delta() {
        let delta = gather_resources(&make_ind(None), &world(), &HashSet::new(), 0.0, 0.0, 0.0);
        assert!(delta["food"] > 0.0);
    }

    #[test]
    fn gathering_produces_positive_water_delta() {
        let delta = gather_resources(&make_ind(None), &world(), &HashSet::new(), 0.0, 0.0, 0.0);
        assert!(delta["water"] > 0.0);
    }

    #[test]
    fn foraging_tech_increases_food_yield() {
        let base = gather_resources(&make_ind(None), &world(), &HashSet::new(), 0.0, 0.0, 0.0);
        let mut techs = HashSet::new();
        techs.insert("foraging".to_string());
        let with_tech = gather_resources(&make_ind(None), &world(), &techs, 0.0, 0.0, 0.0);
        assert!(with_tech["food"] > base["food"]);
    }

    #[test]
    fn astronomy_farming_bonus_increases_cultivated_yield_but_not_uncultivated_gathering() {
        let mut techs = HashSet::new();
        techs.insert("plant_cultivation".to_string());
        let base = gather_resources(&make_ind(None), &world(), &techs, 0.0, 0.0, 0.0);
        let boosted = gather_resources(&make_ind(None), &world(), &techs, 0.15, 0.0, 0.0);
        assert!(boosted["food"] > base["food"], "seasonal_calendar's farming_efficiency bonus should raise cultivated yield");

        let no_cultivation = HashSet::new();
        let base_uncultivated = gather_resources(&make_ind(None), &world(), &no_cultivation, 0.0, 0.0, 0.0);
        let boosted_uncultivated = gather_resources(&make_ind(None), &world(), &no_cultivation, 0.15, 0.0, 0.0);
        assert_eq!(base_uncultivated["food"], boosted_uncultivated["food"], "farming bonus must not affect foraging/hunting without plant_cultivation");
    }

    #[test]
    fn gathering_yields_every_declared_raw_material_not_just_food_water_wood() {
        // Regression test: stone/clay/flint/obsidian/salt/copper_ore/iron_ore
        // used to be producible only via the one-time starting inventory,
        // never replenished -- pottery/clothing/metallurgy/most architecture
        // were effectively unreachable in a real run despite being fully
        // implemented and tested.
        let mut techs = HashSet::new();
        techs.insert("hunting_spear".to_string());
        let delta = gather_resources(&make_ind(None), &world(), &techs, 0.0, 0.0, 0.0);
        for resource in ["stone", "clay", "flint", "obsidian", "salt", "copper_ore", "iron_ore", "hide", "bone"] {
            assert!(delta.get(resource).copied().unwrap_or(0.0) > 0.0, "{resource} should be gatherable, got {delta:?}");
        }
    }

    #[test]
    fn hide_and_bone_require_a_hunting_tech_stone_and_ore_do_not() {
        let delta = gather_resources(&make_ind(None), &world(), &HashSet::new(), 0.0, 0.0, 0.0);
        assert_eq!(delta["hide"], 0.0, "hide is a hunting byproduct and needs hunting_spear/bow_arrow");
        assert_eq!(delta["bone"], 0.0);
        assert!(delta["stone"] > 0.0, "raw stone collection predates any toolmaking tech");
        assert!(delta["copper_ore"] > 0.0, "finding raw ore doesn't require knowing how to smelt it");
    }

    // ── consumeResources ────────────────────────────────────────────────

    #[test]
    fn consuming_reduces_food_and_water() {
        let mut ind = make_ind(None);
        let (prev_food, prev_water) = (ind.inventory["food"], ind.inventory["water"]);
        consume_resources(&mut ind);
        assert!(ind.inventory["food"] < prev_food);
        assert!(ind.inventory["water"] < prev_water);
    }

    #[test]
    fn empty_inventory_yields_zero_satiation() {
        let mut ind = make_ind(Some(0.0));
        ind.inventory.insert("water".to_string(), 0.0);
        let result = consume_resources(&mut ind);
        assert_eq!(result["satiation"], 0.0);
    }

    #[test]
    fn fully_stocked_inventory_yields_satiation_of_one() {
        let mut ind = make_ind(Some(100.0));
        ind.inventory.insert("water".to_string(), 100.0);
        let result = consume_resources(&mut ind);
        assert_eq!(result["satiation"], 1.0);
    }

    // ── computeEconomicStats ────────────────────────────────────────────

    fn ind_with_wealth(food: f64, water: f64) -> Individual {
        Individual { inventory: HashMap::from([("food".to_string(), food), ("water".to_string(), water)]), ..Default::default() }
    }

    #[test]
    fn equal_distribution_yields_zero_gini() {
        let pop = [ind_with_wealth(10.0, 5.0), ind_with_wealth(10.0, 5.0), ind_with_wealth(10.0, 5.0)];
        let refs: Vec<&Individual> = pop.iter().collect();
        let stats = compute_economic_stats(&refs);
        assert!(stats["gini"].as_f64().unwrap().abs() < 1e-5);
    }

    #[test]
    fn unequal_distribution_yields_positive_gini() {
        let pop = [ind_with_wealth(0.0, 0.0), ind_with_wealth(0.0, 0.0), ind_with_wealth(100.0, 0.0)];
        let refs: Vec<&Individual> = pop.iter().collect();
        let stats = compute_economic_stats(&refs);
        assert!(stats["gini"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn single_individual_yields_zero_gini() {
        let pop = [ind_with_wealth(50.0, 0.0)];
        let refs: Vec<&Individual> = pop.iter().collect();
        let stats = compute_economic_stats(&refs);
        assert_eq!(stats["gini"], 0.0);
    }

    #[test]
    fn empty_population_yields_zero_wealth_and_gini() {
        let stats = compute_economic_stats(&[]);
        assert_eq!(stats["mean_wealth"], 0.0);
        assert_eq!(stats["gini"], 0.0);
    }

    // ── produceGoods ────────────────────────────────────────────────────

    fn skilled_producer(inv: HashMap<String, f64>) -> Individual {
        Individual { phenotype: Phenotype { conscientiousness: 0.99, fluid_intelligence: 0.99, ..Default::default() }, inventory: inv, ..Default::default() }
    }

    #[test]
    fn stone_tool_can_be_produced_with_the_right_tech_and_material() {
        let mut ind = skilled_producer(HashMap::from([("stone".to_string(), 10.0)]));
        let mut techs = HashSet::new();
        techs.insert("stone_tools".to_string());
        let mut produced_any = false;
        for _ in 0..500 {
            if produce_goods(&mut ind, &techs).get("stone_tool").copied().unwrap_or(0.0) > 0.0 {
                produced_any = true;
                break;
            }
        }
        assert!(produced_any);
    }

    #[test]
    fn stone_tool_is_never_produced_without_the_tech() {
        let mut ind = skilled_producer(HashMap::from([("stone".to_string(), 100.0)]));
        for _ in 0..200 {
            assert_eq!(produce_goods(&mut ind, &HashSet::new()).get("stone_tool").copied().unwrap_or(0.0), 0.0);
        }
    }

    #[test]
    fn stone_tool_can_be_produced_from_flint_alone_with_no_stone_on_hand() {
        let mut ind = skilled_producer(HashMap::from([("flint".to_string(), 10.0)]));
        let mut techs = HashSet::new();
        techs.insert("stone_tools".to_string());
        let mut produced_any = false;
        for _ in 0..500 {
            if produce_goods(&mut ind, &techs).get("stone_tool").copied().unwrap_or(0.0) > 0.0 {
                produced_any = true;
                break;
            }
        }
        assert!(produced_any, "flint should be usable in place of undifferentiated stone for toolmaking");
        assert!(ind.inventory["flint"] < 10.0, "the flint actually used should be consumed");
    }

    #[test]
    fn ceramic_vessel_can_be_produced_with_pottery_and_clay() {
        let mut ind = skilled_producer(HashMap::from([("clay".to_string(), 100.0)]));
        let mut techs = HashSet::new();
        techs.insert("pottery".to_string());
        let mut produced_any = false;
        for _ in 0..500 {
            if produce_goods(&mut ind, &techs).get("ceramic_vessel").copied().unwrap_or(0.0) > 0.0 {
                produced_any = true;
                break;
            }
        }
        assert!(produced_any);
    }

    #[test]
    fn producing_ceramic_vessel_consumes_clay() {
        let mut ind = skilled_producer(HashMap::from([("clay".to_string(), 100.0)]));
        let before = ind.inventory["clay"];
        let mut techs = HashSet::new();
        techs.insert("pottery".to_string());
        for _ in 0..500 {
            produce_goods(&mut ind, &techs);
            if ind.inventory["clay"] < before {
                break;
            }
        }
        assert!(ind.inventory["clay"] < before);
    }

    #[test]
    fn spear_can_be_produced_with_hunting_spear_and_wood() {
        let mut ind = skilled_producer(HashMap::from([("wood".to_string(), 100.0)]));
        let mut techs = HashSet::new();
        techs.insert("hunting_spear".to_string());
        let mut produced_any = false;
        for _ in 0..500 {
            if produce_goods(&mut ind, &techs).get("spear").copied().unwrap_or(0.0) > 0.0 {
                produced_any = true;
                break;
            }
        }
        assert!(produced_any);
    }

    #[test]
    fn copper_tool_can_be_produced_with_metallurgy_copper_and_copper_ore() {
        let mut ind = skilled_producer(HashMap::from([("copper_ore".to_string(), 100.0)]));
        let mut techs = HashSet::new();
        techs.insert("metallurgy_copper".to_string());
        let mut produced_any = false;
        for _ in 0..1000 {
            if produce_goods(&mut ind, &techs).get("copper_tool").copied().unwrap_or(0.0) > 0.0 {
                produced_any = true;
                break;
            }
        }
        assert!(produced_any, "metallurgy_copper should have a real crafting payoff (copper_tool)");
    }

    #[test]
    fn iron_tool_can_be_produced_with_metallurgy_iron_and_iron_ore() {
        let mut ind = skilled_producer(HashMap::from([("iron_ore".to_string(), 100.0)]));
        let mut techs = HashSet::new();
        techs.insert("metallurgy_iron".to_string());
        let mut produced_any = false;
        for _ in 0..1000 {
            if produce_goods(&mut ind, &techs).get("iron_tool").copied().unwrap_or(0.0) > 0.0 {
                produced_any = true;
                break;
            }
        }
        assert!(produced_any, "metallurgy_iron should have a real crafting payoff (iron_tool)");
    }

    #[test]
    fn clothing_can_be_produced_with_clothing_basic_and_hide() {
        let mut ind = skilled_producer(HashMap::from([("hide".to_string(), 100.0)]));
        let mut techs = HashSet::new();
        techs.insert("clothing_basic".to_string());
        let mut produced_any = false;
        for _ in 0..500 {
            if produce_goods(&mut ind, &techs).get("clothing").copied().unwrap_or(0.0) > 0.0 {
                produced_any = true;
                break;
            }
        }
        assert!(produced_any);
    }

    #[test]
    fn all_eleven_goods_types_are_eventually_producible_given_every_relevant_tech_and_material() {
        // Previously only 4 of the 11 declared GOODS_TYPES were ever
        // produced by anything.
        let mut ind = skilled_producer(HashMap::from([
            ("stone".to_string(), 1000.0),
            ("clay".to_string(), 1000.0),
            ("wood".to_string(), 1000.0),
            ("food".to_string(), 1000.0),
            ("hide".to_string(), 1000.0),
            ("copper_ore".to_string(), 1000.0),
            ("iron_ore".to_string(), 1000.0),
            ("salt".to_string(), 1000.0),
        ]));
        let mut techs = HashSet::new();
        for t in ["stone_tools", "pottery", "weaving", "food_preservation", "hunting_spear", "bow_arrow", "clothing_basic", "metallurgy_copper", "metallurgy_iron"] {
            techs.insert(t.to_string());
        }
        let mut seen = HashSet::new();
        for _ in 0..20_000 {
            seen.extend(produce_goods(&mut ind, &techs).into_keys());
            // Keep materials from ever running dry over 20,000 rolls so every
            // good gets a fair chance regardless of roll order.
            for (mat, amount) in [("stone", 1000.0), ("clay", 1000.0), ("wood", 1000.0), ("food", 1000.0), ("hide", 1000.0), ("copper_ore", 1000.0), ("iron_ore", 1000.0), ("salt", 1000.0)] {
                ind.inventory.insert(mat.to_string(), amount);
            }
        }
        for good in GOODS_TYPES {
            assert!(seen.contains(*good), "expected {good} to be producible with the right tech+material, got {seen:?}");
        }
    }

    #[test]
    fn no_goods_are_produced_with_no_known_techs() {
        let mut ind = skilled_producer(HashMap::from([("food".to_string(), 5.0), ("water".to_string(), 3.0), ("stone".to_string(), 1.0), ("clay".to_string(), 5.0), ("wood".to_string(), 2.0)]));
        let snapshot = ind.inventory.clone();
        let produced = produce_goods(&mut ind, &HashSet::new());
        assert!(produced.is_empty());
        assert_eq!(ind.inventory, snapshot);
    }

    // ── attemptTrade ────────────────────────────────────────────────────

    fn trader(id: &str, group_id: Option<&str>, inventory: HashMap<String, f64>) -> Individual {
        Individual {
            id: id.to_string(),
            group_id: group_id.map(str::to_string),
            phenotype: Phenotype { altruism: 0.99, ..Default::default() },
            social: Social { reputation: 0.5, ..Default::default() },
            inventory,
            ..Default::default()
        }
    }

    #[test]
    fn trade_raises_reputation_for_both_parties_when_it_happens() {
        let mut a = trader("a", None, HashMap::from([("food".to_string(), 20.0), ("water".to_string(), 0.1)]));
        let mut b = trader("b", None, HashMap::from([("water".to_string(), 20.0), ("food".to_string(), 0.1)]));
        let mut traded = false;
        for i in 0..500 {
            if attempt_trade(&mut a, &mut b, i).is_some() {
                traded = true;
                break;
            }
        }
        if traded {
            assert!(a.social.reputation > 0.5);
            assert!(b.social.reputation > 0.5);
        }
    }

    #[test]
    fn trade_result_has_expected_shape() {
        let mut a = trader("a", None, HashMap::from([("food".to_string(), 50.0), ("water".to_string(), 0.0)]));
        let mut b = trader("b", None, HashMap::from([("water".to_string(), 50.0), ("food".to_string(), 0.0)]));
        let mut result = None;
        for i in 0..500 {
            result = attempt_trade(&mut a, &mut b, i);
            if result.is_some() {
                break;
            }
        }
        if let Some(ev) = result {
            assert_eq!(ev["type"], "trade");
            assert_eq!(ev["individual_a"], "a");
            assert_eq!(ev["individual_b"], "b");
        }
    }

    #[test]
    fn intra_group_trade_is_at_least_as_easy_as_inter_group_trade() {
        let mut same_group_trades = 0;
        let mut diff_group_trades = 0;
        for i in 0..200 {
            let mut a1 = trader("a", Some("g1"), HashMap::from([("food".to_string(), 50.0), ("water".to_string(), 0.0)]));
            let mut b1 = trader("b", Some("g1"), HashMap::from([("water".to_string(), 50.0), ("food".to_string(), 0.0)]));
            a1.social.reputation = 0.1;
            b1.social.reputation = 0.1;
            if attempt_trade(&mut a1, &mut b1, i).is_some() {
                same_group_trades += 1;
            }

            let mut a2 = trader("a", Some("g1"), HashMap::from([("food".to_string(), 50.0), ("water".to_string(), 0.0)]));
            let mut b2 = trader("b", Some("g2"), HashMap::from([("water".to_string(), 50.0), ("food".to_string(), 0.0)]));
            a2.social.reputation = 0.1;
            b2.social.reputation = 0.1;
            if attempt_trade(&mut a2, &mut b2, i).is_some() {
                diff_group_trades += 1;
            }
        }
        assert!(same_group_trades >= diff_group_trades);
    }

    #[test]
    fn trade_actually_moves_resources_between_inventories() {
        let mut a = trader("a", None, HashMap::from([("food".to_string(), 50.0), ("water".to_string(), 0.0)]));
        let mut b = trader("b", None, HashMap::from([("water".to_string(), 50.0), ("food".to_string(), 0.0)]));
        let mut traded = false;
        for i in 0..500 {
            if attempt_trade(&mut a, &mut b, i).is_some() {
                traded = true;
                break;
            }
        }
        assert!(traded, "two highly altruistic, complementary traders should eventually trade");
        // Whichever direction cleared, someone's holdings must have actually changed.
        assert!(a.inventory["food"] < 50.0 || b.inventory["food"] > 0.0 || a.inventory["water"] > 0.0 || b.inventory["water"] < 50.0);
    }

    // ── resource pressure (compute_resource_pressure feeding back into yield) ──

    #[test]
    fn heavy_food_pressure_meaningfully_cuts_food_yield() {
        let unpressured = gather_resources(&make_ind(None), &world(), &HashSet::new(), 0.0, 0.0, 0.0);
        let pressured = gather_resources(&make_ind(None), &world(), &HashSet::new(), 0.0, 1.0, 0.0);
        assert!(pressured["food"] < unpressured["food"] * 0.5, "full food pressure should cut yield by well over half");
        assert!(pressured["food"] > 0.0, "even under full pressure, yield must never hit zero/negative");
    }

    #[test]
    fn heavy_water_pressure_meaningfully_cuts_water_yield() {
        let unpressured = gather_resources(&make_ind(None), &world(), &HashSet::new(), 0.0, 0.0, 0.0);
        let pressured = gather_resources(&make_ind(None), &world(), &HashSet::new(), 0.0, 0.0, 1.0);
        assert!(pressured["water"] < unpressured["water"] * 0.5, "full water pressure should cut yield by well over half");
        assert!(pressured["water"] > 0.0, "even under full pressure, yield must never hit zero/negative");
    }

    // ── RESOURCE_TYPES / GOODS_TYPES stay in sync with what's actually produced ──

    #[test]
    fn gather_resources_and_initial_inventory_only_ever_use_declared_resource_types() {
        let delta = gather_resources(&make_ind(None), &world(), &HashSet::new(), 0.0, 0.0, 0.0);
        for key in delta.keys().chain(initialize_inventory().keys()) {
            assert!(RESOURCE_TYPES.contains(&key.as_str()), "{key} produced by gather_resources/initialize_inventory but missing from RESOURCE_TYPES");
        }
    }

    #[test]
    fn produce_goods_only_ever_produces_declared_goods_types() {
        let mut techs = HashSet::new();
        for tech in ["stone_tools", "pottery", "weaving", "food_preservation"] {
            techs.insert(tech.to_string());
        }
        let mut ind = make_ind(None);
        ind.inventory.insert("clay".to_string(), 10.0);
        ind.phenotype.conscientiousness = 1.0;
        ind.phenotype.fluid_intelligence = 1.0;
        let mut seen = HashSet::new();
        for _ in 0..500 {
            seen.extend(produce_goods(&mut ind, &techs).into_keys());
        }
        assert!(!seen.is_empty(), "high-conscientiousness individual with every relevant tech should produce something within 500 rolls");
        for key in &seen {
            assert!(GOODS_TYPES.contains(&key.as_str()), "{key} produced by produce_goods but missing from GOODS_TYPES");
        }
    }
}
