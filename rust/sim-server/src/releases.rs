//! Proxies GitHub Releases (metadata + binary assets) for the Android and
//! desktop update checkers.
//!
//! Both `client/src/utils/androidUpdate.ts` and the desktop Tauri updater
//! (see `desktop/src-tauri/tauri.conf.json`) used to hit `github.com`/
//! `api.github.com` directly and unauthenticated -- which only works while
//! the repo stays *public*. Routing both checks through this server instead
//! means the repo can go private without breaking in-app update checks:
//! this server (not the end user's device) holds the one token
//! (`GITHUB_RELEASES_TOKEN`) needed to read a private repo's releases, and
//! re-serves the same information/bytes it always served when the repo was
//! public. `GITHUB_RELEASES_TOKEN` is optional in theory -- every function
//! here degrades to the same unauthenticated GitHub request that worked
//! fine while the repo was public -- but the repo (`atabeyler/anatolia.bold.sim`)
//! is private now, so it is effectively required: unset or wrong, every
//! request here 404s (see fetch_latest_release's own comment on why a
//! private repo's failure mode is specifically a 404).
use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

// Repo was recreated under this name (see CLAUDE.md's git-identity notes) --
// this constant drifting out of sync with the real repo is exactly why
// in-app update checks (both Android and desktop, see fetch_latest_release
// below) started failing outright: GitHub 404s a releases/latest request
// for a repo name that no longer exists, which this module then surfaces
// as a 502 to the client.
const REPO: &str = "atabeyler/anatolia.bold.sim";

fn github_token() -> Option<String> {
    std::env::var("GITHUB_RELEASES_TOKEN").ok().filter(|t| !t.trim().is_empty())
}

fn extract_version(tag_name: &str) -> String {
    tag_name.trim_start_matches('v').to_string()
}

/// Finds the first asset whose name matches `predicate`, returning its
/// GitHub numeric asset id (needed by the `/releases/assets/{id}` download
/// endpoint -- the only one that works for a private repo's assets).
fn find_asset_id(assets: &[Value], predicate: impl Fn(&str) -> bool) -> Option<u64> {
    assets
        .iter()
        .find(|a| a.get("name").and_then(|n| n.as_str()).map(&predicate).unwrap_or(false))
        .and_then(|a| a.get("id").and_then(Value::as_u64))
}

/// Points the desktop updater manifest's windows binary at our own asset
/// proxy instead of the GitHub URL release.yml originally wrote into it --
/// same manifest shape Tauri's updater plugin expects, just one field
/// rewritten so the actual download also goes through this server.
fn rewrite_desktop_manifest(mut manifest: Value, base_url: &str, nsis_asset_id: u64) -> Value {
    if let Some(platform) = manifest.pointer_mut("/platforms/windows-x86_64").and_then(Value::as_object_mut) {
        platform.insert("url".to_string(), json!(format!("{base_url}/api/updates/desktop/asset/{nsis_asset_id}")));
    }
    manifest
}

async fn fetch_latest_release(client: &reqwest::Client) -> Result<Value, String> {
    let token = github_token();
    let mut req = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header(header::ACCEPT, "application/vnd.github+json")
        .header(header::USER_AGENT, "anatolia-sim-server");
    if let Some(token) = &token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let resp = req.send().await.map_err(|err| format!("fetch failed: {err}"))?;
    if !resp.status().is_success() {
        // A private repo returns 404 (not 401/403) to an unauthenticated or
        // badly-scoped request, indistinguishable from "repo doesn't exist"
        // in the status code alone -- naming whether a token was even sent
        // is the difference between "GITHUB_RELEASES_TOKEN isn't set" and
        // "it's set but wrong/expired/under-scoped" when this 404s.
        return Err(format!("github releases/latest returned {} (token_configured: {})", resp.status(), token.is_some()));
    }
    resp.json::<Value>().await.map_err(|err| format!("invalid JSON from github: {err}"))
}

async fn fetch_asset_bytes(client: &reqwest::Client, asset_id: u64) -> Result<(bytes::Bytes, String), String> {
    let token = github_token();
    let mut req = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/assets/{asset_id}"))
        .header(header::ACCEPT, "application/octet-stream")
        .header(header::USER_AGENT, "anatolia-sim-server");
    if let Some(token) = &token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let resp = req.send().await.map_err(|err| format!("fetch failed: {err}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "github releases/assets/{asset_id} returned {} (token_configured: {})",
            resp.status(),
            token.is_some()
        ));
    }
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = resp.bytes().await.map_err(|err| format!("read failed: {err}"))?;
    Ok((bytes, content_type))
}

fn bad_gateway(err: String) -> Response {
    (StatusCode::BAD_GATEWAY, Json(json!({ "error": err }))).into_response()
}

pub async fn android_latest() -> Response {
    let client = reqwest::Client::new();
    let release = match fetch_latest_release(&client).await {
        Ok(r) => r,
        Err(err) => return bad_gateway(err),
    };
    let tag = release.get("tag_name").and_then(Value::as_str).unwrap_or_default();
    let assets = release.get("assets").and_then(Value::as_array).cloned().unwrap_or_default();
    let Some(asset_id) = find_asset_id(&assets, |name| name.ends_with(".apk")) else {
        return bad_gateway("no-apk-asset".to_string());
    };
    Json(json!({
        "version": extract_version(tag),
        "download_url": format!("/api/updates/android/asset/{asset_id}"),
    }))
    .into_response()
}

pub async fn android_asset(Path(asset_id): Path<u64>) -> Response {
    let client = reqwest::Client::new();
    match fetch_asset_bytes(&client, asset_id).await {
        Ok((bytes, _content_type)) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/vnd.android.package-archive")
            .body(axum::body::Body::from(bytes))
            .unwrap(),
        Err(err) => bad_gateway(err),
    }
}

pub async fn desktop_latest_json() -> Response {
    let client = reqwest::Client::new();
    let release = match fetch_latest_release(&client).await {
        Ok(r) => r,
        Err(err) => return bad_gateway(err),
    };
    let assets = release.get("assets").and_then(Value::as_array).cloned().unwrap_or_default();
    let Some(manifest_asset_id) = find_asset_id(&assets, |name| name == "latest.json") else {
        return bad_gateway("no-latest.json-asset".to_string());
    };
    let Some(nsis_asset_id) = find_asset_id(&assets, |name| name.ends_with(".nsis.zip")) else {
        return bad_gateway("no-nsis-asset".to_string());
    };
    let manifest_bytes = match fetch_asset_bytes(&client, manifest_asset_id).await {
        Ok((bytes, _)) => bytes,
        Err(err) => return bad_gateway(err),
    };
    let manifest: Value = match serde_json::from_slice(&manifest_bytes) {
        Ok(v) => v,
        Err(err) => return bad_gateway(format!("bad latest.json: {err}")),
    };
    let rewritten = rewrite_desktop_manifest(manifest, &crate::auth::cloud_api_url(), nsis_asset_id);
    Json(rewritten).into_response()
}

pub async fn desktop_asset(Path(asset_id): Path<u64>) -> Response {
    let client = reqwest::Client::new();
    match fetch_asset_bytes(&client, asset_id).await {
        Ok((bytes, content_type)) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .body(axum::body::Body::from(bytes))
            .unwrap(),
        Err(err) => bad_gateway(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_strips_a_leading_v() {
        assert_eq!(extract_version("v2.3.101"), "2.3.101");
        assert_eq!(extract_version("2.3.101"), "2.3.101");
    }

    #[test]
    fn find_asset_id_matches_by_name_predicate() {
        let assets = vec![
            json!({ "id": 111, "name": "anatolia-sim_2.3.101_x64.nsis.zip" }),
            json!({ "id": 222, "name": "latest.json" }),
            json!({ "id": 333, "name": "anatolia-sim-2.3.101.apk" }),
        ];
        assert_eq!(find_asset_id(&assets, |n| n.ends_with(".apk")), Some(333));
        assert_eq!(find_asset_id(&assets, |n| n == "latest.json"), Some(222));
        assert_eq!(find_asset_id(&assets, |n| n.ends_with(".nsis.zip")), Some(111));
    }

    #[test]
    fn find_asset_id_returns_none_when_nothing_matches() {
        let assets = vec![json!({ "id": 1, "name": "readme.txt" })];
        assert_eq!(find_asset_id(&assets, |n| n.ends_with(".apk")), None);
    }

    #[test]
    fn rewrite_desktop_manifest_replaces_only_the_windows_url() {
        let manifest = json!({
            "version": "v2.3.101",
            "notes": "Anatolia Sim v2.3.101",
            "pub_date": "2026-07-15T00:00:00Z",
            "platforms": {
                "windows-x86_64": {
                    "signature": "abc123",
                    "url": "https://github.com/atabeyler/anatolia-sim/releases/download/v2.3.101/anatolia-sim_2.3.101_x64.nsis.zip",
                },
            },
        });
        let rewritten = rewrite_desktop_manifest(manifest, "https://anatolia-bold-sim.fly.dev", 111);
        assert_eq!(
            rewritten.pointer("/platforms/windows-x86_64/url").and_then(Value::as_str),
            Some("https://anatolia-bold-sim.fly.dev/api/updates/desktop/asset/111")
        );
        // The signature (and everything else Tauri's updater plugin
        // verifies against) must survive untouched -- only the url changes.
        assert_eq!(rewritten.pointer("/platforms/windows-x86_64/signature").and_then(Value::as_str), Some("abc123"));
        assert_eq!(rewritten.get("version").and_then(Value::as_str), Some("v2.3.101"));
    }
}
