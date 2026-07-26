//! Read-only projections of `SimulationState` into the shapes a client
//! (native sim-server's HTTP/WS layer, or a WASM build with no server at
//! all) actually renders: per-tick aggregate stats, a single individual's
//! display record, and plain-English event descriptions. Factored out of
//! sim-server so there is exactly one implementation of "what does this
//! state look like to a player" shared by every caller instead of two
//! copies that could quietly drift apart.
use std::collections::HashMap;

use serde_json::{json, Value};

use crate::state::{Individual, SimulationState, WorldState};
use crate::{
    compute_cultural_prestige, compute_economic_stats, compute_genetic_diversity, compute_health_stats, compute_population_psych_stats, create_founder_for_simulation, create_world_state,
    derive_phoneme_palette_from_population, get_language_summary, known_techs_json,
};

/// Builds a brand-new two-founder `SimulationState`, ready to persist and
/// tick -- shared by sim-server's `POST /simulations` handler (which then
/// stamps `user_id`/an owner onto the result) and sim-wasm's own
/// `create_simulation` export (which has no owner concept at all, matching
/// how a no-account WASM-local trial never has one). `founder_1_params`/
/// `founder_2_params` are the raw wizard payload (may be `{}`); genome/name/
/// appearance overrides inside them are honored, everything else (sex,
/// starting position, age, founder genome defaults) is filled in the same
/// way for every simulation.
pub fn new_simulation(name: Option<String>, latitude: f64, longitude: f64, founder_1_params: &Value, founder_2_params: &Value) -> SimulationState {
    let simulation_id = uuid::Uuid::new_v4().to_string();
    let world_state: WorldState = serde_json::from_value(create_world_state(latitude, longitude)).unwrap_or_default();
    let mut sim = SimulationState {
        id: Some(simulation_id.clone()),
        name: Some(name.unwrap_or_else(|| "Untitled Simulation".to_string())),
        start_latitude: Some(latitude),
        start_longitude: Some(longitude),
        current_day: 0,
        current_year: 0,
        status: Some("paused".to_string()),
        speed_multiplier: Some(1),
        world_state,
        individuals: vec![],
        founder_1: Some(founder_1_params.clone()),
        founder_2: Some(founder_2_params.clone()),
        // Matches SimulationEngine's constructor: every simulation starts with
        // these two techs already known, rather than requiring rediscovery.
        discovered_techs: vec!["foraging".to_string(), "stone_tools".to_string()],
        ..Default::default()
    };

    // create_founder_for_simulation takes (x, y) where x=longitude, y=latitude
    // (matches environment::is_on_land(lat, lon) callers and the client's
    // WorldGlobe, which reads ind.y as latitude and ind.x as longitude).
    let founder_1 = create_founder_for_simulation(founder_1_params, "male", longitude, latitude, 22, true, &simulation_id, None, None);
    let founder_2 = create_founder_for_simulation(founder_2_params, "female", longitude + 0.1, latitude, 20, true, &simulation_id, None, None);
    sim.individuals = vec![founder_1, founder_2];
    sim.total_ever_born = sim.individuals.len() as i32;
    // This civilization's own sound repertoire, derived from these two
    // founders' FOXP2/CNTNAP2 alleles -- see derive_phoneme_palette.
    sim.world_state.phoneme_palette = Some(derive_phoneme_palette_from_population(&sim.individuals));
    sim
}

/// A synthetic "disaster_type" used only by `terminate_simulation`'s
/// deliberate 100%-mortality ending, never rolled naturally by
/// `environment::process_disaster`'s own random disaster selection.
pub const TERMINATION_DISASTER_CAUSE: &str = "meteor_tsunami";

const EPI_LOCI: &[&str] = &["HPA_AXIS", "BDNF_PROMOTER", "MAOA_REGULATION", "LEPTIN_RESIST", "INSULIN_SENS", "AVP_REGULATION", "OXTR_METHYL", "IMMUNE_PRIMING"];
// (client key, Phenotype field extractor) -- must match BiologyPanel.tsx's
// GENE_LABELS keys exactly (both the short gene-symbol keys like "FOXP2"
// and the direct trait-name keys like "fluid_intelligence").
#[allow(clippy::type_complexity)]
const ALLELE_FREQ_TRAITS: &[(&str, fn(&crate::types::Phenotype) -> f64)] = &[
    ("FOXP2", |p| p.language_capacity),
    ("OXTR", |p| p.social_bonding),
    ("DRD4", |p| p.curiosity),
    ("MAOA", |p| p.aggression),
    ("BDNF", |p| p.learning_rate),
    ("fluid_intelligence", |p| p.fluid_intelligence),
    ("physical_strength", |p| p.physical_strength),
    ("empathy", |p| p.empathy),
    ("curiosity", |p| p.curiosity),
    ("conscientiousness", |p| p.conscientiousness),
    ("aggression", |p| p.aggression),
    ("immune_strength", |p| p.immune_strength),
    ("artistic_sense", |p| p.artistic_sense),
];
const AGE_PYRAMID_BANDS: &[&str] = &["0-4", "5-9", "10-14", "15-19", "20-24", "25-29", "30-34", "35-39", "40-44", "45-49", "50-54", "55-59", "60-64", "65+"];

pub fn age_years(ind: &Individual, current_day: i32) -> f64 {
    let age_day = if ind.is_dead { ind.death_day.unwrap_or(current_day) } else { current_day };
    ((age_day - ind.birth_day).max(0) as f64) / 365.0
}

pub fn life_stage(age_years: f64) -> &'static str {
    if age_years < 2.0 {
        "infant"
    } else if age_years < 12.0 {
        "child"
    } else if age_years < 18.0 {
        "adolescent"
    } else if age_years < 45.0 {
        "adult"
    } else {
        "elder"
    }
}

/// Per-tick aggregate stats -- the same payload the WS `tick` broadcast,
/// `GET /:id/stats`, and the report's `current_stats` all share.
pub fn derive_stats(sim: &SimulationState) -> Value {
    let alive: Vec<&Individual> = sim.individuals.iter().filter(|i| i.alive && !i.is_dead).collect();
    let alive_count = alive.len();
    let total = sim.individuals.len();
    let max_n = alive_count.max(1) as f64;
    let avg_age = if total > 0 {
        let sum: f64 = sim.individuals.iter().map(|ind| age_years(ind, sim.current_day)).sum();
        Some((sum / total as f64 * 10.0).round() / 10.0)
    } else {
        None
    };
    let max_language_stage = sim.individuals.iter().map(|ind| ind.language.stage).max().unwrap_or(0);

    let avg_consciousness = alive.iter().map(|i| i.mind.consciousness).sum::<f64>() / max_n;
    let avg_lang_stage = alive.iter().map(|i| i.language.stage as f64).sum::<f64>() / max_n;
    let avg_health = alive.iter().map(|i| i.health.hp).sum::<f64>() / max_n;
    let avg_wellbeing = alive.iter().map(|i| i.psychology.wellbeing).sum::<f64>() / max_n;
    let qol_index = avg_consciousness * 0.3 + (avg_lang_stage / 6.0) * 0.2 + avg_health * 0.3 + avg_wellbeing * 0.2;

    let econ_stats = compute_economic_stats(&alive);
    let gini = econ_stats.get("gini").and_then(Value::as_f64).unwrap_or(0.0);
    let psych_stats = compute_population_psych_stats(&sim.individuals, gini);
    let happiness_index = psych_stats.get("happiness_index").and_then(Value::as_f64).unwrap_or(0.5);
    let mean_stress = psych_stats.get("mean_stress").and_then(Value::as_f64).unwrap_or(0.0);

    let mut mental_state_distribution: HashMap<String, i64> = HashMap::new();
    for ind in &alive {
        let state = if ind.psychology.mental_state.is_empty() { "calm".to_string() } else { ind.psychology.mental_state.clone() };
        *mental_state_distribution.entry(state).or_insert(0) += 1;
    }
    let mental_state_distribution: Value = mental_state_distribution.into_iter().map(|(k, v)| (k, json!(v))).collect::<serde_json::Map<String, Value>>().into();

    let epigenetics: Value = EPI_LOCI
        .iter()
        .map(|locus| {
            let avg = alive.iter().map(|i| i.epigenome.get(*locus).map(|l| l.methylation).unwrap_or(0.5)).sum::<f64>() / max_n;
            (locus.to_string(), json!((avg * 100.0).round() / 100.0))
        })
        .collect::<serde_json::Map<String, Value>>()
        .into();

    let mut pyramid_male = vec![0_i64; AGE_PYRAMID_BANDS.len()];
    let mut pyramid_female = vec![0_i64; AGE_PYRAMID_BANDS.len()];
    for ind in &alive {
        let age = age_years(ind, sim.current_day);
        let band = if age >= 65.0 { AGE_PYRAMID_BANDS.len() - 1 } else { (age / 5.0).floor() as usize };
        if ind.sex == "male" {
            pyramid_male[band] += 1;
        } else {
            pyramid_female[band] += 1;
        }
    }
    let age_pyramid: Vec<Value> = AGE_PYRAMID_BANDS
        .iter()
        .enumerate()
        .map(|(idx, band)| json!({ "group": band, "male": pyramid_male[idx], "female": pyramid_female[idx] }))
        .collect();

    let births = sim.individuals.iter().filter(|i| !i.is_founder).count();
    let deaths = sim.individuals.iter().filter(|i| i.is_dead).count();
    let avg_intelligence = alive.iter().map(|i| i.phenotype.fluid_intelligence).sum::<f64>() / max_n;
    let sick_count = alive.iter().filter(|i| i.health.disease.is_some()).count();
    let sick_rate = sick_count as f64 / max_n;
    let male_count = alive.iter().filter(|i| i.sex == "male").count();
    let sex_ratio = male_count as f64 / max_n;
    let avg_cultural_prestige = if sim.groups.is_empty() { 0.0 } else { sim.groups.iter().map(compute_cultural_prestige).sum::<f64>() / sim.groups.len() as f64 };
    let health_stats = compute_health_stats(&sim.individuals);
    let language_stage_distribution = get_language_summary(&sim.individuals);
    let genetic_diversity = compute_genetic_diversity(&alive);
    let allele_frequencies: Value = ALLELE_FREQ_TRAITS
        .iter()
        .map(|(key, extract)| {
            let avg = alive.iter().map(|i| extract(&i.phenotype)).sum::<f64>() / max_n;
            (key.to_string(), json!((avg * 1000.0).round() / 1000.0))
        })
        .collect::<serde_json::Map<String, Value>>()
        .into();

    let (centroid_x, centroid_y) = if alive_count > 0 {
        (Some(alive.iter().map(|i| i.x).sum::<f64>() / max_n), Some(alive.iter().map(|i| i.y).sum::<f64>() / max_n))
    } else {
        (None, None)
    };
    // Distinct words actually in use across the living population's own
    // `language.vocabulary` (concept -> word), not `sim.events.len()` --
    // the event log length was previously exposed under this same
    // "word_count" key and consumed by LanguagePanel/HypothesisPanel/
    // ReportPanel as if it were a linguistic measurement (including feeding
    // it into the LLM-backed hypothesis-testing prompt as ground truth),
    // which had no relationship to vocabulary size at all.
    let vocabulary_size = {
        let mut words: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for ind in &alive {
            for word in ind.language.vocabulary.values() {
                words.insert(word.as_str());
            }
        }
        words.len()
    };
    let dominant_drive = {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for ind in &alive {
            if let Some(action) = ind.extra.get("_currentAction").and_then(Value::as_str) {
                *counts.entry(action).or_insert(0) += 1;
            }
        }
        counts.into_iter().max_by_key(|(_, n)| *n).map(|(action, _)| action)
    };

    json!({
        "day": sim.current_day,
        "year": sim.current_year,
        "population": alive_count,
        "total_population": total,
        "avg_age": avg_age,
        "max_language_stage": max_language_stage,
        "word_count": vocabulary_size,
        "events_count": sim.events.len(),
        "happiness_index": (happiness_index * 1000.0).round() / 1000.0,
        "gini": (gini * 1000.0).round() / 1000.0,
        "mean_stress": (mean_stress * 1000.0).round() / 1000.0,
        "mental_state_distribution": mental_state_distribution,
        "qol_index": (qol_index * 1000.0).round() / 1000.0,
        "avg_consciousness": (avg_consciousness * 1000.0).round() / 1000.0,
        "births": births,
        "deaths": deaths,
        "epigenetics": epigenetics,
        "age_pyramid": age_pyramid,
        "season": sim.world_state.season,
        "weather": sim.world_state.extra.get("current_weather").cloned().unwrap_or_else(|| json!("clear")),
        "technologies": sim.discovered_techs.len(),
        "beliefs": sim.discovered_beliefs.len(),
        "groups": sim.groups.len(),
        "civilization_name": sim.civilization_name,
        "avg_cultural_prestige": (avg_cultural_prestige * 1000.0).round() / 1000.0,
        "temperature": sim.world_state.temperature,
        "food_abundance": sim.world_state.food_abundance,
        "water_abundance": sim.world_state.water_abundance,
        "avg_intelligence": (avg_intelligence * 1000.0).round() / 1000.0,
        "sick_rate": (sick_rate * 1000.0).round() / 1000.0,
        "sex_ratio": (sex_ratio * 1000.0).round() / 1000.0,
        "speed_multiplier": sim.speed_multiplier.unwrap_or(1),
        "pathogen_diversity": health_stats.get("pathogen_diversity").cloned().unwrap_or_else(|| json!(0)),
        "language_stage_distribution": language_stage_distribution,
        "centroid_x": centroid_x,
        "centroid_y": centroid_y,
        "dominant_drive": dominant_drive,
        "genetic_diversity": genetic_diversity,
        "allele_frequencies": allele_frequencies,
    })
}

/// A single individual's display record -- the per-row shape of
/// `GET /:id/population`/`/:id/population/:individualId` and the `report`'s
/// `individuals` array.
pub fn serialize_individual(ind: &Individual, current_day: i32) -> Value {
    let age = age_years(ind, current_day);
    let name = ind.phenotype.name.clone().or_else(|| ind.extra.get("name").and_then(|v| v.as_str()).map(str::to_string));
    json!({
        "id": ind.id,
        "name": name,
        "sex": ind.sex,
        "birth_day": ind.birth_day,
        "death_day": ind.death_day,
        "alive": ind.alive && !ind.is_dead,
        "age_years": (age * 10.0).round() / 10.0,
        "x": ind.x,
        "y": ind.y,
        "parent_1_id": ind.parent_1_id,
        "parent_2_id": ind.parent_2_id,
        "death_cause": ind.extra.get("death_cause").cloned().unwrap_or(Value::Null),
        "genome": ind.genome,
        "phenotype": ind.phenotype,
        "epigenome": ind.epigenome,
        "health": ind.health,
        "mind": ind.mind,
        "psychology": ind.psychology,
        "social": ind.social,
        "skills": ind.skills,
        // Not the raw archetype ids (see belief.rs) -- this is polled
        // frequently and deliberately avoids requiring belief_labels just to
        // label these, so a count is what's safe to send here.
        "beliefs_count": ind.beliefs.len(),
        "language": ind.language,
        "memory": ind.memory,
        "inventory": ind.inventory,
        "inbreeding_coeff": ind.inbreeding_coeff.unwrap_or(0.0),
        "is_founder": ind.is_founder,
        "life_stage": life_stage(age),
        "known_techs": known_techs_json(ind),
    })
}

const MAX_STORED_EVENTS: usize = 1000;

/// Ends a simulation deliberately via the same mass-mortality path every
/// organic disaster already uses (a real disaster at full mortality means
/// literally everyone -- see `environment::process_disaster`'s own doc
/// comment) rather than deleting the historical record. Individuals, events,
/// and the simulation itself all stay intact; only status changes.
pub fn terminate(sim: &mut SimulationState) {
    let new_events = crate::environment::process_disaster(TERMINATION_DISASTER_CAUSE, 1.0, &mut sim.individuals, &mut sim.groups, sim.current_day);
    sim.events.extend(new_events);
    if sim.events.len() > MAX_STORED_EVENTS {
        let excess = sim.events.len() - MAX_STORED_EVENTS;
        sim.events.drain(0..excess);
    }
    sim.status = Some("completed".to_string());
}

/// Whether `individuals` can no longer meaningfully continue: nobody left
/// alive, only one person left, or the survivors are all one sex (no path to
/// reproduction). Read-only -- callers decide what, if anything, to do about
/// it. Shared so sim-server's WS layer (which only ever *reports* this to a
/// connected client) and its tick loop (which needs to actually stop
/// wasting compute/DB writes on a population that's never coming back) can't
/// drift into disagreeing about what "extinct" means.
pub fn extinction_reason(individuals: &[Individual]) -> Option<&'static str> {
    let alive: Vec<&Individual> = individuals.iter().filter(|i| i.alive && !i.is_dead).collect();
    if alive.is_empty() {
        return Some("population_zero");
    }
    if alive.len() == 1 {
        return Some("single_individual");
    }
    if alive.iter().all(|i| i.sex == "male") {
        return Some("no_females");
    }
    if alive.iter().all(|i| i.sex == "female") {
        return Some("no_males");
    }
    None
}

/// Ends a simulation whose population died out on its own -- unlike
/// `terminate`, this never injects a synthetic disaster (there's nothing
/// left to kill, and whatever actually happened is already the real, final
/// entries in `sim.events`); it only records the moment civilization ended
/// and flips `status` the same way `terminate` does, so the tick loop that
/// detects this can stop for good instead of ticking a dead population
/// forever.
pub fn mark_extinct(sim: &mut SimulationState, reason: &str) {
    sim.events.push(json!({
        "type": "extinction",
        "day": sim.current_day,
        "reason": reason,
        "importance": "high",
        "description": "The civilization has come to an end",
    }));
    if sim.events.len() > MAX_STORED_EVENTS {
        let excess = sim.events.len() - MAX_STORED_EVENTS;
        sim.events.drain(0..excess);
    }
    sim.status = Some("completed".to_string());
}

/// Individuals list view (population endpoint's in-memory-live-state branch,
/// and any client-only caller with the same needs): optionally filtered to
/// only-alive/only-dead, ordered by birth (closest in-memory proxy for
/// insertion order), optionally capped at `limit`.
pub fn population_view(sim: &SimulationState, alive: Option<bool>, limit: Option<usize>) -> Vec<Value> {
    let mut individuals: Vec<&Individual> = sim.individuals.iter().collect();
    if let Some(alive) = alive {
        individuals.retain(|ind| if alive { ind.alive && !ind.is_dead } else { !ind.alive || ind.is_dead });
    }
    individuals.sort_by_key(|ind| ind.birth_day);
    if let Some(limit) = limit {
        individuals.truncate(limit);
    }
    individuals.into_iter().map(|ind| serialize_individual(ind, sim.current_day)).collect()
}

/// `GET /:id/events/summary`'s payload: total event count, a per-type
/// breakdown, and how many individuals the engine itself has marked
/// dead/not-alive (a cheap "how much has actually happened" sanity figure
/// distinct from the event log's own size).
pub fn events_summary(sim: &SimulationState) -> Value {
    let mut counts = std::collections::BTreeMap::<String, i64>::new();
    for event in &sim.events {
        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
        *counts.entry(event_type.to_string()).or_insert(0) += 1;
    }
    let deaths = sim.individuals.iter().filter(|ind| ind.is_dead || !ind.alive).count() as i64;
    json!({
        "total": sim.events.len(),
        "countsByType": counts,
        "engineDeaths": deaths,
    })
}

pub fn individual_display_name(ind: &Individual) -> String {
    ind.phenotype.name.clone().or_else(|| ind.extra.get("name").and_then(Value::as_str).map(str::to_string)).unwrap_or_else(|| "Unnamed".to_string())
}

pub fn find_individual<'a>(sim: &'a SimulationState, id: &str) -> Option<&'a Individual> {
    sim.individuals.iter().find(|i| i.id == id)
}

/// Rust's `{cause:?}` Debug formatting of `DeathCause` yields PascalCase
/// ("OldAge", "Drowning"); some death events instead set an already-lowercase
/// literal cause ("infection"). Normalizes either to the snake_case keys the
/// client's `CAUSE_TR`/`CAUSE_DE`/... translation tables are keyed by.
pub fn pascal_to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Builds a plain-English description for events that need per-instance data
/// (a name, a cause, a death toll, ...) plugged into a template. Events whose
/// originating module already attaches a self-contained `description` (art
/// forms, norms, cultural memes, celestial/astronomy knowledge, settlements,
/// milestones) are passed straight through. Belief events are the one
/// exception among that group: they carry no description of their own (see
/// `belief.rs`) because only this layer has `sim` to resolve `belief_labels`,
/// which is never a real-world religion name.
///
/// These exact phrasings are matched by `translateEventDescription` in the
/// client's `i18n.ts`, which is what actually localizes them for tr/de/fr/ar.
pub fn build_event_description(event_type: &str, raw: &Value, sim: &SimulationState) -> String {
    match event_type {
        "birth" => {
            let child_id = raw.get("individual_id").and_then(Value::as_str).unwrap_or("");
            match find_individual(sim, child_id) {
                Some(child) => {
                    let name = individual_display_name(child);
                    let parents = child
                        .parent_1_id
                        .as_deref()
                        .and_then(|id| find_individual(sim, id))
                        .zip(child.parent_2_id.as_deref().and_then(|id| find_individual(sim, id)));
                    match parents {
                        Some((p1, p2)) => format!("Born: {name} ({} & {})", individual_display_name(p1), individual_display_name(p2)),
                        None => format!("Born: {name}"),
                    }
                }
                None => "New individual born".to_string(),
            }
        }
        "death" => {
            let id = raw.get("individual_id").and_then(Value::as_str).unwrap_or("");
            let name = find_individual(sim, id).map(individual_display_name).unwrap_or_else(|| "Individual".to_string());
            let cause = raw.get("cause").and_then(Value::as_str).map(pascal_to_snake).unwrap_or_else(|| "unknown".to_string());
            format!("{name} died: {cause}")
        }
        "discovery" => {
            let tech = raw.get("tech_id").and_then(Value::as_str).unwrap_or("unknown");
            format!("Technology discovered: {tech}")
        }
        "epidemic_outbreak" => {
            let pathogen = raw.get("pathogen_id").and_then(Value::as_str).unwrap_or("unknown");
            format!("A {pathogen} outbreak begins")
        }
        "disaster" => {
            let disaster_type = raw.get("disaster_type").and_then(Value::as_str).unwrap_or("disaster");
            let deaths = raw.get("deaths").and_then(Value::as_i64).unwrap_or(0);
            // capitalize() only uppercases the first letter, leaving compound
            // identifiers like "meteor_tsunami" (TERMINATION_DISASTER_CAUSE)
            // with a stray underscore in the English base text -- the only
            // disaster label besides the naturally-occurring ones (which are
            // all single words already) that needs this.
            let label = if disaster_type == TERMINATION_DISASTER_CAUSE { "Meteor impact and tsunami".to_string() } else { capitalize(disaster_type) };
            format!("{label} killed {deaths} individuals")
        }
        "ritual_emerged" => {
            // The belief_id itself is never shown verbatim -- only its opaque
            // numeric suffix (never the archetype string) until
            // sim.belief_labels' procedurally generated label (see
            // belief::try_label_belief) exists, once the population's own
            // language has actually coined one.
            let belief_id = raw.get("belief").and_then(Value::as_str).unwrap_or("");
            let code = belief_id.strip_prefix("belief_").unwrap_or(belief_id);
            match sim.belief_labels.get(belief_id) {
                Some(label) => format!("A {label} ritual emerges in the group"),
                None => format!("A ritual (belief #{code}) emerges in the group"),
            }
        }
        "belief_formed" => {
            let founder_id = raw.get("founder_id").and_then(Value::as_str).unwrap_or("");
            let belief_id = raw.get("belief_id").and_then(Value::as_str).unwrap_or("");
            let name = find_individual(sim, founder_id).map(individual_display_name);
            let label = sim.belief_labels.get(belief_id);
            let code = belief_id.strip_prefix("belief_").unwrap_or(belief_id);
            match (name, label) {
                (Some(name), Some(label)) => format!("{name} gave rise to {label}"),
                (Some(name), None) => format!("{name} gave rise to belief #{code}"),
                (None, Some(label)) => format!("A new belief, {label}, takes hold"),
                (None, None) => format!("A new belief (#{code}) takes hold"),
            }
        }
        "belief_named" => {
            let label = raw.get("label").and_then(Value::as_str).unwrap_or("");
            format!("Their belief becomes known as {label}")
        }
        "group_named" => {
            let name = raw.get("name").and_then(Value::as_str).unwrap_or("");
            format!("The group becomes known as {name}")
        }
        "civilization_named" => {
            let name = raw.get("name").and_then(Value::as_str).unwrap_or("");
            format!("Their civilization becomes known as {name}")
        }
        "belief_spread" => {
            let id = raw.get("individual_id").and_then(Value::as_str).unwrap_or("");
            let name = find_individual(sim, id).map(individual_display_name).unwrap_or_else(|| "Someone".to_string());
            let belief_id = raw.get("belief_id").and_then(Value::as_str).unwrap_or("");
            let code = belief_id.strip_prefix("belief_").unwrap_or(belief_id);
            match sim.belief_labels.get(belief_id) {
                Some(label) => format!("{name} embraced {label}"),
                None => format!("{name} embraced belief #{code}"),
            }
        }
        "group_split" => "A group split into two bands".to_string(),
        "conflict" => {
            let casualties = raw.get("casualties").and_then(Value::as_i64).unwrap_or(0);
            format!("A clash between rival groups left {casualties} dead")
        }
        "leadership_change" => {
            let id = raw.get("new_leader_id").and_then(Value::as_str).unwrap_or("");
            let name = find_individual(sim, id).map(individual_display_name).unwrap_or_else(|| "Someone".to_string());
            format!("{name} became the new leader")
        }
        "trade" => {
            let a = raw.get("individual_a").and_then(Value::as_str).unwrap_or("");
            let b = raw.get("individual_b").and_then(Value::as_str).unwrap_or("");
            let name_a = find_individual(sim, a).map(individual_display_name).unwrap_or_else(|| "Someone".to_string());
            let name_b = find_individual(sim, b).map(individual_display_name).unwrap_or_else(|| "someone".to_string());
            format!("{name_a} traded with {name_b}")
        }
        _ => raw.get("description").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| event_type.to_string()),
    }
}

/// The sim engine's internal event objects (rust/sim-core, e.g. `tick.rs`,
/// `environment.rs`, `belief.rs`, ...) all key the event kind as `"type"`.
/// The client (`SimEvent` in `simStore.ts`, `EventsPanel.tsx`,
/// `MilestoneToast.tsx`) reads `event_type`/`sim_day`/`sim_year`/`data`
/// instead. This adapts one raw event to the shape the client expects,
/// without having to rename the field at every one of the ~30 push sites
/// across the engine.
pub fn to_client_event(raw: &Value, sim: &SimulationState) -> Value {
    let event_type = raw.get("type").and_then(Value::as_str).unwrap_or("unknown");
    let day = raw.get("day").and_then(Value::as_i64).unwrap_or(0);
    let importance = raw.get("importance").cloned().unwrap_or_else(|| json!("low"));
    let mut data = serde_json::Map::new();
    if let Some(obj) = raw.as_object() {
        for (k, v) in obj {
            if k != "type" && k != "day" && k != "importance" {
                data.insert(k.clone(), v.clone());
            }
        }
    }
    json!({
        "event_type": event_type,
        "sim_day": day,
        "sim_year": day / 365,
        "importance": importance,
        "description": build_event_description(event_type, raw, sim),
        "data": data,
    })
}

#[cfg(test)]
mod extinction_tests {
    use super::*;

    fn ind(sex: &str, alive: bool, is_dead: bool) -> Individual {
        Individual { sex: sex.to_string(), alive, is_dead, ..Default::default() }
    }

    #[test]
    fn empty_population_is_population_zero() {
        assert_eq!(extinction_reason(&[]), Some("population_zero"));
    }

    #[test]
    fn only_dead_individuals_is_population_zero() {
        let pop = vec![ind("male", false, true), ind("female", false, true)];
        assert_eq!(extinction_reason(&pop), Some("population_zero"));
    }

    #[test]
    fn one_living_individual_is_single_individual() {
        let pop = vec![ind("male", true, false), ind("female", false, true)];
        assert_eq!(extinction_reason(&pop), Some("single_individual"));
    }

    #[test]
    fn only_males_alive_is_no_females() {
        let pop = vec![ind("male", true, false), ind("male", true, false)];
        assert_eq!(extinction_reason(&pop), Some("no_females"));
    }

    #[test]
    fn only_females_alive_is_no_males() {
        let pop = vec![ind("female", true, false), ind("female", true, false)];
        assert_eq!(extinction_reason(&pop), Some("no_males"));
    }

    #[test]
    fn both_sexes_alive_is_not_extinct() {
        let pop = vec![ind("male", true, false), ind("female", true, false)];
        assert_eq!(extinction_reason(&pop), None);
    }

    #[test]
    fn mark_extinct_records_reason_and_completes_without_a_fake_disaster() {
        let mut sim = SimulationState { current_day: 500, status: Some("running".to_string()), ..Default::default() };
        mark_extinct(&mut sim, "population_zero");
        assert_eq!(sim.status.as_deref(), Some("completed"));
        let last = sim.events.last().expect("extinction event recorded");
        assert_eq!(last.get("type").and_then(Value::as_str), Some("extinction"));
        assert_eq!(last.get("reason").and_then(Value::as_str), Some("population_zero"));
        assert_eq!(last.get("day").and_then(Value::as_i64), Some(500));
    }
}

#[cfg(test)]
mod derive_stats_tests {
    use super::*;
    use crate::types::Language;

    fn ind_with_words(words: &[(&str, &str)]) -> Individual {
        let mut language = Language::default();
        for (concept, word) in words {
            language.vocabulary.insert((*concept).to_string(), (*word).to_string());
        }
        Individual { alive: true, is_dead: false, language, ..Default::default() }
    }

    #[test]
    fn word_count_reflects_actual_distinct_vocabulary_not_the_event_log_length() {
        // H-04 regression: "word_count" used to be sim.events.len() -- the
        // event log's length, unrelated to language. LanguagePanel/
        // HypothesisPanel/ReportPanel all surfaced it as a linguistic
        // measurement, so it must actually count distinct words the living
        // population knows.
        let sim = SimulationState {
            individuals: vec![
                ind_with_words(&[("food", "aba"), ("water", "eno")]),
                ind_with_words(&[("food", "aba"), ("danger", "tik")]), // shares "aba" with the first
            ],
            events: vec![json!({"type": "birth"}), json!({"type": "death"}), json!({"type": "birth"}), json!({"type": "birth"}), json!({"type": "death"})],
            ..Default::default()
        };
        let stats = derive_stats(&sim);
        // 3 distinct words across the population: aba, eno, tik (aba is shared, not double-counted).
        assert_eq!(stats.get("word_count").and_then(Value::as_i64), Some(3));
        assert_eq!(stats.get("events_count").and_then(Value::as_i64), Some(5));
    }

    #[test]
    fn word_count_is_zero_for_a_population_with_no_language_yet() {
        let sim = SimulationState { individuals: vec![ind_with_words(&[])], ..Default::default() };
        let stats = derive_stats(&sim);
        assert_eq!(stats.get("word_count").and_then(Value::as_i64), Some(0));
    }
}
