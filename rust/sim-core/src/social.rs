use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};

use crate::state::Individual;

pub const RELATIONSHIP_TYPES: &[&str] = &["kin", "mate", "ally", "rival", "neutral", "outgroup"];
/// The 6 roles any non-founder can hold, emergent from `_behaviorCounts`/age
/// (see `compute_role_for`). Deliberately excludes "anchor" -- the founder-only
/// 7th value `compute_role_for` returns -- since founders are the one
/// documented exception to every emergence rule in this codebase (the same
/// carve-out that lets them have a player-given name/genome). Anything that
/// iterates GROUP_ROLES to bucket/count roles across a population should
/// treat founders as a separate case, not expect them to appear here.
pub const GROUP_ROLES: &[&str] = &["leader", "elder", "warrior", "gatherer", "healer", "member"];

// A household with two founders and a handful of dependent descendants is not
// yet a pair of politically independent bands. Keep fission and warfare behind
// the adulthood boundary so juveniles cannot create a rival group merely from
// inherited traits.
const SOCIAL_ADULT_AGE_YEARS: f64 = 18.0;
const MIN_FISSION_ADULT_MEMBERS: usize = 8;
const MIN_CONFLICT_POPULATION: usize = 12;
const MIN_CONFLICT_ADULT_MEMBERS_PER_GROUP: usize = 2;

fn is_social_adult(individual: &Individual, sim_day: i32) -> bool {
    crate::biology::individual::get_age(individual, sim_day) >= SOCIAL_ADULT_AGE_YEARS
}

pub fn compute_social_status(individual: &Individual, group: Option<&Value>) -> f64 {
    let Some(group) = group else {
        return 0.0;
    };
    let p = &individual.phenotype;
    let founded_day = group.get("founded_day").and_then(Value::as_i64).unwrap_or(0) as f64;
    let weight = (founded_day / 1000.0).clamp(0.0, 1.0);
    let dom = p.dominance;
    let iq = p.fluid_intelligence;
    let emp = p.empathy;
    let strn = p.physical_strength;
    let age = individual.age_days.unwrap_or(0) as f64 / 365.0;
    let rep = individual.social.reputation;
    (dom * 0.3
        + iq * 0.25 * weight
        + emp * 0.2 * weight
        + if age < 40.0 { strn } else { 0.0 } * 0.15 * (1.0 - weight)
        + rep * 0.1)
        .clamp(0.0, 1.0)
}

/// A challenger unseats the leader when their social status is well ahead of
/// the leader's (>0.2) and a low-probability contest roll fires; the outcome
/// is still probabilistic, weighted by dominance + physical_strength on both
/// sides -- a much stronger challenger is likely, not certain, to win.
fn find_and_resolve_leadership_challenge(members: &[&mut Individual], leader_idx: usize, group: &Value) -> Option<String> {
    let leader_status = compute_social_status(members[leader_idx], Some(group));
    let challenger_idx = members.iter().enumerate().find(|(i, m)| {
        *i != leader_idx && compute_social_status(m, Some(group)) - leader_status > 0.2 && rand::random::<f64>() < 0.05
    })?.0;
    let ls = members[leader_idx].phenotype.dominance + members[leader_idx].phenotype.physical_strength;
    let cs = members[challenger_idx].phenotype.dominance + members[challenger_idx].phenotype.physical_strength;
    let leader_wins = rand::random::<f64>() < ls / (ls + cs).max(1e-9);
    if leader_wins {
        None
    } else {
        Some(members[challenger_idx].id.clone())
    }
}

/// Members with high independence and low standing in the group occasionally
/// break away together, forming a rival splinter group under their own
/// strongest (dominance + intelligence) dissenter. Requires >=8 adult members
/// and >=3 simultaneous adult dissenters: dependent children remain with their
/// caregivers and cannot create an artificial rival band.
fn attempt_group_fission(members: &mut [&mut Individual], group: &Value, sim_day: i32) -> Option<Value> {
    if members.iter().filter(|m| is_social_adult(m, sim_day)).count() < MIN_FISSION_ADULT_MEMBERS {
        return None;
    }
    let dissenter_idx: Vec<usize> = members
        .iter()
        .enumerate()
        .filter(|(_, m)| is_social_adult(m, sim_day) && m.phenotype.independence > 0.6 && compute_social_status(m, Some(group)) < 0.4 && rand::random::<f64>() < 0.1)
        .map(|(i, _)| i)
        .collect();
    if dissenter_idx.len() < 3 {
        return None;
    }
    let leader_idx = *dissenter_idx
        .iter()
        .max_by(|&&a, &&b| {
            let sa = members[a].phenotype.dominance + members[a].phenotype.fluid_intelligence;
            let sb = members[b].phenotype.dominance + members[b].phenotype.fluid_intelligence;
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();
    let new_group_id = format!("group_{}_{}", sim_day, uuid::Uuid::new_v4());
    let (nx, ny) = (members[dissenter_idx[0]].x, members[dissenter_idx[0]].y);
    let new_leader_id = members[leader_idx].id.clone();
    let member_ids: Vec<Value> = dissenter_idx.iter().map(|&i| json!(members[i].id)).collect();
    for &i in &dissenter_idx {
        members[i].group_id = Some(new_group_id.clone());
    }
    Some(json!({
        "id": new_group_id,
        "member_ids": member_ids,
        "leader_id": new_leader_id,
        "founded_day": sim_day,
        "territory": { "x": nx, "y": ny },
        "internal_tension": 0.2,
        "norms": [],
        "culture": [],
        "rival_ids": [group.get("id").cloned().unwrap_or(Value::Null)],
    }))
}

/// Raises every group's internal_tension a small amount in proportion to
/// real environmental resource pressure (food/water scarcity relative to
/// carrying capacity -- see environment::compute_resource_pressure).
///
/// Previously nothing anywhere in the simulation ever *raised*
/// internal_tension -- every other write site (belief/art/culture spread
/// effects) only ever lowers it, so a group's tension could decay toward 0
/// but never organically climb. That made both `process_group_dynamics`'s
/// own tension-based fission trigger (>0.8) and
/// `process_intergroup_conflict`'s tension gate (>=0.5) permanently
/// unreachable in a real, running simulation -- reachable only in
/// hand-constructed test fixtures that set internal_tension directly.
///
/// Deliberately small and bounded: at max pressure this adds 0.01/day, so a
/// population enduring a genuinely severe, sustained famine takes on the
/// order of months (not days) to reach the fission/conflict thresholds --
/// an ordinary lean season shouldn't destabilize every group in the
/// simulation overnight.
pub fn apply_resource_tension(groups: &mut [Value], food_pressure: f64, water_pressure: f64) {
    let gain = ((food_pressure.max(0.0) + water_pressure.max(0.0)) / 2.0 * 0.01).min(0.01);
    if gain <= 0.0 {
        return;
    }
    for group in groups.iter_mut() {
        let tension = group.get("internal_tension").and_then(Value::as_f64).unwrap_or(0.0);
        if let Some(obj) = group.as_object_mut() {
            obj.insert("internal_tension".to_string(), json!((tension + gain).min(1.0)));
        }
    }
}

/// Raises a single group's internal_tension when its settlement is
/// overcrowded (see architecture::check_settlement_overcrowding) -- the
/// other half of giving tension a real driver: overcrowding is a concrete,
/// per-group stressor real human settlements actually experience, not an
/// abstract global average.
pub fn apply_overcrowding_tension(groups: &mut [Value], group_id: &str) {
    if let Some(group) = groups.iter_mut().find(|g| g.get("id").and_then(Value::as_str) == Some(group_id)) {
        let tension = group.get("internal_tension").and_then(Value::as_f64).unwrap_or(0.0);
        if let Some(obj) = group.as_object_mut() {
            obj.insert("internal_tension".to_string(), json!((tension + 0.02).min(1.0)));
        }
    }
}

pub fn process_group_dynamics(population: &mut [Individual], groups: &mut Vec<Value>, sim_day: i32) -> Vec<Value> {
    let mut events = Vec::new();
    let mut spawned_groups = Vec::new();

    // group_id -> members via one single mutable pass over `population`
    // (each individual can only end up in one group's bucket, so this is
    // safe -- no overlapping &mut borrows) instead of an O(members) filter
    // scan of the *entire* population once per group, an O(groups *
    // population) cost that got sharply worse as both grew over a long run.
    // Keyed off each group's own member_ids (not individual.group_id) to
    // preserve the exact membership semantics the per-group filter had.
    let mut individual_group: HashMap<String, String> = HashMap::new();
    for g in groups.iter() {
        let Some(gid) = g.get("id").and_then(Value::as_str) else { continue };
        let Some(ids) = g.get("member_ids").and_then(Value::as_array) else { continue };
        for id in ids.iter().filter_map(Value::as_str) {
            individual_group.insert(id.to_string(), gid.to_string());
        }
    }
    let mut members_by_group: HashMap<String, Vec<&mut Individual>> = HashMap::new();
    for ind in population.iter_mut() {
        if let Some(gid) = individual_group.get(ind.id.as_str()) {
            members_by_group.entry(gid.clone()).or_default().push(ind);
        }
    }

    for group in groups.iter_mut() {
        let Some(group_id) = group.get("id").and_then(Value::as_str).map(String::from) else {
            continue;
        };
        let Some(mut members) = members_by_group.remove(&group_id) else {
            continue;
        };
        if members.len() < 2 {
            continue;
        }
        let leader_id = group.get("leader_id").and_then(Value::as_str).map(String::from);
        let leader_idx = leader_id.as_ref().and_then(|lid| members.iter().position(|m| &m.id == lid));

        if let Some(leader_idx) = leader_idx {
            if let Some(new_leader_id) = find_and_resolve_leadership_challenge(&members, leader_idx, group) {
                if let Some(obj) = group.as_object_mut() {
                    obj.insert("leader_id".to_string(), json!(new_leader_id));
                }
                events.push(json!({
                    "type": "leadership_change",
                    "group_id": group.get("id").cloned().unwrap_or(Value::Null),
                    "new_leader_id": new_leader_id,
                    "day": sim_day
                }));
            }
        } else {
            let new_leader = members
                .iter()
                .max_by(|a, b| {
                    compute_social_status(a, Some(group))
                        .partial_cmp(&compute_social_status(b, Some(group)))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|ind| ind.id.clone());
            if let Some(new_leader) = new_leader {
                if let Some(obj) = group.as_object_mut() {
                    obj.insert("leader_id".to_string(), json!(new_leader));
                }
            }
        }

        let tension = group.get("internal_tension").and_then(Value::as_f64).unwrap_or(0.0);
        if members.len() > 25 || tension > 0.8 {
            if let Some(new_group) = attempt_group_fission(&mut members, group, sim_day) {
                let dissenter_ids: Vec<Value> = new_group.get("member_ids").and_then(Value::as_array).cloned().unwrap_or_default();
                if let Some(obj) = group.as_object_mut() {
                    let remaining: Vec<Value> = obj
                        .get("member_ids")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|id| !dissenter_ids.contains(id))
                        .collect();
                    obj.insert("member_ids".to_string(), Value::Array(remaining));
                    let mut rivals = obj.get("rival_ids").and_then(Value::as_array).cloned().unwrap_or_default();
                    rivals.push(new_group.get("id").cloned().unwrap_or(Value::Null));
                    obj.insert("rival_ids".to_string(), Value::Array(rivals));
                }
                events.push(json!({
                    "type": "group_split",
                    "parent_group_id": group.get("id").cloned().unwrap_or(Value::Null),
                    "new_group_id": new_group.get("id").cloned().unwrap_or(Value::Null),
                    "day": sim_day
                }));
                spawned_groups.push(new_group);
            }
        }
    }
    groups.append(&mut spawned_groups);
    events
}

fn avg_aggression(members: &[&Individual]) -> f64 {
    if members.is_empty() {
        return 0.0;
    }
    members.iter().map(|m| m.phenotype.aggression).sum::<f64>() / members.len() as f64
}

/// Pure (no randomness) resolution of how many defenders a lost clash costs.
/// Losses scale with how outmatched the defenders are, and shrink as
/// `defense_bonus` grows -- a defended settlement's whole point is to blunt
/// this. Always leaves at least one defender alive.
fn compute_conflict_losses(attacker_power: f64, defender_power: f64, defender_count: usize, defense_bonus: f64) -> usize {
    let base_losses = 1.0 + (attacker_power - defender_power) / defender_power.max(0.1);
    ((base_losses / (1.0 + defense_bonus)).round() as usize).clamp(1, defender_count.saturating_sub(1).max(1))
}

/// Tense rivalries (tracked via `rival_ids`, set when a group fissions --
/// see `process_group_dynamics` above) can escalate into a rare clash. This
/// is the only place `architecture::compute_settlement_defense` is ever
/// called: it was computed and unit-tested since the JS port but never
/// wired into anything, so a "defensive_wall" structure had no actual
/// effect on anyone's survival. A defended settlement now measurably
/// reduces the defending group's casualties in a clash it loses.
///
/// Deliberately conservative (tension must already be high, and the daily
/// roll is tiny even then) -- this models an occasional raid between
/// already-hostile bands, not routine warfare, so it can't destabilize
/// population growth in the common case where groups never fission at all.
pub fn process_intergroup_conflict(population: &mut [Individual], groups: &mut [Value], settlements: &[Value], sim_day: i32) -> Vec<Value> {
    let mut events = Vec::new();
    let mut casualty_ids: Vec<(String, String)> = Vec::new(); // (individual_id, defender_group_id)

    // The first two founders and their first few descendants are a household,
    // not competing societies. This global, uniform gate only permits the
    // existing emergent conflict process once there is a viable population.
    let living_population = population.iter().filter(|ind| ind.alive && !ind.is_dead).count();
    if living_population < MIN_CONFLICT_POPULATION {
        return events;
    }

    // group_id -> alive members, built once via a single population pass
    // (mirrors process_group_dynamics's own precomputed map above) instead of
    // two full population.iter().filter() scans per qualifying group.
    // apply_resource_tension raises every group's tension in lockstep under
    // sustained famine, so many groups can cross the 0.5 gate in the same
    // tick -- the old per-group scan made the worst case O(groups *
    // population) precisely when the simulation is already under the most
    // stress.
    let mut individual_group: HashMap<&str, &str> = HashMap::new();
    for g in groups.iter() {
        let Some(gid) = g.get("id").and_then(Value::as_str) else { continue };
        let Some(ids) = g.get("member_ids").and_then(Value::as_array) else { continue };
        for id in ids.iter().filter_map(Value::as_str) {
            individual_group.insert(id, gid);
        }
    }
    let mut members_by_group: HashMap<&str, Vec<&Individual>> = HashMap::new();
    for ind in population.iter() {
        if !ind.alive || ind.is_dead {
            continue;
        }
        if let Some(&gid) = individual_group.get(ind.id.as_str()) {
            members_by_group.entry(gid).or_default().push(ind);
        }
    }

    for group in groups.iter() {
        let Some(group_id) = group.get("id").and_then(Value::as_str) else { continue };
        let tension = group.get("internal_tension").and_then(Value::as_f64).unwrap_or(0.0);
        if tension < 0.5 {
            continue;
        }
        let Some(rival_ids) = group.get("rival_ids").and_then(Value::as_array).filter(|a| !a.is_empty()) else { continue };
        if rand::random::<f64>() > tension * 0.01 {
            continue;
        }
        let rival_id = match rival_ids[rand::random::<usize>() % rival_ids.len()].as_str() {
            Some(id) => id.to_string(),
            None => continue,
        };
        // Still confirms the rival group actually still exists (could have
        // been dissolved/merged away since this stale rival_ids entry was
        // recorded at fission time) -- the group's own data is no longer
        // needed after that, since attacker/defender membership now comes
        // from members_by_group instead of re-reading member_ids here.
        if !groups.iter().any(|g| g.get("id").and_then(Value::as_str) == Some(rival_id.as_str())) {
            continue;
        }

        let attackers: &[&Individual] = members_by_group.get(group_id).map(Vec::as_slice).unwrap_or(&[]);
        let defenders: &[&Individual] = members_by_group.get(rival_id.as_str()).map(Vec::as_slice).unwrap_or(&[]);
        let adult_attackers = attackers.iter().filter(|ind| is_social_adult(ind, sim_day)).count();
        let adult_defenders = defenders.iter().filter(|ind| is_social_adult(ind, sim_day)).count();
        if attackers.len() < 2 || defenders.len() < 2
            || adult_attackers < MIN_CONFLICT_ADULT_MEMBERS_PER_GROUP
            || adult_defenders < MIN_CONFLICT_ADULT_MEMBERS_PER_GROUP
        {
            continue;
        }

        let attacker_power = avg_aggression(attackers) * attackers.len() as f64;
        let defense_bonus = settlements
            .iter()
            .filter(|s| s.get("group_id").and_then(Value::as_str) == Some(rival_id.as_str()))
            .map(crate::architecture::compute_settlement_defense)
            .fold(0.0_f64, f64::max);
        let defender_power = avg_aggression(defenders) * defenders.len() as f64 * (1.0 + defense_bonus);
        if attacker_power <= defender_power {
            continue;
        }

        let losses = compute_conflict_losses(attacker_power, defender_power, defenders.len(), defense_bonus);
        let mut pool: Vec<&Individual> = defenders.to_vec();
        for _ in 0..losses {
            if pool.is_empty() {
                break;
            }
            // Every other death pathway in this codebase (background
            // mortality, disaster, infection, predator/disease exposure --
            // see mortality.rs/microbiome.rs) deliberately halves a
            // founder's risk; the uniform `idx = random() % pool.len()`
            // this replaced gave founders no such protection at all when a
            // clash was lost, making conflict a disproportionate share of
            // observed founder deaths (see founder_mortality_probe.rs).
            // Weighting the casualty draw the same way (founder weight 0.5
            // vs 1.0) brings this in line with the rest of the model instead
            // of being the one exception to it.
            const FOUNDER_CASUALTY_WEIGHT: f64 = 0.5;
            let weights: Vec<f64> = pool.iter().map(|ind| if ind.is_founder { FOUNDER_CASUALTY_WEIGHT } else { 1.0 }).collect();
            let total: f64 = weights.iter().sum();
            let mut roll = rand::random::<f64>() * total;
            let mut idx = pool.len() - 1;
            for (i, w) in weights.iter().enumerate() {
                if roll < *w {
                    idx = i;
                    break;
                }
                roll -= w;
            }
            casualty_ids.push((pool.remove(idx).id.clone(), rival_id.clone()));
        }

        // No "description" field baked in here -- mirrors group_split/leadership_change
        // (see routes.rs's describe_event), which generates a richer, translatable
        // description at the server layer instead.
        events.push(json!({
            "type": "conflict",
            "attacker_group_id": group_id,
            "defender_group_id": rival_id,
            "casualties": losses,
            "defense_bonus": (defense_bonus * 100.0).round() / 100.0,
            "day": sim_day,
        }));
    }

    if !casualty_ids.is_empty() {
        let dead_ids: HashSet<&str> = casualty_ids.iter().map(|(id, _)| id.as_str()).collect();
        for ind in population.iter_mut() {
            if dead_ids.contains(ind.id.as_str()) && !ind.is_dead {
                ind.is_dead = true;
                ind.alive = false;
                ind.death_day = Some(sim_day);
                ind.extra.insert("death_cause".to_string(), json!("conflict"));
                // The aggregate "conflict" event above only carries a
                // casualty *count*, never who died -- unlike every other
                // mortality path, which pushes an individual "death" event
                // per victim. Without this, a conflict death is correctly
                // reflected in the individuals table but never appears in
                // the event log.
                events.push(json!({
                    "type": "death",
                    "individual_id": ind.id,
                    "cause": "conflict",
                    "day": sim_day,
                    "importance": "medium",
                    "is_founder": ind.is_founder,
                }));
            }
        }
        for group in groups.iter_mut() {
            if let Some(members) = group.get_mut("member_ids").and_then(Value::as_array_mut) {
                members.retain(|v| !v.as_str().is_some_and(|id| dead_ids.contains(id)));
            }
        }
    }
    events
}

/// Shared by `assign_group_roles` (contiguous member slices, e.g. tests) and the
/// tick orchestrator (which assigns roles across a scattered population by group
/// membership rather than a contiguous slice).
pub fn compute_role_for(member: &Individual, leader_id: Option<&str>) -> &'static str {
    if member.is_founder {
        return "anchor";
    }
    if leader_id.is_some_and(|leader| leader == member.id) {
        return "leader";
    }
    let age_years = member.age_days.unwrap_or(0) as f64 / 365.0;
    let dominant = member
        .extra
        .get("_behaviorCounts")
        .and_then(Value::as_object)
        .and_then(|counts| counts.iter().max_by_key(|(_, v)| v.as_i64().unwrap_or(0)).map(|(k, _)| k.clone()));
    // "healer" deliberately still derives from `socialize` dominance, not
    // phenotype -- see role_emerges_from_behavior_counts_not_from_dominance_
    // or_other_phenotype below, an explicit invariant of this codebase that
    // group roles are 100% behavior-driven, never phenotype-driven, even
    // though phenotype would be cardinal-rule-legal (it's still genetic
    // inheritance). There is no dedicated `heal` action in agent.rs's action
    // set, so "the member who most often engages others socially" is the
    // closest behavioral proxy available -- not an arbitrary mislabel: in
    // real small-band societies the healer/shaman role was as much a social-
    // cohesion and mediator function as a medical one. MIN_SPECIALIZATION_COUNT
    // guards against a single incidental socialize action (against otherwise
    // all-zero counts) trivially winning the role -- a real specialization
    // signal requires a real behavioral track record.
    const MIN_SPECIALIZATION_COUNT: i64 = 5;
    let dominant_count = member
        .extra
        .get("_behaviorCounts")
        .and_then(Value::as_object)
        .and_then(|counts| dominant.as_deref().and_then(|d| counts.get(d)))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let specialized = dominant_count >= MIN_SPECIALIZATION_COUNT;
    match dominant.as_deref() {
        Some("hunt") if specialized => "warrior",
        Some("socialize") if specialized => "healer",
        Some("forage") if specialized => "gatherer",
        _ if age_years > 40.0 => "elder",
        _ => "member",
    }
}

pub fn assign_group_roles(members: &mut [Individual], leader_id: Option<&str>) {
    for member in members {
        let role = compute_role_for(member, leader_id);
        member.extra.insert("group_role".to_string(), json!(role));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member_with_behavior(dominant_action: &str) -> Individual {
        let mut m = Individual { age_days: Some(365 * 25), ..Default::default() };
        m.extra.insert("_behaviorCounts".to_string(), json!({ dominant_action: 50, "socialize": 1 }));
        m
    }

    #[test]
    fn role_emerges_from_behavior_counts_not_from_dominance_or_other_phenotype() {
        // Cardinal rule: roles emerge from `_behaviorCounts`, never assigned from
        // phenotype. Two members with identical (high) dominance but different
        // behavior histories must get different roles.
        let mut hunter = member_with_behavior("hunt");
        let mut gatherer = member_with_behavior("forage");
        hunter.phenotype = crate::types::Phenotype { dominance: 0.95, ..Default::default() };
        gatherer.phenotype = crate::types::Phenotype { dominance: 0.95, ..Default::default() }; // same dominance, different behavior

        assert_eq!(compute_role_for(&hunter, None), "warrior");
        assert_eq!(compute_role_for(&gatherer, None), "gatherer");
    }

    #[test]
    fn high_dominance_phenotype_alone_never_grants_the_leader_role() {
        // Only an explicit leader_id (itself derived from compute_social_status,
        // which is a behavioral/social computation, not a raw phenotype lookup)
        // may grant "leader" -- never dominance by itself.
        let mut ambitious = member_with_behavior("hunt");
        ambitious.phenotype = crate::types::Phenotype { dominance: 1.0, ..Default::default() };
        assert_ne!(compute_role_for(&ambitious, None), "leader");
        assert_eq!(compute_role_for(&ambitious, Some("someone-else")), "warrior");
    }

    #[test]
    fn only_the_designated_leader_id_receives_the_leader_role() {
        let mut member = member_with_behavior("hunt");
        member.id = "chief".to_string();
        assert_eq!(compute_role_for(&member, Some("chief")), "leader");
    }

    #[test]
    fn founders_are_always_anchors_regardless_of_behavior_or_leadership() {
        let mut founder = member_with_behavior("hunt");
        founder.is_founder = true;
        founder.id = "founder-1".to_string();
        assert_eq!(compute_role_for(&founder, Some("founder-1")), "anchor");
    }

    #[test]
    fn assign_group_roles_applies_the_same_rule_across_a_contiguous_slice() {
        let mut members = vec![member_with_behavior("hunt"), member_with_behavior("forage")];
        assign_group_roles(&mut members, None);
        assert_eq!(members[0].extra.get("group_role").and_then(Value::as_str), Some("warrior"));
        assert_eq!(members[1].extra.get("group_role").and_then(Value::as_str), Some("gatherer"));
    }

    // ── tension drivers (V-04 regression) ───────────────────────────────

    #[test]
    fn resource_pressure_raises_every_groups_tension() {
        let mut groups = vec![json!({ "id": "g1", "internal_tension": 0.2 }), json!({ "id": "g2", "internal_tension": 0.5 })];
        apply_resource_tension(&mut groups, 1.0, 1.0);
        assert!(groups[0]["internal_tension"].as_f64().unwrap() > 0.2);
        assert!(groups[1]["internal_tension"].as_f64().unwrap() > 0.5);
    }

    #[test]
    fn zero_resource_pressure_leaves_tension_unchanged() {
        let mut groups = vec![json!({ "id": "g1", "internal_tension": 0.2 })];
        apply_resource_tension(&mut groups, 0.0, 0.0);
        assert_eq!(groups[0]["internal_tension"], 0.2);
    }

    #[test]
    fn resource_tension_never_exceeds_one() {
        let mut groups = vec![json!({ "id": "g1", "internal_tension": 0.999 })];
        for _ in 0..1000 {
            apply_resource_tension(&mut groups, 1.0, 1.0);
        }
        assert!(groups[0]["internal_tension"].as_f64().unwrap() <= 1.0);
    }

    #[test]
    fn overcrowding_raises_only_the_matching_groups_tension() {
        let mut groups = vec![json!({ "id": "g1", "internal_tension": 0.2 }), json!({ "id": "g2", "internal_tension": 0.2 })];
        apply_overcrowding_tension(&mut groups, "g1");
        assert!(groups[0]["internal_tension"].as_f64().unwrap() > 0.2);
        assert_eq!(groups[1]["internal_tension"], 0.2);
    }

    #[test]
    fn sustained_resource_pressure_can_eventually_push_a_group_past_the_conflict_threshold() {
        // End-to-end proof that tension can now organically reach the >=0.5
        // gate process_intergroup_conflict requires -- previously impossible
        // without a test manually setting internal_tension, since nothing in
        // the whole simulation ever raised it.
        let mut groups = vec![json!({ "id": "g1", "internal_tension": 0.0 })];
        for _ in 0..200 {
            apply_resource_tension(&mut groups, 1.0, 1.0);
        }
        assert!(groups[0]["internal_tension"].as_f64().unwrap() >= 0.5);
    }

    // ── definition checks ───────────────────────────────────────────────

    #[test]
    fn defines_six_relationship_types() {
        assert_eq!(RELATIONSHIP_TYPES.len(), 6);
        for t in ["kin", "mate", "ally", "rival", "neutral", "outgroup"] {
            assert!(RELATIONSHIP_TYPES.contains(&t));
        }
    }

    #[test]
    fn defines_six_group_roles() {
        assert_eq!(GROUP_ROLES.len(), 6);
        for r in ["leader", "elder", "warrior", "gatherer", "healer", "member"] {
            assert!(GROUP_ROLES.contains(&r));
        }
    }

    #[test]
    fn founder_only_anchor_role_is_deliberately_excluded_from_group_roles() {
        // Locks in the decision documented on GROUP_ROLES itself: a caller
        // that iterates GROUP_ROLES to bucket/count roles across a
        // population must not expect founders (role "anchor") to show up in
        // it -- this is intentional, not a missing 7th entry.
        assert!(!GROUP_ROLES.contains(&"anchor"));
        assert_eq!(compute_role_for(&Individual { is_founder: true, ..Default::default() }, None), "anchor");
    }

    // ── computeSocialStatus ─────────────────────────────────────────────

    fn status_individual(dominance: f64, iq: f64, empathy: f64, strength: f64, reputation: f64, age_years: i32) -> Individual {
        Individual {
            phenotype: crate::types::Phenotype { dominance, fluid_intelligence: iq, empathy, physical_strength: strength, ..Default::default() },
            social: crate::types::Social { reputation, ..Default::default() },
            age_days: Some(age_years * 365),
            ..Default::default()
        }
    }

    #[test]
    fn social_status_is_zero_with_no_group() {
        let ind = status_individual(0.5, 0.5, 0.5, 0.5, 0.5, 25);
        assert_eq!(compute_social_status(&ind, None), 0.0);
    }

    #[test]
    fn social_status_stays_within_bounds() {
        let ind = status_individual(0.9, 0.9, 0.9, 0.9, 1.0, 25);
        let group = json!({ "id": "g1", "founded_day": 0 });
        let status = compute_social_status(&ind, Some(&group));
        assert!((0.0..=1.0).contains(&status));
    }

    #[test]
    fn higher_dominance_yields_higher_status() {
        let group = json!({ "id": "g1", "founded_day": 0 });
        let high = status_individual(0.9, 0.5, 0.5, 0.5, 0.5, 25);
        let low = status_individual(0.1, 0.5, 0.5, 0.5, 0.5, 25);
        assert!(compute_social_status(&high, Some(&group)) > compute_social_status(&low, Some(&group)));
    }

    #[test]
    fn higher_reputation_increases_status() {
        let group = json!({ "id": "g1", "founded_day": 0 });
        let base = status_individual(0.5, 0.5, 0.5, 0.5, 0.2, 25);
        let rep = status_individual(0.5, 0.5, 0.5, 0.5, 0.9, 25);
        assert!(compute_social_status(&rep, Some(&group)) > compute_social_status(&base, Some(&group)));
    }

    #[test]
    fn physical_strength_matters_more_in_young_groups_than_old_ones() {
        let young_group = json!({ "id": "g1", "founded_day": 0 });
        let old_group = json!({ "id": "g2", "founded_day": 5000 });
        let strong = status_individual(0.0, 0.0, 0.0, 0.99, 0.0, 30);
        assert!(compute_social_status(&strong, Some(&young_group)) > compute_social_status(&strong, Some(&old_group)));
    }

    // ── processGroupDynamics — leadership challenge ──────────────────────

    fn dynamics_individual(id: &str, dominance: f64, strength: f64) -> Individual {
        Individual {
            id: id.to_string(),
            group_id: Some("g1".to_string()),
            phenotype: crate::types::Phenotype { dominance, physical_strength: strength, fluid_intelligence: 0.5, empathy: 0.5, independence: 0.3, ..Default::default() },
            social: crate::types::Social { reputation: 0.5, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn a_much_stronger_challenger_can_eventually_unseat_the_leader() {
        let leader = dynamics_individual("leader", 0.1, 0.1);
        let challenger = dynamics_individual("challenger", 0.99, 0.99);
        let mut groups = vec![json!({ "id": "g1", "member_ids": ["leader", "challenger"], "leader_id": "leader", "founded_day": 0 })];
        let mut changed = false;
        for day in 0..3000 {
            let evs = process_group_dynamics(&mut [leader.clone(), challenger.clone()], &mut groups, day);
            if evs.iter().any(|e| e["type"] == "leadership_change") {
                changed = true;
                break;
            }
        }
        assert!(changed, "an overwhelmingly stronger challenger should eventually unseat the leader");
    }

    #[test]
    fn leadership_change_event_has_the_expected_shape() {
        let leader = dynamics_individual("leader", 0.05, 0.05);
        let challenger = dynamics_individual("challenger", 0.99, 0.99);
        let mut groups = vec![json!({ "id": "g1", "member_ids": ["leader", "challenger"], "leader_id": "leader", "founded_day": 0 })];
        let mut found = None;
        for day in 0..3000 {
            let evs = process_group_dynamics(&mut [leader.clone(), challenger.clone()], &mut groups, day);
            if let Some(ev) = evs.into_iter().find(|e| e["type"] == "leadership_change") {
                found = Some(ev);
                break;
            }
        }
        let ev = found.expect("expected a leadership change within 3000 days");
        assert_eq!(ev["group_id"], "g1");
        assert!(ev["new_leader_id"].is_string());
        assert!(ev["day"].as_i64().unwrap() >= 0);
    }

    // ── processGroupDynamics — group fission ─────────────────────────────

    fn fission_member(idx: usize) -> Individual {
        Individual {
            id: format!("m{idx}"),
            group_id: Some("g1".to_string()),
            birth_day: -25 * 365,
            x: 32.0,
            y: 38.0,
            phenotype: crate::types::Phenotype {
                dominance: 0.5,
                fluid_intelligence: 0.5,
                empathy: 0.5,
                physical_strength: 0.5,
                independence: 0.95,
                ..Default::default()
            },
            social: crate::types::Social { reputation: 0.2, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn a_large_high_tension_group_can_eventually_split() {
        let mut members: Vec<Individual> = (0..12).map(fission_member).collect();
        let member_ids: Vec<Value> = members.iter().map(|m| json!(m.id)).collect();
        let mut groups = vec![json!({ "id": "g1", "member_ids": member_ids, "leader_id": "m0", "internal_tension": 0.9, "founded_day": 0 })];
        let mut split = false;
        for day in 0..500 {
            let evs = process_group_dynamics(&mut members, &mut groups, day);
            if evs.iter().any(|e| e["type"] == "group_split") {
                split = true;
                break;
            }
        }
        assert!(split, "a large group under high tension should eventually fission");
        assert!(groups.len() >= 2, "fission must actually create a second group, not just emit an event");
    }

    #[test]
    fn group_split_event_has_the_expected_shape_and_moves_dissenters() {
        let mut members: Vec<Individual> = (0..12).map(fission_member).collect();
        let member_ids: Vec<Value> = members.iter().map(|m| json!(m.id)).collect();
        let mut groups = vec![json!({ "id": "g1", "member_ids": member_ids, "leader_id": "m0", "internal_tension": 0.9, "founded_day": 0 })];
        let mut ev = None;
        for day in 0..500 {
            let evs = process_group_dynamics(&mut members, &mut groups, day);
            if let Some(found) = evs.into_iter().find(|e| e["type"] == "group_split") {
                ev = Some(found);
                break;
            }
        }
        let ev = ev.expect("expected a group_split within 500 days");
        assert_eq!(ev["parent_group_id"], "g1");
        assert!(ev["new_group_id"].is_string());
        let new_group_id = ev["new_group_id"].as_str().unwrap();
        // Every dissenter moved into the new group must actually carry the new group_id.
        assert!(members.iter().any(|m| m.group_id.as_deref() == Some(new_group_id)));
    }

    #[test]
    fn dependent_children_cannot_split_a_founder_household() {
        let mut members: Vec<Individual> = (0..8).map(fission_member).collect();
        for child in members.iter_mut().skip(2) {
            child.birth_day = 0;
        }
        let member_ids: Vec<Value> = members.iter().map(|m| json!(m.id)).collect();
        let mut groups = vec![json!({
            "id": "family",
            "member_ids": member_ids,
            "leader_id": "m0",
            "internal_tension": 1.0,
            "founded_day": 0,
        })];

        for day in 0..365 {
            let events = process_group_dynamics(&mut members, &mut groups, day);
            assert!(
                !events.iter().any(|event| event["type"] == "group_split"),
                "two adults and six dependent children must remain one household"
            );
        }
        assert_eq!(groups.len(), 1);
    }

    // ── intergroup conflict ────────────────────────────────────────────

    fn conflict_member(id: &str, group_id: &str, aggression: f64) -> Individual {
        Individual {
            id: id.to_string(),
            group_id: Some(group_id.to_string()),
            alive: true,
            birth_day: -25 * 365,
            phenotype: crate::types::Phenotype { aggression, ..Default::default() },
            ..Default::default()
        }
    }

    fn two_rival_groups(attacker_n: usize, attacker_aggression: f64, defender_n: usize, defender_aggression: f64, tension: f64) -> (Vec<Individual>, Vec<Value>) {
        let mut members: Vec<Individual> = (0..attacker_n).map(|i| conflict_member(&format!("a{i}"), "attackers", attacker_aggression)).collect();
        members.extend((0..defender_n).map(|i| conflict_member(&format!("d{i}"), "defenders", defender_aggression)));
        let attacker_ids: Vec<Value> = (0..attacker_n).map(|i| json!(format!("a{i}"))).collect();
        let defender_ids: Vec<Value> = (0..defender_n).map(|i| json!(format!("d{i}"))).collect();
        let groups = vec![
            json!({ "id": "attackers", "member_ids": attacker_ids, "internal_tension": tension, "rival_ids": ["defenders"] }),
            json!({ "id": "defenders", "member_ids": defender_ids, "internal_tension": 0.0, "rival_ids": ["attackers"] }),
        ];
        (members, groups)
    }

    #[test]
    fn avg_aggression_of_an_empty_slice_is_zero() {
        assert_eq!(avg_aggression(&[]), 0.0);
    }

    #[test]
    fn no_conflict_when_tension_is_below_threshold() {
        let (mut members, mut groups) = two_rival_groups(6, 0.9, 3, 0.1, 0.49);
        for day in 0..500 {
            let events = process_intergroup_conflict(&mut members, &mut groups, &[], day);
            assert!(events.is_empty(), "tension below 0.5 must never trigger a conflict roll");
        }
        assert!(members.iter().all(|m| !m.is_dead));
    }

    #[test]
    fn no_conflict_without_any_rival_groups() {
        let (mut members, mut groups) = two_rival_groups(6, 0.9, 3, 0.1, 1.0);
        for group in groups.iter_mut() {
            group["rival_ids"] = json!([]);
        }
        for day in 0..500 {
            let events = process_intergroup_conflict(&mut members, &mut groups, &[], day);
            assert!(events.is_empty(), "a group with no rivals can never clash, no matter how high tension is");
        }
    }

    #[test]
    fn founder_household_scale_cannot_start_an_intergroup_conflict() {
        // Two founders plus six descendants may be manually arranged into two
        // groups, but that is still too early for an intergroup-war event.
        let (mut members, mut groups) = two_rival_groups(5, 1.0, 3, 0.01, 1.0);
        for day in 0..500 {
            let events = process_intergroup_conflict(&mut members, &mut groups, &[], day);
            assert!(events.is_empty(), "an eight-person founder household must not generate a conflict event");
        }
        assert!(members.iter().all(|member| !member.is_dead));
    }

    #[test]
    fn compute_conflict_losses_always_leaves_at_least_one_defender_alive() {
        let losses = compute_conflict_losses(100.0, 1.0, 3, 0.0);
        assert!(losses < 3);
    }

    #[test]
    fn a_higher_defense_bonus_reduces_losses_for_the_same_power_differential() {
        let undefended = compute_conflict_losses(10.0, 2.0, 20, 0.0);
        let defended = compute_conflict_losses(10.0, 2.0, 20, 1.5);
        assert!(defended <= undefended, "a defended settlement should never increase losses (undefended={undefended}, defended={defended})");
        assert!(defended < undefended, "a strong defense_bonus should strictly reduce losses for an otherwise-identical clash");
    }

    #[test]
    fn a_lost_clash_kills_defenders_with_conflict_as_the_death_cause_and_prunes_group_membership() {
        // Overwhelming attacker aggression/count vs a weak, tiny defending group,
        // and max tension so the daily roll (tension*0.01 = 1%) fires within a
        // generous number of tries.
        let (mut members, mut groups) = two_rival_groups(10, 1.0, 2, 0.05, 1.0);
        let mut conflict_event = None;
        for day in 0..20_000 {
            let events = process_intergroup_conflict(&mut members, &mut groups, &[], day);
            if let Some(ev) = events.into_iter().find(|e| e["type"] == "conflict") {
                conflict_event = Some(ev);
                break;
            }
        }
        let ev = conflict_event.expect("expected at least one conflict within 20,000 days at a 1%/day roll");
        assert_eq!(ev["attacker_group_id"], "attackers");
        assert_eq!(ev["defender_group_id"], "defenders");
        assert!(ev["casualties"].as_u64().unwrap() >= 1);

        let dead: Vec<&Individual> = members.iter().filter(|m| m.is_dead).collect();
        assert!(!dead.is_empty(), "the losing side should have at least one real casualty");
        assert!(dead.iter().all(|m| m.group_id.as_deref() == Some("defenders")), "only defenders should die when attackers win");
        assert!(dead.iter().all(|m| m.extra.get("death_cause").and_then(Value::as_str) == Some("conflict")));

        let defender_group = groups.iter().find(|g| g["id"] == "defenders").unwrap();
        let remaining_ids: Vec<&str> = defender_group["member_ids"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
        for dead_ind in &dead {
            assert!(!remaining_ids.contains(&dead_ind.id.as_str()), "a dead defender must be pruned from member_ids");
        }
    }

    #[test]
    fn founders_are_less_likely_to_be_picked_as_conflict_casualties_than_non_founders() {
        // founder_mortality_probe.rs found conflict was responsible for a
        // disproportionate 26% of observed founder deaths over 10 simulated
        // years -- the one death pathway in this codebase that gave founders
        // no protection at all when their side lost a clash, unlike
        // background mortality/disaster/infection (all 0.4-0.5x). This large,
        // evenly-matched pool isolates the casualty-selection weighting from
        // compute_conflict_losses' own count logic: with a defending group
        // split 50/50 founder/non-founder and heavy, repeated losses, a
        // uniform draw would kill each side about equally often.
        const TRIALS: usize = 300;
        let mut founder_casualties = 0u32;
        let mut non_founder_casualties = 0u32;
        for trial in 0..TRIALS {
            let mut members: Vec<Individual> = (0..20).map(|i| conflict_member(&format!("a{i}"), "attackers", 1.0)).collect();
            for i in 0..20 {
                let mut d = conflict_member(&format!("d{i}"), "defenders", 0.05);
                d.is_founder = i % 2 == 0;
                members.push(d);
            }
            let attacker_ids: Vec<Value> = (0..20).map(|i| json!(format!("a{i}"))).collect();
            let defender_ids: Vec<Value> = (0..20).map(|i| json!(format!("d{i}"))).collect();
            let mut groups = vec![
                json!({ "id": "attackers", "member_ids": attacker_ids, "internal_tension": 1.0, "rival_ids": ["defenders"] }),
                json!({ "id": "defenders", "member_ids": defender_ids, "internal_tension": 0.0, "rival_ids": ["attackers"] }),
            ];
            for day in 0..(trial as i32) % 200 + 1 {
                process_intergroup_conflict(&mut members, &mut groups, &[], day);
            }
            for m in &members {
                if m.is_dead && m.group_id.as_deref() == Some("defenders") {
                    if m.is_founder {
                        founder_casualties += 1;
                    } else {
                        non_founder_casualties += 1;
                    }
                }
            }
        }
        assert!(founder_casualties + non_founder_casualties > 0, "expected at least some conflict casualties across {TRIALS} trials");
        assert!(
            non_founder_casualties > founder_casualties,
            "non-founders ({non_founder_casualties}) should be picked as conflict casualties more often than founders ({founder_casualties})"
        );
    }

    #[test]
    fn conflict_death_event_carries_is_founder_matching_the_actual_victim() {
        // The frontend plays a distinct founder-death alarm keyed off
        // data.is_founder -- this must reflect who actually died in a lost
        // clash, not just be a hardcoded field.
        let (mut members, mut groups) = two_rival_groups(10, 1.0, 2, 0.05, 1.0);
        let last = members.len() - 1;
        members[last].is_founder = true; // one of the two defenders is a founder
        let mut death_events = Vec::new();
        for day in 0..20_000 {
            let events = process_intergroup_conflict(&mut members, &mut groups, &[], day);
            death_events.extend(events.into_iter().filter(|e| e["type"] == "death"));
            if !death_events.is_empty() {
                break;
            }
        }
        assert!(!death_events.is_empty(), "expected at least one death within 20,000 days");
        for ev in &death_events {
            let victim_id = ev["individual_id"].as_str().unwrap();
            let victim = members.iter().find(|m| m.id == victim_id).unwrap();
            assert_eq!(ev["is_founder"], victim.is_founder, "death event's is_founder must match the actual victim {victim_id}");
        }
    }

    #[test]
    fn a_defensive_wall_settlement_lowers_the_reported_defense_bonus_below_what_an_undefended_group_gets() {
        let (mut members, mut groups) = two_rival_groups(10, 1.0, 2, 0.05, 1.0);
        let settlements = vec![json!({ "group_id": "defenders", "structures": [{ "type": "defensive_wall", "condition": 1.0 }] })];
        let mut conflict_event = None;
        for day in 0..20_000 {
            let events = process_intergroup_conflict(&mut members, &mut groups, &settlements, day);
            if let Some(ev) = events.into_iter().find(|e| e["type"] == "conflict") {
                conflict_event = Some(ev);
                break;
            }
        }
        let ev = conflict_event.expect("expected at least one conflict within 20,000 days");
        assert!(ev["defense_bonus"].as_f64().unwrap() > 0.0, "a defensive_wall settlement should produce a nonzero defense_bonus");
    }
}
