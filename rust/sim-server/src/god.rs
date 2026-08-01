use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    analysis::lang_name,
    auth::authenticate,
    db::{load_full_state, load_simulation, row_to_state, save_existing_state, upsert_individuals, AppState},
    gemini::{self, GeminiRequest},
};

#[derive(Debug, Deserialize)]
pub struct GodPayload {
    #[serde(rename = "type")]
    pub intervention_type: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub user_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QuarantinePayload {
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TalkPayload {
    pub message: String,
    #[serde(default)]
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MigratePayload {
    pub source_simulation_id: String,
    pub individual_id: String,
}

async fn is_allowed(state: &AppState, headers: &axum::http::HeaderMap, sim_user_id: Option<String>) -> bool {
    let Some(user) = authenticate(state, headers).await else { return false; };
    if user.role == "admin" {
        return true;
    }
    sim_user_id.map(|id| id == user.id).unwrap_or(false)
}

async fn save_snapshot(sim_id: &str, state: &AppState, sim: &serde_json::Value) -> Result<(), sqlx::Error> {
    let mut sim_state: sim_core::SimulationState = serde_json::from_value(sim.clone()).unwrap_or_default();
    sim_state.id = Some(sim_id.to_string());
    save_existing_state(&state.backend, &sim_state).await?;
    upsert_individuals(&state.backend, &sim_state, true).await?;
    Ok(())
}

pub async fn intervene(
    Path(sim_id): Path<String>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<GodPayload>,
) -> impl IntoResponse {
    let sim_state = match load_full_state(&state.backend, &sim_id).await {
        Ok(Some(sim_state)) => sim_state,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "Simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    if !is_allowed(&state, &headers, sim_state.user_id.clone()).await {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Simulation owner required."}))).into_response();
    }

    let mut sim = serde_json::to_value(&sim_state).unwrap_or_else(|_| json!({}));
    let day = sim.get("current_day").and_then(|v| v.as_i64()).unwrap_or(0);
    let year = sim.get("current_year").and_then(|v| v.as_i64()).unwrap_or(day / 365);
    let alive_count = sim
        .get("individuals")
        .and_then(|v| v.as_array())
        .map(|v| v.iter().filter(|i| !i.get("is_dead").and_then(Value::as_bool).unwrap_or(false)).count())
        .unwrap_or(0);

    let (affected, deaths) = match sim_core::apply_intervention(&mut sim, &payload.intervention_type, &payload.params, day, alive_count) {
        Ok(result) => result,
        Err(err) => return (StatusCode::BAD_REQUEST, Json(json!({"error": err}))).into_response(),
    };

    match save_snapshot(&sim_id, &state, &sim).await {
        Ok(()) => {
            if let crate::db::DbBackend::Sqlite(pool) = &state.backend {
                let _ = sqlx::query(
                    "INSERT INTO god_interventions (id, simulation_id, sim_day, sim_year, type, params, affected_individuals, deaths, user_note) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(Uuid::new_v4().to_string())
                .bind(&sim_id)
                .bind(day)
                .bind(year)
                .bind(&payload.intervention_type)
                .bind(payload.params.to_string())
                .bind(affected)
                .bind(deaths)
                .bind(payload.user_note)
                .execute(pool)
                .await;
            }
            Json(json!({
                "message": "Intervention applied.",
                "affected_individuals": affected,
                "deaths": deaths,
            }))
            .into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

/// Cross-simulation migration/gene flow: carries one individual verbatim
/// (genome/phenotype/epigenome/language/skills/beliefs) from another
/// simulation into this one as a new arrival -- an explicit, rare player
/// action, never anything the tick loop triggers on its own. Requires
/// owning (or admin over) *both* simulations, so this can never be used to
/// exfiltrate another user's simulation data. See
/// sim_core::migrate_individual_arrival for exactly what does and doesn't
/// carry over.
pub async fn migrate_individual(
    Path(sim_id): Path<String>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<MigratePayload>,
) -> impl IntoResponse {
    if payload.source_simulation_id == sim_id {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "Source and target simulation must differ."}))).into_response();
    }
    let target_state = match load_full_state(&state.backend, &sim_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "Target simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    if !is_allowed(&state, &headers, target_state.user_id.clone()).await {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Target simulation owner required."}))).into_response();
    }
    let source_state = match load_full_state(&state.backend, &payload.source_simulation_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "Source simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    if !is_allowed(&state, &headers, source_state.user_id.clone()).await {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Source simulation owner required."}))).into_response();
    }
    let Some(source_individual) = source_state.individuals.iter().find(|i| i.id == payload.individual_id && i.alive && !i.is_dead) else {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "Individual not found or not alive"}))).into_response();
    };

    let arrival = sim_core::migrate_individual_arrival(source_individual, source_state.current_day, target_state.current_day);
    let arrival_id = arrival.id.clone();
    let mut target_sim = target_state;
    target_sim.total_ever_born += 1;
    target_sim.individuals.push(arrival);

    if let Err(err) = save_existing_state(&state.backend, &target_sim).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response();
    }
    if let Err(err) = upsert_individuals(&state.backend, &target_sim, true).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response();
    }
    Json(json!({ "message": "Individual migrated.", "arrived_individual_id": arrival_id })).into_response()
}

pub async fn quarantine(
    Path(sim_id): Path<String>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<QuarantinePayload>,
) -> impl IntoResponse {
    let row = match load_simulation(&state.backend, &sim_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "Simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let sim_state = row_to_state(&row);
    if !is_allowed(&state, &headers, sim_state.user_id.clone()).await {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Simulation owner required."}))).into_response();
    }
    // Must land in world_state (flattened into the top-level world JSON
    // environment::natural_disaster_probability reads via "quarantine_mode"),
    // not the simulation's own top-level `extra` -- writing there left this
    // toggle checked nowhere, silently doing nothing.
    let mut sim = serde_json::to_value(&sim_state).unwrap_or_else(|_| json!({}));
    if let Some(sim_obj) = sim.as_object_mut() {
        let world_state = sim_obj.entry("world_state").or_insert_with(|| json!({}));
        if let Some(world_state_obj) = world_state.as_object_mut() {
            world_state_obj.insert("quarantine_mode".to_string(), json!(payload.enabled.unwrap_or(true)));
        }
    }
    match save_snapshot(&sim_id, &state, &sim).await {
        Ok(()) => Json(json!({"message": "Quarantine updated."})).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

/// The old stub echoed the message back verbatim regardless of who was
/// asked; this is the shared fallback for when Gemini is unavailable *and*
/// for an unknown individual_id, so behavior stays predictable either way.
fn echo_response(individual_id: &str, message: &str) -> String {
    format!("Rust divine response to {individual_id}: {message}")
}

pub async fn talk(
    Path((sim_id, individual_id)): Path<(String, String)>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<TalkPayload>,
) -> impl IntoResponse {
    let sim_state = match load_full_state(&state.backend, &sim_id).await {
        Ok(Some(sim_state)) => sim_state,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "Simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    if !is_allowed(&state, &headers, sim_state.user_id.clone()).await {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Simulation owner required."}))).into_response();
    }
    let message = payload.message.trim();
    let lang = lang_name(payload.lang.clone());

    let Some(individual) = sim_state.individuals.iter().find(|i| i.id == individual_id) else {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "Individual not found"}))).into_response();
    };

    // Cardinal-rule-adjacent: the response must be constrained by this
    // individual's *own* language stage and vocabulary (genetics + observed
    // learning), never a generic assistant voice -- so a stage-0 descendant
    // can't suddenly speak in full sentences just because an LLM is available.
    let stage = individual.language.stage;
    let stage_name = if individual.language.stage_name.is_empty() { "pre-linguistic".to_string() } else { individual.language.stage_name.clone() };
    let known_words: Vec<&str> = individual.language.vocabulary.values().map(String::as_str).collect();
    let name = individual.phenotype.name.clone().unwrap_or_else(|| individual_id.clone());

    let system = if stage <= 1 || known_words.is_empty() {
        format!(
            "You are {name}, an individual in the Anatolia-Sim civilization simulator at language stage \
             {stage} ({stage_name}) with no real vocabulary yet. Reply with ONLY a short bracketed \
             description of a gesture, sound or facial expression (e.g. \"[points at the sky and grunts]\"), \
             written in {lang}, never actual spoken words."
        )
    } else {
        format!(
            "You are {name}, an individual in the Anatolia-Sim civilization simulator at language stage \
             {stage} ({stage_name}). Your entire known vocabulary is limited to these words: {}. Reply \
             ONLY using words from that list, staying in character for this exact stage of language \
             development -- never use a concept or word you don't know, even if the question is in {lang}. \
             Keep the reply to a few words or one short sentence.",
            known_words.join(", ")
        )
    };

    let response = match gemini::chat(GeminiRequest { system: &system, user: message, max_tokens: 400, temperature: 0.8, json_response: false }).await {
        Ok(text) => text,
        Err(err) => {
            tracing::warn!(%err, sim_id, individual_id, "gemini god-talk call failed, falling back to echo");
            echo_response(&individual_id, message)
        }
    };

    Json(json!({ "response": response, "language": lang })).into_response()
}

