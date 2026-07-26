//! Thin Gemini (`generativelanguage.googleapis.com`) REST client shared by
//! ARIA, BOLD AI analysis, hypothesis testing and God Mode's "speak to
//! individual" feature.
//!
//! `GEMINI_API_KEY` is optional: every caller in this crate must fall back to
//! its previous deterministic behaviour when it's unset or every retry here
//! fails, so the app stays fully functional (and tests stay hermetic, with
//! no outbound network calls) without a configured key.
use serde_json::{json, Value};
use std::time::Duration;

// "gemini-2.0-flash-lite"/"gemini-2.0-flash" (the pre-migration Node.js
// backend's old default, and this file's own former default) both come back
// with a hard "free_tier_requests limit: 0" from Google now -- confirmed via
// a direct curl against the same API key that those specific model names
// have lost free-tier quota entirely, while "gemini-flash-latest" (currently
// resolving to gemini-3.5-flash) works fine on the exact same key. This
// alias tracks Google's current model automatically instead of pinning to a
// dated snapshot that quietly loses quota. GEMINI_MODEL still overrides this.
const DEFAULT_MODEL: &str = "gemini-flash-latest";

/// Shared grounding context for every assistant-shaped Gemini call (ARIA,
/// BOLD analysis/hypothesis) so replies stay anchored to what this app
/// actually is instead of drifting into generic chatbot small talk when the
/// user's message alone doesn't give the model enough to go on.
pub const APP_PRIMER: &str = "Anatolia-Sim is a scientific agent-based civilization simulator. Two \
    DNA-engineered founders are placed in a world; every descendant's language, consciousness, \
    belief, technology, art, law and culture must emerge purely from genetic inheritance and \
    observational learning -- no individual besides the two founders is ever scripted. The \
    dashboard has panels for: population, biology/genome, epigenetics, psychology, language, \
    belief, culture, art, technology, architecture, law, astronomy, social groups, economy, \
    microbiome/disease, environment, genealogy, plus God Mode (trigger earthquakes/floods/drought/\
    epidemics/volcanoes/meteors, or grant a founder longevity), a Time Machine (checkpoint replay), \
    and this AI analysis/hypothesis-testing panel.";
const RETRYABLE_STATUS: &[u16] = &[429, 500, 502, 503, 504];
const MAX_ATTEMPTS: u32 = 3;

pub struct GeminiRequest<'a> {
    pub system: &'a str,
    pub user: &'a str,
    pub max_tokens: u32,
    pub temperature: f32,
    /// Ask Gemini to constrain output to valid JSON (used by hypothesis testing).
    pub json_response: bool,
}

/// Sends one prompt to Gemini, retrying transient failures up to
/// `MAX_ATTEMPTS` times with 1s/2s/4s backoff. Returns `Err` (never panics)
/// when `GEMINI_API_KEY` is unset, empty, or every attempt fails.
pub async fn chat(req: GeminiRequest<'_>) -> Result<String, String> {
    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    if api_key.trim().is_empty() {
        return Err("GEMINI_API_KEY not set".to_string());
    }
    let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}");

    let mut generation_config = json!({
        "maxOutputTokens": req.max_tokens,
        "temperature": req.temperature,
        // No `thinkingConfig` here on purpose. This used to hard-code
        // `thinkingBudget: 0` to disable "thinking" (gemini-flash-latest
        // silently burns a large chunk of maxOutputTokens on invisible
        // reasoning before it ever writes the visible reply, which cut
        // replies off mid-sentence once the budget ran out) -- but
        // gemini-flash-latest is a rolling alias, and it has since moved to
        // a model version (confirmed via direct API calls: `gemini-3.6-flash`)
        // that rejects `thinkingBudget: 0` outright with a 400
        // INVALID_ARGUMENT, which made every single call here fail and
        // silently fall back to the heuristic response, permanently, with
        // no visible error to the user. A nonzero budget (tried 1 and 100)
        // doesn't actually reduce the thinking-token spend on this model
        // version either, so there's no budget value left that both (a) is
        // accepted and (b) meaningfully caps cost -- omitting the field
        // entirely is the only option that survives the alias moving again.
        // Callers compensate by sizing `max_tokens` with headroom over the
        // observed ~100-110 token thinking floor instead.
    });
    if req.json_response {
        generation_config["responseMimeType"] = json!("application/json");
    }
    let body = json!({
        "system_instruction": { "parts": [{ "text": req.system }] },
        "contents": [{ "role": "user", "parts": [{ "text": req.user }] }],
        "generationConfig": generation_config,
    });

    let client = reqwest::Client::new();
    let mut last_err = String::new();
    for attempt in 0..MAX_ATTEMPTS {
        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return match resp.json::<Value>().await {
                        Ok(payload) => extract_text(&payload).ok_or_else(|| "Gemini response had no text".to_string()),
                        Err(err) => Err(format!("invalid JSON from Gemini: {err}")),
                    };
                }
                // The status alone ("Gemini HTTP 429") doesn't say *why* --
                // Google's error body carries the actual reason (quota
                // exhausted vs. no billing vs. bad API key vs. wrong model),
                // which is exactly what's needed to tell a real quota problem
                // apart from a misconfigured key without guessing.
                let body = resp.text().await.unwrap_or_default();
                let truncated: String = body.chars().take(500).collect();
                last_err = format!("Gemini HTTP {status}: {truncated}");
                if !RETRYABLE_STATUS.contains(&status.as_u16()) {
                    break;
                }
            }
            Err(err) => last_err = format!("Gemini request failed: {err}"),
        }
        if attempt + 1 < MAX_ATTEMPTS {
            tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
        }
    }
    Err(last_err)
}

fn extract_text(payload: &Value) -> Option<String> {
    payload
        .get("candidates")?
        .as_array()?
        .first()?
        .get("content")?
        .get("parts")?
        .as_array()?
        .first()?
        .get("text")?
        .as_str()
        .map(|s| s.trim().to_string())
}

/// Strips a ```json ... ``` / ``` ... ``` markdown fence if present, so
/// callers that ask for `json_response` can still `serde_json::from_str`
/// even when the model wraps its output in a code block anyway.
pub fn strip_code_fence(text: &str) -> &str {
    let trimmed = text.trim();
    let without_open = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    without_open.strip_suffix("```").unwrap_or(without_open).trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_text_reads_the_first_candidate_part() {
        let payload = json!({ "candidates": [{ "content": { "parts": [{ "text": "hello" }] } }] });
        assert_eq!(extract_text(&payload), Some("hello".to_string()));
    }

    #[test]
    fn extract_text_returns_none_on_empty_candidates() {
        assert_eq!(extract_text(&json!({ "candidates": [] })), None);
    }

    #[test]
    fn extract_text_returns_none_when_candidates_key_is_missing() {
        assert_eq!(extract_text(&json!({})), None);
    }

    #[test]
    fn strip_code_fence_removes_json_fence() {
        assert_eq!(strip_code_fence("```json\n{\"a\":1}\n```"), "{\"a\":1}");
    }

    #[test]
    fn strip_code_fence_removes_plain_fence() {
        assert_eq!(strip_code_fence("```\n{\"a\":1}\n```"), "{\"a\":1}");
    }

    #[test]
    fn strip_code_fence_leaves_unfenced_text_untouched() {
        assert_eq!(strip_code_fence("{\"a\":1}"), "{\"a\":1}");
    }

    #[tokio::test]
    async fn chat_fails_fast_without_an_api_key_and_makes_no_network_call() {
        std::env::remove_var("GEMINI_API_KEY");
        let err = chat(GeminiRequest { system: "s", user: "u", max_tokens: 10, temperature: 0.2, json_response: false })
            .await
            .unwrap_err();
        assert!(err.contains("GEMINI_API_KEY"));
    }

    #[tokio::test]
    async fn chat_fails_fast_when_the_api_key_is_blank() {
        std::env::set_var("GEMINI_API_KEY", "   ");
        let err = chat(GeminiRequest { system: "s", user: "u", max_tokens: 10, temperature: 0.2, json_response: false })
            .await
            .unwrap_err();
        std::env::remove_var("GEMINI_API_KEY");
        assert!(err.contains("GEMINI_API_KEY"));
    }
}
