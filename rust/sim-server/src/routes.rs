use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::{authenticate, cloud_api_url, is_local_backend};
use crate::gemini::{self, GeminiRequest};
use crate::db::{
    insert_checkpoint,
    db_status_counts,
    delete_simulation,
    list_checkpoints,
    list_live_snapshots,
    list_simulations,
    load_current_day,
    load_full_state,
    load_individual_payload,
    load_individual_payloads,
    load_live_snapshot,
    load_simulation,
    load_checkpoint,
    row_to_state,
    save_existing_state,
    save_state,
    system_counts,
    update_simulation_fields,
    upsert_individuals,
    upsert_live_snapshot,
    AppState,
};
use sim_core::{
    advance_one_day, age_years, derive_stats, individual_display_name, pascal_to_snake, serialize_individual, to_client_event,
    Individual,
    SimulationState,
};

/// Every `/api/simulations/:id/*` handler below is individually responsible
/// for its own access control -- this app has no global auth middleware
/// (see main.rs's router assembly, whose only `.layer()` is CORS). Mirrors
/// `god::is_allowed`'s ownership-or-admin check, generalized here since
/// nearly every handler in this file needs it.
///
/// Simulations created before per-user ownership existed have no `user_id`
/// at all (see `list_live`'s own "orphaned records predating the ownership
/// system" comment) -- those stay reachable by any authenticated caller
/// rather than becoming permanently inaccessible to whoever actually
/// created them, since locking them down now would be a regression, not a
/// fix. Returns 404 (not 403) for a real-but-unowned simulation, matching
/// every other "not found" response in this file and not confirming to a
/// caller that an ID they don't own actually exists.
pub(crate) async fn authorize_sim_access(state: &AppState, headers: &axum::http::HeaderMap, sim_id: &str) -> Result<(), axum::response::Response> {
    let Some(claims) = authenticate(state, headers).await else {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response());
    };
    if claims.role == "admin" {
        return Ok(());
    }
    let row = match load_simulation(&state.backend, sim_id).await {
        Ok(Some(row)) => row,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response()),
        Err(err) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response()),
    };
    // Pulled straight from the state_json column rather than row_to_state,
    // which deserializes the whole SimulationState -- including every
    // individual ever born -- just to read one field.
    let owner = row.state_json.get("user_id").and_then(Value::as_str);
    if owner.is_none() || owner == Some(claims.id.as_str()) {
        Ok(())
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response())
    }
}

/// Prefers the tick loop's own live in-memory state (only ever populated
/// while upload is paused -- see runtime.rs's live_state) over a fresh DB
/// reload. Without this, a paused-upload session would make every one of
/// these read endpoints (simulation snapshot, stats, events) look frozen at
/// the last synced state even though the simulation keeps computing.
async fn load_live_or_full_state(state: &AppState, id: &str) -> Result<Option<SimulationState>, sqlx::Error> {
    if let Some(sim) = state.runtime.live_state(id).await {
        return Ok(Some(sim));
    }
    load_full_state(&state.backend, id).await
}

pub fn simulation_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_all).post(create_simulation))
        .route("/live", get(list_live))
        .route("/live-sync", post(live_sync))
        .route("/live/:simId", get(get_live_snapshot))
        .route("/import", post(import_simulation))
        .route("/:id/upload-to-cloud", post(upload_to_cloud))
        .route("/:id/live-sync-tick", post(live_sync_tick))
        .route("/:id", get(get_simulation))
        .route("/:id/stats", get(get_stats))
        .route("/:id/legends", get(get_legends))
        .route("/:id/documentary", get(documentary))
        .route("/:id/export", get(export_simulation))
        .route("/:id/start", post(start_simulation))
        .route("/:id/pause", post(pause_simulation))
        .route("/:id/pause-upload", post(pause_upload))
        .route("/:id/resume-upload", post(resume_upload))
        .route("/:id/engines", post(set_disabled_engines))
        .route("/:id/complete", post(complete_simulation))
        .route("/:id/speed", post(set_speed))
        .route("/:id/tick", post(tick_simulation))
        .route("/:id/population", get(get_population))
        .route("/:id/population/:individualId", get(get_individual))
        .route("/:id/events", get(get_events))
        .route("/:id/events/summary", get(get_events_summary))
        .route("/:id/checkpoints", get(get_checkpoints))
        .route("/:id/checkpoint", post(create_checkpoint))
        .route("/:id/restore/:checkpointId", post(restore_checkpoint))
        .route("/:id/report", get(get_report))
        .route("/:id/metrics", get(get_metrics))
        .route("/:id/diagnostics", get(get_diagnostics))
        .route("/:id/db-status", get(get_db_status))
        .route("/:id/fast-forward", post(fast_forward))
        .route("/:id/fast-forward/cancel", post(cancel_fast_forward))
        .route("/:id/terminate", post(terminate_simulation))
        .route("/:id", axum::routing::delete(delete_simulation_route))
        .route("/compare", get(compare_simulations))
}

/// `?a=<simId>&b=<simId>` -- side-by-side aggregate stats for two
/// simulations the caller owns (or is admin over), meant for comparing e.g.
/// two different founder-genome experiments' emergent outcomes. Read-only,
/// purely a projection of each simulation's own already-tracked stats via
/// `derive_stats` -- computes nothing new about any individual.
#[derive(serde::Deserialize)]
struct CompareQuery {
    a: String,
    b: String,
}

async fn compare_simulations(State(state): State<AppState>, Query(params): Query<CompareQuery>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &params.a).await {
        return resp;
    }
    if let Err(resp) = authorize_sim_access(&state, &headers, &params.b).await {
        return resp;
    }
    let sim_a = match load_live_or_full_state(&state, &params.a).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation a not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let sim_b = match load_live_or_full_state(&state, &params.b).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation b not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    Json(json!({
        "a": { "id": params.a, "name": sim_a.name, "stats": derive_stats(&sim_a) },
        "b": { "id": params.b, "name": sim_b.name, "stats": derive_stats(&sim_b) },
    }))
    .into_response()
}

// Render injects RENDER_GIT_COMMIT (the deployed commit's full SHA) into
// every service automatically -- reading it once at startup and handing it
// back here is what lets SimulationPage.tsx's poll-and-reload-on-deploy
// effect actually detect a new deploy (it compares this value across polls
// and reloads once it changes). Without RENDER_GIT_COMMIT (local/desktop),
// this stays constant for the process's whole lifetime, so that effect
// never fires there -- there's no separate "deploy" to detect locally.
fn deployed_version() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| std::env::var("RENDER_GIT_COMMIT").unwrap_or_else(|_| "dev".to_string()))
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "runtime": "rust",
        "core": "tokio+axum+rayon+sqlx",
        "db_backend": crate::db::backend_name(&state.backend),
        "version": deployed_version(),
    }))
}

pub async fn system_status(State(state): State<AppState>) -> impl IntoResponse {
    match system_counts(&state.backend).await {
        Ok((active_sims, total_population)) => Json(json!({
            "status": "online",
            "genome_loci": 32,
            "epi_loci": 8,
            "lang_stages": 7,
            "active_sims": active_sims,
            "total_population": total_population,
        }))
        .into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "status": "degraded",
            "genome_loci": 32,
            "epi_loci": 8,
            "lang_stages": 7,
            "active_sims": 0,
            "total_population": 0,
        }))).into_response(),
    }
}

async fn list_all(State(state): State<AppState>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    let Some(claims) = authenticate(&state, &headers).await else {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response();
    };
    match list_simulations(&state.backend).await {
        Ok(rows) => {
            let sims: Vec<SimulationState> = rows
                .iter()
                .map(row_to_state)
                .filter(|sim| sim.user_id.as_deref() == Some(claims.id.as_str()))
                .collect();
            Json(json!(sims)).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

// Simulations another device currently has running, for the "Canlı İzle"
// (watch live) list on the dashboard -- must be registered ahead of
// "/:id" or it would be routed there instead. On the cloud backend this
// also includes desktop "Yerel" mode simulations pushed via live_sync_tick
// (see live_snapshots' own doc comment) -- those never have a row in this
// server's own `simulations` table, only in `live_snapshots`, so they'd be
// invisible here otherwise even while genuinely running on someone's device.
async fn list_live(State(state): State<AppState>) -> impl IntoResponse {
    match list_simulations(&state.backend).await {
        Ok(rows) => {
            let mut live: Vec<Value> = rows
                .iter()
                .filter(|row| row.status == "running")
                .map(|row| {
                    json!({
                        "simulation_id": row.id,
                        "simulation_name": row.name,
                        "current_day": row.current_day,
                        "current_year": row.current_year,
                        "population_count": row.population_count,
                        "updated_at": row.updated_at,
                    })
                })
                .collect();
            if !is_local_backend(&state) {
                match list_live_snapshots(&state.backend).await {
                    Ok(snapshots) => live.extend(snapshots),
                    Err(err) => tracing::warn!(error = %err, "list_live_snapshots failed"),
                }
            }
            Json(live).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

// Target of the desktop "Yerel" dashboard's one-time "Buluta Yükle" (Upload
// to Cloud) action. Takes a full simulation snapshot pushed from a local
// (SQLite) device and inserts it as a brand-new simulation owned by the
// authenticated user -- never overwrites an existing cloud simulation, so
// re-uploading after continuing to play locally just creates another copy
// (see the one-time-upload decision this feature was built around).
async fn import_simulation(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(mut sim): Json<SimulationState>,
) -> impl IntoResponse {
    let Some(claims) = authenticate(&state, &headers).await else {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response();
    };
    // Keyed per user (unlike register's global window): the caller is
    // already an authenticated account, so nothing stops them from repeatedly
    // POSTing a large `individuals` payload otherwise -- axum's default body
    // limit is the only other bound on this route.
    if !state.rate_limiter.check(&format!("import:{}", claims.id), 20, std::time::Duration::from_secs(15 * 60)) {
        return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": "Too many imports. Please try again later."}))).into_response();
    }
    let new_id = Uuid::new_v4().to_string();
    sim.id = Some(new_id.clone());
    sim.user_id = Some(claims.id);
    sim.status = Some("paused".to_string());
    // Trust the uploaded individuals list itself, not whatever total_ever_born
    // the uploading client happened to send (an older client may not know the
    // field at all, deserializing it to 0 via #[serde(default)]) -- this is a
    // brand-new row, so save_state's own "individuals.len() is authoritative
    // at creation" invariant applies here exactly as it does for a fresh sim.
    sim.total_ever_born = sim.individuals.len() as i32;

    match save_state(&state.backend, &sim).await {
        Ok(_) => {
            let _ = upsert_individuals(&state.backend, &sim, true).await;
            (StatusCode::CREATED, Json(json!({"simulation_id": new_id, "name": sim.name}))).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

#[derive(Deserialize)]
struct LiveSyncPayload {
    simulation_id: String,
    simulation_name: String,
    current_day: i32,
    current_year: i32,
    population_count: i32,
    #[serde(default)]
    agents_snapshot: Value,
    #[serde(default)]
    stats: Value,
    #[serde(default)]
    groups: Value,
    is_running: bool,
}

// Cloud-side receiver for the desktop "Yerel" dashboard's periodic push
// (see live_sync_tick below, which calls this). Meaningful on the cloud
// backend only -- a local server has no live_snapshots table of its own to
// write into (upsert_live_snapshot no-ops on SQLite).
async fn live_sync(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(payload): Json<LiveSyncPayload>) -> impl IntoResponse {
    let Some(claims) = authenticate(&state, &headers).await else {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response();
    };
    match upsert_live_snapshot(
        &state.backend,
        &claims.id,
        &payload.simulation_id,
        &payload.simulation_name,
        payload.current_day,
        payload.current_year,
        payload.population_count,
        &payload.agents_snapshot,
        &payload.stats,
        &payload.groups,
        payload.is_running,
    )
    .await
    {
        Ok(()) => Json(json!({"message": "synced"})).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

// WatchPage.tsx's data source -- must be registered ahead of "/:id" (see
// list_live's own comment on the same routing gotcha).
async fn get_live_snapshot(State(state): State<AppState>, Path(sim_id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if authenticate(&state, &headers).await.is_none() {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response();
    }
    match load_live_snapshot(&state.backend, &sim_id).await {
        Ok(Some(snapshot)) => Json(snapshot).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "no live snapshot for this simulation"}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

// Local-device half of the periodic sync: the client (SimulationPage.tsx)
// fires this at its own local sim-server every 20s while a "Yerel" mode
// simulation is running, reusing the caller's own bearer token exactly like
// upload_to_cloud does -- the local server then builds a lightweight
// snapshot from its own already-loaded state and forwards it to the cloud's
// live_sync above. Only meaningful for the local/SQLite backend; the cloud
// backend has nowhere further to push to (mirrors upload_to_cloud's own
// is_local_backend guard).
async fn live_sync_tick(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if !is_local_backend(&state) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "This simulation is already in the cloud."}))).into_response();
    }
    let Some(token) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    else {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response();
    };

    let sim = match load_full_state(&state.backend, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    let agents_snapshot: Vec<Value> = sim
        .individuals
        .iter()
        .filter(|i| i.alive && !i.is_dead)
        .map(|i| json!({ "id": i.id, "x": i.x, "y": i.y, "sex": i.sex, "group_id": i.group_id }))
        .collect();
    let groups: Vec<Value> = sim
        .groups
        .iter()
        .map(|g| {
            json!({
                "id": g.get("id").cloned().unwrap_or(Value::Null),
                "name": g.get("name").cloned().unwrap_or(Value::Null),
                "size": g.get("member_ids").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0),
            })
        })
        .collect();
    let stats = derive_stats(&sim);
    let payload = json!({
        "simulation_id": id,
        "simulation_name": sim.name,
        "current_day": sim.current_day,
        "current_year": sim.current_year,
        "population_count": sim.world_state.alive_count.unwrap_or(agents_snapshot.len()),
        "agents_snapshot": agents_snapshot,
        "stats": stats,
        "groups": groups,
        "is_running": sim.status.as_deref() == Some("running"),
    });

    let client = reqwest::Client::new();
    let resp = match client.post(format!("{}/api/simulations/live-sync", cloud_api_url())).bearer_auth(token).json(&payload).send().await {
        Ok(resp) => resp,
        Err(err) => return (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Could not reach the cloud: {err}")}))).into_response(),
    };
    if !resp.status().is_success() {
        let body: Value = resp.json().await.unwrap_or_else(|_| json!({}));
        return (StatusCode::BAD_GATEWAY, Json(json!({"error": body.get("error").cloned().unwrap_or_else(|| json!("Live sync failed."))}))).into_response();
    }
    Json(json!({"message": "synced"})).into_response()
}

// Local-device half of the "Buluta Yükle" action: reads this device's own
// copy of the simulation and forwards it to the cloud's `import_simulation`
// above, reusing the caller's own bearer token (desktop is required to be
// online, and that token was itself obtained from the cloud -- see
// auth::authenticate). Only meaningful for the local/SQLite backend; the
// cloud backend has nowhere further to upload to.
async fn upload_to_cloud(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if !is_local_backend(&state) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "This simulation is already in the cloud."}))).into_response();
    }
    let Some(token) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    else {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response();
    };

    let sim = match load_full_state(&state.backend, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/simulations/import", cloud_api_url()))
        .bearer_auth(token)
        .json(&sim)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => return (StatusCode::BAD_GATEWAY, Json(json!({"error": format!("Could not reach the cloud: {err}")}))).into_response(),
    };

    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return (StatusCode::BAD_GATEWAY, Json(json!({"error": body.get("error").cloned().unwrap_or_else(|| json!("Cloud upload failed."))}))).into_response();
    }
    (StatusCode::CREATED, Json(body)).into_response()
}

async fn get_simulation(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    match load_live_or_full_state(&state, &id).await {
        Ok(Some(sim)) => Json(json!(sim)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

// Lightweight snapshot of `derive_stats`, the same payload the WebSocket
// broadcasts on every tick. Lets the client populate `stats` immediately on
// mount/reconnect instead of waiting for the next scheduled WS tick.
async fn get_stats(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let sim = match load_live_or_full_state(&state, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    Json(derive_stats(&sim)).into_response()
}

async fn get_legends(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let sim = match load_live_or_full_state(&state, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    Json(compute_legends(&sim)).into_response()
}

fn legend_entry(ind: &Individual, value: Value) -> Value {
    json!({
        "id": ind.id,
        "name": individual_display_name(ind),
        "sex": ind.sex,
        "birth_year": ind.birth_day.div_euclid(365),
        "death_year": ind.death_day.map(|d| d.div_euclid(365)),
        "alive": ind.alive && !ind.is_dead,
        "is_founder": ind.is_founder,
        "value": value,
    })
}

/// "Legends" -- one record-holder per category, surfaced out of an
/// otherwise huge population list. Purely a read-only projection of
/// already-tracked fields (consciousness, children, longevity, reputation,
/// discovery events); computes nothing new about any individual and grants
/// no behavior, so it sits outside the cardinal rule's scope entirely.
fn compute_legends(sim: &SimulationState) -> Value {
    let highest_consciousness = sim
        .individuals
        .iter()
        .max_by(|a, b| a.mind.consciousness.partial_cmp(&b.mind.consciousness).unwrap_or(std::cmp::Ordering::Equal))
        .map(|ind| legend_entry(ind, json!((ind.mind.consciousness * 1000.0).round() / 1000.0)));

    let most_children = sim
        .individuals
        .iter()
        .filter(|ind| !ind.social.children_ids.is_empty())
        .max_by_key(|ind| ind.social.children_ids.len())
        .map(|ind| legend_entry(ind, json!(ind.social.children_ids.len())));

    // Longevity in days-lived, not calendar death_day -- a descendant born
    // on day 10000 who died at 80 must be able to beat a founder who died on
    // day 5000 at age 40, even though the founder's raw death_day is smaller.
    let longest_lived = sim
        .individuals
        .iter()
        .filter(|ind| ind.is_dead)
        .max_by_key(|ind| ind.death_day.unwrap_or(ind.birth_day) - ind.birth_day)
        .map(|ind| legend_entry(ind, json!((ind.death_day.unwrap_or(ind.birth_day) - ind.birth_day).div_euclid(365))));

    let highest_reputation = sim
        .individuals
        .iter()
        .max_by(|a, b| a.social.reputation.partial_cmp(&b.social.reputation).unwrap_or(std::cmp::Ordering::Equal))
        .map(|ind| legend_entry(ind, json!((ind.social.reputation * 1000.0).round() / 1000.0)));

    // Discoveries are attributed via events (tick.rs's "discovery" events
    // carry discoverer_id/tech_id), not a per-individual counter field --
    // events is the single source of truth for who discovered what.
    let mut discovery_counts: HashMap<&str, usize> = HashMap::new();
    for event in &sim.events {
        if event.get("type").and_then(Value::as_str) != Some("discovery") {
            continue;
        }
        if let Some(discoverer_id) = event.get("discoverer_id").and_then(Value::as_str) {
            *discovery_counts.entry(discoverer_id).or_insert(0) += 1;
        }
    }
    let most_technologies = discovery_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .and_then(|(id, count)| sim.individuals.iter().find(|i| i.id == id).map(|ind| legend_entry(ind, json!(count))));

    json!({
        "highest_consciousness": highest_consciousness,
        "most_children": most_children,
        "longest_lived": longest_lived,
        "highest_reputation": highest_reputation,
        "most_technologies": most_technologies,
    })
}

#[derive(Debug, Deserialize)]
struct DocumentaryQuery {
    lang: Option<String>,
}

/// AI-narrated "documentary" over a civilization's own tracked history --
/// reuses get_report's own notable-event filter (importance medium/high),
/// sampled evenly across the full timeline, and asks Gemini to narrate them
/// as documentary scenes constrained to only the facts given. Falls back to
/// a deterministic heuristic (one scene per event, verbatim descriptions)
/// on any Gemini failure, same reliability contract as every other
/// AI-backed feature in this app.
async fn documentary(State(state): State<AppState>, Path(id): Path<String>, Query(params): Query<DocumentaryQuery>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let sim = match load_live_or_full_state(&state, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let lang = crate::analysis::lang_name(params.lang);
    let civilization_name = sim.civilization_name.clone().unwrap_or_else(|| "Unnamed Civilization".to_string());

    let mut notable: Vec<Value> = sim
        .events
        .iter()
        .filter(|event| matches!(event.get("importance").and_then(Value::as_str), Some("medium") | Some("high")))
        .map(|event| to_client_event(event, &sim))
        .collect();
    notable.sort_by_key(|e| e.get("sim_day").and_then(Value::as_i64).unwrap_or(0));

    // Spread across the whole timeline rather than only the most recent
    // events -- a documentary should span the civilization's life, not just
    // its last few weeks.
    const MAX_SCENES: usize = 18;
    let selected: Vec<&Value> = if notable.len() <= MAX_SCENES {
        notable.iter().collect()
    } else {
        (0..MAX_SCENES).map(|i| &notable[i * (notable.len() - 1) / (MAX_SCENES - 1)]).collect()
    };

    let stats = derive_stats(&sim);
    let fallback = heuristic_documentary(&civilization_name, &selected, &stats);

    let event_lines: String = selected
        .iter()
        .map(|e| format!("Year {}: {}", e.get("sim_year").and_then(Value::as_i64).unwrap_or(0), e.get("description").and_then(Value::as_str).unwrap_or("")))
        .collect::<Vec<_>>()
        .join("\n");
    if event_lines.is_empty() {
        return Json(json!({ "civilization_name": civilization_name, "scenes": fallback, "generated_by": "heuristic" })).into_response();
    }

    let system = format!(
        "{}\n\nYou are a documentary narrator for this simulation. Given a chronological list of real \
         events from one civilization's history, write a short documentary broken into scenes. Respond \
         ONLY with a JSON array, each element shaped exactly like \
         {{\"year\": <integer>, \"title\": <short string>, \"narration\": <2-3 sentence string>}}. Use \
         only the facts given below -- never invent individuals, events, or numbers not present in the \
         data. Write in {lang}.",
        gemini::APP_PRIMER
    );
    let user_prompt = format!(
        "Civilization: {civilization_name}\nCurrent population: {}\nTechnologies discovered: {}\nHighest language stage reached: {}\n\nChronological events:\n{event_lines}",
        stats.get("population").and_then(Value::as_i64).unwrap_or(0),
        stats.get("technologies").and_then(Value::as_i64).unwrap_or(0),
        stats.get("max_language_stage").and_then(Value::as_i64).unwrap_or(0),
    );

    let (scenes, generated_by) = match gemini::chat(GeminiRequest { system: &system, user: &user_prompt, max_tokens: 2000, temperature: 0.6, json_response: true }).await {
        Ok(text) => match serde_json::from_str::<Value>(gemini::strip_code_fence(&text)) {
            Ok(Value::Array(arr)) if !arr.is_empty() && arr.iter().all(|s| s.get("year").is_some() && s.get("title").is_some() && s.get("narration").is_some()) => {
                (Value::Array(arr), "gemini")
            }
            _ => {
                tracing::warn!(sim_id = %id, "documentary: gemini returned an unusable shape, falling back to heuristic");
                (Value::Array(fallback.clone()), "heuristic")
            }
        },
        Err(err) => {
            tracing::warn!(%err, sim_id = %id, "documentary: gemini call failed, falling back to heuristic");
            (Value::Array(fallback.clone()), "heuristic")
        }
    };

    Json(json!({ "civilization_name": civilization_name, "scenes": scenes, "generated_by": generated_by })).into_response()
}

/// Deterministic fallback when Gemini is unavailable or misbehaves -- one
/// scene per selected notable event (using its own real description
/// verbatim, never inventing text), plus a closing "present day" scene.
/// Same reliability contract every other AI-backed feature in this app
/// already makes (see README: "falls back to the heuristic path whenever
/// GEMINI_API_KEY is unset or a call fails").
fn heuristic_documentary(civilization_name: &str, selected: &[&Value], stats: &Value) -> Vec<Value> {
    let mut scenes: Vec<Value> = selected
        .iter()
        .map(|e| {
            let year = e.get("sim_year").and_then(Value::as_i64).unwrap_or(0);
            let event_type = e.get("event_type").and_then(Value::as_str).unwrap_or("event");
            let title = pascal_to_snake(event_type).replace('_', " ");
            json!({
                "year": year,
                "title": title,
                "narration": e.get("description").and_then(Value::as_str).unwrap_or("").to_string(),
            })
        })
        .collect();
    scenes.push(json!({
        "year": stats.get("year").and_then(Value::as_i64).unwrap_or(0),
        "title": "present day",
        "narration": format!(
            "{civilization_name} now numbers {} individuals, with {} technologies discovered and language reaching stage {}.",
            stats.get("population").and_then(Value::as_i64).unwrap_or(0),
            stats.get("technologies").and_then(Value::as_i64).unwrap_or(0),
            stats.get("max_language_stage").and_then(Value::as_i64).unwrap_or(0),
        ),
    }));
    scenes
}

// Backs the dashboard's "Yedek Al" (export) button, which downloads the full
// simulation state as a JSON file client-side. Deliberately the same shape
// `import_simulation` accepts, so an exported file can be re-imported as-is.
async fn export_simulation(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    match load_full_state(&state.backend, &id).await {
        Ok(Some(sim)) => Json(json!(sim)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

#[derive(Deserialize)]
struct PopulationQuery {
    alive: Option<bool>,
    limit: Option<usize>,
}

// Reads individuals from their own table (kept in sync by upsert_individuals
// on every tick) instead of loading+deserializing the simulation's state_json
// blob -- that blob's size only grows with total-ever-born, not current
// population, and this endpoint is polled every few seconds by the client.
async fn get_population(State(state): State<AppState>, Path(id): Path<String>, Query(params): Query<PopulationQuery>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    // While upload is paused, the individuals table isn't being written to
    // (see runtime.rs), so the DB path below would show a frozen population
    // -- serve straight from the tick loop's live in-memory state instead
    // whenever it's available (see load_live_or_full_state's own comment).
    if let Some(sim) = state.runtime.live_state(&id).await {
        return Json(sim_core::population_view(&sim, params.alive, params.limit)).into_response();
    }
    let current_day = match load_current_day(&state.backend, &id).await {
        Ok(Some(day)) => day,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let limit = params.limit.map(|l| l as i64);
    let payloads = match load_individual_payloads(&state.backend, &id, params.alive, limit).await {
        Ok(payloads) => payloads,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let rows: Vec<Value> = payloads
        .into_iter()
        .filter_map(|payload| serde_json::from_value::<Individual>(payload).ok())
        .map(|ind| serialize_individual(&ind, current_day))
        .collect();
    Json(rows).into_response()
}

async fn get_individual(
    State(state): State<AppState>,
    Path((id, individual_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    if let Some(sim) = state.runtime.live_state(&id).await {
        return match sim.individuals.iter().find(|ind| ind.id == individual_id) {
            Some(ind) => Json(serialize_individual(ind, sim.current_day)).into_response(),
            None => (StatusCode::NOT_FOUND, Json(json!({"error": "Individual not found"}))).into_response(),
        };
    }
    let current_day = match load_current_day(&state.backend, &id).await {
        Ok(Some(day)) => day,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let payload = match load_individual_payload(&state.backend, &id, &individual_id).await {
        Ok(payload) => payload,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    match payload.and_then(|p| serde_json::from_value::<Individual>(p).ok()) {
        Some(ind) => Json(serialize_individual(&ind, current_day)).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "Individual not found"}))).into_response(),
    }
}

async fn get_events(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let sim = match load_live_or_full_state(&state, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let events: Vec<Value> = sim.events.iter().map(|e| to_client_event(e, &sim)).collect();
    Json(events).into_response()
}

async fn get_events_summary(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let sim = match load_live_or_full_state(&state, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    Json(sim_core::events_summary(&sim)).into_response()
}

async fn get_checkpoints(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    match list_checkpoints(&state.backend, &id).await {
        Ok(rows) => {
            let payload: Vec<Value> = rows
                .into_iter()
                .map(|row| json!({
                    "id": row.id,
                    "simulation_id": row.simulation_id,
                    "sim_day": row.sim_day,
                    "sim_year": row.sim_year,
                    "population_count": row.population_count,
                    "stats": row.stats,
                    "created_at": row.created_at,
                }))
                .collect();
            Json(payload).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

async fn create_simulation(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateSimulationRequest>,
) -> impl IntoResponse {
    let Some(claims) = authenticate(&state, &headers).await else {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Sign in required."}))).into_response();
    };
    let mut sim = sim_core::new_simulation(payload.name.clone(), payload.latitude, payload.longitude, &payload.founder_1_params, &payload.founder_2_params);
    sim.user_id = Some(claims.id.clone());

    match save_state(&state.backend, &sim).await {
        Ok(_) => {
            let _ = upsert_individuals(&state.backend, &sim, true).await;
            (StatusCode::CREATED, Json(json!(sim))).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

async fn create_checkpoint(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let sim = match load_full_state(&state.backend, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    let checkpoint_id = Uuid::new_v4().to_string();
    let population_snapshot = serde_json::to_value(&sim).unwrap_or_else(|_| json!({}));
    let tech_state = json!(sim.discovered_techs);
    let belief_state = json!(sim.discovered_beliefs);
    let art_state = json!(sim.discovered_arts);
    let groups = sim.extra.get("groups").cloned().unwrap_or_else(|| json!([]));
    let stats = derive_stats(&sim);

    match insert_checkpoint(
        &state.backend,
        &checkpoint_id,
        &id,
        sim.current_day,
        sim.current_year,
        sim.alive_count() as i64,
        population_snapshot,
        serde_json::to_value(&sim.world_state).unwrap_or_else(|_| json!({})),
        tech_state,
        belief_state,
        art_state,
        groups,
        stats,
    )
    .await
    {
        Ok(()) => Json(json!({
            "id": checkpoint_id,
            "simulation_id": id,
            "sim_day": sim.current_day,
            "sim_year": sim.current_year,
            "population_count": sim.alive_count(),
        }))
        .into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

async fn restore_checkpoint(
    State(state): State<AppState>,
    Path((id, checkpoint_id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let checkpoint = match load_checkpoint(&state.backend, &checkpoint_id, &id).await {
        Ok(Some(row)) => row,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "checkpoint not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    let mut sim: SimulationState = serde_json::from_value(checkpoint.population_snapshot.clone()).unwrap_or_default();
    sim.id = Some(id.clone());
    sim.current_day = checkpoint.sim_day as i32;
    sim.current_year = checkpoint.sim_year as i32;
    sim.status = Some("paused".to_string());

    match save_existing_state(&state.backend, &sim).await {
        Ok(_) => {
            let _ = upsert_individuals(&state.backend, &sim, true).await;
            Json(json!({
                "message": "Checkpoint restored",
                "sim_day": checkpoint.sim_day,
                "sim_year": checkpoint.sim_year
            }))
            .into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

async fn get_report(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let sim = match load_full_state(&state.backend, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let checkpoints = list_checkpoints(&state.backend, &id).await.unwrap_or_default();
    let current_stats = derive_stats(&sim);
    // Built once so each individual's top relationships can be resolved to a
    // display name in O(1) instead of an O(n) `find_individual` scan per bond.
    let name_by_id: std::collections::HashMap<&str, String> =
        sim.individuals.iter().map(|i| (i.id.as_str(), individual_display_name(i))).collect();
    let individuals: Vec<Value> = sim
        .individuals
        .iter()
        .map(|ind| {
            let age = age_years(ind, sim.current_day);
            let age_at_death = ind.death_day.map(|d| ((d - ind.birth_day).max(0) as f64) / 365.0);
            let ph = &ind.phenotype;
            // Strongest bonds first (by magnitude, so a deep rivalry ranks
            // alongside a close bond) -- psychology::process_bonding is the
            // sole writer of this map, see psychology.rs.
            let mut relationship_entries: Vec<(&String, &f64)> = ind.psychology.relationships.iter().collect();
            relationship_entries.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
            let relationships: Vec<Value> = relationship_entries
                .into_iter()
                .take(5)
                .map(|(other_id, bond)| {
                    json!({
                        "id": other_id,
                        "name": name_by_id.get(other_id.as_str()).cloned().unwrap_or_else(|| "Unnamed".to_string()),
                        "bond": (bond * 100.0).round() / 100.0,
                    })
                })
                .collect();
            json!({
                "id": ind.id,
                "name": ph.name.as_deref().unwrap_or("Unnamed"),
                "sex": ind.sex,
                "is_founder": ind.is_founder,
                "birth_year": ind.birth_day / 365,
                "death_year": ind.death_day.map(|d| d / 365),
                "age_at_death": age_at_death,
                "death_cause": ind.extra.get("death_cause").cloned().unwrap_or(Value::Null),
                "is_dead": ind.is_dead || !ind.alive,
                "intelligence": (ph.fluid_intelligence * 100.0).round() / 100.0,
                "age_years": (age * 10.0).round() / 10.0,
                "mental_state": ind.psychology.mental_state,
                "wellbeing": (ind.psychology.wellbeing * 100.0).round() / 100.0,
                "theory_of_mind": ind.psychology.theory_of_mind,
                "reputation": (ind.social.reputation * 100.0).round() / 100.0,
                "role": ind.extra.get("group_role").cloned().unwrap_or(Value::Null),
                "relationships": relationships,
                "inner_thought_log": ind.mind.extra.get("inner_thought_log").cloned().unwrap_or_else(|| json!([])),
            })
        })
        .collect();
    let population_history: Vec<Value> = checkpoints
        .iter()
        .map(|cp| {
            json!({
                "year": cp.sim_year,
                "day": cp.sim_day,
                "population": cp.population_count,
                "avg_age": cp.stats.get("avg_age").cloned(),
                "happiness_index": cp.stats.get("happiness_index").cloned(),
                "gini": cp.stats.get("gini").cloned(),
                "food_abundance": cp.stats.get("food_abundance").cloned(),
                "water_abundance": cp.stats.get("water_abundance").cloned(),
                "technologies": cp.tech_state,
                // Belief archetype ids ("belief_1".."belief_6") are opaque
                // engine bucketing keys, never a real-world religion name (see
                // belief.rs's own doc comment) -- a count is all a checkpoint-time
                // snapshot needs, unlike belief_timeline below which resolves each
                // one through belief_labels for display.
                "beliefs": cp.belief_state.as_array().map(|a| a.len()).unwrap_or(0),
                "centroid_x": cp.stats.get("centroid_x").cloned(),
                "centroid_y": cp.stats.get("centroid_y").cloned(),
                "season": cp.stats.get("season").cloned(),
                "weather": cp.stats.get("weather").cloned(),
                "deaths_total": cp.stats.get("deaths").cloned(),
                "births_total": cp.stats.get("births").cloned(),
                "sick_rate": cp.stats.get("sick_rate").cloned(),
                "word_count": cp.stats.get("word_count").cloned(),
                "max_language_stage": cp.stats.get("max_language_stage").cloned(),
                "avg_consciousness": cp.stats.get("avg_consciousness").cloned(),
                "qol_index": cp.stats.get("qol_index").cloned(),
                // Each checkpoint's own stats blob already carries this (see
                // derive_stats), letting GeneticDiversityPanel plot a real
                // trend across the population's history instead of only
                // ever showing a single current-moment snapshot.
                "genetic_diversity": cp.stats.get("genetic_diversity").cloned(),
            })
        })
        .collect();

    let tech_timeline: Vec<Value> = sim
        .discovered_techs
        .iter()
        .map(|tech| json!({ "name": tech, "year": sim.current_year, "day": sim.current_day }))
        .collect();
    // Cardinal rule (see belief.rs): a belief archetype id ("belief_1"..
    // "belief_6") is an opaque engine bucketing key, never a real-world
    // religion name -- the only player-facing name is whatever this
    // population's own language generated in `sim.belief_labels`
    // (belief::try_label_belief), and until a believer reaches proto-words
    // it stays unnamed. The raw `code` is exposed alongside `name` (it is
    // just an opaque tier id, safe to show) so the client can render its
    // own mechanically-derived description (see i18n.ts) instead of a bare
    // "Unnamed belief" placeholder.
    let belief_timeline: Vec<Value> = sim
        .discovered_beliefs
        .iter()
        .map(|belief_id| {
            json!({
                "name": sim.belief_labels.get(belief_id).cloned(),
                "code": belief_id,
                "year": sim.current_year,
                "day": sim.current_day,
            })
        })
        .collect();
    let art_timeline: Vec<Value> = sim
        .discovered_arts
        .iter()
        .map(|art| json!({ "name": art, "year": sim.current_year, "day": sim.current_day, "type": "art" }))
        .collect();
    // The engine tags every event's `importance` with a string ("low"/
    // "medium"/"high" -- see e.g. tick.rs/belief.rs/law.rs push sites), never
    // a number, so the previous `.as_i64() >= 3` check always defaulted to 1
    // and silently filtered out every single event; this section always
    // reported empty regardless of what happened in the simulation. Mapping
    // through `to_client_event` also gives each entry the `sim_day`/
    // `sim_year`/`event_type`/`description` shape the report/client already
    // expect (raw engine events only carry `type`/`day`).
    let notable_events: Vec<Value> = sim
        .events
        .iter()
        .filter(|event| matches!(event.get("importance").and_then(Value::as_str), Some("medium") | Some("high")))
        .map(|event| to_client_event(event, &sim))
        .collect();
    // Real band relocations logged by `tick::track_migration` (previously
    // this section was hardcoded to an empty array with no producer at all).
    let migration_history: Vec<Value> = sim
        .events
        .iter()
        .filter(|event| event.get("type").and_then(Value::as_str) == Some("migration"))
        .map(|event| {
            let day = event.get("day").and_then(Value::as_i64).unwrap_or(0);
            json!({
                "year": day / 365,
                "day": day,
                "distance_km": event.get("distance_km").cloned().unwrap_or(Value::Null),
                "reason": event.get("reason").cloned().unwrap_or(Value::Null),
                "from": event.get("from").cloned().unwrap_or(Value::Null),
                "to": event.get("to").cloned().unwrap_or(Value::Null),
                "food_abundance": event.get("food_abundance").cloned().unwrap_or(Value::Null),
                "water_abundance": event.get("water_abundance").cloned().unwrap_or(Value::Null),
                "season": event.get("season").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    let death_total = individuals.iter().filter(|i| i.get("is_dead").and_then(|v| v.as_bool()).unwrap_or(false)).count() as i64;
    let dead_individuals: Vec<&Value> = individuals.iter().filter(|i| i.get("is_dead").and_then(Value::as_bool).unwrap_or(false)).collect();
    let mut death_by_cause: serde_json::Map<String, Value> = serde_json::Map::new();
    for i in &dead_individuals {
        if let Some(cause) = i.get("death_cause").and_then(Value::as_str) {
            // Same normalization build_event_description already applies to
            // the narrative event log (see pascal_to_snake's own doc
            // comment) -- without it here, the exact same cause fragments
            // into two separate buckets depending on which engine set it
            // (ordinary mortality rolls format!("{cause:?}") a DeathCause
            // enum variant, PascalCase; microbiome/social-conflict deaths
            // set an already-lowercase literal), silently undercounting
            // both in this report's death-cause breakdown.
            let cause = pascal_to_snake(cause);
            let count = death_by_cause.get(&cause).and_then(Value::as_i64).unwrap_or(0);
            death_by_cause.insert(cause, json!(count + 1));
        }
    }
    let mut death_by_age_group: serde_json::Map<String, Value> = serde_json::Map::new();
    for i in &dead_individuals {
        if let Some(age) = i.get("age_at_death").and_then(Value::as_f64) {
            let key = if age < 1.0 {
                "infant_0_1"
            } else if age < 15.0 {
                "child_1_15"
            } else if age < 30.0 {
                "young_adult_15_30"
            } else if age < 50.0 {
                "adult_30_50"
            } else {
                "elder_50plus"
            };
            let count = death_by_age_group.get(key).and_then(Value::as_i64).unwrap_or(0);
            death_by_age_group.insert(key.to_string(), json!(count + 1));
        }
    }
    let avg_age_at_death = if dead_individuals.is_empty() {
        None
    } else {
        Some(dead_individuals.iter().filter_map(|i| i.get("age_at_death").and_then(Value::as_f64)).sum::<f64>() / dead_individuals.len() as f64)
    };
    let leading_cause_of_death = death_by_cause.iter().max_by_key(|(_, v)| v.as_i64().unwrap_or(0)).map(|(k, _)| k.clone());
    let total_ever = (individuals.len().max(1)) as f64;
    let infant_mortality_rate = death_by_age_group.get("infant_0_1").and_then(Value::as_i64).unwrap_or(0) as f64 / total_ever;
    let child_mortality_rate = death_by_age_group.get("child_1_15").and_then(Value::as_i64).unwrap_or(0) as f64 / total_ever;
    let total_migration_distance_km: f64 = migration_history.iter().filter_map(|m| m.get("distance_km").and_then(Value::as_f64)).sum();
    // Same cardinal-rule fix as belief_timeline above: never the raw archetype
    // string, only its opaque numeric code until a real label exists.
    let belief_names: Vec<String> = sim
        .discovered_beliefs
        .iter()
        .map(|belief_id| {
            sim.belief_labels.get(belief_id).cloned().unwrap_or_else(|| {
                let code = belief_id.strip_prefix("belief_").unwrap_or(belief_id);
                format!("Unnamed belief (#{code})")
            })
        })
        .collect();
    let summary = json!({
        "civilization_name": sim.name,
        "total_years": sim.current_year,
        "total_days": sim.current_day,
        "start_coordinates": { "latitude": sim.start_latitude, "longitude": sim.start_longitude },
        "biome": sim.world_state.biome,
        "total_individuals_ever": individuals.len(),
        "peak_population": population_history.iter().map(|c| c.get("population").and_then(|v| v.as_i64()).unwrap_or(0)).max().unwrap_or(0),
        "peak_population_year": population_history.iter().max_by_key(|c| c.get("population").and_then(|v| v.as_i64()).unwrap_or(0)).and_then(|c| c.get("year")).cloned(),
        "current_population": current_stats.get("population").cloned(),
        "technologies_discovered": tech_timeline.len(),
        "technology_list": sim.discovered_techs.clone(),
        "beliefs_formed": belief_timeline.len(),
        "belief_list": belief_names,
        "art_forms": art_timeline.len(),
        "language_stage": current_stats.get("max_language_stage").cloned(),
        "language_stage_name": "unknown",
        "vocabulary_size": current_stats.get("word_count").cloned(),
        "total_deaths": death_total,
        "avg_age_at_death_years": avg_age_at_death.map(|a| (a * 10.0).round() / 10.0),
        "infant_mortality_rate": (infant_mortality_rate * 1000.0).round() / 1000.0,
        "child_mortality_rate": (child_mortality_rate * 1000.0).round() / 1000.0,
        "leading_cause_of_death": leading_cause_of_death,
        "migration_events": migration_history.len(),
        "total_migration_distance_km": (total_migration_distance_km * 10.0).round() / 10.0,
        "epidemic_count": notable_events.iter().filter(|e| e.get("event_type").and_then(|v| v.as_str()) == Some("epidemic")).count(),
        "disaster_count": notable_events.iter().filter(|e| e.get("event_type").and_then(|v| v.as_str()) == Some("disaster")).count(),
        "final_happiness_index": current_stats.get("happiness_index").cloned(),
        "final_gini": current_stats.get("gini").cloned(),
        "final_qol_index": current_stats.get("qol_index").cloned(),
        "report_generated_at": format!("{:?}", std::time::SystemTime::now()),
    });

    Json(json!({
        "simulation": {
            "id": sim.id,
            "name": sim.name,
            "status": sim.status,
            "start_latitude": sim.start_latitude,
            "start_longitude": sim.start_longitude,
            "biome": sim.world_state.biome,
            "current_year": sim.current_year,
            "current_day": sim.current_day,
            "created_at": null,
            "intervened": sim.extra.get("intervened").and_then(|v| v.as_bool()).unwrap_or(false),
        },
        "summary": summary,
        "current_stats": current_stats,
        "population_history": population_history,
        "technology_timeline": tech_timeline,
        "belief_timeline": belief_timeline,
        "art_timeline": art_timeline,
        "migration_history": migration_history,
        "death_statistics": {
            "total": death_total,
            "avg_age_at_death": avg_age_at_death.map(|a| (a * 10.0).round() / 10.0),
            "by_cause": death_by_cause,
            "by_age_group": death_by_age_group,
        },
        "individuals": individuals,
        "notable_events": notable_events,
        "all_events": sim.events.clone(),
        "generated_at": format!("{:?}", std::time::SystemTime::now()),
    }))
    .into_response()
}

async fn get_metrics(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let row = match load_simulation(&state.backend, &id).await {
        Ok(Some(row)) => row,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let sim = row_to_state(&row);
    let is_warping = state.runtime.is_fast_forwarding(&id).await;
    let upload_paused = state.runtime.is_upload_paused(&id).await;
    let timing = state.runtime.tick_timing(&id).await;
    Json(json!({
        "current_day": sim.current_day,
        "current_year": sim.current_year,
        // Cheap scalars already tracked on every save -- avoids loading the
        // full `individuals` table just to serve this frequently-polled panel.
        "population": sim.world_state.alive_count.unwrap_or(0),
        "total_ever": row.population_count,
        "milestones_reached": sim.milestones,
        "speed_multiplier": sim.speed_multiplier.unwrap_or(1),
        "status": sim.status,
        "is_warping": is_warping,
        // True while pause_upload has been called and resume_upload hasn't
        // yet -- see runtime.rs's should_flush. The tick loop keeps
        // computing regardless; only the DB writes are skipped.
        "upload_paused": upload_paused,
        // Surfaces the rayon thread-pool sizing (main.rs's
        // configure_rayon_thread_pool) directly in the Performance panel --
        // added after a local-mode device screenshot showed the per-
        // individual parallel pass far slower than population size alone
        // would predict, root-caused to a cloud-only thread cap that was
        // accidentally also capping desktop/Android's local sim-server to 2
        // threads regardless of how many cores the device actually has.
        // cpu_cores_used reflects the *configured* pool size (current_num_threads
        // called outside any rayon worker reports the global pool's size),
        // not how many are busy on this exact tick.
        "cpu_cores_available": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        "cpu_cores_used": rayon::current_num_threads(),
        "tick_last_ms": timing.map(|t| t.last_ms),
        "tick_avg_ms": timing.map(|t| t.avg_ms),
        "tick_max_ms": timing.map(|t| t.max_ms),
        "tick_min_ms": timing.map(|t| t.min_ms),
        "ticks_per_second": timing.map(|t| t.ticks_per_second),
        // Breakdown of the last iteration, so a slow tick can be attributed
        // to DB/network latency vs. actual sim computation instead of
        // guessing. Per-batch totals (not per-day, unlike tick_*_ms above).
        "tick_load_ms": timing.map(|t| t.last_load_ms),
        "tick_compute_ms": timing.map(|t| t.last_compute_ms),
        "tick_save_ms": timing.map(|t| t.last_save_ms),
        "tick_upsert_ms": timing.map(|t| t.last_upsert_ms),
        // Sub-breakdown of tick_compute_ms by engine group (see
        // sim_core::PhaseTimings), so the Performance panel's "MODULE /
        // PERFORMANCE" block can show which part of the sim itself is slow.
        "tick_phase_setup_ms": timing.map(|t| t.last_phases.setup_ms),
        "tick_phase_economy_ms": timing.map(|t| t.last_phases.economy_ms),
        "tick_phase_consciousness_psychology_ms": timing.map(|t| t.last_phases.consciousness_psychology_ms),
        "tick_phase_language_naming_ms": timing.map(|t| t.last_phases.language_naming_ms),
        "tick_phase_microbiome_agent_ms": timing.map(|t| t.last_phases.microbiome_agent_ms),
        "tick_phase_movement_ms": timing.map(|t| t.last_phases.movement_ms),
        "tick_phase_observation_learning_ms": timing.map(|t| t.last_phases.observation_learning_ms),
        "tick_phase_tech_emergence_ms": timing.map(|t| t.last_phases.tech_emergence_ms),
        "tick_phase_reproduction_ms": timing.map(|t| t.last_phases.reproduction_ms),
        "tick_phase_mortality_roll_ms": timing.map(|t| t.last_phases.mortality_roll_ms),
        "tick_phase_microbiome_outbreak_ms": timing.map(|t| t.last_phases.microbiome_outbreak_ms),
        "tick_phase_group_pruning_ms": timing.map(|t| t.last_phases.group_pruning_ms),
        "tick_phase_belief_ms": timing.map(|t| t.last_phases.belief_ms),
        "tick_phase_culture_art_ms": timing.map(|t| t.last_phases.culture_art_ms),
        "tick_phase_social_ms": timing.map(|t| t.last_phases.social_ms),
        "tick_phase_law_ms": timing.map(|t| t.last_phases.law_ms),
        "tick_phase_architecture_conflict_ms": timing.map(|t| t.last_phases.architecture_conflict_ms),
        "tick_phase_astronomy_ms": timing.map(|t| t.last_phases.astronomy_ms),
        "tick_phase_trade_disease_ms": timing.map(|t| t.last_phases.trade_disease_ms),
        // Names in sim_core::TOGGLEABLE_ENGINES currently skipped for this
        // simulation (diagnostic-only -- see runtime.rs's disabled_engines).
        "disabled_engines": state.runtime.disabled_engines(&id).await.into_iter().collect::<Vec<_>>(),
        // Rust has no distinct heavy/light execution mode -- Rayon-based
        // data parallelism is always active regardless of population size.
        "heavy_mode": false,
        // Rayon-based data parallelism is always active in the Rust runtime --
        // there's no equivalent of the JS engine's optional worker-thread pool.
        "workers_disabled": false,
    }))
    .into_response()
}

async fn get_diagnostics(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let row = match load_simulation(&state.backend, &id).await {
        Ok(Some(row)) => row,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let sim = row_to_state(&row);
    let checkpoints = list_checkpoints(&state.backend, &id).await.unwrap_or_default();
    let latest_checkpoint = checkpoints.first();
    // None when this simulation has no active tick-loop session in this
    // process (never started here, or paused/completed already) -- the
    // Performance panel's startup-checks/error-log sections just render
    // their empty state then, same as before a simulation's first tick.
    let runtime_diag = state.runtime.diagnostics(&id).await;
    Json(json!({
        "status": "ok",
        "runtime": "rust",
        "sim_id": id,
        "running": sim.status.as_deref() == Some("running"),
        "population": sim.world_state.alive_count.unwrap_or(0),
        "current_day": sim.current_day,
        "current_year": sim.current_year,
        "checkpoint_count": checkpoints.len(),
        "event_count": sim.events.len(),
        "latest_checkpoint_world_state": latest_checkpoint.map(|cp| cp.world_state.clone()),
        "latest_checkpoint_art_state": latest_checkpoint.map(|cp| cp.art_state.clone()),
        "latest_checkpoint_groups": latest_checkpoint.map(|cp| cp.groups.clone()),
        "consecutive_errors": runtime_diag.as_ref().map(|d| d.consecutive_errors).unwrap_or(0),
        "startup": runtime_diag.as_ref().and_then(|d| d.startup.clone()),
        "error_log": runtime_diag.map(|d| d.error_log.into_iter().collect::<Vec<_>>()).unwrap_or_default(),
    }))
    .into_response()
}

/// Restores GET /:id/db-status, dropped entirely in the Node-to-Rust
/// migration (the route never existed here) -- the client's Performance
/// panel has always called this and silently swallowed the resulting 404,
/// so its whole "DATABASE STATUS" section has rendered nothing at all,
/// on every platform, since the rewrite.
///
/// `cloud_db` is only ever populated when this process itself IS the cloud
/// (Postgres backend) -- for a local (SQLite) instance, getting a live
/// cloud count would mean an extra authenticated HTTP round trip to
/// CLOUD_API_URL on every 5-second panel poll, for a number that's
/// secondary at best. Left at zero there rather than adding that traffic;
/// `upload_to_cloud`'s own flow is the source of truth for "is this synced
/// to the cloud" instead.
async fn get_db_status(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let sim = match load_full_state(&state.backend, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let counts = match db_status_counts(&state.backend, &id).await {
        Ok(counts) => counts,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    let cloud_db = if is_local_backend(&state) {
        json!({ "size_bytes": null, "cloud_checkpoints": 0, "live_snapshots": 0 })
    } else {
        // At most one row per (user, simulation) -- see live_snapshots'
        // primary key -- so this is really "is this sim live-syncing right
        // now" (0 or 1), not a count in any richer sense.
        json!({
            "size_bytes": counts.db_size_bytes,
            "cloud_checkpoints": counts.checkpoints,
            "live_snapshots": load_live_snapshot(&state.backend, &id).await.ok().flatten().is_some() as i32,
        })
    };

    Json(json!({
        "sim_db": {
            "size_bytes": counts.db_size_bytes,
            "individuals": { "total": counts.individuals_total, "alive": counts.individuals_alive },
            "checkpoints": counts.checkpoints,
            // Read from the already-loaded state, not a DB query -- see
            // db_status_counts' own doc comment on why simulation_events
            // (a table nothing ever inserts into) isn't the source here.
            "events": sim.events.len(),
            // No 1:1 Rust equivalent of Node's separate per-domain tables --
            // these are counted straight from the loaded state's own
            // top-level collections instead. "languages"/"conversations"/
            // "publications" had no simulation-level collection to draw
            // from at all (language is per-individual, not a simulation-
            // wide record; conversations/publications were old-Node-only
            // concepts with nothing carried over), so those stay 0 rather
            // than guessing at a substitute.
            "technologies": sim.discovered_techs.len(),
            "beliefs": sim.discovered_beliefs.len(),
            "languages": 0,
            "groups": sim.groups.len(),
            "conversations": 0,
            "publications": 0,
            "current_day": sim.current_day,
        },
        "cloud_db": cloud_db,
    }))
    .into_response()
}

async fn tick_simulation(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let mut sim = match load_full_state(&state.backend, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    let (report, phases) = advance_one_day(&mut sim);
    match save_existing_state(&state.backend, &sim).await {
        Ok(true) => {}
        Ok(false) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
    let _ = upsert_individuals(&state.backend, &sim, true).await;

    (StatusCode::OK, Json(json!({
        "report": report,
        "phases": phases,
        "state": sim
    })))
        .into_response()
}

async fn start_simulation(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    match update_simulation_fields(&state.backend, &id, Some("running"), None).await {
        Ok(true) => {
            state.runtime.start(state.backend.clone(), id.clone()).await;
            Json(json!({"message": "Simulation started"})).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

async fn pause_simulation(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    match update_simulation_fields(&state.backend, &id, Some("paused"), None).await {
        Ok(true) => {
            state.runtime.pause(&id).await;
            Json(json!({"message": "Simulation paused"})).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

/// Stops the tick loop's own per-batch DB writes (see runtime.rs's
/// `should_flush`) without stopping the simulation itself -- ticks keep
/// computing in memory, they just aren't persisted every batch. A no-op if
/// the simulation isn't currently running (no active tick-loop session to
/// toggle), same as `pause_simulation`'s own runtime.pause() call.
async fn pause_upload(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    state.runtime.pause_upload(&id).await;
    Json(json!({"message": "Upload paused", "upload_paused": true})).into_response()
}

/// Resumes per-batch DB writes. No separate "flush now" step -- see
/// runtime.rs's `resume_upload` doc comment for why the next tick loop
/// iteration already flushes everything accumulated while paused.
async fn resume_upload(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    state.runtime.resume_upload(&id).await;
    Json(json!({"message": "Upload resumed", "upload_paused": false})).into_response()
}

#[derive(Debug, Deserialize)]
struct SetDisabledEnginesRequest {
    disabled: Vec<String>,
}

/// Diagnostic-only: replaces the full set of engines the tick loop skips for
/// this simulation (see sim_core::TOGGLEABLE_ENGINES and runtime.rs's own
/// disabled_engines doc comment). Full-replace semantics, not incremental
/// add/remove -- simplest for a client that always knows its own current
/// toggle state and just POSTs the whole thing back. Rejects any name not in
/// the canonical list rather than silently ignoring a typo.
async fn set_disabled_engines(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<SetDisabledEnginesRequest>,
) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    if let Some(unknown) = payload.disabled.iter().find(|name| !sim_core::TOGGLEABLE_ENGINES.contains(&name.as_str())) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": format!("Unknown engine: {unknown}")}))).into_response();
    }
    let engines: std::collections::HashSet<String> = payload.disabled.into_iter().collect();
    state.runtime.set_disabled_engines(&id, engines.clone()).await;
    Json(json!({"disabled_engines": engines.into_iter().collect::<Vec<_>>()})).into_response()
}

async fn complete_simulation(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    match update_simulation_fields(&state.backend, &id, Some("completed"), None).await {
        Ok(true) => {
            state.runtime.pause(&id).await;
            Json(json!({"message": "Simulation completed"})).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SpeedRequest {
    speed_multiplier: i32,
}

async fn set_speed(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<SpeedRequest>,
) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    if !(1..=1000).contains(&payload.speed_multiplier) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "Speed must be an integer between 1 and 1000"}))).into_response();
    }
    match update_simulation_fields(&state.backend, &id, None, Some(payload.speed_multiplier)).await {
        Ok(true) => Json(json!({"speed_multiplier": payload.speed_multiplier})).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

async fn fast_forward(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<FastForwardRequest>,
) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    let target_year = payload.target_year;
    if target_year < 1 {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "target_year must be a positive integer"}))).into_response();
    }

    let row = match load_simulation(&state.backend, &id).await {
        Ok(Some(row)) => row,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    let sim = row_to_state(&row);
    let current_year = sim.current_day / 365;
    if target_year <= current_year {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": format!("Already past year {} (current: {})", target_year, current_year)}))).into_response();
    }

    let target_day = target_year * 365;
    state.runtime.fast_forward(&id, target_day).await;
    Json(json!({
        "message": format!("Fast-forwarding to year {} (day {})", target_year, target_day),
        "current_year": current_year,
        "target_year": target_year
    }))
    .into_response()
}

async fn cancel_fast_forward(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    state.runtime.cancel_fast_forward(&id).await;
    Json(json!({"message": "Fast-forward cancelled"})).into_response()
}

async fn terminate_simulation(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    // Stop (and wait for) the tick loop first -- otherwise a save already in
    // flight from the tick loop's own batch could race the mass-death write
    // below and clobber it, the same reasoning RuntimeManager::terminate's
    // own doc comment already covers for the delete case.
    state.runtime.terminate(&id).await;

    let mut sim = match load_full_state(&state.backend, &id).await {
        Ok(Some(sim)) => sim,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    // A user-terminated civilization ends the same way the engine already
    // models every other mass-mortality event, rather than deleting the
    // historical record outright -- see sim_core::terminate. Individuals,
    // events, and the simulation row all stay in the database exactly as any
    // other completed run's would; only the dedicated DELETE /:id route
    // (Dashboard's own separate, explicit "sil" action) removes anything.
    sim_core::terminate(&mut sim);

    match save_existing_state(&state.backend, &sim).await {
        Ok(true) => {}
        Ok(false) => return (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
    let _ = upsert_individuals(&state.backend, &sim, true).await;

    Json(json!({"message": "Simulation terminated"})).into_response()
}

async fn delete_simulation_route(State(state): State<AppState>, Path(id): Path<String>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if let Err(resp) = authorize_sim_access(&state, &headers, &id).await {
        return resp;
    }
    state.runtime.terminate(&id).await;
    match delete_simulation(&state.backend, &id).await {
        Ok(true) => Json(json!({"message": "Simulation deleted"})).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "simulation not found"}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct CreateSimulationRequest {
    pub name: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub founder_1_params: Value,
    #[serde(default)]
    pub founder_2_params: Value,
}

#[derive(Debug, Deserialize)]
struct FastForwardRequest {
    target_year: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{create_founder_for_simulation as create_founder, FOUNDER_GENOME_DEFAULTS};
    use crate::db::{load_bounded_tick_state_no_genealogy, load_genealogy_index, save_tick_progress};
    use axum::{body::{to_bytes, Body}, http::Request, Router};
    use serde_json::{json, Value};
    use sqlx::sqlite::SqlitePoolOptions;
    use tower::ServiceExt;

    fn test_token() -> String {
        let claims = crate::auth::Claims {
            id: "11111111-1111-1111-1111-111111111111".to_string(),
            username: "tester".to_string(),
            email: "tester@example.com".to_string(),
            role: "user".to_string(),
            exp: (chrono::Utc::now().timestamp() + 900) as usize,
        };
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(crate::auth::access_secret().as_bytes()),
        )
        .expect("sign test token")
    }

    async fn test_state() -> AppState {
        let db_path = std::env::temp_dir().join(format!("anatolia-sim-test-{}.db", uuid::Uuid::new_v4()));
        let db_path = db_path.to_string_lossy().replace('\\', "/");
        let sqlite_url = format!("sqlite:///{}?mode=rwc", db_path);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect(&sqlite_url)
            .await
            .expect("sqlite pool");
        let backend = crate::db::DbBackend::Sqlite(pool);
        crate::db::migrate(&backend).await.expect("migrate");
        AppState {
            backend,
            runtime: std::sync::Arc::new(crate::runtime::RuntimeManager::new()),
            rate_limiter: std::sync::Arc::new(crate::ratelimit::RateLimiter::new()),
        }
    }

    fn test_app(state: AppState) -> Router {
        Router::new()
            .route("/api/health", axum::routing::get(health))
            .route("/api/system/status", axum::routing::get(system_status))
            .nest("/api/simulations", simulation_routes())
            .nest(
                "/api/auth",
                Router::new()
                    .route(
                        "/wizard-defaults",
                        axum::routing::get(crate::auth::get_wizard_defaults_route).post(crate::auth::set_wizard_defaults_route),
                    )
                    .route("/register", axum::routing::post(crate::auth::register)),
            )
            .nest(
                "/api/analysis",
                Router::new()
                    .route("/local", axum::routing::post(crate::analysis::analyze_local))
                    .route("/local/hypothesis", axum::routing::post(crate::analysis::hypothesis_local))
                    .route("/:simId", axum::routing::post(crate::analysis::analyze))
                    .route("/:simId/hypothesis", axum::routing::post(crate::analysis::hypothesis)),
            )
            .nest(
                "/api/god",
                Router::new()
                    .route("/:simId/intervene", axum::routing::post(crate::god::intervene))
                    .route("/:simId/migrate-individual", axum::routing::post(crate::god::migrate_individual)),
            )
            .with_state(state)
    }

    async fn body_json(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
        serde_json::from_slice(&bytes).expect("json body")
    }

    #[tokio::test]
    async fn health_reports_rust_runtime() {
        let app = test_app(test_state().await);
        let response = app
            .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["runtime"], "rust");
        // Regression test: the client's auto-reload-on-deploy effect
        // (SimulationPage.tsx) compares this field across polls -- it was
        // silently absent before, so that effect could never detect a new
        // deploy at all.
        assert!(body["version"].is_string() && !body["version"].as_str().unwrap().is_empty());
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn create_tick_and_metrics_work() {
        let app = test_app(test_state().await);
        let create_payload = json!({
            "name": "Integration Sim",
            "latitude": 41.0,
            "longitude": 29.0,
            "founder_1_params": {},
            "founder_2_params": {}
        });

        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/simulations")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(create_payload.to_string()))
                    .unwrap(),
            )
            .await
            .expect("create response");
        assert_eq!(create_response.status(), StatusCode::CREATED);
        let created = body_json(create_response).await;
        let sim_id = created["id"].as_str().expect("simulation id").to_string();
        assert_eq!(created["current_day"], 0);
        assert_eq!(created["individuals"].as_array().map(|a| a.len()), Some(2));

        let tick_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/tick"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("tick response");
        assert_eq!(tick_response.status(), StatusCode::OK);
        let ticked = body_json(tick_response).await;
        assert_eq!(ticked["report"]["current_day"], 1);

        let metrics_response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/metrics"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        assert_eq!(metrics_response.status(), StatusCode::OK);
        let metrics = body_json(metrics_response).await;
        assert_eq!(metrics["current_day"], 1);
    }

    fn other_user_token() -> String {
        let claims = crate::auth::Claims {
            id: "22222222-2222-2222-2222-222222222222".to_string(),
            username: "other".to_string(),
            email: "other@example.com".to_string(),
            role: "user".to_string(),
            exp: (chrono::Utc::now().timestamp() + 900) as usize,
        };
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(crate::auth::access_secret().as_bytes()),
        )
        .expect("sign other-user test token")
    }

    fn admin_token() -> String {
        let claims = crate::auth::Claims {
            id: "33333333-3333-3333-3333-333333333333".to_string(),
            username: "root".to_string(),
            email: "root@example.com".to_string(),
            role: "admin".to_string(),
            exp: (chrono::Utc::now().timestamp() + 900) as usize,
        };
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(crate::auth::access_secret().as_bytes()),
        )
        .expect("sign admin test token")
    }

    // Regression test for the IDOR fix: every /:id-scoped handler used to
    // take no `headers` parameter at all and never called authenticate(),
    // so any caller who knew (or guessed, or scraped from list_live) a
    // simulation ID could read another account's full population/genome
    // data, tick/pause/terminate it, or delete it outright. This drives the
    // same handful of representative endpoints (read, control-flow, and
    // destructive) as a second, unrelated account and confirms every one of
    // them now responds exactly as if the simulation didn't exist -- 404,
    // not 403, so a non-owner can't even confirm the ID is real -- while
    // the rightful owner is unaffected.
    #[tokio::test]
    async fn a_user_cannot_read_or_control_another_users_simulation() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;
        let other_token = other_user_token();

        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {other_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("get response");
        assert_eq!(get_resp.status(), StatusCode::NOT_FOUND, "a non-owner must not be able to read the simulation");

        let pop_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/population"))
                    .header("authorization", format!("Bearer {other_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("population response");
        assert_eq!(pop_resp.status(), StatusCode::NOT_FOUND, "a non-owner must not be able to read population/genome data");

        let tick_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/tick"))
                    .header("authorization", format!("Bearer {other_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("tick response");
        assert_eq!(tick_resp.status(), StatusCode::NOT_FOUND, "a non-owner must not be able to advance the simulation");

        let terminate_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/terminate"))
                    .header("authorization", format!("Bearer {other_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("terminate response");
        assert_eq!(terminate_resp.status(), StatusCode::NOT_FOUND, "a non-owner must not be able to terminate the simulation");

        let delete_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {other_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("delete response");
        assert_eq!(delete_resp.status(), StatusCode::NOT_FOUND, "a non-owner must not be able to delete the simulation");

        // The rightful owner is unaffected by any of the above.
        let owner_resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("owner response");
        assert_eq!(owner_resp.status(), StatusCode::OK, "the actual owner must still be able to read their own simulation");
    }

    #[tokio::test]
    async fn an_admin_can_access_any_users_simulation() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("admin response");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn requests_with_no_token_are_rejected_not_leaked() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let resp = app
            .oneshot(Request::builder().uri(format!("/api/simulations/{sim_id}")).body(Body::empty()).unwrap())
            .await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // Simulations created before per-user ownership existed (see
    // list_live's own "orphaned records predating the ownership system"
    // comment) have no user_id at all -- authorize_sim_access must let any
    // authenticated caller through rather than treating a missing owner as
    // "nobody may ever access this again".
    #[tokio::test]
    async fn a_simulation_with_no_recorded_owner_stays_reachable() {
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        let mut sim = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        sim.user_id = None;
        save_existing_state(&backend, &sim).await.expect("save");

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", other_user_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(resp.status(), StatusCode::OK, "an orphaned simulation must stay reachable, not become permanently locked out");
    }

    /// live_sync_tick's own guardrails, testable against the SQLite test
    /// backend without a real network call (upsert_live_snapshot/
    /// load_live_snapshot both no-op on SQLite -- only the cloud/Postgres
    /// backend actually stores anything, so the full local-push ->
    /// cloud-store -> WatchPage-read roundtrip can only be verified against
    /// a real deployment, same as every other Postgres-only code path in
    /// this file).
    #[tokio::test]
    async fn live_sync_tick_requires_auth_and_a_real_simulation() {
        let app = test_app(test_state().await);

        let no_token = app
            .clone()
            .oneshot(Request::builder().method("POST").uri("/api/simulations/nonexistent-id/live-sync-tick").body(Body::empty()).unwrap())
            .await
            .expect("response");
        assert_eq!(no_token.status(), StatusCode::UNAUTHORIZED);

        let sim_id = create_simulation(&app).await;
        let bad_id = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/simulations/00000000-0000-0000-0000-000000000000/live-sync-tick")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(bad_id.status(), StatusCode::NOT_FOUND);

        // A real, existing local simulation clears auth + lookup and reaches
        // the outbound-HTTP step, which fails in this sandboxed test
        // environment (no network) -- confirms it doesn't panic or 4xx
        // before that point.
        let real_id = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/live-sync-tick"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(real_id.status(), StatusCode::BAD_GATEWAY, "expected the outbound cloud push to be attempted and fail (no network in tests), not rejected earlier");
    }

    #[tokio::test]
    async fn get_live_snapshot_requires_auth() {
        let app = test_app(test_state().await);
        let no_token = app
            .oneshot(Request::builder().uri("/api/simulations/live/some-id").body(Body::empty()).unwrap())
            .await
            .expect("response");
        assert_eq!(no_token.status(), StatusCode::UNAUTHORIZED);
    }

    // analyze/hypothesis used to load any sim_id off the path with no
    // authorize_sim_access call at all -- anyone who could guess or observe a
    // simulation id could read another user's live state through it and
    // spend the operator's Gemini quota doing so.
    #[tokio::test]
    async fn analyze_requires_auth() {
        let app = test_app(test_state().await);
        let no_token = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/analysis/some-id")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(no_token.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn hypothesis_requires_auth() {
        let app = test_app(test_state().await);
        let no_token = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/analysis/some-id/hypothesis")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"hypothesis": "test"}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(no_token.status(), StatusCode::UNAUTHORIZED);
    }

    // The WASM-local counterparts (client/src/wasmLocal/apiAdapter.ts's
    // handleAnalysis) have no simulation row to authorize against, but must
    // still reject an unauthenticated caller -- there's no ownership check
    // to fall back on, so a missing bearer token is the only gate.
    #[tokio::test]
    async fn analyze_local_requires_auth() {
        let app = test_app(test_state().await);
        let no_token = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/analysis/local")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(no_token.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn hypothesis_local_requires_auth() {
        let app = test_app(test_state().await);
        let no_token = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/analysis/local/hypothesis")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"hypothesis": "test"}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(no_token.status(), StatusCode::UNAUTHORIZED);
    }

    /// Every mortality path (ordinary death, birth complications, infection,
    /// natural disaster, intergroup conflict) must set is_dead on the
    /// individuals table *and* push an individual "death" event -- disaster
    /// and conflict deaths used to only push an aggregate event carrying a
    /// death *count*, and birth-complication deaths of a newborn/twin/triplet
    /// (as opposed to the mother) pushed no event at all. That left deaths
    /// correctly reflected in the population panel/top-bar counts (which
    /// read the individuals table) while silently missing from the event
    /// log (which reads these events) -- exercised through the real
    /// individuals-table + event-log write path (save_existing_state +
    /// upsert_individuals), same as every real mortality code path uses,
    /// rather than natural (unseeded rand::thread_rng()) mortality over a
    /// long tick loop. That used to run 9000 real ticks hoping a death
    /// happened to land inside the event log's own 1000-entry retention
    /// window by the time the run ended -- both genuinely flaky (a run can
    /// land zero deaths, or generate enough non-death events afterward to
    /// evict every death from the window) and slow (~4 minutes/run). This
    /// exercises the identical consistency invariant deterministically.
    #[tokio::test]
    async fn every_dead_individual_has_a_matching_death_event() {
        let state = test_state().await;
        let app = test_app(state.clone());
        let sim_id = create_simulation(&app).await;

        let mut sim = crate::db::load_full_state(&state.backend, &sim_id)
            .await
            .expect("load state")
            .expect("simulation exists");
        let victim = sim.individuals[0].clone();
        let death_day = sim.current_day;
        sim.individuals[0].alive = false;
        sim.individuals[0].is_dead = true;
        sim.individuals[0].death_day = Some(death_day);
        sim.events.push(json!({ "type": "death", "individual_id": victim.id, "cause": "TestInduced", "day": death_day, "importance": "medium" }));

        crate::db::save_existing_state(&state.backend, &sim).await.expect("save state");
        crate::db::upsert_individuals(&state.backend, &sim, true).await.expect("upsert individuals");

        let pop_dead_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/population?alive=false&limit=200"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("population(dead) response");
        let pop_dead = body_json(pop_dead_resp).await;
        let pop_dead_ids: std::collections::HashSet<String> = pop_dead.as_array().unwrap().iter().filter_map(|i| i["id"].as_str().map(String::from)).collect();
        assert!(pop_dead_ids.contains(&victim.id), "the killed individual should be listed by population?alive=false");

        let events_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/events"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("events response");
        let events = body_json(events_resp).await;
        let death_event_individual_ids: std::collections::HashSet<String> = events
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event_type"].as_str() == Some("death"))
            .filter_map(|e| e["data"]["individual_id"].as_str().map(String::from))
            .collect();
        assert!(death_event_individual_ids.contains(&victim.id), "the killed individual should have a matching death event in the log");
    }

    async fn create_simulation(app: &Router) -> String {
        let payload = json!({
            "name": "Lifecycle Sim",
            "latitude": 41.0,
            "longitude": 29.0,
            "founder_1_params": {},
            "founder_2_params": {}
        });
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/simulations")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .expect("create response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = body_json(response).await;
        body["id"].as_str().expect("simulation id").to_string()
    }

    #[tokio::test]
    async fn a_speed_change_mid_batch_survives_the_tick_loops_own_stale_save() {
        // Reproduces the exact race runtime.rs's tick loop can lose to: it
        // loads `state` at the top of a batch (speed_multiplier=1 here),
        // spends time in spawn_blocking computing that batch, and only then
        // calls save_tick_progress with that now-stale `state` object -- all
        // while a user's own speed-change request can land on the DB in
        // between. save_tick_progress is documented to never touch
        // status/speed_multiplier for exactly this reason; this test pins
        // that down without relying on wall-clock timing, unlike the manual
        // reproduction used to find this bug (a real runtime.rs session at
        // speed=200 processed ~2 days instead of ~500 in 3 real seconds,
        // because speed_multiplier kept reverting to 1 on SQLite before it
        // had a dedicated column).
        let state = test_state().await;
        let app = test_app(state.clone());
        let sim_id = create_simulation(&app).await;

        let stale_tick_state = crate::db::load_bounded_tick_state_no_genealogy(&state.backend, &sim_id)
            .await
            .expect("load tick state")
            .expect("simulation exists");
        assert_eq!(stale_tick_state.speed_multiplier, Some(1));

        crate::db::update_simulation_fields(&state.backend, &sim_id, None, Some(50))
            .await
            .expect("speed change");

        crate::db::save_tick_progress(&state.backend, &stale_tick_state)
            .await
            .expect("tick loop's own batch-end save");

        let after = crate::db::load_bounded_tick_state_no_genealogy(&state.backend, &sim_id)
            .await
            .expect("reload tick state")
            .expect("simulation exists");
        assert_eq!(after.speed_multiplier, Some(50));
    }

    #[tokio::test]
    async fn setting_speed_is_visible_to_the_next_tick_loop_load() {
        let state = test_state().await;
        let app = test_app(state.clone());
        let sim_id = create_simulation(&app).await;

        let speed_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/speed"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({"speed_multiplier": 50}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("speed response");
        assert_eq!(speed_response.status(), StatusCode::OK);
        let body = body_json(speed_response).await;
        assert_eq!(body["speed_multiplier"], 50);

        // What runtime.rs's tick loop itself loads on its very next
        // iteration -- this is the value that actually governs
        // batch_size/pacing, not just what the API echoes back.
        let tick_state = crate::db::load_bounded_tick_state_no_genealogy(&state.backend, &sim_id)
            .await
            .expect("load tick state")
            .expect("simulation exists");
        assert_eq!(tick_state.speed_multiplier, Some(50));

        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("get response");
        let get_body = body_json(get_response).await;
        assert_eq!(get_body["speed_multiplier"], 50);
    }

    #[tokio::test]
    async fn lifecycle_endpoints_update_state_and_terminate() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let start_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/start"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("start response");
        assert_eq!(start_response.status(), StatusCode::OK);

        let running = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("get running sim");
        let running_body = body_json(running).await;
        assert_eq!(running_body["status"], "running");

        let pause_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/pause"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("pause response");
        assert_eq!(pause_response.status(), StatusCode::OK);

        let paused = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("get paused sim");
        let paused_body = body_json(paused).await;
        assert_eq!(paused_body["status"], "paused");

        let terminate_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/terminate"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("terminate response");
        assert_eq!(terminate_response.status(), StatusCode::OK);

        // Terminating archives, it doesn't delete -- the record (and every
        // individual, marked dead with the termination's own cause) must
        // still be there afterward, distinct from the real DELETE route
        // Dashboard's own separate "sil" action uses.
        let after = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("get terminated sim");
        assert_eq!(after.status(), StatusCode::OK);
        let after_body = body_json(after).await;
        assert_eq!(after_body["status"], "completed");
        let individuals = after_body["individuals"].as_array().expect("individuals array");
        assert!(!individuals.is_empty());
        assert!(individuals.iter().all(|i| i["is_dead"] == true && i["alive"] == false));

        let events_response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/events?limit=100"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("events response");
        let events_body = body_json(events_response).await;
        let events = events_body.as_array().expect("events array");
        assert!(
            events.iter().any(|e| e["event_type"] == "death" && e["data"]["cause"] == "meteor_tsunami"),
            "expected at least one death event with the termination's own cause"
        );
    }

    // Regression test for a real bug this session's own 150-year deep-dive
    // testing surfaced: a population that dies out on its own (as opposed to
    // the manual "Sonlandır" terminate button) used to tick forever -- the
    // runtime loop never checked population state at all, so a naturally
    // extinct simulation just kept burning compute/DB writes indefinitely
    // while status stayed "running". See sim_core::mark_extinct.
    #[tokio::test]
    async fn a_naturally_extinct_population_is_auto_terminated_by_the_tick_loop() {
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        // Kill both founders directly, bypassing the engine's own mortality
        // roll -- the tick loop must notice this on its very next batch
        // regardless of how the population reached zero.
        let mut sim = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        for ind in sim.individuals.iter_mut() {
            ind.alive = false;
            ind.is_dead = true;
        }
        save_existing_state(&backend, &sim).await.expect("save");
        // The tick loop reads population from the `individuals` table, not
        // state_json (see load_bounded_tick_state_no_genealogy) -- the edit
        // above must be upserted there too for it to actually see anyone as
        // dead.
        upsert_individuals(&backend, &sim, true).await.expect("upsert");

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/start"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("start response");

        // Fast enough that the first batch (and its extinction check) lands
        // in well under a second instead of waiting through the default
        // speed=1 per-batch pacing delay -- see pausing_upload_stops_db_
        // progress_and_resuming_flushes_it's own comment for why.
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/speed"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({"speed_multiplier": 1000}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("speed response");

        let mut status = String::new();
        for _ in 0..80 {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/simulations/{sim_id}"))
                        .header("authorization", format!("Bearer {}", test_token()))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("get sim");
            let body = body_json(resp).await;
            status = body["status"].as_str().unwrap_or("").to_string();
            if status == "completed" {
                let events = body["events"].as_array().expect("events array");
                assert!(
                    events.iter().any(|e| e["type"] == "extinction" && e["reason"] == "population_zero"),
                    "expected an extinction event recording why the simulation ended"
                );
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("expected the tick loop to auto-terminate an extinct population, last status was {status:?}");
    }

    // Regression test for a real bug: a manual tick (or the runtime loop's
    // own periodic save, which goes through the same save_existing_state)
    // used to go through save_state's upsert, which would silently
    // recreate a simulation row that had just been deleted -- e.g. a delete
    // racing a still-in-flight tick. save_existing_state is a plain UPDATE,
    // so ticking a deleted simulation must fail loudly (404) instead of
    // resurrecting it.
    #[tokio::test]
    async fn ticking_a_deleted_simulation_does_not_resurrect_it() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let delete_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("delete response");
        assert_eq!(delete_response.status(), StatusCode::OK);

        let tick_after_delete = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/tick"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("tick response");
        assert_eq!(tick_after_delete.status(), StatusCode::NOT_FOUND);

        let still_gone = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("get after tick attempt");
        assert_eq!(still_gone.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn checkpoint_restore_roundtrip_works() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        async fn tick_once(app: Router, sim_id: String) {
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/simulations/{sim_id}/tick"))
                        .header("authorization", format!("Bearer {}", test_token()))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("tick response");
            assert_eq!(response.status(), StatusCode::OK);
        }

        tick_once(app.clone(), sim_id.clone()).await;

        let checkpoint_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/checkpoint"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("checkpoint response");
        assert_eq!(checkpoint_response.status(), StatusCode::OK);
        let checkpoint_body = body_json(checkpoint_response).await;
        let checkpoint_id = checkpoint_body["id"].as_str().expect("checkpoint id").to_string();
        assert_eq!(checkpoint_body["sim_day"], 1);

        tick_once(app.clone(), sim_id.clone()).await;

        let before_restore = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("get before restore");
        let before_restore_body = body_json(before_restore).await;
        assert_eq!(before_restore_body["current_day"], 2);

        let restore_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/restore/{checkpoint_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("restore response");
        assert_eq!(restore_response.status(), StatusCode::OK);

        let restored = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("get restored sim");
        let restored_body = body_json(restored).await;
        assert_eq!(restored_body["current_day"], 1);
        assert_eq!(restored_body["status"], "paused");
    }

    // runtime.rs's auto-checkpoint loop seeds its "how recently did we last
    // checkpoint" state from this query alone (not the full list_checkpoints
    // blob) -- covering it here since it has no dedicated HTTP route of its
    // own to exercise through the router.
    #[tokio::test]
    async fn latest_checkpoint_day_reflects_the_most_recent_checkpoint_and_none_when_absent() {
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        assert_eq!(
            crate::db::latest_checkpoint_day(&backend, &sim_id).await.expect("query"),
            None,
            "no checkpoint has been created yet"
        );

        for day in [91, 182, 45] {
            crate::db::insert_checkpoint(
                &backend,
                &uuid::Uuid::new_v4().to_string(),
                &sim_id,
                day,
                day / 365,
                1,
                json!({}),
                json!({}),
                json!([]),
                json!([]),
                json!([]),
                json!([]),
                json!({}),
            )
            .await
            .expect("insert checkpoint");
        }

        assert_eq!(
            crate::db::latest_checkpoint_day(&backend, &sim_id).await.expect("query"),
            Some(182),
            "must reflect the highest sim_day across all checkpoints, not insertion order"
        );
    }

    #[tokio::test]
    async fn fast_forward_and_cancel_work() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/fast-forward"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({ "target_year": 3 }).to_string()))
                    .unwrap(),
            )
            .await
            .expect("fast forward response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["target_year"], 3);

        let cancel = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/fast-forward/cancel"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("cancel response");
        assert_eq!(cancel.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_metrics_reports_cpu_core_counts() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/metrics"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        let body = body_json(response).await;
        assert!(body["cpu_cores_available"].as_u64().expect("cpu_cores_available") >= 1);
        assert!(body["cpu_cores_used"].as_u64().expect("cpu_cores_used") >= 1);
    }

    #[tokio::test]
    async fn get_metrics_reports_is_warping_only_while_fast_forwarding() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/start"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("start response");

        let before = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/metrics"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        assert_eq!(body_json(before).await["is_warping"], false);

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/fast-forward"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({ "target_year": 3 }).to_string()))
                    .unwrap(),
            )
            .await
            .expect("fast forward response");

        let during = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/metrics"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        assert_eq!(body_json(during).await["is_warping"], true);

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/fast-forward/cancel"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("cancel response");

        let after = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/metrics"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        assert_eq!(body_json(after).await["is_warping"], false);
    }

    #[tokio::test]
    async fn pause_upload_and_resume_upload_are_reflected_in_metrics_and_owner_scoped() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/start"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("start response");

        let before = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/metrics"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        assert_eq!(body_json(before).await["upload_paused"], false);

        let other_pause_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/pause-upload"))
                    .header("authorization", format!("Bearer {}", other_user_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("other-user pause-upload response");
        assert_eq!(other_pause_resp.status(), StatusCode::NOT_FOUND, "a non-owner must not be able to pause another user's upload");

        let pause_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/pause-upload"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("pause-upload response");
        assert_eq!(pause_resp.status(), StatusCode::OK);
        assert_eq!(body_json(pause_resp).await["upload_paused"], true);

        let during = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/metrics"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        assert_eq!(body_json(during).await["upload_paused"], true);

        let resume_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/resume-upload"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("resume-upload response");
        assert_eq!(resume_resp.status(), StatusCode::OK);
        assert_eq!(body_json(resume_resp).await["upload_paused"], false);

        let after = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/metrics"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        assert_eq!(body_json(after).await["upload_paused"], false);
    }

    #[tokio::test]
    async fn set_disabled_engines_is_reflected_in_metrics_rejects_unknown_names_and_is_owner_scoped() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/start"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("start response");

        let other_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/engines"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", other_user_token()))
                    .body(Body::from(json!({"disabled": ["law"]}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("other-user engines response");
        assert_eq!(other_resp.status(), StatusCode::NOT_FOUND, "a non-owner must not be able to toggle another user's engines");

        let bad_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/engines"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({"disabled": ["not_a_real_engine"]}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("bad engines response");
        assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST, "an unknown engine name must be rejected, not silently accepted");

        let set_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/engines"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({"disabled": ["law", "astronomy"]}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("set engines response");
        assert_eq!(set_resp.status(), StatusCode::OK);
        let mut set_body = body_json(set_resp).await["disabled_engines"].as_array().expect("disabled_engines array").iter().map(|v| v.as_str().unwrap_or("").to_string()).collect::<Vec<_>>();
        set_body.sort();
        assert_eq!(set_body, vec!["astronomy".to_string(), "law".to_string()]);

        let metrics_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/metrics"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("metrics response");
        let mut metrics_engines = body_json(metrics_resp).await["disabled_engines"].as_array().expect("disabled_engines array").iter().map(|v| v.as_str().unwrap_or("").to_string()).collect::<Vec<_>>();
        metrics_engines.sort();
        assert_eq!(metrics_engines, vec!["astronomy".to_string(), "law".to_string()]);

        // Full-replace semantics: posting a new set drops whatever wasn't
        // included, it doesn't merge with the previous one.
        let replace_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/engines"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({"disabled": []}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("clear engines response");
        assert_eq!(replace_resp.status(), StatusCode::OK);
        assert_eq!(body_json(replace_resp).await["disabled_engines"].as_array().expect("disabled_engines array").len(), 0);
    }

    #[tokio::test]
    /// Exercises the real, running runtime_loop (not just the toggle
    /// endpoints in isolation): pausing upload must actually stop the DB's
    /// current_day from advancing further (the loop keeps computing in
    /// memory regardless -- this only proves the *write* side stops), and
    /// resuming must flush that accumulated progress back to the DB without
    /// any separate "flush now" call.
    async fn pausing_upload_stops_db_progress_and_resuming_flushes_it() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/start"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("start response");

        // Fast enough that a whole 100-day batch (MAX_BATCH_SIZE) lands in
        // ~100ms wall time instead of ~1s, so this test doesn't need to wait
        // long to observe multiple batches.
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/speed"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({"speed_multiplier": 1000}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("speed response");

        async fn current_day(app: &Router, sim_id: &str) -> i64 {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/simulations/{sim_id}/metrics"))
                        .header("authorization", format!("Bearer {}", test_token()))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("metrics response");
            body_json(resp).await["current_day"].as_i64().unwrap_or(0)
        }

        let mut day_before_pause = 0;
        for _ in 0..40 {
            day_before_pause = current_day(&app, &sim_id).await;
            if day_before_pause > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
        assert!(day_before_pause > 0, "expected the tick loop to have made real progress before pausing");

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/pause-upload"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("pause-upload response");

        // Allow at most one more in-flight batch to land (paused was read at
        // the top of whatever iteration was already running), then the DB's
        // current_day must plateau -- proving further batches stop reaching
        // the DB while paused, even though the loop keeps ticking in memory.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let plateau_start = current_day(&app, &sim_id).await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let plateau_end = current_day(&app, &sim_id).await;
        assert_eq!(plateau_start, plateau_end, "current_day must not keep advancing in the DB while upload is paused");

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/resume-upload"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("resume-upload response");

        let mut resumed = false;
        for _ in 0..40 {
            if current_day(&app, &sim_id).await > plateau_end {
                resumed = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
        assert!(resumed, "expected current_day to advance again in the DB once upload resumed");
    }

    #[tokio::test]
    /// Population/stats/individual reads must stay live while upload is
    /// paused, not freeze at the last DB sync -- this is what
    /// RuntimeManager::live_state exists for (see get_stats, get_population,
    /// get_individual, and ws.rs's tick broadcast). Proven the same way as
    /// pausing_upload_stops_db_progress_and_resuming_flushes_it proves the
    /// DB plateaus: GET /:id/stats' "day" (sourced from live_state when
    /// present) must keep climbing well past GET /:id/metrics' "current_day"
    /// (always DB-sourced), and GET /:id/population must still resolve real
    /// individuals from memory instead of erroring or returning stale rows.
    async fn population_and_stats_stay_live_while_upload_is_paused() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/start"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("start response");

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/speed"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({"speed_multiplier": 1000}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("speed response");

        async fn metrics_current_day(app: &Router, sim_id: &str) -> i64 {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/simulations/{sim_id}/metrics"))
                        .header("authorization", format!("Bearer {}", test_token()))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("metrics response");
            body_json(resp).await["current_day"].as_i64().unwrap_or(0)
        }

        async fn stats_day(app: &Router, sim_id: &str) -> i64 {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/simulations/{sim_id}/stats"))
                        .header("authorization", format!("Bearer {}", test_token()))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("stats response");
            body_json(resp).await["day"].as_i64().unwrap_or(0)
        }

        let mut day_before_pause = 0;
        for _ in 0..40 {
            day_before_pause = metrics_current_day(&app, &sim_id).await;
            if day_before_pause > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
        assert!(day_before_pause > 0, "expected the tick loop to have made real progress before pausing");

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/pause-upload"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("pause-upload response");

        // live_state only refreshes once per LIVE_STATE_REFRESH_INTERVAL (1s
        // -- see runtime.rs), and up to one more batch can still land in the
        // DB right after pause-upload is requested (same race the sibling
        // test's own plateau check accounts for). A single fixed sleep
        // shorter than that refresh interval could catch a live_state
        // snapshot taken *before* that lingering batch's flush landed, so
        // poll instead of asserting after one sleep.
        let mut live_day = 0;
        let mut db_day = 0;
        let mut caught_up = false;
        for _ in 0..40 {
            db_day = metrics_current_day(&app, &sim_id).await;
            live_day = stats_day(&app, &sim_id).await;
            if live_day > db_day {
                caught_up = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(
            caught_up,
            "expected GET /stats' live-state day ({live_day}) to be well ahead of GET /metrics' frozen DB day ({db_day}) while paused"
        );

        let pop_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/population"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("population response");
        assert_eq!(pop_resp.status(), StatusCode::OK);
        let pop_body = body_json(pop_resp).await;
        let individuals = pop_body.as_array().expect("population response is an array");
        assert!(!individuals.is_empty(), "expected the live-state population view to still resolve real individuals while paused");
    }

    #[tokio::test]
    /// The old Node engine ran a one-time startup validation when a
    /// simulation's tick loop began, surfaced through GET /:id/diagnostics'
    /// `startup`/`error_log`/`consecutive_errors` fields -- lost in the
    /// Rust rewrite (the endpoint kept existing but stopped populating
    /// them), while the client's Performance panel kept expecting them,
    /// silently showing "not started yet" / "no errors" forever regardless
    /// of actual state. This is the happy-path half of restoring it: the
    /// panic-catching/auto-pause half can't be exercised from here without
    /// a test-only hook into sim-core to force a panic.
    async fn diagnostics_reports_startup_checks_once_running() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/start"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("start response");

        // The runtime loop's startup check runs as its very first action
        // after the spawned task is scheduled, not synchronously with the
        // /start response -- poll briefly rather than assume it's already
        // landed.
        let mut diag = json!({});
        for _ in 0..40 {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/simulations/{sim_id}/diagnostics"))
                        .header("authorization", format!("Bearer {}", test_token()))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("diagnostics response");
            diag = body_json(resp).await;
            if !diag["startup"].is_null() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        assert_eq!(diag["sim_id"], sim_id);
        assert_eq!(diag["running"], true);
        assert_eq!(diag["consecutive_errors"], 0);
        assert_eq!(diag["error_log"].as_array().map(|a| a.len()), Some(0));
        let checks = diag["startup"]["checks"].as_array().expect("startup checks present");
        assert!(!checks.is_empty(), "expected at least one startup check");
        let names: Vec<&str> = checks.iter().filter_map(|c| c["name"].as_str()).collect();
        assert!(names.contains(&"population"), "expected a population startup check, got {names:?}");
        assert!(checks.iter().all(|c| c["ok"] == true), "expected every startup check to pass: {checks:?}");
    }

    #[tokio::test]
    /// GET /:id/db-status never existed in the Rust port at all -- the
    /// client's Performance panel has called it since before the Rust
    /// migration and silently swallowed the 404, so its whole "DATABASE
    /// STATUS" section rendered nothing, on every platform, until now.
    async fn db_status_reports_real_counts() {
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/tick"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("tick response");

        let checkpoint_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/checkpoint"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("checkpoint response");
        assert_eq!(checkpoint_response.status(), StatusCode::OK);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/db-status"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("db-status response");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;

        // create_simulation (the test helper) seeds two founders and never
        // kills them within a single tick.
        assert_eq!(body["sim_db"]["individuals"]["total"], 2);
        assert_eq!(body["sim_db"]["individuals"]["alive"], 2);
        assert_eq!(body["sim_db"]["checkpoints"], 1);
        assert_eq!(body["sim_db"]["current_day"], 1);
        // The test harness's DbBackend is SQLite -- exercises the "no
        // server-wide database size" branch (Some for Postgres, None here).
        assert!(body["sim_db"]["size_bytes"].is_null());
        assert_eq!(body["cloud_db"]["live_snapshots"], 0);

        // Regression: "events" used to come from `SELECT COUNT(*) FROM
        // simulation_events`, a table nothing anywhere ever inserts into --
        // always 0 regardless of how many events actually happened. A bare
        // tick on two fresh founders may not itself produce any (no births/
        // deaths/milestones expected in a single day), so push a synthetic
        // one directly onto the state and re-save -- if this were still
        // reading the dead table, it would keep reporting 0 right through
        // this.
        let mut sim = load_full_state(&backend, &sim_id).await.expect("load state").expect("simulation exists");
        let events_before = sim.events.len();
        sim.events.push(json!({ "type": "test_event", "day": sim.current_day }));
        assert!(save_existing_state(&backend, &sim).await.expect("save state"), "simulation should still exist");

        let resp2 = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/db-status"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("second db-status response");
        let body2 = body_json(resp2).await;
        assert_eq!(body2["sim_db"]["events"], json!(events_before + 1));
    }

    #[test]
    /// Regression: this table used to store one value per locus and apply it
    /// to *both* alleles (`json!({ "a1": value, "a2": value })`), silently
    /// collapsing AGENTS.md's own documented, intentionally-heterozygous
    /// founder defaults (e.g. `FOXP2_01(0.90/0.88)`) into homozygous pairs
    /// for 14 of the 19 loci. Locks the table itself against a regression to
    /// that shape, independent of the create_founder wiring test below.
    fn founder_genome_defaults_match_the_documented_heterozygous_values() {
        let expected: &[(&str, f64, f64)] = &[
            ("OXTR_01", 0.82, 0.82), ("AVPR1A_01", 0.78, 0.78), ("IMMUNE_01", 0.88, 0.85), ("IMMUNE_02", 0.85, 0.82),
            ("TERT_01", 0.85, 0.85), ("APOE_01", 0.80, 0.80), ("FOXP2_01", 0.90, 0.88), ("CNTNAP2_01", 0.82, 0.80),
            ("BDNF_01", 0.80, 0.78), ("COMT_01", 0.78, 0.76), ("DTNBP1_01", 0.80, 0.78), ("NRXN1_01", 0.82, 0.80),
            ("SHANK3_01", 0.80, 0.78), ("RELN_01", 0.80, 0.78), ("DRD4_01", 0.75, 0.75), ("DRD2_01", 0.75, 0.72),
            ("STRENGTH_01", 0.78, 0.75), ("ACTN3_01", 0.76, 0.74), ("FSHR_01", 0.70, 0.68),
        ];
        assert_eq!(FOUNDER_GENOME_DEFAULTS, expected);
        assert!(
            FOUNDER_GENOME_DEFAULTS.iter().filter(|(_, a1, a2)| (a1 - a2).abs() > 1e-9).count() >= 14,
            "most founder loci are meant to be heterozygous (a1 != a2), not a single value duplicated onto both alleles"
        );
    }

    #[tokio::test]
    /// Wires the same table through the actual `create_founder` (routes.rs)
    /// helper the wizard/simulation-creation endpoint calls, confirming the
    /// asymmetric a1/a2 values genuinely reach the founder's genome rather
    /// than getting flattened somewhere in the founder_params/sim_core hop.
    async fn create_founder_applies_asymmetric_alleles_not_a_single_homozygous_value() {
        let individual = create_founder(&json!({}), "female", 0.0, 0.0, 25, true, "sim1", None, None);
        let foxp2 = &individual.genome["FOXP2_01"];
        assert_eq!(foxp2.allele1.value, Some(0.90));
        assert_eq!(foxp2.allele2.value, Some(0.88));
        let bdnf = &individual.genome["BDNF_01"];
        assert_eq!(bdnf.allele1.value, Some(0.80));
        assert_eq!(bdnf.allele2.value, Some(0.78));
    }

    #[tokio::test]
    /// Regression: check_reproduction's own doc comment names Node's
    /// immediate post-conception urge reset (mother -> 0, father -= 0.7) as
    /// intentional, but nothing in tick.rs actually applied it -- a father's
    /// mating_urge was completely untouched by having just sired a child, so
    /// nothing curbed him from immediately seeking another conception.
    async fn conceiving_a_child_resets_the_mothers_urge_and_cuts_the_fathers() {
        let mut state = sim_core::SimulationState::default();
        let mut mother = sim_core::create_founder(&json!({ "sex": "female", "ageYears": 25, "x": 0, "y": 0 }));
        let mut father = sim_core::create_founder(&json!({ "sex": "male", "ageYears": 25, "x": 0, "y": 0 }));
        mother.extra.insert("mating_urge".to_string(), json!(0.9));
        father.extra.insert("mating_urge".to_string(), json!(0.9));
        state.individuals = vec![mother, father];

        // conception_probability is randomized and age/urge-gated -- run
        // enough days that a close, fertile founder pair conceives, matching
        // reproduction.rs's own "eventually conceives" test pattern. A
        // conception shows up as a mother with health.pregnancy set (the
        // pending-birth queue only becomes a population member once the
        // pregnancy reaches term, days later).
        let mut conceived = false;
        for _ in 0..1000 {
            sim_core::advance_one_day(&mut state);
            if state.individuals.iter().any(|i| i.sex == "female" && i.health.pregnancy.is_some()) {
                conceived = true;
                break;
            }
        }
        assert!(conceived, "a close, fertile founder pair should eventually conceive within 1000 days");

        let mother = state.individuals.iter().find(|i| i.sex == "female").unwrap();
        let father = state.individuals.iter().find(|i| i.sex == "male").unwrap();
        let mother_urge = mother.extra.get("mating_urge").and_then(Value::as_f64).unwrap_or(-1.0);
        let father_urge = father.extra.get("mating_urge").and_then(Value::as_f64).unwrap_or(-1.0);
        assert!(mother_urge.abs() < 1e-9, "mother's urge should reset to exactly 0 right after conceiving, got {mother_urge}");
        // Urge climbs toward 1.0 (its ceiling) over however many days it took
        // this pair to conceive, so the father's own -0.7 reset can land
        // anywhere in [0.0, 0.3] depending on how close to that ceiling he
        // was the instant conception happened -- what matters is that it
        // dropped by a full 0.7 from wherever it was, not a specific value.
        assert!(father_urge <= 0.3 + 1e-6, "father's urge should drop sharply (-0.7) right after siring a child, got {father_urge}");
    }

    #[tokio::test]
    /// Regression: `upsert_individuals`'s dead-individual grace window used
    /// to be a flat 3 days, checked against `state.current_day` *after* a
    /// whole runtime_loop batch (up to MAX_BATCH_SIZE = 100 days) had
    /// already run. An individual who died on a batch's first day was only
    /// ever considered for upsert once, at that batch's end -- by which
    /// point `current_day - death_day` could already exceed the grace
    /// window, so their death silently never reached the `individuals`
    /// table. `stats.deaths`/`population?alive=false` (sourced from that
    /// table) then undercounted real deaths against the event log (a
    /// separate, always-correctly-persisted path) -- e.g. reported live:
    /// 2 vs. 62. Simulates exactly that batch shape directly against
    /// `upsert_individuals` rather than relying on random mortality.
    async fn dead_individual_from_early_in_a_large_batch_is_still_upserted() {
        let state = test_state().await;
        let app = test_app(state.clone());
        let sim_id = create_simulation(&app).await;

        let mut sim = crate::db::load_full_state(&state.backend, &sim_id)
            .await
            .expect("load state")
            .expect("simulation exists");
        assert_eq!(sim.individuals.len(), 2, "create_simulation seeds two founders");

        // Kill one founder on what stands in for the first day of a big
        // batch, then fast-forward current_day the way runtime_loop does
        // after running the whole batch -- MAX_BATCH_SIZE days later.
        let death_day = sim.current_day;
        sim.individuals[0].alive = false;
        sim.individuals[0].is_dead = true;
        sim.individuals[0].death_day = Some(death_day);
        sim.current_day = death_day + crate::runtime::MAX_BATCH_SIZE as i32;

        crate::db::upsert_individuals(&state.backend, &sim, true).await.expect("upsert individuals");

        let reloaded = crate::db::load_full_state(&state.backend, &sim_id)
            .await
            .expect("reload state")
            .expect("simulation still exists");
        let dead_id = &sim.individuals[0].id;
        let persisted = reloaded.individuals.iter().find(|i| &i.id == dead_id).expect("dead individual persisted");
        assert!(persisted.is_dead, "death from early in a large batch must still reach the individuals table");
        assert!(!persisted.alive);
    }

    #[tokio::test]
    /// Reproduces the real production failure: upload paused (or any other
    /// reason) can let a batch go by without an upsert for far longer than
    /// DEAD_UPSERT_GRACE_DAYS spans, so a parent can die and age out of that
    /// window without ever being upserted even once -- then their still-
    /// eligible child's parent_id points at a DB row that was never
    /// written, and every future upsert_individuals call fails with a
    /// foreign key violation, forever. The fix transitively includes any
    /// referenced parent still resolvable in `state.individuals` (which
    /// never drops anyone once loaded -- see tick.rs's
    /// strip_dead_individual_if_due), regardless of their own grace-window
    /// eligibility. The other half of the fix -- recovering when a parent
    /// *isn't* resolvable anywhere (an already-lost ancestor) -- can't be
    /// exercised here since this test harness's SQLite backend never
    /// declares or enforces the individuals table's foreign keys the way
    /// production's Postgres does; see db.rs's own
    /// sanitize_dangling_parents/is_foreign_key_violation_signal unit tests
    /// for that path.
    async fn upsert_transitively_persists_a_long_dead_parent_so_a_childs_foreign_key_resolves() {
        let state = test_state().await;
        let app = test_app(state.clone());
        let sim_id = create_simulation(&app).await;

        let mut sim = crate::db::load_full_state(&state.backend, &sim_id)
            .await
            .expect("load state")
            .expect("simulation exists");

        // A parent who has never been upserted (unlike the two founders,
        // which create_simulation already upserted at creation time), dead
        // long enough ago that the plain grace-window filter alone would
        // exclude them from this upsert.
        let mut phantom_parent = alive_individual();
        phantom_parent.id = uuid::Uuid::new_v4().to_string();
        phantom_parent.alive = false;
        phantom_parent.is_dead = true;
        phantom_parent.death_day = Some(0);

        let mut child = alive_individual();
        child.id = uuid::Uuid::new_v4().to_string();
        child.parent_1_id = Some(phantom_parent.id.clone());

        // DEAD_UPSERT_GRACE_DAYS is MAX_BATCH_SIZE (100) + 7 = 107; comfortably
        // past that so the plain grace-window filter alone would exclude
        // phantom_parent.
        sim.current_day = 500;
        sim.individuals.push(phantom_parent.clone());
        sim.individuals.push(child.clone());

        crate::db::upsert_individuals(&state.backend, &sim, true)
            .await
            .expect("upsert must succeed by including the referenced parent, not fail with a foreign key violation");

        let reloaded = crate::db::load_full_state(&state.backend, &sim_id)
            .await
            .expect("reload state")
            .expect("simulation still exists");
        assert!(
            reloaded.individuals.iter().any(|i| i.id == phantom_parent.id),
            "the never-before-upserted parent must be persisted alongside their eligible child"
        );
        let persisted_child = reloaded.individuals.iter().find(|i| i.id == child.id).expect("child persisted");
        assert_eq!(persisted_child.parent_1_id.as_deref(), Some(phantom_parent.id.as_str()));
    }

    #[tokio::test]
    async fn upsert_with_include_ancestors_false_skips_the_transitive_parent_walk() {
        // Companion to the test above: `include_ancestors: false` (what
        // runtime.rs's hot loop uses once full_resync_needed clears) must
        // skip the expensive transitive walk entirely -- a never-before-
        // upserted, long-dead parent outside the grace window must NOT be
        // pulled in just because their child is eligible. This is the
        // trade-off the flag exists for: runtime.rs only ever passes `false`
        // once it already knows (from an earlier `true` pass) that every
        // such parent still referenced by someone eligible is already
        // persisted, so this scenario (a parent that was *never* persisted)
        // is deliberately out of scope for the `false` path -- it's exactly
        // what the caller is trusted not to hit.
        let state = test_state().await;
        let app = test_app(state.clone());
        let sim_id = create_simulation(&app).await;

        let mut sim = crate::db::load_full_state(&state.backend, &sim_id)
            .await
            .expect("load state")
            .expect("simulation exists");

        let mut phantom_parent = alive_individual();
        phantom_parent.id = uuid::Uuid::new_v4().to_string();
        phantom_parent.alive = false;
        phantom_parent.is_dead = true;
        phantom_parent.death_day = Some(0);

        let mut child = alive_individual();
        child.id = uuid::Uuid::new_v4().to_string();
        child.parent_1_id = Some(phantom_parent.id.clone());

        sim.current_day = 500;
        sim.individuals.push(phantom_parent.clone());
        sim.individuals.push(child.clone());

        crate::db::upsert_individuals(&state.backend, &sim, false).await.expect("upsert (cheap path) itself must still succeed");

        let reloaded = crate::db::load_full_state(&state.backend, &sim_id)
            .await
            .expect("reload state")
            .expect("simulation still exists");
        assert!(
            !reloaded.individuals.iter().any(|i| i.id == phantom_parent.id),
            "include_ancestors=false must not transitively persist a parent outside the eligible window"
        );
        assert!(
            reloaded.individuals.iter().any(|i| i.id == child.id),
            "the eligible (alive) child itself must still be persisted"
        );
    }

    #[tokio::test]
    /// The wizard's "remember my last inputs" convenience used to live only
    /// in localStorage -- iOS Safari's ITP caps script-writable storage to
    /// 7 days of no top-level visit and silently wipes it, which surfaced
    /// as "my wizard forgets everything after a deploy" (the user's actual
    /// visits to the site were infrequent enough to cross that window,
    /// something Chrome has no equivalent policy for). Account-scoped
    /// server storage sidesteps that entirely.
    async fn wizard_defaults_round_trip_and_default_to_null() {
        let state = test_state().await;
        let user = crate::db::create_or_update_user(&state.backend, "TESTCODE", "wizard-defaults-tester@example.com", "Test", "User", "", "hash", "user", true)
            .await
            .expect("create test user")
            .expect("user row");
        let claims = crate::auth::Claims {
            id: user.id.clone(),
            username: "wizard-defaults-tester".to_string(),
            email: user.email.clone(),
            role: "user".to_string(),
            exp: (chrono::Utc::now().timestamp() + 900) as usize,
        };
        let token = jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &jsonwebtoken::EncodingKey::from_secret(crate::auth::access_secret().as_bytes())).expect("sign token");

        let app = test_app(state);

        let before = app
            .clone()
            .oneshot(Request::builder().uri("/api/auth/wizard-defaults").header("authorization", format!("Bearer {token}")).body(Body::empty()).unwrap())
            .await
            .expect("get response");
        assert_eq!(before.status(), StatusCode::OK);
        assert!(body_json(before).await["defaults"].is_null(), "nothing saved yet -- should be null, not an error");

        let payload = json!({ "name": "Anatolia", "latitude": "39.9", "longitude": "32.8", "f1": { "sex": "male" }, "f2": { "sex": "female" } });
        let post = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/wizard-defaults")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .expect("post response");
        assert_eq!(post.status(), StatusCode::OK);

        let after = app
            .oneshot(Request::builder().uri("/api/auth/wizard-defaults").header("authorization", format!("Bearer {token}")).body(Body::empty()).unwrap())
            .await
            .expect("get response");
        assert_eq!(body_json(after).await["defaults"], payload);
    }

    #[tokio::test]
    async fn health_endpoint_handles_concurrent_requests() {
        let app = test_app(test_state().await);
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..12 {
            let clone = app.clone();
            tasks.spawn(async move {
                let response = clone
                    .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
                    .await
                    .expect("health response");
                assert_eq!(response.status(), StatusCode::OK);
                let body = body_json(response).await;
                assert_eq!(body["runtime"], "rust");
            });
        }

        while let Some(result) = tasks.join_next().await {
            result.expect("task should not panic");
        }
    }

    // ── derive_stats (ported from SimulationEngine.computeStats) ────────

    fn stats_sim(individuals: Vec<Individual>) -> SimulationState {
        SimulationState { individuals, current_day: 0, ..Default::default() }
    }

    fn alive_individual() -> Individual {
        Individual { alive: true, is_dead: false, sex: "male".to_string(), ..Default::default() }
    }

    #[test]
    fn derive_stats_returns_correct_population_count() {
        let sim = stats_sim(vec![alive_individual(), alive_individual(), alive_individual()]);
        let stats = derive_stats(&sim);
        assert_eq!(stats["population"], 3);
    }

    // ── compute_legends ──────────────────────────────────────────────────

    #[test]
    fn highest_consciousness_legend_picks_the_real_maximum() {
        let mut low = alive_individual();
        low.id = "low".to_string();
        low.mind.consciousness = 0.1;
        let mut high = alive_individual();
        high.id = "high".to_string();
        high.mind.consciousness = 0.9;
        let sim = stats_sim(vec![low, high]);
        let legends = compute_legends(&sim);
        assert_eq!(legends["highest_consciousness"]["id"], "high");
        assert_eq!(legends["highest_consciousness"]["value"], 0.9);
    }

    #[test]
    fn most_children_legend_is_absent_when_nobody_has_any() {
        let sim = stats_sim(vec![alive_individual(), alive_individual()]);
        let legends = compute_legends(&sim);
        assert_eq!(legends["most_children"], Value::Null, "no individual has children, so there is no record holder");
    }

    #[test]
    fn most_children_legend_picks_the_individual_with_the_most() {
        let mut parent_of_one = alive_individual();
        parent_of_one.id = "one-child".to_string();
        parent_of_one.social.children_ids = vec!["c1".to_string()];
        let mut parent_of_three = alive_individual();
        parent_of_three.id = "three-children".to_string();
        parent_of_three.social.children_ids = vec!["c1".to_string(), "c2".to_string(), "c3".to_string()];
        let sim = stats_sim(vec![parent_of_one, parent_of_three]);
        let legends = compute_legends(&sim);
        assert_eq!(legends["most_children"]["id"], "three-children");
        assert_eq!(legends["most_children"]["value"], 3);
    }

    #[test]
    fn longest_lived_legend_uses_lifespan_in_years_not_raw_death_day() {
        // A founder who died early (small death_day) but lived a long life
        // must beat a much-later-born descendant who died young.
        let mut short_lived_descendant = alive_individual();
        short_lived_descendant.id = "descendant".to_string();
        short_lived_descendant.is_dead = true;
        short_lived_descendant.birth_day = 9000;
        short_lived_descendant.death_day = Some(9000 + 365 * 5); // lived 5 years
        let mut long_lived_founder = alive_individual();
        long_lived_founder.id = "founder".to_string();
        long_lived_founder.is_dead = true;
        long_lived_founder.birth_day = -365 * 60;
        long_lived_founder.death_day = Some(365 * 20); // lived 80 years
        let sim = stats_sim(vec![short_lived_descendant, long_lived_founder]);
        let legends = compute_legends(&sim);
        assert_eq!(legends["longest_lived"]["id"], "founder");
        assert_eq!(legends["longest_lived"]["value"], 80);
    }

    #[test]
    fn longest_lived_legend_ignores_the_living() {
        let mut alive = alive_individual();
        alive.id = "alive".to_string();
        alive.birth_day = -365 * 200; // would "win" on raw age if not excluded
        let sim = stats_sim(vec![alive]);
        let legends = compute_legends(&sim);
        assert_eq!(legends["longest_lived"], Value::Null, "only the dead have a final lifespan to record");
    }

    #[test]
    fn most_technologies_legend_counts_discovery_events_by_discoverer_id() {
        let mut prolific = alive_individual();
        prolific.id = "prolific".to_string();
        let mut sim = stats_sim(vec![prolific.clone(), alive_individual()]);
        sim.events = vec![
            json!({ "type": "discovery", "tech_id": "fire_making", "discoverer_id": "prolific", "discovery_day": 1 }),
            json!({ "type": "discovery", "tech_id": "stone_tools", "discoverer_id": "prolific", "discovery_day": 2 }),
            json!({ "type": "birth", "individual_id": "prolific", "day": 0 }),
        ];
        let legends = compute_legends(&sim);
        assert_eq!(legends["most_technologies"]["id"], "prolific");
        assert_eq!(legends["most_technologies"]["value"], 2);
    }

    #[test]
    fn avg_cultural_prestige_is_zero_with_no_groups() {
        let sim = stats_sim(vec![alive_individual()]);
        assert_eq!(sim.groups.len(), 0);
        let stats = derive_stats(&sim);
        assert_eq!(stats["avg_cultural_prestige"], 0.0);
    }

    #[test]
    fn avg_cultural_prestige_reflects_group_culture_size() {
        let mut sim = stats_sim(vec![alive_individual()]);
        sim.groups = vec![
            json!({ "id": "g1", "culture": ["shared_greeting", "mourning_ritual"] }),
            json!({ "id": "g2", "culture": [] }),
        ];
        let stats = derive_stats(&sim);
        let prestige = stats["avg_cultural_prestige"].as_f64().unwrap();
        assert!(prestige > 0.0 && prestige < 1.0);
    }

    #[test]
    fn avg_consciousness_is_between_zero_and_one() {
        let mut a = alive_individual();
        a.mind.consciousness = 0.1;
        let mut b = alive_individual();
        b.mind.consciousness = 0.3;
        let stats = derive_stats(&stats_sim(vec![a, b]));
        let avg = stats["avg_consciousness"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&avg));
    }

    #[test]
    fn qol_index_is_between_zero_and_one() {
        let stats = derive_stats(&stats_sim(vec![alive_individual()]));
        let qol = stats["qol_index"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&qol));
    }

    #[test]
    fn gini_reflects_actual_wealth_inequality_not_a_hardcoded_zero() {
        // H-09 regression: compute_economic_stats existed fully implemented
        // and unit-tested but was never called from anywhere in the running
        // server, so derive_stats's "gini" field always silently fell back
        // to a hardcoded 0.0 regardless of actual inventory distribution.
        let mut poor = alive_individual();
        poor.inventory.insert("food".to_string(), 0.0);
        let mut poor2 = alive_individual();
        poor2.inventory.insert("food".to_string(), 0.0);
        let mut rich = alive_individual();
        rich.inventory.insert("food".to_string(), 1000.0);
        let stats = derive_stats(&stats_sim(vec![poor, poor2, rich]));
        assert!(stats["gini"].as_f64().unwrap() > 0.1, "a wildly unequal population should report a real, nonzero Gini coefficient");
    }

    #[test]
    fn equal_wealth_yields_a_near_zero_gini() {
        let mut a = alive_individual();
        a.inventory.insert("food".to_string(), 10.0);
        let mut b = alive_individual();
        b.inventory.insert("food".to_string(), 10.0);
        let stats = derive_stats(&stats_sim(vec![a, b]));
        assert!(stats["gini"].as_f64().unwrap() < 1e-4);
    }

    #[test]
    fn happiness_index_reflects_actual_wellbeing_and_stress_not_a_hardcoded_default() {
        let mut happy = alive_individual();
        happy.psychology.wellbeing = 1.0;
        happy.psychology.stress_level = 0.0;
        let stats = derive_stats(&stats_sim(vec![happy]));
        assert!(stats["happiness_index"].as_f64().unwrap() > 0.9);

        let mut miserable = alive_individual();
        miserable.psychology.wellbeing = 0.0;
        miserable.psychology.stress_level = 1.0;
        let stats = derive_stats(&stats_sim(vec![miserable]));
        assert!(stats["happiness_index"].as_f64().unwrap() < 0.1);
    }

    #[test]
    fn mean_stress_is_exposed_and_reflects_the_populations_real_stress_level() {
        let mut stressed = alive_individual();
        stressed.psychology.stress_level = 0.8;
        let stats = derive_stats(&stats_sim(vec![stressed]));
        assert!((stats["mean_stress"].as_f64().unwrap() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn mental_state_distribution_reflects_the_populations_actual_states() {
        let mut anxious = alive_individual();
        anxious.psychology.mental_state = "anxious".to_string();
        let mut content = alive_individual();
        content.psychology.mental_state = "content".to_string();
        let mut content2 = alive_individual();
        content2.psychology.mental_state = "content".to_string();
        let stats = derive_stats(&stats_sim(vec![anxious, content, content2]));
        assert_eq!(stats["mental_state_distribution"]["anxious"], 1);
        assert_eq!(stats["mental_state_distribution"]["content"], 2);
    }

    #[test]
    fn genetic_diversity_is_exposed_and_reflects_a_skewed_sex_ratio() {
        // compute_genetic_diversity existed fully implemented and unit-tested
        // in sim-core but had no caller anywhere in the server -- there was
        // no way to see gene-pool health (heterozygosity, effective
        // population size) short of exporting the full report by hand.
        let males = vec![alive_individual(), alive_individual(), alive_individual()];
        let mut female = alive_individual();
        female.sex = "female".to_string();
        let mut all = males;
        all.push(female);
        let stats = derive_stats(&stats_sim(all));
        // Ne = 4*Nm*Nf/(Nm+Nf) = 4*3*1/4 = 3.0
        assert_eq!(stats["genetic_diversity"]["effective_population_size"], 3.0);
        assert!(stats["genetic_diversity"]["avg_heterozygosity"].as_f64().is_some());
        assert!(stats["genetic_diversity"]["allelic_variance"].as_f64().is_some());
        assert!(stats["genetic_diversity"]["avg_inbreeding_coefficient"].as_f64().is_some());
    }

    #[test]
    fn allele_frequencies_reflects_real_phenotype_averages_not_the_permanently_empty_dead_field() {
        // BiologyPanel.tsx's "Allele Frequency Snapshot" section (Feature 15)
        // already existed fully built, gated on `stats?.allele_frequencies`
        // being a non-empty object -- but derive_stats never produced that
        // key at all, so the section could never render for any simulation.
        let mut smart = alive_individual();
        smart.phenotype.fluid_intelligence = 0.9;
        let stats = derive_stats(&stats_sim(vec![smart]));
        assert!(!stats["allele_frequencies"].as_object().unwrap().is_empty());
        assert_eq!(stats["allele_frequencies"]["fluid_intelligence"], 0.9);
    }

    #[test]
    fn age_pyramid_returns_fourteen_bands() {
        let stats = derive_stats(&stats_sim(vec![alive_individual()]));
        assert_eq!(stats["age_pyramid"].as_array().unwrap().len(), 14);
    }

    #[test]
    fn epigenetics_returns_eight_loci_averages() {
        let stats = derive_stats(&stats_sim(vec![alive_individual()]));
        assert_eq!(stats["epigenetics"].as_object().unwrap().len(), 8);
    }

    #[test]
    fn births_and_deaths_totals_match_population_composition() {
        let founder = Individual { is_founder: true, alive: true, ..Default::default() };
        let mut dead_child = Individual { is_founder: false, is_dead: true, ..Default::default() };
        dead_child.parent_1_id = Some("founder".to_string());
        let alive_child = Individual { is_founder: false, alive: true, ..Default::default() };
        let stats = derive_stats(&stats_sim(vec![founder, dead_child, alive_child]));
        assert_eq!(stats["births"], 2); // both non-founders
        assert_eq!(stats["deaths"], 1);
    }

    #[tokio::test]
    async fn population_endpoint_filters_by_alive_query_param() {
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        // Kill one of the two founders directly in the backing store, the
        // same way the tick loop's mortality pass would.
        let mut sim = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        let dead_id = sim.individuals[0].id.clone();
        sim.individuals[0].alive = false;
        sim.individuals[0].is_dead = true;
        save_existing_state(&backend, &sim).await.expect("save");
        upsert_individuals(&backend, &sim, true).await.expect("upsert");

        async fn fetch_population(app: &Router, sim_id: &str, query: &str) -> Vec<Value> {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/simulations/{sim_id}/population{query}"))
                        .header("authorization", format!("Bearer {}", test_token()))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("population response");
            assert_eq!(response.status(), StatusCode::OK);
            body_json(response).await.as_array().expect("array body").clone()
        }

        let alive = fetch_population(&app, &sim_id, "?alive=true").await;
        let dead = fetch_population(&app, &sim_id, "?alive=false").await;
        let unfiltered = fetch_population(&app, &sim_id, "").await;

        assert_eq!(unfiltered.len(), 2, "both founders should still exist in total");
        assert_eq!(alive.len(), 1, "only the surviving founder should show up as alive");
        assert_eq!(dead.len(), 1, "only the killed founder should show up as dead");
        assert!(!alive.iter().any(|i| i["id"] == dead_id), "the dead founder must not appear in the alive=true list");
        assert!(dead.iter().any(|i| i["id"] == dead_id), "the dead founder must appear in the alive=false list");
    }

    #[tokio::test]
    async fn upsert_skips_rewriting_a_long_dead_individuals_data_json() {
        // Regression test for the upsert_individuals write-amplification fix:
        // a simulation running for years accumulates dead individuals who are
        // never touched again by the tick loop, so re-upserting their full
        // data_json on every single batch forever was pure waste that grew
        // worse the longer a simulation ran.
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        let mut sim = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        let dead_id = sim.individuals[0].id.clone();
        sim.individuals[0].alive = false;
        sim.individuals[0].is_dead = true;
        sim.individuals[0].death_day = Some(sim.current_day);
        upsert_individuals(&backend, &sim, true).await.expect("initial upsert");
        let original_payload = load_individual_payload(&backend, &sim_id, &dead_id).await.expect("load").expect("payload");

        // Well past the grace window (which must cover a whole runtime_loop
        // batch -- see DEAD_UPSERT_GRACE_DAYS's own doc comment), and with
        // the in-memory copy mutated as if something had (incorrectly)
        // touched a dead individual -- this must not reach the DB.
        sim.current_day += crate::runtime::MAX_BATCH_SIZE as i32 + 30;
        sim.individuals[0].x = 999.0;
        upsert_individuals(&backend, &sim, true).await.expect("second upsert");

        let payload_after_stale_death =
            load_individual_payload(&backend, &sim_id, &dead_id).await.expect("load").expect("payload");
        assert_eq!(payload_after_stale_death, original_payload, "a long-dead individual's row should not be rewritten");

        // An individual who is still alive must keep being upserted every time.
        let alive_id = sim.individuals[1].id.clone();
        sim.individuals[1].x = 777.0;
        upsert_individuals(&backend, &sim, true).await.expect("third upsert");
        let alive_payload = load_individual_payload(&backend, &sim_id, &alive_id).await.expect("load").expect("payload");
        assert_eq!(alive_payload["x"], json!(777.0), "an alive individual's row must still be kept up to date");
    }

    #[tokio::test]
    async fn state_json_no_longer_carries_individuals_but_load_full_state_still_does() {
        // Regression test for the state_json/individuals-table de-duplication:
        // ticking used to re-serialize every individual (dead ones included)
        // into state_json on every single batch, unbounded write amplification
        // that grew with total-ever-born. Now the `individuals` table (kept
        // current by upsert_individuals) is the sole store for per-individual
        // data, and state_json's own "individuals" key stays empty.
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        let mut sim = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        assert_eq!(sim.individuals.len(), 2, "a freshly created simulation should have both founders");
        sim.individuals[0].x = 42.0;
        save_existing_state(&backend, &sim).await.expect("save");
        upsert_individuals(&backend, &sim, true).await.expect("upsert");

        let row = load_simulation(&backend, &sim_id).await.expect("load").expect("row");
        let raw_individuals = row.state_json.get("individuals").and_then(Value::as_array);
        assert!(
            raw_individuals.is_none_or(Vec::is_empty),
            "state_json's individuals array should stay empty after a save, not re-embed the population: {:?}",
            row.state_json.get("individuals")
        );

        let reloaded = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        assert_eq!(reloaded.individuals.len(), 2, "load_full_state must still reconstruct the full population from the individuals table");
        assert_eq!(reloaded.individuals[0].x, 42.0, "the mutated field must round-trip through the individuals table");
    }

    #[tokio::test]
    async fn bounded_tick_state_drops_long_dead_individuals_but_genealogy_index_still_resolves_them() {
        // Regression test for the tick loop's bounded in-memory load: once an
        // individual is past the same grace window strip_dead_individual_if_due
        // already uses (DEAD_FIELD_STRIP_GRACE_DAYS), the tick loop should stop
        // carrying their full Individual payload in memory at all -- but their
        // parent linkage must still resolve through the lightweight genealogy
        // index, since a much-later descendant's inbreeding coefficient
        // depends on tracing back through them.
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        let mut sim = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        let founder_id = sim.individuals[0].id.clone();
        sim.individuals[0].alive = false;
        sim.individuals[0].is_dead = true;
        sim.individuals[0].death_day = Some(sim.current_day);
        upsert_individuals(&backend, &sim, true).await.expect("upsert dead founder");

        // Still within the grace window: must still be in the bounded load.
        let bounded = load_bounded_tick_state_no_genealogy(&backend, &sim_id).await.expect("load").expect("state");
        assert!(bounded.individuals.iter().any(|i| i.id == founder_id), "recently-dead individual should still be in the bounded window");

        // Advance current_day well past the strip grace window and re-save,
        // the same way the tick loop's own current_day advances across
        // batches.
        sim.current_day += sim_core::DEAD_FIELD_STRIP_GRACE_DAYS + 10;
        save_existing_state(&backend, &sim).await.expect("save advanced day");

        let bounded_later = load_bounded_tick_state_no_genealogy(&backend, &sim_id).await.expect("load").expect("state");
        assert!(
            !bounded_later.individuals.iter().any(|i| i.id == founder_id),
            "long-dead individual should no longer be in the bounded tick-loop load"
        );
        assert!(
            bounded_later.individuals.iter().any(|i| i.id == sim.individuals[1].id),
            "the still-alive founder must remain in the bounded load"
        );

        let genealogy = load_genealogy_index(&backend, &sim_id, None).await.expect("genealogy");
        assert!(genealogy.contains_key(&founder_id), "the long-dead founder must still resolve in the genealogy index");
    }

    #[tokio::test]
    async fn load_genealogy_index_since_birth_day_only_returns_the_delta() {
        // Regression test for the tick loop's incremental genealogy cache
        // (runtime.rs): re-fetching everyone ever born on every single batch
        // only got slower as a simulation aged, so the loop now merges in
        // just the delta born since its last watermark. That's only correct
        // if a `since_birth_day` cutoff is inclusive (>=, not >) -- otherwise
        // a same-day sibling of the previous batch's last birth would be
        // silently dropped forever.
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        let mut sim = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        let founder_id = sim.individuals[0].id.clone();

        let child_birth_day = sim.current_day + 100;
        let mut child = Individual { id: "child-since-test".to_string(), birth_day: child_birth_day, alive: true, ..Default::default() };
        child.parent_1_id = Some(founder_id.clone());
        child.parent_2_id = Some(sim.individuals[1].id.clone());
        sim.individuals.push(child.clone());
        sim.total_ever_born += 1;
        upsert_individuals(&backend, &sim, true).await.expect("upsert child");

        let full = load_genealogy_index(&backend, &sim_id, None).await.expect("genealogy");
        assert!(full.contains_key(&founder_id), "a full load must still see the founder");
        assert!(full.contains_key(&child.id), "a full load must see the new child");

        let at_cutoff = load_genealogy_index(&backend, &sim_id, Some(child_birth_day)).await.expect("genealogy");
        assert!(at_cutoff.contains_key(&child.id), "the cutoff must be inclusive of its own birth_day");
        assert!(!at_cutoff.contains_key(&founder_id), "an individual born before the cutoff must be excluded from the delta");

        let past_cutoff = load_genealogy_index(&backend, &sim_id, Some(child_birth_day + 1)).await.expect("genealogy");
        assert!(!past_cutoff.contains_key(&child.id), "a cutoff strictly after the birth_day must exclude it");
    }

    #[tokio::test]
    async fn total_ever_born_survives_a_bounded_reload_and_a_birth() {
        // Regression test for the total_ever_born counter: population_count
        // must keep reflecting everyone ever born even once the tick loop's
        // own in-memory individuals list is bounded and no longer equals
        // total-ever-born by itself.
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        let mut sim = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        assert_eq!(sim.total_ever_born, 2, "a freshly created simulation starts with both founders counted");

        // Simulate a birth the same way tick.rs's due-birth processing does:
        // append someone new to individuals and bump total_ever_born
        // alongside it.
        let mut child = Individual { id: "child-1".to_string(), alive: true, ..Default::default() };
        child.parent_1_id = Some(sim.individuals[0].id.clone());
        sim.individuals.push(child);
        sim.total_ever_born += 1;
        save_tick_progress(&backend, &sim).await.expect("save tick progress");
        upsert_individuals(&backend, &sim, true).await.expect("upsert");

        let row = load_simulation(&backend, &sim_id).await.expect("load").expect("row");
        assert_eq!(row.population_count, 3, "population_count should track total_ever_born, not individuals.len() alone");

        // Even a bounded reload (which only carries alive+recent individuals
        // in memory) must still report the correct total_ever_born, sourced
        // from this same dedicated column.
        let bounded = load_bounded_tick_state_no_genealogy(&backend, &sim_id).await.expect("load").expect("state");
        assert_eq!(bounded.total_ever_born, 3);
    }

    #[tokio::test]
    async fn get_individual_reads_from_the_individuals_table_not_state_json() {
        // Regression test for the get_population/get_individual rewrite: both
        // now read from the `individuals` table (kept in sync by
        // upsert_individuals) instead of deserializing the simulation's
        // state_json blob, to keep these frequently-polled endpoints cheap
        // regardless of how large state_json has grown from total-ever-born.
        let state = test_state().await;
        let backend = state.backend.clone();
        let app = test_app(state);
        let sim_id = create_simulation(&app).await;

        let sim = load_full_state(&backend, &sim_id).await.expect("load").expect("state");
        let founder_id = sim.individuals[0].id.clone();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/population/{founder_id}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("individual response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["id"], founder_id);

        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/population/does-not-exist"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("missing individual response");
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_report_surfaces_relationships_psychology_migration_and_death_breakdowns() {
        // Regression coverage for a cluster of report bugs found in review:
        // (1) notable_events always empty (importance was compared as i64
        //     against a string), (2) migration_history hardcoded to `[]` with
        //     no producer, (3) death_statistics.by_cause/by_age_group always
        //     `{}`, and (4) per-individual relationships/mental_state/
        //     theory_of_mind/reputation/role/inner_thought_log never reaching
        //     the report at all despite being tracked by the engine.
        let state = test_state().await;
        let app = test_app(state.clone());
        let sim_id = create_simulation(&app).await;

        let mut sim = crate::db::load_full_state(&state.backend, &sim_id).await.expect("load state").expect("simulation exists");
        let survivor_id = sim.individuals[0].id.clone();
        let victim_id = sim.individuals[1].id.clone();

        sim.individuals[0].psychology.relationships.insert(victim_id.clone(), 0.87);
        sim.individuals[0].psychology.mental_state = "grieving".to_string();
        sim.individuals[0].psychology.theory_of_mind = 2;
        sim.individuals[0].social.reputation = 0.42;
        sim.individuals[0].extra.insert("group_role".to_string(), json!("elder"));
        sim.individuals[0]
            .mind
            .extra
            .insert("inner_thought_log".to_string(), json!([{ "day": 5, "kind": "first_word", "thought": { "proto": "aba", "annotated": "aba [food]" } }]));

        let death_day = sim.current_day;
        sim.individuals[1].alive = false;
        sim.individuals[1].is_dead = true;
        sim.individuals[1].birth_day = death_day - 200; // < 1 year old at death -> infant_0_1 bucket
        sim.individuals[1].death_day = Some(death_day);
        sim.individuals[1].extra.insert("death_cause".to_string(), json!("predator"));

        sim.events.push(json!({ "type": "death", "individual_id": victim_id, "cause": "predator", "day": death_day, "importance": "high" }));
        sim.events.push(json!({
            "type": "migration", "day": death_day, "distance_km": 12.5, "reason": "food_scarcity",
            "from": { "x": 30.0, "y": 40.0 }, "to": { "x": 30.1, "y": 40.1 },
            "food_abundance": 0.2, "water_abundance": 0.6, "season": "winter", "importance": "medium",
        }));

        crate::db::save_existing_state(&state.backend, &sim).await.expect("save state");
        crate::db::upsert_individuals(&state.backend, &sim, true).await.expect("upsert individuals");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/report"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("report response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;

        let individuals = body["individuals"].as_array().expect("individuals array");
        let survivor = individuals.iter().find(|i| i["id"] == survivor_id).expect("survivor present");
        assert_eq!(survivor["mental_state"], "grieving");
        assert_eq!(survivor["theory_of_mind"], 2);
        assert_eq!(survivor["reputation"], 0.42);
        assert_eq!(survivor["role"], "elder");
        assert_eq!(survivor["relationships"][0]["id"], victim_id);
        assert_eq!(survivor["relationships"][0]["bond"], 0.87);
        assert_eq!(survivor["inner_thought_log"][0]["kind"], "first_word");

        let notable = body["notable_events"].as_array().expect("notable_events array");
        assert!(notable.iter().any(|e| e["event_type"] == "death"), "a high-importance event must survive the notable_events filter");
        let notable_death = notable.iter().find(|e| e["event_type"] == "death").unwrap();
        assert!(notable_death.get("description").and_then(Value::as_str).is_some_and(|d| !d.is_empty()), "notable_events must carry a real description, not a raw engine event");

        let migrations = body["migration_history"].as_array().expect("migration_history array");
        assert_eq!(migrations.len(), 1);
        assert_eq!(migrations[0]["distance_km"], 12.5);
        assert_eq!(migrations[0]["reason"], "food_scarcity");
        assert_eq!(body["summary"]["migration_events"], 1);
        assert_eq!(body["summary"]["total_migration_distance_km"], 12.5);

        assert_eq!(body["death_statistics"]["by_cause"]["predator"], 1);
        assert_eq!(body["death_statistics"]["by_age_group"]["infant_0_1"], 1);
        assert_eq!(body["summary"]["leading_cause_of_death"], "predator");
    }

    #[tokio::test]
    async fn get_report_death_by_cause_merges_the_same_cause_regardless_of_which_engine_set_its_case() {
        // Found by actually running the simulation engine for 60 sim-years:
        // ordinary mortality rolls format!("{cause:?}") a DeathCause enum
        // variant ("Infection", PascalCase), while microbiome outbreak
        // deaths set an already-lowercase literal ("infection") -- both
        // real, simultaneously-reachable causes for the exact same
        // underlying concept. Before normalizing with pascal_to_snake
        // (already used elsewhere for event descriptions, just not here),
        // this fragmented into two separate by_cause buckets, silently
        // undercounting each.
        let state = test_state().await;
        let app = test_app(state.clone());
        let sim_id = create_simulation(&app).await;

        let mut sim = crate::db::load_full_state(&state.backend, &sim_id).await.expect("load state").expect("simulation exists");
        sim.individuals[0].alive = false;
        sim.individuals[0].is_dead = true;
        sim.individuals[0].death_day = Some(sim.current_day);
        sim.individuals[0].extra.insert("death_cause".to_string(), json!("Infection"));
        sim.individuals[1].alive = false;
        sim.individuals[1].is_dead = true;
        sim.individuals[1].death_day = Some(sim.current_day);
        sim.individuals[1].extra.insert("death_cause".to_string(), json!("infection"));
        crate::db::save_existing_state(&state.backend, &sim).await.expect("save state");
        crate::db::upsert_individuals(&state.backend, &sim, true).await.expect("upsert individuals");

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/report"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("report response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;

        assert_eq!(body["death_statistics"]["by_cause"]["infection"], 2, "PascalCase and lowercase spellings of the same cause must merge into one bucket");
        assert!(body["death_statistics"]["by_cause"].get("Infection").is_none(), "no raw PascalCase key should ever survive into the report");
    }

    #[tokio::test]
    async fn get_report_population_history_carries_genetic_diversity_from_each_checkpoints_own_stats() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let checkpoint_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/checkpoint"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("checkpoint response");
        assert_eq!(checkpoint_response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/report"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("report response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;

        let history = body["population_history"].as_array().expect("population_history array");
        assert_eq!(history.len(), 1);
        let gd = &history[0]["genetic_diversity"];
        assert!(gd["effective_population_size"].as_f64().is_some(), "each checkpoint must carry its own genetic_diversity snapshot");
    }

    #[tokio::test]
    async fn get_report_never_leaks_a_real_world_belief_name() {
        // Cardinal-rule regression: belief_id ("belief_1".."belief_6") is an
        // opaque engine bucketing key, never a real-world religion name (see
        // belief.rs's own doc comment) -- the only player-facing *name* comes
        // from sim.belief_labels, generated once the population's own
        // language can express one; before that, only the opaque numeric
        // code is shown (matching build_event_description and
        // BeliefPanel.tsx). The /report export bypassed this everywhere:
        // belief_timeline, summary.belief_list, the population_history
        // checkpoint snapshot, and each individual's own beliefs set all
        // leaked the raw archetype id straight through.
        let state = test_state().await;
        let app = test_app(state.clone());
        let sim_id = create_simulation(&app).await;

        let mut sim = crate::db::load_full_state(&state.backend, &sim_id).await.expect("load state").expect("simulation exists");
        sim.discovered_beliefs = vec!["belief_1".to_string(), "belief_2".to_string()];
        sim.belief_labels.insert("belief_1".to_string(), "Sekibo".to_string());
        // "belief_2" deliberately left unlabeled -- the population hasn't
        // reached proto-words for it yet.
        sim.individuals[0].beliefs.insert("belief_2".to_string());
        crate::db::save_existing_state(&state.backend, &sim).await.expect("save state");
        crate::db::upsert_individuals(&state.backend, &sim, true).await.expect("upsert individuals");

        // A checkpoint so population_history has a row to check too.
        let checkpoint_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/simulations/{sim_id}/checkpoint"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("checkpoint response");
        assert_eq!(checkpoint_resp.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/report"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("report response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        let raw = body.to_string();
        for real_world_name in ["animism", "ancestor_cult", "shamanism", "polytheism", "monotheism", "philosophical"] {
            assert!(!raw.contains(real_world_name), "a real-world religion name must never appear anywhere in the report: {raw}");
        }

        let belief_timeline = body["belief_timeline"].as_array().expect("belief_timeline array");
        let codes: Vec<&str> = belief_timeline.iter().map(|e| e["code"].as_str().unwrap()).collect();
        assert!(codes.contains(&"belief_1"), "the opaque code is safe to show and must be present");
        assert!(codes.contains(&"belief_2"), "the opaque code is safe to show and must be present");
        let names: Vec<Option<&str>> = belief_timeline.iter().map(|e| e["name"].as_str()).collect();
        assert!(names.contains(&Some("Sekibo")), "the labeled belief must show its generated name");
        assert!(names.contains(&None), "the unlabeled belief must have no name yet -- only its opaque code");

        let belief_list = body["summary"]["belief_list"].as_array().expect("belief_list array");
        let list_names: Vec<&str> = belief_list.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(list_names.contains(&"Sekibo"));
        assert!(list_names.contains(&"Unnamed belief (#2)"));

        let pop_history = body["population_history"].as_array().expect("population_history array");
        assert_eq!(pop_history[0]["beliefs"], 2, "population_history should carry a count, not the raw archetype id list");

        // /report's own individual serialization (distinct from serialize_individual
        // below) never included a beliefs field at all -- confirm it still doesn't.
        let individuals = body["individuals"].as_array().expect("individuals array");
        for ind in individuals {
            assert!(ind.get("beliefs").is_none(), "no individual should expose a raw beliefs id set");
        }

        // serialize_individual (used by /population and /population/:id, the
        // endpoints the live UI actually polls) is the other place that used to
        // leak the raw set -- check it directly.
        let pop_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/population"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("population response");
        let pop_body = body_json(pop_resp).await;
        let raw_pop = pop_body.to_string();
        for real_world_name in ["animism", "ancestor_cult", "shamanism", "polytheism", "monotheism", "philosophical"] {
            assert!(!raw_pop.contains(real_world_name), "a real-world religion name must never appear on the population endpoint: {raw_pop}");
        }
        let pop_individuals = pop_body.as_array().expect("population array");
        for ind in pop_individuals {
            assert!(ind.get("beliefs").is_none(), "no individual should expose a raw beliefs id set");
        }
        let with_belief = pop_individuals.iter().find(|i| i["id"] == sim.individuals[0].id).expect("individual present");
        assert_eq!(with_belief["beliefs_count"], 1);
    }

    #[tokio::test]
    async fn stats_endpoint_returns_current_derive_stats_snapshot() {
        // Regression test for the "sim looks frozen until the next WS tick"
        // bug: the client needs a REST snapshot of `derive_stats` it can fetch
        // immediately on mount/reconnect instead of waiting for a WebSocket tick.
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/stats"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("stats response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["population"], 2, "both founders should be alive right after creation");
        assert_eq!(body["day"], 0);
        assert!(body.get("qol_index").is_some());
    }

    #[tokio::test]
    async fn import_simulation_is_rate_limited_per_user() {
        // Unlike login (per user_code) or register (a global window), an
        // already-authenticated caller could otherwise POST an unbounded
        // number of imports with no throttle at all -- axum's default body
        // limit was the only other bound on this route.
        let app = test_app(test_state().await);
        let token = test_token();
        let mut last_status = StatusCode::OK;
        for _ in 0..21 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/simulations/import")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .expect("import response");
            last_status = response.status();
        }
        assert_eq!(last_status, StatusCode::TOO_MANY_REQUESTS, "the 21st import within the same 15-minute window should be throttled");
    }

    #[tokio::test]
    async fn register_is_rate_limited_by_a_global_window() {
        // Unlike login (keyed per user_code), a registration spammer controls
        // every field in the payload, so a per-key limit is trivially
        // bypassed by varying the user_code/email each time -- only a global
        // window actually bounds this. The rate-limit check runs first, ahead
        // of even the is_local_backend check, so it's exercised regardless of
        // what backend the test harness uses (test_state() is always Sqlite,
        // i.e. "local", so every one of these calls 403s -- what this test
        // asserts is that the 21st becomes 429 instead of a 21st 403).
        let app = test_app(test_state().await);
        let empty_payload = json!({ "first_name": "", "last_name": "", "tc_no": "", "email": "", "password": "", "user_code": "" });
        let mut statuses = Vec::new();
        for _ in 0..21 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/auth/register")
                        .header("content-type", "application/json")
                        .body(Body::from(empty_payload.to_string()))
                        .unwrap(),
                )
                .await
                .expect("register response");
            statuses.push(response.status());
        }
        assert_eq!(statuses[0], StatusCode::FORBIDDEN, "the first attempt should reach past the rate limiter and hit the local-backend gate");
        assert_eq!(statuses[19], StatusCode::FORBIDDEN, "the 20th attempt is still within the window's allowance");
        assert_eq!(statuses[20], StatusCode::TOO_MANY_REQUESTS, "the 21st registration attempt within the same 15-minute window should be throttled");
    }

    // ── to_client_event / build_event_description ───────────────────────
    //
    // Regression tests for the bug where the Event Log's description field
    // was always just the bare event_type slug ("death", "birth", ...): the
    // client's translateEventDescription() in i18n.ts was written to parse
    // rich English sentences like "X died: predator" back out and translate
    // them, but the Rust engine never produced that shape, so every event
    // looked identical and untranslatable regardless of language.

    fn named_individual(id: &str, name: &str) -> Individual {
        Individual {
            id: id.to_string(),
            phenotype: sim_core::Phenotype { name: Some(name.to_string()), ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn birth_description_names_both_parents_when_resolvable() {
        let mother = named_individual("mom", "Ayla");
        let father = named_individual("dad", "Kaan");
        let child = Individual {
            id: "kid".to_string(),
            parent_1_id: Some("mom".to_string()),
            parent_2_id: Some("dad".to_string()),
            phenotype: sim_core::Phenotype { name: Some("Deniz".to_string()), ..Default::default() },
            ..Default::default()
        };
        let sim = stats_sim(vec![mother, father, child]);
        let raw = json!({ "type": "birth", "individual_id": "kid", "day": 10 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Born: Deniz (Ayla & Kaan)");
    }

    #[test]
    fn birth_description_falls_back_when_child_cannot_be_resolved() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "birth", "individual_id": "missing", "day": 10 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "New individual born");
    }

    #[test]
    fn death_description_names_the_individual_and_snake_cases_the_cause() {
        let dead = named_individual("ind1", "Elif");
        let sim = stats_sim(vec![dead]);
        let raw = json!({ "type": "death", "individual_id": "ind1", "cause": "OldAge", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Elif died: old_age");
    }

    #[test]
    fn death_description_passes_through_an_already_snake_case_cause() {
        let dead = named_individual("ind1", "Elif");
        let sim = stats_sim(vec![dead]);
        let raw = json!({ "type": "death", "individual_id": "ind1", "cause": "infection", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Elif died: infection");
    }

    #[test]
    fn discovery_description_names_the_technology() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "discovery", "tech_id": "fire_making", "day": 1 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Technology discovered: fire_making");
    }

    #[test]
    fn belief_spread_description_falls_back_to_an_opaque_code_before_the_belief_is_named() {
        // Cardinal rule: the raw belief_id ("belief_2") is an opaque
        // bucketing key, never a real-world religion name -- see belief.rs.
        // With no entry in belief_labels yet (the population hasn't reached
        // proto-words), the description shows only the opaque numeric code,
        // per the user's directive that we should roughly know what kind of
        // belief this is (via the code) even before it has a real name.
        let believer = named_individual("ind1", "Elif");
        let sim = stats_sim(vec![believer]);
        let raw = json!({ "type": "belief_spread", "individual_id": "ind1", "belief_id": "belief_2", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Elif embraced belief #2");
    }

    #[test]
    fn belief_spread_description_names_the_believer_and_the_generated_label_once_known() {
        let believer = named_individual("ind1", "Elif");
        let mut belief_labels = std::collections::HashMap::new();
        belief_labels.insert("belief_2".to_string(), "Sekibo".to_string());
        let sim = SimulationState { belief_labels, ..stats_sim(vec![believer]) };
        let raw = json!({ "type": "belief_spread", "individual_id": "ind1", "belief_id": "belief_2", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Elif embraced Sekibo");
    }

    #[test]
    fn ritual_emerged_description_falls_back_to_an_opaque_code_before_the_belief_is_named() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "ritual_emerged", "belief": "belief_3", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "A ritual (belief #3) emerges in the group");
    }

    #[test]
    fn ritual_emerged_description_names_the_generated_label_once_known() {
        let mut belief_labels = std::collections::HashMap::new();
        belief_labels.insert("belief_3".to_string(), "Odubwe".to_string());
        let sim = SimulationState { belief_labels, ..stats_sim(vec![]) };
        let raw = json!({ "type": "ritual_emerged", "belief": "belief_3", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "A Odubwe ritual emerges in the group");
    }

    #[test]
    fn belief_formed_description_never_names_a_real_world_religion() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "belief_formed", "belief_id": "belief_5", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "A new belief (#5) takes hold");
    }

    #[test]
    fn belief_formed_description_names_the_founder_even_before_the_belief_has_a_label() {
        // Regression: this used to always say "A new belief takes hold" no
        // matter what, ignoring founder_id entirely -- unlike belief_spread/
        // ritual_emerged, which both already name the individual involved.
        let founder = named_individual("ind1", "Mete");
        let sim = stats_sim(vec![founder]);
        let raw = json!({ "type": "belief_formed", "founder_id": "ind1", "belief_id": "belief_5", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Mete gave rise to belief #5");
    }

    #[test]
    fn belief_formed_description_names_both_the_founder_and_the_generated_label_once_known() {
        let founder = named_individual("ind1", "Mete");
        let mut belief_labels = std::collections::HashMap::new();
        belief_labels.insert("belief_5".to_string(), "Karvun".to_string());
        let sim = SimulationState { belief_labels, ..stats_sim(vec![founder]) };
        let raw = json!({ "type": "belief_formed", "founder_id": "ind1", "belief_id": "belief_5", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Mete gave rise to Karvun");
    }

    #[test]
    fn belief_named_description_announces_the_generated_label() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "belief_named", "belief_id": "belief_5", "label": "Kelu", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Their belief becomes known as Kelu");
    }

    #[test]
    fn group_named_description_announces_the_generated_name() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "group_named", "group_id": "g1", "name": "Baru", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "The group becomes known as Baru");
    }

    #[test]
    fn civilization_named_description_announces_the_generated_name() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "civilization_named", "name": "Anoteva", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Their civilization becomes known as Anoteva");
    }

    #[test]
    fn group_split_description_is_a_fixed_sentence() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "group_split", "parent_group_id": "g1", "new_group_id": "g2", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "A group split into two bands");
    }

    #[test]
    fn conflict_description_names_the_casualty_count() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "conflict", "attacker_group_id": "g1", "defender_group_id": "g2", "casualties": 2, "defense_bonus": 0.5, "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "A clash between rival groups left 2 dead");
    }

    #[test]
    fn leadership_change_description_names_the_new_leader() {
        let leader = named_individual("ind1", "Kaan");
        let sim = stats_sim(vec![leader]);
        let raw = json!({ "type": "leadership_change", "group_id": "g1", "new_leader_id": "ind1", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Kaan became the new leader");
    }

    #[test]
    fn trade_description_names_both_traders() {
        let a = named_individual("a", "Ayla");
        let b = named_individual("b", "Kaan");
        let sim = stats_sim(vec![a, b]);
        let raw = json!({ "type": "trade", "individual_a": "a", "individual_b": "b", "day": 5 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Ayla traded with Kaan");
    }

    #[test]
    fn an_existing_description_field_is_preserved_not_overwritten_with_the_event_type() {
        // Mirrors norm_emerged / cultural_meme_emerged / belief_formed / art_created,
        // which already attach a self-contained sentence at push time.
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "norm_emerged", "norm_id": "reciprocity", "day": 1, "description": "Members are expected to return favors" });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "Members are expected to return favors");
    }

    #[test]
    fn an_event_type_with_no_template_and_no_description_falls_back_to_the_type_slug() {
        let sim = stats_sim(vec![]);
        let raw = json!({ "type": "some_future_event", "day": 1 });
        let event = to_client_event(&raw, &sim);
        assert_eq!(event["description"], "some_future_event");
    }

    // ── Feature #8: comparative simulation analysis (/api/simulations/compare) ──

    #[tokio::test]
    async fn compare_endpoint_returns_side_by_side_stats_for_two_owned_simulations() {
        let app = test_app(test_state().await);
        let sim_a = create_simulation(&app).await;
        let sim_b = create_simulation(&app).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/compare?a={sim_a}&b={sim_b}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("compare response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["a"]["id"], sim_a);
        assert_eq!(body["b"]["id"], sim_b);
        assert_eq!(body["a"]["stats"]["population"], 2, "each freshly created sim starts with its two founders");
        assert_eq!(body["b"]["stats"]["population"], 2);
    }

    #[tokio::test]
    async fn compare_endpoint_rejects_a_simulation_the_caller_does_not_own() {
        // A second, differently-authenticated user's simulation must not be
        // readable through the comparison endpoint just by knowing its id.
        let app = test_app(test_state().await);
        let sim_a = create_simulation(&app).await;

        let other_claims = crate::auth::Claims {
            id: "22222222-2222-2222-2222-222222222222".to_string(),
            username: "other".to_string(),
            email: "other@example.com".to_string(),
            role: "user".to_string(),
            exp: (chrono::Utc::now().timestamp() + 900) as usize,
        };
        let other_token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &other_claims,
            &jsonwebtoken::EncodingKey::from_secret(crate::auth::access_secret().as_bytes()),
        )
        .expect("sign other token");

        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/simulations")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {other_token}"))
                    .body(Body::from(json!({"name": "Other's sim", "latitude": 10.0, "longitude": 10.0, "founder_1_params": {}, "founder_2_params": {}}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("create response");
        let sim_owned_by_other = body_json(create_resp).await["id"].as_str().unwrap().to_string();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/compare?a={sim_a}&b={sim_owned_by_other}"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("compare response");
        assert_ne!(response.status(), StatusCode::OK, "must not leak another user's simulation stats");
    }

    // ── Feature #10: cross-simulation migration (/api/god/:id/migrate-individual) ──

    #[tokio::test]
    async fn migrate_individual_carries_the_source_individuals_genome_into_the_target_as_a_non_founder_arrival() {
        let app = test_app(test_state().await);
        let source_sim = create_simulation(&app).await;
        let target_sim = create_simulation(&app).await;

        let pop_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{source_sim}/population"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("population response");
        assert_eq!(pop_resp.status(), StatusCode::OK);
        let source_population = body_json(pop_resp).await;
        let source_individuals = source_population.as_array().expect("population array");
        assert_eq!(source_individuals.len(), 2, "a fresh simulation has exactly its two founders");
        let source_individual = &source_individuals[0];
        let source_individual_id = source_individual["id"].as_str().expect("individual id").to_string();
        let source_foxp2 = source_individual["phenotype"]["language_capacity"].as_f64().expect("language_capacity present");

        let migrate_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/god/{target_sim}/migrate-individual"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({"source_simulation_id": source_sim, "individual_id": source_individual_id}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("migrate response");
        assert_eq!(migrate_resp.status(), StatusCode::OK, "migration between two simulations owned by the same user must succeed");
        let migrate_body = body_json(migrate_resp).await;
        let arrived_id = migrate_body["arrived_individual_id"].as_str().expect("arrived_individual_id present").to_string();
        assert_ne!(arrived_id, source_individual_id, "the arrival must get a fresh id, not reuse the source's");

        let target_pop_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{target_sim}/population"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("target population response");
        let target_population = body_json(target_pop_resp).await;
        let target_individuals = target_population.as_array().expect("population array");
        assert_eq!(target_individuals.len(), 3, "the target simulation should now have its 2 founders plus the new arrival");

        let arrival = target_individuals.iter().find(|i| i["id"] == arrived_id).expect("arrival present in target population");
        assert_eq!(arrival["is_founder"], false, "a migrated individual must never be marked as a founder");
        assert_eq!(arrival["parent_1_id"], Value::Null, "the source simulation's genealogy must not leak a dangling parent id into the target");
        assert_eq!(arrival["parent_2_id"], Value::Null);
        assert_eq!(
            arrival["phenotype"]["language_capacity"], source_foxp2,
            "the arrival's genome-derived phenotype must carry over from the source individual unchanged"
        );
    }

    #[tokio::test]
    async fn migrate_individual_is_rejected_when_the_caller_does_not_own_the_source_simulation() {
        let app = test_app(test_state().await);
        let target_sim = create_simulation(&app).await;

        let other_claims = crate::auth::Claims {
            id: "33333333-3333-3333-3333-333333333333".to_string(),
            username: "other2".to_string(),
            email: "other2@example.com".to_string(),
            role: "user".to_string(),
            exp: (chrono::Utc::now().timestamp() + 900) as usize,
        };
        let other_token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &other_claims,
            &jsonwebtoken::EncodingKey::from_secret(crate::auth::access_secret().as_bytes()),
        )
        .expect("sign other token");
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/simulations")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {other_token}"))
                    .body(Body::from(json!({"name": "Someone else's sim", "latitude": 5.0, "longitude": 5.0, "founder_1_params": {}, "founder_2_params": {}}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("create response");
        let sim_owned_by_other = body_json(create_resp).await["id"].as_str().unwrap().to_string();
        let pop_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_owned_by_other}/population"))
                    .header("authorization", format!("Bearer {other_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("population response");
        let other_individual_id = body_json(pop_resp).await[0]["id"].as_str().unwrap().to_string();

        let migrate_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/god/{target_sim}/migrate-individual"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::from(json!({"source_simulation_id": sim_owned_by_other, "individual_id": other_individual_id}).to_string()))
                    .unwrap(),
            )
            .await
            .expect("migrate response");
        assert_eq!(
            migrate_resp.status(),
            StatusCode::FORBIDDEN,
            "must not be able to pull an individual out of a simulation the caller doesn't own"
        );
    }

    // ── Legends panel (/api/simulations/:id/legends) ────────────────────

    #[tokio::test]
    async fn legends_endpoint_reports_the_two_founders_as_the_only_current_record_holders() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/legends"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("legends response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        // A freshly created simulation has only its two founders, neither
        // dead nor with children yet -- consciousness/reputation record
        // holders must still resolve to one of them, and longevity/children
        // must both be absent (nobody has died or reproduced yet).
        assert!(body["highest_consciousness"]["id"].is_string());
        assert!(body["highest_reputation"]["id"].is_string());
        assert_eq!(body["most_children"], Value::Null);
        assert_eq!(body["longest_lived"], Value::Null);
        assert_eq!(body["most_technologies"], Value::Null);
    }

    #[tokio::test]
    async fn legends_endpoint_rejects_a_simulation_the_caller_does_not_own() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/legends"))
                    .header("authorization", format!("Bearer {}", other_test_token("legends-outsider")))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("legends response");
        assert_ne!(response.status(), StatusCode::OK);
    }

    // ── Documentary (/api/simulations/:id/documentary) ──────────────────

    #[tokio::test]
    async fn documentary_endpoint_falls_back_to_the_heuristic_without_a_gemini_key() {
        // No GEMINI_API_KEY is set in the test environment, so this must
        // deterministically take the heuristic path rather than failing.
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/documentary"))
                    .header("authorization", format!("Bearer {}", test_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("documentary response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["generated_by"], "heuristic");
        let scenes = body["scenes"].as_array().expect("scenes array");
        assert!(!scenes.is_empty(), "the heuristic fallback must always produce at least the present-day scene");
        let last = scenes.last().unwrap();
        assert_eq!(last["title"], "present day");
        assert!(last["narration"].as_str().unwrap().contains("individuals"));
    }

    #[tokio::test]
    async fn documentary_endpoint_rejects_a_simulation_the_caller_does_not_own() {
        let app = test_app(test_state().await);
        let sim_id = create_simulation(&app).await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/simulations/{sim_id}/documentary"))
                    .header("authorization", format!("Bearer {}", other_test_token("documentary-outsider")))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("documentary response");
        assert_ne!(response.status(), StatusCode::OK);
    }

    fn other_test_token(username: &str) -> String {
        let claims = crate::auth::Claims {
            id: uuid::Uuid::new_v4().to_string(),
            username: username.to_string(),
            email: format!("{username}@example.com"),
            role: "user".to_string(),
            exp: (chrono::Utc::now().timestamp() + 900) as usize,
        };
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(crate::auth::access_secret().as_bytes()),
        )
        .expect("sign other token")
    }
}
