use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use tokio::time::interval;

use sim_core::{derive_stats, to_client_event};

use crate::{
    auth::authenticate_token,
    db::{load_bounded_tick_state_no_genealogy, load_simulation, row_to_state, AppState},
};

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    #[serde(rename = "simId")]
    pub sim_id: Option<String>,
}

/// Mirrors routes.rs's `authorize_sim_access`: an orphaned simulation (no
/// `user_id` at all -- a legacy row predating the ownership column) is
/// treated as accessible to any authenticated caller, the same as every
/// other simulation-scoped endpoint, rather than being uniquely locked out
/// here just because `None != Some(claims.id)`.
fn owns_simulation_or_is_admin(sim_user_id: Option<&str>, claims_id: &str, claims_role: &str) -> bool {
    claims_role == "admin" || sim_user_id.is_none() || sim_user_id == Some(claims_id)
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    let Some(sim_id) = query.sim_id else {
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    };
    ws.on_upgrade(move |socket| handle_socket(socket, state, sim_id))
}

async fn close_socket(socket: &mut WebSocket) {
    let _ = socket.send(Message::Close(None)).await;
}

// A silently-dropped mobile connection (NAT rebind, OS killing the socket
// while backgrounded, no FIN/RST ever delivered -- see useSimWebSocket.ts's
// own staleness comment for the client-side half of this) can leave a
// server-side `socket.send().await` hanging for however long the OS's TCP
// retransmission timeout happens to be, which is typically tuned for
// minutes, not seconds. Until that call resolves, this connection's entire
// select! loop is stuck -- not just failing to notice the client is gone,
// but still polling the DB on every tick.tick() in the meantime, doubling
// load on the very same simulation a *new* reconnecting connection is
// already polling. Bounding every send with a short timeout turns a
// potentially minutes-long stall into a five-second one.
const SEND_TIMEOUT: Duration = Duration::from_secs(5);

/// Returns true if the caller should break out of the connection's loop --
/// either the send failed outright, or it didn't complete within
/// SEND_TIMEOUT, which for a live connection (RTT of tens to hundreds of ms)
/// never legitimately happens.
async fn send_or_timeout(socket: &mut WebSocket, msg: Message) -> bool {
    !matches!(tokio::time::timeout(SEND_TIMEOUT, socket.send(msg)).await, Ok(Ok(())))
}

/// `derive_stats` computes both `day` and `year` from `sim` -- the stale
/// full-DB reload, which can lag behind the tick loop's true in-memory
/// progress (a batch still in flight, or upload deliberately paused -- see
/// runtime.rs's should_flush_upload). The broadcast loop already tracks the
/// freshest known day as `current_day` (via live_day); this patches both
/// fields onto it so they always agree with each other and with the
/// separate top-level `current_day` sent alongside `stats` -- missing the
/// `year` half of this patch is exactly what let a paused-upload session's
/// day counter visibly climb while the year counter stayed frozen.
fn patch_stats_day_and_year(mut stats: serde_json::Value, current_day: i32) -> serde_json::Value {
    if let Some(obj) = stats.as_object_mut() {
        obj.insert("day".to_string(), json!(current_day));
        obj.insert("year".to_string(), json!(current_day / 365));
    }
    stats
}

async fn handle_socket(mut socket: WebSocket, state: AppState, sim_id: String) {
    let auth_msg = match socket.recv().await {
        Some(Ok(Message::Text(text))) => text,
        _ => {
            close_socket(&mut socket).await;
            return;
        }
    };

    let token = match serde_json::from_str::<serde_json::Value>(&auth_msg)
        .ok()
        .and_then(|v| {
            if v.get("type").and_then(|v| v.as_str()) == Some("auth") {
                v.get("token").and_then(|v| v.as_str()).map(|s| s.to_string())
            } else {
                None
            }
        }) {
        Some(token) => token,
        None => {
            close_socket(&mut socket).await;
            return;
        }
    };

    // Must go through authenticate_token (not decode_access_token directly) --
    // the local/SQLite backend (Android "Yerel" mode, desktop) has no
    // JWT_SECRET of its own and never mints its own logins, so a token
    // decoded with its default secret would never match one the cloud
    // actually signed. authenticate_token knows to vouch for local-backend
    // tokens via the cloud's /api/auth/me instead. Every earlier version of
    // this handler skipped that and closed the socket (codeless, client-side
    // close code 1005) on every single connection attempt in Yerel mode.
    let claims = match authenticate_token(&state, &token).await {
        Some(claims) => claims,
        None => {
            close_socket(&mut socket).await;
            return;
        }
    };

    let sim_row = match load_simulation(&state.backend, &sim_id).await {
        Ok(Some(row)) => row,
        _ => {
            close_socket(&mut socket).await;
            return;
        }
    };

    let sim = row_to_state(&sim_row);
    if !owns_simulation_or_is_admin(sim.user_id.as_deref(), &claims.id, &claims.role) {
        close_socket(&mut socket).await;
        return;
    }

    let mut last_day: Option<i32> = None;
    let mut last_fast_day: Option<i32> = None;
    let mut sent_events: usize = sim.events.len();
    // Guards against re-sending the same reason every second for as long as
    // the condition holds -- the client shows a confirmation modal on
    // receipt, and re-triggering it every tick while the user is still
    // looking at (or has dismissed) the first one would be unusable. Resets
    // naturally on reconnect (a fresh handle_socket call), which is the
    // right amount of "ask again" -- once per session, not once ever.
    let mut last_extinction_reason: Option<&'static str> = None;
    let mut heartbeat = interval(Duration::from_secs(30));
    let mut tick = interval(Duration::from_secs(1));
    // Lightweight day-only ping: the tick loop (runtime.rs) only writes to the
    // DB once per batch (up to `speed` days at a time), so polling it once a
    // second here would still show the on-screen day counter jumping in
    // speed-sized chunks. runtime.rs paces its in-memory `live_day` after
    // every single simulated day; sampling *that* several times a second (no
    // DB round trip, just an atomic read) is what makes the counter climb
    // smoothly again instead of jumping.
    let mut fast_tick = interval(Duration::from_millis(120));

    'conn: loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if send_or_timeout(&mut socket, Message::Ping(vec![])).await {
                    break;
                }
            }
            _ = fast_tick.tick() => {
                if let Some(day) = state.runtime.live_day(&sim_id).await {
                    if last_fast_day != Some(day) {
                        last_fast_day = Some(day);
                        if send_or_timeout(&mut socket, Message::Text(json!({
                            "type": "tick",
                            "current_day": day,
                        }).to_string())).await {
                            break;
                        }
                    }
                }
            }
            _ = tick.tick() => {
                // Prefer the tick loop's own live in-memory state (only ever
                // populated while upload is paused -- see runtime.rs's
                // live_state) over a DB reload: while paused, the DB stops
                // being written to almost entirely, so a plain load_full_state
                // here would make population/events/stats appear frozen even
                // though the simulation keeps running.
                //
                // The fallback reload uses the same bounded query the tick
                // loop's own per-batch load does (alive + recently-dead only,
                // no full genealogy walk) instead of load_full_state's
                // "every individual ever born" scan -- this runs once a
                // second per open connection, and load_full_state's cost
                // grows with the simulation's entire lifetime population, not
                // just its current size. At high speed that population (and
                // so this reload's latency) grows fast enough to blow past
                // the 1s tick interval, which stalls this whole select! arm
                // (including the fast_tick day-only pings that only fire
                // between iterations) until it finishes -- observed as the
                // on-screen day counter freezing for several seconds then
                // jumping forward by a whole speed-sized batch at once,
                // instead of the smooth per-day climb fast_tick is meant to
                // provide. Stats/events/extinction-check below only ever look
                // at current data, never at long-dead individuals, so the
                // narrower window loses nothing they read.
                let sim = match state.runtime.live_state(&sim_id).await {
                    Some(sim) => sim,
                    None => match load_bounded_tick_state_no_genealogy(&state.backend, &sim_id).await {
                        Ok(Some(sim)) => sim,
                        _ => break,
                    },
                };
                // live_day (runtime.rs, updated in-memory after every single
                // simulated day) can be ahead of sim.current_day (only ever as
                // fresh as the tick loop's last DB save) for as long as a batch
                // is still in flight -- up to `speed` days behind at high speed.
                // Reporting the stale, smaller value here would visibly regress
                // the day counter this same connection's fast_tick branch had
                // already advanced past just moments earlier, once a second,
                // until the batch's save caught up -- the sim clock appearing to
                // jump backward then forward again. Never report a day older
                // than what's already been shown.
                let live_day = state.runtime.live_day(&sim_id).await;
                let current_day = live_day.map(|d| d.max(sim.current_day)).unwrap_or(sim.current_day);
                let status_running = sim.status.as_deref() == Some("running");

                if send_or_timeout(&mut socket, Message::Text(json!({
                    "type": "status",
                    "runtime_running": status_running,
                    "is_warping": false,
                    "fast_forward_target": serde_json::Value::Null,
                    "current_day": current_day,
                }).to_string())).await {
                    break;
                }

                // The tick loop itself (runtime.rs) now auto-terminates a
                // simulation that hits this same condition -- see its own
                // sim_core::mark_extinct call -- but that write can lag a few
                // hundred ms behind what this socket already has in memory.
                // Reporting it here too, once per reason per connection, lets
                // the client pop the end-of-simulation modal immediately
                // instead of waiting for its next status poll to notice.
                if status_running {
                    if let Some(reason) = sim_core::extinction_reason(&sim.individuals) {
                        if last_extinction_reason != Some(reason) {
                            last_extinction_reason = Some(reason);
                            if send_or_timeout(&mut socket, Message::Text(json!({
                                "type": "simulation_ended",
                                "reason": reason,
                            }).to_string())).await {
                                break;
                            }
                        }
                    }
                }

                // Forward every birth/death/milestone event recorded since the
                // last poll (previously hardcoded to `[]`, so the client never
                // received them regardless of what the sim engine produced).
                let new_events: Vec<serde_json::Value> = if sent_events <= sim.events.len() {
                    sim.events[sent_events..].to_vec()
                } else {
                    // state.events was trimmed (MAX_EVENTS) since our last read.
                    sim.events.clone()
                };
                sent_events = sim.events.len();

                if last_day.map(|d| d != current_day).unwrap_or(false) || !new_events.is_empty() {
                    let client_events: Vec<serde_json::Value> = new_events.iter().map(|e| to_client_event(e, &sim)).collect();
                    // Full stats (births/deaths/technologies/etc.), not just the four
                    // fields the tick payload used to carry -- setStats() on the client
                    // replaces the whole `stats` object rather than merging into it, so
                    // anything left out here was silently wiped every tick.
                    let stats = patch_stats_day_and_year(derive_stats(&sim), current_day);
                    let tick_payload = json!({
                        "type": "tick",
                        "current_day": current_day,
                        "stats": stats,
                        "events": client_events,
                        "centroid_trail": [],
                        "is_warping": false,
                        "fast_forward_target": serde_json::Value::Null,
                    });
                    if send_or_timeout(&mut socket, Message::Text(tick_payload.to_string())).await {
                        break;
                    }
                }

                for event in &new_events {
                    if event.get("type").and_then(serde_json::Value::as_str) == Some("milestone")
                        && send_or_timeout(&mut socket, Message::Text(json!({
                            "type": "milestone",
                            "key": event.get("key"),
                            "description": event.get("description"),
                            "icon": event.get("icon"),
                            "day": event.get("day"),
                        }).to_string())).await
                    {
                        break 'conn;
                    }
                }
                last_day = Some(current_day);
                last_fast_day = Some(current_day);
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── owns_simulation_or_is_admin() ────────────────────────────────────
    // Regression tests for parity with routes.rs's authorize_sim_access,
    // which was previously stricter here: `sim.user_id.unwrap_or_default()
    // != claims.id` denied even an orphaned (no user_id at all) simulation
    // to any non-admin, since `"" != claims.id` is always true.

    #[test]
    fn orphaned_simulation_is_accessible_to_any_authenticated_user() {
        assert!(owns_simulation_or_is_admin(None, "user-1", "user"));
    }

    #[test]
    fn the_owner_can_access_their_own_simulation() {
        assert!(owns_simulation_or_is_admin(Some("user-1"), "user-1", "user"));
    }

    #[test]
    fn a_non_owner_cannot_access_someone_elses_simulation() {
        assert!(!owns_simulation_or_is_admin(Some("user-1"), "user-2", "user"));
    }

    #[test]
    fn an_admin_can_access_anyones_simulation() {
        assert!(owns_simulation_or_is_admin(Some("user-1"), "user-2", "admin"));
    }

    // extinction_reason's own behavior is covered by sim_core::client_view's
    // test module now that it lives there (shared with runtime.rs).

    #[test]
    fn patching_day_forward_also_advances_year_to_match() {
        // Simulates exactly the paused-upload bug: derive_stats' own stale
        // "day"/"year" (both frozen at the DB's last save) get overridden by
        // a much-further-along live current_day -- year must move with it,
        // not stay behind.
        let stale_stats = json!({ "day": 10, "year": 0, "population": 5 });
        let patched = patch_stats_day_and_year(stale_stats, 800);
        assert_eq!(patched["day"], 800);
        assert_eq!(patched["year"], 2);
    }

    #[test]
    fn day_and_year_always_agree_with_each_other() {
        for current_day in [0, 1, 364, 365, 366, 3650] {
            let patched = patch_stats_day_and_year(json!({}), current_day);
            assert_eq!(patched["year"], current_day / 365, "day={current_day}");
        }
    }
}
