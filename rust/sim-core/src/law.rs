use std::collections::HashSet;

use serde_json::{json, Value};

#[allow(clippy::type_complexity)]
pub const NORM_TYPES: &[(&str, i32, f64, usize, f64, &[&str])] = &[
    ("reciprocity", 1, 0.0, 2, 0.2, &[]),
    ("no_theft", 1, 0.0, 3, 0.3, &[]),
    ("incest_taboo", 1, 0.0, 2, 0.2, &[]),
    ("elder_respect", 2, 0.3, 4, 0.4, &[]),
    ("hospitality", 2, 0.3, 5, 0.4, &[]),
    ("blood_feud", 2, 0.3, 4, 0.4, &[]),
    ("communal_work", 2, 0.35, 5, 0.4, &[]),
    ("leader_arbitration", 3, 0.5, 8, 0.55, &[]),
    ("property_rights", 3, 0.5, 8, 0.55, &[]),
    ("punishment_exile", 3, 0.5, 8, 0.55, &[]),
    ("written_law", 4, 0.65, 15, 0.65, &["writing_system"]),
    ("tax_system", 4, 0.65, 20, 0.65, &["writing_system", "mathematics_basic"]),
    ("contract_law", 4, 0.7, 20, 0.7, &["writing_system"]),
];

fn norm_description(norm_id: &str) -> &str {
    match norm_id {
        "reciprocity" => "Members are expected to return favors",
        "no_theft" => "Taking others' possessions is prohibited",
        "incest_taboo" => "Mating between close relatives is forbidden",
        "elder_respect" => "Elders are addressed with deference",
        "hospitality" => "Strangers must be offered food and shelter",
        "blood_feud" => "Violence against a kin member demands revenge",
        "communal_work" => "All able members must contribute to group tasks",
        "leader_arbitration" => "The leader resolves disputes",
        "property_rights" => "Individual ownership of goods is recognized",
        "punishment_exile" => "Persistent violators may be driven out",
        "written_law" => "Rules are codified in written form",
        "tax_system" => "Members contribute a portion of resources to the group",
        "contract_law" => "Agreements between parties are legally binding",
        other => other,
    }
}

/// `members` is this group's own membership (alive only), already resolved
/// by the caller (see tick.rs) -- letting this filter the *entire*
/// population itself, once per group, was an O(groups * population) cost
/// that got sharply worse as both grew over a long run.
pub fn process_law_tick(
    group: &mut Value,
    members: &[&crate::state::Individual],
    discovered_techs: &HashSet<String>,
    sim_day: i32,
) -> Vec<Value> {
    let mut events = Vec::new();
    if members.len() < 2 {
        return events;
    }
    let norms: HashSet<String> = group
        .get("norms")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).map(ToString::to_string).collect())
        .unwrap_or_default();
    let avg_foxp2 = members.iter().map(|m| m.language.foxp2_expression).sum::<f64>() / members.len() as f64;
    let avg_iq = members.iter().map(|m| m.phenotype.fluid_intelligence).sum::<f64>() / members.len() as f64;
    let leader = members.iter().find(|m| group.get("leader_id").and_then(Value::as_str) == Some(m.id.as_str()));
    let mut updated_norms = norms;
    for (norm_id, stage, iq_min, group_min, foxp2_min, requires_tech) in NORM_TYPES {
        if updated_norms.contains(*norm_id) || members.len() < *group_min || avg_foxp2 < *foxp2_min || avg_iq < *iq_min {
            continue;
        }
        if requires_tech.iter().any(|t| !discovered_techs.contains(*t)) {
            continue;
        }
        let tension = group.get("internal_tension").and_then(Value::as_f64).unwrap_or(0.3);
        let lead_iq = leader.map(|m| m.phenotype.fluid_intelligence).unwrap_or(0.4);
        if rand::random::<f64>() < (tension + lead_iq * 0.3) * 0.0005 {
            updated_norms.insert((*norm_id).to_string());
            events.push(json!({
                "type": "norm_emerged",
                "norm_id": norm_id,
                "group_id": group.get("id").cloned().unwrap_or(Value::Null),
                "day": sim_day,
                "importance": if *stage >= 4 { "high" } else if *stage >= 3 { "medium" } else { "low" },
                "description": norm_description(norm_id),
            }));
        }
    }
    group["norms"] = Value::Array(updated_norms.into_iter().map(Value::String).collect());
    events
}

/// Cardinal rule: whether *this* individual violates an existing norm today is
/// driven only by their own phenotype (aggression vs. conscientiousness), never
/// by an external lottery that ignores who they are. Returns the violated norm.
pub fn check_norm_violation(individual: &crate::state::Individual, norms: &HashSet<String>) -> Option<&'static str> {
    if norms.is_empty() || individual.is_dead {
        return None;
    }
    let propensity = (individual.phenotype.aggression * 0.7 + (1.0 - individual.phenotype.conscientiousness) * 0.3).clamp(0.0, 1.0);
    if rand::random::<f64>() >= propensity * 0.01 {
        return None;
    }
    // Which specific adopted norm gets attributed isn't itself part of the
    // cardinal-rule gate above (that only governs *whether* a violation
    // happens) -- but picking the first NORM_TYPES-table-order entry the
    // group happened to have adopted made every violation by every
    // individual in a given group always pin to the same rule, an artifact
    // of table order rather than anything about that instance. Pick
    // uniformly among the group's own currently-adopted norms instead.
    let adopted: Vec<&'static str> = NORM_TYPES.iter().map(|(id, ..)| *id).filter(|id| norms.contains(*id)).collect();
    if adopted.is_empty() {
        return None;
    }
    Some(adopted[rand::random::<usize>() % adopted.len()])
}

/// Records the violation against the individual's own history and, once a group
/// has adopted `punishment_exile` and the same person has crossed the threshold,
/// removes them from the group. No random victim selection -- the exiled person
/// is always the one whose own violation history triggered it.
pub fn process_norm_enforcement(group: &mut Value, individual: &mut crate::state::Individual, violated_norm: &str, sim_day: i32) -> Value {
    const EXILE_THRESHOLD: i64 = 3;
    let has_exile_norm = group
        .get("norms")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().any(|v| v.as_str() == Some("punishment_exile")))
        .unwrap_or(false);

    let mut counts = individual.extra.get("_normViolations").and_then(Value::as_object).cloned().unwrap_or_default();
    let count = counts.get(violated_norm).and_then(Value::as_i64).unwrap_or(0) + 1;
    counts.insert(violated_norm.to_string(), json!(count));
    individual.extra.insert("_normViolations".to_string(), Value::Object(counts));

    if has_exile_norm && count >= EXILE_THRESHOLD {
        individual.group_id = None;
        if let Some(obj) = group.as_object_mut() {
            let mut members = obj.get("member_ids").and_then(Value::as_array).cloned().unwrap_or_default();
            members.retain(|v| v.as_str() != Some(individual.id.as_str()));
            obj.insert("member_ids".to_string(), Value::Array(members));
        }
        return json!({
            "type": "norm_violation", "norm_id": violated_norm, "individual_id": individual.id,
            "punishment": "exile", "day": sim_day, "importance": "high",
            "description": format!("Norm violated: {}", norm_description(violated_norm)),
        });
    }
    json!({
        "type": "norm_violation", "norm_id": violated_norm, "individual_id": individual.id,
        "punishment": "none", "day": sim_day, "importance": "low",
        "description": format!("Norm violated: {}", norm_description(violated_norm)),
    })
}

pub fn compute_social_order(group: &Value) -> f64 {
    let member_count = group.get("member_ids").and_then(Value::as_array).map(|arr| arr.len().max(2)).unwrap_or(2) as f64;
    let norm_count = group.get("norms").and_then(Value::as_array).map(|arr| arr.len()).unwrap_or(0) as f64;
    let tension = group.get("internal_tension").and_then(Value::as_f64).unwrap_or(0.5);
    ((norm_count / member_count.log2()) * 0.08 + (1.0 - tension) * 0.4).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Individual;

    fn group_with_members(n: usize, foxp2: f64, iq: f64) -> (Value, Vec<Individual>) {
        let members: Vec<Individual> = (0..n)
            .map(|i| Individual {
                id: format!("m{i}"),
                phenotype: crate::types::Phenotype { fluid_intelligence: iq, ..Default::default() },
                language: crate::types::Language { foxp2_expression: foxp2, ..Default::default() },
                ..Default::default()
            })
            .collect();
        let group = json!({
            "id": "g1",
            "member_ids": members.iter().map(|m| json!(m.id)).collect::<Vec<_>>(),
            "leader_id": Value::Null,
            "norms": [],
            "internal_tension": 0.9,
        });
        (group, members)
    }

    #[test]
    fn a_group_below_the_size_and_language_thresholds_never_gets_any_norm() {
        let (mut group, members) = group_with_members(1, 0.0, 0.0); // below every norm's group_min
        let member_refs: Vec<&Individual> = members.iter().collect();
        let techs = HashSet::new();
        for day in 0..5000 {
            process_law_tick(&mut group, &member_refs, &techs, day);
        }
        assert!(group["norms"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_qualifying_group_can_eventually_emerge_a_norm() {
        let (mut group, members) = group_with_members(3, 0.5, 0.5); // meets reciprocity/no_theft/incest_taboo minimums
        let member_refs: Vec<&Individual> = members.iter().collect();
        let techs = HashSet::new();
        let mut emerged = false;
        for day in 0..200_000 {
            process_law_tick(&mut group, &member_refs, &techs, day);
            if !group["norms"].as_array().unwrap().is_empty() {
                emerged = true;
                break;
            }
        }
        assert!(emerged, "a qualifying group should eventually emerge at least one stage-1 norm");
    }

    fn individual_with(aggression: f64, conscientiousness: f64) -> Individual {
        Individual {
            id: "ind".to_string(),
            phenotype: crate::types::Phenotype { aggression, conscientiousness, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn no_violation_is_possible_when_the_group_has_no_norms() {
        let ind = individual_with(1.0, 0.0); // maximally violation-prone
        let norms = HashSet::new();
        for _ in 0..1000 {
            assert!(check_norm_violation(&ind, &norms).is_none());
        }
    }

    #[test]
    fn violation_likelihood_is_driven_by_the_individuals_own_phenotype() {
        // Cardinal rule check: high-aggression/low-conscientiousness individuals
        // must violate norms far more often than low-aggression/high-conscientiousness
        // ones under the *same* external conditions (same norm set, same trials).
        let mut norms = HashSet::new();
        norms.insert("no_theft".to_string());
        let prone = individual_with(1.0, 0.0);
        let restrained = individual_with(0.0, 1.0);

        let trials = 20_000;
        let prone_violations = (0..trials).filter(|_| check_norm_violation(&prone, &norms).is_some()).count();
        let restrained_violations = (0..trials).filter(|_| check_norm_violation(&restrained, &norms).is_some()).count();

        assert!(
            prone_violations > restrained_violations * 5,
            "expected the aggressive/low-conscientiousness individual to violate far more often: {prone_violations} vs {restrained_violations}"
        );
        assert_eq!(restrained_violations, 0, "propensity 0 must never roll a violation");
    }

    #[test]
    fn violated_norm_attribution_is_not_pinned_to_a_fixed_table_order_entry() {
        // Regression test: check_norm_violation used to always return the
        // first NORM_TYPES-table-order norm present in the group's adopted
        // set, so a group that adopted e.g. both no_theft and reciprocity
        // would attribute every single violation to whichever of the two
        // came first in NORM_TYPES, regardless of anything about that
        // instance. With multiple adopted norms, repeated trials should
        // eventually attribute more than just one of them.
        let mut norms = HashSet::new();
        norms.insert("no_theft".to_string());
        norms.insert("reciprocity".to_string());
        norms.insert("elder_respect".to_string());
        let prone = individual_with(1.0, 0.0);

        let mut seen: HashSet<&'static str> = HashSet::new();
        for _ in 0..2000 {
            if let Some(norm) = check_norm_violation(&prone, &norms) {
                assert!(norms.contains(norm), "attributed norm {norm} must be one the group actually adopted");
                seen.insert(norm);
            }
        }
        assert!(seen.len() > 1, "expected more than one distinct norm to be attributed across 2000 trials, got {seen:?}");
    }

    #[test]
    fn repeated_violations_lead_to_exile_only_once_the_group_has_adopted_punishment_exile() {
        let mut group = json!({
            "id": "g1",
            "member_ids": ["culprit", "other"],
            "norms": ["no_theft", "punishment_exile"],
        });
        let mut culprit = individual_with(1.0, 0.0);
        culprit.id = "culprit".to_string();
        culprit.group_id = Some("g1".to_string());

        let mut last_event = json!(null);
        for _ in 0..3 {
            last_event = process_norm_enforcement(&mut group, &mut culprit, "no_theft", 1);
        }
        assert_eq!(last_event["punishment"], "exile");
        assert!(culprit.group_id.is_none(), "the exiled individual must leave their group");
        assert!(!group["member_ids"].as_array().unwrap().iter().any(|v| v == "culprit"));
    }

    #[test]
    fn exile_never_happens_without_the_punishment_exile_norm_present() {
        let mut group = json!({ "id": "g1", "member_ids": ["culprit"], "norms": ["no_theft"] });
        let mut culprit = individual_with(1.0, 0.0);
        culprit.id = "culprit".to_string();
        culprit.group_id = Some("g1".to_string());
        for _ in 0..10 {
            let ev = process_norm_enforcement(&mut group, &mut culprit, "no_theft", 1);
            assert_eq!(ev["punishment"], "none");
        }
        assert!(culprit.group_id.is_some());
    }

    #[test]
    fn written_law_never_emerges_without_the_writing_system_prerequisite() {
        let (mut group, members) = group_with_members(30, 0.9, 0.9);
        let member_refs: Vec<&Individual> = members.iter().collect();
        let techs = HashSet::new(); // writing_system deliberately absent
        for day in 0..50_000 {
            process_law_tick(&mut group, &member_refs, &techs, day);
        }
        let norms: Vec<String> = group["norms"].as_array().unwrap().iter().filter_map(|v| v.as_str().map(String::from)).collect();
        assert!(!norms.contains(&"written_law".to_string()));
    }

    #[test]
    fn a_group_with_fewer_than_two_members_never_emerges_a_norm() {
        let (mut group, members) = group_with_members(1, 0.9, 0.9);
        let member_refs: Vec<&Individual> = members.iter().collect();
        let techs = HashSet::new();
        let events = process_law_tick(&mut group, &member_refs, &techs, 1);
        assert!(events.is_empty());
    }

    #[test]
    fn norm_emerged_event_has_the_expected_shape() {
        let (mut group, members) = group_with_members(10, 0.9, 0.9);
        let member_refs: Vec<&Individual> = members.iter().collect();
        let techs = HashSet::new();
        let mut norm_event = None;
        for day in 0..5000 {
            let events = process_law_tick(&mut group, &member_refs, &techs, day);
            if let Some(ev) = events.into_iter().find(|e| e["type"] == "norm_emerged") {
                norm_event = Some(ev);
                break;
            }
        }
        let ev = norm_event.expect("expected a norm_emerged event within 5000 days");
        assert_eq!(ev["type"], "norm_emerged");
        assert_eq!(ev["group_id"], "g1");
        assert!(ev["norm_id"].is_string());
        assert!(ev["description"].is_string());
    }

    // ── NORM_TYPES — definition checks ──────────────────────────────────

    #[test]
    fn defines_thirteen_norms() {
        assert_eq!(NORM_TYPES.len(), 13);
    }

    #[test]
    fn stage_one_norms_have_low_iq_min() {
        for (_, stage, iq_min, ..) in NORM_TYPES {
            if *stage == 1 {
                assert!(*iq_min <= 0.1);
            }
        }
    }

    #[test]
    fn written_law_requires_writing_system() {
        let (.., requires_tech) = NORM_TYPES.iter().find(|(id, ..)| *id == "written_law").unwrap();
        assert!(requires_tech.contains(&"writing_system"));
    }

    #[test]
    fn tax_system_requires_writing_and_math() {
        let (.., requires_tech) = NORM_TYPES.iter().find(|(id, ..)| *id == "tax_system").unwrap();
        assert!(requires_tech.contains(&"writing_system"));
        assert!(requires_tech.contains(&"mathematics_basic"));
    }

    // ── computeSocialOrder ───────────────────────────────────────────────

    fn group_with_norms(member_ids: &[&str], norms: &[&str], tension: f64) -> Value {
        json!({
            "id": "g1",
            "member_ids": member_ids,
            "norms": norms,
            "internal_tension": tension,
        })
    }

    #[test]
    fn social_order_is_zero_to_one_for_a_group_with_no_norms() {
        let group = group_with_norms(&["i1", "i2"], &[], 0.5);
        let order = compute_social_order(&group);
        assert!((0.0..=1.0).contains(&order));
    }

    #[test]
    fn more_norms_yield_higher_social_order() {
        let few = group_with_norms(&["i1", "i2", "i3", "i4"], &["reciprocity"], 0.3);
        let many = group_with_norms(
            &["i1", "i2", "i3", "i4"],
            &["reciprocity", "no_theft", "incest_taboo", "elder_respect"],
            0.3,
        );
        assert!(compute_social_order(&many) > compute_social_order(&few));
    }

    #[test]
    fn lower_tension_yields_higher_social_order() {
        let tense = group_with_norms(&["i1", "i2", "i3"], &["reciprocity"], 0.9);
        let calm = group_with_norms(&["i1", "i2", "i3"], &["reciprocity"], 0.1);
        assert!(compute_social_order(&calm) > compute_social_order(&tense));
    }

    #[test]
    fn social_order_is_always_between_zero_and_one() {
        let all_norms: Vec<&str> = NORM_TYPES.iter().map(|(id, ..)| *id).collect();
        let group = group_with_norms(&["i1", "i2", "i3"], &all_norms, 0.0);
        let order = compute_social_order(&group);
        assert!((0.0..=1.0).contains(&order));
    }
}
