//! Thin WASM bindings around sim-core, for running the simulation engine
//! directly inside a browser tab or a mobile app's WebView (no server round
//! trip at all) -- the client-only counterpart to how sim-server wraps
//! sim-core for the cloud/native-local backends. All I/O is JSON strings
//! in/out; the caller (TypeScript) owns persistence and the tick-loop timer,
//! this crate only ever advances one day per call, exactly like
//! sim-core-cli.rs does over stdin/stdout for the native CLI. Every export
//! here mirrors one sim-server route/handler 1:1 (see each function's doc
//! comment for which) so the client-side adapter that calls these can stay
//! a thin dispatcher rather than reimplementing any simulation logic itself.
//!
//! Every `#[wasm_bindgen]` export is a one-line wrapper around a plain-Rust
//! `_impl` function returning `Result<String, String>` -- `JsValue` itself
//! only actually works inside a real wasm/JS host (its `__wbindgen_describe`
//! hook aborts the process if called from a native `cargo test`), so keeping
//! all real logic on the native-testable side lets `#[cfg(test)]` below
//! exercise every code path without needing a browser or `wasm-pack test`.
use std::collections::HashSet;

use serde::Serialize;
use wasm_bindgen::prelude::*;

use sim_core::{advance_one_day, create_founder, create_world_state, PhaseTimings, SimulationState, TickReport, WorldState};

// Exposes an async `initThreadPool(numThreads)` export in the generated JS
// (worker.ts calls it right after `init()`, before any other export) that
// spins up a real Web Worker-backed rayon thread pool -- sim-core's existing
// `par_iter`/`par_iter_mut` calls then actually run across multiple cores
// instead of rayon's single-threaded wasm32 fallback. Only meaningful for a
// real wasm32 target with atomics enabled (see .cargo/config.toml); gated
// out of native `cargo test` runs, which don't have a WebAssembly::Module or
// Memory to query in the first place (see wasm_bindgen::module()/memory()
// inside wasm-bindgen-rayon's own init_thread_pool).
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_rayon::init_thread_pool;

#[derive(Serialize)]
struct TickResult {
    state: SimulationState,
    report: TickReport,
    phases: PhaseTimings,
}

fn parse_state(state_json: &str) -> Result<SimulationState, String> {
    serde_json::from_str(state_json).map_err(|e| e.to_string())
}

fn to_json_string(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

/// Builds a fresh SimulationState with an empty population, ready for
/// create_founder_json output to be pushed into its `individuals` array.
/// Mirrors how sim-server's own POST /simulations handler seeds a new run.
fn create_world_impl(latitude: f64, longitude: f64) -> Result<String, String> {
    let world_state: WorldState = serde_json::from_value(create_world_state(latitude, longitude)).map_err(|e| e.to_string())?;
    let state = SimulationState {
        start_latitude: Some(latitude),
        start_longitude: Some(longitude),
        world_state,
        ..Default::default()
    };
    to_json_string(&state)
}

/// `params_json` matches create_founder's own wire shape, e.g.
/// `{"sex":"male","ageYears":22,"x":37.0,"y":35.0,"name":"Adam"}`.
fn create_founder_json_impl(params_json: &str) -> Result<String, String> {
    let params: serde_json::Value = serde_json::from_str(params_json).map_err(|e| e.to_string())?;
    let founder = create_founder(&params);
    to_json_string(&founder)
}

/// Builds a brand-new two-founder simulation in one call -- mirrors
/// `POST /api/simulations` end to end (world + both founders + starting
/// techs), minus the account/ownership fields a no-account WASM-local trial
/// has no use for (`user_id` stays `None`). `founder_1_params`/
/// `founder_2_params` match the wizard's own wire shape (may be `"{}"`).
fn create_simulation_impl(name: Option<String>, latitude: f64, longitude: f64, founder_1_params_json: &str, founder_2_params_json: &str) -> Result<String, String> {
    let founder_1_params: serde_json::Value = serde_json::from_str(founder_1_params_json).map_err(|e| e.to_string())?;
    let founder_2_params: serde_json::Value = serde_json::from_str(founder_2_params_json).map_err(|e| e.to_string())?;
    let sim = sim_core::new_simulation(name, latitude, longitude, &founder_1_params, &founder_2_params);
    to_json_string(&sim)
}

/// Advances one simulated day. `state_json` is a full SimulationState (as
/// produced by this same module, or by sim-server's own export endpoint --
/// the wire shape is identical). Returns `{ state, report, phases }`.
/// `disabled_engines_json` is a JSON array of engine names (mirrors
/// `POST /:id/engines`'s body), passed in on every call rather than living on
/// `state_json` itself: `SimulationState.disabled_engines` is `#[serde(skip)]`
/// (a diagnostic toggle deliberately never persisted), so sim-server keeps it
/// as session-scoped state outside the JSON and injects it fresh before each
/// tick (see runtime.rs) -- this mirrors that same pattern for the caller,
/// which must keep the disabled set itself and resend it every call.
fn advance_day_impl(state_json: &str, disabled_engines_json: &str) -> Result<String, String> {
    let mut state = parse_state(state_json)?;
    state.disabled_engines = serde_json::from_str::<HashSet<String>>(disabled_engines_json).map_err(|e| e.to_string())?;
    let (report, phases) = advance_one_day(&mut state);
    let result = TickResult { state, report, phases };
    to_json_string(&result)
}

/// Mirrors `GET /:id/stats` and the WS `tick` broadcast's `stats` field.
fn get_stats_impl(state_json: &str) -> Result<String, String> {
    let state = parse_state(state_json)?;
    to_json_string(&sim_core::derive_stats(&state))
}

/// Mirrors `GET /:id/population?alive=&limit=`. `alive` is a tri-state via
/// `Option<bool>` (omit the query param entirely to mean "either"); `limit`
/// likewise optional.
fn get_population_impl(state_json: &str, alive: Option<bool>, limit: Option<usize>) -> Result<String, String> {
    let state = parse_state(state_json)?;
    to_json_string(&sim_core::population_view(&state, alive, limit))
}

/// Mirrors `GET /:id/population/:individualId`. Returns `null` (not an
/// error) when no individual with that id exists, matching how a missing
/// optional field round-trips through JSON rather than needing a second
/// found/not-found channel.
fn get_individual_impl(state_json: &str, individual_id: &str) -> Result<String, String> {
    let state = parse_state(state_json)?;
    let found = sim_core::find_individual(&state, individual_id).map(|ind| sim_core::serialize_individual(ind, state.current_day));
    to_json_string(&found)
}

/// Mirrors `GET /:id/events`.
fn get_events_impl(state_json: &str) -> Result<String, String> {
    let state = parse_state(state_json)?;
    let events: Vec<serde_json::Value> = state.events.iter().map(|e| sim_core::to_client_event(e, &state)).collect();
    to_json_string(&events)
}

/// Mirrors `GET /:id/events/summary`.
fn get_events_summary_impl(state_json: &str) -> Result<String, String> {
    let state = parse_state(state_json)?;
    to_json_string(&sim_core::events_summary(&state))
}

#[derive(Serialize)]
struct InterveneResult {
    state: SimulationState,
    affected_individuals: i64,
    deaths: i64,
}

/// Mirrors `POST /api/god/:simId/intervene`. Returns
/// `{ state, affected_individuals, deaths }` on success (the caller
/// persists/replaces its own held state with the returned one) or an `Err`
/// with the same message `god::intervene` would have returned as a 400
/// (e.g. a Cardinal Rule rejection) on failure -- callers should treat any
/// rejection here exactly like a rejected fetch() to that route.
fn apply_intervention_impl(state_json: &str, intervention_type: &str, params_json: &str) -> Result<String, String> {
    let state = parse_state(state_json)?;
    let params: serde_json::Value = serde_json::from_str(params_json).map_err(|e| e.to_string())?;
    let day = state.current_day as i64;
    let alive_count = state.individuals.iter().filter(|i| i.alive && !i.is_dead).count();
    // apply_intervention operates on a bare Value (see interventions.rs's own
    // doc comment on why -- it's shared with sim-server's HTTP layer, which
    // never has a typed SimulationState on hand at that call site either).
    let mut sim_value = serde_json::to_value(&state).map_err(|e| e.to_string())?;
    let (affected, deaths) = sim_core::apply_intervention(&mut sim_value, intervention_type, &params, day, alive_count)?;
    let state: SimulationState = serde_json::from_value(sim_value).map_err(|e| e.to_string())?;
    to_json_string(&InterveneResult { state, affected_individuals: affected, deaths })
}

/// Mirrors `POST /:id/terminate` -- ends the civilization via the same
/// mass-mortality path every organic disaster uses, keeping the historical
/// record intact (see `sim_core::terminate`'s own doc comment).
fn terminate_impl(state_json: &str) -> Result<String, String> {
    let mut state = parse_state(state_json)?;
    sim_core::terminate(&mut state);
    to_json_string(&state)
}

fn to_js_result(result: Result<String, String>) -> Result<String, JsValue> {
    result.map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn create_world(latitude: f64, longitude: f64) -> Result<String, JsValue> {
    to_js_result(create_world_impl(latitude, longitude))
}

#[wasm_bindgen]
pub fn create_founder_json(params_json: &str) -> Result<String, JsValue> {
    to_js_result(create_founder_json_impl(params_json))
}

#[wasm_bindgen]
pub fn create_simulation(name: Option<String>, latitude: f64, longitude: f64, founder_1_params_json: &str, founder_2_params_json: &str) -> Result<String, JsValue> {
    to_js_result(create_simulation_impl(name, latitude, longitude, founder_1_params_json, founder_2_params_json))
}

#[wasm_bindgen]
pub fn advance_day(state_json: &str, disabled_engines_json: &str) -> Result<String, JsValue> {
    to_js_result(advance_day_impl(state_json, disabled_engines_json))
}

#[wasm_bindgen]
pub fn get_stats(state_json: &str) -> Result<String, JsValue> {
    to_js_result(get_stats_impl(state_json))
}

#[wasm_bindgen]
pub fn get_population(state_json: &str, alive: Option<bool>, limit: Option<usize>) -> Result<String, JsValue> {
    to_js_result(get_population_impl(state_json, alive, limit))
}

#[wasm_bindgen]
pub fn get_individual(state_json: &str, individual_id: &str) -> Result<String, JsValue> {
    to_js_result(get_individual_impl(state_json, individual_id))
}

#[wasm_bindgen]
pub fn get_events(state_json: &str) -> Result<String, JsValue> {
    to_js_result(get_events_impl(state_json))
}

#[wasm_bindgen]
pub fn get_events_summary(state_json: &str) -> Result<String, JsValue> {
    to_js_result(get_events_summary_impl(state_json))
}

#[wasm_bindgen]
pub fn apply_intervention(state_json: &str, intervention_type: &str, params_json: &str) -> Result<String, JsValue> {
    to_js_result(apply_intervention_impl(state_json, intervention_type, params_json))
}

#[wasm_bindgen]
pub fn terminate(state_json: &str) -> Result<String, JsValue> {
    to_js_result(terminate_impl(state_json))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_world_with_two_founders() -> String {
        let world = create_world_impl(38.0, 35.0).expect("create_world should succeed");
        let mut state: serde_json::Value = serde_json::from_str(&world).unwrap();
        let f1 = create_founder_json_impl(r#"{"sex":"male","ageYears":22,"x":35.0,"y":38.0,"name":"Adam"}"#).unwrap();
        let f2 = create_founder_json_impl(r#"{"sex":"female","ageYears":20,"x":35.1,"y":38.1,"name":"Eve"}"#).unwrap();
        let f1: serde_json::Value = serde_json::from_str(&f1).unwrap();
        let f2: serde_json::Value = serde_json::from_str(&f2).unwrap();
        state["individuals"] = serde_json::json!([f1, f2]);
        serde_json::to_string(&state).unwrap()
    }

    #[test]
    fn create_simulation_seeds_a_two_founder_paused_simulation() {
        let result = create_simulation_impl(Some("Test Civ".to_string()), 38.0, 35.0, "{}", "{}").expect("create_simulation should succeed");
        let sim: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(sim["name"].as_str().unwrap(), "Test Civ");
        assert_eq!(sim["status"].as_str().unwrap(), "paused");
        assert!(sim["user_id"].is_null());
        let individuals = sim["individuals"].as_array().unwrap();
        assert_eq!(individuals.len(), 2);
        assert!(individuals.iter().all(|i| i["is_founder"].as_bool().unwrap()));
        assert_eq!(sim["discovered_techs"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn full_round_trip_through_every_export_works_end_to_end() {
        let mut state_json = seed_world_with_two_founders();

        for _ in 0..30 {
            let result = advance_day_impl(&state_json, "[]").expect("advance_day should succeed");
            let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
            state_json = serde_json::to_string(&parsed["state"]).unwrap();
        }

        let stats = get_stats_impl(&state_json).expect("get_stats should succeed");
        let stats: serde_json::Value = serde_json::from_str(&stats).unwrap();
        assert_eq!(stats["population"].as_i64().unwrap(), 2);

        let population = get_population_impl(&state_json, Some(true), None).expect("get_population should succeed");
        let population: Vec<serde_json::Value> = serde_json::from_str(&population).unwrap();
        assert_eq!(population.len(), 2);
        let first_id = population[0]["id"].as_str().unwrap().to_string();

        let individual = get_individual_impl(&state_json, &first_id).expect("get_individual should succeed");
        let individual: serde_json::Value = serde_json::from_str(&individual).unwrap();
        assert_eq!(individual["id"].as_str().unwrap(), first_id);

        let missing = get_individual_impl(&state_json, "no-such-id").expect("get_individual should succeed even for a miss");
        assert_eq!(missing, "null");

        let events = get_events_impl(&state_json).expect("get_events should succeed");
        let events: Vec<serde_json::Value> = serde_json::from_str(&events).unwrap();
        assert!(events.iter().all(|e| e["description"].is_string()));

        let summary = get_events_summary_impl(&state_json).expect("get_events_summary should succeed");
        let summary: serde_json::Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(summary["total"].as_i64().unwrap(), events.len() as i64);

        let intervened = apply_intervention_impl(&state_json, "resource_boost", "{}").expect("apply_intervention should succeed");
        let intervened: serde_json::Value = serde_json::from_str(&intervened).unwrap();
        assert!(intervened["affected_individuals"].as_i64().unwrap() > 0);
        state_json = serde_json::to_string(&intervened["state"]).unwrap();

        let err = apply_intervention_impl(&state_json, "not_a_real_type", "{}").unwrap_err();
        assert!(err.contains("Unknown intervention type"));

        let terminated = terminate_impl(&state_json).expect("terminate should succeed");
        let terminated: serde_json::Value = serde_json::from_str(&terminated).unwrap();
        assert_eq!(terminated["status"].as_str().unwrap(), "completed");
        assert!(terminated["individuals"].as_array().unwrap().iter().all(|i| i["is_dead"].as_bool().unwrap_or(false)));
    }

    #[test]
    fn disabled_engines_are_re_applied_every_call_and_never_persisted_in_state_json() {
        let mut state_json = seed_world_with_two_founders();

        for _ in 0..30 {
            let result = advance_day_impl(&state_json, r#"["economy"]"#).expect("advance_day should succeed");
            let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
            // SimulationState.disabled_engines is #[serde(skip)] by design (see
            // its doc comment in state.rs) -- it must never round-trip through
            // state_json, otherwise callers relying on the caller-owned set
            // (as this fix requires) would silently disagree with what's
            // actually serialized.
            assert!(!parsed["state"].as_object().unwrap().contains_key("disabled_engines"));
            state_json = serde_json::to_string(&parsed["state"]).unwrap();
        }

        // Economy is the only engine that ever populates inventory (it lazily
        // initializes it on first gather) -- if the disabled set were only
        // honored on the first call and then silently dropped (the original
        // bug: state_json can't carry it because of #[serde(skip)]), some of
        // these 30 calls would have run economy for real and inventory would
        // be non-empty here.
        let state: serde_json::Value = serde_json::from_str(&state_json).unwrap();
        for individual in state["individuals"].as_array().unwrap() {
            assert!(individual["inventory"].as_object().unwrap().is_empty(), "economy should have stayed disabled on every call");
        }
    }
}
