use std::collections::HashSet;

use serde_json::{json, Value};

use crate::biology::individual::get_life_stage;
use crate::state::Individual;

/// (id, tier, capacity, requires_tech, materials, labor_days, durability, purpose)
#[allow(clippy::type_complexity)]
pub const STRUCTURE_TYPES: &[(&str, i32, usize, &[&str], &[&str], i32, f64, Option<&str>)] = &[
    ("cave_dwelling", 0, 8, &[], &[], 0, 1.0, None),
    ("lean_to", 0, 4, &[], &["wood"], 1, 0.2, None),
    ("pit_house", 1, 6, &["stone_tools"], &["wood", "stone"], 5, 0.5, None),
    ("post_frame_hut", 1, 6, &["stone_tools"], &["wood"], 4, 0.4, None),
    ("storage_pit", 1, 0, &["stone_tools"], &[], 2, 0.7, Some("storage")),
    ("mud_brick_house", 2, 8, &["pottery", "plant_cultivation"], &["clay"], 15, 0.7, None),
    ("granary", 2, 0, &["plant_cultivation", "pottery"], &["clay", "wood"], 10, 0.6, Some("granary")),
    ("defensive_wall", 2, 0, &["stone_tools"], &["stone", "wood"], 20, 0.8, Some("defense")),
    // Tier bumped 3 -> 4 to match these four structures' own tech
    // prerequisites: architecture_stone/wheel are both Tier-4 techs (see
    // technology.rs's TECH_TREE), so a "Tier 3" structure requiring a
    // Tier-4 tech was an internally inconsistent stale numbering.
    ("stone_temple", 4, 50, &["architecture_stone", "metallurgy_copper"], &["stone"], 200, 1.0, Some("ritual")),
    ("stone_house", 4, 10, &["architecture_stone"], &["stone"], 30, 1.0, None),
    ("marketplace", 4, 100, &["wheel", "writing_system"], &["stone", "wood"], 50, 0.9, Some("trade")),
    ("city_wall", 4, 0, &["architecture_stone", "metallurgy_copper"], &["stone"], 500, 1.0, Some("defense")),
];

const BUILD_MATERIALS: &[&str] = &["wood", "stone", "clay", "flint", "bone", "hide"];
const KEEP_THRESHOLD: f64 = 2.0;

fn has_materials(settlement: &Value, materials: &[&str]) -> bool {
    materials.iter().all(|m| settlement.get("stockpile").and_then(|s| s.get(m)).and_then(Value::as_f64).unwrap_or(0.0) >= 1.0)
}

fn consume_materials(settlement: &mut Value, materials: &[&str]) {
    if let Some(stock) = settlement.get_mut("stockpile").and_then(Value::as_object_mut) {
        for m in materials {
            let current = stock.get(*m).and_then(Value::as_f64).unwrap_or(0.0);
            stock.insert((*m).to_string(), json!((current - 1.0).max(0.0)));
        }
    }
}

/// `group_members` is this settlement's owning group's membership (alive
/// only, by `individual.group_id`), already resolved by the caller (see
/// tick.rs) -- letting this filter the *entire* population itself, once per
/// settlement, was an O(settlements * population) cost that got sharply
/// worse as both grew over a long run.
pub fn process_architecture_tick(settlement: &mut Value, group_members: &mut [&mut Individual], discovered_techs: &HashSet<String>, world_state: &Value, sim_day: i32) -> Vec<Value> {
    let mut events = Vec::new();
    if settlement.get("group_id").and_then(Value::as_str).is_none() {
        return events;
    }
    if settlement.get("structures").and_then(Value::as_array).is_none() {
        settlement["structures"] = json!([]);
    }
    if settlement.get("stockpile").and_then(Value::as_object).is_none() {
        settlement["stockpile"] = json!({});
    }

    let group_size = group_members.len();

    // Labor accrues from adults only (life stage is derived from age, not
    // a scripted tag): fixes a latent bug where labor never accumulated
    // because nothing ever wrote a "life_stage" field onto individuals.
    let labor: f64 = group_members.iter().filter(|m| get_life_stage(m, sim_day) == "adult").count() as f64 * 0.1;
    let current_labor = settlement.get("labor_pool").and_then(Value::as_f64).unwrap_or(0.0);
    settlement["labor_pool"] = json!(current_labor + labor);

    // Members donate surplus building materials above KEEP_THRESHOLD into the
    // settlement's shared stockpile; the donated amount is actually deducted
    // from their own inventory (not just credited to the stockpile), so
    // materials aren't duplicated out of thin air.
    let mut donated: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();
    for ind in group_members.iter_mut() {
        for mat in BUILD_MATERIALS {
            let held = ind.inventory.get(*mat).copied().unwrap_or(0.0);
            if held > KEEP_THRESHOLD {
                *donated.entry(mat).or_insert(0.0) += held - KEEP_THRESHOLD;
                ind.inventory.insert((*mat).to_string(), KEEP_THRESHOLD);
            }
        }
    }
    if let Some(stock) = settlement.get_mut("stockpile").and_then(Value::as_object_mut) {
        for (mat, amount) in donated {
            let current = stock.get(mat).and_then(Value::as_f64).unwrap_or(0.0);
            stock.insert(mat.to_string(), json!(current + amount));
        }
    }

    let priority = build_priority(settlement, group_size, world_state, discovered_techs);
    if let Some(id) = priority {
        if let Some((_, tier, _, requires_tech, materials, labor_days, _, _)) = STRUCTURE_TYPES.iter().find(|(sid, ..)| *sid == id) {
            if requires_tech.iter().all(|t| discovered_techs.contains(*t)) {
                let current_labor = settlement.get("labor_pool").and_then(Value::as_f64).unwrap_or(0.0);
                if current_labor >= *labor_days as f64 && has_materials(settlement, materials) {
                    consume_materials(settlement, materials);
                    settlement["labor_pool"] = json!(current_labor - *labor_days as f64);
                    if let Some(arr) = settlement.get_mut("structures").and_then(Value::as_array_mut) {
                        arr.push(json!({ "id": format!("struct_{}_{}", sim_day, rand::random::<u16>()), "type": id, "built_day": sim_day, "condition": 1.0 }));
                    }
                    events.push(json!({
                        "type": "structure_built",
                        "structure_type": id,
                        "settlement_id": settlement.get("id").cloned().unwrap_or(Value::Null),
                        "day": sim_day,
                        "importance": if *tier >= 3 { "high" } else { "medium" },
                        "description": format!("The settlement completed construction of {id}"),
                    }));
                }
            }
        }
    }

    // Structures decay every tick (more durable types decay slower) and are
    // demolished once they fall below a minimal upkeep threshold.
    if let Some(arr) = settlement.get_mut("structures").and_then(Value::as_array_mut) {
        for s in arr.iter_mut() {
            let structure_type = s.get("type").and_then(Value::as_str).unwrap_or("");
            let durability = STRUCTURE_TYPES.iter().find(|(id, ..)| *id == structure_type).map(|(_, _, _, _, _, _, d, _)| *d);
            let decay = durability.map(|d| (1.0 - d) * 0.001).unwrap_or(0.002);
            let condition = s.get("condition").and_then(Value::as_f64).unwrap_or(1.0);
            s["condition"] = json!((condition - decay).max(0.0));
        }
        arr.retain(|s| s.get("condition").and_then(Value::as_f64).unwrap_or(1.0) > 0.05);
    }

    events
}

fn build_priority(settlement: &Value, group_size: usize, world_state: &Value, discovered_techs: &HashSet<String>) -> Option<&'static str> {
    let structures = settlement.get("structures").and_then(Value::as_array).cloned().unwrap_or_default();
    let built: HashSet<&str> = structures.iter().filter_map(|s| s.get("type").and_then(Value::as_str)).collect();
    let cap: usize = structures.iter().filter_map(|s| s.get("type").and_then(Value::as_str)).filter_map(|id| STRUCTURE_TYPES.iter().find(|(sid, ..)| *sid == id).map(|(_, _, cap, ..)| *cap)).sum();
    let has_any_shelter = built.contains("lean_to") || built.contains("pit_house") || built.contains("post_frame_hut") || built.contains("mud_brick_house") || built.contains("stone_house");

    // Needs a shelter of some kind, or more housing capacity than currently
    // built. Prefer the best shelter type discovered tech actually unlocks
    // (stone_house > pit_house > post_frame_hut) over the always-available,
    // no-tech-required lean_to -- previously pit_house/stone_house could
    // never be selected here at all (this branch always proposed
    // post_frame_hut/lean_to outright), so those two structure types were
    // permanently unbuildable regardless of tech level.
    if !has_any_shelter || cap < (group_size as f64 * 0.7) as usize {
        if discovered_techs.contains("architecture_stone") && !built.contains("stone_house") {
            return Some("stone_house");
        }
        if discovered_techs.contains("stone_tools") && !built.contains("pit_house") {
            return Some("pit_house");
        }
        if discovered_techs.contains("stone_tools") && !built.contains("post_frame_hut") {
            return Some("post_frame_hut");
        }
        // No sturdier tech-gated shelter available yet: a fresh settlement
        // always has somewhere to sleep, tech or no tech. Once *some*
        // shelter exists, don't keep re-proposing lean_to just because
        // capacity is still tight and nothing better has unlocked yet --
        // wait for stone_tools/architecture_stone instead of spamming lean_tos.
        if !has_any_shelter {
            return Some("lean_to");
        }
    }
    if group_size >= 6 && !built.contains("storage_pit") {
        return Some("storage_pit");
    }
    if group_size >= 8 && !built.contains("mud_brick_house") {
        return Some("mud_brick_house");
    }
    if group_size >= 10 && !built.contains("granary") {
        return Some("granary");
    }
    if world_state.get("recent_disaster").and_then(Value::as_str) == Some("conflict") {
        // city_wall (tier-4 tech, no durability decay concerns at 1.0) is
        // strictly better than defensive_wall once its tech is available --
        // previously city_wall could never be selected at all.
        if discovered_techs.contains("architecture_stone") && discovered_techs.contains("metallurgy_copper") && !built.contains("city_wall") {
            return Some("city_wall");
        }
        if !built.contains("defensive_wall") {
            return Some("defensive_wall");
        }
    }
    if group_size >= 15 && !built.contains("marketplace") {
        return Some("marketplace");
    }
    if group_size >= 20 && !built.contains("stone_temple") {
        return Some("stone_temple");
    }
    None
}

pub fn compute_settlement_capacity(settlement: &Value) -> usize {
    settlement
        .get("structures")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.get("type").and_then(Value::as_str))
                .filter_map(|id| STRUCTURE_TYPES.iter().find(|(sid, ..)| *sid == id).map(|(_, _, cap, ..)| *cap))
                .sum()
        })
        .unwrap_or(0)
}

// Edge-triggered, not level-triggered: without `overcrowded_reported`, this
// fired every single day the settlement stayed over capacity (population
// doesn't shrink on its own and building a bigger structure can take a
// while), flooding the event log/report with dozens of identical daily
// entries instead of the one-time notices every other architecture/social
// event gives. The flag is cleared once the settlement is no longer
// overcrowded (new capacity built, or the group shrank/moved away), so a
// later recurrence is reported again as its own new event.
pub fn check_settlement_overcrowding(settlement: &mut Value, group_size: usize, sim_day: i32) -> Option<Value> {
    let cap = compute_settlement_capacity(settlement);
    let is_overcrowded = cap > 0 && (group_size as f64) > cap as f64 * 1.2;
    let already_reported = settlement.get("overcrowded_reported").and_then(Value::as_bool).unwrap_or(false);
    if is_overcrowded == already_reported {
        return None;
    }
    if let Some(obj) = settlement.as_object_mut() {
        obj.insert("overcrowded_reported".to_string(), json!(is_overcrowded));
    }
    if is_overcrowded {
        Some(json!({
            "type": "settlement_overcrowded",
            "settlement_id": settlement.get("id").cloned().unwrap_or(Value::Null),
            "current": group_size,
            "capacity": cap,
            "day": sim_day,
            "importance": "medium",
            "description": format!("The settlement is overcrowded ({group_size} of {cap} capacity)"),
        }))
    } else {
        None
    }
}

pub fn compute_settlement_defense(settlement: &Value) -> f64 {
    settlement
        .get("structures")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter(|s| {
            s.get("type").and_then(Value::as_str) == Some("defensive_wall")
                || s.get("type").and_then(Value::as_str) == Some("city_wall")
        }).map(|s| s.get("condition").and_then(Value::as_f64).unwrap_or(1.0) * 0.5).sum())
        .unwrap_or(0.0)
}

pub fn create_settlement(group: &Value, world_state: &Value, sim_day: i32) -> Value {
    json!({
        "id": format!("settlement_{}_{}", sim_day, rand::random::<u16>()),
        "name": Value::Null,
        "group_id": group.get("id").cloned().unwrap_or(Value::Null),
        "x": group.get("territory").and_then(|v| v.get("x")).cloned().unwrap_or(Value::Null),
        "y": group.get("territory").and_then(|v| v.get("y")).cloned().unwrap_or(Value::Null),
        "biome": world_state.get("biome").cloned().unwrap_or(Value::Null),
        "structures": [],
        "labor_pool": 0,
        "stockpile": {},
        "founded_day": sim_day
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_settlement() -> Value {
        json!({
            "id": "settlement-1",
            "name": "Test Camp",
            "group_id": "g1",
            "x": 32, "y": 38,
            "biome": "mediterranean",
            "structures": [],
            "labor_pool": 0,
            "stockpile": {},
            "founded_day": 0,
        })
    }

    fn make_ind(id: &str, age_years: i32) -> Individual {
        Individual {
            id: id.to_string(),
            is_dead: false,
            group_id: Some("g1".to_string()),
            birth_day: -age_years * 365,
            age_days: Some(age_years * 365),
            inventory: HashMap::from([("food".to_string(), 5.0), ("water".to_string(), 3.0), ("wood".to_string(), 10.0), ("stone".to_string(), 10.0), ("clay".to_string(), 5.0)]),
            ..Default::default()
        }
    }

    const WORLD: fn() -> Value = || json!({ "food_abundance": 0.6, "biome": "mediterranean" });

    // ── STRUCTURE_TYPES ─────────────────────────────────────────────────

    #[test]
    fn defines_twelve_structure_types() {
        assert_eq!(STRUCTURE_TYPES.len(), 12);
    }

    #[test]
    fn cave_dwelling_requires_no_tech_and_no_labor() {
        let (_, _, _, requires_tech, _, labor_days, ..) = STRUCTURE_TYPES.iter().find(|(id, ..)| *id == "cave_dwelling").unwrap();
        assert!(requires_tech.is_empty());
        assert_eq!(*labor_days, 0);
    }

    #[test]
    fn city_wall_is_the_most_labor_intensive_structure() {
        let max_labor = STRUCTURE_TYPES.iter().map(|(_, _, _, _, _, labor, ..)| *labor).max().unwrap();
        let city_wall_labor = STRUCTURE_TYPES.iter().find(|(id, ..)| *id == "city_wall").unwrap().5;
        assert_eq!(city_wall_labor, max_labor);
    }

    #[test]
    fn stone_temple_requires_architecture_stone() {
        let (_, _, _, requires_tech, ..) = STRUCTURE_TYPES.iter().find(|(id, ..)| *id == "stone_temple").unwrap();
        assert!(requires_tech.contains(&"architecture_stone"));
    }

    // ── createSettlement ────────────────────────────────────────────────

    #[test]
    fn creates_settlement_with_correct_group_id_and_position() {
        let group = json!({ "id": "g1", "territory": { "x": 30, "y": 40 } });
        let s = create_settlement(&group, &WORLD(), 1);
        assert_eq!(s["group_id"], "g1");
        assert_eq!(s["x"], 30);
        assert_eq!(s["y"], 40);
    }

    #[test]
    fn starts_with_empty_structures_and_stockpile() {
        let group = json!({ "id": "g1", "territory": { "x": 0, "y": 0 } });
        let s = create_settlement(&group, &WORLD(), 1);
        assert!(s["structures"].as_array().unwrap().is_empty());
        assert!(s["stockpile"].as_object().unwrap().is_empty());
    }

    #[test]
    fn records_founded_day() {
        let group = json!({ "id": "g1", "territory": { "x": 0, "y": 0 } });
        let s = create_settlement(&group, &WORLD(), 42);
        assert_eq!(s["founded_day"], 42);
    }

    // ── computeSettlementCapacity ───────────────────────────────────────

    #[test]
    fn capacity_is_zero_with_no_structures() {
        assert_eq!(compute_settlement_capacity(&make_settlement()), 0);
    }

    #[test]
    fn capacity_sums_across_structures() {
        let mut s = make_settlement();
        s["structures"] = json!([{ "type": "cave_dwelling", "condition": 1.0 }, { "type": "post_frame_hut", "condition": 1.0 }]);
        assert_eq!(compute_settlement_capacity(&s), 14); // 8 + 6
    }

    // ── computeSettlementDefense ────────────────────────────────────────

    #[test]
    fn defense_is_zero_with_no_defensive_structures() {
        let mut s = make_settlement();
        s["structures"] = json!([{ "type": "cave_dwelling", "condition": 1.0 }]);
        assert_eq!(compute_settlement_defense(&s), 0.0);
    }

    #[test]
    fn defensive_wall_contributes_positive_defense() {
        let mut s = make_settlement();
        s["structures"] = json!([{ "type": "defensive_wall", "condition": 1.0 }]);
        assert!(compute_settlement_defense(&s) > 0.0);
    }

    #[test]
    fn degraded_wall_contributes_less_defense_than_intact_wall() {
        let mut intact = make_settlement();
        intact["structures"] = json!([{ "type": "defensive_wall", "condition": 1.0 }]);
        let mut damaged = make_settlement();
        damaged["structures"] = json!([{ "type": "defensive_wall", "condition": 0.3 }]);
        assert!(compute_settlement_defense(&intact) > compute_settlement_defense(&damaged));
    }

    // ── checkSettlementOvercrowding ─────────────────────────────────────

    #[test]
    fn no_overcrowding_event_when_capacity_is_zero() {
        assert!(check_settlement_overcrowding(&mut make_settlement(), 50, 1).is_none());
    }

    #[test]
    fn no_overcrowding_event_when_group_fits() {
        let mut s = make_settlement();
        s["structures"] = json!([{ "type": "stone_temple", "condition": 1.0 }]);
        assert!(check_settlement_overcrowding(&mut s, 30, 1).is_none());
    }

    #[test]
    fn overcrowding_event_fires_above_120_percent_capacity() {
        let mut s = make_settlement();
        s["structures"] = json!([{ "type": "cave_dwelling", "condition": 1.0 }]); // cap 8
        let ev = check_settlement_overcrowding(&mut s, 12, 1).unwrap();
        assert_eq!(ev["type"], "settlement_overcrowded");
        assert_eq!(ev["settlement_id"], "settlement-1");
    }

    #[test]
    fn exactly_120_percent_capacity_is_not_overcrowded() {
        let mut s = make_settlement();
        s["structures"] = json!([{ "type": "cave_dwelling", "condition": 1.0 }]); // cap 8, 120% = 9.6
        assert!(check_settlement_overcrowding(&mut s, 9, 1).is_none());
    }

    #[test]
    fn overcrowding_only_reports_once_until_it_clears() {
        let mut s = make_settlement();
        s["structures"] = json!([{ "type": "cave_dwelling", "condition": 1.0 }]); // cap 8
        assert!(check_settlement_overcrowding(&mut s, 12, 1).is_some(), "first day over capacity should report");
        assert!(check_settlement_overcrowding(&mut s, 12, 2).is_none(), "still overcrowded the next day should stay silent");
        assert!(check_settlement_overcrowding(&mut s, 12, 3).is_none(), "and the day after that");

        // Capacity increases (a new structure finished) and the group no
        // longer counts as overcrowded -- the flag clears.
        s["structures"] = json!([{ "type": "cave_dwelling", "condition": 1.0 }, { "type": "stone_temple", "condition": 1.0 }]);
        assert!(check_settlement_overcrowding(&mut s, 12, 4).is_none(), "no longer overcrowded, and no event for the transition itself");

        // Overcrowded again later (population grew back past capacity) --
        // this is a new occurrence and must report again.
        assert!(check_settlement_overcrowding(&mut s, 200, 5).is_some(), "a later, separate overcrowding episode must report again");
    }

    // ── processArchitectureTick ──────────────────────────────────────────

    #[test]
    fn labor_pool_accumulates_from_adult_members() {
        let mut s = make_settlement();
        // Empty inventory (unlike make_ind's default) so no materials land in
        // the stockpile and nothing gets built this tick -- isolates labor
        // accrual from being fully spent by a same-tick lean_to (whose 1.0
        // labor_days can exactly consume what 10 adults accrue in one day).
        let mut members: Vec<Individual> = (0..10).map(|i| { let mut m = make_ind(&format!("i{i}"), 25); m.inventory.clear(); m }).collect();
        let mut member_refs: Vec<&mut Individual> = members.iter_mut().collect();
        process_architecture_tick(&mut s, &mut member_refs, &HashSet::new(), &WORLD(), 1);
        assert!(s["labor_pool"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn children_do_not_contribute_labor() {
        let mut s = make_settlement();
        let mut members: Vec<Individual> = (0..10).map(|i| make_ind(&format!("i{i}"), 5)).collect();
        let mut member_refs: Vec<&mut Individual> = members.iter_mut().collect();
        process_architecture_tick(&mut s, &mut member_refs, &HashSet::new(), &WORLD(), 1);
        assert_eq!(s["labor_pool"].as_f64().unwrap(), 0.0);
    }

    #[test]
    fn surplus_materials_are_transferred_to_stockpile() {
        let mut s = make_settlement();
        let mut ind = make_ind("i1", 25);
        ind.inventory = HashMap::from([("wood".to_string(), 20.0)]);
        process_architecture_tick(&mut s, &mut [&mut ind], &HashSet::new(), &WORLD(), 1);
        assert!(s["stockpile"]["wood"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn lean_to_can_be_built_once_labor_and_materials_are_available() {
        let mut s = make_settlement();
        s["labor_pool"] = json!(5);
        s["stockpile"] = json!({ "wood": 5 });
        let mut members: Vec<Individual> = (0..5).map(|i| { let mut m = make_ind(&format!("i{i}"), 25); m.inventory.clear(); m }).collect();
        let mut member_refs: Vec<&mut Individual> = members.iter_mut().collect();
        // No tech discovered at all: a brand new settlement must still have
        // somewhere to sleep, so lean_to (the only structure with zero tech
        // requirement) is the deterministic first choice.
        let evs = process_architecture_tick(&mut s, &mut member_refs, &HashSet::new(), &WORLD(), 1);
        let built = evs.iter().find(|e| e["type"] == "structure_built").expect("expected lean_to to be built with no tech required");
        assert_eq!(built["structure_type"], "lean_to");
        assert_eq!(built["day"], 1);
    }

    #[test]
    fn pit_house_is_preferred_over_lean_to_once_stone_tools_is_known() {
        let mut s = make_settlement();
        s["labor_pool"] = json!(10);
        s["stockpile"] = json!({ "wood": 5, "stone": 5 });
        let mut members: Vec<Individual> = (0..5).map(|i| { let mut m = make_ind(&format!("i{i}"), 25); m.inventory.clear(); m }).collect();
        let mut member_refs: Vec<&mut Individual> = members.iter_mut().collect();
        let mut techs = HashSet::new();
        techs.insert("stone_tools".to_string());
        let evs = process_architecture_tick(&mut s, &mut member_refs, &techs, &WORLD(), 1);
        let built = evs.iter().find(|e| e["type"] == "structure_built").expect("expected a structure to be built");
        assert_eq!(built["structure_type"], "pit_house");
    }

    #[test]
    fn stone_house_marketplace_and_city_wall_are_all_eventually_reachable() {
        // H-06/V-02 regression: build_priority previously never selected
        // pit_house, stone_house, marketplace, or city_wall at all --
        // permanently unbuildable regardless of tech level or population.
        let mut s = make_settlement();
        s["labor_pool"] = json!(100_000);
        s["stockpile"] = json!({ "wood": 100_000, "stone": 100_000, "clay": 100_000 });
        let mut members: Vec<Individual> = (0..25).map(|i| { let mut m = make_ind(&format!("i{i}"), 25); m.inventory.clear(); m }).collect();
        let mut techs = HashSet::new();
        for t in ["stone_tools", "pottery", "plant_cultivation", "architecture_stone", "metallurgy_copper", "wheel", "writing_system"] {
            techs.insert(t.to_string());
        }
        let world = json!({ "food_abundance": 0.9, "recent_disaster": "conflict" });
        // Track every structure_built event across the run, not just what's
        // still standing at the end -- lower-durability types (pit_house,
        // durability 0.5) decay and get demolished well before 5000 days,
        // which would otherwise make "was this ever buildable at all" appear
        // false for a reason that has nothing to do with build_priority.
        let mut ever_built: HashSet<String> = HashSet::new();
        for day in 0..3000 {
            let mut member_refs: Vec<&mut Individual> = members.iter_mut().collect();
            let evs = process_architecture_tick(&mut s, &mut member_refs, &techs, &world, day);
            for ev in evs {
                if ev["type"] == "structure_built" {
                    if let Some(t) = ev["structure_type"].as_str() {
                        ever_built.insert(t.to_string());
                    }
                }
            }
        }
        for expected in ["pit_house", "stone_house", "marketplace", "city_wall"] {
            assert!(ever_built.contains(expected), "expected {expected} to have been built within 3000 days, built: {ever_built:?}");
        }
    }

    #[test]
    fn structure_built_event_has_expected_shape() {
        let mut s = make_settlement();
        s["labor_pool"] = json!(1000);
        s["stockpile"] = json!({ "wood": 100, "stone": 100, "clay": 100 });
        let mut members: Vec<Individual> = (0..10).map(|i| { let mut m = make_ind(&format!("i{i}"), 25); m.inventory.clear(); m }).collect();
        let mut member_refs: Vec<&mut Individual> = members.iter_mut().collect();
        let mut techs = HashSet::new();
        techs.insert("stone_tools".to_string());
        let evs = process_architecture_tick(&mut s, &mut member_refs, &techs, &WORLD(), 5);
        if let Some(ev) = evs.iter().find(|e| e["type"] == "structure_built") {
            assert_eq!(ev["settlement_id"], "settlement-1");
            assert_eq!(ev["day"], 5);
            assert!(ev["structure_type"].is_string());
        }
    }

    #[test]
    fn a_structure_cannot_be_built_without_its_required_materials() {
        let mut s = make_settlement();
        s["labor_pool"] = json!(1000); // labor plentiful
        s["stockpile"] = json!({}); // but zero materials
        let mut members: Vec<Individual> = (0..10).map(|i| { let mut m = make_ind(&format!("i{i}"), 25); m.inventory.clear(); m }).collect();
        let mut member_refs: Vec<&mut Individual> = members.iter_mut().collect();
        let evs = process_architecture_tick(&mut s, &mut member_refs, &HashSet::new(), &WORLD(), 1);
        assert!(evs.iter().all(|e| e["type"] != "structure_built"), "lean_to requires wood; none was in stock");
    }

    #[test]
    fn structures_degrade_over_time() {
        let mut s = make_settlement();
        s["structures"] = json!([{ "id": "st1", "type": "lean_to", "built_day": 0, "condition": 1.0 }]);
        process_architecture_tick(&mut s, &mut [], &HashSet::new(), &WORLD(), 1);
        assert!(s["structures"][0]["condition"].as_f64().unwrap() < 1.0);
    }

    #[test]
    fn structures_at_minimal_condition_are_removed() {
        let mut s = make_settlement();
        s["structures"] = json!([{ "id": "st1", "type": "lean_to", "built_day": 0, "condition": 0.03 }]);
        process_architecture_tick(&mut s, &mut [], &HashSet::new(), &WORLD(), 1);
        assert!(s["structures"].as_array().unwrap().is_empty());
    }
}
