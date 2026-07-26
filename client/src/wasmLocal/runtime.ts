// Tick-loop manager for WASM-local mode -- the in-browser counterpart to
// sim-server's RuntimeManager (runtime.rs) and its WS broadcast (ws.rs).
// There is no server and no socket: this module owns a setInterval loop that
// calls the sim-wasm worker directly and pushes results into the exact same
// Zustand store setters useSimWebSocket.ts already calls, so every panel
// stays byte-for-byte unaware of which backend is driving it.
import { useSimStore } from '../store/simStore';
import { engine } from './engineClient';
import { dbSaveSimulation, dbLoadSimulation, dbCreateCheckpoint, type StoredSimRecord } from './db';

const TICK_INTERVAL_MS = 100;
const FULL_SYNC_INTERVAL_MS = 1000;
const PERSIST_INTERVAL_MS = 5000;
const CHECKPOINT_INTERVAL_DAYS = 365;
// A 100ms-interval timer firing this late since its last call is not normal
// frame jitter -- it means the browser throttled/suspended this tab's timers
// (backgrounded tab, device sleep, etc.) for a while. daysOwed still
// correctly accounts for that whole gap (it's computed from real elapsed
// time, not frame count), but the catch-up must land directly on the
// caught-up day instead of visibly animating through every missed day one
// by one -- same intent as fast-forward's own "without watching every
// intermediate day". An earlier version instead capped how many days a
// single tick() could process (MAX_DAYS_PER_TICK) and always reported every
// day, so the backlog after a background gap drained gradually across many
// ticks -- deliberately reverted per user preference: a brief single
// catch-up burst (the fullSync-throttle fix below already removes the
// dominant cause of that burst actually stalling) is preferred over a
// multi-second "fast-forwarding" animation on every resume.
const BACKGROUND_CATCHUP_THRESHOLD_MS = TICK_INTERVAL_MS * 10;
// Real per-day compute (a genuine worker postMessage round trip, unlike
// sim-server's own tick loop) can take longer than TICK_INTERVAL_MS once the
// population grows, especially at high speed_multiplier. Left uncapped, an
// ordinary (foregrounded, NOT a real background-throttled gap) slow tick's
// own wall-clock time inflates the *next* tick's measured dt (see
// wasHiddenSinceLastTick's doc comment above for how dt is computed), which
// computes an even larger daysOwed/daysThisFrame next time -- a
// self-reinforcing spiral observed directly in testing (daysThisFrame
// climbing 0, 0, 1, 27, 459 across a handful of ticks at 100x). Capping how
// many days a single tickOnce() call will take on, ONLY for this ordinary
// case, bounds each tick's wall-clock cost, which keeps dt (and thus
// daysOwed) from compounding, and -- since the day-loop already checks
// session.running every iteration -- keeps Pause/speed changes responsive
// even under a large backlog: leftover owed days simply carry over to the
// next tick (daysOwed -= daysThisFrame below already handles that) instead
// of being forced into one unbounded batch. Deliberately NOT applied to a
// real background catch-up (isBackgroundCatchUp below, gated on an actual
// visibilitychange event) -- that case is meant to land on the caught-up day
// in one atomic jump, per this file's own BACKGROUND_CATCHUP_THRESHOLD_MS
// doc comment; capping it too would silently reintroduce the gradual
// multi-second "fast-forwarding" animation that was deliberately reverted.
const MAX_DAYS_PER_TICK = 30;

// Mirrors sim-server's TickTiming (runtime.rs) exactly, so the Performance
// panel's "Tick Timing"/"Module" sections read identically regardless of
// which backend produced them -- but the two sections mean different things
// even on the server (see runtime.rs's own tick_timing block): last_ms is
// the *full* per-day processing time (load+compute+save+upsert there; here,
// the whole postMessage round trip to the wasm worker and back, since that
// round trip is this mode's real equivalent of a server request's DB I/O),
// while lastComputeMs/lastPhases are engine-only time -- sum(lastPhases)
// approximately equals lastComputeMs, the same relationship the "Module"
// rows have to the "Compute" bucket on the server. Conflating the two here
// used to make the Compute bucket show the full round trip while every
// Module row under it showed genuine (and much smaller) per-phase time,
// so they could never add up -- see this session's own performance
// deep-dive that caught it. max/min run for the life of the session, avg is
// the same 0.9/0.1 exponential moving average.
export interface TickTiming {
  lastMs: number | null;
  avgMs: number;
  maxMs: number;
  minMs: number;
  lastComputeMs: number | null;
  sampleCount: number;
  lastPhases: Record<string, number> | null;
  // Diagnostic-only: persist()/maybeCheckpoint() are awaited inside
  // tickOnce() but never wrapped by recordTickTiming, so an app-visible
  // "engine looks fast" Performance panel could still coexist with real
  // periodic stalls hiding in these two IndexedDB writes -- both scale with
  // the size of stateJson, which only ever grows (individuals are
  // field-stripped on death, never removed from the array). Surfaced here
  // so that theory can be confirmed or ruled out with real numbers instead
  // of guesswork. null until each has run at least once; not reset to null
  // in between runs, so the panel keeps showing the last real measurement.
  lastPersistMs: number | null;
  lastCheckpointMs: number | null;
  // Same rationale as above, for fullSync()'s own two worker round trips
  // (getStats + getEvents) -- these run roughly once a second regardless of
  // whether a checkpoint happens to fire that cycle, and their cost scales
  // with total individuals ever born (never removed, only field-stripped)
  // and the event log, via postMessage's own structured-clone cost, not
  // just Rust-side compute. A real report showed both lastMs/lastPersistMs/
  // lastCheckpointMs looking completely healthy while the sim still visibly
  // stalled -- this was the one remaining unmeasured worker round trip in
  // the tick loop.
  lastFullSyncMs: number | null;
}

interface ActiveSession {
  id: string;
  stateJson: string;
  running: boolean;
  daysOwed: number;
  sentEventCount: number;
  lastFullSyncAt: number;
  lastPersistAt: number;
  lastCheckpointDay: number;
  extinctionReported: string | null;
  fastForwardTarget: number | null;
  timer: ReturnType<typeof setInterval> | null;
  tickTiming: TickTiming;
  // A large catch-up burst (thousands of days after a long background gap)
  // can take real wall-clock time to run through -- setInterval doesn't wait
  // for a previous async callback to settle before scheduling the next one,
  // so without this guard a still-running catch-up could overlap with fresh
  // tick() calls racing on the same session.stateJson/daysOwed. Skipping any
  // tick() that fires while one is already in flight keeps the whole
  // catch-up a single, atomic jump regardless of how long it takes.
  tickInFlight: boolean;
  // Kept out of stateJson deliberately: SimulationState.disabled_engines is
  // #[serde(skip)] on the Rust side (a diagnostic toggle never meant to
  // persist), so sim-wasm's advance_day takes it as a separate argument and
  // this must be resent on every single call, exactly like sim-server keeps
  // its own copy outside the DB-persisted state (runtime.rs).
  disabledEngines: string[];
  // Bumped by setSpeed() -- lets an in-flight day-processing loop notice a
  // manual speed change mid-batch and stop, the same way it already stops
  // for session.running going false (pauseSim()). Without this, a batch
  // already sized (daysThisFrame, up to MAX_DAYS_PER_TICK) under the OLD
  // speed right before the user picked a new one keeps running to
  // completion regardless -- confirmed over a real run: switching 100x to 1x
  // still showed ~20-30 days advancing over the next few real seconds,
  // because the still-in-flight ~30-day batch from just before the click
  // simply hadn't finished yet (each day is a real worker round trip, and at
  // large population that batch alone can take several real seconds).
  // Resetting daysOwed in setSpeed() alone doesn't touch this -- that batch
  // was already decided before the reset ran.
  speedChangeSeq: number;
  // Authoritative speed for the tick loop's own income math -- deliberately
  // NOT read from parseState(session.stateJson).speed_multiplier each tick,
  // even though that field also exists in stateJson (kept there too, for
  // persistence/display). sim-core's advance_day() just echoes whatever
  // speed_multiplier was in the state it was given straight back out
  // unchanged; each day-loop iteration captures session.stateJson as its
  // input *before* awaiting, so if setSpeed() writes a new speed into
  // session.stateJson while a handful of already-in-flight advanceDay calls
  // (from before the change) are still resolving, each of those calls
  // finishes by doing session.stateJson = JSON.stringify(result.state) --
  // clobbering the just-written new speed right back to the stale one echoed
  // in its own (pre-change) snapshot. Confirmed directly: after a real
  // speed change, tickOnce()'s own dt/speed logging kept reading the OLD
  // speed for several ticks afterward despite setSpeed() having already run.
  // Exactly the same class of problem disabledEngines above solves by living
  // outside stateJson entirely -- speed_multiplier can't do that (native
  // sim-server does persist it as real state), so instead this field is the
  // one thing tickOnce() trusts for its own math, and every state write
  // re-stamps stateJson's own copy from this rather than the engine's echo.
  speedMultiplier: number;
}

let session: ActiveSession | null = null;

function recordTickTiming(timing: TickTiming, elapsedMs: number, phases: Record<string, number>): void {
  timing.lastMs = elapsedMs;
  timing.maxMs = timing.sampleCount === 0 ? elapsedMs : Math.max(timing.maxMs, elapsedMs);
  timing.minMs = timing.sampleCount === 0 ? elapsedMs : Math.min(timing.minMs, elapsedMs);
  timing.avgMs = timing.sampleCount === 0 ? elapsedMs : timing.avgMs * 0.9 + elapsedMs * 0.1;
  timing.lastPhases = phases;
  timing.lastComputeMs = Object.values(phases).reduce((sum, ms) => sum + ms, 0);
  timing.sampleCount += 1;
}

function parseState(stateJson: string): Record<string, unknown> {
  return JSON.parse(stateJson);
}

function extinctionReason(state: Record<string, unknown>): string | null {
  const individuals = (state.individuals as Array<Record<string, unknown>>) ?? [];
  const alive = individuals.filter((i) => i.alive && !i.is_dead);
  if (alive.length === 0) return 'population_zero';
  if (alive.length === 1) return 'single_individual';
  if (alive.every((i) => i.sex === 'male')) return 'no_females';
  if (alive.every((i) => i.sex === 'female')) return 'no_males';
  return null;
}

async function persist(): Promise<void> {
  if (!session) return;
  const startedAt = performance.now();
  const state = parseState(session.stateJson);
  const record: StoredSimRecord = {
    id: session.id,
    name: (state.name as string) ?? 'Untitled Simulation',
    status: session.running ? 'running' : ((state.status as StoredSimRecord['status']) ?? 'paused'),
    current_day: (state.current_day as number) ?? 0,
    current_year: (state.current_year as number) ?? 0,
    start_latitude: (state.start_latitude as number) ?? 0,
    start_longitude: (state.start_longitude as number) ?? 0,
    speed_multiplier: session.speedMultiplier,
    stateJson: session.stateJson,
    created_at: Date.now(),
    updated_at: Date.now(),
  };
  const existing = await dbLoadSimulation(session.id);
  if (existing) record.created_at = existing.created_at;
  await dbSaveSimulation(record);
  session.lastPersistAt = Date.now();
  session.tickTiming.lastPersistMs = performance.now() - startedAt;
}

async function maybeCheckpoint(): Promise<void> {
  if (!session) return;
  const state = parseState(session.stateJson);
  const currentDay = (state.current_day as number) ?? 0;
  if (currentDay - session.lastCheckpointDay < CHECKPOINT_INTERVAL_DAYS) return;
  const startedAt = performance.now();
  session.lastCheckpointDay = currentDay;
  const stats = await engine.getStats(session.stateJson);
  await dbCreateCheckpoint({
    simulation_id: session.id,
    sim_day: currentDay,
    sim_year: (state.current_year as number) ?? 0,
    population_count: (stats.population as number) ?? 0,
    stats,
    stateJson: session.stateJson,
    created_at: Date.now(),
  });
  session.tickTiming.lastCheckpointMs = performance.now() - startedAt;
}

async function fullSync(): Promise<void> {
  if (!session) return;
  const store = useSimStore.getState();
  const startedAt = performance.now();
  const stats = await engine.getStats(session.stateJson);
  const events = await engine.getEvents(session.stateJson);
  session.tickTiming.lastFullSyncMs = performance.now() - startedAt;
  const newEvents = events.slice(session.sentEventCount);
  session.sentEventCount = events.length;
  store.setStats(stats as never);
  for (const event of newEvents) {
    store.addEvent(event as never);
    if ((event as Record<string, unknown>).event_type === 'milestone') {
      const data = (event as Record<string, unknown>).data as Record<string, unknown>;
      store.addMilestone({
        key: (data?.key as string) ?? '',
        description: (event as Record<string, unknown>).description as string,
        icon: (data?.icon as string) ?? '🏆',
        day: (event as Record<string, unknown>).sim_day as number,
      });
    }
  }
  const state = parseState(session.stateJson);
  const reason = extinctionReason(state);
  if (reason && session.extinctionReported !== reason) {
    session.extinctionReported = reason;
    store.setSimulationEnded(reason);
  }
  await maybeCheckpoint();
  session.lastFullSyncAt = Date.now();
}

let lastFrameAt = 0;

// A `dt` gap this large used to be treated as proof the tab was backgrounded
// (see BACKGROUND_CATCHUP_THRESHOLD_MS's own doc comment), but it has
// exactly one other, entirely normal cause: this mode's own tick loop is
// single-threaded (see worker.ts's forced wantedThreads=1, itself a fix for
// a worse threaded-deadlock freeze), so a slow-but-genuinely-foregrounded
// tick -- e.g. a larger population's advance_day() calls simply taking a
// while in a row -- inflates the *next* tick's measured dt exactly the same
// way a real background gap would. Reported directly: at high population/
// speed the on-screen day counter was jumping in large blocks ("10 years at
// a time") on a tab that was never backgrounded -- tickOnce() was
// misclassifying an ordinary slow tick as a background catch-up and
// skipping the smooth per-day updates it would otherwise have shown for
// what was actually only a handful of owed days. Tracking real
// visibilitychange events distinguishes the two: only an actual hidden
// period suppresses per-day updates now, regardless of how long a
// foregrounded tick took.
let wasHiddenSinceLastTick = false;
if (typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) wasHiddenSinceLastTick = true;
  });
}

async function tick(): Promise<void> {
  if (!session || !session.running || session.tickInFlight) return;
  session.tickInFlight = true;
  try {
    await tickOnce(session);
  } finally {
    if (session) session.tickInFlight = false;
  }
}

async function tickOnce(session: ActiveSession): Promise<void> {
  const now = Date.now();
  const dt = lastFrameAt ? now - lastFrameAt : TICK_INTERVAL_MS;
  const isBackgroundCatchUp = dt > BACKGROUND_CATCHUP_THRESHOLD_MS && wasHiddenSinceLastTick;
  wasHiddenSinceLastTick = false;

  const state = parseState(session.stateJson);
  const speed = session.speedMultiplier;
  session.daysOwed += (speed * dt) / 1000;
  let daysThisFrame = Math.floor(session.daysOwed);
  if (!isBackgroundCatchUp) daysThisFrame = Math.min(daysThisFrame, MAX_DAYS_PER_TICK);
  if (session.fastForwardTarget != null) {
    const currentDay = (state.current_day as number) ?? 0;
    daysThisFrame = Math.max(daysThisFrame, Math.min(30, Math.max(session.fastForwardTarget * 365 - currentDay, 0)));
  }
  session.daysOwed -= daysThisFrame;

  // Ordinary frame-to-frame ticking (even a high speed_multiplier, which can
  // legitimately process many days within one normal 100ms frame) still
  // updates the day counter per simulated day below, for the deliberately-
  // visible fast-clock feel the speed slider is supposed to have. A
  // catch-up after the timer was throttled skips that per-day updating
  // entirely and jumps straight to the final day once, after the loop --
  // this can take real wall-clock time for a large backlog, but the
  // tickInFlight guard above keeps it a single atomic run no matter how long
  // it takes, so nothing else can sneak in a partial update meanwhile.
  const showIntermediateDays = !isBackgroundCatchUp;
  const disabledEnginesJson = JSON.stringify(session.disabledEngines);
  // Reported directly: pausing mid-catch-up (a backlog built up at high
  // speed) took ~2.7 real seconds to actually stop the day counter, even
  // though pauseSim() itself returns instantly -- this loop never checked
  // session.running, so once a large daysThisFrame was computed above, it
  // ran to completion regardless of a pause requested partway through.
  // `await engine.advanceDay()` already yields back to the event loop once
  // per iteration, which is exactly when a pauseSim() call from a click
  // handler gets to run and flip session.running -- checking it here is
  // enough to make Pause take effect within a single day's worth of compute
  // instead of the whole backlog. Whatever days this tick didn't get to
  // process go back into daysOwed rather than being silently dropped, so
  // resuming later still simulates every owed day, just later.
  let daysProcessed = 0;
  const speedSeqAtStart = session.speedChangeSeq;
  let interruptedBySpeedChange = false;
  for (let i = 0; i < daysThisFrame; i++) {
    if (!session.running) break;
    if (session.speedChangeSeq !== speedSeqAtStart) { interruptedBySpeedChange = true; break; }
    const dayStart = performance.now();
    const result = await engine.advanceDay(session.stateJson, disabledEnginesJson);
    // pauseSim() may have run (and already persisted its own paused
    // snapshot) while the call above was in flight -- discard this day's
    // result rather than committing it over that, so a pause requested
    // mid-computation never gets silently overwritten by the day that was
    // already underway. Its own compute time still counts toward this
    // tick's tracked timing (real work happened), but it's not applied.
    // Same idea for a speed change landing mid-await: this one day, already
    // paid for, still gets applied (nothing gained by discarding it too),
    // but the loop stops claiming more of the OLD speed's batch afterward.
    if (!session.running) break;
    recordTickTiming(session.tickTiming, performance.now() - dayStart, result.phases as Record<string, number>);
    // Re-stamp speed_multiplier from the authoritative session field rather
    // than trusting result.state's own copy (see ActiveSession.speedMultiplier's
    // doc comment) -- this call's input snapshot may predate a setSpeed()
    // that ran while it was in flight, and the engine echoes that stale
    // value straight back through untouched.
    (result.state as Record<string, unknown>).speed_multiplier = session.speedMultiplier;
    session.stateJson = JSON.stringify(result.state);
    daysProcessed++;
    const day = (result.state as Record<string, unknown>).current_day as number;
    if (showIntermediateDays) useSimStore.getState().setStatsDay(day);
    if (session.fastForwardTarget != null && day >= session.fastForwardTarget * 365) {
      session.fastForwardTarget = null;
      break;
    }
    if (session.speedChangeSeq !== speedSeqAtStart) { interruptedBySpeedChange = true; break; }
  }
  // A speed change already reset daysOwed to 0 (setSpeed()'s own doc
  // comment) precisely so the new speed takes effect immediately -- carrying
  // this interrupted batch's untouched remainder back into daysOwed here
  // would silently undo that reset with a chunk of the old speed's debt.
  if (daysProcessed < daysThisFrame && !interruptedBySpeedChange) {
    session.daysOwed += daysThisFrame - daysProcessed;
  }
  if (!showIntermediateDays && daysProcessed > 0) {
    const finalDay = (parseState(session.stateJson).current_day as number) ?? 0;
    useSimStore.getState().setStatsDay(finalDay);
  }

  // `|| daysThisFrame > 0` used to defeat this throttle entirely: at any
  // speed high enough that daysThisFrame is >0 on nearly every 100ms tick
  // (speed >= ~10x), fullSync() -- two full-state worker round trips
  // (getStats + getEvents), each scaling with total individuals ever born
  // and the capped event log -- ran on almost every tick instead of once a
  // second as FULL_SYNC_INTERVAL_MS's name implies. A 50-year/100x
  // reproduction run measured this directly: 1825 ticks fired, and
  // getEvents was called 1825 times (should be ~183, once per ~10 ticks) --
  // a real ~10x excess of worker round trips, compounding as the
  // ever-growing individuals array (dead entries are field-stripped but
  // never removed, see tick.rs's strip_dead_individual_if_due) makes each
  // one more expensive. This is the dominant cause of the periodic
  // freeze-then-jump the user could still reproduce with the tab fully
  // foregrounded (i.e. not explained by the earlier background-throttling
  // fix): each of these calls runs inside the same tickInFlight-guarded
  // tickOnce(), so a slow one visibly stalls setStatsDay until it resolves,
  // and the backlog that piles up during that stall then drains in a rapid
  // burst of consecutive capped ticks once unblocked.
  if (now - session.lastFullSyncAt >= FULL_SYNC_INTERVAL_MS) {
    await fullSync();
  }
  if (now - session.lastPersistAt >= PERSIST_INTERVAL_MS) {
    await persist();
  }
  // Deliberately stamped now, at the very end of this tick's real work, not
  // at the top before the day loop ran (as it was originally, and as `now`
  // above still is for this tick's own dt/isBackgroundCatchUp math -- only
  // where the NEXT tick's reference point gets set changes here). Confirmed
  // over a real multi-hundred-day run: once a capped MAX_DAYS_PER_TICK batch
  // itself takes real seconds of wall-clock compute (large population), the
  // old top-of-function stamp meant the *next* tick's dt spanned from this
  // tick's START to that next tick's start -- silently re-including this
  // tick's own multi-second compute time as if it were idle time the sim
  // still owed days for, even though those days were already simulated. That
  // owed-again chunk reappeared on literally the next tick, which is why
  // resetting daysOwed in setSpeed() alone didn't fix a speed change feeling
  // unresponsive: 1x still "owed" a fresh handful of days from the very tick
  // immediately after, sized by how long the last (100x-driven) batch had
  // taken to compute. Stamping lastFrameAt after all of this tick's work is
  // done makes dt measure only genuine idle time between ticks (normally
  // ~TICK_INTERVAL_MS), so daysOwed no longer compounds from compute time at
  // all -- MAX_DAYS_PER_TICK and setSpeed's reset remain as belt-and-braces,
  // but this is what stops the backlog from reappearing in the first place.
  lastFrameAt = Date.now();
}

function ensureSession(id: string, stateJson: string): ActiveSession {
  if (session && session.id === id) return session;
  session = {
    id,
    stateJson,
    running: false,
    daysOwed: 0,
    sentEventCount: 0,
    lastFullSyncAt: 0,
    lastPersistAt: 0,
    lastCheckpointDay: (parseState(stateJson).current_day as number) ?? 0,
    extinctionReported: null,
    fastForwardTarget: null,
    timer: null,
    tickTiming: { lastMs: null, avgMs: 0, maxMs: 0, minMs: 0, lastComputeMs: null, sampleCount: 0, lastPhases: null, lastPersistMs: null, lastCheckpointMs: null, lastFullSyncMs: null },
    disabledEngines: [],
    tickInFlight: false,
    speedChangeSeq: 0,
    speedMultiplier: Math.min(Math.max((parseState(stateJson).speed_multiplier as number) ?? 1, 1), 1000),
  };
  // There's no socket to open in WASM-local mode -- the engine is either
  // ready right here on the main thread or it isn't -- but useSimWebSocket.ts
  // no-ops entirely in this mode (see its own isWasmLocalModeActive() guard),
  // so wsStatus is otherwise left stuck at the store's initial 'connecting'
  // forever. Flip it once a session exists so the Performance panel's "Live
  // Connection" reads as live instead of perpetually "connecting…".
  useSimStore.getState().setWsStatus('open');
  return session;
}

export function getActiveSimId(): string | null {
  return session?.id ?? null;
}

export function getActiveStateJson(): string | null {
  return session?.stateJson ?? null;
}

export function getTickTiming(): TickTiming | null {
  return session?.tickTiming ?? null;
}

export function getFastForwardTarget(): number | null {
  return session?.fastForwardTarget ?? null;
}

export function getDisabledEngines(): string[] {
  return session?.disabledEngines ?? [];
}

export function setActiveStateJson(stateJson: string): void {
  if (session) session.stateJson = stateJson;
}

export async function loadIntoSession(id: string): Promise<string> {
  if (session && session.id === id) return session.stateJson;
  const record = await dbLoadSimulation(id);
  if (!record) throw new Error('simulation not found');
  ensureSession(id, record.stateJson);
  return record.stateJson;
}

export async function startSim(id: string): Promise<void> {
  const stateJson = await loadIntoSession(id);
  const s = ensureSession(id, stateJson);
  const state = parseState(s.stateJson);
  state.status = 'running';
  s.stateJson = JSON.stringify(state);
  s.running = true;
  lastFrameAt = 0;
  await persist();
  if (!s.timer) {
    s.timer = setInterval(() => {
      tick().catch((err) => console.error('[wasmLocal] tick failed:', err));
    }, TICK_INTERVAL_MS);
  }
}

export async function pauseSim(id: string): Promise<void> {
  if (!session || session.id !== id) return;
  session.running = false;
  if (session.timer) {
    clearInterval(session.timer);
    session.timer = null;
  }
  const state = parseState(session.stateJson);
  state.status = 'paused';
  session.stateJson = JSON.stringify(state);
  await persist();
}

export async function setSpeed(id: string, speedMultiplier: number): Promise<void> {
  if (!session || session.id !== id) return;
  const clamped = Math.min(Math.max(speedMultiplier, 1), 1000);
  session.speedMultiplier = clamped;
  const state = parseState(session.stateJson);
  state.speed_multiplier = clamped;
  session.stateJson = JSON.stringify(state);
  // Reported directly, confirmed over a real multi-hundred-day run: once real
  // per-day compute can't keep up with a high speed_multiplier (population
  // grown enough that a single MAX_DAYS_PER_TICK-capped batch takes longer
  // than TICK_INTERVAL_MS to compute), daysOwed keeps a genuine backlog even
  // with that cap in place -- it just bounds each individual tick's jump
  // instead of the backlog itself. Left alone, switching to a much lower
  // speed still drained that leftover backlog first, at the OLD speed's
  // effective rate, before the newly chosen speed's own (much smaller) pace
  // took over -- e.g. picking 1x after a 100x backlog built up still showed
  // ~20-30 days advancing in the next few seconds, not the ~3 a real 1x
  // implies. Unlike a paused backlog (pauseSim() deliberately preserves
  // daysOwed so nothing is lost across a pause -- see tickOnce()'s own doc
  // comment), a manual speed change is an explicit request for a different
  // pace starting now: the debt is an artifact of the old speed outrunning
  // real compute, not simulated time the user is owed, so it's dropped
  // rather than carried into the new speed.
  session.daysOwed = 0;
  // Also lets an already-in-flight tick's day loop notice this change and
  // stop claiming more of a batch sized under the old speed -- see
  // ActiveSession.speedChangeSeq's own doc comment.
  session.speedChangeSeq += 1;
}

// Full-replace semantics, matching POST /:id/engines. Kept as session state
// rather than written into stateJson: SimulationState.disabled_engines is
// #[serde(skip)] on the Rust side (see runtime.ts's ActiveSession.disabledEngines
// doc comment), so this is resent to advance_day on every tick instead.
export async function setDisabledEngines(id: string, disabled: string[]): Promise<void> {
  if (!session || session.id !== id) return;
  session.disabledEngines = disabled;
}

export async function startFastForward(id: string, targetYear: number): Promise<void> {
  if (!session || session.id !== id) return;
  session.fastForwardTarget = targetYear;
}

export async function cancelFastForward(id: string): Promise<void> {
  if (!session || session.id !== id) return;
  session.fastForwardTarget = null;
}

export async function terminateSim(id: string): Promise<Record<string, unknown>> {
  const stateJson = await loadIntoSession(id);
  const s = ensureSession(id, stateJson);
  const result = await engine.terminate(s.stateJson);
  s.stateJson = JSON.stringify(result);
  s.running = false;
  if (s.timer) {
    clearInterval(s.timer);
    s.timer = null;
  }
  await persist();
  return result;
}
