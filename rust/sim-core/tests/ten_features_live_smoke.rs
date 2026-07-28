//! Live-simulation smoke tests for the ten feature extensions added on top
//! of the existing engine: these drive the real `advance_one_day` tick
//! orchestrator (not just the isolated unit functions each engine module
//! already tests) to catch wiring bugs between modules that unit tests
//! can't see.

use sim_core::{advance_one_day, create_founder, create_world_state, SimulationState, WorldState};
use std::collections::HashMap;

fn boosted_founder(sex: &str, name: &str) -> sim_core::Individual {
    // High articulatory/cognitive/social alleles across the board so language,
    // consciousness and social structure have the best realistic chance of
    // progressing within the short window an automated test can afford --
    // mirrors FOUNDER_GENOME_DEFAULTS' own intent, just pushed further.
    let mut genome_overrides = serde_json::Map::new();
    for locus in [
        "FOXP2_01", "CNTNAP2_01", "BDNF_01", "COMT_01", "DTNBP1_01", "NRG1_01", "DISC1_01", "NRXN1_01", "SHANK3_01", "RELN_01", "OXTR_01", "DRD4_01", "DRD2_01",
        "AVPR1A_01", "IMMUNE_01", "IMMUNE_02", "TERT_01", "APOE_01", "FSHR_01", "STRENGTH_01", "METABOLISM_01",
    ] {
        genome_overrides.insert(locus.to_string(), serde_json::json!({ "a1": 0.95, "a2": 0.95 }));
    }
    create_founder(&serde_json::json!({
        "sex": sex, "ageYears": 22, "x": 37.0, "y": 35.0, "name": name,
        "genome": genome_overrides,
    }))
}

fn boosted_two_founders_state() -> SimulationState {
    let world_value = create_world_state(37.0, 35.0);
    let world_state: WorldState = serde_json::from_value(world_value).unwrap();
    let founder_1 = boosted_founder("male", "Adam");
    let founder_2 = boosted_founder("female", "Havva");
    SimulationState {
        id: Some("live-smoke-test".to_string()),
        current_day: 0,
        current_year: 0,
        status: Some("running".to_string()),
        world_state,
        individuals: vec![founder_1, founder_2],
        discovered_techs: vec!["foraging".to_string(), "stone_tools".to_string()],
        ..Default::default()
    }
}

/// #1 (X-linked clotting locus) + #2/#4 (per-group genetic/vocabulary
/// breakdowns) + #3 (seasonal fertility doesn't crash once calendar is
/// known) -- driven through ~16 in-sim years of real ticking, not synthetic
/// state construction.
#[test]
fn long_run_exercises_all_backend_features_without_panicking() {
    let mut state = boosted_two_founders_state();
    let days = 365 * 16;
    let mut saw_calendar = false;

    for day in 0..days {
        // Force calendar knowledge partway through so check_reproduction's
        // seasonal multiplier path (feature #3) actually runs live across
        // every season, not just the neutral pre-calendar branch.
        if day == 365 * 3 && !state.discovered_techs.contains(&"calendar".to_string()) {
            state.discovered_techs.push("calendar".to_string());
        }
        if state.discovered_techs.contains(&"calendar".to_string()) {
            saw_calendar = true;
        }
        let (_report, _phases) = advance_one_day(&mut state);
    }

    assert_eq!(state.current_day, days);
    assert!(saw_calendar, "calendar should have been force-discovered and stayed discovered");
    let alive: Vec<&sim_core::Individual> = state.individuals.iter().filter(|i| i.alive && !i.is_dead).collect();
    assert!(!alive.is_empty(), "someone should still be alive after 16 years with boosted founders");
    assert!(state.individuals.len() > 2, "the population should have grown beyond the two founders");

    // #1: every individual (founder and descendant alike) must carry the
    // X-linked CLOT_01 locus and expose a valid clotting_factor.
    for ind in &alive {
        assert!(ind.genome.contains_key("CLOT_01"), "{} is missing the CLOT_01 locus", ind.id);
        let clotting = ind.phenotype.extra.get("clotting_factor").and_then(|v| v.as_f64());
        assert!(clotting.is_some(), "{} has no clotting_factor in phenotype.extra", ind.id);
        let clotting = clotting.unwrap();
        assert!((0.0..=1.0).contains(&clotting), "{} clotting_factor {clotting} out of range", ind.id);
    }
    // Sons must be hemizygous (single allele) at CLOT_01, daughters diploid --
    // the same X-linkage invariant genome.rs already guarantees for MAOA_01.
    for ind in &alive {
        let locus = &ind.genome["CLOT_01"];
        if ind.sex == "male" {
            assert_eq!(locus.expression_type, "hemizygous", "{} (male) should be hemizygous at CLOT_01", ind.id);
            assert!(locus.allele2.value.is_none());
        } else if ind.sex == "female" {
            // Diploid X-linked loci keep the literal "x_linked" expression_type
            // (see genome.rs::hydrate_genome_metadata) -- pick_value's `_`
            // catch-all still averages both alleles for them, same as MAOA_01.
            assert_eq!(locus.expression_type, "x_linked", "{} (female) should stay diploid/x_linked at CLOT_01", ind.id);
            assert!(locus.allele2.value.is_some());
        }
    }

    // Stats surface (derive_stats is the exact function the HTTP layer calls).
    let stats = sim_core::client_view::derive_stats(&state);
    assert!(stats.get("genetic_diversity_by_group").is_some(), "genetic_diversity_by_group must always be present in stats, even if empty");
    assert!(stats.get("vocabulary_by_group").is_some(), "vocabulary_by_group must always be present in stats, even if empty");

    // #2/#4: if the population actually split into multiple groups (not
    // guaranteed within 16 years, but common with boosted founders), the
    // per-group breakdowns must be non-empty and internally consistent.
    let group_ids: std::collections::HashSet<&str> = alive.iter().filter_map(|i| i.group_id.as_deref()).collect();
    if group_ids.len() > 1 {
        let by_group = stats["genetic_diversity_by_group"].as_object().expect("object");
        assert!(!by_group.is_empty(), "multiple groups exist but genetic_diversity_by_group is empty");
        for gid in &group_ids {
            assert!(by_group.contains_key(*gid), "group {gid} missing from genetic_diversity_by_group");
        }
    }

    // #4: if any group has vocabulary, the comparison map must expose it.
    let any_vocab = alive.iter().any(|i| !i.language.vocabulary.is_empty() && i.group_id.is_some());
    if any_vocab {
        let vocab_by_group = stats["vocabulary_by_group"].as_object().expect("object");
        assert!(!vocab_by_group.is_empty(), "at least one grouped individual has vocabulary but vocabulary_by_group is empty");
    }
}

/// #5 (written records) + #6 (collective trauma epigenetics) + #7 (learned
/// leadership) driven through a handful of *real* ticks on top of a
/// deliberately engineered scenario -- naturally reaching writing (stage 6,
/// needs a 40-member group at generation 25) or a natural disaster inside a
/// short test window isn't realistic, so the scenario is set up directly
/// and then run through the real tick pipeline to catch wiring bugs.
#[test]
fn engineered_scenario_exercises_written_records_collective_trauma_and_leadership_observation() {
    let mut state = boosted_two_founders_state();
    // Give both founders a shared group and writing, so tick.rs's real
    // observation-learning pass (not a synthetic call to language.rs
    // functions) has to carry a record between them.
    state.groups = vec![serde_json::json!({
        "id": "g1", "member_ids": [state.individuals[0].id, state.individuals[1].id],
        "leader_id": state.individuals[0].id, "founded_day": 0, "culture": [], "norms": [],
    })];
    state.individuals[0].group_id = Some("g1".to_string());
    state.individuals[1].group_id = Some("g1".to_string());
    state.individuals[0].language.writing = true;
    state.individuals[0].language.stage = 6;
    state.individuals[1].language.writing = true;
    state.individuals[1].language.stage = 6;
    // Keep them right on top of each other so the spatial-grid observation
    // pass in tick.rs is guaranteed to consider them "nearby".
    state.individuals[1].x = state.individuals[0].x;
    state.individuals[1].y = state.individuals[0].y;

    // #6: seed a collective (non-kin_death) trauma event dated to the very
    // next tick, on founder 2, mirroring what environment::process_disaster
    // + psychology::update_mental_state would have produced organically.
    state.individuals[1].psychology.trauma_events.push(serde_json::json!({ "type": "flood", "day": 1 }));
    let hpa_before = state.individuals[1].epigenome.get("HPA_AXIS").map(|l| l.methylation).unwrap_or(0.5);

    for _ in 0..5 {
        advance_one_day(&mut state);
    }

    // #6: the collective-trauma individual's HPA_AXIS methylation must have
    // moved measurably upward from the neutral baseline within the real tick
    // pipeline (not just the isolated epigenetics::update_epigenome unit).
    let hpa_after = state.individuals.iter().find(|i| i.sex == "female").unwrap().epigenome.get("HPA_AXIS").map(|l| l.methylation).unwrap_or(0.5);
    assert!(hpa_after > hpa_before, "collective trauma should have raised HPA_AXIS methylation within a live tick run, before={hpa_before} after={hpa_after}");

    // #5: at least one of the two literate individuals should have recorded
    // *something* to posterity over 5 real ticks (each day appends the day's
    // most notable event for every writing individual).
    let any_records = state.individuals.iter().any(|i| i.memory.get("written_records").and_then(|v| v.as_array()).map(|a| !a.is_empty()).unwrap_or(false));
    assert!(any_records, "no writing individual accumulated any written_records over 5 real ticks");

    // #7: place a juvenile child of the group's leader and confirm the
    // leadership-observation pass in tick.rs nudges their _behaviorCounts,
    // exactly as social::observe_leadership_style's own unit tests check in
    // isolation -- here it must also survive being wired into a real tick.
    let leader_id = state.individuals[0].id.clone();
    let mut child = boosted_founder("male", "Kayin");
    child.id = "juvenile-1".to_string();
    child.is_founder = false;
    child.parent_1_id = Some(leader_id.clone());
    child.birth_day = state.current_day - 365 * 5; // 5 years old, well under JUVENILE_MAX_AGE_YEARS (13)
    child.age_days = None;
    child.group_id = Some("g1".to_string());
    child.x = state.individuals[0].x;
    child.y = state.individuals[0].y;
    // Give the leader a clearly dominant tracked behavior to observe.
    state.individuals[0].extra.insert("_behaviorCounts".to_string(), serde_json::json!({ "hunt": 50, "forage": 1 }));
    state.groups[0]["member_ids"] = serde_json::json!([state.individuals[0].id, state.individuals[1].id, "juvenile-1"]);
    state.individuals.push(child);

    for _ in 0..3 {
        advance_one_day(&mut state);
    }

    let child_after = state.individuals.iter().find(|i| i.id == "juvenile-1").expect("juvenile still present");
    let hunt_count = child_after.extra.get("_behaviorCounts").and_then(|v| v.get("hunt")).and_then(|v| v.as_i64()).unwrap_or(0);
    assert!(hunt_count > 0, "the leader's child should have picked up an observational bias toward the leader's dominant ('hunt') behavior within 3 real ticks, got _behaviorCounts={:?}", child_after.extra.get("_behaviorCounts"));
}

/// #8/#10 are HTTP-layer features (sim-server's /compare and
/// /migrate-individual) and are covered by sim-server's own integration
/// tests (routes.rs), which exercise them through the real axum router.
/// #9 (auto-generated biography) is a pure client-side (TypeScript)
/// projection of already-tracked fields with no server-side logic to test
/// here; its data dependencies (birth_day, generation, language.stage_name,
/// extra.group_role) are exercised by the tests above.
#[test]
fn placeholder_documents_where_the_remaining_features_are_covered() {
    let _map: HashMap<&str, &str> = HashMap::new();
}
