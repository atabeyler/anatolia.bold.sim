#![recursion_limit = "256"]

pub mod biology;
pub mod consciousness;
pub mod consciousness_sensitivity;
pub mod environment;
pub mod epigenetics;
pub mod hormones;
pub mod language;
pub mod technology;
pub mod economy;
pub mod social;
pub mod culture;
pub mod law;
pub mod astronomy;
pub mod microbiome;
pub mod art;
pub mod architecture;
pub mod belief;
pub mod agent;
pub mod interventions;
pub mod client_view;
pub mod psychology;
pub mod milestones;
mod naming;
pub mod spatial;
mod state;
mod tick;
pub mod types;

pub use biology::genome::{
    coefficient_of_relationship, combine_gametes, compute_genetic_diversity, compute_genetic_diversity_by_group, compute_inbreeding_coefficient, compute_phenotype, create_gamete,
    create_genome, GenealogyEntry, GenealogyIndex,
};
pub use biology::individual::{create_child, create_founder, create_founder_for_simulation, get_age, get_life_stage, is_fertile, migrate_individual_arrival, FOUNDER_GENOME_DEFAULTS};
pub use biology::mortality::{compute_daily_death_risk, roll_death, DeathCause};
pub use biology::reproduction::check_reproduction;
pub use consciousness::{update_consciousness, update_inner_thought};
pub use environment::{compute_resource_pressure, create_world_state, get_biome, is_on_land, update_world_state};
pub use epigenetics::{compute_epigenetic_age, inherit_epigenome, initialize_epigenome, update_epigenome};
pub use hormones::{apply_birth_surge, apply_mating_surge, compute_population_hormone_stats, initialize_hormones, update_hormones};
pub use language::{
    derive_phoneme_palette, derive_phoneme_palette_from_population, generate_proto_word, get_language_summary, get_vocabulary_by_group, learn_from_teacher, read_written_records,
    record_event_for_posterity, try_acquire_word_from_environment, update_foxp2_expression, update_language_stage, CORE_CONCEPTS, LANGUAGE_STAGES,
};
pub use technology::{known_techs_json, learn_tech_from_observation, TECH_TREE};
pub use economy::{attempt_trade, compute_economic_stats, consume_resources, gather_resources, initialize_inventory, produce_goods, GOODS_TYPES, RESOURCE_TYPES};
pub use social::{assign_group_roles, compute_social_status, process_group_dynamics, GROUP_ROLES, RELATIONSHIP_TYPES};
pub use culture::{compute_cultural_prestige, process_culture_tick, CULTURAL_MEMES};
pub use law::{check_norm_violation, compute_social_order, process_law_tick, process_norm_enforcement, NORM_TYPES};
pub use astronomy::{get_astronomy_bonus, process_astronomy_tick, ASTRONOMY_KNOWLEDGE};
pub use microbiome::{compute_health_stats, process_microbiome_tick, spread_infection, update_gut_microbiome, PATHOGEN_TYPES};
pub use art::{apply_art_effects, process_art_tick, ART_FORMS};
pub use architecture::{check_settlement_overcrowding, compute_settlement_capacity, compute_settlement_defense, create_settlement, process_architecture_tick, STRUCTURE_TYPES};
pub use belief::{check_ritual_emergence, try_form_belief, update_belief_spread, BELIEF_ARCHETYPES};
pub use agent::{select_action, ACTIONS};
pub use interventions::apply_intervention;
pub use client_view::{
    age_years, build_event_description, derive_stats, events_summary, extinction_reason, find_individual, individual_display_name, mark_extinct, new_simulation, pascal_to_snake, population_view,
    serialize_individual, terminate, to_client_event, TERMINATION_DISASTER_CAUSE,
};
pub use psychology::{compute_population_psych_stats, initialize_psychology, process_bonding, update_mental_state};
pub use milestones::check_milestones;
pub use state::{Individual, PhaseTimings, SimulationState, TickReport, WorldState, TOGGLEABLE_ENGINES};
pub use types::{Allele, Epigenome, EpigeneticLocus, Genome, Health, Hormones, Language, Locus, Mind, Phenotype, PhonemePalette, Psychology, Social, Volatile};
pub use tick::{advance_one_day, DEAD_FIELD_STRIP_GRACE_DAYS};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_day_and_updates_alive_ages() {
        let mut state = SimulationState {
            current_day: 10,
            world_state: WorldState::default(),
            individuals: vec![
                Individual {
                    id: "a".to_string(),
                    birth_day: 0,
                    alive: true,
                    // Individual::default() zeroes health (derive(Default) doesn't see the
                    // #[serde(default = "one")] hints used for JSON deserialization), which
                    // made this individual an occasional starvation/dehydration death within
                    // a single tick and flaked this test. Give it real health explicitly.
                    health: crate::types::Health { hp: 1.0, calories: 1.0, hydration: 1.0, ..Default::default() },
                    ..Individual::default()
                },
                Individual {
                    id: "b".to_string(),
                    birth_day: 3,
                    alive: false,
                    ..Individual::default()
                },
            ],
            ..SimulationState::default()
        };

        let (report, _phases) = advance_one_day(&mut state);

        assert_eq!(state.current_day, 11);
        assert_eq!(state.individuals[0].age_days, Some(11));
        assert_eq!(state.individuals[1].age_days, None);
        assert_eq!(report.current_day, 11);
        // Not `assert_eq!(report.alive_count, 1)`: mortality::compute_daily_death_risk
        // applies a small unconditional per-tick background risk (~0.022% for an
        // infant) on top of the starvation/dehydration/disease checks already
        // neutralized above via explicit health -- there's no individual state that
        // brings it to exactly zero, so "a" has a tiny but real chance of dying on
        // this same tick. Assert alive_count against the actual post-tick state
        // instead of a hardcoded "always survives" constant, so this test still
        // catches a real counting regression without flaking on that residual risk.
        let expected_alive = state.individuals.iter().filter(|i| i.alive && !i.is_dead).count();
        assert_eq!(report.alive_count, expected_alive);
        assert_eq!(report.updated_age_count, 1);
    }

    #[test]
    fn preserves_unknown_fields_on_roundtrip() {
        let json = r#"
        {
          "current_day": 42,
          "world_state": {
            "biome": "temperate_forest",
            "season": "spring"
          },
          "individuals": [
            {
              "id": "abc",
              "birth_day": -730,
              "alive": true,
              "custom_note": "kept"
            }
          ],
          "unknown_flag": true
        }
        "#;

        let state: SimulationState = serde_json::from_str(json).expect("state should parse");
        assert_eq!(state.current_day, 42);
        assert_eq!(state.world_state.biome.as_deref(), Some("temperate_forest"));
        assert_eq!(state.individuals[0].extra.get("custom_note").and_then(|v| v.as_str()), Some("kept"));
        assert_eq!(state.extra.get("unknown_flag").and_then(|v| v.as_bool()), Some(true));

        let encoded = serde_json::to_string(&state).expect("state should serialize");
        let decoded: SimulationState = serde_json::from_str(&encoded).expect("state should deserialize");
        assert_eq!(decoded.current_day, 42);
        assert_eq!(decoded.individuals.len(), 1);
    }

    #[test]
    fn creates_basic_founder_and_child() {
        let founder = create_founder(&serde_json::json!({
            "sex": "female",
            "ageYears": 22,
            "x": 1.0,
            "y": 2.0,
            "name": "Ada"
        }));
        assert!(founder.is_founder);
        assert_eq!(founder.generation, Some(0));
        assert_eq!(founder.phenotype.name.as_deref(), Some("Ada"));

        let child = create_child(&founder, &founder, 0, "sim-1");
        assert_eq!(child.generation, Some(1));
        assert_eq!(child.simulation_id.as_deref(), Some("sim-1"));
    }
}
