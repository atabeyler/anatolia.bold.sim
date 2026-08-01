// Regression tests for the "tab backgrounded, then reopened" bug: the tick
// loop's setInterval gets throttled by the browser while the tab is hidden,
// but daysOwed correctly accounts for the whole missed real-time gap once it
// fires again -- the bug was that the resulting catch-up burst visibly
// animated through every missed simulated day (setStatsDay called once per
// day), instead of jumping directly to the caught-up day the moment the tab
// is reopened. (An intermediate version of this file capped how many days a
// single tick() call could process and drained EVERY backlog gradually
// across several ticks, including a real background gap -- deliberately
// reverted per user preference: a brief single catch-up burst is preferred
// over a multi-second "fast-forwarding" animation on every resume, once the
// fullSync-throttle fix below removes the dominant cause of that burst ever
// actually stalling. MAX_DAYS_PER_TICK in runtime.ts brought a cap back, but
// scoped to ordinary foreground ticks only -- a real background gap
// (isBackgroundCatchUp, gated on an actual visibilitychange event) still
// gets the single uncapped jump this file's tests below assert on.)
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { StoredSimRecord } from './db';

vi.mock('./db', () => ({
  dbSaveSimulation: vi.fn().mockResolvedValue(undefined),
  dbLoadSimulation: vi.fn(),
  dbCreateCheckpoint: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('./engineClient', () => ({
  engine: {
    advanceDay: vi.fn(),
    getStats: vi.fn().mockResolvedValue({ population: 2 }),
    getEvents: vi.fn().mockResolvedValue([]),
  },
}));

const SIM_ID = 'sim-1';

function fakeRecord(currentDay: number): StoredSimRecord {
  return {
    id: SIM_ID,
    name: 'Test Sim',
    status: 'running',
    current_day: currentDay,
    current_year: 0,
    start_latitude: 0,
    start_longitude: 0,
    speed_multiplier: 1,
    stateJson: JSON.stringify({ current_day: currentDay, speed_multiplier: 1, individuals: [] }),
    created_at: 0,
    updated_at: 0,
  };
}

describe('wasmLocal runtime tick loop', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  it('jumps directly to the caught-up day after a long throttled gap, instead of animating through every missed day', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const { useSimStore } = await import('../store/simStore');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));
    // Each call advances current_day by exactly one, mirroring sim-core's
    // real advance_one_day semantics.
    let day = 0;
    vi.mocked(engine.advanceDay).mockImplementation(async () => {
      day += 1;
      return { state: { current_day: day, speed_multiplier: 1, individuals: [] }, report: {}, phases: {} };
    });

    const realSetStatsDay = useSimStore.getState().setStatsDay;
    const setStatsDaySpy = vi.fn((d: number) => realSetStatsDay(d));
    useSimStore.setState({ setStatsDay: setStatsDaySpy });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);

    // First ordinary 100ms tick: establishes lastFrameAt (0.1 days owed at
    // speed 1, not yet a whole day, so nothing is processed or reported yet).
    await vi.advanceTimersByTimeAsync(100);
    setStatsDaySpy.mockClear();

    // Simulate the tab being backgrounded and throttled for an hour: the
    // system clock jumps far ahead, but the timer only fires once (exactly
    // like a real throttled setInterval catching up with a single callback,
    // not one per missed 100ms). A long dt alone no longer implies this --
    // it also needs real evidence the tab was actually hidden (see
    // runtime.ts's own doc comment on why), so dispatch the same
    // visibilitychange a genuine background/foreground cycle would fire.
    Object.defineProperty(document, 'hidden', { value: true, configurable: true });
    document.dispatchEvent(new Event('visibilitychange'));
    Object.defineProperty(document, 'hidden', { value: false, configurable: true });
    vi.setSystemTime(t0 + 100 + 3600 * 1000);
    await vi.advanceTimersByTimeAsync(100);

    // ~3600 days were owed and processed, but the UI should have been
    // updated exactly once with the final day, not once per missed day.
    expect(setStatsDaySpy).toHaveBeenCalledTimes(1);
    expect(setStatsDaySpy).toHaveBeenLastCalledWith(day);
    expect(day).toBeGreaterThan(3000);

    runtime.pauseSim(SIM_ID);
  });

  // Regression test for a real report: since worker.ts forces single-
  // threaded rayon (a fix for a worse threaded-deadlock freeze), a slower
  // sequential tick at higher population/speed can itself inflate the next
  // tick's measured dt past BACKGROUND_CATCHUP_THRESHOLD_MS -- purely from
  // genuine foreground compute time, with the tab never once backgrounded.
  // Before gating on real visibilitychange evidence, this misclassified as
  // a background catch-up and silently collapsed a normal handful of owed
  // days into one jump, reported as the day counter advancing "10 years at
  // a time" on a tab that was never sent to the background.
  it('does not jump-and-hide per-day updates for a long gap when the tab was never actually hidden', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const { useSimStore } = await import('../store/simStore');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));
    let day = 0;
    vi.mocked(engine.advanceDay).mockImplementation(async () => {
      day += 1;
      return { state: { current_day: day, speed_multiplier: 1, individuals: [] }, report: {}, phases: {} };
    });

    const realSetStatsDay = useSimStore.getState().setStatsDay;
    const setStatsDaySpy = vi.fn((d: number) => realSetStatsDay(d));
    useSimStore.setState({ setStatsDay: setStatsDaySpy });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);
    await vi.advanceTimersByTimeAsync(100);
    setStatsDaySpy.mockClear();

    // Same large dt as the background-catch-up test above, but with no
    // visibilitychange at all -- e.g. a slow synchronous tick, not a
    // throttled/hidden tab.
    vi.setSystemTime(t0 + 100 + 3600 * 1000);
    await vi.advanceTimersByTimeAsync(100);

    expect(setStatsDaySpy.mock.calls.length).toBeGreaterThan(1);
    expect(setStatsDaySpy).toHaveBeenLastCalledWith(day);

    runtime.pauseSim(SIM_ID);
  });

  it('still updates the day counter per simulated day during ordinary fast-multiplier play', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const { useSimStore } = await import('../store/simStore');
    const runtime = await import('./runtime');

    // speed_multiplier 500 legitimately advances many days within one
    // ordinary 100ms frame -- this is the deliberately-visible fast-clock
    // feel the speed slider is supposed to have, and must not be silently
    // collapsed into a single jump the way a throttled-tab catch-up is.
    vi.mocked(dbLoadSimulation).mockResolvedValue({ ...fakeRecord(0), stateJson: JSON.stringify({ current_day: 0, speed_multiplier: 500, individuals: [] }) });
    let day = 0;
    vi.mocked(engine.advanceDay).mockImplementation(async () => {
      day += 1;
      return { state: { current_day: day, speed_multiplier: 500, individuals: [] }, report: {}, phases: {} };
    });

    const realSetStatsDay = useSimStore.getState().setStatsDay;
    const setStatsDaySpy = vi.fn((d: number) => realSetStatsDay(d));
    useSimStore.setState({ setStatsDay: setStatsDaySpy });

    vi.setSystemTime(1_000_000);
    await runtime.startSim(SIM_ID);
    await vi.advanceTimersByTimeAsync(100);

    // ~50 days owed in this single ordinary 100ms frame (500 * 100ms/1000).
    expect(day).toBeGreaterThan(10);
    expect(setStatsDaySpy).toHaveBeenCalledTimes(day);

    runtime.pauseSim(SIM_ID);
  });

  it('an overlapping tick is skipped while a catch-up burst is still in flight, instead of racing it', async () => {
    // setInterval doesn't wait for a previous async callback to settle
    // before scheduling the next one -- a catch-up burst busy processing a
    // large backlog (which can take real wall-clock time) could otherwise be
    // joined by a fresh tick() call racing on the same session.stateJson.
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));

    let day = 0;
    let callCount = 0;
    // A plain object property (not a bare `let`) so TypeScript doesn't narrow
    // it to its initial `null` in the outer scope just because the only
    // reassignment happens inside a nested closure.
    const blocker: { release: (() => void) | null } = { release: null };
    const firstCallStarted = new Promise<void>((resolveStarted) => {
      vi.mocked(engine.advanceDay).mockImplementation(async () => {
        callCount += 1;
        if (callCount === 1) {
          resolveStarted();
          // Blocks until the test explicitly releases it, simulating a
          // catch-up burst still busy on its very first day when the next
          // setInterval firing happens.
          await new Promise<void>((resolve) => { blocker.release = resolve; });
        }
        day += 1;
        return { state: { current_day: day, speed_multiplier: 1, individuals: [] }, report: {}, phases: {} };
      });
    });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);
    await vi.advanceTimersByTimeAsync(100); // establishes lastFrameAt, 0 days owed yet
    // The very first tick ever always passes the "enough time since last
    // sync" check (lastFullSyncAt starts at 0, "now" is a real timestamp far
    // past that), so it calls fullSync once regardless of this test's
    // scenario -- clear that unrelated baseline call before asserting on the
    // overlapping-tick behavior below.
    vi.mocked(engine.getStats).mockClear();
    vi.mocked(engine.getEvents).mockClear();

    // A large gap triggers a multi-day catch-up burst; its first advanceDay
    // call blocks immediately. Marked as a real background gap (not just a
    // slow foreground tick) so MAX_DAYS_PER_TICK's cap doesn't truncate it --
    // this test is about the tickInFlight overlap guard, not that cap.
    Object.defineProperty(document, 'hidden', { value: true, configurable: true });
    document.dispatchEvent(new Event('visibilitychange'));
    Object.defineProperty(document, 'hidden', { value: false, configurable: true });
    vi.setSystemTime(t0 + 100 + 3600 * 1000);
    const burstTick = vi.advanceTimersByTimeAsync(100);
    await firstCallStarted;
    expect(callCount).toBe(1);

    // A second setInterval firing while the burst is still stuck on its
    // first day must be a no-op entirely -- not just skipping its own
    // advanceDay calls (its own owed-days share is naturally near zero right
    // after the burst's own synchronous prefix already ran, guard or not),
    // but also never reaching fullSync's engine.getStats/getEvents calls,
    // which an un-guarded overlapping tick *would* reach here (lastFullSyncAt
    // is still its initial 0, so "enough time has passed" is trivially true).
    await vi.advanceTimersByTimeAsync(100);
    expect(callCount).toBe(1);
    expect(engine.getStats).not.toHaveBeenCalled();
    expect(engine.getEvents).not.toHaveBeenCalled();

    // Release the first call so the burst can run to completion. The
    // remaining ~3599 advanceDay calls resolve immediately (no artificial
    // delay), but each is still its own chained microtask -- burstTick
    // itself may already have settled while the loop was blocked (fake
    // timers only guarantee synchronous progress is flushed, not a
    // still-pending real promise further down the chain), so drain
    // microtasks directly rather than relying on re-awaiting it.
    blocker.release?.();
    await burstTick;
    for (let i = 0; i < 10000 && day <= 3000; i++) {
      await Promise.resolve();
    }

    expect(day).toBeGreaterThan(3000);
    runtime.pauseSim(SIM_ID);
  });

  // Regression test for a real report: pausing mid-catch-up (a backlog built
  // up at high speed) took ~2.7 real seconds to actually stop the day
  // counter, confirmed by driving a real browser -- the day-processing loop
  // never checked session.running, so once a large daysThisFrame was
  // computed it ran to completion regardless of a pause requested partway
  // through.
  it('pausing mid-catch-up stops within the day already in flight, not the whole backlog', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));

    let day = 0;
    let callCount = 0;
    const blocker: { release: (() => void) | null } = { release: null };
    const firstCallStarted = new Promise<void>((resolveStarted) => {
      vi.mocked(engine.advanceDay).mockImplementation(async () => {
        callCount += 1;
        if (callCount === 1) {
          resolveStarted();
          await new Promise<void>((resolve) => { blocker.release = resolve; });
        }
        day += 1;
        return { state: { current_day: day, speed_multiplier: 1, individuals: [] }, report: {}, phases: {} };
      });
    });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);
    await vi.advanceTimersByTimeAsync(100);

    // A large gap owes a multi-thousand-day backlog; its first advanceDay
    // call blocks immediately, simulating real in-flight compute.
    vi.setSystemTime(t0 + 100 + 3600 * 1000);
    const burstTick = vi.advanceTimersByTimeAsync(100);
    await firstCallStarted;
    expect(callCount).toBe(1);

    // Pause while that first day is still being "computed" -- pauseSim()
    // itself must not block on the in-flight tick.
    await runtime.pauseSim(SIM_ID);

    // Release the blocked call and let any already-queued microtasks run.
    blocker.release?.();
    await burstTick;
    for (let i = 0; i < 50; i++) await Promise.resolve();

    // The huge backlog (3600+ owed days) must NOT have drained -- the loop
    // should have stopped at or just past the single day already in flight
    // when pause was requested, not run to completion.
    expect(day).toBeLessThan(5);
    expect(callCount).toBeLessThan(5);
  });

  // Regression test for a real report: at 100x, the day counter went
  // "haywire" the longer the sim ran, and Pause/speed-change stopped
  // responding. Root cause: ordinary (foreground, not background-throttled)
  // ticks had no ceiling on daysThisFrame, so a tick slow enough to take
  // longer than TICK_INTERVAL_MS inflated the *next* tick's measured dt,
  // which computed an even larger daysOwed/daysThisFrame next time -- a
  // self-reinforcing spiral, confirmed in a real browser (daysThisFrame
  // climbing 0, 0, 1, 27, 459 across a handful of 100ms ticks). Simulates
  // that same slow-tick condition with a fixed per-call delay and asserts
  // daysThisFrame never exceeds MAX_DAYS_PER_TICK (30) on any single tick.
  it('caps ordinary (non-background) ticks so a slow tick cannot compound into an ever-larger backlog', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));

    let day = 0;
    const perTickDayCounts: number[] = [];
    let daysThisTick = 0;
    vi.mocked(engine.advanceDay).mockImplementation(async () => {
      day += 1;
      daysThisTick += 1;
      return { state: { current_day: day, speed_multiplier: 100, individuals: [] }, report: {}, phases: {} };
    });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);

    // 20 ordinary 100ms ticks at 100x (10 owed days/tick before any cap) --
    // real wall-clock time isn't simulated here (advanceDay resolves
    // instantly), but the cap must still hold on every tick regardless of
    // how much is owed, since a real slow tick's inflated dt would otherwise
    // hand this same loop a much larger daysThisFrame with no ceiling.
    for (let i = 0; i < 20; i++) {
      daysThisTick = 0;
      await vi.advanceTimersByTimeAsync(100);
      perTickDayCounts.push(daysThisTick);
    }

    for (const count of perTickDayCounts) {
      expect(count).toBeLessThanOrEqual(30);
    }

    runtime.pauseSim(SIM_ID);
  });

  // Regression test for a real report: even with MAX_DAYS_PER_TICK capping
  // each individual tick's jump, a genuine backlog (daysOwed) still builds up
  // once real per-day compute can't keep pace with a high speed_multiplier --
  // confirmed over a real multi-hundred-day run, where switching from 100x to
  // 1x kept advancing ~20-30 days over the next few seconds instead of ~3.
  // setSpeed() must drop that backlog so the newly chosen speed's pace is
  // reflected immediately, not after draining leftover debt from the old one.
  it('a manual speed change drops any accumulated backlog instead of draining it at the old pace', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));

    let day = 0;
    vi.mocked(engine.advanceDay).mockImplementation(async () => {
      day += 1;
      return { state: { current_day: day, speed_multiplier: 100, individuals: [] }, report: {}, phases: {} };
    });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);
    await vi.advanceTimersByTimeAsync(100); // establishes lastFrameAt

    // Simulate two real ticks that each took 3s of wall-clock compute (not a
    // background gap -- no visibilitychange dispatched, so this stays on the
    // ordinary/capped path) -- exactly what happens once population growth
    // makes a single 30-day capped batch itself take longer than
    // TICK_INTERVAL_MS to compute. Each owes 300 days at 100x but only
    // processes 30, so ~270 stays in daysOwed after the first, growing
    // further after the second -- a genuine backlog underneath the per-tick
    // cap, not fabricated.
    vi.setSystemTime(t0 + 100 + 3000);
    await vi.advanceTimersByTimeAsync(100);
    vi.setSystemTime(t0 + 100 + 6000);
    await vi.advanceTimersByTimeAsync(100);

    await runtime.setSpeed(SIM_ID, 1);
    day = 0; // reset the counter to measure only what happens after the speed change

    // One ordinary 100ms tick at the new 1x speed should own ~0-1 days, not a
    // chunk of the several-hundred-day backlog left over from 100x.
    await vi.advanceTimersByTimeAsync(100);
    expect(day).toBeLessThanOrEqual(1);

    runtime.pauseSim(SIM_ID);
  });

  // Regression test for the root cause behind the above: real per-day
  // compute (advanceDay's own worker round trip) taking real wall-clock time
  // used to inflate the *next* tick's dt, because lastFrameAt was stamped at
  // the top of tickOnce (before the day loop ran) rather than after -- so a
  // capped batch's own multi-second compute time got counted a second time,
  // as if it were idle time still owed at whatever speed was current on the
  // next tick. Simulates a single tick whose capped batch takes several real
  // seconds (each advanceDay call advances the system clock, mimicking real
  // compute latency) and asserts the following ordinary tick sees only a
  // small, TICK_INTERVAL_MS-sized dt, not the whole previous batch's
  // duration.
  it('a slow tick that took real wall-clock time to compute does not inflate the next tick\'s owed days', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));

    let day = 0;
    vi.mocked(engine.advanceDay).mockImplementation(async () => {
      day += 1;
      // Each simulated day "costs" 200ms of real compute -- a 30-day capped
      // batch then takes 6s of wall-clock time, same order of magnitude as
      // what a large population showed in real WASM-Local testing.
      vi.setSystemTime(Date.now() + 200);
      return { state: { current_day: day, speed_multiplier: 100, individuals: [] }, report: {}, phases: {} };
    });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);
    await runtime.setSpeed(SIM_ID, 100);

    // First-ever tick always uses dt = TICK_INTERVAL_MS (lastFrameAt starts
    // at 0), so this owes 10 days (100 * 100ms / 1000), under the 30-day
    // cap, and all 10 run -- each costing 200ms, so this tick's own compute
    // takes 2s of (mocked) wall-clock time.
    await vi.advanceTimersByTimeAsync(100);
    expect(day).toBe(10);

    day = 0;
    // The next ordinary 100ms tick must see only a small dt (~100ms, not the
    // ~2s the previous tick's own compute took) -- so at 100x it should owe
    // roughly another 10 days, not a chunk inflated by the last tick's
    // compute time compounding into this one.
    await vi.advanceTimersByTimeAsync(100);
    expect(day).toBeLessThanOrEqual(15);

    runtime.pauseSim(SIM_ID);
  });

  // Regression test for the other half of the same real report: resetting
  // daysOwed in setSpeed() only stops a FUTURE tick from claiming old-speed
  // debt -- it does nothing about a batch that was already sized and is
  // already mid-flight (each advanceDay call is a real worker round trip, so
  // a capped 30-day batch can itself take real seconds at a large
  // population). Confirmed live: switching 100x to 1x mid-batch still showed
  // ~20-30 days advancing over the next few seconds, because that in-flight
  // batch simply ran to completion, same shape as the pause-responsiveness
  // bug this session fixed earlier but for a speed change instead of a
  // pause. session.speedChangeSeq lets the loop notice and stop.
  it('a manual speed change stops an already in-flight batch instead of letting it finish at the old speed', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));

    let day = 0;
    let callCount = 0;
    const blocker: { release: (() => void) | null } = { release: null };
    const firstCallStarted = new Promise<void>((resolveStarted) => {
      vi.mocked(engine.advanceDay).mockImplementation(async () => {
        callCount += 1;
        if (callCount === 1) {
          resolveStarted();
          await new Promise<void>((resolve) => { blocker.release = resolve; });
        }
        day += 1;
        return { state: { current_day: day, speed_multiplier: 100, individuals: [] }, report: {}, phases: {} };
      });
    });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);
    await runtime.setSpeed(SIM_ID, 100);

    // This tick's batch (up to MAX_DAYS_PER_TICK=30 days, since 100x over
    // 100ms owes 10, well under the cap) starts; its first advanceDay call
    // blocks immediately, simulating a real in-flight batch.
    const burstTick = vi.advanceTimersByTimeAsync(100);
    await firstCallStarted;
    expect(callCount).toBe(1);

    // User picks 1x while that batch is still only one day in.
    await runtime.setSpeed(SIM_ID, 1);

    // Release the blocked call and let the rest of the (already-decided)
    // batch's own microtask chain settle.
    blocker.release?.();
    await burstTick;
    for (let i = 0; i < 50; i++) await Promise.resolve();

    // The batch must NOT have run to its full old-speed size (10 days) --
    // it should have stopped at or just past the single day already in
    // flight when the speed change landed.
    expect(day).toBeLessThan(5);
    expect(callCount).toBeLessThan(5);

    runtime.pauseSim(SIM_ID);
  });

  // Regression test for the actual root cause of the whole "speed change
  // doesn't take effect" report: sim-core's advance_day() echoes whatever
  // speed_multiplier was in the state it was given straight back out
  // unchanged. Each day-loop iteration reads session.stateJson as its input
  // *before* awaiting advanceDay -- so an in-flight call started just before
  // setSpeed() runs still resolves with the OLD speed baked into its
  // snapshot, and committing that result (session.stateJson =
  // JSON.stringify(result.state)) silently clobbered the just-written new
  // speed right back to the stale one. Confirmed live: after a real speed
  // change, the tick loop's own dt/speed logging kept reading the OLD speed
  // for several ticks afterward despite setSpeed() having already run and
  // even despite speedChangeSeq having already stopped the in-flight batch
  // early. session.speedMultiplier (an authoritative field the loop reads
  // instead of parseState(session.stateJson).speed_multiplier, and
  // re-stamps into every result before committing it) is what actually
  // fixes this -- speedChangeSeq alone only stopped the batch early, it
  // never stopped the clobbering.
  it('a stale in-flight advanceDay result cannot resurrect the old speed after a speed change', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));

    let day = 0;
    const blocker: { release: (() => void) | null } = { release: null };
    const firstCallStarted = new Promise<void>((resolveStarted) => {
      vi.mocked(engine.advanceDay).mockImplementation(async (stateJson: string) => {
        day += 1;
        const inputSpeed = (JSON.parse(stateJson) as { speed_multiplier: number }).speed_multiplier;
        if (day === 1) {
          resolveStarted();
          await new Promise<void>((resolve) => { blocker.release = resolve; });
        }
        // Echoes the speed that was in its OWN input snapshot, exactly like
        // the real wasm engine does -- the first call's input still says
        // 100, captured before the speed change below ever ran.
        return { state: { current_day: day, speed_multiplier: inputSpeed, individuals: [] }, report: {}, phases: {} };
      });
    });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);
    await runtime.setSpeed(SIM_ID, 100);

    const burstTick = vi.advanceTimersByTimeAsync(100);
    await firstCallStarted;

    // Speed change lands while the first call (input snapshot says 100) is
    // still in flight.
    await runtime.setSpeed(SIM_ID, 1);

    blocker.release?.();
    await burstTick;
    for (let i = 0; i < 50; i++) await Promise.resolve();

    // Once that stale call resolves and commits its result, the NEXT tick's
    // own income calculation must still see speed 1, not 100 clobbered back
    // in via the committed (stale) state.
    day = 0;
    await vi.advanceTimersByTimeAsync(100);
    expect(day).toBeLessThanOrEqual(1);

    runtime.pauseSim(SIM_ID);
  });

  // Regression test for a real bug this session's own performance deep-dive
  // caught: the Performance panel's "Compute" bucket used to reuse the same
  // number as "TICK TIMING" (the full postMessage round trip, dominated by
  // worker/serialization overhead in a resource-constrained browser), while
  // the "Module" rows beneath it came from the wasm call's own genuine
  // per-phase timings -- so "Compute" could show ~200ms while every row
  // under it rounded to 0ms and never explained where the time went.
  // lastComputeMs must instead track the phases sum, distinct from lastMs.
  it('tracks engine-only compute time (phases sum) separately from the full worker round trip', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));
    let day = 0;
    vi.mocked(engine.advanceDay).mockImplementation(async () => {
      day += 1;
      return {
        state: { current_day: day, speed_multiplier: 1, individuals: [] },
        report: {},
        // Sums to 3ms of genuine engine time -- what the "Module" rows in
        // the Performance panel should add up to.
        phases: { setup_ms: 1, economy_ms: 2 },
      };
    });

    await runtime.startSim(SIM_ID);
    // speed=1 owes 0.1 simulated days per 100ms tick -- floating-point
    // accumulation of ten 0.1s doesn't reliably clear 1.0 (classic IEEE754
    // rounding), so this generously over-loops rather than assuming exactly
    // ten ticks owe a whole day. Polls this test's own session via
    // getTickTiming()'s sampleCount specifically (not the shared `day`
    // closure the mock bumps) -- a still-unwinding async chain from a
    // previous test's own module instance can keep calling this same mocked
    // engine.advanceDay well after that test returns (the mock is a shared
    // module-level double, but each test's `runtime` import is its own
    // fresh module instance with its own `session`), which would otherwise
    // bump `day` without ever touching *this* session's tickTiming.
    for (let i = 0; i < 20 && !runtime.getTickTiming()?.sampleCount; i++) {
      await vi.advanceTimersByTimeAsync(100);
    }

    const timing = runtime.getTickTiming();
    // Before this fix, tick_compute_ms (fed from lastMs, the full round
    // trip including postMessage/serialization overhead) had no relation to
    // the phases actually reported -- the Performance panel's "Compute"
    // bucket and its own "Module" rows could never add up. lastComputeMs
    // must equal the phases sum precisely, as its own dedicated figure.
    expect(timing?.lastPhases).toEqual({ setup_ms: 1, economy_ms: 2 });
    expect(timing?.lastComputeMs).toBe(3);

    runtime.pauseSim(SIM_ID);
  });

  // Regression test for a real bug a 50-simulated-year/100x reproduction run
  // caught: fullSync() (two full-state worker round trips -- getStats +
  // getEvents, each scaling with total individuals ever born) is meant to be
  // throttled to once a second by FULL_SYNC_INTERVAL_MS, but an `||
  // daysThisFrame > 0` clause defeated that entirely at any speed high
  // enough for daysThisFrame to be >0 on nearly every 100ms tick (speed >=
  // ~10x) -- firing on almost every tick instead. The reproduction run
  // measured 1825 getEvents calls across 1825 ticks (should be ~183, once
  // per ~10 ticks) -- a ~10x excess of worker round trips that got more
  // expensive as the ever-growing individuals array grew, and was the
  // dominant cause of a periodic freeze-then-jump reproducible even with the
  // tab fully foregrounded (unrelated to the earlier background-throttling
  // fix, since tickInFlight visibly stalls setStatsDay for however long one
  // of these slow calls takes, then the backlog that piled up during the
  // stall drains in a rapid burst of consecutive capped ticks).
  it('throttles fullSync (getStats/getEvents) to roughly once a second even at high speed, not once per tick', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    const SPEED = 100;
    let day = 0;
    vi.mocked(engine.advanceDay).mockImplementation(async () => {
      day += 1;
      return { state: { current_day: day, speed_multiplier: SPEED, individuals: [] }, report: {}, phases: {} };
    });
    vi.mocked(dbLoadSimulation).mockResolvedValue({
      id: SIM_ID,
      name: 'Test Sim',
      status: 'running',
      current_day: 0,
      current_year: 0,
      start_latitude: 0,
      start_longitude: 0,
      speed_multiplier: SPEED,
      stateJson: JSON.stringify({ current_day: 0, speed_multiplier: SPEED, individuals: [] }),
      created_at: 0,
      updated_at: 0,
    });

    const t0 = 1_000_000;
    vi.setSystemTime(t0);
    await runtime.startSim(SIM_ID);

    // 10 real seconds at 100 simulated ticks/sec -- every one of them advances
    // at least one simulated day (daysThisFrame > 0), which is exactly the
    // condition that used to force fullSync on every single tick.
    const TOTAL_TICKS = 100;
    for (let i = 0; i < TOTAL_TICKS; i++) {
      await vi.advanceTimersByTimeAsync(100);
    }

    // Should be close to TOTAL_TICKS / 10 (once a second), not close to
    // TOTAL_TICKS (once a tick, the pre-fix behavior).
    expect(vi.mocked(engine.getEvents).mock.calls.length).toBeLessThan(TOTAL_TICKS * 0.3);
    expect(vi.mocked(engine.getEvents).mock.calls.length).toBeGreaterThan(TOTAL_TICKS * 0.05);

    runtime.pauseSim(SIM_ID);
  });

  // Diagnostic instrumentation: persist()/maybeCheckpoint()/fullSync()'s own
  // getStats+getEvents call are all awaited inside tickOnce() but were never
  // wrapped by recordTickTiming, so a real periodic stall in any of them
  // (all three scale with stateJson's size or total-events, which only grow
  // over a run) could hide entirely from the Performance panel's tick
  // timing -- a real user report showed exactly that: healthy tick/persist/
  // checkpoint numbers alongside a visible stall, with fullSync's own
  // getStats/getEvents round trip being the one remaining unmeasured piece.
  // lastPersistMs/lastCheckpointMs/lastFullSyncMs surface each one's own
  // last duration so that can be confirmed with real numbers instead of
  // guesswork.
  it('records persist()/checkpoint()/fullSync() durations separately from tick timing', async () => {
    const { dbLoadSimulation } = await import('./db');
    const { engine } = await import('./engineClient');
    const runtime = await import('./runtime');

    vi.mocked(dbLoadSimulation).mockResolvedValue(fakeRecord(0));
    let day = 0;
    vi.mocked(engine.advanceDay).mockImplementation(async () => {
      day += 1;
      return { state: { current_day: day, speed_multiplier: 1, individuals: [] }, report: {}, phases: {} };
    });

    // A far-future system time (not 0/near-0, where fake timers start) so
    // the very first tick's "enough time since last sync" check trivially
    // passes -- matching the same trick other tests in this file use.
    vi.setSystemTime(1_000_000);
    await runtime.startSim(SIM_ID);
    // startSim() itself calls persist() once before the timer even starts.
    expect(runtime.getTickTiming()?.lastPersistMs).not.toBeNull();
    // The tick loop's own income math reads session.speedMultiplier (set
    // here), not whatever advanceDay's mock echoes back in state -- see
    // ActiveSession.speedMultiplier's own doc comment for why.
    await runtime.setSpeed(SIM_ID, 5000);

    // speed_multiplier is clamped to 1000, but MAX_DAYS_PER_TICK caps
    // ordinary (non-background-catch-up) ticks at 30 days each regardless --
    // a checkpoint (every 365 sim days) needs several fullSync calls
    // (fullSync itself is throttled to ~once a second) to have enough
    // accumulated days, hence the generous loop bound.
    for (let i = 0; i < 60 && runtime.getTickTiming()?.lastCheckpointMs == null; i++) {
      await vi.advanceTimersByTimeAsync(100);
    }

    const timing = runtime.getTickTiming();
    expect(timing?.lastCheckpointMs).not.toBeNull();
    expect(timing?.lastFullSyncMs).not.toBeNull();
    expect(typeof timing?.lastPersistMs).toBe('number');
    expect(typeof timing?.lastCheckpointMs).toBe('number');
    expect(typeof timing?.lastFullSyncMs).toBe('number');

    runtime.pauseSim(SIM_ID);
  });
});
