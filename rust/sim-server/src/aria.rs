use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    analysis::lang_name,
    auth::authenticate,
    db::AppState,
    gemini::{self, GeminiRequest},
};

#[derive(Debug, Deserialize)]
pub struct CommandPayload {
    pub message: String,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub stats: Option<Value>,
    #[serde(default)]
    pub events: Option<Vec<Value>>,
    #[serde(default)]
    pub context: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct SpeakPayload {
    pub text: String,
    #[serde(default)]
    pub lang: Option<String>,
}

/// Splits into whitespace/punctuation-delimited tokens so keyword matching
/// below can require a whole-token match for short keywords -- a raw
/// substring search over the whole message let "sel" (flood) fire on
/// "Selam" ("hi" in Turkish), which really did trigger a live flood
/// intervention on the user's simulation just from a greeting.
fn tokenize(message: &str) -> Vec<String> {
    message.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect()
}

/// Short keywords (<=4 chars) must match a whole token exactly, since a
/// substring-anywhere check on something that short collides with unrelated
/// words too easily (as "sel" did). Longer keywords keep matching anywhere
/// within a token, so Turkish suffixed forms ("depremi", "başlatın") and
/// English inflections ("earthquake" containing "quake") still work.
fn has_keyword(tokens: &[String], keyword: &str) -> bool {
    if keyword.chars().count() <= 4 {
        tokens.iter().any(|t| t == keyword)
    } else {
        tokens.iter().any(|t| t.contains(keyword))
    }
}

fn classify_command(message: &str) -> Vec<Value> {
    let tokens = tokenize(message);
    let has = |kw: &str| has_keyword(&tokens, kw);
    let mut actions = Vec::new();
    if has("analysis") || has("analiz") {
        actions.push(json!({"type": "navigate_panel", "panel": "analysis"}));
    }
    if has("god") || has("tanrı") {
        actions.push(json!({"type": "navigate_panel", "panel": "god"}));
    }
    if has("population") || has("nüfus") {
        actions.push(json!({"type": "navigate_panel", "panel": "population"}));
    }
    if has("start") || has("başlat") {
        actions.push(json!({"type": "start_simulation"}));
    }
    if has("pause") || has("duraklat") {
        actions.push(json!({"type": "pause_simulation"}));
    }
    if has("speed") || has("hız") {
        actions.push(json!({"type": "change_speed", "speed": 5}));
    }
    if has("quake") || has("earthquake") || has("deprem") {
        actions.push(json!({"type": "apply_disaster", "disaster": "earthquake", "params": {"magnitude": 7, "radius": 200}}));
    }
    if has("flood") || has("sel") {
        actions.push(json!({"type": "apply_disaster", "disaster": "flood", "params": {"severity": 0.7, "radius": 200}}));
    }
    actions
}

/// Every action type the client's `executeAction` switch (AriaButton.tsx)
/// actually knows how to run. Gemini picks freely from these based on the
/// natural-language command; anything outside this list is dropped rather
/// than forwarded to the client, since the model can hallucinate a type.
const ARIA_ALLOWED_ACTIONS: &[&str] = &[
    "navigate_panel",
    "open_panel",
    "close_panel",
    "change_speed",
    "set_speed",
    "toggle_simulation",
    "start_simulation",
    "pause_simulation",
    "apply_disaster",
    "navigate_to",
    "set_tab",
    "toggle_lang",
    "set_lang",
    "god_mode",
    "toggle_sidebar",
    "terminate_simulation",
    "open_menu",
    "open_menu_page",
    "close_menu",
    "logout",
    "create_simulation",
    "open_simulation",
    "toggle_compare",
    "delete_simulation",
    "wizard_next",
    "wizard_back",
    "wizard_submit",
    "wizard_exit",
    "wizard_set",
];

/// Builds the action grammar Gemini is allowed to choose from for the
/// dashboard's *current* screen -- ported from AriaButton.tsx's own
/// (client-side, never actually wired up) `buildPrompt()`, which already
/// enumerated every action the client can execute per screen. Restricting
/// the grammar to what's actually reachable from the current page keeps
/// Gemini from e.g. offering wizard_next while the user is on the dashboard.
fn build_aria_system_prompt(context: Option<&Value>, lang: &str) -> String {
    let page = context.and_then(|c| c.get("page")).and_then(Value::as_str).unwrap_or("/");
    let is_wizard = context.and_then(|c| c.get("wizardOpen")).and_then(Value::as_bool).unwrap_or(false);
    let is_sim = page.starts_with("/simulation/");
    let is_dash = page == "/" && !is_wizard;

    let mut grammar = String::new();
    if is_wizard {
        grammar.push_str(
            "WIZARD commands (founder-creation screen): wizard_next | wizard_back | wizard_submit | wizard_exit | \
             wizard_set{\"field\":F,\"value\":V,\"founder\":1|2}. Fields: sim_name, sim_latitude, sim_longitude, \
             founder_name, founder_age, founder_sex(male/female), founder_height, founder_weight, \
             founder_eye(brown/hazel/green/blue), founder_hair(black/dark/brown/light/blond/red), \
             founder_skin(fair/light/olive/tan/brown/dark), current_trait(0-100). Turkish trait names map to: \
             zeka=fluid_intelligence merak=curiosity dil=language_capacity öğrenme=learning_rate \
             disiplin=conscientiousness stres=stress_resilience risk=risk_tolerance inovasyon=innovation \
             sanat=artistic_sense empati=empathy işbirliği=cooperation liderlik=dominance güç=physical_strength \
             dayanıklılık=endurance bağışıklık=immune_strength üreme=fertility uzun_ömür=longevity.\n",
        );
    } else if is_sim {
        grammar.push_str(
            "SIMULATION commands: navigate_panel{\"panel\":ID} | close_panel | change_speed{\"speed\":N} | \
             start_simulation | pause_simulation | toggle_simulation | terminate_simulation | toggle_sidebar | \
             god_mode | set_tab{\"tab\":\"harita\"|\"durum\"} | apply_disaster{\"disaster\":\"earthquake\"|\"flood\"|\
             \"drought\"|\"epidemic\"|\"volcano\"|\"meteor\",\"params\":{}} | open_menu | close_menu | \
             open_menu_page{\"menuPage\":\"guide\"|\"about\"|\"mission\"|\"contact\"|\"language\"}. Panel IDs: \
             population, olaylar, language, timemachine, analysis, biology, god, psychology, environment, \
             technology, belief, social, economy, culture, art, astronomy, hypothesis, epigenetics, architecture, \
             law, microbiome, genealogy, moments.\n",
        );
    } else if is_dash {
        grammar.push_str(
            "DASHBOARD commands: create_simulation | open_simulation{\"index\":N} | delete_simulation{\"index\":N} | \
             toggle_compare | logout.\n",
        );
    }
    grammar.push_str("GLOBAL commands (always available): navigate_to{\"route\":\"/\"} | toggle_lang | set_lang{\"lang\":\"tr\"|\"en\"|\"de\"|\"fr\"|\"ar\"}.\n");

    let page_label = if is_wizard {
        "founder-creation wizard"
    } else if is_sim {
        "running simulation"
    } else if is_dash {
        "dashboard"
    } else {
        "login"
    };

    format!(
        "{primer}\n\nYou are ARIA, this app's voice/text command controller. The user is currently on the \
         {page_label} screen. Read their message and decide which of the actions below (zero or more) match what \
         they actually asked for -- never invent an action type outside this list, and never pick an action the \
         message doesn't support just to have something to do.\n{grammar}\nReply with ONLY a JSON object of the \
         exact shape {{\"text\": <a short spoken confirmation of the action(s) you picked, or if none matched, a \
         direct helpful answer to their question using the app context above -- in {lang}>, \"actions\": \
         [{{\"type\": <one of the action types above, plus any parameters it needs}}]}}. No markdown, no extra keys.",
        primer = gemini::APP_PRIMER,
    )
}

/// Parses ARIA's `{"text": ..., "actions": [...]}` JSON, dropping any action
/// whose `type` isn't in `ARIA_ALLOWED_ACTIONS` rather than failing the whole
/// response -- a single hallucinated action type shouldn't also throw away a
/// perfectly good reply and the other, valid actions alongside it.
fn parse_aria_response(raw: &str) -> Option<(String, Vec<Value>)> {
    let cleaned = gemini::strip_code_fence(raw);
    let value: Value = serde_json::from_str(cleaned).ok()?;
    let text = value.get("text")?.as_str()?.to_string();
    let actions = value
        .get("actions")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter(|a| a.get("type").and_then(Value::as_str).is_some_and(|t| ARIA_ALLOWED_ACTIONS.contains(&t)))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    Some((text, actions))
}

pub async fn command(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(payload): Json<CommandPayload>) -> impl IntoResponse {
    if authenticate(&state, &headers).await.is_none() {
        return Json(json!({"error": "Unauthorized"})).into_response();
    }
    let lang = lang_name(payload.lang.clone());
    // Fallback path only: a tiny fixed keyword set covering the most common
    // commands, used solely when Gemini is unavailable so basic control
    // still works without an API key configured.
    let fallback_actions = classify_command(&payload.message);
    let fallback_text = if fallback_actions.is_empty() { "Rust ARIA: command received." } else { "Rust ARIA: command parsed." };

    let system = build_aria_system_prompt(payload.context.as_ref(), lang);
    let mut user_prompt = format!("User message: \"{}\".", payload.message.trim());
    if let Some(context) = &payload.context {
        user_prompt.push_str(&format!(" Dashboard context: {context}."));
    }
    if let Some(stats) = &payload.stats {
        user_prompt.push_str(&format!(" Live stats: {stats}."));
    }
    if let Some(events) = &payload.events {
        if !events.is_empty() {
            user_prompt.push_str(&format!(" Recent events: {}.", json!(events)));
        }
    }

    let (text, actions) = match gemini::chat(GeminiRequest { system: &system, user: &user_prompt, max_tokens: 650, temperature: 0.4, json_response: true }).await {
        Ok(raw) => parse_aria_response(&raw).unwrap_or_else(|| {
            tracing::warn!(raw = %raw, "gemini aria response was not valid JSON, falling back to keyword router");
            (fallback_text.to_string(), fallback_actions.clone())
        }),
        Err(err) => {
            tracing::warn!(%err, "gemini aria command call failed, falling back to keyword router");
            (fallback_text.to_string(), fallback_actions.clone())
        }
    };

    Json(json!({ "text": text, "actions": actions, "retry_after": 0 })).into_response()
}

pub async fn speak(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(payload): Json<SpeakPayload>) -> impl IntoResponse {
    if authenticate(&state, &headers).await.is_none() {
        return Json(json!({"error": "Unauthorized"})).into_response();
    }
    let lang = lang_name(payload.lang.clone());
    let system = format!(
        "{}\n\nYou are ARIA, this dashboard's friendly voice assistant. Reply conversationally and \
         helpfully in 1-2 short sentences, grounded in the app context above. Respond only in {lang}.",
        gemini::APP_PRIMER
    );
    let text = match gemini::chat(GeminiRequest { system: &system, user: &payload.text, max_tokens: 550, temperature: 0.6, json_response: false }).await {
        Ok(text) => text,
        Err(err) => {
            tracing::warn!(%err, "gemini aria speak call failed, falling back to echo");
            format!("Rust ARIA speaks: {}", payload.text)
        }
    };
    Json(json!({"text": text})).into_response()
}

pub async fn inner_voice(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(payload): Json<SpeakPayload>) -> impl IntoResponse {
    if authenticate(&state, &headers).await.is_none() {
        return Json(json!({"error": "Unauthorized"})).into_response();
    }
    let lang = lang_name(payload.lang.clone());
    let system = format!(
        "You are the reflective inner voice of the person operating the Anatolia-Sim dashboard -- a \
         brief, introspective first-person thought, not an assistant's answer. One short sentence only. \
         Respond only in {lang}."
    );
    let text = match gemini::chat(GeminiRequest { system: &system, user: &payload.text, max_tokens: 450, temperature: 0.7, json_response: false }).await {
        Ok(text) => text,
        Err(err) => {
            tracing::warn!(%err, "gemini aria inner_voice call failed, falling back to echo");
            format!("Rust inner voice: {}", payload.text)
        }
    };
    Json(json!({"text": text})).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_command_recognises_english_and_turkish_keywords() {
        assert_eq!(classify_command("show population")[0]["type"], "navigate_panel");
        assert_eq!(classify_command("nüfusu göster")[0]["panel"], "population");
    }

    /// Regression test: "Selam" ("hi" in Turkish) contains "sel" ("flood") as
    /// a substring, so a plain greeting used to fire a real flood
    /// intervention on the user's live simulation. It must not trigger any
    /// action at all.
    #[test]
    fn a_turkish_greeting_does_not_trigger_a_flood_disaster() {
        assert!(classify_command("Selam nasılsın").is_empty());
        assert!(classify_command("selamlar!").is_empty());
    }

    #[test]
    fn the_standalone_word_sel_still_triggers_a_flood_disaster() {
        let actions = classify_command("sel felaketi başlat");
        assert!(actions.iter().any(|a| a["disaster"] == "flood"));
    }

    #[test]
    fn deprem_still_triggers_an_earthquake_with_turkish_suffixes() {
        let actions = classify_command("büyük bir deprem oldu");
        assert!(actions.iter().any(|a| a["disaster"] == "earthquake"));
    }

    #[test]
    fn parse_aria_response_reads_text_and_actions() {
        let raw = r#"{"text":"Starting it now.","actions":[{"type":"start_simulation"}]}"#;
        let (text, actions) = parse_aria_response(raw).unwrap();
        assert_eq!(text, "Starting it now.");
        assert_eq!(actions[0]["type"], "start_simulation");
    }

    #[test]
    fn parse_aria_response_strips_a_markdown_fence() {
        let raw = "```json\n{\"text\":\"ok\",\"actions\":[]}\n```";
        let (text, actions) = parse_aria_response(raw).unwrap();
        assert_eq!(text, "ok");
        assert!(actions.is_empty());
    }

    #[test]
    fn parse_aria_response_drops_a_hallucinated_action_type_but_keeps_the_reply() {
        let raw = r#"{"text":"Done.","actions":[{"type":"delete_everything"},{"type":"pause_simulation"}]}"#;
        let (text, actions) = parse_aria_response(raw).unwrap();
        assert_eq!(text, "Done.");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["type"], "pause_simulation");
    }

    #[test]
    fn parse_aria_response_rejects_malformed_json() {
        assert!(parse_aria_response("not json").is_none());
    }

    #[test]
    fn wizard_context_exposes_wizard_grammar_not_simulation_grammar() {
        let ctx = json!({"page": "/", "wizardOpen": true});
        let system = build_aria_system_prompt(Some(&ctx), "English");
        assert!(system.contains("wizard_set"));
        assert!(!system.contains("apply_disaster"));
    }

    #[test]
    fn simulation_context_exposes_disaster_and_panel_grammar() {
        let ctx = json!({"page": "/simulation/abc123", "wizardOpen": false});
        let system = build_aria_system_prompt(Some(&ctx), "English");
        assert!(system.contains("apply_disaster"));
        assert!(system.contains("navigate_panel"));
    }

    #[test]
    fn dashboard_context_exposes_dashboard_grammar() {
        let ctx = json!({"page": "/", "wizardOpen": false});
        let system = build_aria_system_prompt(Some(&ctx), "English");
        assert!(system.contains("create_simulation"));
    }

    #[test]
    fn global_commands_are_always_present_regardless_of_page() {
        let system = build_aria_system_prompt(None, "English");
        assert!(system.contains("toggle_lang"));
    }
}
