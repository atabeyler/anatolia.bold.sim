use serde_json::{json, Value};
use std::collections::HashSet;

/// Emits high-importance "civilization first" events the first time each
/// threshold is crossed. Pure bookkeeping -- fires once per key, never again,
/// tracked via `already_fired` (persisted on SimulationState.milestones).
#[allow(clippy::too_many_arguments)]
pub fn check_milestones(
    alive_count: usize,
    discovered_techs_count: usize,
    discovered_beliefs_count: usize,
    discovered_arts_count: usize,
    max_language_stage: i32,
    day: i32,
    already_fired: &mut HashSet<String>,
) -> Vec<Value> {
    let candidates: &[(&str, bool, &str, &str)] = &[
        ("pop_10", alive_count >= 10, "Population reached 10 individuals", "\u{1F465}"),
        ("pop_25", alive_count >= 25, "Population reached 25 individuals", "\u{1F465}"),
        ("pop_50", alive_count >= 50, "Population reached 50 individuals", "\u{1F465}"),
        ("pop_100", alive_count >= 100, "Population milestone: 100 individuals", "\u{1F465}"),
        ("pop_250", alive_count >= 250, "Population milestone: 250 individuals", "\u{1F465}"),
        ("pop_500", alive_count >= 500, "Population milestone: 500 individuals", "\u{1F465}"),
        ("tech_5", discovered_techs_count >= 5, "5 technologies discovered", "\u{2699}\u{FE0F}"),
        ("tech_10", discovered_techs_count >= 10, "10 technologies discovered", "\u{2699}\u{FE0F}"),
        ("tech_15", discovered_techs_count >= 15, "15 technologies discovered", "\u{2699}\u{FE0F}"),
        ("belief_first", discovered_beliefs_count >= 1, "First belief system emerged", "\u{263D}"),
        ("belief_5", discovered_beliefs_count >= 5, "5 belief systems recorded", "\u{263D}"),
        ("art_first", discovered_arts_count >= 1, "First art form created", "\u{1F3A8}"),
        ("lang_stage2", max_language_stage >= 2, "First phonemic language stage reached", "\u{1F524}"),
        ("lang_stage3", max_language_stage >= 3, "Morphemic grammar emerged in the community", "\u{1F524}"),
        ("lang_stage4", max_language_stage >= 4, "Complex syntax achieved -- full language capacity", "\u{1F524}"),
        ("lang_stage5", max_language_stage >= 5, "Writing system invented", "\u{1F4DC}"),
        ("lang_stage6", max_language_stage >= 6, "Literature era begins", "\u{1F4D6}"),
        ("year_10", day >= 10 * 365, "Civilization survived 10 years", "\u{23F3}"),
        ("year_100", day >= 100 * 365, "Civilization survived 100 years", "\u{23F3}"),
        ("year_500", day >= 500 * 365, "Civilization survived 500 years", "\u{23F3}"),
        ("year_1000", day >= 1000 * 365, "Civilization survived 1000 years", "\u{23F3}"),
    ];

    let mut events = Vec::new();
    for (key, condition, description, icon) in candidates {
        if !*condition || already_fired.contains(*key) {
            continue;
        }
        already_fired.insert((*key).to_string());
        events.push(json!({
            "type": "milestone",
            "key": key,
            "description": description,
            "icon": icon,
            "day": day,
            "importance": "high",
        }));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_pop_10_milestone_when_population_reaches_ten() {
        let mut fired = HashSet::new();
        let events = check_milestones(10, 0, 0, 0, 0, 1, &mut fired);
        assert!(events.iter().any(|e| e["key"] == "pop_10"));
    }

    #[test]
    fn does_not_re_fire_an_already_triggered_milestone() {
        let mut fired = HashSet::new();
        fired.insert("pop_10".to_string());
        let events = check_milestones(10, 0, 0, 0, 0, 1, &mut fired);
        assert!(!events.iter().any(|e| e["key"] == "pop_10"));
    }

    #[test]
    fn fires_year_10_milestone_at_day_3650() {
        let mut fired = HashSet::new();
        let events = check_milestones(0, 0, 0, 0, 0, 10 * 365, &mut fired);
        assert!(events.iter().any(|e| e["key"] == "year_10"));
    }

    #[test]
    fn milestone_event_has_type_milestone_and_importance_high() {
        let mut fired = HashSet::new();
        let events = check_milestones(10, 0, 0, 0, 0, 1, &mut fired);
        let ev = events.iter().find(|e| e["type"] == "milestone").expect("a milestone event");
        assert_eq!(ev["importance"], "high");
        assert!(ev["description"].is_string());
    }

    #[test]
    fn each_milestone_key_fires_at_most_once_across_repeated_calls() {
        let mut fired = HashSet::new();
        let mut total_pop_10_fires = 0;
        for day in 1..5 {
            let events = check_milestones(10, 0, 0, 0, 0, day, &mut fired);
            total_pop_10_fires += events.iter().filter(|e| e["key"] == "pop_10").count();
        }
        assert_eq!(total_pop_10_fires, 1);
    }
}
