use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::types::PhonemePalette;

/// A group can only name itself once it has developed the naming_ceremony
/// cultural practice in its own culture -- nothing forces this; a group
/// whose culture never reaches that meme stays unnamed forever, same as an
/// individual whose language never reaches proto-words. Built from this
/// simulation's own phoneme_palette, never a fixed real-world place name.
fn try_name_group(group_id: &str, palette: &PhonemePalette) -> Option<String> {
    let raw = crate::language::generate_proto_word(group_id, "group_name", palette);
    if raw.is_empty() {
        return None;
    }
    let mut chars = raw.chars();
    chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
}

/// Once most of the living population belongs to a group that has already
/// named itself (see `try_name_group`), the civilization as a whole earns a
/// name too -- one level up, same phoneme_palette, still never forced and
/// never re-chosen once set (callers only invoke this while
/// `civilization_name` is still `None`).
pub fn try_name_civilization(population: &[crate::state::Individual], groups: &[Value], palette: &PhonemePalette) -> Option<String> {
    let alive_total = population.iter().filter(|i| i.alive && !i.is_dead).count();
    if alive_total == 0 {
        return None;
    }
    let named_group_ids: HashSet<&str> =
        groups.iter().filter(|g| g.get("name").is_some_and(Value::is_string)).filter_map(|g| g.get("id").and_then(Value::as_str)).collect();
    let named_alive = population.iter().filter(|i| i.alive && !i.is_dead).filter(|i| i.group_id.as_deref().is_some_and(|gid| named_group_ids.contains(gid))).count();
    if named_alive as f64 / alive_total as f64 <= 0.5 {
        return None;
    }
    let raw = crate::language::generate_proto_word("civilization", "civilization", palette);
    if raw.is_empty() {
        return None;
    }
    let mut chars = raw.chars();
    chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
}

#[allow(clippy::type_complexity)]
pub const CULTURAL_MEMES: &[(&str, i32, f64, usize, f64, &[&str])] = &[
    ("shared_greeting", 1, 0.2, 2, 0.05, &[]),
    ("mourning_ritual", 1, 0.3, 3, 0.03, &[]),
    ("food_sharing_norm", 1, 0.2, 2, 0.06, &[]),
    ("reciprocity_norm", 2, 0.4, 4, 0.04, &[]),
    ("gender_roles", 2, 0.4, 5, 0.04, &[]),
    ("age_hierarchy", 2, 0.4, 4, 0.05, &[]),
    ("gift_exchange", 2, 0.5, 5, 0.03, &[]),
    ("body_decoration", 3, 0.5, 3, 0.04, &[]),
    ("storytelling", 3, 0.55, 4, 0.05, &[]),
    ("music_drumming", 3, 0.5, 3, 0.06, &[]),
    ("dance_ritual", 3, 0.5, 4, 0.05, &[]),
    ("naming_ceremony", 3, 0.55, 3, 0.03, &[]),
    ("marriage_ceremony", 4, 0.6, 5, 0.03, &[]),
    ("seasonal_festival", 4, 0.6, 6, 0.03, &[]),
    ("taboo_system", 4, 0.6, 5, 0.02, &[]),
    ("trade_ceremony", 4, 0.65, 6, 0.02, &[]),
    ("written_myth", 5, 0.7, 10, 0.02, &["writing_system"]),
    ("legal_code", 5, 0.7, 10, 0.01, &["writing_system"]),
];

fn meme_description(meme_id: &str) -> &str {
    match meme_id {
        "shared_greeting" => "A consistent greeting gesture develops",
        "mourning_ritual" => "Communal mourning practices emerge for the dead",
        "food_sharing_norm" => "Food is shared equally among group members",
        "reciprocity_norm" => "Gifts and favors are expected to be returned",
        "gender_roles" => "Different tasks become associated with different sexes",
        "age_hierarchy" => "Elders are accorded special respect",
        "gift_exchange" => "Ceremonial gift-giving strengthens social bonds",
        "body_decoration" => "Pigments and natural materials used for body adornment",
        "storytelling" => "Oral narratives preserve group memory and values",
        "music_drumming" => "Rhythmic percussion emerges as social bonding activity",
        "dance_ritual" => "Coordinated movement used in group ceremonies",
        "naming_ceremony" => "Birth is marked with naming rites",
        "marriage_ceremony" => "Pair-bonding is formalized through ritual",
        "seasonal_festival" => "Cyclical celebrations mark the seasons",
        "taboo_system" => "Certain behaviors become culturally forbidden",
        "trade_ceremony" => "Exchange is ritualized to build trust",
        "written_myth" => "Origin stories are recorded in written form",
        "legal_code" => "Rules and punishments are written and formalized",
        other => other,
    }
}

fn meme_adoptable(meme_id: &str, avg_lang_stage: f64, avg_foxp2: f64, member_count: usize, discovered_techs: &HashSet<String>) -> bool {
    let Some((_, stage, foxp2_min, group_min, _, requires_tech)) = CULTURAL_MEMES.iter().find(|(id, ..)| *id == meme_id) else {
        return false;
    };
    if avg_lang_stage < *stage as f64 || avg_foxp2 < *foxp2_min || member_count < *group_min {
        return false;
    }
    requires_tech.iter().all(|t| discovered_techs.contains(*t))
}

pub fn process_culture_tick(
    population: &[crate::state::Individual],
    groups: &mut [Value],
    discovered_techs: &HashSet<String>,
    sim_day: i32,
    palette: &PhonemePalette,
) -> Vec<Value> {
    let mut events = Vec::new();

    // Snapshot every group's id + culture set + prestige up front so
    // inter-group diffusion can read *other* groups' culture while this loop
    // mutates the current one.
    let culture_snapshot: Vec<(String, HashSet<String>, f64)> = groups
        .iter()
        .filter_map(|g| {
            let id = g.get("id")?.as_str()?.to_string();
            let culture: HashSet<String> = g
                .get("culture")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
                .unwrap_or_default();
            let prestige = compute_cultural_prestige(g);
            Some((id, culture, prestige))
        })
        .collect();

    // group_id -> members, built with two O(population) passes instead of
    // filtering the *entire* population once per group -- an O(groups *
    // population) cost that got sharply worse as both grew over a long run.
    // Keyed off each group's own member_ids (not individual.group_id) to
    // preserve the exact membership semantics the per-group filter used to
    // have. Owned Strings (not &str borrowed from `groups`) so this doesn't
    // hold an immutable borrow of `groups` across the `groups.iter_mut()`
    // loop below that mutates it.
    let mut individual_group: HashMap<String, String> = HashMap::new();
    for g in groups.iter() {
        let Some(gid) = g.get("id").and_then(Value::as_str) else { continue };
        let Some(ids) = g.get("member_ids").and_then(Value::as_array) else { continue };
        for id in ids.iter().filter_map(Value::as_str) {
            individual_group.insert(id.to_string(), gid.to_string());
        }
    }
    let mut members_by_group: HashMap<String, Vec<&crate::state::Individual>> = HashMap::new();
    for ind in population {
        if let Some(gid) = individual_group.get(ind.id.as_str()) {
            members_by_group.entry(gid.clone()).or_default().push(ind);
        }
    }

    for group in groups.iter_mut() {
        let group_id = group.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let culture = group
            .get_mut("culture")
            .and_then(Value::as_array_mut)
            .cloned()
            .unwrap_or_default();
        let Some(members) = members_by_group.get(&group_id) else { continue };
        if members.len() < 2 {
            continue;
        }
        let avg_foxp2 = members.iter().map(|m| m.language.foxp2_expression).sum::<f64>() / members.len() as f64;
        let avg_art = members.iter().map(|m| m.phenotype.artistic_sense).sum::<f64>() / members.len() as f64;
        let avg_lang_stage = members.iter().map(|m| m.language.stage as f64).sum::<f64>() / members.len() as f64;
        let mut culture_set: HashSet<String> = culture.iter().filter_map(Value::as_str).map(ToString::to_string).collect();
        let mut pressure_map = group.get("_culturePressure").cloned().unwrap_or_else(|| json!({}));
        for (meme_id, stage, foxp2_min, group_min, spread_rate, requires_tech) in CULTURAL_MEMES {
            if culture_set.contains(*meme_id) || avg_lang_stage < *stage as f64 || avg_foxp2 < *foxp2_min || members.len() < *group_min {
                continue;
            }
            if requires_tech.iter().any(|t| !discovered_techs.contains(*t)) {
                continue;
            }
            let threshold = 100.0 / (avg_art * *spread_rate).max(0.001);
            let current = pressure_map.get(*meme_id).and_then(Value::as_f64).unwrap_or(0.0) + 1.0;
            pressure_map[*meme_id] = json!(current);
            if current >= threshold {
                culture_set.insert((*meme_id).to_string());
                pressure_map[*meme_id] = json!(0);
                let tension = group.get("internal_tension").and_then(Value::as_f64).unwrap_or(0.5);
                group["internal_tension"] = json!((tension - 0.03).max(0.0));
                events.push(json!({
                    "type": "cultural_meme_emerged",
                    "meme_id": meme_id,
                    "group_id": group.get("id").cloned().unwrap_or(Value::Null),
                    "day": sim_day,
                    "importance": if *stage >= 4 { "high" } else { "low" },
                    "description": meme_description(meme_id),
                }));
            }
        }
        group["culture"] = Value::Array(culture_set.iter().map(|s| Value::String(s.clone())).collect());
        group["_culturePressure"] = pressure_map;

        // Group naming: once naming_ceremony has entered this group's own
        // culture, the group can name itself -- see try_name_group.
        if culture_set.contains("naming_ceremony") && group.get("name").map(Value::is_null).unwrap_or(true) {
            if let Some(name) = try_name_group(&group_id, palette) {
                group["name"] = json!(name);
                events.push(json!({
                    "type": "group_named",
                    "group_id": group_id,
                    "name": name,
                    "day": sim_day,
                    "importance": "medium",
                }));
            }
        }

        // Inter-group contact: 1 point/day; exchange fires roughly every 67 days.
        let diffusion_pressure = group.get("_diffusionPressure").and_then(Value::as_f64).unwrap_or(0.0) + 1.0;
        if diffusion_pressure >= 67.0 {
            group["_diffusionPressure"] = json!(0);
            let others: Vec<&(String, HashSet<String>, f64)> =
                culture_snapshot.iter().filter(|(id, culture, _)| *id != group_id && !culture.is_empty()).collect();
            if !others.is_empty() {
                let src = prestige_weighted_pick(&others);
                let novel = src.1.iter().find(|m| !culture_set.contains(*m) && meme_adoptable(m, avg_lang_stage, avg_foxp2, members.len(), discovered_techs));
                if let Some(novel) = novel {
                    culture_set.insert(novel.clone());
                    group["culture"] = Value::Array(culture_set.iter().map(|s| Value::String(s.clone())).collect());
                    events.push(json!({
                        "type": "cultural_diffusion",
                        "meme_id": novel,
                        "from_group": src.0,
                        "to_group": group_id,
                        "day": sim_day,
                        "importance": "low",
                        "description": meme_description(novel),
                    }));
                }
            }
        } else {
            group["_diffusionPressure"] = json!(diffusion_pressure);
        }
    }
    events
}

pub fn compute_cultural_prestige(group: &Value) -> f64 {
    group
        .get("culture")
        .and_then(Value::as_array)
        .map(|arr| (arr.len() as f64 * 0.05).min(1.0))
        .unwrap_or(0.0)
}

/// Prestige-biased cultural transmission: a candidate source group's chance
/// of being copied from scales with its prestige (people preferentially
/// imitate higher-status groups), not a uniform coin flip across every
/// group with any culture at all. The flat +0.1 keeps a zero-prestige group
/// reachable too, just far less likely than an established one.
fn prestige_weighted_pick<'a>(candidates: &[&'a (String, HashSet<String>, f64)]) -> &'a (String, HashSet<String>, f64) {
    let weights: Vec<f64> = candidates.iter().map(|(_, _, prestige)| prestige + 0.1).collect();
    let total: f64 = weights.iter().sum();
    let mut pick = rand::random::<f64>() * total;
    for (i, w) in weights.iter().enumerate() {
        if pick < *w {
            return candidates[i];
        }
        pick -= w;
    }
    candidates[candidates.len() - 1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::full_palette;
    use crate::state::Individual;
    use crate::types::{Language, Phenotype};

    fn make_ind(id: &str, artistic: f64, foxp2: f64) -> Individual {
        Individual {
            id: id.to_string(),
            phenotype: Phenotype { artistic_sense: artistic, ..Default::default() },
            // stage 5 so none of the existing tests below (which don't vary
            // language stage) are affected by the meme-stage gate added
            // below -- a dedicated test exercises that gate directly.
            language: Language { stage: 5, foxp2_expression: foxp2, ..Default::default() },
            ..Default::default()
        }
    }

    fn make_group(id: &str, member_ids: &[&str]) -> Value {
        json!({
            "id": id,
            "member_ids": member_ids,
            "culture": [],
            "_culturePressure": {},
            "_diffusionPressure": 0,
            "internal_tension": 0.5,
        })
    }

    // ── CULTURAL_MEMES — definition checks ──────────────────────────────

    #[test]
    fn defines_eighteen_memes() {
        assert_eq!(CULTURAL_MEMES.len(), 18);
    }

    #[test]
    fn shared_greeting_is_the_most_accessible_meme() {
        let min = CULTURAL_MEMES.iter().map(|(_, _, foxp2_min, ..)| *foxp2_min).fold(f64::INFINITY, f64::min);
        let shared_greeting = CULTURAL_MEMES.iter().find(|(id, ..)| *id == "shared_greeting").unwrap().2;
        assert_eq!(shared_greeting, min);
    }

    #[test]
    fn written_myth_and_legal_code_require_writing_system() {
        for id in ["written_myth", "legal_code"] {
            let (.., requires_tech) = CULTURAL_MEMES.iter().find(|(mid, ..)| *mid == id).unwrap();
            assert!(requires_tech.contains(&"writing_system"));
        }
    }

    #[test]
    fn stage_five_memes_require_the_largest_group_size() {
        let max_s5 = CULTURAL_MEMES.iter().filter(|(_, stage, ..)| *stage == 5).map(|(_, _, _, gs, ..)| *gs).max().unwrap();
        let max_s4 = CULTURAL_MEMES.iter().filter(|(_, stage, ..)| *stage == 4).map(|(_, _, _, gs, ..)| *gs).max().unwrap();
        assert!(max_s5 >= max_s4);
    }

    // ── processCultureTick — meme emergence ─────────────────────────────

    #[test]
    fn returns_empty_for_a_group_with_fewer_than_two_members() {
        let members = vec![make_ind("i1", 0.7, 0.6)];
        let mut group = make_group("g1", &["i1"]);
        let events = process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), 1, &crate::language::full_palette());
        assert!(events.is_empty());
    }

    #[test]
    fn meme_blocked_when_foxp2_requirement_not_met() {
        let members: Vec<Individual> = (0..5).map(|i| make_ind(&format!("i{i}"), 0.7, 0.01)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        for day in 0..1000 {
            process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), day, &crate::language::full_palette());
        }
        assert!(group["culture"].as_array().unwrap().is_empty());
    }

    #[test]
    fn meme_blocked_when_group_language_stage_is_too_low() {
        // H-06 regression: process_culture_tick/meme_adoptable used to ignore
        // each meme's own `stage` field entirely, so a group that hadn't
        // reached even gestural language (stage 0) could still adopt
        // "shared_greeting" (stage 1) purely off foxp2/group-size, the same
        // gating gap fixed for individual belief formation in belief.rs.
        let members: Vec<Individual> = (0..5)
            .map(|i| Individual {
                id: format!("i{i}"),
                phenotype: Phenotype { artistic_sense: 0.99, ..Default::default() },
                language: Language { stage: 0, foxp2_expression: 0.99, ..Default::default() },
                ..Default::default()
            })
            .collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        for day in 0..3000 {
            process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), day, &crate::language::full_palette());
        }
        assert!(group["culture"].as_array().unwrap().is_empty(), "a stage-0 group must not adopt any meme, however high its foxp2/artistic sense");
    }

    #[test]
    fn meme_blocked_when_group_too_small() {
        let members: Vec<Individual> = (0..3).map(|i| make_ind(&format!("i{i}"), 0.9, 0.8)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        for day in 0..2000 {
            process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), day, &crate::language::full_palette());
        }
        let culture: Vec<String> = group["culture"].as_array().unwrap().iter().filter_map(|v| v.as_str().map(String::from)).collect();
        assert!(!culture.contains(&"gift_exchange".to_string()));
    }

    #[test]
    fn written_myth_blocked_without_writing_system_tech() {
        let members: Vec<Individual> = (0..15).map(|i| make_ind(&format!("i{i}"), 0.99, 0.99)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        for day in 0..3000 {
            process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), day, &crate::language::full_palette());
        }
        let culture: Vec<String> = group["culture"].as_array().unwrap().iter().filter_map(|v| v.as_str().map(String::from)).collect();
        assert!(!culture.contains(&"written_myth".to_string()));
    }

    #[test]
    fn meme_emergence_event_has_the_expected_shape() {
        let members: Vec<Individual> = (0..5).map(|i| make_ind(&format!("i{i}"), 0.99, 0.99)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        let mut event = None;
        for day in 0..10_000 {
            let events = process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), day, &crate::language::full_palette());
            if let Some(ev) = events.into_iter().find(|e| e["type"] == "cultural_meme_emerged") {
                event = Some(ev);
                break;
            }
        }
        let ev = event.expect("expected a cultural_meme_emerged event within 10000 days");
        assert_eq!(ev["type"], "cultural_meme_emerged");
        assert_eq!(ev["group_id"], "g1");
        assert!(ev["meme_id"].is_string());
        assert!(ev["description"].is_string());
    }

    #[test]
    fn already_known_meme_is_never_added_again() {
        let members: Vec<Individual> = (0..5).map(|i| make_ind(&format!("i{i}"), 0.99, 0.99)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        let all: Vec<Value> = CULTURAL_MEMES.iter().map(|(id, ..)| json!(id)).collect();
        group["culture"] = Value::Array(all);
        let before = group["culture"].as_array().unwrap().len();
        let mut techs = HashSet::new();
        techs.insert("writing_system".to_string());
        for day in 0..200 {
            process_culture_tick(&members, std::slice::from_mut(&mut group), &techs, day, &crate::language::full_palette());
        }
        assert_eq!(group["culture"].as_array().unwrap().len(), before);
    }

    #[test]
    fn meme_emergence_reduces_internal_tension() {
        let members: Vec<Individual> = (0..5).map(|i| make_ind(&format!("i{i}"), 0.99, 0.99)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        group["_culturePressure"]["shared_greeting"] = json!(99999);
        let events = process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), 1, &crate::language::full_palette());
        if events.iter().any(|e| e["meme_id"] == "shared_greeting") {
            assert!(group["internal_tension"].as_f64().unwrap() < 0.5);
        }
    }

    // ── processCultureTick — inter-group diffusion ──────────────────────

    #[test]
    fn diffusion_fires_after_sixty_seven_ticks_of_pressure_accumulation() {
        let members1: Vec<Individual> = (0..5).map(|i| make_ind(&format!("g1_{i}"), 0.7, 0.8)).collect();
        let members2: Vec<Individual> = (0..5).map(|i| make_ind(&format!("g2_{i}"), 0.7, 0.8)).collect();
        let ids1: Vec<&str> = members1.iter().map(|m| m.id.as_str()).collect();
        let ids2: Vec<&str> = members2.iter().map(|m| m.id.as_str()).collect();
        let mut group1 = make_group("g1", &ids1);
        group1["culture"] = json!(["shared_greeting"]);
        let mut group2 = make_group("g2", &ids2);
        group2["_diffusionPressure"] = json!(66);
        let mut all_members = members1;
        all_members.extend(members2);
        let mut groups = vec![group1, group2];
        let mut diffused = false;
        for day in 0..200 {
            let events = process_culture_tick(&all_members, &mut groups, &HashSet::new(), day, &crate::language::full_palette());
            if events.iter().any(|e| e["type"] == "cultural_diffusion") {
                diffused = true;
                break;
            }
        }
        assert!(diffused, "expected a cultural_diffusion event once diffusion pressure crosses 67");
    }

    #[test]
    fn a_single_candidate_is_always_picked() {
        let only: (String, HashSet<String>, f64) = ("g1".to_string(), HashSet::from(["shared_greeting".to_string()]), 0.5);
        let candidates = vec![&only];
        for _ in 0..20 {
            assert_eq!(prestige_weighted_pick(&candidates).0, "g1");
        }
    }

    #[test]
    fn prestige_weighted_pick_favors_the_higher_prestige_candidate() {
        let low: (String, HashSet<String>, f64) = ("low".to_string(), HashSet::from(["shared_greeting".to_string()]), 0.0);
        let high: (String, HashSet<String>, f64) = ("high".to_string(), HashSet::from(["shared_greeting".to_string()]), 1.0);
        let candidates = vec![&low, &high];
        let high_wins = (0..2000).filter(|_| prestige_weighted_pick(&candidates).0 == "high").count();
        assert!(high_wins > 1400, "a prestige-1.0 group should be picked far more often than a prestige-0.0 one (got {high_wins}/2000)");
    }

    // ── computeCulturalPrestige ──────────────────────────────────────────

    #[test]
    fn prestige_is_zero_for_a_group_with_no_culture() {
        let group = make_group("g1", &[]);
        assert_eq!(compute_cultural_prestige(&group), 0.0);
    }

    #[test]
    fn more_memes_yield_higher_prestige() {
        let mut few = make_group("g1", &[]);
        few["culture"] = json!(["shared_greeting"]);
        let mut many = make_group("g2", &[]);
        many["culture"] = json!(["shared_greeting", "mourning_ritual", "food_sharing_norm", "gift_exchange", "storytelling"]);
        assert!(compute_cultural_prestige(&many) > compute_cultural_prestige(&few));
    }

    #[test]
    fn prestige_is_capped_at_one() {
        let mut group = make_group("g1", &[]);
        let all: Vec<Value> = CULTURAL_MEMES.iter().map(|(id, ..)| json!(id)).collect();
        group["culture"] = Value::Array(all);
        assert!(compute_cultural_prestige(&group) <= 1.0);
    }

    // ── group naming ─────────────────────────────────────────────────────

    #[test]
    fn a_group_never_names_itself_without_naming_ceremony() {
        let members: Vec<Individual> = (0..5).map(|i| make_ind(&format!("i{i}"), 0.99, 0.99)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        group["culture"] = json!(["shared_greeting"]);
        for day in 0..2000 {
            process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), day, &crate::language::full_palette());
        }
        assert!(group.get("name").is_none() || group["name"].is_null());
    }

    #[test]
    fn a_group_names_itself_once_naming_ceremony_is_in_its_own_culture() {
        let members: Vec<Individual> = (0..5).map(|i| make_ind(&format!("i{i}"), 0.99, 0.99)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        group["culture"] = json!(["naming_ceremony"]);
        let events = process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), 1, &crate::language::full_palette());
        assert!(group["name"].is_string());
        assert!(events.iter().any(|e| e["type"] == "group_named" && e["group_id"] == "g1"));
    }

    #[test]
    fn an_already_named_group_is_never_renamed_or_re_emitted() {
        let members: Vec<Individual> = (0..5).map(|i| make_ind(&format!("i{i}"), 0.99, 0.99)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        group["culture"] = json!(["naming_ceremony"]);
        group["name"] = json!("Original");
        let events = process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), 1, &crate::language::full_palette());
        assert_eq!(group["name"], "Original");
        assert!(!events.iter().any(|e| e["type"] == "group_named"));
    }

    #[test]
    fn a_group_with_no_phonemes_available_never_names_itself() {
        let members: Vec<Individual> = (0..5).map(|i| make_ind(&format!("i{i}"), 0.99, 0.99)).collect();
        let ids: Vec<&str> = members.iter().map(|m| m.id.as_str()).collect();
        let mut group = make_group("g1", &ids);
        group["culture"] = json!(["naming_ceremony"]);
        process_culture_tick(&members, std::slice::from_mut(&mut group), &HashSet::new(), 1, &PhonemePalette::default());
        assert!(group.get("name").is_none() || group["name"].is_null());
    }

    // ── civilization naming ──────────────────────────────────────────────

    fn alive_member(id: &str, group_id: &str) -> Individual {
        Individual { id: id.to_string(), alive: true, is_dead: false, group_id: Some(group_id.to_string()), ..Default::default() }
    }

    #[test]
    fn no_civilization_name_with_an_empty_population() {
        assert!(try_name_civilization(&[], &[], &full_palette()).is_none());
    }

    #[test]
    fn no_civilization_name_while_no_group_has_named_itself() {
        let pop = vec![alive_member("i1", "g1"), alive_member("i2", "g1")];
        let groups = vec![make_group("g1", &["i1", "i2"])];
        assert!(try_name_civilization(&pop, &groups, &full_palette()).is_none());
    }

    #[test]
    fn no_civilization_name_when_named_groups_hold_a_minority() {
        let pop = vec![alive_member("i1", "named"), alive_member("i2", "unnamed"), alive_member("i3", "unnamed")];
        let mut named = make_group("named", &["i1"]);
        named["name"] = json!("Kalu");
        let unnamed = make_group("unnamed", &["i2", "i3"]);
        assert!(try_name_civilization(&pop, &[named, unnamed], &full_palette()).is_none());
    }

    #[test]
    fn civilization_names_itself_once_the_majority_belongs_to_named_groups() {
        let pop = vec![alive_member("i1", "named"), alive_member("i2", "named"), alive_member("i3", "unnamed")];
        let mut named = make_group("named", &["i1", "i2"]);
        named["name"] = json!("Kalu");
        let unnamed = make_group("unnamed", &["i3"]);
        assert!(try_name_civilization(&pop, &[named, unnamed], &full_palette()).is_some());
    }

    #[test]
    fn civilization_naming_never_draws_from_an_empty_palette() {
        let pop = vec![alive_member("i1", "named")];
        let mut named = make_group("named", &["i1"]);
        named["name"] = json!("Kalu");
        assert!(try_name_civilization(&pop, &[named], &PhonemePalette::default()).is_none());
    }
}
