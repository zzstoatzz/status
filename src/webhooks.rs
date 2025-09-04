use crate::db::{StatusFromDb, WebhookConfig};
// actix header types not needed here
use async_sqlite::Pool;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Serialize;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize)]
pub struct WebhookEvent<'a> {
    pub r#type: &'a str,
    pub user_did: &'a str,
    pub emoji: Option<&'a str>,
    pub text: Option<&'a str>,
    pub expires_at: Option<String>,
    pub status_uri: Option<&'a str>,
    pub timestamp: String,
    pub event_id: String,
    pub schema: &'a str,
}

pub async fn send_status_event(
    pool: &Pool,
    user_did: &str,
    event_type: &str,
    status: Option<&StatusFromDb>,
) -> bool {
    // Load config
    let config = match WebhookConfig::get_by_did(pool, user_did.to_string()).await {
        Ok(Some(c)) if c.enabled => c,
        _ => return false,
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let event_id = Uuid::new_v4().to_string();

    let (emoji, text, expires_at, status_uri) = match status {
        Some(s) => (
            Some(s.status.as_str()),
            s.text.as_deref(),
            s.expires_at.map(|e| e.to_rfc3339()),
            Some(s.uri.as_str()),
        ),
        None => (None, None, None, None),
    };

    let payload = WebhookEvent {
        r#type: event_type,
        user_did,
        emoji,
        text,
        expires_at,
        status_uri,
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_id: event_id.clone(),
        schema: "status-webhook.v1",
    };

    let body = match serde_json::to_string(&payload) {
        Ok(b) => b,
        Err(_) => return false,
    };

    // Compute signature: v1=hex(hmac(secret, ts + "." + body))
    let ts = now.to_string();
    let base_string = format!("{}.{}", ts, body);
    let mac_opt = HmacSha256::new_from_slice(config.secret.as_bytes()).ok();
    if mac_opt.is_none() {
        return false;
    }
    let mut mac = mac_opt.unwrap();
    mac.update(base_string.as_bytes());
    let sig_bytes = mac.finalize().into_bytes();
    let sig_hex = hex::encode(sig_bytes);
    let signature = format!("v1={}", sig_hex);

    // Send HTTP POST
    let client = Client::new();
    match client
        .post(config.url)
        .header("x-status-timestamp", &ts)
        .header("x-status-event-id", &event_id)
        .header("x-status-signature", &signature)
        .header("idempotency-key", &event_id)
        .body(body)
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

pub async fn send_status_event_direct(
    url: &str,
    secret: &str,
    user_did: &str,
    event_type: &str,
    status: Option<&StatusFromDb>,
) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let event_id = Uuid::new_v4().to_string();

    let (emoji, text, expires_at, status_uri) = match status {
        Some(s) => (
            Some(s.status.as_str()),
            s.text.as_deref(),
            s.expires_at.map(|e| e.to_rfc3339()),
            Some(s.uri.as_str()),
        ),
        None => (None, None, None, None),
    };

    let payload = WebhookEvent {
        r#type: event_type,
        user_did,
        emoji,
        text,
        expires_at,
        status_uri,
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_id: event_id.clone(),
        schema: "status-webhook.v1",
    };

    let body = match serde_json::to_string(&payload) {
        Ok(b) => b,
        Err(_) => return false,
    };

    let ts = now.to_string();
    let base_string = format!("{}.{}", ts, body);
    let mac_opt = HmacSha256::new_from_slice(secret.as_bytes()).ok();
    if mac_opt.is_none() {
        return false;
    }
    let mut mac = mac_opt.unwrap();
    mac.update(base_string.as_bytes());
    let sig_bytes = mac.finalize().into_bytes();
    let sig_hex = hex::encode(sig_bytes);
    let signature = format!("v1={}", sig_hex);

    let client = Client::new();
    match client
        .post(url.to_string())
        .header("x-status-timestamp", &ts)
        .header("x-status-event-id", &event_id)
        .header("x-status-signature", &signature)
        .header("idempotency-key", &event_id)
        .body(body)
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
