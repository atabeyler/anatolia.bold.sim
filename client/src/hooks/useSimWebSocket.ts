import { useEffect, useRef } from 'react';
import axios from 'axios';
import { useSimStore } from '../store/simStore';
import { playTick, playNotification } from '../utils/audioEngine';
import { LOCAL_SERVER_URL, isNativeAndroidApp, isYerelModeActive } from '../utils/nativeMode';
import { CLOUD_API_URL } from '../utils/cloud';
import { isWasmLocalModeActive } from '../wasmLocal/mode';

export function useSimWebSocket(simId: string | null) {
  const ws = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectDelay = useRef(3000);
  const unmounted = useRef(false);
  const isFirstConnect = useRef(true);
  const lastMessageAt = useRef(0);
  const lastTickDay = useRef<number | null>(null);
  const { accessToken, setStats, setStatsDay, addEvent, setCentroidTrail, addMilestone, setIsWarping, setFastForwardTarget, setWsStatus, setWsLastMessageAt, setWsCloseInfo, incrementWsReconnectCount } = useSimStore();

  useEffect(() => {
    // WASM-local mode has no server and no socket at all -- runtime.ts's
    // own tick loop already pushes stats/events/milestones straight into
    // this same store, so this hook has nothing to do but stay out of the
    // way (also: it has no accessToken to gate on, since that mode has no
    // account either).
    if (isWasmLocalModeActive()) return;
    if (!simId || !accessToken) return;
    unmounted.current = false;
    reconnectDelay.current = 3000;
    isFirstConnect.current = true;

    function connect() {
      if (unmounted.current) return;
      setWsStatus('connecting');
      // Android "Yerel" mode's local sim-server isn't reachable at
      // location.host (the Capacitor webview's own virtual origin) --
      // it's a real subprocess bound to 127.0.0.1, same as desktop's.
      // Android "Bulut" mode doesn't navigate the WebView away from its own
      // bundled origin either (NativeModeGate.tsx's chooseCloud() only ever
      // repoints axios.defaults.baseURL, never window.location -- a
      // deliberate workaround for a Capacitor plugin-bridge bug on a real
      // navigation), so location.host there is still the Capacitor virtual
      // host, not anatolia-sim.onrender.com -- the socket silently tried to
      // connect to a nonexistent local endpoint and never delivered a
      // single tick, leaving the live watch screen looking frozen even
      // though the simulation kept running server-side.
      const wsHost = isYerelModeActive()
        ? LOCAL_SERVER_URL.replace(/^http/, 'ws')
        : isNativeAndroidApp()
        ? CLOUD_API_URL.replace(/^http/, 'ws')
        : `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}`;
      const url = `${wsHost}/ws?simId=${encodeURIComponent(simId!)}`;
      const socket = new WebSocket(url);
      ws.current = socket;

      // Send token in first message, never in URL (URL is logged by proxies/CDNs).
      socket.onopen = () => {
        reconnectDelay.current = 3000; // reset backoff on successful connect
        lastMessageAt.current = Date.now();
        setWsStatus('open');
        setWsLastMessageAt(Date.now());
        socket.send(JSON.stringify({ type: 'auth', token: accessToken }));
      };

      socket.onmessage = (e) => {
        lastMessageAt.current = Date.now();
        setWsLastMessageAt(Date.now());
        // Respond to JSON-level pings (some proxies strip native WS ping frames)
        if (e.data === '{"type":"ping"}') { socket.send('{"type":"pong"}'); return; }
        try {
          const data = JSON.parse(e.data);
          if (data.type === 'tick') {
            if (data.stats) setStats(data.stats);
            if (data.events) data.events.forEach(addEvent);
            if (data.centroid_trail) setCentroidTrail(data.centroid_trail);
            if (typeof data.is_warping === 'boolean') setIsWarping(data.is_warping);
            if ('fast_forward_target' in data) setFastForwardTarget(data.fast_forward_target ?? null);
            if (typeof data.current_day === 'number' && data.current_day !== lastTickDay.current) {
              lastTickDay.current = data.current_day;
              // A lightweight day-only "tick" (no `stats`) arrives far more often
              // than the full-stats one at high sim speed, so the on-screen day
              // counter climbs smoothly instead of jumping in speed-sized chunks.
              if (!data.stats) setStatsDay(data.current_day);
              if (useSimStore.getState().soundSettings.tickEnabled) playTick();
            }
          } else if (data.type === 'milestone') {
            if (useSimStore.getState().soundSettings.notificationEnabled) playNotification();
            addMilestone({ key: data.key, description: data.description, icon: data.icon ?? '🏆', day: data.day });
          } else if (data.type === 'status') {
            // Server tells us the real Rust runtime state on connect.
            if (typeof data.is_warping === 'boolean') setIsWarping(data.is_warping);
            if ('fast_forward_target' in data) setFastForwardTarget(data.fast_forward_target ?? null);
            // Auto-trigger start only on the first connection per session,
            // not on reconnects — prevents restart loop when user intentionally pauses.
            if (data.runtime_running === false && isFirstConnect.current) {
              const { currentSim, accessToken: tok } = useSimStore.getState();
              if (currentSim?.status === 'running' && tok) {
                // axios (not a bare fetch()) so this respects the
                // axios.defaults.baseURL redirect NativeModeGate sets for
                // Android "Yerel" mode -- a relative fetch() here resolves
                // against the Capacitor webview's own origin, not the local
                // sim-server at 127.0.0.1, and silently fails, leaving a
                // simulation whose runtime session died (e.g. the local
                // sim-server subprocess was restarted) stuck at status
                // "running" in the DB with nothing actually ticking it.
                axios.post(`/api/simulations/${currentSim.id}/start`, {}, {
                  headers: { Authorization: `Bearer ${tok}` },
                }).catch(() => {});
              }
            }
          } else if (data.type === 'simulation_ended') {
            useSimStore.getState().setSimulationEnded(data.reason ?? 'unknown');
          } else if (data.type === 'error') {
            console.error('[WS]', data.error);
          }
        } catch (err) { console.debug('[WS] message parse error:', err); }
      };

      socket.onerror = (err) => { console.debug('[WS] socket error:', err); setWsStatus('error'); };

      socket.onclose = (event) => {
        isFirstConnect.current = false;
        if (unmounted.current) return;
        // A stale connection can be force-closed by onVisibilityChange while a
        // new socket is already connecting; ignore the old socket's belated
        // close event so it doesn't schedule a second, duplicate reconnect.
        if (ws.current !== socket) return;
        setWsStatus('closed');
        setWsCloseInfo({ code: event.code, reason: event.reason || '' });
        // 1008 = simulation not found / policy violation — stop reconnecting
        if (event.code === 1008) return;
        // Reconnect with capped exponential backoff (3s → 6s → 12s → 30s max)
        incrementWsReconnectCount();
        const delay = reconnectDelay.current;
        reconnectDelay.current = Math.min(delay * 2, 30000);
        reconnectTimer.current = setTimeout(connect, delay);
      };
    }

    // Immediately reconnect when user returns to the tab (mobile background/lock screen fix).
    // Resets exponential backoff so there's no 30s wait after a long absence.
    //
    // readyState alone isn't enough: the server sends a message roughly every
    // second while the connection is genuinely alive (see ws.rs's 1s tick loop),
    // but a "zombie" connection -- silently dropped by a proxy or OS while the
    // tab was backgrounded -- can sit in readyState OPEN indefinitely without
    // ever firing onclose, so the client would otherwise never notice and the
    // UI would look frozen until the user manually refreshes. Track how long
    // it's been since we last heard anything and force a fresh connection if
    // that gap is too large to be a live connection.
    function onVisibilityChange() {
      if (document.hidden || unmounted.current) return;
      const state = ws.current?.readyState;
      const stale = state === WebSocket.OPEN && Date.now() - lastMessageAt.current > 5000;
      if (state === WebSocket.CLOSED || state === WebSocket.CLOSING || ws.current == null || stale) {
        if (reconnectTimer.current) { clearTimeout(reconnectTimer.current); reconnectTimer.current = null; }
        reconnectDelay.current = 3000;
        ws.current?.close();
        connect();
      }
    }

    // iOS bfcache: page restored from memory with stale WS object — force fresh connect.
    function onPageShow(e: PageTransitionEvent) {
      if (!e.persisted || unmounted.current) return;
      ws.current?.close();
      if (reconnectTimer.current) { clearTimeout(reconnectTimer.current); reconnectTimer.current = null; }
      reconnectDelay.current = 3000;
      connect();
    }

    connect();
    document.addEventListener('visibilitychange', onVisibilityChange);
    window.addEventListener('pageshow', onPageShow);

    return () => {
      unmounted.current = true;
      document.removeEventListener('visibilitychange', onVisibilityChange);
      window.removeEventListener('pageshow', onPageShow);
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
        reconnectTimer.current = null;
      }
      ws.current?.close();
    };
  }, [simId, accessToken]);

  return ws;
}
