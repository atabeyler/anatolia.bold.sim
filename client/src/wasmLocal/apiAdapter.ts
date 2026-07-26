// Custom axios adapter for WASM-local mode: intercepts every
// `/api/simulations/...` and `/api/god/...` call and services it from the
// in-browser engine/IndexedDB instead of the network, returning the exact
// same response shapes sim-server would -- so every existing page/panel's
// own axios call sites stay completely unchanged. `/api/analysis/...` is
// special-cased: there's no client-side Gemini key to answer with locally,
// so it's translated into a real network call to the server's own
// `/api/analysis/local[/hypothesis]` endpoints, carrying the caller's own
// state summary (population/day/events/techs) instead of a DB-backed
// `sim_id` (see `handleAnalysis` below and rust/sim-server/src/analysis.rs's
// `analyze_local`/`hypothesis_local`). Anything else (auth, absolute URLs)
// passes through to a real fetch() call, exactly like it would reach the
// real backend today. WASM-local still requires a real login (see
// BrowserModeGate.tsx) -- only the simulation data itself is kept on-device,
// mirroring desktop/Android's own "Yerel" mode.
import axios, { AxiosError, type AxiosResponse, type InternalAxiosRequestConfig } from 'axios';
import { isWasmLocalModeActive } from './mode';
import { engine } from './engineClient';
import * as runtime from './runtime';
import { dbListSimulations, dbSaveSimulation, dbDeleteSimulation, dbListCheckpoints, dbGetCheckpoint, dbCreateCheckpoint, dbEstimateUsage, type StoredSimRecord } from './db';
import { buildReport } from './report';

type AnyRecord = Record<string, unknown>;

function ok<T>(config: InternalAxiosRequestConfig, data: T, status = 200): AxiosResponse<T> {
  return { data, status, statusText: 'OK', headers: {}, config };
}

function fail(config: InternalAxiosRequestConfig, status: number, message: string): never {
  const response: AxiosResponse = { data: { error: message }, status, statusText: message, headers: {}, config };
  const err = new AxiosError(message, undefined, config, undefined, response);
  throw err;
}

function pathOf(url: string | undefined): string {
  try {
    return new URL(url ?? '', window.location.origin).pathname;
  } catch {
    return url ?? '';
  }
}

function queryOf(url: string | undefined): URLSearchParams {
  try {
    return new URL(url ?? '', window.location.origin).searchParams;
  } catch {
    return new URLSearchParams();
  }
}

function authHeaderOf(config: InternalAxiosRequestConfig): string | undefined {
  if (!config.headers) return undefined;
  for (const [k, v] of Object.entries(config.headers as Record<string, unknown>)) {
    if (k.toLowerCase() === 'authorization' && typeof v === 'string') return v;
  }
  return undefined;
}

function parseBody(data: unknown): AnyRecord {
  if (data == null) return {};
  if (typeof data === 'string') {
    try {
      return JSON.parse(data) as AnyRecord;
    } catch {
      return {};
    }
  }
  return data as AnyRecord;
}

async function passthroughViaFetch(config: InternalAxiosRequestConfig): Promise<AxiosResponse> {
  const url = new URL(config.url ?? '', (config.baseURL as string) || window.location.origin).toString();
  const headers: Record<string, string> = {};
  if (config.headers) {
    for (const [k, v] of Object.entries(config.headers as Record<string, unknown>)) {
      if (typeof v === 'string') headers[k] = v;
    }
  }
  const method = (config.method ?? 'get').toUpperCase();
  const init: RequestInit = { method, headers, credentials: config.withCredentials ? 'include' : 'same-origin' };
  if (config.data !== undefined && method !== 'GET') {
    init.body = typeof config.data === 'string' ? config.data : JSON.stringify(config.data);
    if (!headers['Content-Type']) headers['Content-Type'] = 'application/json';
  }
  let res: Response;
  try {
    res = await fetch(url, init);
  } catch (err) {
    fail(config, 0, err instanceof Error ? err.message : 'Network request failed');
  }
  const text = await res.text();
  let data: unknown = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    data = text;
  }
  if (!res.ok) {
    const message = (data as AnyRecord)?.error as string | undefined;
    fail(config, res.status, message ?? res.statusText);
  }
  return ok(config, data, res.status);
}

function toSimulationSummary(row: StoredSimRecord): AnyRecord {
  return {
    id: row.id,
    name: row.name,
    status: row.status,
    current_day: row.current_day,
    current_year: row.current_year,
    start_latitude: row.start_latitude,
    start_longitude: row.start_longitude,
    speed_multiplier: row.speed_multiplier,
  };
}

// PhaseTimings' own field names (sim-core/src/state.rs) match tick_phase_*_ms
// 1:1 once prefixed -- listed in the same order as that struct so a diff
// against it is easy.
const PHASE_FIELDS = [
  'setup_ms', 'economy_ms', 'consciousness_psychology_ms', 'language_naming_ms',
  'microbiome_agent_ms', 'movement_ms', 'observation_learning_ms', 'tech_emergence_ms',
  'reproduction_ms', 'mortality_roll_ms', 'microbiome_outbreak_ms', 'group_pruning_ms',
  'belief_ms', 'culture_art_ms', 'social_ms', 'law_ms', 'architecture_conflict_ms',
  'astronomy_ms', 'trade_disease_ms',
] as const;

async function buildMetrics(state: AnyRecord): Promise<AnyRecord> {
  const individuals = (state.individuals as AnyRecord[]) ?? [];
  const alive = individuals.filter((i) => i.alive && !i.is_dead).length;
  const timing = runtime.getTickTiming();
  const fastForwardTarget = runtime.getFastForwardTarget();
  const phaseFields: AnyRecord = {};
  for (const f of PHASE_FIELDS) {
    phaseFields[`tick_phase_${f}`] = timing?.lastPhases?.[f] ?? null;
  }
  // Whatever the worker's own initThreadPool actually managed (see its doc
  // comment), plus why it isn't at full strength if it isn't -- falls back
  // to a benign "1 thread, no error" shape if the worker isn't ready yet or
  // the query itself fails for any reason; a Performance panel poll should
  // never throw over this.
  const threadDiag = await engine.getThreadDiagnostics().catch(() => ({ threadCount: 1, crossOriginIsolated: false, error: null }));
  return {
    population: alive,
    total_ever: state.total_ever_born ?? individuals.length,
    current_day: state.current_day ?? 0,
    current_year: state.current_year ?? 0,
    milestones_reached: state.milestones ?? [],
    speed_multiplier: state.speed_multiplier ?? 1,
    status: state.status ?? 'paused',
    is_warping: fastForwardTarget != null,
    fast_forward_target: fastForwardTarget,
    upload_paused: false,
    cpu_cores_available: navigator.hardwareConcurrency || 1,
    cpu_cores_used: threadDiag.threadCount,
    cross_origin_isolated: threadDiag.crossOriginIsolated,
    thread_pool_error: threadDiag.error,
    disabled_engines: runtime.getDisabledEngines(),
    // Repurposed from "always 0, no server round trip" to fullSync()'s own
    // getStats+getEvents worker round trip (runs roughly once a second,
    // regardless of whether a checkpoint also fires that cycle) -- the one
    // remaining tick-loop operation not wrapped by tick timing above, and
    // (like Save/Upsert below) its cost scales with total individuals ever
    // born and the event log, not just current population.
    tick_load_ms: timing?.lastFullSyncMs ?? 0,
    // Engine-only time (sum of tick_phase_*_ms below), not the full worker
    // round trip -- see TickTiming's own doc comment (runtime.ts) for why
    // conflating the two here used to make every phase row look like it
    // rounded to 0 next to a much larger "Compute" figure that was actually
    // measuring postMessage/serialization overhead, not the engine.
    tick_compute_ms: timing?.lastComputeMs ?? null,
    // Repurposed from "always 0, no server round trip" to this mode's real
    // IndexedDB write costs: persist() (every ~5s) as Save, maybeCheckpoint()
    // (every ~365 sim days) as Upsert -- both scale with the size of
    // stateJson, which only grows over a run (see TickTiming's own doc
    // comment on why), and neither is wrapped by tick timing above, so a
    // "tick timing looks fine" Performance panel could otherwise hide a real
    // periodic stall here.
    tick_save_ms: timing?.lastPersistMs ?? 0,
    tick_upsert_ms: timing?.lastCheckpointMs ?? 0,
    tick_last_ms: timing?.lastMs ?? null,
    tick_avg_ms: timing?.avgMs ?? 0,
    tick_max_ms: timing?.maxMs ?? 0,
    tick_min_ms: timing?.minMs ?? 0,
    ticks_per_second: timing?.lastMs ? 1000 / timing.lastMs : 0,
    ...phaseFields,
    // Dead on the native backend too (ws.rs always sends an empty array --
    // nothing populates a real migration-centroid history there either), so
    // this isn't a WASM-local gap to fill, just matching what already ships.
    centroid_trail: [],
    heavy_mode: false,
    workers_disabled: false,
  };
}

function buildDiagnostics(state: AnyRecord, checkpoints: Awaited<ReturnType<typeof dbListCheckpoints>>, events: AnyRecord[]): AnyRecord {
  const individuals = (state.individuals as AnyRecord[]) ?? [];
  const alive = individuals.filter((i) => i.alive && !i.is_dead).length;
  const latest = checkpoints[checkpoints.length - 1];
  return {
    status: state.status ?? 'paused',
    running: state.status === 'running',
    population: alive,
    current_day: state.current_day,
    current_year: state.current_year,
    checkpoint_count: checkpoints.length,
    event_count: events.length,
    latest_checkpoint_day: latest?.sim_day ?? null,
    latest_checkpoint_year: latest?.sim_year ?? null,
    consecutive_errors: 0,
    startup: { ts: Date.now(), day: state.current_day, checks: [] },
    error_log: [],
  };
}

async function buildDbStatus(id: string, state: AnyRecord): Promise<AnyRecord> {
  const { usage, quota } = await dbEstimateUsage();
  const checkpoints = await dbListCheckpoints(id);
  const individuals = (state.individuals as AnyRecord[]) ?? [];
  const alive = individuals.filter((i) => i.alive && !i.is_dead).length;
  return {
    sim_db: {
      size_bytes: usage,
      individuals: { total: individuals.length, alive },
      checkpoints: checkpoints.length,
      events: ((state.events as unknown[]) ?? []).length,
      technologies: ((state.discovered_techs as unknown[]) ?? []).length,
      beliefs: ((state.discovered_beliefs as unknown[]) ?? []).length,
      languages: 0,
      groups: ((state.groups as unknown[]) ?? []).length,
      conversations: 0,
      publications: 0,
    },
    // Matches the native backend's own much-slimmer cloud_db shape exactly
    // (see routes.rs's get_db_status: {size_bytes, cloud_checkpoints,
    // live_snapshots} -- no individuals/events/etc breakdown, unlike sim_db)
    // -- this used to ship a fabricated wider shape with the wrong field
    // names entirely (`checkpoints` instead of `cloud_checkpoints`, no
    // `live_snapshots` at all), which PerformancePanel.tsx's DbStatus
    // interface doesn't read, so the panel rendered literal "undefined"
    // for both fields. A WASM-local trial with nothing uploaded genuinely
    // has neither, so 0/0 here is correct, not just a type-shape fix.
    cloud_db: { size_bytes: null, cloud_checkpoints: 0, live_snapshots: 0 },
    quota_bytes: quota,
  };
}

const SIM_ROUTE = /^\/api\/simulations(\/.*)?$/;
const GOD_ROUTE = /^\/api\/god\//;
const ANALYSIS_ROUTE = /^\/api\/analysis\//;
// Cross-device/account concepts that only ever make sense against a real
// cloud account -- never intercepted, even while WASM-local mode is active,
// so `/upload-to-cloud`'s own outbound `/api/simulations/import` call (see
// its handler below) and any other cross-account bridge keep hitting a real
// server.
const CLOUD_ONLY_ROUTES = [/^\/api\/simulations\/import$/, /^\/api\/simulations\/live$/, /^\/api\/simulations\/live-sync$/, /^\/api\/simulations\/live\//];

async function handleSimulations(path: string, method: string, config: InternalAxiosRequestConfig): Promise<AxiosResponse> {
  const body = parseBody(config.data);

  if (method === 'post' && path === '/api/simulations') {
    const { name, latitude, longitude, founder_1_params, founder_2_params } = body;
    const state = await engine.createSimulation(
      (name as string) ?? null,
      latitude as number,
      longitude as number,
      JSON.stringify(founder_1_params ?? {}),
      JSON.stringify(founder_2_params ?? {}),
    );
    await dbSaveSimulation({
      id: state.id as string,
      name: (state.name as string) ?? 'Untitled Simulation',
      status: 'paused',
      current_day: 0,
      current_year: 0,
      start_latitude: latitude as number,
      start_longitude: longitude as number,
      speed_multiplier: 1,
      stateJson: JSON.stringify(state),
      created_at: Date.now(),
      updated_at: Date.now(),
    });
    return ok(config, state, 201);
  }

  if (method === 'get' && path === '/api/simulations') {
    const rows = await dbListSimulations();
    return ok(config, rows.map(toSimulationSummary));
  }

  const simMatch = path.match(/^\/api\/simulations\/([^/]+)(\/.*)?$/);
  if (!simMatch) fail(config, 404, 'Not found (WASM-local mode)');
  const id = decodeURIComponent(simMatch[1]);
  const rest = simMatch[2] ?? '';

  if (method === 'get' && rest === '') {
    const stateJson = await runtime.loadIntoSession(id);
    return ok(config, JSON.parse(stateJson));
  }
  if (method === 'get' && rest === '/stats') {
    const stateJson = await runtime.loadIntoSession(id);
    return ok(config, await engine.getStats(stateJson));
  }
  if (method === 'get' && rest === '/population') {
    const stateJson = await runtime.loadIntoSession(id);
    const q = queryOf(config.url);
    const alive = q.has('alive') ? q.get('alive') === 'true' : null;
    const limit = q.has('limit') ? Number(q.get('limit')) : null;
    return ok(config, await engine.getPopulation(stateJson, alive, limit));
  }
  const indMatch = rest.match(/^\/population\/([^/]+)$/);
  if (method === 'get' && indMatch) {
    const stateJson = await runtime.loadIntoSession(id);
    const found = await engine.getIndividual(stateJson, decodeURIComponent(indMatch[1]));
    if (!found) fail(config, 404, 'Individual not found');
    return ok(config, found);
  }
  if (method === 'get' && rest === '/events') {
    const stateJson = await runtime.loadIntoSession(id);
    return ok(config, await engine.getEvents(stateJson));
  }
  if (method === 'get' && rest === '/events/summary') {
    const stateJson = await runtime.loadIntoSession(id);
    return ok(config, await engine.getEventsSummary(stateJson));
  }
  if (method === 'get' && rest === '/checkpoints') {
    const rows = await dbListCheckpoints(id);
    return ok(
      config,
      rows.map((c) => ({ id: c.id, simulation_id: c.simulation_id, sim_day: c.sim_day, sim_year: c.sim_year, population_count: c.population_count, stats: c.stats, created_at: c.created_at })),
    );
  }
  if (method === 'post' && rest === '/checkpoint') {
    const stateJson = await runtime.loadIntoSession(id);
    const state = JSON.parse(stateJson) as AnyRecord;
    const stats = (await engine.getStats(stateJson)) as AnyRecord;
    const checkpointId = await dbCreateCheckpoint({
      simulation_id: id,
      sim_day: state.current_day as number,
      sim_year: state.current_year as number,
      population_count: (stats.population as number) ?? 0,
      stats,
      stateJson,
      created_at: Date.now(),
    });
    return ok(config, { id: checkpointId, message: 'Checkpoint created' }, 201);
  }
  const restoreMatch = rest.match(/^\/restore\/(\d+)$/);
  if (method === 'post' && restoreMatch) {
    const checkpoint = await dbGetCheckpoint(Number(restoreMatch[1]));
    if (!checkpoint) fail(config, 404, 'Checkpoint not found');
    await runtime.loadIntoSession(id);
    runtime.setActiveStateJson(checkpoint.stateJson);
    await runtime.pauseSim(id);
    return ok(config, { message: 'Restored' });
  }
  if (method === 'get' && rest === '/report') {
    const stateJson = await runtime.loadIntoSession(id);
    const checkpoints = await dbListCheckpoints(id);
    return ok(config, await buildReport(JSON.parse(stateJson), checkpoints));
  }
  if (method === 'get' && rest === '/export') {
    const stateJson = await runtime.loadIntoSession(id);
    return ok(config, JSON.parse(stateJson));
  }
  if (method === 'post' && rest === '/start') {
    await runtime.startSim(id);
    return ok(config, { message: 'Started' });
  }
  if (method === 'post' && rest === '/pause') {
    await runtime.pauseSim(id);
    return ok(config, { message: 'Paused' });
  }
  if (method === 'post' && rest === '/speed') {
    await runtime.setSpeed(id, (body.speed_multiplier as number) ?? 1);
    return ok(config, { message: 'Speed updated' });
  }
  if (method === 'post' && rest === '/fast-forward') {
    await runtime.startFastForward(id, body.target_year as number);
    return ok(config, { message: 'Fast-forward started' });
  }
  if (method === 'post' && rest === '/fast-forward/cancel') {
    await runtime.cancelFastForward(id);
    return ok(config, { message: 'Fast-forward cancelled' });
  }
  if (method === 'post' && rest === '/terminate') {
    await runtime.terminateSim(id);
    return ok(config, { message: 'Simulation terminated' });
  }
  if (method === 'post' && rest === '/engines') {
    const disabled = Array.isArray(body.disabled) ? (body.disabled as string[]) : [];
    await runtime.setDisabledEngines(id, disabled);
    return ok(config, { disabled_engines: disabled });
  }
  if (method === 'delete' && rest === '') {
    await dbDeleteSimulation(id);
    return ok(config, { message: 'Deleted' });
  }
  if (method === 'get' && rest === '/metrics') {
    const stateJson = await runtime.loadIntoSession(id);
    return ok(config, await buildMetrics(JSON.parse(stateJson)));
  }
  if (method === 'get' && rest === '/diagnostics') {
    const stateJson = await runtime.loadIntoSession(id);
    const state = JSON.parse(stateJson) as AnyRecord;
    const checkpoints = await dbListCheckpoints(id);
    const events = (await engine.getEvents(stateJson)) as AnyRecord[];
    return ok(config, buildDiagnostics(state, checkpoints, events));
  }
  if (method === 'get' && rest === '/db-status') {
    const stateJson = await runtime.loadIntoSession(id);
    return ok(config, await buildDbStatus(id, JSON.parse(stateJson)));
  }
  if (method === 'post' && rest === '/upload-to-cloud') {
    const authHeader = authHeaderOf(config);
    if (!authHeader) fail(config, 401, 'Sign in required.');
    const stateJson = await runtime.loadIntoSession(id);
    let res: Response;
    try {
      res = await fetch('/api/simulations/import', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Authorization: authHeader },
        body: stateJson,
      });
    } catch (err) {
      fail(config, 502, err instanceof Error ? err.message : 'Could not reach the cloud.');
    }
    const text = await res.text();
    let data: unknown = null;
    try { data = text ? JSON.parse(text) : null; } catch { data = text; }
    if (!res.ok) fail(config, res.status, (data as AnyRecord)?.error as string | undefined ?? res.statusText);
    return ok(config, data, res.status);
  }

  fail(config, 404, 'Not found (WASM-local mode)');
}

async function handleGod(path: string, method: string, config: InternalAxiosRequestConfig): Promise<AxiosResponse> {
  const body = parseBody(config.data);
  const godMatch = path.match(/^\/api\/god\/([^/]+)(\/.*)?$/);
  if (!godMatch) fail(config, 404, 'Not found (WASM-local mode)');
  const id = decodeURIComponent(godMatch[1]);
  const rest = godMatch[2] ?? '';

  if (method === 'post' && rest === '/intervene') {
    const stateJson = await runtime.loadIntoSession(id);
    try {
      const result = await engine.applyIntervention(stateJson, body.type as string, JSON.stringify(body.params ?? {}));
      runtime.setActiveStateJson(JSON.stringify(result.state));
      return ok(config, { message: 'Intervention applied.', affected_individuals: result.affected_individuals, deaths: result.deaths });
    } catch (err) {
      fail(config, 400, err instanceof Error ? err.message : String(err));
    }
  }
  if (method === 'post' && rest === '/quarantine') {
    const stateJson = await runtime.loadIntoSession(id);
    const result = await engine.applyIntervention(stateJson, 'quarantine', JSON.stringify({ enabled: (body.enabled as boolean) ?? true }));
    runtime.setActiveStateJson(JSON.stringify(result.state));
    return ok(config, { message: 'Quarantine updated.' });
  }
  if (rest.startsWith('/talk/')) {
    fail(config, 501, 'The oracle needs a real connection -- unavailable in WASM-local mode.');
  }

  fail(config, 404, 'Not found (WASM-local mode)');
}

// Proxies `/api/analysis/:simId` (chat-style question) and
// `/api/analysis/:simId/hypothesis` (verdict test) to the real backend's
// `/api/analysis/local[/hypothesis]` endpoints (see analysis.rs's
// `analyze_local`/`hypothesis_local`) instead of servicing them from the
// browser: the app's Gemini key only ever lives server-side, so a genuine AI
// answer in WASM-local mode has to go through the network regardless. What
// changes versus the normal `/:simId` path is *what* gets sent -- the
// caller's own state summary (population/day/events/techs pulled from
// IndexedDB) rather than a `sim_id` the server could look up itself, since
// this simulation was never written to the server's database. Falls back to
// the server's own Gemini-failure heuristic (identical thresholds) exactly
// like the normal path does whenever Gemini itself is unreachable or errors.
async function handleAnalysis(path: string, method: string, config: InternalAxiosRequestConfig): Promise<AxiosResponse> {
  const body = parseBody(config.data);
  const match = path.match(/^\/api\/analysis\/([^/]+)(\/hypothesis)?$/);
  if (!match || method !== 'post') fail(config, 404, 'Not found (WASM-local mode)');
  const id = decodeURIComponent(match[1]);
  const isHypothesis = !!match[2];

  const stateJson = await runtime.loadIntoSession(id);
  const state = JSON.parse(stateJson) as AnyRecord;
  const individuals = (state.individuals as AnyRecord[]) ?? [];
  const population = individuals.filter((i) => i.alive && !i.is_dead).length;
  const day = (state.current_day as number) ?? 0;
  const stateEvents = (state.events as unknown[])?.length ?? 0;
  const techs = (state.discovered_techs as unknown[])?.length ?? 0;

  const localUrl = isHypothesis ? '/api/analysis/local/hypothesis' : '/api/analysis/local';
  const localBody = isHypothesis
    ? { hypothesis: body.hypothesis, lang: body.lang, events: body.events, stats: body.stats, day, population }
    : { message: body.message, lang: body.lang, day, population, events: stateEvents, techs };

  return passthroughViaFetch({ ...config, url: localUrl, data: JSON.stringify(localBody) });
}

export async function wasmLocalAdapter(config: InternalAxiosRequestConfig): Promise<AxiosResponse> {
  if (!isWasmLocalModeActive()) return passthroughViaFetch(config);

  // An explicit absolute URL (e.g. DashboardPage's `${CLOUD_API_URL}/api/simulations`
  // cloud-sims fetch) always means "the real cloud, regardless of path" -- only
  // relative same-origin calls are ever answered from the in-browser engine.
  if (/^https?:\/\//i.test(config.url ?? '')) return passthroughViaFetch(config);

  const path = pathOf(config.url);
  if (CLOUD_ONLY_ROUTES.some((re) => re.test(path))) return passthroughViaFetch(config);
  if (!SIM_ROUTE.test(path) && !GOD_ROUTE.test(path) && !ANALYSIS_ROUTE.test(path)) return passthroughViaFetch(config);

  const method = (config.method ?? 'get').toLowerCase();
  try {
    if (GOD_ROUTE.test(path)) return await handleGod(path, method, config);
    if (ANALYSIS_ROUTE.test(path)) return await handleAnalysis(path, method, config);
    return await handleSimulations(path, method, config);
  } catch (err) {
    if (err instanceof AxiosError) throw err;
    fail(config, 500, err instanceof Error ? err.message : String(err));
  }
}

// Installs this adapter as axios's global default -- called once, from
// wasmLocal's entry-point wiring, before any WASM-local page mounts. Every
// existing axios call site (relative or absolute) is unaffected until
// activateWasmLocalMode() flips the flag this checks at call time.
export function installWasmLocalAdapter(): void {
  axios.defaults.adapter = wasmLocalAdapter;
}
