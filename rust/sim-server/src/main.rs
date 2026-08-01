#![recursion_limit = "256"]

mod admin;
mod analysis;
mod aria;
mod db;
mod auth;
mod email;
mod gemini;
mod god;
mod ratelimit;
mod releases;
mod routes;
mod runtime;
mod ws;

use std::{net::SocketAddr, path::PathBuf};

use axum::{extract::Request, middleware::{self, Next}, response::Response, routing::{get, post}, Router};
use db::AppState;
use routes::{health, simulation_routes, system_status};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, services::{ServeDir, ServeFile}};
use tracing_subscriber::EnvFilter;

// Cross-origin isolation, required for `SharedArrayBuffer` -- which
// wasm-bindgen-rayon's real multi-threaded WASM-local build (see
// rust/sim-wasm's own Cargo.toml) needs to share memory across the Web
// Worker thread pool it spins up (client/src/wasmLocal/worker.ts's
// initThreadPool call). Applied to every response, not just the static
// client bundle: COOP/COEP only take effect when present on the top-level
// document's own response, and this is the one server that ever serves that
// document to a real browser (see main()'s own ServeDir fallback -- there is
// no separate static-site host for the browser-facing deployment).
// `require-corp`, not `credentialless`: this app used to load two
// cross-origin subresources -- the Google Fonts `@import` in index.css and
// the Earth textures WorldGlobe.tsx pulled from raw.githubusercontent.com --
// which sent no CORP header and would be blocked outright under
// `require-corp`, so `credentialless` (isolates the page but only strips
// credentials from cross-origin no-cors requests instead of rejecting them)
// was used instead. Both are now self-hosted (client/public/textures,
// @fontsource packages -- see index.css/WorldGlobe.tsx), leaving no
// cross-origin subresource at all, so there's no longer a reason not to use
// `require-corp` -- confirmed necessary in practice: iOS WebKit (both Safari
// and Chrome-on-iOS, which share the same engine) does not reliably report
// crossOriginIsolated=true under `credentialless`, silently falling back to
// 1 thread, while a desktop browser correctly went multi-threaded under the
// same header. `require-corp` has universal support and fixed it.
async fn cross_origin_isolation_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert("Cross-Origin-Opener-Policy", axum::http::HeaderValue::from_static("same-origin"));
    headers.insert("Cross-Origin-Embedder-Policy", axum::http::HeaderValue::from_static("require-corp"));
    response
}

// Rayon's default global pool sizes itself off the *host* machine's core
// count (via num_cpus), not necessarily a shared/throttled container's real
// cgroup CPU quota. Left fully open (no platform-specific cap) since a Fly.io
// machine's reported core count matches what it's actually allotted -- this
// same binary also runs as desktop's and Android "Yerel" mode's local
// sim-server subprocess, with the device's own cores entirely to itself, so
// a shared default that just uses everything available serves both cases.
// Override with RAYON_NUM_THREADS if a specific deployment ever needs tuning.
fn configure_rayon_thread_pool() {
    let available = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
    let threads: usize = std::env::var("RAYON_NUM_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(available);
    if let Err(err) = rayon::ThreadPoolBuilder::new().num_threads(threads.max(1)).build_global() {
        tracing::warn!(error = %err, "failed to configure rayon thread pool (already initialized?)");
    }
}

// CORS allowlist for credentialed cross-origin requests (the refresh-token
// cookie flow): the production origin, plus the two concrete local-bridge
// origins the native shells actually present -- desktop's Tauri sidecar
// (http://127.0.0.1:<dynamic port>) and Android's Capacitor webview
// (https://localhost) -- see client/src/utils/cloud.ts's isLocalOrigin().
// Never an arbitrary attacker-controlled origin.
fn is_allowed_origin(origin_str: &str) -> bool {
    if origin_str == "https://anatolia-bold-sim.fly.dev" {
        return true;
    }
    for scheme_host in ["http://127.0.0.1", "https://127.0.0.1", "http://localhost", "https://localhost"] {
        if origin_str == scheme_host || origin_str.starts_with(&format!("{scheme_host}:")) {
            return true;
        }
    }
    false
}

// Same host-vs-cgroup oversubscription problem as rayon above: #[tokio::main]'s
// default multi-thread runtime also sizes worker_threads off the *host's*
// detected core count, not the container's real CPU quota. Capping this too
// (TOKIO_WORKER_THREADS overrides it) is what actually keeps a trivial,
// zero-I/O handler like /api/health inside Render's 5s health-check window
// while the tick loop's rayon work is running -- capping rayon alone left
// tokio free to still oversubscribe on its own.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let worker_threads: usize = std::env::var("TOKIO_WORKER_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(2);
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .build()?
        .block_on(run())
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("sim_server=info".parse()?))
        .init();
    configure_rayon_thread_pool();

    let state = AppState::new().await?;
    // access_secret()/refresh_secret() (auth.rs) fall back to hardcoded
    // strings when JWT_SECRET/JWT_REFRESH_SECRET aren't set -- deliberately
    // panic-free there, since a local (SQLite) instance's own login/register
    // routes are already hard-blocked (is_local_backend) and its /refresh
    // handler only ever fails to decode a cookie it could never have issued
    // itself, and the test suite relies on that same fallback being stable
    // and always available. Only the cloud/Postgres backend actually mints
    // sessions real accounts rely on, so only it needs this checked loudly
    // at startup -- this guards a self-hosted or misconfigured deployment
    // from silently signing every session with a secret anyone can read in
    // this file.
    if !auth::is_local_backend(&state) && (std::env::var("JWT_SECRET").is_err() || std::env::var("JWT_REFRESH_SECRET").is_err()) {
        panic!("JWT_SECRET and JWT_REFRESH_SECRET must both be set when running against the cloud/Postgres backend -- refusing to start with the hardcoded fallback secret.");
    }
    // The refresh-token cookie flow needs credentialed cross-origin requests
    // to work (desktop's Yerel/local mode calls this cloud server's
    // /api/auth/* from the 127.0.0.1 origin, Android's Capacitor webview from
    // https://localhost -- see client/src/utils/cloud.ts authUrl()/
    // isLocalOrigin()). The CORS spec forbids combining a wildcard origin
    // with credentials, so this used to mirror the request's own Origin back
    // verbatim -- functionally equivalent to `Access-Control-Allow-Origin: *`
    // combined with `Allow-Credentials: true`, accepting *any* origin's
    // credentialed request rather than just the desktop/Android bridge cases
    // it exists for. Replaced with an explicit allowlist: the production
    // origin, plus the two concrete local-bridge origins the native shells
    // actually present (any port, since Tauri's bundled sidecar and a local
    // dev server both pick a dynamic one).
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(|origin, _request_parts| origin.to_str().map(is_allowed_origin).unwrap_or(false)))
        .allow_credentials(true)
        .allow_methods(tower_http::cors::AllowMethods::mirror_request())
        .allow_headers(tower_http::cors::AllowHeaders::mirror_request());

    let mut app = Router::new()
        .route("/api/health", axum::routing::get(health))
        .route("/api/system/status", axum::routing::get(system_status))
        .route("/ws", get(ws::ws_handler))
        .nest("/api/auth", Router::new()
            .route("/register", post(auth::register))
            .route("/login", post(auth::login))
            .route("/refresh", post(auth::refresh))
            .route("/logout", post(auth::logout))
            .route("/me", get(auth::me))
            .route("/wizard-defaults", get(auth::get_wizard_defaults_route).post(auth::set_wizard_defaults_route))
            .route("/pending-status/:userCode", get(auth::pending_status)))
        .nest("/api/admin", Router::new()
            .route("/users", get(admin::list_users))
            .route("/users/:id/approve", post(admin::approve_user))
            .route("/users/:id/reject", post(admin::reject_user))
            .route("/users/:id/ban", post(admin::ban_user))
            .route("/users/:id/unban", post(admin::unban_user))
            .route("/users/:id", axum::routing::delete(admin::delete_user_route))
            .route("/seed-admin", post(admin::seed_admin))
            .route("/cleanup-admin", post(admin::cleanup_admin))
            .route("/test-email", get(admin::test_email))
            .route("/review/:token", get(admin::review))
            // POST, not GET: these mutate state (approve/reject a
            // registration), and a GET-with-side-effects is vulnerable to
            // corporate email-security-scanner link prefetching silently
            // triggering it before the admin ever reads the email. The
            // review page above now renders these as form submissions
            // rather than plain links.
            .route("/quick-approve/:token", post(admin::quick_approve))
            .route("/quick-reject/:token", post(admin::quick_reject)))
        .nest("/api/analysis", Router::new()
            .route("/local", post(analysis::analyze_local))
            .route("/local/hypothesis", post(analysis::hypothesis_local))
            .route("/:simId", post(analysis::analyze))
            .route("/:simId/hypothesis", post(analysis::hypothesis)))
        .nest("/api/aria", Router::new()
            .route("/command", post(aria::command))
            .route("/speak", post(aria::speak))
            .route("/inner-voice", post(aria::inner_voice)))
        .nest("/api/god", Router::new()
            .route("/:simId/intervene", post(god::intervene))
            .route("/:simId/quarantine", post(god::quarantine))
            .route("/:simId/talk/:individualId", post(god::talk))
            .route("/:simId/migrate-individual", post(god::migrate_individual)))
        .nest("/api/updates", Router::new()
            .route("/android/latest", get(releases::android_latest))
            .route("/android/asset/:id", get(releases::android_asset))
            .route("/desktop/latest.json", get(releases::desktop_latest_json))
            .route("/desktop/asset/:id", get(releases::desktop_asset)))
        .nest("/api/simulations", simulation_routes())
        .layer(cors)
        .with_state(state);

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let client_dist = [cwd.join("client/dist"), cwd.join("../client/dist")]
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| cwd.join("client/dist"));
    if client_dist.exists() {
        let index_file = client_dist.join("index.html");
        app = app.fallback_service(
            ServeDir::new(client_dist).not_found_service(ServeFile::new(index_file))
        );
    }
    // Must wrap the router *after* the fallback static service above, not
    // before -- a layer only covers whatever routes/services already exist
    // on the Router at the point .layer() is called, so applying it earlier
    // would leave the fallback_service (i.e. the actual HTML document real
    // browsers load) without these headers, defeating the whole point.
    app = app.layer(middleware::from_fn(cross_origin_isolation_headers));

    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(3002);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("sim-server listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CORS allowlist ───────────────────────────────────────────────────

    #[test]
    fn the_production_origin_is_allowed() {
        assert!(is_allowed_origin("https://anatolia-bold-sim.fly.dev"));
    }

    #[test]
    fn desktops_tauri_sidecar_origin_is_allowed_on_any_port() {
        assert!(is_allowed_origin("http://127.0.0.1:54321"));
        assert!(is_allowed_origin("http://127.0.0.1:1"));
    }

    #[test]
    fn androids_capacitor_webview_origin_is_allowed() {
        assert!(is_allowed_origin("https://localhost"));
        assert!(is_allowed_origin("http://localhost:5173"));
    }

    #[test]
    fn an_arbitrary_attacker_controlled_origin_is_rejected() {
        assert!(!is_allowed_origin("https://evil.example.com"));
        assert!(!is_allowed_origin("http://anatolia-bold-sim.fly.dev.evil.com"));
        assert!(!is_allowed_origin("null"));
    }

    #[test]
    fn a_lookalike_origin_that_merely_contains_the_allowed_host_is_rejected() {
        // Regression guard: must be an exact scheme+host (+ optional
        // ":<port>") match, not a substring/prefix check that a
        // similarly-named attacker domain could slip through.
        assert!(!is_allowed_origin("https://anatolia-bold-sim.fly.dev.attacker.io"));
        assert!(!is_allowed_origin("https://not-127.0.0.1.attacker.io"));
    }

    // ── cross-origin isolation headers ───────────────────────────────────
    // Regression guard for the WASM-local multi-threading feature (see
    // rust/sim-wasm's wasm-bindgen-rayon dependency): without these two
    // headers on every response, `SharedArrayBuffer` never becomes available
    // to the client and initThreadPool silently falls back to 1 thread.

    #[tokio::test]
    async fn every_response_carries_cross_origin_isolation_headers() {
        use tower::ServiceExt;

        let app = Router::new()
            .route("/probe", get(|| async { "ok" }))
            .layer(middleware::from_fn(cross_origin_isolation_headers));

        let response = app
            .oneshot(Request::builder().uri("/probe").body(axum::body::Body::empty()).unwrap())
            .await
            .expect("response");

        assert_eq!(response.headers().get("Cross-Origin-Opener-Policy").unwrap(), "same-origin");
        // require-corp, not credentialless -- see cross_origin_isolation_headers'
        // own doc comment on why (no cross-origin subresources left to break,
        // and require-corp is the one iOS WebKit reliably honors).
        assert_eq!(response.headers().get("Cross-Origin-Embedder-Policy").unwrap(), "require-corp");
    }
}
