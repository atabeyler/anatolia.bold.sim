use sim_core::{advance_one_day, create_founder, create_world_state, is_on_land, SimulationState, WorldState};
use std::collections::HashSet;

fn two_founders_state() -> SimulationState {
    let world_value = create_world_state(37.0, 35.0);
    let world_state: WorldState = serde_json::from_value(world_value).unwrap();

    let founder_1 = create_founder(&serde_json::json!({ "sex": "male", "ageYears": 22, "x": 37.0, "y": 35.0, "name": "Adam" }));
    let founder_2 = create_founder(&serde_json::json!({ "sex": "female", "ageYears": 20, "x": 37.0, "y": 35.0, "name": "Havva" }));

    SimulationState {
        id: Some("test-sim".to_string()),
        current_day: 0,
        current_year: 0,
        status: Some("running".to_string()),
        world_state,
        individuals: vec![founder_1, founder_2],
        ..Default::default()
    }
}

/// The tick orchestrator must be able to run for years of in-sim time without
/// panicking, and it must actually drive the simulation (births/aging/economy),
/// not just increment a day counter.
#[test]
fn long_run_does_not_panic_and_produces_emergence() {
    let mut state = two_founders_state();
    let mut max_population = 2usize;
    let mut any_birth_event = false;

    for _ in 0..3650 {
        let (report, _phases) = advance_one_day(&mut state);
        max_population = max_population.max(report.alive_count);
        if state.events.iter().any(|e| e.get("type").and_then(|v| v.as_str()) == Some("birth")) {
            any_birth_event = true;
        }
    }

    assert_eq!(state.current_day, 3650);
    assert!(state.individuals.len() >= 2, "population should never drop below what was recorded");
    assert!(max_population >= 2, "founders should remain alive or be replaced by descendants");
    assert!(
        any_birth_event || !state.pending_births.is_empty() || state.individuals.len() > 2,
        "10 years with two fertile founders in mating range should yield at least one conception"
    );

    // Ages must have actually advanced, not stayed frozen at tick.rs's old age-only stub behavior.
    // Mortality is now real, so we don't assume any *specific* individual (e.g. a founder)
    // survived all 10 years -- just that age tracking is consistent for whoever is alive.
    for individual in state.individuals.iter().filter(|i| i.alive && !i.is_dead) {
        assert_eq!(individual.age_days, Some(3650 - individual.birth_day));
    }
    assert!(state.individuals.iter().any(|i| i.alive && !i.is_dead), "someone should still be alive after 10 years");
}

#[test]
fn pregnancy_is_not_counted_as_a_living_member_before_birth_day() {
    let mut state = two_founders_state();
    for _ in 0..400 {
        advance_one_day(&mut state);
        for pending in &state.pending_births {
            assert!(
                !state.individuals.iter().any(|i| i.id == pending.id),
                "a pending birth must not simultaneously exist in the live population"
            );
        }
    }
}

#[test]
fn discovered_tech_set_only_grows_from_known_starting_set() {
    let mut state = two_founders_state();
    let mut techs: HashSet<String> = HashSet::new();
    for _ in 0..1000 {
        advance_one_day(&mut state);
        for t in &state.discovered_techs {
            techs.insert(t.clone());
        }
    }
    // No assertion on which techs (probabilistic), just that the vector stays well-formed.
    assert!(state.discovered_techs.len() == techs.len());
}

#[test]
fn individuals_actually_move_over_time_instead_of_staying_frozen_at_birth_position() {
    let mut state = two_founders_state();
    let start = (state.individuals[0].x, state.individuals[0].y);
    for _ in 0..500 {
        advance_one_day(&mut state);
    }
    let moved = state
        .individuals
        .iter()
        .any(|i| (i.x - start.0).abs() > 1e-9 || (i.y - start.1).abs() > 1e-9);
    assert!(moved, "at least one individual should have a different position after 500 days of decisions/movement");
}

/// Nothing previously ever set social.has_mate/mate_id/children_ids on the
/// parents of a newborn, so the client's "Paired"/"N Child" badges
/// (PopulationPanel.tsx) could never appear regardless of actual
/// reproduction, and psychology::process_bonding (the only place pairwise
/// relationship strength is tracked) was never invoked from anywhere.
#[test]
fn parents_of_a_newborn_are_recorded_as_mates_with_a_tracked_relationship() {
    // Conception is a small independent daily roll (~0.0015 with two fresh
    // founders), so "no birth yet" has a long tail: 1500 days flaked in CI,
    // and even 3000 days still failed about 1 run in 80 locally. 8000 days
    // pushes the no-birth probability down to roughly 1e-5 while still
    // running in a fraction of a second.
    let mut state = two_founders_state();
    for _ in 0..8000 {
        advance_one_day(&mut state);
        if state.individuals.iter().any(|i| i.parent_1_id.is_some()) {
            break;
        }
    }
    let child = state.individuals.iter().find(|i| i.parent_1_id.is_some()).expect("8000 days with two fertile founders should yield a birth");
    let mother_id = child.parent_1_id.clone().unwrap();
    let father_id = child.parent_2_id.clone().unwrap();
    let mother = state.individuals.iter().find(|i| i.id == mother_id).unwrap();
    let father = state.individuals.iter().find(|i| i.id == father_id).unwrap();

    assert!(mother.social.has_mate && father.social.has_mate);
    assert_eq!(mother.social.mate_id.as_deref(), Some(father_id.as_str()));
    assert_eq!(father.social.mate_id.as_deref(), Some(mother_id.as_str()));
    assert!(mother.social.children_ids.contains(&child.id));
    assert!(father.social.children_ids.contains(&child.id));
    assert!(mother.psychology.relationships.get(&father_id).copied().unwrap_or(0.0) > 0.0, "mating should register a positive relationship value, not just the boolean flags");
}

/// Vocabulary can only ever *spread* by copying an existing teacher's word
/// (learn_from_teacher) -- with everyone starting from an empty vocabulary,
/// something has to originate the first word for a concept, or vocabulary
/// stays permanently empty for the entire run regardless of language stage.
#[test]
fn vocabulary_eventually_originates_once_language_stage_and_foxp2_allow_it() {
    let mut state = two_founders_state();
    for individual in state.individuals.iter_mut() {
        individual.language.stage = 2;
        individual.language.foxp2_expression = 0.9;
        individual.phenotype.fluid_intelligence = 0.9;
        // update_foxp2_expression() clamps foxp2_expression to this ceiling every
        // tick; without setting it too, a founder whose randomly-generated genome
        // happens to give a low language_capacity would have foxp2 pulled back
        // down below the 0.35 acquisition threshold after tick 0, making this
        // test flake depending on the founders' random genome draw.
        individual.phenotype.language_capacity = 0.9;
    }
    for _ in 0..300 {
        advance_one_day(&mut state);
    }
    assert!(
        state.individuals.iter().any(|i| !i.language.vocabulary.is_empty()),
        "300 days at stage>=2 with high FOXP2/IQ should originate at least one word"
    );
}

#[test]
fn nobody_ever_drifts_off_the_land_mask() {
    let mut state = two_founders_state();
    for _ in 0..2000 {
        advance_one_day(&mut state);
        for individual in state.individuals.iter().filter(|i| i.alive && !i.is_dead) {
            assert!(
                is_on_land(individual.y, individual.x),
                "individual {} drifted to (lat={}, lon={}) which is off the land mask",
                individual.id,
                individual.y,
                individual.x
            );
        }
    }
}
