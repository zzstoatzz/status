use async_sqlite::Pool;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Serialize;
use sha2::Sha256;

use crate::db::{Webhook, get_user_webhooks};

#[derive(Serialize)]
pub struct StatusEvent<'a> {
    pub event: &'a str, // "status.created" | "status.deleted" | "status.cleared"
    pub did: &'a str,
    pub handle: Option<&'a str>,
    pub status: Option<&'a str>,
    pub text: Option<&'a str>,
    pub uri: Option<&'a str>,
    pub since: Option<&'a str>,
    pub expires: Option<&'a str>,
}

fn should_send(h: &Webhook, event: &str) -> bool {
    if !h.active {
        return false;
    }
    let events = h.events.trim();
    if events == "*" || events.is_empty() {
        return true;
    }
    events
        .split(',')
        .map(|e| e.trim())
        .any(|e| e.eq_ignore_ascii_case(event))
}

pub async fn send_status_event(pool: std::sync::Arc<Pool>, did: &str, event: StatusEvent<'_>) {
    let client = Client::new();
    let hooks = match get_user_webhooks(&pool, did).await {
        Ok(h) => h,
        Err(e) => {
            log::error!("webhooks: failed to load webhooks for {}: {}", did, e);
            return;
        }
    };
    let payload = match serde_json::to_vec(&event) {
        Ok(p) => p,
        Err(e) => {
            log::error!("webhooks: failed to serialize payload: {}", e);
            return;
        }
    };
    let ts = chrono::Utc::now().timestamp().to_string();

    for h in hooks.into_iter().filter(|h| should_send(h, event.event)) {
        let mut mac = Hmac::<Sha256>::new_from_slice(h.secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(ts.as_bytes());
        mac.update(b".");
        mac.update(&payload);
        let sig = hex::encode(mac.finalize().into_bytes());

        let res = client
            .post(&h.url)
            .header("User-Agent", "status-webhooks/1.0")
            .header("Content-Type", "application/json")
            .header("X-Status-Webhook-Timestamp", &ts)
            .header("X-Status-Webhook-Signature", format!("sha256={}", sig))
            .body(payload.clone())
            .send()
            .await;

        match res {
            Ok(resp) => {
                if !resp.status().is_success() {
                    log::warn!(
                        "webhook delivery failed: {} -> status {}",
                        &h.url,
                        resp.status()
                    );
                }
            }
            Err(e) => log::warn!("webhook delivery error to {}: {}", &h.url, e),
        }
    }
}
