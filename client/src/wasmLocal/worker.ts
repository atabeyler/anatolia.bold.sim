// Dedicated Web Worker hosting the sim-wasm module, so a heavy tick (large
// population) never janks the main UI thread. All communication is plain
// JSON-serializable messages -- see engineClient.ts for the main-thread side
// of this contract. Kept deliberately dumb: this file owns no simulation
// logic of its own, only dispatch into whichever sim-wasm export the caller
// named, exactly mirroring how sim-wasm's own lib.rs is a thin dispatcher
// over sim-core.
import init, {
  initThreadPool,
  create_simulation,
  create_founder_json,
  create_world,
  advance_day,
  get_stats,
  get_population,
  get_individual,
  get_events,
  get_events_summary,
  apply_intervention,
  terminate,
} from './pkg/sim_wasm.js';

type WasmFn = (...args: never[]) => string;

const fns: Record<string, WasmFn> = {
  create_simulation: create_simulation as WasmFn,
  create_founder_json: create_founder_json as WasmFn,
  create_world: create_world as WasmFn,
  advance_day: advance_day as WasmFn,
  get_stats: get_stats as WasmFn,
  get_population: get_population as WasmFn,
  get_individual: get_individual as WasmFn,
  get_events: get_events as WasmFn,
  get_events_summary: get_events_summary as WasmFn,
  apply_intervention: apply_intervention as WasmFn,
  terminate: terminate as WasmFn,
};

// Real thread count once initThreadPool succeeds -- 1 if it's unavailable
// (no cross-origin isolation, so no SharedArrayBuffer -- see main.rs's COOP/
// COEP headers) or fails for any other reason, since sim-core's rayon calls
// still work correctly single-threaded in that case, just without the
// parallelism. Read by engineClient.ts's getThreadDiagnostics() so the
// Performance panel reports what's actually running instead of a hardcoded
// guess.
let threadCount = 1;
// Why it's still 1, if it is -- surfaced end-to-end into the Performance
// panel (see apiAdapter.ts's buildMetrics -> simStore's RuntimeMetrics ->
// PerformancePanel.tsx) so diagnosing a stuck "1 / 4" in production doesn't
// require the user to open DevTools themselves; they can just paste the
// report back.
let threadPoolError: string | null = null;

let readyPromise: Promise<void> | null = null;
function ensureReady(): Promise<void> {
  if (readyPromise) return readyPromise;
  const p = init().then(async () => {
    // Re-enabled after finding tick.rs's maybe_par_iter_mut! macro had been
    // silently forcing sim-core's own par_iter_mut calls to run sequentially
    // on every wasm32 build regardless of this thread pool's state, ever
    // since it was added (predates initThreadPool existing at all, never
    // updated once it landed) -- meaning the freeze reproduced earlier with
    // this forced to 1 happened while sim-core was dispatching *zero* real
    // parallel work, so it can't have been a panic inside one of those
    // closures. With the macro fixed (see tick.rs) to actually use rayon on
    // every target, this is worth re-testing under real parallel dispatch --
    // the previous repro's conditions no longer match production once threads
    // are back on.
    const wantedThreads = (self as unknown as { navigator: { hardwareConcurrency?: number } }).navigator.hardwareConcurrency || 1;
    if (wantedThreads > 1) {
      if (!(self as unknown as { crossOriginIsolated?: boolean }).crossOriginIsolated) {
        threadPoolError = 'crossOriginIsolated is false (COOP/COEP headers missing or not applied)';
      } else {
        try {
          await initThreadPool(wantedThreads);
          threadCount = wantedThreads;
        } catch (err) {
          // Falls back to single-threaded silently -- e.g. the browser
          // doesn't support wasm threads at all despite being cross-origin
          // isolated. sim-core's own rayon calls still run correctly, just
          // serially.
          threadPoolError = err instanceof Error ? err.message : String(err);
          console.warn('[wasmLocal] initThreadPool unavailable, falling back to single-threaded:', err);
        }
      }
    }
  });
  readyPromise = p;
  return p;
}

interface CallMessage {
  id: number;
  fn: string;
  args: unknown[];
}

const ctx = self as unknown as { onmessage: ((e: MessageEvent) => void) | null; postMessage: (msg: unknown) => void };

ctx.onmessage = (e: MessageEvent) => {
  const { id, fn, args } = e.data as CallMessage;
  ensureReady()
    .then(() => {
      // Not a real sim-wasm export -- engineClient.ts's own
      // getThreadDiagnostics() needs the pool size/outcome decided inside
      // ensureReady above, which callers can't know ahead of time (it
      // depends on whether initThreadPool actually succeeded). Handled here
      // rather than added to `fns` so its return type doesn't have to
      // pretend to match WasmFn's string shape for something that was never
      // a wasm call in the first place.
      if (fn === 'get_thread_diagnostics') {
        ctx.postMessage({
          id,
          result: JSON.stringify({
            threadCount,
            crossOriginIsolated: (self as unknown as { crossOriginIsolated?: boolean }).crossOriginIsolated ?? false,
            error: threadPoolError,
          }),
        });
        return;
      }
      const impl = fns[fn];
      if (!impl) throw new Error(`Unknown wasm function: ${fn}`);
      const result = impl(...(args as never[]));
      ctx.postMessage({ id, result });
    })
    .catch((err: unknown) => {
      ctx.postMessage({ id, error: err instanceof Error ? err.message : String(err) });
    });
};
