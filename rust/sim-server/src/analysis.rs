use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    auth::authenticate,
    db::{load_simulation, row_to_state, AppState},
    gemini::{self, GeminiRequest},
    routes::authorize_sim_access,
};

#[derive(Debug, Deserialize)]
pub struct AnalysisPayload {
    pub message: Option<String>,
    pub lang: Option<String>,
}

/// Client-supplied twin of `AnalysisPayload` for callers with no DB-backed
/// simulation to load (WASM-local mode, whose sims live only in the
/// caller's own IndexedDB -- see client/src/wasmLocal/apiAdapter.ts). The
/// caller reports its own state summary instead of a `sim_id` to look up.
#[derive(Debug, Deserialize)]
pub struct LocalAnalysisPayload {
    pub message: Option<String>,
    pub lang: Option<String>,
    pub day: Option<i64>,
    pub population: Option<i64>,
    pub events: Option<i64>,
    pub techs: Option<i64>,
}

/// Client-supplied twin of `HypothesisPayload` -- see `LocalAnalysisPayload`.
#[derive(Debug, Deserialize)]
pub struct LocalHypothesisPayload {
    pub hypothesis: String,
    pub lang: Option<String>,
    pub events: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub stats: Option<serde_json::Value>,
    pub day: Option<i64>,
    pub population: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct HypothesisPayload {
    pub hypothesis: String,
    pub lang: Option<String>,
    pub events: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub stats: Option<serde_json::Value>,
}

pub(crate) fn lang_name(lang: Option<String>) -> &'static str {
    match lang.as_deref() {
        Some("tr") => "Turkish",
        Some("de") => "German",
        Some("fr") => "French",
        Some("ar") => "Arabic",
        _ => "English",
    }
}

// `population` is passed in from the dedicated `population_count` DB column
// rather than read off `sim` -- state_json no longer embeds `individuals` (see
// db.rs's `state_json_for_db`), so `sim.individuals` would always read as
// empty here otherwise.
fn simulation_summary(sim: &serde_json::Value, population: i64) -> String {
    let current_day = sim.get("current_day").and_then(|v| v.as_i64()).unwrap_or(0);
    let events = sim.get("events").and_then(|v| v.as_array()).map(|v| v.len()).unwrap_or(0);
    let techs = sim.get("discovered_techs").and_then(|v| v.as_array()).map(|v| v.len()).unwrap_or(0);
    format!("day={current_day}, population={population}, events={events}, techs={techs}")
}

/// Same shape as `simulation_summary`, built from the caller's own reported
/// counts instead of a DB-loaded `SimulationState` -- see
/// `LocalAnalysisPayload`.
fn local_summary(day: i64, population: i64, events: i64, techs: i64) -> String {
    format!("day={day}, population={population}, events={events}, techs={techs}")
}

/// Parses a hypothesis-test verdict out of a (possibly markdown-fenced) JSON
/// response, rejecting anything that isn't one of the three known verdicts or
/// whose confidence falls outside 0..=1 -- callers fall back to the
/// population/event heuristic when this returns `None`.
fn parse_hypothesis_json(raw: &str) -> Option<(String, f64, String)> {
    let cleaned = gemini::strip_code_fence(raw);
    let value: serde_json::Value = serde_json::from_str(cleaned).ok()?;
    let verdict = value.get("verdict")?.as_str()?.to_string();
    if !["supported", "refuted", "inconclusive"].contains(&verdict.as_str()) {
        return None;
    }
    let confidence = value.get("confidence")?.as_f64()?;
    if !(0.0..=1.0).contains(&confidence) {
        return None;
    }
    let reasoning = value.get("reasoning").and_then(|v| v.as_str()).unwrap_or("").to_string();
    Some((verdict, confidence, reasoning))
}

pub async fn analyze(
    Path(sim_id): Path<String>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<AnalysisPayload>,
) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &sim_id).await {
        return resp;
    }
    let row = match load_simulation(&state.backend, &sim_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let population_count = row.population_count;
    let sim = row_to_state(&row);
    let summary_value = serde_json::to_value(&sim).unwrap_or_else(|_| json!({}));
    let lang = lang_name(payload.lang);
    let message = payload.message.unwrap_or_else(|| "Analyze the current simulation state.".to_string());
    let summary = simulation_summary(&summary_value, population_count);

    let system = format!(
        "{}\n\nYou are BOLD, this app's scientific analysis assistant. Answer the user's question \
         about their live simulation using only the state summary you are given below; do not invent \
         numbers that aren't in it. Be concise (2-4 sentences) and concrete. Respond only in {lang}.",
        gemini::APP_PRIMER
    );
    let user_prompt = format!("Simulation state: {summary}\n\nQuestion: {}", message.trim());

    let response = match gemini::chat(GeminiRequest { system: &system, user: &user_prompt, max_tokens: 900, temperature: 0.4, json_response: false }).await {
        Ok(text) => text,
        Err(err) => {
            tracing::warn!(%err, sim_id, "gemini analysis call failed, falling back to heuristic summary");
            format!("[Rust analysis/{lang}] {} | {}", message.trim(), summary)
        }
    };

    Json(json!({ "response": response, "language": lang })).into_response()
}

pub async fn hypothesis(
    Path(sim_id): Path<String>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<HypothesisPayload>,
) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &sim_id).await {
        return resp;
    }
    let row = match load_simulation(&state.backend, &sim_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let population = row.population_count;
    let sim = row_to_state(&row);
    let day = sim.current_day as i64;
    let events = payload.events.as_ref().map(|v| v.len() as i64).unwrap_or(0);
    let lang = lang_name(payload.lang);

    let heuristic_confidence = if population > 100 {
        0.72
    } else if population > 20 {
        0.58
    } else {
        0.44
    };
    let heuristic_verdict = if population > 50 && events > 10 {
        "supported"
    } else if population == 0 {
        "refuted"
    } else {
        "inconclusive"
    };
    let heuristic_reasoning = format!("Rust backend heuristic for '{}' at day {} with population {}.", payload.hypothesis, day, population);

    let stats_summary = payload
        .stats
        .as_ref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| simulation_summary(&serde_json::to_value(&sim).unwrap_or_else(|_| json!({})), population));

    let system = format!(
        "You are BOLD, the scientific hypothesis-testing engine of Anatolia-Sim, an agent-based \
         civilization simulator. Evaluate the user's hypothesis strictly against the given \
         simulation statistics -- never invent data that isn't provided. Reply with ONLY a JSON \
         object of the exact shape {{\"verdict\": \"supported\" | \"refuted\" | \"inconclusive\", \
         \"confidence\": <number 0..1>, \"reasoning\": <string>}}, no markdown, no extra keys. \
         Write the reasoning in {lang}."
    );
    let user_prompt = format!(
        "Simulation statistics: {stats_summary}\nDay: {day}\nPopulation: {population}\nRecorded evidence events: {events}\n\nHypothesis: {}",
        payload.hypothesis
    );

    let (verdict, confidence, reasoning) = match gemini::chat(GeminiRequest { system: &system, user: &user_prompt, max_tokens: 700, temperature: 0.2, json_response: true }).await {
        Ok(raw) => parse_hypothesis_json(&raw).unwrap_or_else(|| {
            tracing::warn!(raw = %raw, sim_id, "gemini hypothesis response was not valid JSON, using heuristic");
            (heuristic_verdict.to_string(), heuristic_confidence, heuristic_reasoning.clone())
        }),
        Err(err) => {
            tracing::warn!(%err, sim_id, "gemini hypothesis call failed, using heuristic");
            (heuristic_verdict.to_string(), heuristic_confidence, heuristic_reasoning.clone())
        }
    };

    Json(json!({
        "verdict": verdict,
        "confidence": confidence,
        "n_evidence": events.max(1),
        "language": lang,
        "reasoning": reasoning,
        "ci_lower": (confidence - 0.12_f64).max(0.0),
        "ci_upper": (confidence + 0.12_f64).min(1.0),
    }))
    .into_response()
}

/// `analyze`'s counterpart for WASM-local callers, whose simulation has no
/// DB row to load: same prompt, same Gemini-failure heuristic fallback,
/// just built from the caller's self-reported state summary rather than
/// `load_simulation`. Still requires a valid bearer token -- there is no
/// sim to own, but this still shouldn't be an open proxy onto the app's
/// Gemini key for anyone unauthenticated.
pub async fn analyze_local(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(payload): Json<LocalAnalysisPayload>) -> impl IntoResponse {
    if authenticate(&state, &headers).await.is_none() {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response();
    }

    let lang = lang_name(payload.lang);
    let message = payload.message.unwrap_or_else(|| "Analyze the current simulation state.".to_string());
    let summary = local_summary(payload.day.unwrap_or(0), payload.population.unwrap_or(0), payload.events.unwrap_or(0), payload.techs.unwrap_or(0));

    let system = format!(
        "{}\n\nYou are BOLD, this app's scientific analysis assistant. Answer the user's question \
         about their live simulation using only the state summary you are given below; do not invent \
         numbers that aren't in it. Be concise (2-4 sentences) and concrete. Respond only in {lang}.",
        gemini::APP_PRIMER
    );
    let user_prompt = format!("Simulation state: {summary}\n\nQuestion: {}", message.trim());

    let response = match gemini::chat(GeminiRequest { system: &system, user: &user_prompt, max_tokens: 900, temperature: 0.4, json_response: false }).await {
        Ok(text) => text,
        Err(err) => {
            tracing::warn!(%err, "gemini local analysis call failed, falling back to heuristic summary");
            format!("[Rust analysis/{lang}] {} | {}", message.trim(), summary)
        }
    };

    Json(json!({ "response": response, "language": lang })).into_response()
}

/// `hypothesis`'s counterpart for WASM-local callers -- see `analyze_local`.
pub async fn hypothesis_local(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(payload): Json<LocalHypothesisPayload>) -> impl IntoResponse {
    if authenticate(&state, &headers).await.is_none() {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response();
    }

    let population = payload.population.unwrap_or(0);
    let day = payload.day.unwrap_or(0);
    let events = payload.events.as_ref().map(|v| v.len() as i64).unwrap_or(0);
    let lang = lang_name(payload.lang);

    let heuristic_confidence = if population > 100 {
        0.72
    } else if population > 20 {
        0.58
    } else {
        0.44
    };
    let heuristic_verdict = if population > 50 && events > 10 {
        "supported"
    } else if population == 0 {
        "refuted"
    } else {
        "inconclusive"
    };
    let heuristic_reasoning = format!("Rust backend heuristic for '{}' at day {} with population {}.", payload.hypothesis, day, population);

    let stats_summary = payload.stats.as_ref().map(|s| s.to_string()).unwrap_or_else(|| local_summary(day, population, events, 0));

    let system = format!(
        "You are BOLD, the scientific hypothesis-testing engine of Anatolia-Sim, an agent-based \
         civilization simulator. Evaluate the user's hypothesis strictly against the given \
         simulation statistics -- never invent data that isn't provided. Reply with ONLY a JSON \
         object of the exact shape {{\"verdict\": \"supported\" | \"refuted\" | \"inconclusive\", \
         \"confidence\": <number 0..1>, \"reasoning\": <string>}}, no markdown, no extra keys. \
         Write the reasoning in {lang}."
    );
    let user_prompt = format!(
        "Simulation statistics: {stats_summary}\nDay: {day}\nPopulation: {population}\nRecorded evidence events: {events}\n\nHypothesis: {}",
        payload.hypothesis
    );

    let (verdict, confidence, reasoning) = match gemini::chat(GeminiRequest { system: &system, user: &user_prompt, max_tokens: 700, temperature: 0.2, json_response: true }).await {
        Ok(raw) => parse_hypothesis_json(&raw).unwrap_or_else(|| {
            tracing::warn!(raw = %raw, "gemini local hypothesis response was not valid JSON, using heuristic");
            (heuristic_verdict.to_string(), heuristic_confidence, heuristic_reasoning.clone())
        }),
        Err(err) => {
            tracing::warn!(%err, "gemini local hypothesis call failed, using heuristic");
            (heuristic_verdict.to_string(), heuristic_confidence, heuristic_reasoning.clone())
        }
    };

    Json(json!({
        "verdict": verdict,
        "confidence": confidence,
        "n_evidence": events.max(1),
        "language": lang,
        "reasoning": reasoning,
        "ci_lower": (confidence - 0.12_f64).max(0.0),
        "ci_upper": (confidence + 0.12_f64).min(1.0),
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hypothesis_json_reads_a_clean_object() {
        let raw = r#"{"verdict":"supported","confidence":0.81,"reasoning":"population grew steadily"}"#;
        let (verdict, confidence, reasoning) = parse_hypothesis_json(raw).unwrap();
        assert_eq!(verdict, "supported");
        assert_eq!(confidence, 0.81);
        assert_eq!(reasoning, "population grew steadily");
    }

    #[test]
    fn parse_hypothesis_json_strips_a_markdown_fence() {
        let raw = "```json\n{\"verdict\":\"refuted\",\"confidence\":0.2,\"reasoning\":\"no evidence\"}\n```";
        let (verdict, confidence, _) = parse_hypothesis_json(raw).unwrap();
        assert_eq!(verdict, "refuted");
        assert_eq!(confidence, 0.2);
    }

    #[test]
    fn parse_hypothesis_json_rejects_an_unknown_verdict() {
        assert!(parse_hypothesis_json(r#"{"verdict":"maybe","confidence":0.5,"reasoning":"x"}"#).is_none());
    }

    #[test]
    fn parse_hypothesis_json_rejects_out_of_range_confidence() {
        assert!(parse_hypothesis_json(r#"{"verdict":"supported","confidence":1.4,"reasoning":"x"}"#).is_none());
    }

    #[test]
    fn parse_hypothesis_json_rejects_malformed_json() {
        assert!(parse_hypothesis_json("not json at all").is_none());
    }
}
