// Registration-approval email notifications, ported from the pre-migration
// Node.js backend (server/src/utils/email.js, since deleted). That version
// supported Resend or raw SMTP; this only implements Resend (the transport
// actually configured for this deployment) since adding a full SMTP stack
// (e.g. `lettre`) isn't justified without a concrete need for it.
use serde_json::json;

fn admin_email() -> String {
    std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "info@boldkimya.com.tr".to_string())
}

fn app_url() -> String {
    std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

pub(crate) fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

async fn deliver(to: &str, subject: &str, text: &str, html: Option<&str>) {
    let Ok(api_key) = std::env::var("RESEND_API_KEY") else {
        tracing::warn!(%subject, %to, "RESEND_API_KEY not set -- email not sent");
        return;
    };
    let from = std::env::var("RESEND_FROM").unwrap_or_else(|_| "ANATOLİA-SİM <onboarding@resend.dev>".to_string());
    let mut body = json!({
        "from": from,
        "to": [to],
        "subject": subject,
        "text": text,
    });
    if let Some(html) = html {
        body["html"] = json!(html);
    }
    let client = reqwest::Client::new();
    match client
        .post("https://api.resend.com/emails")
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!(%subject, %to, "email sent via Resend");
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(%subject, %to, %status, %body, "Resend send failed");
        }
        Err(err) => {
            tracing::warn!(%subject, %to, error = %err, "Resend request failed");
        }
    }
}

pub struct RegistrationInfo<'a> {
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub tc_no: &'a str,
    pub email: &'a str,
    pub user_code: &'a str,
    pub approval_token: &'a str,
}

pub async fn send_admin_registration_notification(info: RegistrationInfo<'_>) {
    let review_link = format!("{}/api/admin/review/{}", app_url(), info.approval_token);
    let text = format!(
        "New registration request - ANATOLIA-SIM\n\nFull Name: {} {}\nID No: {}\nEmail: {}\nUser Code: {}\n\nReview: {}",
        info.first_name, info.last_name, info.tc_no, info.email, info.user_code, review_link
    );
    let html = format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"></head>
<body style="background:#0a0a1e;color:#c8d4f0;font-family:'Courier New',monospace;padding:32px;max-width:480px;margin:0 auto">
<h2 style="color:#4f6ef7;font-size:12px;letter-spacing:.3em;margin-bottom:20px">ANATOLİA-SİM &mdash; NEW REGISTRATION REQUEST</h2>
<table style="width:100%;border-collapse:collapse;margin-bottom:24px;font-size:14px">
  <tr><td style="color:#6070a0;padding:6px 0;width:140px">Full Name</td><td style="color:#e0e8ff;font-weight:bold">{} {}</td></tr>
  <tr><td style="color:#6070a0;padding:6px 0">ID No</td><td style="color:#e0e8ff">{}</td></tr>
  <tr><td style="color:#6070a0;padding:6px 0">Email</td><td style="color:#e0e8ff">{}</td></tr>
  <tr><td style="color:#6070a0;padding:6px 0">User Code</td><td style="color:#e0e8ff;font-weight:bold">{}</td></tr>
</table>
<a href="{}" style="display:inline-block;padding:14px 32px;background:rgba(79,110,247,0.25);border:1px solid rgba(79,110,247,0.6);color:#a0b4ff;text-decoration:none;font-size:13px;letter-spacing:.15em">REVIEW &amp; DECIDE</a>
<p style="margin-top:24px;font-size:11px;color:#404060">Link valid for 7 days &middot; Bold Askeri Teknoloji ve Savunma Sanayii A.S. &middot; RST Q-Nation 200120401018</p>
</body></html>"#,
        escape_html(info.first_name),
        escape_html(info.last_name),
        escape_html(info.tc_no),
        escape_html(info.email),
        escape_html(info.user_code),
        escape_html(&review_link),
    );
    deliver(&admin_email(), &format!("[REGISTRATION REQUEST] {} {}", info.first_name, info.last_name), &text, Some(&html)).await;
}

pub async fn send_approval_email(first_name: &str, last_name: &str, email: &str, user_code: &str) {
    let text = format!(
        "Dear {first_name} {last_name},\n\nYour ANATOLIA-SIM registration has been approved.\n\nYour User Code: {user_code}\nLog in at: {}/login\n\nBold Askeri Teknoloji ve Savunma Sanayii A.S.",
        app_url()
    );
    deliver(email, "ANATOLİA-SİM - Your Registration Has Been Approved", &text, None).await;
}

pub async fn send_rejection_email(first_name: &str, last_name: &str, email: &str) {
    let text = format!(
        "Dear {first_name} {last_name},\n\nYour registration has been rejected by management.\nFor questions, you can write to {}.\n\nBold Askeri Teknoloji ve Savunma Sanayii A.S.",
        admin_email()
    );
    deliver(email, "ANATOLİA-SİM - Registration Request Rejected", &text, None).await;
}

pub async fn send_test_email() {
    let text = format!("Email system is working.\nDate: {}", chrono::Utc::now().to_rfc3339());
    deliver(&admin_email(), "ANATOLİA-SİM - Test", &text, None).await;
}
