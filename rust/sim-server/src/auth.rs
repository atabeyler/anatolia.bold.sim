use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::db::{create_or_update_user, get_wizard_defaults, load_user_by_code, load_user_by_id, set_wizard_defaults, DbBackend, UserRow, AppState};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
}

#[derive(Debug, Deserialize)]
pub struct RegisterPayload {
    pub first_name: String,
    pub last_name: String,
    pub tc_no: String,
    pub email: String,
    pub password: String,
    pub user_code: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub user_code: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct PublicUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub first_name: String,
    pub last_name: String,
}

pub fn public_user(user: &UserRow) -> PublicUser {
    PublicUser {
        id: user.id.clone(),
        username: user.user_code.clone().or_else(|| user.username.clone()).unwrap_or_default(),
        email: user.email.clone(),
        role: user.role.clone().unwrap_or_else(|| "pending".to_string()),
        first_name: user.first_name.clone(),
        last_name: user.last_name.clone(),
    }
}

pub fn access_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "anatolia-sim-local-access-secret".to_string())
}

pub fn refresh_secret() -> String {
    std::env::var("JWT_REFRESH_SECRET").unwrap_or_else(|_| "anatolia-sim-local-refresh-secret".to_string())
}

// Accounts live only in the cloud (Postgres) database -- the SQLite backend
// used by desktop's "Yerel" mode never has a users table worth trusting, so
// it must never mint or check its own logins. It borrows identity from the
// cloud instead (see `authenticate` below), which is safe because desktop
// is required to be online before it ever reaches this server.
pub fn is_local_backend(state: &AppState) -> bool {
    matches!(state.backend, DbBackend::Sqlite(_))
}

pub fn cloud_api_url() -> String {
    std::env::var("CLOUD_API_URL").unwrap_or_else(|_| "https://anatolia-bold-sim.fly.dev".to_string())
}

pub fn validate_password(password: &str) -> Option<&'static str> {
    if password.len() < 8 {
        return Some("Password must be at least 8 characters.");
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Some("Password must contain at least one uppercase letter.");
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Some("Password must contain at least one lowercase letter.");
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Some("Password must contain at least one digit.");
    }
    if !password.chars().any(|c| !c.is_ascii_alphanumeric()) {
        return Some("Password must contain at least one punctuation/special character.");
    }
    None
}

fn sign_access(user: &UserRow) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        id: user.id.clone(),
        username: user.user_code.clone().or_else(|| user.username.clone()).unwrap_or_default(),
        email: user.email.clone(),
        role: user.role.clone().unwrap_or_else(|| "pending".to_string()),
        exp: (Utc::now().timestamp() + 15 * 60) as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(access_secret().as_bytes()))
}

fn sign_refresh(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        id: user_id.to_string(),
        username: String::new(),
        email: String::new(),
        role: String::new(),
        exp: (Utc::now().timestamp() + 30 * 24 * 60 * 60) as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(refresh_secret().as_bytes()))
}

/// True only when the request's `Origin` genuinely names a different host
/// than the one it was sent to -- the desktop app's Yerel/local mode calling
/// this cloud API from the 127.0.0.1 sidecar (see CLOUD_API_URL/authUrl in
/// the client) is the one legitimate case. A same-origin browser tab hitting
/// its own `/api/auth/login` (the overwhelming majority of traffic: mobile/
/// desktop web visitors) still sends an `Origin` header on this POST, but it
/// matches `Host`, so this returns false for them.
fn is_cross_origin_request(headers: &axum::http::HeaderMap) -> bool {
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok());
    let origin_host = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .and_then(|o| o.split("://").nth(1));
    matches!((host, origin_host), (Some(host), Some(origin_host)) if origin_host != host)
}

/// SameSite=None is what Safari's Intelligent Tracking Prevention scrutinizes
/// hardest -- it's the flag a cookie uses to declare itself eligible for
/// cross-site use in the first place, and ITP is far more willing to evict or
/// reject it than a same-site Lax/Strict cookie, independent of whether the
/// traffic pattern is actually cross-site. Only the desktop bridge case above
/// genuinely needs None; every normal same-origin web visit (the vast
/// majority, including mobile Safari/Chrome) gets Lax, which Safari treats
/// as an ordinary first-party cookie with none of ITP's extra restrictions.
fn cookie_value(token: &str, headers: &axum::http::HeaderMap) -> String {
    let is_prod = std::env::var("NODE_ENV").map(|v| v == "production").unwrap_or(false) || std::env::var("RENDER").is_ok();
    let same_site = if !is_prod {
        "Strict"
    } else if is_cross_origin_request(headers) {
        "None"
    } else {
        "Lax"
    };
    format!(
        "refresh_token={}; Path=/; HttpOnly; SameSite={}; Max-Age={};{}",
        token,
        same_site,
        30 * 24 * 60 * 60,
        if is_prod { " Secure;" } else { "" },
    )
}

fn extract_cookie(req: &axum::http::HeaderMap, name: &str) -> Option<String> {
    let raw = req.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .map(|part| part.trim())
        .find_map(|part| part.strip_prefix(&format!("{name}=")).map(|v| v.to_string()))
}

pub fn decode_access_token(token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(access_secret().as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .ok()
    .map(|data| data.claims)
}

pub fn decode_refresh_token(token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(refresh_secret().as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .ok()
    .map(|data| data.claims)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApprovalClaims {
    pub user_id: String,
    pub purpose: String,
    pub exp: usize,
}

/// Signs a link a pending user's registration can be approved/rejected
/// through directly from the admin's email, without needing to be logged
/// in -- possession of this token (mailed only to ADMIN_EMAIL) is the
/// authorization. Reuses the refresh-token secret rather than adding a new
/// one; distinct from a login/refresh token by shape (decode::<ApprovalClaims>
/// requires `user_id`/`purpose`, which those don't have).
pub fn sign_approval_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = ApprovalClaims {
        user_id: user_id.to_string(),
        purpose: "registration_approval".to_string(),
        exp: (Utc::now().timestamp() + 7 * 24 * 60 * 60) as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(refresh_secret().as_bytes()))
}

pub fn decode_approval_token(token: &str) -> Option<String> {
    let claims = decode::<ApprovalClaims>(
        token,
        &DecodingKey::from_secret(refresh_secret().as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .ok()?
    .claims;
    if claims.purpose != "registration_approval" {
        return None;
    }
    Some(claims.user_id)
}

fn bearer_token(headers: &axum::http::HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(|s| s.to_string())
}

pub fn auth_user_from_headers(headers: &axum::http::HeaderMap) -> Option<Claims> {
    decode_access_token(&bearer_token(headers)?)
}

pub fn require_admin(headers: &axum::http::HeaderMap) -> bool {
    auth_user_from_headers(headers).map(|u| u.role == "admin").unwrap_or(false)
}

#[derive(Debug, Deserialize)]
struct MeResponse {
    id: String,
    username: String,
    email: String,
    role: String,
}

fn remote_claims_cache() -> &'static Mutex<HashMap<String, (Claims, Instant)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (Claims, Instant)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Verifies a bearer token against whichever backend this process is
/// running as. The cloud/Postgres backend holds the JWT signing secret and
/// decodes tokens itself; the desktop/SQLite backend has no accounts of its
/// own (see `is_local_backend`) and instead asks the cloud to vouch for the
/// token over HTTPS, since desktop is required to be online. A short-lived
/// in-memory cache avoids a network round trip on every local-mode request.
pub async fn authenticate(state: &AppState, headers: &axum::http::HeaderMap) -> Option<Claims> {
    let token = bearer_token(headers)?;
    authenticate_token(state, &token).await
}

/// Token-only half of `authenticate`, for callers that don't have an HTTP
/// header map to pull a bearer token from -- namely the WebSocket handler,
/// which receives the token as its first text message instead. Must stay
/// the single source of truth for "is this token good" so the local/SQLite
/// backend's cloud-vouching path (see `authenticate` above) can never be
/// bypassed by a caller reaching for `decode_access_token` directly.
pub async fn authenticate_token(state: &AppState, token: &str) -> Option<Claims> {
    let token = token.to_string();
    if !is_local_backend(state) {
        return decode_access_token(&token);
    }

    // The route test suite exercises the SQLite/local code path directly
    // (it's the fast, no-external-services backend), with no real cloud
    // deployment to call out to. Decode locally there too -- same secret
    // the test harness signs its tokens with -- instead of reaching out
    // over the network as the rest of this function does.
    if cfg!(test) {
        return decode_access_token(&token);
    }

    let cached = remote_claims_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(&token).cloned())
        .filter(|(_, seen)| seen.elapsed() < Duration::from_secs(60))
        .map(|(claims, _)| claims);
    if cached.is_some() {
        return cached;
    }

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/auth/me", cloud_api_url()))
        .bearer_auth(&token)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let me: MeResponse = resp.json().await.ok()?;
    let claims = Claims {
        id: me.id,
        username: me.username,
        email: me.email,
        role: me.role,
        exp: (Utc::now().timestamp() + 60) as usize,
    };
    if let Ok(mut cache) = remote_claims_cache().lock() {
        cache.insert(token, (claims.clone(), Instant::now()));
    }
    Some(claims)
}

/// Echoes the caller's own identity. This is what lets desktop's local
/// (SQLite) backend borrow the cloud's notion of "who is this" -- see
/// `authenticate` above -- so it must always decode with the local secret
/// regardless of which backend this particular process is running (calling
/// it against the local backend would be meaningless, since desktop never
/// signs its own tokens any more).
pub async fn me(State(state): State<AppState>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    let claims = match auth_user_from_headers(&headers) {
        Some(claims) => claims,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid session."}))).into_response(),
    };
    match load_user_by_id(&state.backend, &claims.id).await {
        Ok(Some(user)) => Json(public_user(&user)).into_response(),
        _ => (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid session."}))).into_response(),
    }
}

/// Last-used "create simulation" wizard values (name/lat/lon/founder
/// params) -- account-scoped so they survive iOS Safari's ITP wiping
/// localStorage after 7 days of no top-level visit, and follow the user
/// across devices/browsers. `defaults` is an opaque JSON blob the client
/// owns the shape of entirely; this just round-trips it.
pub async fn get_wizard_defaults_route(State(state): State<AppState>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    let claims = match auth_user_from_headers(&headers) {
        Some(claims) => claims,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid session."}))).into_response(),
    };
    match get_wizard_defaults(&state.backend, &claims.id).await {
        Ok(Some(raw)) => {
            let parsed: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
            Json(json!({ "defaults": parsed })).into_response()
        }
        Ok(None) => Json(json!({ "defaults": null })).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

pub async fn set_wizard_defaults_route(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(payload): Json<Value>) -> impl IntoResponse {
    let claims = match auth_user_from_headers(&headers) {
        Some(claims) => claims,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid session."}))).into_response(),
    };
    match set_wizard_defaults(&state.backend, &claims.id, &payload.to_string()).await {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

pub async fn register(State(state): State<AppState>, Json(payload): Json<RegisterPayload>) -> impl IntoResponse {
    // Checked before is_local_backend/field validation so it's a cheap,
    // backend-independent first line of defense. Unlike login (keyed per
    // user_code), an attacker spamming registrations controls every field in
    // the payload, so a per-key limit is trivially bypassed by varying the
    // user_code/email each time. A single global window (same reasoning as
    // admin::seed_admin's) is what actually bounds this: every registration
    // also fires an admin notification email, so unthrottled spam floods the
    // admin's inbox and grows the users table with no cap.
    if !state.rate_limiter.check("register", 20, Duration::from_secs(15 * 60)) {
        return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": "Too many registration attempts. Please try again later."}))).into_response();
    }

    if is_local_backend(&state) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Accounts are managed in the cloud. Please register while connected to the cloud."})),
        )
            .into_response();
    }

    if payload.first_name.trim().is_empty()
        || payload.last_name.trim().is_empty()
        || payload.tc_no.trim().is_empty()
        || payload.email.trim().is_empty()
        || payload.password.is_empty()
        || payload.user_code.trim().is_empty()
    {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "All fields are required."}))).into_response();
    }

    if !payload.tc_no.chars().all(|c| c.is_ascii_digit()) || payload.tc_no.len() != 11 {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "National ID must be an 11-digit number."}))).into_response();
    }

    let code = payload.user_code.trim().to_uppercase();
    if !(4..=20).contains(&code.len()) || !code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "User code must be 4-20 characters, letters and digits only."}))).into_response();
    }

    if let Some(err) = validate_password(&payload.password) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": err}))).into_response();
    }

    let hashed = match hash(&payload.password, DEFAULT_COST) {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    match create_or_update_user(
        &state.backend,
        &code,
        &payload.email.trim().to_lowercase(),
        payload.first_name.trim(),
        payload.last_name.trim(),
        payload.tc_no.trim(),
        &hashed,
        "pending",
        false,
    )
    .await
    {
        Ok(Some(user)) => {
            if let Ok(token) = sign_approval_token(&user.id) {
                crate::email::send_admin_registration_notification(crate::email::RegistrationInfo {
                    first_name: &user.first_name,
                    last_name: &user.last_name,
                    tc_no: user.tc_no.as_deref().unwrap_or(""),
                    email: &user.email,
                    user_code: user.user_code.as_deref().unwrap_or(&code),
                    approval_token: &token,
                })
                .await;
            }
            (StatusCode::CREATED, Json(json!({"message": "Your registration request has been received. Awaiting admin approval."}))).into_response()
        }
        Ok(None) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Registration failed."}))).into_response(),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("unique") {
                (StatusCode::CONFLICT, Json(json!({"error": "This email, national ID, or user code is already registered."}))).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Registration failed."}))).into_response()
            }
        }
    }
}

pub async fn login(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(payload): Json<LoginPayload>) -> impl IntoResponse {
    if is_local_backend(&state) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Accounts are managed in the cloud. Please sign in while connected to the cloud."})),
        )
            .into_response();
    }

    // Password hashing/comparison itself was already sound, but nothing
    // capped how many guesses per second an attacker could throw at a known
    // user_code. Keyed by user_code (not IP): the actual threat is
    // brute-forcing a specific account's password, which an attacker can
    // attempt from any/rotating IP, but not without the account's own code.
    let rate_key = payload.user_code.trim().to_uppercase();
    if !state.rate_limiter.check(&format!("login:{rate_key}"), 10, Duration::from_secs(15 * 60)) {
        return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": "Too many login attempts. Please try again later."}))).into_response();
    }

    let user = match load_user_by_code(&state.backend, &payload.user_code).await {
        Ok(Some(user)) => user,
        Ok(None) => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid user code or password."}))).into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    if !verify(&payload.password, &user.password_hash).unwrap_or(false) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid user code or password."}))).into_response();
    }
    if user.is_banned != 0 {
        let reason = user.ban_reason.clone().map(|r| format!(" Reason: {r}")).unwrap_or_default();
        return (StatusCode::FORBIDDEN, Json(json!({"error": format!("Your account has been banned.{reason}")}))).into_response();
    }
    if user.is_approved == 0 {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Your account has not been approved yet. Please wait for admin approval."}))).into_response();
    }

    let access_token = match sign_access(&user) {
        Ok(token) => token,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let refresh_token = match sign_refresh(&user.id) {
        Ok(token) => token,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };

    let user_public = public_user(&user);
    let mut response = Json(json!({
        "access_token": access_token,
        "user": user_public,
    }))
    .into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie_value(&refresh_token, &headers)) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

pub async fn refresh(State(state): State<AppState>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    let token = match extract_cookie(&headers, "refresh_token") {
        Some(token) => token,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Session expired."}))).into_response(),
    };
    let claims = match decode_refresh_token(&token) {
        Some(claims) => claims,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid session."}))).into_response(),
    };
    let user = match load_user_by_id(&state.backend, &claims.id).await {
        Ok(Some(user)) => user,
        _ => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid session."}))).into_response(),
    };
    if user.is_banned != 0 || user.is_approved == 0 {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid session."}))).into_response();
    }
    let access_token = match sign_access(&user) {
        Ok(token) => token,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    Json(json!({
        "access_token": access_token,
        "user": public_user(&user),
    }))
    .into_response()
}

pub async fn logout() -> impl IntoResponse {
    let mut response = Json(json!({"message": "Logged out."})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("refresh_token=; Path=/; HttpOnly; Max-Age=0; SameSite=Strict"),
    );
    response
}

pub async fn pending_status(State(state): State<AppState>, axum::extract::Path(user_code): axum::extract::Path<String>) -> impl IntoResponse {
    match load_user_by_code(&state.backend, &user_code).await {
        Ok(Some(user)) if user.is_banned != 0 => Json(json!({ "status": "banned" })).into_response(),
        Ok(Some(user)) if user.is_approved != 0 => Json(json!({ "status": "approved" })).into_response(),
        Ok(Some(_)) => Json(json!({ "status": "pending" })).into_response(),
        Ok(None) => Json(json!({ "status": "not_found" })).into_response(),
        Err(_) => Json(json!({ "status": "error" })).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use std::sync::Mutex;

    // dev_cookie_is_always_strict / prod_same_origin_cookie_is_lax_not_none /
    // prod_cross_origin_cookie_is_none all mutate the process-wide RENDER
    // env var to force cookie_value()'s prod/dev branch -- cargo test runs
    // them on separate threads by default, so without serializing, one
    // test's remove_var can land between another's set_var and its own
    // assertion (e.g. dev_cookie_is_always_strict clearing RENDER out from
    // under prod_cross_origin_cookie_is_none), flipping the SameSite value
    // it observes. Hold this for the env-var-set-read-clear span, not just
    // the read.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    fn headers(origin: Option<&str>, host: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Some(origin) = origin {
            h.insert(header::ORIGIN, HeaderValue::from_str(origin).unwrap());
        }
        if let Some(host) = host {
            h.insert(header::HOST, HeaderValue::from_str(host).unwrap());
        }
        h
    }

    // ── is_cross_origin_request() ───────────────────────────────────────

    #[test]
    fn same_origin_web_visit_is_not_cross_origin() {
        // The overwhelming majority of traffic: a browser tab on
        // anatolia-bold-sim.fly.dev calling its own /api/auth/login.
        let h = headers(Some("https://anatolia-bold-sim.fly.dev"), Some("anatolia-bold-sim.fly.dev"));
        assert!(!is_cross_origin_request(&h));
    }

    #[test]
    fn desktop_bridge_from_localhost_is_cross_origin() {
        // The one legitimate cross-origin case: desktop's Yerel/local mode
        // calling the cloud API from its 127.0.0.1 sidecar (see authUrl()
        // in the client).
        let h = headers(Some("http://127.0.0.1:1420"), Some("anatolia-bold-sim.fly.dev"));
        assert!(is_cross_origin_request(&h));
    }

    #[test]
    fn missing_origin_header_defaults_to_same_origin() {
        // Browsers omit Origin on some same-origin requests; without it we
        // have no evidence of cross-origin traffic, so don't assume it.
        let h = headers(None, Some("anatolia-bold-sim.fly.dev"));
        assert!(!is_cross_origin_request(&h));
    }

    #[test]
    fn scheme_difference_alone_is_not_cross_origin() {
        // Origin's scheme (http/https) is irrelevant to SameSite cookie
        // scoping -- only the host:port authority matters.
        let h = headers(Some("http://anatolia-bold-sim.fly.dev"), Some("anatolia-bold-sim.fly.dev"));
        assert!(!is_cross_origin_request(&h));
    }

    // ── cookie_value() SameSite selection ───────────────────────────────

    #[test]
    fn dev_cookie_is_always_strict() {
        let _guard = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("RENDER");
        std::env::remove_var("NODE_ENV");
        let h = headers(Some("http://127.0.0.1:1420"), Some("localhost:3002"));
        assert!(cookie_value("tok", &h).contains("SameSite=Strict"));
    }

    #[test]
    fn prod_same_origin_cookie_is_lax_not_none() {
        let _guard = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("RENDER", "1");
        let h = headers(Some("https://anatolia-bold-sim.fly.dev"), Some("anatolia-bold-sim.fly.dev"));
        let cookie = cookie_value("tok", &h);
        std::env::remove_var("RENDER");
        assert!(cookie.contains("SameSite=Lax"), "same-origin prod cookie should be Lax, got: {cookie}");
        assert!(cookie.contains("Secure"));
    }

    #[test]
    fn prod_cross_origin_cookie_is_none() {
        let _guard = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("RENDER", "1");
        let h = headers(Some("http://127.0.0.1:1420"), Some("anatolia-bold-sim.fly.dev"));
        let cookie = cookie_value("tok", &h);
        std::env::remove_var("RENDER");
        assert!(cookie.contains("SameSite=None"), "cross-origin prod cookie should be None, got: {cookie}");
    }
}
