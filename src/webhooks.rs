use async_sqlite::Pool;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Serialize;
use sha2::Sha256;

use crate::db::{StatusFromDb, Webhook, get_user_webhooks};
use futures_util::future;

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

fn hmac_sig_hex(secret: &str, ts: &str, payload: &[u8]) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(ts.as_bytes());
    mac.update(b".");
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
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

    let futures = hooks
        .into_iter()
        .filter(|h| should_send(h, event.event))
        .map(|h| {
            let payload = payload.clone();
            let ts = ts.clone();
            let client = client.clone();
            async move {
                let sig = hmac_sig_hex(&h.secret, &ts, &payload);
                let res = client
                    .post(&h.url)
                    .header("User-Agent", "status-webhooks/1.0")
                    .header("Content-Type", "application/json")
                    .header("X-Status-Webhook-Timestamp", &ts)
                    .header("X-Status-Webhook-Signature", format!("sha256={}", sig))
                    .timeout(std::time::Duration::from_secs(5))
                    .body(payload)
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
        });

    future::join_all(futures).await;
}

pub async fn emit_created(pool: std::sync::Arc<Pool>, s: &StatusFromDb) {
    let did = s.author_did.clone();
    let emoji = s.status.clone();
    let text = s.text.clone();
    let uri = s.uri.clone();
    let since = s.started_at.to_rfc3339();
    let expires = s.expires_at.map(|e| e.to_rfc3339());
    let event = StatusEvent {
        event: "status.created",
        did: &did,
        handle: None,
        status: Some(&emoji),
        text: text.as_deref(),
        uri: Some(&uri),
        since: Some(&since),
        expires: expires.as_deref(),
    };
    send_status_event(pool, &did, event).await;
}

pub async fn emit_deleted(pool: std::sync::Arc<Pool>, did: &str, uri: &str) {
    let did_owned = did.to_string();
    let uri_owned = uri.to_string();
    let event = StatusEvent {
        event: "status.deleted",
        did: &did_owned,
        handle: None,
        status: None,
        text: None,
        uri: Some(&uri_owned),
        since: None,
        expires: None,
    };
    send_status_event(pool, &did_owned, event).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_send_wildcard() {
        let h = Webhook {
            id: 1,
            did: "d".into(),
            url: "u".into(),
            secret: "s".into(),
            events: "*".into(),
            active: true,
            created_at: 0,
            updated_at: 0,
        };
        assert!(should_send(&h, "status.created"));
    }

    #[test]
    fn test_should_send_specific() {
        let h = Webhook {
            id: 1,
            did: "d".into(),
            url: "u".into(),
            secret: "s".into(),
            events: "status.deleted".into(),
            active: true,
            created_at: 0,
            updated_at: 0,
        };
        assert!(should_send(&h, "status.deleted"));
        assert!(!should_send(&h, "status.created"));
    }

    #[test]
    fn test_hmac_sig_hex() {
        let sig = hmac_sig_hex("secret", "1234567890", b"{\"a\":1}");
        // Deterministic expected if inputs fixed
        assert_eq!(sig.len(), 64);
    }
}
