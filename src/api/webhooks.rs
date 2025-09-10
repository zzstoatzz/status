use crate::db::{WebhookConfig, WebhookDelivery};
use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Result};
use async_sqlite::Pool;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize, Deserialize)]
pub struct WebhookConfigRequest {
    pub webhook_url: String,
    pub webhook_secret: Option<String>, // If None, we'll generate one
}

#[derive(Serialize, Deserialize)]
pub struct WebhookConfigResponse {
    pub id: i64,
    pub webhook_url: String,
    pub webhook_secret: String,
    pub enabled: bool,
    pub last_delivery_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize)]
pub struct WebhookTestRequest {
    pub webhook_url: String,
    pub webhook_secret: String,
}

#[derive(Serialize, Deserialize)]
pub struct WebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub user_did: String,
    pub handle: String,
    pub emoji: String,
    pub text: Option<String>,
    pub expires_at: Option<String>,
    pub status_uri: String,
    pub timestamp: String,
    pub event_id: String,
    pub schema: String,
}

impl WebhookEvent {
    pub fn new_status_set(
        user_did: String,
        handle: String,
        emoji: String,
        text: Option<String>,
        expires_at: Option<String>,
        status_uri: String,
    ) -> Self {
        Self {
            event_type: "status.set".to_string(),
            user_did,
            handle,
            emoji,
            text,
            expires_at,
            status_uri,
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_id: Uuid::new_v4().to_string(),
            schema: "status-webhook.v1".to_string(),
        }
    }

    pub fn new_status_cleared(
        user_did: String,
        handle: String,
        status_uri: String,
    ) -> Self {
        Self {
            event_type: "status.cleared".to_string(),
            user_did,
            handle,
            emoji: String::new(),
            text: None,
            expires_at: None,
            status_uri,
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_id: Uuid::new_v4().to_string(),
            schema: "status-webhook.v1".to_string(),
        }
    }
}

/// Generate HMAC signature for webhook payload
pub fn generate_signature(secret: &str, timestamp: i64, payload: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    let message = format!("v0:{}:{}", timestamp, payload);
    mac.update(message.as_bytes());
    let result = mac.finalize();
    format!("v1={}", hex::encode(result.into_bytes()))
}

/// Verify webhook signature
pub fn verify_signature(secret: &str, timestamp: i64, payload: &str, signature: &str) -> bool {
    if !signature.starts_with("v1=") {
        return false;
    }
    
    let expected_signature = generate_signature(secret, timestamp, payload);
    signature == expected_signature
}

/// Get webhook configuration for authenticated user
#[get("/api/webhook/config")]
pub async fn get_webhook_config(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
) -> Result<HttpResponse> {
    let user_did = match session.get::<String>("did").unwrap_or(None) {
        Some(did) => did,
        None => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Not authenticated"
            })));
        }
    };

    match WebhookConfig::get_by_user_did(&db_pool, &user_did).await {
        Ok(Some(config)) => {
            let response = WebhookConfigResponse {
                id: config.id.unwrap_or(0),
                webhook_url: config.webhook_url,
                webhook_secret: config.webhook_secret,
                enabled: config.enabled,
                last_delivery_at: config.last_delivery_at,
                created_at: config.created_at,
                updated_at: config.updated_at,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Webhook not configured"
        }))),
        Err(err) => {
            log::error!("Failed to get webhook config: {}", err);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database error"
            })))
        }
    }
}

/// Configure or update webhook for authenticated user
#[post("/api/webhook/config")]
pub async fn configure_webhook(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    req: web::Json<WebhookConfigRequest>,
) -> Result<HttpResponse> {
    let user_did = match session.get::<String>("did").unwrap_or(None) {
        Some(did) => did,
        None => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Not authenticated"
            })));
        }
    };

    // Validate webhook URL
    if !req.webhook_url.starts_with("https://") {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Webhook URL must use HTTPS"
        })));
    }

    // Generate secret if not provided
    let webhook_secret = match &req.webhook_secret {
        Some(secret) => secret.clone(),
        None => {
            // Generate a secure random secret
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let secret_bytes: [u8; 32] = rng.gen();
            hex::encode(secret_bytes)
        }
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut config = WebhookConfig::new(user_did, req.webhook_url.clone(), webhook_secret);
    config.updated_at = now;

    match config.save_or_update(&db_pool).await {
        Ok(config_id) => {
            config.id = Some(config_id);
            let response = WebhookConfigResponse {
                id: config_id,
                webhook_url: config.webhook_url,
                webhook_secret: config.webhook_secret,
                enabled: config.enabled,
                last_delivery_at: config.last_delivery_at,
                created_at: config.created_at,
                updated_at: config.updated_at,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Err(err) => {
            log::error!("Failed to save webhook config: {}", err);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to save webhook configuration"
            })))
        }
    }
}

/// Test webhook delivery
#[post("/api/webhook/test")]
pub async fn test_webhook(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    req: web::Json<WebhookTestRequest>,
) -> Result<HttpResponse> {
    let user_did = match session.get::<String>("did").unwrap_or(None) {
        Some(did) => did,
        None => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Not authenticated"
            })));
        }
    };

    // Create a test event
    let test_event = WebhookEvent::new_status_set(
        user_did.clone(),
        "test.example.com".to_string(),
        "🧪".to_string(),
        Some("This is a test webhook delivery".to_string()),
        None,
        format!("at://{}/io.zzstoatzz.status.record/test", user_did),
    );

    let payload = serde_json::to_string(&test_event).map_err(|e| {
        log::error!("Failed to serialize test event: {}", e);
        actix_web::error::ErrorInternalServerError("Serialization error")
    })?;

    // Try to deliver the webhook
    let result = deliver_webhook_event(&req.webhook_url, &req.webhook_secret, &payload).await;

    match result {
        Ok((status, response_body)) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "status_code": status,
                "response": response_body,
                "message": "Test webhook delivered successfully"
            })))
        }
        Err(err) => {
            log::warn!("Test webhook delivery failed: {}", err);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": format!("Webhook delivery failed: {}", err),
                "message": "Test webhook delivery failed"
            })))
        }
    }
}

/// Get recent webhook deliveries
#[get("/api/webhook/deliveries")]
pub async fn get_webhook_deliveries(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
) -> Result<HttpResponse> {
    let user_did = match session.get::<String>("did").unwrap_or(None) {
        Some(did) => did,
        None => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Not authenticated"
            })));
        }
    };

    // First get the webhook config to get the config_id
    match WebhookConfig::get_by_user_did(&db_pool, &user_did).await {
        Ok(Some(config)) => {
            let config_id = config.id.unwrap_or(0);
            match WebhookDelivery::get_recent_deliveries(&db_pool, config_id, 20).await {
                Ok(deliveries) => Ok(HttpResponse::Ok().json(deliveries)),
                Err(err) => {
                    log::error!("Failed to get webhook deliveries: {}", err);
                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to get delivery history"
                    })))
                }
            }
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Webhook not configured"
        }))),
        Err(err) => {
            log::error!("Failed to get webhook config: {}", err);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database error"
            })))
        }
    }
}

/// Delete webhook configuration
#[post("/api/webhook/delete")]
pub async fn delete_webhook(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
) -> Result<HttpResponse> {
    let user_did = match session.get::<String>("did").unwrap_or(None) {
        Some(did) => did,
        None => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Not authenticated"
            })));
        }
    };

    match WebhookConfig::delete_by_user_did(&db_pool, &user_did).await {
        Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Webhook configuration deleted"
        }))),
        Err(err) => {
            log::error!("Failed to delete webhook config: {}", err);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to delete webhook configuration"
            })))
        }
    }
}

/// Deliver a webhook event via HTTP POST
pub async fn deliver_webhook_event(
    webhook_url: &str,
    webhook_secret: &str,
    payload: &str,
) -> Result<(u16, String), reqwest::Error> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let signature = generate_signature(webhook_secret, timestamp, payload);
    let event_id = Uuid::new_v4().to_string();

    let client = reqwest::Client::new();
    let response = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "Status-Webhook/1.0")
        .header("X-Status-Timestamp", timestamp.to_string())
        .header("X-Status-Event-Id", &event_id)
        .header("X-Status-Signature", &signature)
        .header("Idempotency-Key", &event_id)
        .body(payload.to_string())
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();

    Ok((status, body))
}

/// Send webhook event for status changes (called from status operations)
pub async fn emit_webhook_event(
    db_pool: &Pool,
    user_did: &str,
    event: &WebhookEvent,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get webhook config for the user
    if let Some(config) = WebhookConfig::get_by_user_did(db_pool, user_did).await? {
        if !config.enabled {
            log::debug!("Webhook disabled for user {}", user_did);
            return Ok(());
        }

        let config_id = config.id.unwrap_or(0);
        let payload = serde_json::to_string(event)?;

        // Create delivery record
        let mut delivery = WebhookDelivery::new(
            config_id,
            event.event_id.clone(),
            event.event_type.clone(),
            payload.clone(),
        );

        let delivery_id = delivery.save(db_pool).await?;
        delivery.id = Some(delivery_id);

        // Try to deliver
        match deliver_webhook_event(&config.webhook_url, &config.webhook_secret, &payload).await {
            Ok((status, response_body)) => {
                let success = status >= 200 && status < 300;
                delivery.update_result(db_pool, status as i32, Some(response_body), success).await?;
                
                // Update last delivery time on config
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                config.update_last_delivery(db_pool, now).await?;

                if success {
                    log::debug!("Webhook delivered successfully for user {} (status: {})", user_did, status);
                } else {
                    log::warn!("Webhook delivery failed for user {} (status: {})", user_did, status);
                }
            }
            Err(err) => {
                log::error!("Webhook delivery failed for user {}: {}", user_did, err);
                delivery.update_result(db_pool, 0, Some(err.to_string()), false).await?;
                
                // TODO: Implement retry logic here
                // For now, we just log the failure
            }
        }
    }

    Ok(())
}