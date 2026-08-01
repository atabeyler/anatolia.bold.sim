// Main-thread handle to the sim-wasm Web Worker (worker.ts) -- every method
// posts one call, keyed by an incrementing id, and resolves/rejects the
// matching pending promise when the worker replies. One worker per tab,
// created lazily on first use.
let worker: Worker | null = null;
let nextId = 1;
const pending = new Map<number, { resolve: (value: string) => void; reject: (error: Error) => void }>();

function ensureWorker(): Worker {
  if (worker) return worker;
  worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
  worker.onmessage = (e: MessageEvent) => {
    const { id, result, error } = e.data as { id: number; result?: string; error?: string };
    const p = pending.get(id);
    if (!p) return;
    pending.delete(id);
    if (error !== undefined) p.reject(new Error(error));
    else p.resolve(result as string);
  };
  worker.onerror = (e) => {
    // A worker-level error (e.g. the wasm module itself failed to load)
    // has no `id` to route to a specific caller -- reject everything still
    // outstanding so no caller hangs forever waiting for a reply that will
    // never arrive.
    const err = new Error(e.message || 'wasm worker error');
    for (const [id, p] of pending) {
      p.reject(err);
      pending.delete(id);
    }
  };
  return worker;
}

function call(fn: string, ...args: unknown[]): Promise<string> {
  const w = ensureWorker();
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    w.postMessage({ id, fn, args });
  });
}

async function callJson<T>(fn: string, ...args: unknown[]): Promise<T> {
  const raw = await call(fn, ...args);
  return JSON.parse(raw) as T;
}

// Thin, typed wrappers -- one per sim-wasm export. Every function here
// mirrors exactly one sim-server route; see rust/sim-wasm/src/lib.rs's own
// doc comments for the authoritative mapping.
export const engine = {
  createSimulation: (name: string | null, latitude: number, longitude: number, founder1ParamsJson: string, founder2ParamsJson: string) =>
    callJson<Record<string, unknown>>('create_simulation', name, latitude, longitude, founder1ParamsJson, founder2ParamsJson),
  advanceDay: (stateJson: string, disabledEnginesJson: string) =>
    callJson<{ state: Record<string, unknown>; report: Record<string, unknown>; phases: Record<string, unknown> }>('advance_day', stateJson, disabledEnginesJson),
  getStats: (stateJson: string) => callJson<Record<string, unknown>>('get_stats', stateJson),
  getPopulation: (stateJson: string, alive: boolean | null, limit: number | null) => callJson<Record<string, unknown>[]>('get_population', stateJson, alive, limit),
  getIndividual: (stateJson: string, individualId: string) => callJson<Record<string, unknown> | null>('get_individual', stateJson, individualId),
  getEvents: (stateJson: string) => callJson<Record<string, unknown>[]>('get_events', stateJson),
  getEventsSummary: (stateJson: string) => callJson<Record<string, unknown>>('get_events_summary', stateJson),
  applyIntervention: (stateJson: string, interventionType: string, paramsJson: string) =>
    callJson<{ state: Record<string, unknown>; affected_individuals: number; deaths: number }>('apply_intervention', stateJson, interventionType, paramsJson),
  terminate: (stateJson: string) => callJson<Record<string, unknown>>('terminate', stateJson),
  // Not a sim-wasm export -- reads worker.ts's own record of how many
  // threads initThreadPool actually spun up, whether the page is
  // cross-origin isolated at all, and (if the pool didn't reach the
  // requested size) why -- see worker.ts's own doc comment. Used by
  // apiAdapter.ts's buildMetrics to report real core usage instead of a
  // hardcoded guess, and to make a stuck "1 / N" diagnosable from the
  // Performance panel alone, without DevTools.
  getThreadDiagnostics: () => callJson<{ threadCount: number; crossOriginIsolated: boolean; error: string | null }>('get_thread_diagnostics'),
};
