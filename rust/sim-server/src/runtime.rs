use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Mutex as StdMutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tokio::{sync::Mutex, task::JoinHandle, time::sleep};

use crate::db::{
    insert_checkpoint, latest_checkpoint_day, load_bounded_tick_state_no_genealogy, load_genealogy_index, load_simulation, save_tick_progress,
    update_simulation_fields, upsert_individuals, DbBackend,
};
use serde_json::json;
use sim_core::{advance_one_day, derive_stats, PhaseTimings, DEAD_FIELD_STRIP_GRACE_DAYS};
use uuid::Uuid;

/// How many sim-days between automatic checkpoints (see the `should_flush`
/// block in `runtime_loop`). Previously checkpoints were only ever created
/// by the player clicking "Save Now" in TimeMachinePanel -- runs nobody
/// manually checkpointed left the Report's Population History (and the
/// Time Machine restore list) completely empty. 91 days (one season) means
/// ReportPanel's own "every 4th checkpoint" decimation lands on
/// approximately annual samples, matching what that panel's comment already
/// assumed.
const AUTO_CHECKPOINT_INTERVAL_DAYS: i32 = 91;

/// Ported from the old Node engine's `_runStartupValidation()`/`_errorLog`/
/// `_consecutiveErrors`: a one-time sanity check when a simulation's tick
/// loop starts, plus a rolling log of any panic `advance_one_day` throws
/// while running. Node's engine object lived in memory for the simulation's
/// whole lifetime, so it could just keep these as instance fields; Rust's
/// runtime_loop is the closest equivalent (also long-lived, one per running
/// simulation), so this lives alongside TickTiming as another piece of
/// state a session exposes to GET /:id/diagnostics.
#[derive(Clone, Default)]
pub struct DiagnosticsState {
    pub startup: Option<StartupLog>,
    pub error_log: VecDeque<ErrorLogEntry>,
    pub consecutive_errors: u32,
}

const ERROR_LOG_CAPACITY: usize = 20;
/// Matches the old Node engine's own threshold: after this many consecutive
/// tick failures the simulation auto-pauses rather than retrying forever.
const MAX_CONSECUTIVE_ERRORS: u32 = 5;

/// Caps how many simulated days a single runtime_loop iteration computes
/// (and holds in memory) before the batch is saved to the DB. `db.rs`'s
/// `DEAD_UPSERT_GRACE_DAYS` must stay >= this: an individual who dies on the
/// very first day of a batch is only ever eligible for an upsert once, at
/// the *end* of that whole batch, by which point `state.current_day` has
/// already advanced by up to this many days past their `death_day`.
pub(crate) const MAX_BATCH_SIZE: usize = 100;

/// While upload is paused, the tick loop skips its own per-batch DB writes
/// entirely -- but still forces one anyway after this long, so a server
/// restart/crash mid-pause loses at most a few minutes of progress instead
/// of the whole paused stretch. Does not apply once upload is resumed: the
/// very next iteration always flushes (see `runtime_loop`'s `should_flush`).
const UPLOAD_PAUSE_SAFETY_FLUSH: Duration = Duration::from_secs(300);

/// How often live_state actually gets a fresh clone while paused. Every
/// reader of it (ws.rs's ~1s broadcast tick, the Population panel's ~5s
/// poll) already can't observe updates faster than its own poll interval,
/// so re-cloning the whole in-memory population on every single tick-loop
/// iteration (which, at high speed, can run many times a second) was pure
/// waste -- real cost that scaled with population size, paid for
/// "freshness" no client could ever actually see.
const LIVE_STATE_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Serialize)]
pub struct StartupCheck {
    pub ok: bool,
    pub name: String,
    /// Structured, language-neutral values only (counts, biome/season ids,
    /// day/year numbers) -- this used to be a pre-formatted Turkish
    /// sentence ("toplam {}, yaşayan {}", "biome={b}, mevsim={}"), which
    /// PerformancePanel.tsx then rendered completely unlocalized regardless
    /// of the user's actual `lang` setting (every other string in that
    /// panel is properly run through `t(...)`). The server has no notion of
    /// which language a given client/device is displaying in -- and
    /// multiple devices watching the same simulation could each have a
    /// different one -- so formatting the human-readable sentence has to
    /// happen client-side, from these raw values.
    pub detail: serde_json::Value,
}

#[derive(Clone, Serialize)]
pub struct StartupLog {
    pub ts: i64,
    pub day: i32,
    pub checks: Vec<StartupCheck>,
}

#[derive(Clone, Serialize)]
pub struct ErrorLogEntry {
    pub day: i32,
    pub ts: i64,
    pub msg: String,
    pub stack: String,
}

fn now_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn run_startup_checks(state: &sim_core::SimulationState) -> StartupLog {
    let mut checks = Vec::new();
    checks.push(StartupCheck { ok: true, name: "PATHOGEN_TYPES".to_string(), detail: json!({ "count": sim_core::PATHOGEN_TYPES.len() }) });
    let alive = state.individuals.iter().filter(|i| i.alive && !i.is_dead).count();
    checks.push(StartupCheck { ok: true, name: "population".to_string(), detail: json!({ "total": state.individuals.len(), "alive": alive }) });
    checks.push(match state.world_state.biome.as_deref() {
        Some(b) => StartupCheck {
            ok: true,
            name: "world_state".to_string(),
            detail: json!({ "biome": b, "season": state.world_state.season }),
        },
        None => StartupCheck { ok: false, name: "world_state".to_string(), detail: json!({}) },
    });
    checks.push(StartupCheck { ok: true, name: "sim_day".to_string(), detail: json!({ "day": state.current_day, "year": state.current_day / 365 }) });
    StartupLog { ts: now_millis(), day: state.current_day, checks }
}

/// Extracts a printable message from a caught panic's payload -- the same
/// "downcast to &str, then String, else give up" dance std's own default
/// panic hook does internally, since `Box<dyn Any>` doesn't implement
/// Display.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// How long to sleep between each individual day inside a batch. Reserves
/// `predicted_db_overhead_ms` (the *previous* iteration's load+save+upsert
/// time -- this iteration's own isn't known yet, since save/upsert haven't
/// run) out of `target_delay_ms` before dividing the rest across the batch,
/// so a batch's total wall-clock time (pacing + the real DB round trip)
/// stays close to `target_delay_ms` instead of the DB round trip stacking on
/// top of the full pacing budget -- the actual selected speed is what the
/// simulation runs at, not selected-speed-plus-however-slow-the-DB-is (see
/// PR #101, which fixed this at the whole-iteration level).
///
/// The *whole batch's* per-day sleeps are additionally floored to span at
/// least `MIN_BATCH_SPAN_MS` in total (divided evenly per day), not just
/// each individual day's sleep floored to some small constant -- a flat
/// per-day floor is the wrong shape, since at a large batch_size (high
/// speed) many tiny per-day floors still sum to a span too short for ws.rs's
/// 120ms `fast_tick` poll to land more than once inside it. Spreading a
/// fixed total span instead means several polls land inside it regardless
/// of batch_size, so `live_day` (which that poll reads to drive the
/// on-screen day counter) advances in visible steps across a batch instead
/// of by one DB-overhead-sized jump, even when overhead has eaten most or
/// all of the natural per-day budget. A batch with little DB overhead
/// relative to its budget is untouched either way, since `max` only ever
/// raises the per-day delay above what the natural share would give.
///
/// This still can't make the counter advance *exactly* one day at a time --
/// only reducing DB round-trip latency itself (e.g. keeping the Fly machine
/// and its Postgres instance in the same region) shrinks how much of a
/// batch's total time is DB overhead the counter must sit frozen through
/// versus how much is this function's own paced, visibly-counting span.
const MIN_BATCH_SPAN_MS: u64 = 500;

fn compute_per_day_delay_ms(target_delay_ms: u64, batch_size: usize, fast_forwarding: bool, predicted_db_overhead_ms: u64) -> u64 {
    if fast_forwarding {
        return 0;
    }
    let batch_size = batch_size.max(1) as u64;
    let pacing_budget_ms = target_delay_ms.saturating_sub(predicted_db_overhead_ms);
    let natural_per_day = pacing_budget_ms / batch_size;
    let min_per_day_for_smooth_span = MIN_BATCH_SPAN_MS / batch_size;
    natural_per_day.max(min_per_day_for_smooth_span)
}

/// Whether this iteration should actually write to the DB. Not paused always
/// flushes (existing behavior, unchanged) -- which is also what makes a
/// resume need no separate "flush now" step, since the very next iteration
/// after `resume_upload` reads `paused == false` and lands here. While
/// paused, only the periodic safety net forces a write.
fn should_flush_upload(paused: bool, elapsed_since_last_flush: Duration, safety_flush_interval: Duration) -> bool {
    !paused || elapsed_since_last_flush >= safety_flush_interval
}

/// Whether upsert_individuals' expensive transitive-ancestor walk
/// (`include_ancestors`) should still be requested on the *next* flush,
/// given whether this one just ran (successfully or not) and whether
/// upload is currently paused. See `full_resync_needed`'s own declaration
/// comment in `runtime_loop` for the full reasoning; in short: once a
/// non-paused flush succeeds, every ancestor still referenced by anyone
/// eligible is already correctly persisted, so subsequent non-paused
/// flushes can skip the walk -- but for as long as upload stays paused,
/// each successful flush still leaves the next one owing a full pass
/// (paused is exactly the scenario the walk exists to cover). A failed
/// upsert didn't persist anything new either way, so it doesn't change
/// what's still owed.
fn next_full_resync_needed(current: bool, paused: bool, upsert_ok: bool) -> bool {
    if !upsert_ok {
        return current;
    }
    paused
}

/// Whether enough sim-days have passed since the last checkpoint (auto- or
/// manually-saved) to justify creating another one. `last_checkpoint_day ==
/// None` means no checkpoint exists yet for this simulation at all, in which
/// case the very first one still waits for a full interval rather than
/// firing immediately on day 0.
fn is_due_for_checkpoint(last_checkpoint_day: Option<i32>, current_day: i32, interval_days: i32) -> bool {
    match last_checkpoint_day {
        Some(last) => current_day - last >= interval_days,
        None => current_day >= interval_days,
    }
}

/// Whether an individual should still occupy this process's in-memory
/// working set right after a confirmed-successful flush while paused.
/// Mirrors load_bounded_tick_state_no_genealogy's own cutoff (db.rs) -- the
/// same window a fresh, unpaused reload would exclude someone past -- so a
/// session paused indefinitely doesn't let `state.individuals` (and the
/// live_state clone built from it) grow forever just because nothing else
/// ever reloads and re-bounds it while paused. Only ever applied after a
/// flush that's known to have persisted everyone currently in memory (see
/// upsert_individuals' own transitive parent expansion in db.rs), so
/// dropping someone here can never lose data no save ever wrote down.
fn should_keep_in_memory_after_flush(alive: bool, death_day: Option<i32>, current_day: i32) -> bool {
    alive || death_day.is_none_or(|d| current_day - d < DEAD_FIELD_STRIP_GRACE_DAYS)
}

/// Per-simulation tick throughput, sampled directly in the runtime loop
/// below (the same measurement the speed-throttle delay uses) and surfaced
/// via GET /:id/metrics for the Performance panel. `sample_count == 0`
/// means the loop hasn't completed a batch yet (simulation just started,
/// or is paused).
#[derive(Clone, Copy, Default)]
pub struct TickTiming {
    pub last_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
    pub min_ms: f64,
    pub ticks_per_second: f64,
    pub sample_count: u64,
    // Breakdown of the last iteration's total time, so a slow tick can be
    // attributed to network/DB latency vs. the actual sim computation
    // instead of guessing -- these are per-batch totals (not per-day).
    pub last_load_ms: f64,
    pub last_compute_ms: f64,
    pub last_save_ms: f64,
    pub last_upsert_ms: f64,
    // Sub-breakdown of last_compute_ms by engine group (see
    // sim_core::PhaseTimings) -- lets the Performance panel's "MODULE /
    // PERFORMANCE" block show which part of the sim itself is slow, not just
    // that compute as a whole is slow.
    pub last_phases: PhaseTimings,
}

#[derive(Clone, Default)]
pub struct RuntimeManager {
    sessions: Arc<Mutex<HashMap<String, RuntimeSession>>>,
}

struct RuntimeSession {
    stop: Arc<AtomicBool>,
    fast_forward_target: Arc<AtomicI32>,
    tick_timing: Arc<StdMutex<TickTiming>>,
    diagnostics: Arc<StdMutex<DiagnosticsState>>,
    live_day: Arc<AtomicI32>,
    upload_paused: Arc<AtomicBool>,
    live_state: Arc<StdMutex<Option<sim_core::SimulationState>>>,
    /// Diagnostic-only per-engine on/off toggles (see
    /// sim_core::TOGGLEABLE_ENGINES) -- refreshed into the tick loop's
    /// `state.disabled_engines` every iteration (see runtime_loop), same
    /// pattern as upload_paused, so a toggle takes effect on the very next
    /// batch regardless of whether upload is paused.
    disabled_engines: Arc<StdMutex<HashSet<String>>>,
    handle: JoinHandle<()>,
}

impl RuntimeManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(&self, backend: DbBackend, sim_id: String) {
        let mut sessions = self.sessions.lock().await;
        if let Some(existing) = sessions.get(&sim_id) {
            if !existing.handle.is_finished() {
                return;
            }
            sessions.remove(&sim_id);
        }
        if sessions.contains_key(&sim_id) {
            return;
        }

        let stop = Arc::new(AtomicBool::new(false));
        let fast_forward_target = Arc::new(AtomicI32::new(-1));
        let tick_timing = Arc::new(StdMutex::new(TickTiming::default()));
        let diagnostics = Arc::new(StdMutex::new(DiagnosticsState::default()));
        let live_day = Arc::new(AtomicI32::new(-1));
        let upload_paused = Arc::new(AtomicBool::new(false));
        let live_state = Arc::new(StdMutex::new(None));
        let disabled_engines = Arc::new(StdMutex::new(HashSet::new()));
        let stop_clone = Arc::clone(&stop);
        let target_clone = Arc::clone(&fast_forward_target);
        let timing_clone = Arc::clone(&tick_timing);
        let diagnostics_clone = Arc::clone(&diagnostics);
        let live_day_clone = Arc::clone(&live_day);
        let upload_paused_clone = Arc::clone(&upload_paused);
        let live_state_clone = Arc::clone(&live_state);
        let disabled_engines_clone = Arc::clone(&disabled_engines);
        let sim_id_clone = sim_id.clone();
        let sessions_clone = Arc::clone(&self.sessions);

        let handle = tokio::spawn(async move {
            runtime_loop(
                backend,
                sim_id_clone,
                stop_clone,
                target_clone,
                timing_clone,
                diagnostics_clone,
                live_day_clone,
                upload_paused_clone,
                live_state_clone,
                disabled_engines_clone,
                sessions_clone,
            )
            .await;
        });

        sessions.insert(
            sim_id,
            RuntimeSession {
                stop,
                fast_forward_target,
                tick_timing,
                diagnostics,
                live_day,
                upload_paused,
                live_state,
                disabled_engines,
                handle,
            },
        );
    }

    pub async fn pause(&self, sim_id: &str) {
        let _ = sim_id;
    }

    /// Signals the tick loop to stop AND waits for it to actually exit
    /// before returning. Callers that delete the simulation's DB row right
    /// after calling this (delete_simulation_route, terminate_simulation)
    /// depend on that: save_state/upsert_individuals are upserts, so a tick
    /// still in flight when the row is deleted would simply re-insert it on
    /// its next save -- resurrecting a "deleted" simulation. Only returning
    /// once the loop has truly stopped closes that window.
    ///
    /// Removing the session and setting `stop` must happen under the same
    /// lock acquisition, not two separate ones: releasing the lock between
    /// them left a window where a concurrent start() (which only checks
    /// whether the map already holds a live session) could find the map
    /// empty and spawn a second tick loop for this sim_id before the first
    /// loop had any way to learn it should stop -- two loops then racing to
    /// load/tick/save the same simulation, right as this function's own
    /// caller (terminate_simulation) is about to write the archived
    /// mass-death state that reasoning above exists to protect.
    pub async fn terminate(&self, sim_id: &str) {
        let session = {
            let mut sessions = self.sessions.lock().await;
            let session = sessions.remove(sim_id);
            if let Some(session) = &session {
                session.stop.store(true, Ordering::SeqCst);
            }
            session
        };
        if let Some(session) = session {
            let _ = session.handle.await;
        }
    }

    /// Skips the tick loop's per-batch DB writes (save_tick_progress /
    /// upsert_individuals) while set -- computation keeps running in memory
    /// regardless, at the cost of a bounded data-loss window if the process
    /// dies mid-pause (see UPLOAD_PAUSE_SAFETY_FLUSH). A no-op if the
    /// simulation has no active session (not started, or already stopped).
    pub async fn pause_upload(&self, sim_id: &str) {
        if let Some(session) = self.sessions.lock().await.get(sim_id) {
            session.upload_paused.store(true, Ordering::SeqCst);
        }
    }

    /// Resumes normal per-batch DB writes. No separate "flush now" call is
    /// needed: the tick loop's own `should_flush` check (see runtime_loop)
    /// always flushes on the first iteration it observes `upload_paused` as
    /// false, pushing everything accumulated in memory while paused.
    pub async fn resume_upload(&self, sim_id: &str) {
        if let Some(session) = self.sessions.lock().await.get(sim_id) {
            session.upload_paused.store(false, Ordering::SeqCst);
        }
    }

    pub async fn is_upload_paused(&self, sim_id: &str) -> bool {
        match self.sessions.lock().await.get(sim_id) {
            Some(session) => session.upload_paused.load(Ordering::SeqCst),
            None => false,
        }
    }

    /// The tick loop's own most-recent in-memory state -- only ever kept in
    /// sync while upload is paused (see runtime_loop), `None` otherwise,
    /// since the DB is already at least as fresh whenever uploads are
    /// flowing normally. Lets population/stats/event reads bypass a DB
    /// that's deliberately not being written to, instead of appearing
    /// frozen the moment upload is paused.
    pub async fn live_state(&self, sim_id: &str) -> Option<sim_core::SimulationState> {
        let sessions = self.sessions.lock().await;
        let session = sessions.get(sim_id)?;
        let state = session.live_state.lock().unwrap_or_else(|e| e.into_inner()).clone();
        state
    }

    /// Replaces the full set of diagnostic-only disabled engines for this
    /// session (see sim_core::TOGGLEABLE_ENGINES). A no-op if the simulation
    /// has no active session. Callers are expected to have already validated
    /// each name against TOGGLEABLE_ENGINES -- this layer doesn't re-check,
    /// it just stores whatever set it's given.
    pub async fn set_disabled_engines(&self, sim_id: &str, engines: HashSet<String>) {
        if let Some(session) = self.sessions.lock().await.get(sim_id) {
            *session.disabled_engines.lock().unwrap_or_else(|e| e.into_inner()) = engines;
        }
    }

    pub async fn disabled_engines(&self, sim_id: &str) -> HashSet<String> {
        let sessions = self.sessions.lock().await;
        let Some(session) = sessions.get(sim_id) else {
            return HashSet::new();
        };
        let engines = session.disabled_engines.lock().unwrap_or_else(|e| e.into_inner()).clone();
        engines
    }

    pub async fn fast_forward(&self, sim_id: &str, target_day: i32) {
        let sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get(sim_id) {
            session.fast_forward_target.store(target_day, Ordering::SeqCst);
            session.stop.store(false, Ordering::SeqCst);
        }
    }

    pub async fn cancel_fast_forward(&self, sim_id: &str) {
        if let Some(session) = self.sessions.lock().await.get(sim_id) {
            session.fast_forward_target.store(-1, Ordering::SeqCst);
        }
    }

    /// True while a fast-forward target is active for this simulation, mirroring
    /// the JS engine's `is_warping` metric (derived from `_fastForwardTarget`).
    pub async fn is_fast_forwarding(&self, sim_id: &str) -> bool {
        match self.sessions.lock().await.get(sim_id) {
            Some(session) => session.fast_forward_target.load(Ordering::SeqCst) >= 0,
            None => false,
        }
    }

    pub async fn tick_timing(&self, sim_id: &str) -> Option<TickTiming> {
        let sessions = self.sessions.lock().await;
        let session = sessions.get(sim_id)?;
        let timing = session.tick_timing.lock().unwrap_or_else(|e| e.into_inner());
        (timing.sample_count > 0).then_some(*timing)
    }

    /// `None` when the simulation has no active tick-loop session (e.g. it
    /// was never started this process's lifetime, or has since been
    /// paused/completed and its session removed) -- GET /:id/diagnostics
    /// falls back to an empty startup/error_log in that case rather than
    /// treating it as an error, same as tick_timing's own None case.
    pub async fn diagnostics(&self, sim_id: &str) -> Option<DiagnosticsState> {
        let sessions = self.sessions.lock().await;
        let session = sessions.get(sim_id)?;
        let state = session.diagnostics.lock().unwrap_or_else(|e| e.into_inner()).clone();
        Some(state)
    }

    /// The day the tick loop is *at right now*, mid-batch -- updated after every
    /// single simulated day (see runtime_loop's per-day pacing) rather than only
    /// once the whole batch is saved to the DB. Lets a frequent, lightweight
    /// websocket poll show the day counter climbing smoothly instead of jumping
    /// by a whole batch (up to `speed` days) once per second. `None` before the
    /// loop has processed its first day (session just started).
    pub async fn live_day(&self, sim_id: &str) -> Option<i32> {
        let sessions = self.sessions.lock().await;
        let day = sessions.get(sim_id)?.live_day.load(Ordering::SeqCst);
        (day >= 0).then_some(day)
    }
}

#[allow(clippy::too_many_arguments)]
async fn runtime_loop(
    backend: DbBackend,
    sim_id: String,
    stop: Arc<AtomicBool>,
    fast_forward_target: Arc<AtomicI32>,
    tick_timing: Arc<StdMutex<TickTiming>>,
    diagnostics: Arc<StdMutex<DiagnosticsState>>,
    live_day: Arc<AtomicI32>,
    upload_paused: Arc<AtomicBool>,
    live_state: Arc<StdMutex<Option<sim_core::SimulationState>>>,
    disabled_engines: Arc<StdMutex<HashSet<String>>>,
    sessions: Arc<Mutex<HashMap<String, RuntimeSession>>>,
) {
    // Incrementally-maintained genealogy cache for this loop's lifetime.
    // `load_genealogy_index` used to be re-fetched in full (everyone ever
    // born in the simulation) on every single batch, which only ever grew
    // more expensive as a simulation aged -- see its own doc comment.
    // `genealogy_watermark` is the current_day loaded *before* each batch
    // ran; `None` on the very first iteration triggers one full load, and
    // every iteration after that only fetches the delta born since that
    // watermark (entries are immutable once written, so merging deltas is
    // always correct). advance_one_day already updates state.genealogy
    // in-memory for anyone born mid-batch (tick.rs), so taking it back out
    // after the batch (instead of cloning it) captures those for free too.
    let mut genealogy_cache: sim_core::GenealogyIndex = sim_core::GenealogyIndex::new();
    let mut genealogy_watermark: Option<i32> = None;
    let mut startup_checked = false;
    // Carries `state` across iterations while upload is paused, instead of
    // always reloading from the DB -- see the `need_full_load` check below.
    // `None` forces a full load (loop just started, or the previous
    // iteration didn't have a state to hand off).
    let mut carried_state: Option<sim_core::SimulationState> = None;
    let mut last_upload_flush = Instant::now();
    // Whether upsert_individuals' expensive transitive-ancestor walk is
    // still owed (see its own doc comment). Starts `true` -- conservative
    // on every fresh loop start/resume, since this process can't know
    // whether everyone currently eligible-or-referenced was actually
    // persisted before it started watching. Cleared after the first
    // successful upsert while not paused; forced back on for the whole
    // duration of any pause (below), since a paused run is exactly the
    // scenario the ancestor walk exists to cover.
    let mut full_resync_needed = true;
    // `None` until the first real flush below has a chance to seed it from
    // whatever checkpoint (auto- or manually-saved) already exists in the DB
    // -- without that seed, a server restart mid-run would forget how
    // recently the last checkpoint landed and immediately create another one
    // even if it was only moments ago.
    let mut last_checkpoint_day: Option<i32> = None;
    let mut checkpoint_day_seeded = false;
    // Starts elapsed enough to refresh live_state on the very first paused
    // iteration rather than waiting a full LIVE_STATE_REFRESH_INTERVAL first.
    let mut last_live_state_refresh = Instant::now() - LIVE_STATE_REFRESH_INTERVAL;

    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }

        // Read once per iteration: gates both which load path runs below and
        // whether this iteration's compute gets flushed to the DB. A stale
        // read for one iteration is harmless -- pause/resume already only
        // ever takes effect "on the next iteration" by design.
        let paused = upload_paused.load(Ordering::SeqCst);
        let need_full_load = carried_state.is_none() || !paused;

        let iteration_start = Instant::now();
        let load_start = Instant::now();
        let mut state = if need_full_load {
            // Independent queries -- run concurrently rather than paying two
            // sequential round trips.
            let (state_result, genealogy_result) = tokio::join!(
                load_bounded_tick_state_no_genealogy(&backend, &sim_id),
                load_genealogy_index(&backend, &sim_id, genealogy_watermark),
            );
            let mut state = match state_result {
                Ok(Some(state)) => state,
                _ => break,
            };
            let genealogy_delta = match genealogy_result {
                Ok(delta) => delta,
                Err(err) => {
                    eprintln!("[runtime] load_genealogy_index failed for {sim_id}: {err}");
                    break;
                }
            };
            genealogy_cache.extend(genealogy_delta);
            genealogy_watermark = Some(state.current_day);
            state.genealogy = std::mem::take(&mut genealogy_cache);
            state
        } else {
            // Upload paused and a batch's worth of state is already carried
            // in memory from the previous iteration -- skip the expensive
            // per-individual reload entirely (that's the whole point of
            // pausing) but still cheaply refresh status/speed_multiplier
            // from the `simulations` row alone, so a pause/resume/speed
            // change made elsewhere (those only ever write that one row --
            // see pause_simulation/set_speed) still takes effect on
            // schedule instead of being invisible until upload resumes.
            let mut state = carried_state.take().expect("need_full_load is false only when carried_state is Some");
            match load_simulation(&backend, &sim_id).await {
                Ok(Some(row)) => {
                    state.status = Some(row.status);
                    if let Some(speed) = row.speed_multiplier {
                        state.speed_multiplier = Some(speed);
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    eprintln!("[runtime] load_simulation (paused-upload refresh) failed for {sim_id}: {err}");
                    break;
                }
            }
            state
        };
        // Diagnostic engine toggles live on the session (like upload_paused),
        // not the DB -- refreshed every iteration regardless of which load
        // path ran above, so a toggle flipped mid-pause still takes effect
        // on the very next batch instead of waiting for a full reload.
        state.disabled_engines = disabled_engines.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let load_ms = load_start.elapsed().as_secs_f64() * 1000.0;
        let status = state.status.clone().unwrap_or_default();
        if status == "completed" || status == "terminated" {
            break;
        }
        if status != "running" {
            carried_state = Some(state);
            sleep(Duration::from_millis(250)).await;
            continue;
        }

        if !startup_checked {
            startup_checked = true;
            let log = run_startup_checks(&state);
            diagnostics.lock().unwrap_or_else(|e| e.into_inner()).startup = Some(log);
        }

        let target = fast_forward_target.load(Ordering::SeqCst);
        let fast_forwarding = target >= 0;
        if fast_forwarding && state.current_day >= target {
            fast_forward_target.store(-1, Ordering::SeqCst);
            sleep(Duration::from_millis(50)).await;
            continue;
        }

        let speed = state.speed_multiplier.unwrap_or(1).clamp(1, 1000) as usize;
        let batch_size = if fast_forwarding {
            let remaining = (target - state.current_day).max(1) as usize;
            remaining.min(MAX_BATCH_SIZE)
        } else {
            // Cap the burst size so a single loop iteration never blocks the
            // task for too long, but keep it proportional to `speed` -- the
            // delay below is derived from batch_size so the two no longer
            // both scale with speed independently (that double-counting is
            // what caused the UI's speed selector to be ignored: throughput
            // would rocket past the chosen multiplier and flatline once
            // both values hit their old hardcoded clamps).
            speed.min(MAX_BATCH_SIZE)
        };

        // Chosen so batch_size / delay == speed days/sec, i.e. the "Nx" label in
        // the UI matches actual throughput. Computed once up front (rather than
        // only after the batch, as before) so it can also drive the per-day
        // pacing below -- see that comment for why the delay isn't just applied
        // in one lump at the end.
        let target_delay_ms = (batch_size as u64 * 1000 / speed as u64).max(20);
        // Spreads target_delay_ms evenly across the batch instead of computing
        // all `batch_size` days back-to-back and only then sleeping. Without
        // this, the day counter jumps by a whole batch (up to 100 days) once
        // per second instead of counting up smoothly -- the tick loop reloads
        // from the DB every iteration rather than keeping live state across
        // iterations like the old JS engine did, so batching was needed to
        // afford one DB round trip per many simulated days, but nothing then
        // paced *within* a batch. live_day (below) is updated after every
        // single day so a frequent, lightweight websocket poll can show
        // smooth progress even though the DB itself only advances once per
        // batch. per_day_delay_ms reserves a predicted DB-overhead slice of
        // target_delay_ms first -- see compute_per_day_delay_ms's own
        // comment for why (getting this wrong reintroduces the exact "speed
        // setting throttled below what's selected" bug PR #101 fixed).
        let predicted_db_overhead_ms = {
            let timing = tick_timing.lock().unwrap_or_else(|e| e.into_inner());
            if timing.sample_count > 0 { (timing.last_load_ms + timing.last_save_ms + timing.last_upsert_ms) as u64 } else { 0 }
        };
        let per_day_delay_ms = compute_per_day_delay_ms(target_delay_ms, batch_size, fast_forwarding, predicted_db_overhead_ms);

        // advance_one_day is synchronous, CPU-bound Rust -- with no .await inside
        // this loop, running it directly here would tie up one of this process's
        // handful of tokio worker threads (TOKIO_WORKER_THREADS, capped at 2 by
        // default) for the entire batch. As tick cost grows with population, a
        // batch of up to 100 days can take seconds, during which every other
        // request scheduled on that thread -- including /api/health -- misses
        // Render's 5s health-check window and the instance gets killed and
        // restarted. spawn_blocking moves the computation onto tokio's separate
        // blocking-thread-pool, so the async worker threads stay free to serve
        // HTTP requests no matter how long a batch takes.
        let stop_for_compute = Arc::clone(&stop);
        let target_for_compute = Arc::clone(&fast_forward_target);
        let live_day_for_compute = Arc::clone(&live_day);
        let diagnostics_for_compute = Arc::clone(&diagnostics);
        // real_compute tracks only time spent inside advance_one_day itself,
        // excluding the per-day pacing sleep above -- the Performance panel's
        // "Tick Timing" section is meant to show actual engine cost so a slow
        // tick can be attributed to real compute vs. DB latency, and would be
        // meaningless if it also counted time this loop deliberately idled.
        let spawn_result = tokio::task::spawn_blocking(move || {
            let mut real_compute = Duration::ZERO;
            let mut phases_total = PhaseTimings::default();
            for _ in 0..batch_size {
                if stop_for_compute.load(Ordering::SeqCst) {
                    break;
                }
                let current_target = target_for_compute.load(Ordering::SeqCst);
                if current_target >= 0 && state.current_day >= current_target {
                    target_for_compute.store(-1, Ordering::SeqCst);
                    break;
                }
                let day_start = Instant::now();
                // Ported from the old Node engine's per-tick try/catch (see
                // AGENTS.md/this session's history): a panic deep in sim-core
                // (a rare edge case -- extensive stress testing this session
                // never hit one) would otherwise unwind straight through
                // spawn_blocking's JoinHandle and kill this whole runtime_loop
                // task, silently freezing the simulation with its DB status
                // still "running" and no trace of what happened. Catching it
                // here instead logs it and, after MAX_CONSECUTIVE_ERRORS in a
                // row, auto-pauses the simulation below -- same threshold and
                // behavior Node had.
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| advance_one_day(&mut state))) {
                    Ok((_, day_phases)) => {
                        phases_total.accumulate(&day_phases);
                        let mut diag = diagnostics_for_compute.lock().unwrap_or_else(|e| e.into_inner());
                        diag.consecutive_errors = 0;
                    }
                    Err(payload) => {
                        let mut diag = diagnostics_for_compute.lock().unwrap_or_else(|e| e.into_inner());
                        diag.consecutive_errors += 1;
                        if diag.error_log.len() >= ERROR_LOG_CAPACITY {
                            diag.error_log.pop_front();
                        }
                        diag.error_log.push_back(ErrorLogEntry {
                            day: state.current_day,
                            ts: now_millis(),
                            msg: panic_message(payload.as_ref()),
                            stack: String::new(), // std::panic::catch_unwind's payload carries no backtrace
                        });
                        let should_stop = diag.consecutive_errors >= MAX_CONSECUTIVE_ERRORS;
                        drop(diag);
                        if should_stop {
                            stop_for_compute.store(true, Ordering::SeqCst);
                            break;
                        }
                        // Don't retry the same day in this batch -- move on
                        // like Node did (currentDay++ even on error), rather
                        // than risk spinning on a day that panics every time.
                        state.current_day += 1;
                    }
                }
                let day_compute_elapsed = day_start.elapsed();
                real_compute += day_compute_elapsed;
                live_day_for_compute.store(state.current_day, Ordering::SeqCst);
                if per_day_delay_ms > 0 {
                    let day_remaining = per_day_delay_ms.saturating_sub(day_compute_elapsed.as_millis() as u64);
                    if day_remaining > 0 {
                        std::thread::sleep(Duration::from_millis(day_remaining));
                    }
                }
                if current_target >= 0 && state.current_day >= current_target {
                    target_for_compute.store(-1, Ordering::SeqCst);
                    break;
                }
            }
            (state, real_compute, phases_total)
        })
        .await;
        let (state_after, real_compute, phases_total) = match spawn_result {
            Ok(result) => result,
            Err(join_err) => {
                // The per-day catch_unwind above only guards advance_one_day
                // itself. A panic anywhere else in this closure (or the task
                // being cancelled, e.g. on process shutdown) surfaces here as
                // a JoinError instead. This used to be `.expect(...)`'d,
                // which unwound straight through runtime_loop -- skipping
                // every cleanup step below (extinction check, live_state
                // update, and critically the "paused" status write) and
                // leaving the simulation's DB row stuck at status "running"
                // forever with nothing logged. `state` was moved into the
                // failed closure, so the rest of this iteration's body (which
                // reads it) can't run either -- treat this the same as
                // MAX_CONSECUTIVE_ERRORS: log it, persist "paused", and stop
                // this loop instead of taking the whole task down.
                eprintln!("[runtime] {sim_id}: tick computation task failed: {join_err}");
                {
                    let mut diag = diagnostics.lock().unwrap_or_else(|e| e.into_inner());
                    if diag.error_log.len() >= ERROR_LOG_CAPACITY {
                        diag.error_log.pop_front();
                    }
                    diag.error_log.push_back(ErrorLogEntry {
                        day: live_day.load(Ordering::SeqCst),
                        ts: now_millis(),
                        msg: format!("tick computation task failed: {join_err}"),
                        stack: String::new(),
                    });
                    diag.consecutive_errors = MAX_CONSECUTIVE_ERRORS;
                }
                if let Err(err) = update_simulation_fields(&backend, &sim_id, Some("paused"), None).await {
                    eprintln!("[runtime] update_simulation_fields failed while auto-pausing {sim_id} after task failure: {err}");
                }
                live_state.lock().unwrap_or_else(|e| e.into_inner()).take();
                stop.store(true, Ordering::SeqCst);
                break;
            }
        };
        state = state_after;
        // Reclaim ownership for next iteration's cache instead of cloning --
        // advance_one_day may have grown it in-memory for anyone born this
        // batch (tick.rs), which this carries forward for free.
        genealogy_cache = std::mem::take(&mut state.genealogy);

        // A population that's died out (or lost its only path to
        // reproduction) is never coming back -- left unchecked, this loop
        // would otherwise keep burning compute and DB writes on it forever
        // (see sim_core::mark_extinct's own doc comment for why this
        // doesn't inject a fake disaster the way manual terminate_simulation
        // does; whatever actually happened is already the real last entries
        // in `state.events`). ws.rs's per-connection extinction_reason check
        // still exists for immediate client notification -- this is what
        // actually stops the loop.
        if let Some(reason) = sim_core::extinction_reason(&state.individuals) {
            sim_core::mark_extinct(&mut state, reason);
            if let Err(err) = save_tick_progress(&backend, &state).await {
                eprintln!("[runtime] save_state failed while marking {sim_id} extinct: {err}");
            }
            if let Err(err) = upsert_individuals(&backend, &state, true).await {
                eprintln!("[runtime] upsert_individuals failed while marking {sim_id} extinct: {err}");
            }
            // save_tick_progress deliberately never touches `status` (see its
            // own doc comment) -- status only ever changes through the
            // dedicated column update every other status transition
            // (pause/start/terminate) already goes through.
            if let Err(err) = update_simulation_fields(&backend, &sim_id, Some("completed"), None).await {
                eprintln!("[runtime] update_simulation_fields failed while marking {sim_id} extinct: {err}");
            }
            live_state.lock().unwrap_or_else(|e| e.into_inner()).take();
            break;
        }

        // Keep a live in-memory copy readable by ws.rs/routes.rs while
        // upload is paused, so population/stats/individual reads there don't
        // appear frozen just because the DB has stopped being written to
        // (see RuntimeManager::live_state's own doc comment). Taken *after*
        // genealogy was moved out above, so this clone never carries the
        // full-history genealogy index -- readers only ever need current
        // individuals/stats, not ancestry. Throttled to
        // LIVE_STATE_REFRESH_INTERVAL: no reader polls faster than that
        // anyway, so cloning the whole population on every single iteration
        // (possibly many times a second at high speed) would just be real
        // CPU cost spent on "freshness" nothing could ever observe. Cleared
        // once not paused: the DB is already at least as fresh then, so
        // holding a stale clone would just waste memory.
        if paused {
            if last_live_state_refresh.elapsed() >= LIVE_STATE_REFRESH_INTERVAL {
                *live_state.lock().unwrap_or_else(|e| e.into_inner()) = Some(state.clone());
                last_live_state_refresh = Instant::now();
            }
        } else {
            let mut guard = live_state.lock().unwrap_or_else(|e| e.into_inner());
            if guard.is_some() {
                *guard = None;
            }
        }

        // MAX_CONSECUTIVE_ERRORS reached above -- match Node's own behavior:
        // stop retrying and flip the simulation to "paused" so the client's
        // existing paused-state UI (and the ability to manually resume)
        // takes over, instead of it silently sitting at status "running"
        // while nothing actually advances.
        if stop.load(Ordering::SeqCst) && diagnostics.lock().unwrap_or_else(|e| e.into_inner()).consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
            eprintln!("[runtime] {sim_id}: {MAX_CONSECUTIVE_ERRORS} consecutive tick errors -- pausing");
            let _ = update_simulation_fields(&backend, &sim_id, Some("paused"), None).await;
        }
        let compute_ms = real_compute.as_secs_f64() * 1000.0;

        // While paused, only force a real DB write once the safety-net
        // interval has elapsed (see UPLOAD_PAUSE_SAFETY_FLUSH); the moment
        // `paused` reads false again (resume_upload was called), this always
        // flushes -- so a resume needs no separate "flush now" step, it just
        // rides the very next iteration's normal write.
        let should_flush = should_flush_upload(paused, last_upload_flush.elapsed(), UPLOAD_PAUSE_SAFETY_FLUSH);

        let (save_ms, upsert_ms) = if should_flush {
            let save_start = Instant::now();
            let save_result = save_tick_progress(&backend, &state).await;
            let save_ms = save_start.elapsed().as_secs_f64() * 1000.0;
            match save_result {
                Ok(true) => {}
                // The row is gone -- deleted while this loop was mid-batch.
                // Exiting here (rather than looping back to a load_simulation
                // that would also see it gone) is just belt-and-suspenders;
                // either way, this must never fall through to a save that could
                // recreate it.
                Ok(false) => break,
                Err(err) => {
                    eprintln!("[runtime] save_state failed for {sim_id}: {err}");
                    carried_state = Some(state);
                    sleep(Duration::from_millis(250)).await;
                    continue;
                }
            }
            let upsert_start = Instant::now();
            // Always the safe (ancestors-included) path while paused -- see
            // full_resync_needed's own comment -- otherwise only on the one
            // pass this loop still owes since it started/last resumed.
            let include_ancestors = paused || full_resync_needed;
            let upsert_result = upsert_individuals(&backend, &state, include_ancestors).await;
            full_resync_needed = next_full_resync_needed(full_resync_needed, paused, upsert_result.is_ok());
            if let Err(err) = &upsert_result {
                eprintln!("[runtime] upsert_individuals failed for {sim_id}: {err}");
            }
            let upsert_ms = upsert_start.elapsed().as_secs_f64() * 1000.0;
            last_upload_flush = Instant::now();

            if !checkpoint_day_seeded {
                checkpoint_day_seeded = true;
                last_checkpoint_day = latest_checkpoint_day(&backend, &sim_id).await.unwrap_or(None);
            }
            if is_due_for_checkpoint(last_checkpoint_day, state.current_day, AUTO_CHECKPOINT_INTERVAL_DAYS) {
                let checkpoint_id = Uuid::new_v4().to_string();
                let population_snapshot = serde_json::to_value(&state).unwrap_or_else(|_| json!({}));
                let tech_state = json!(state.discovered_techs);
                let belief_state = json!(state.discovered_beliefs);
                let art_state = json!(state.discovered_arts);
                let groups = state.extra.get("groups").cloned().unwrap_or_else(|| json!([]));
                let stats = derive_stats(&state);
                match insert_checkpoint(
                    &backend,
                    &checkpoint_id,
                    &sim_id,
                    state.current_day,
                    state.current_year,
                    state.alive_count() as i64,
                    population_snapshot,
                    serde_json::to_value(&state.world_state).unwrap_or_else(|_| json!({})),
                    tech_state,
                    belief_state,
                    art_state,
                    groups,
                    stats,
                )
                .await
                {
                    Ok(()) => last_checkpoint_day = Some(state.current_day),
                    Err(err) => eprintln!("[runtime] auto-checkpoint failed for {sim_id}: {err}"),
                }
            }

            // Bounding memory: now that everyone currently in
            // `state.individuals` has been persisted (upsert_individuals'
            // own transitive parent expansion guarantees anyone still
            // referenced as a parent was included too -- see db.rs), it's
            // safe to physically drop long-dead individuals from the
            // in-memory working set. Mirrors what a fresh bounded DB reload
            // would exclude (see load_bounded_tick_state_no_genealogy's own
            // cutoff) -- without this, a session paused indefinitely would
            // let state.individuals (and the live_state clone built from
            // it, above) grow forever, since nothing else ever reloads and
            // re-bounds it while paused. Skipped entirely if the upsert
            // itself failed: we can't assume anyone was actually persisted
            // then, so dropping them here would risk losing data no save
            // ever wrote down.
            if paused && upsert_result.is_ok() {
                let current_day = state.current_day;
                state.individuals.retain(|ind| should_keep_in_memory_after_flush(ind.alive, ind.death_day, current_day));
            }
            (save_ms, upsert_ms)
        } else {
            (0.0, 0.0)
        };

        let elapsed_ms = iteration_start.elapsed().as_millis() as u64;

        // Per-day processing time (load + compute + save + upsert, excluding
        // both the throttle sleep below *and* the per-day pacing sleep folded
        // into `state`'s compute above) -- what the Performance panel's "Tick
        // Timing" section shows. Recorded even while fast-forwarding, since
        // that's exactly when it's most useful to see.
        if batch_size > 0 {
            let processing_ms = load_ms + compute_ms + save_ms + upsert_ms;
            let per_day_ms = processing_ms / batch_size as f64;
            let mut timing = tick_timing.lock().unwrap_or_else(|e| e.into_inner());
            timing.last_ms = per_day_ms;
            timing.max_ms = if timing.sample_count == 0 { per_day_ms } else { timing.max_ms.max(per_day_ms) };
            timing.min_ms = if timing.sample_count == 0 { per_day_ms } else { timing.min_ms.min(per_day_ms) };
            timing.avg_ms = if timing.sample_count == 0 { per_day_ms } else { timing.avg_ms * 0.9 + per_day_ms * 0.1 };
            timing.ticks_per_second = if per_day_ms > 0.0 { 1000.0 / per_day_ms } else { 0.0 };
            timing.last_load_ms = load_ms;
            timing.last_compute_ms = compute_ms;
            timing.last_save_ms = save_ms;
            timing.last_upsert_ms = upsert_ms;
            timing.last_phases = phases_total;
            timing.sample_count += 1;
        }

        if !fast_forwarding {
            // The DB round trips above (load + save + per-individual upsert)
            // are not free, and on a network-hosted DB they can easily eat
            // seconds once the population grows -- sleeping the full target
            // delay on top of that (rather than just the remainder of it)
            // would silently throttle every speed setting well below what the
            // UI claims. Subtracting however long this iteration already took
            // (which, thanks to the per-day pacing above, is normally already
            // close to target_delay_ms on its own) means DB latency comes out
            // of the speed budget instead of stacking on top of it, so "20x"
            // actually means 20x whenever the DB can keep up, and degrades to
            // "as fast as the DB allows" rather than "DB time plus a full
            // extra second" when it can't.
            let remaining_ms = target_delay_ms.saturating_sub(elapsed_ms);
            if remaining_ms > 0 {
                sleep(Duration::from_millis(remaining_ms)).await;
            }
        }

        // Hand `state` off to the next iteration -- cheap (a move, not a
        // clone) even when upload isn't paused, and needed so a pause
        // toggled on right after this iteration has something to carry.
        carried_state = Some(state);
    }

    let mut sessions = sessions.lock().await;
    sessions.remove(&sim_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── compute_per_day_delay_ms() ──────────────────────────────────────

    #[test]
    fn fast_forwarding_never_paces() {
        assert_eq!(compute_per_day_delay_ms(1000, 100, true, 0), 0);
        assert_eq!(compute_per_day_delay_ms(1000, 100, true, 900), 0);
    }

    #[test]
    fn with_no_db_overhead_yet_the_whole_budget_is_paced() {
        // speed=5, batch_size=5 -> target_delay_ms=1000; no prior measurement.
        assert_eq!(compute_per_day_delay_ms(1000, 5, false, 0), 200);
    }

    #[test]
    fn known_db_overhead_is_reserved_before_pacing_the_rest() {
        // 1000ms budget, 5 days, but the DB round trip is predicted to eat
        // 400ms of it -- pacing should only spend the remaining 600ms
        // (120ms/day), not the full 1000ms, so the real DB call afterward
        // doesn't push the iteration past its 1000ms target.
        assert_eq!(compute_per_day_delay_ms(1000, 5, false, 400), 120);
    }

    #[test]
    fn db_overhead_at_or_above_the_whole_budget_still_spans_a_smooth_batch() {
        // Budget fully (or more than fully) consumed by predicted DB overhead
        // would naively pace 0ms/day -- MIN_BATCH_SPAN_MS keeps the whole
        // batch's pacing spanning a fixed total instead (500ms / 5 days =
        // 100ms/day here), so live_day still advances across enough real
        // time for a polling client to observe intermediate days.
        assert_eq!(compute_per_day_delay_ms(1000, 5, false, 1000), 100);
        assert_eq!(compute_per_day_delay_ms(1000, 5, false, 5000), 100);
    }

    #[test]
    fn the_smooth_span_floor_scales_down_per_day_as_batch_size_grows() {
        // A larger batch (higher speed) still only needs MIN_BATCH_SPAN_MS
        // of *total* span, so each individual day's floor shrinks
        // proportionally -- a flat per-day floor would instead make a
        // large batch's total span balloon far past what's needed for
        // smoothness (and needlessly throttle high speeds when DB overhead
        // happens to be large).
        assert_eq!(compute_per_day_delay_ms(1000, 100, false, 5000), 5);
        assert_eq!(compute_per_day_delay_ms(1000, 100, false, 5000) * 100, MIN_BATCH_SPAN_MS);
    }

    #[test]
    fn a_healthy_low_latency_batch_is_never_slowed_by_the_smooth_span_floor() {
        // When DB overhead is small relative to target_delay_ms (the local/
        // SQLite case), the naturally-computed per-day share already spans
        // comfortably more than MIN_BATCH_SPAN_MS -- the floor must never
        // raise the delay above what pacing alone would already produce.
        let natural = compute_per_day_delay_ms(1000, 20, false, 0);
        assert_eq!(natural, 50);
        assert!(natural * 20 > MIN_BATCH_SPAN_MS);
    }

    #[test]
    fn total_iteration_time_matches_target_delay_once_overhead_is_known() {
        // Simulates two consecutive iterations at speed=20 (batch_size=20,
        // target_delay_ms=1000): the first has no prior DB-overhead
        // estimate, so pacing may overshoot; the second, armed with the
        // first's real overhead, should land within a day's rounding of the
        // 1000ms target instead of stacking DB time on top of it.
        let target_delay_ms = 1000;
        let batch_size = 20;
        let real_db_overhead_ms = 300;

        let first_pacing = compute_per_day_delay_ms(target_delay_ms, batch_size, false, 0);
        let first_total = first_pacing * batch_size as u64 + real_db_overhead_ms;
        assert!(first_total > target_delay_ms, "sanity: with zero prior estimate the first iteration does overshoot");

        let second_pacing = compute_per_day_delay_ms(target_delay_ms, batch_size, false, real_db_overhead_ms);
        let second_total = second_pacing * batch_size as u64 + real_db_overhead_ms;
        assert!(
            second_total.abs_diff(target_delay_ms) <= batch_size as u64,
            "expected ~{target_delay_ms}ms total, got {second_total}ms"
        );
    }

    // ── is_due_for_checkpoint() ─────────────────────────────────────────

    #[test]
    fn no_prior_checkpoint_waits_a_full_interval_before_the_first_one() {
        assert!(!is_due_for_checkpoint(None, 0, 91));
        assert!(!is_due_for_checkpoint(None, 90, 91));
        assert!(is_due_for_checkpoint(None, 91, 91));
        assert!(is_due_for_checkpoint(None, 200, 91));
    }

    #[test]
    fn a_prior_checkpoint_gates_on_days_elapsed_since_it() {
        assert!(!is_due_for_checkpoint(Some(1000), 1090, 91));
        assert!(is_due_for_checkpoint(Some(1000), 1091, 91));
        assert!(is_due_for_checkpoint(Some(1000), 5000, 91));
    }

    // ── should_flush_upload() ───────────────────────────────────────────

    #[test]
    fn not_paused_always_flushes_regardless_of_elapsed_time() {
        assert!(should_flush_upload(false, Duration::ZERO, Duration::from_secs(300)));
        assert!(should_flush_upload(false, Duration::from_secs(9999), Duration::from_secs(300)));
    }

    #[test]
    fn paused_and_within_the_safety_window_skips_the_flush() {
        assert!(!should_flush_upload(true, Duration::from_secs(1), Duration::from_secs(300)));
        assert!(!should_flush_upload(true, Duration::from_secs(299), Duration::from_secs(300)));
    }

    #[test]
    fn paused_past_the_safety_window_flushes_anyway() {
        assert!(should_flush_upload(true, Duration::from_secs(300), Duration::from_secs(300)));
        assert!(should_flush_upload(true, Duration::from_secs(600), Duration::from_secs(300)));
    }

    #[test]
    fn the_moment_upload_resumes_the_very_next_check_flushes() {
        // Simulates: paused for a while (no flush), then resume_upload flips
        // the flag -- should_flush_upload must return true immediately, not
        // wait for the safety window, since `paused` is now false.
        assert!(!should_flush_upload(true, Duration::from_secs(1), Duration::from_secs(300)));
        assert!(should_flush_upload(false, Duration::from_secs(1), Duration::from_secs(300)));
    }

    // ── next_full_resync_needed() ────────────────────────────────────────

    #[test]
    fn a_successful_non_paused_flush_clears_the_resync_flag() {
        assert!(!next_full_resync_needed(true, false, true));
        assert!(!next_full_resync_needed(false, false, true));
    }

    #[test]
    fn a_successful_paused_flush_keeps_owing_a_resync_next_time() {
        assert!(next_full_resync_needed(true, true, true));
        assert!(next_full_resync_needed(false, true, true));
    }

    #[test]
    fn a_failed_flush_leaves_the_flag_exactly_as_it_was() {
        assert!(next_full_resync_needed(true, false, false));
        assert!(!next_full_resync_needed(false, false, false));
        assert!(next_full_resync_needed(true, true, false));
        assert!(!next_full_resync_needed(false, true, false));
    }

    // ── should_keep_in_memory_after_flush() ─────────────────────────────

    #[test]
    fn the_living_are_always_kept() {
        assert!(should_keep_in_memory_after_flush(true, None, 10_000));
        assert!(should_keep_in_memory_after_flush(true, Some(0), 10_000));
    }

    #[test]
    fn the_recently_dead_are_kept() {
        assert!(should_keep_in_memory_after_flush(false, Some(100), 100));
        assert!(should_keep_in_memory_after_flush(false, Some(100), 106));
    }

    #[test]
    fn the_long_dead_are_finally_dropped_once_a_flush_has_persisted_them() {
        assert!(!should_keep_in_memory_after_flush(false, Some(100), 107));
        assert!(!should_keep_in_memory_after_flush(false, Some(0), 100_000));
    }

    #[test]
    fn a_missing_death_day_is_kept_forever_fail_safe() {
        // Same "missing death_day means never exclude" fail-safe direction
        // load_bounded_tick_state_no_genealogy's own cutoff already takes.
        assert!(should_keep_in_memory_after_flush(false, None, 1_000_000));
    }
}
