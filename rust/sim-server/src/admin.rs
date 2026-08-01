use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use bcrypt::hash;
use serde::Deserialize;
use serde_json::json;
use crate::{
    auth::{decode_approval_token, require_admin, public_user},
    db::{cleanup_simulation_data, delete_user, list_users as load_users, load_user_by_id, update_user_flag, AppState},
    email::escape_html,
};

#[derive(Debug, Deserialize)]
pub struct BanPayload {
    pub reason: Option<String>,
}

/// Plain `!=` on the seed token would let a network attacker recover it
/// byte-by-byte from response-timing differences (the comparison returns as
/// soon as the first mismatching byte is found). This is only ever called
/// once, at bootstrap, but the fix costs nothing -- no need to accept the
/// theoretical exposure just because the route is rarely hit.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

pub async fn list_users(State(state): State<AppState>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if !require_admin(&headers) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin permission required."}))).into_response();
    }
    match load_users(&state.backend).await {
        Ok(rows) => {
            let payload: Vec<_> = rows
                .into_iter()
                .map(|user| {
                    json!({
                        "id": user.id,
                        "user_code": user.user_code,
                        "first_name": user.first_name,
                        "last_name": user.last_name,
                        "tc_no": user.tc_no,
                        "email": user.email,
                        "role": user.role,
                        "is_approved": user.is_approved != 0,
                        "is_banned": user.is_banned != 0,
                        "ban_reason": user.ban_reason,
                        "created_at": user.created_at,
                        "updated_at": user.updated_at,
                        "email_verified": user.email_verified != 0,
                    })
                })
                .collect();
            Json(payload).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

pub async fn approve_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if !require_admin(&headers) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin permission required."}))).into_response();
    }
    match update_user_flag(&state.backend, &id, Some(true), Some(false), None, Some("user")).await {
        Ok(Some(user)) => {
            crate::email::send_approval_email(
                &user.first_name,
                &user.last_name,
                &user.email,
                user.user_code.as_deref().unwrap_or(""),
            )
            .await;
            Json(json!({"message": "User approved.", "user": public_user(&user)})).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "User not found."}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

pub async fn reject_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if !require_admin(&headers) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin permission required."}))).into_response();
    }
    let user = load_user_by_id(&state.backend, &id).await.ok().flatten();
    match delete_user(&state.backend, &id).await {
        Ok(true) => {
            if let Some(user) = user {
                crate::email::send_rejection_email(&user.first_name, &user.last_name, &user.email).await;
            }
            Json(json!({"message": "Request rejected."})).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "User not found or already approved."}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

pub async fn ban_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<BanPayload>,
) -> impl IntoResponse {
    if !require_admin(&headers) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin permission required."}))).into_response();
    }
    match update_user_flag(&state.backend, &id, None, Some(true), payload.reason.as_deref(), None).await {
        Ok(Some(_)) => Json(json!({"message": "User banned."})).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "User not found."}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

pub async fn unban_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if !require_admin(&headers) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin permission required."}))).into_response();
    }
    match update_user_flag(&state.backend, &id, None, Some(false), None, None).await {
        Ok(Some(_)) => Json(json!({"message": "Ban lifted."})).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "User not found."}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

pub async fn delete_user_route(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if !require_admin(&headers) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin permission required."}))).into_response();
    }
    match delete_user(&state.backend, &id).await {
        Ok(true) => Json(json!({"message": "User deleted."})).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "User not found."}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

pub async fn seed_admin(State(state): State<AppState>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    // Defense in depth alongside the constant-time comparison below: a
    // leaked-but-not-yet-rotated seed token could otherwise be probed at
    // whatever rate the network allows. There's exactly one admin seed
    // token for the whole deployment, so this is a single global window,
    // not keyed per-caller.
    if !state.rate_limiter.check("seed-admin", 5, std::time::Duration::from_secs(15 * 60)) {
        return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": "Too many attempts. Please try again later."}))).into_response();
    }
    let expected = std::env::var("ADMIN_SEED_TOKEN").unwrap_or_default();
    let provided = headers.get("x-seed-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    if expected.is_empty() || !constant_time_eq(provided, &expected) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin seed token required."}))).into_response();
    }
    let code = match std::env::var("ADMIN_USER_CODE") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_uppercase(),
        _ => return (StatusCode::BAD_REQUEST, Json(json!({"error": "ADMIN_USER_CODE, ADMIN_PASSWORD and ADMIN_EMAIL env vars are required."}))).into_response(),
    };
    let pass = match std::env::var("ADMIN_PASSWORD") {
        Ok(v) if !v.is_empty() => v,
        _ => return (StatusCode::BAD_REQUEST, Json(json!({"error": "ADMIN_USER_CODE, ADMIN_PASSWORD and ADMIN_EMAIL env vars are required."}))).into_response(),
    };
    let email = match std::env::var("ADMIN_EMAIL") {
        Ok(v) if !v.is_empty() => v,
        _ => return (StatusCode::BAD_REQUEST, Json(json!({"error": "ADMIN_USER_CODE, ADMIN_PASSWORD and ADMIN_EMAIL env vars are required."}))).into_response(),
    };

    let hash = match hash(pass, bcrypt::DEFAULT_COST) {
        Ok(v) => v,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    };
    let result = crate::db::create_or_update_user(
        &state.backend,
        &code,
        &email,
        "Admin",
        "Administrator",
        "00000000000",
        &hash,
        "admin",
        true,
    )
    .await;
    match result {
        Ok(Some(_)) => Json(json!({"message": "Admin created."})).into_response(),
        Ok(None) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create admin."}))).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create admin.", "detail": err.to_string()}))).into_response(),
    }
}

pub async fn cleanup_admin(State(state): State<AppState>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if !require_admin(&headers) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin permission required."}))).into_response();
    }
    match cleanup_simulation_data(&state.backend).await {
        Ok((checkpoints, events, dead)) => Json(json!({
            "message": "Cleanup completed.",
            "checkpoints_deleted": checkpoints,
            "events_deleted": events,
            "dead_individuals_deleted": dead,
            "errors": [],
        }))
        .into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": err.to_string()}))).into_response(),
    }
}

pub async fn test_email(State(_state): State<AppState>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    if !require_admin(&headers) {
        return (StatusCode::FORBIDDEN, Json(json!({"error": "Admin permission required."}))).into_response();
    }
    crate::email::send_test_email().await;
    Json(json!({"message": "Test email dispatched (check RESEND_API_KEY / server logs if it doesn't arrive)."})).into_response()
}

fn html_page(title: &str, body: &str) -> impl IntoResponse {
    let page = format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>{title}</title>
<style>body{{background:#0a0a1e;color:#c8d4f0;font-family:'Courier New',monospace;padding:48px;max-width:520px;margin:0 auto}}
h1{{color:#4f6ef7;font-size:16px;letter-spacing:.2em}}a{{color:#a0b4ff}}
.btn{{display:inline-block;padding:12px 28px;margin:8px 8px 0 0;text-decoration:none;font-size:13px;letter-spacing:.1em;border:1px solid rgba(79,110,247,0.6);cursor:pointer;font-family:inherit}}
form{{display:inline}}
.approve{{background:rgba(78,203,113,0.15);color:#4ecb71;border-color:rgba(78,203,113,0.6)}}
.reject{{background:rgba(224,90,90,0.15);color:#e05a5a;border-color:rgba(224,90,90,0.6)}}</style>
</head><body><h1>{title}</h1>{body}</body></html>"#
    );
    (
        [(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))],
        page,
    )
}

pub async fn review(State(state): State<AppState>, Path(token): Path<String>) -> impl IntoResponse {
    let Some(user_id) = decode_approval_token(&token) else {
        return html_page("Invalid or expired link", "<p>This approval link is invalid or has expired (7 days).</p>").into_response();
    };
    let Some(user) = load_user_by_id(&state.backend, &user_id).await.ok().flatten() else {
        return html_page("Not found", "<p>This registration request no longer exists (it may have already been decided).</p>").into_response();
    };
    if user.is_approved != 0 {
        return html_page("Already approved", &format!("<p>{} {} has already been approved.</p>", escape_html(&user.first_name), escape_html(&user.last_name))).into_response();
    }
    let body = format!(
        r#"<table style="width:100%;margin:20px 0;font-size:14px">
        <tr><td style="color:#6070a0;padding:6px 0;width:140px">Full Name</td><td>{} {}</td></tr>
        <tr><td style="color:#6070a0;padding:6px 0">Email</td><td>{}</td></tr>
        <tr><td style="color:#6070a0;padding:6px 0">User Code</td><td>{}</td></tr>
        </table>
        <form method="post" action="/api/admin/quick-approve/{token}"><button type="submit" class="btn approve">✓ APPROVE</button></form>
        <form method="post" action="/api/admin/quick-reject/{token}"><button type="submit" class="btn reject">✗ REJECT</button></form>"#,
        escape_html(&user.first_name), escape_html(&user.last_name), escape_html(&user.email), escape_html(user.user_code.as_deref().unwrap_or("-")),
    );
    html_page("Review Registration Request", &body).into_response()
}

pub async fn quick_approve(State(state): State<AppState>, Path(token): Path<String>) -> impl IntoResponse {
    let Some(user_id) = decode_approval_token(&token) else {
        return html_page("Invalid or expired link", "<p>This approval link is invalid or has expired.</p>").into_response();
    };
    match update_user_flag(&state.backend, &user_id, Some(true), Some(false), None, Some("user")).await {
        Ok(Some(user)) => {
            crate::email::send_approval_email(&user.first_name, &user.last_name, &user.email, user.user_code.as_deref().unwrap_or("")).await;
            html_page("Approved", &format!("<p>{} {} has been approved and notified by email.</p>", escape_html(&user.first_name), escape_html(&user.last_name))).into_response()
        }
        Ok(None) => html_page("Not found", "<p>This registration request no longer exists.</p>").into_response(),
        Err(err) => html_page("Error", &format!("<p>{}</p>", err)).into_response(),
    }
}

pub async fn quick_reject(State(state): State<AppState>, Path(token): Path<String>) -> impl IntoResponse {
    let Some(user_id) = decode_approval_token(&token) else {
        return html_page("Invalid or expired link", "<p>This approval link is invalid or has expired.</p>").into_response();
    };
    let user = load_user_by_id(&state.backend, &user_id).await.ok().flatten();
    match delete_user(&state.backend, &user_id).await {
        Ok(true) => {
            if let Some(user) = user {
                crate::email::send_rejection_email(&user.first_name, &user.last_name, &user.email).await;
                html_page("Rejected", &format!("<p>{} {} has been rejected and notified by email.</p>", escape_html(&user.first_name), escape_html(&user.last_name))).into_response()
            } else {
                html_page("Rejected", "<p>Registration request rejected.</p>").into_response()
            }
        }
        Ok(false) => html_page("Not found", "<p>This registration request no longer exists.</p>").into_response(),
        Err(err) => html_page("Error", &format!("<p>{}</p>", err)).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_strings_are_equal() {
        assert!(constant_time_eq("seed-token-123", "seed-token-123"));
    }

    #[test]
    fn different_content_same_length_is_not_equal() {
        assert!(!constant_time_eq("seed-token-123", "seed-token-124"));
    }

    #[test]
    fn different_length_is_not_equal() {
        assert!(!constant_time_eq("short", "much-longer-value"));
    }

    #[test]
    fn empty_strings_are_equal() {
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn empty_vs_nonempty_is_not_equal() {
        assert!(!constant_time_eq("", "x"));
    }
}
