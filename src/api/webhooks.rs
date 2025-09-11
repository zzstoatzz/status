use crate::{config::Config, db, error_handler::AppError};
use actix_session::Session;
use actix_web::{HttpResponse, Responder, Result, delete, get, post, put, web};
use async_sqlite::Pool;
use atrium_api::types::string::Did;
use serde::Deserialize;
use std::sync::Arc;
use url::Url;

#[derive(Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub secret: Option<String>,
    pub events: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateWebhookRequest {
    pub url: Option<String>,
    pub events: Option<String>,
    pub active: Option<bool>,
}

#[get("/api/webhooks")]
pub async fn list_webhooks(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
) -> Result<impl Responder> {
    let did = session.get::<Did>("did")?;
    if let Some(did) = did {
        let hooks = db::get_user_webhooks(&db_pool, did.as_str())
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let response: Vec<serde_json::Value> = hooks
            .into_iter()
            .map(|h| {
                serde_json::json!({
                    "id": h.id,
                    "url": h.url,
                    "events": h.events,
                    "active": h.active,
                    "created_at": h.created_at,
                    "updated_at": h.updated_at,
                    "secret_masked": h.masked_secret()
                })
            })
            .collect();
        Ok(web::Json(serde_json::json!({ "webhooks": response })))
    } else {
        Ok(web::Json(
            serde_json::json!({ "error": "Not authenticated" }),
        ))
    }
}

#[post("/api/webhooks")]
pub async fn create_webhook(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    app_config: web::Data<Config>,
    payload: web::Json<CreateWebhookRequest>,
) -> Result<impl Responder> {
    let did = session.get::<Did>("did")?;
    if let Some(did) = did {
        // Robust URL + SSRF validation
        if let Err(msg) = validate_url(&payload.url, &app_config) {
            return Ok(web::Json(serde_json::json!({ "error": msg })));
        }
        // Events validation
        if let Some(events_str) = &payload.events {
            if let Err(msg) = validate_events(events_str) {
                return Ok(web::Json(serde_json::json!({ "error": msg })));
            }
        }
        let (id, secret) = db::create_webhook(
            &db_pool,
            did.as_str(),
            &payload.url,
            payload.secret.as_deref(),
            payload.events.as_deref(),
        )
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(web::Json(serde_json::json!({
            "id": id,
            "secret": secret, // Only returned once on creation
        })))
    } else {
        Ok(web::Json(
            serde_json::json!({ "error": "Not authenticated" }),
        ))
    }
}

#[put("/api/webhooks/{id}")]
pub async fn update_webhook(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    path: web::Path<i64>,
    payload: web::Json<UpdateWebhookRequest>,
    app_config: web::Data<Config>,
) -> impl Responder {
    match session.get::<Did>("did").unwrap_or(None) {
        Some(did) => {
            let id = path.into_inner();
            if let Some(url) = &payload.url {
                if let Err(msg) = validate_url(url, &app_config) {
                    return HttpResponse::BadRequest().json(serde_json::json!({ "error": msg }));
                }
            }
            if let Some(events_str) = &payload.events {
                if let Err(msg) = validate_events(events_str) {
                    return HttpResponse::BadRequest().json(serde_json::json!({ "error": msg }));
                }
            }
            let res = db::update_webhook(
                &db_pool,
                did.as_str(),
                id,
                payload.url.as_deref(),
                payload.events.as_deref(),
                payload.active,
            )
            .await;
            match res {
                Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
                Err(e) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": e.to_string() })),
            }
        }
        None => {
            HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Not authenticated" }))
        }
    }
}

fn validate_events(s: &str) -> Result<(), &'static str> {
    if s.trim().is_empty() {
        return Ok(());
    }
    const ALLOWED: &[&str] = &["status.created", "status.deleted"];
    for ev in s.split(',').map(|e| e.trim()) {
        if !ALLOWED.contains(&ev) {
            return Err("Unsupported event type");
        }
    }
    Ok(())
}

fn validate_url(raw: &str, cfg: &Config) -> Result<(), &'static str> {
    let url = Url::parse(raw).map_err(|_| "Invalid URL")?;
    let scheme = url.scheme();
    let host = url.host_str().ok_or("Missing host")?.to_ascii_lowercase();

    // Treat localhost explicitly
    let host_is_localname = host == "localhost";

    // If host is an IP literal, apply standard library checks
    let ip_check_blocks = if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                v4.is_private()
                    || v4.is_loopback()
                    || v4.is_link_local()
                    || v4.is_multicast()
                    || v4.is_unspecified()
            }
            std::net::IpAddr::V6(v6) => {
                v6.is_unique_local() || v6.is_loopback() || v6.is_multicast() || v6.is_unspecified()
            }
        }
    } else {
        false
    };

    // Enforce HTTPS in production
    let is_production = !cfg.oauth_redirect_base.starts_with("http://localhost")
        && !cfg.oauth_redirect_base.starts_with("http://127.0.0.1");
    if is_production && scheme != "https" {
        return Err("HTTPS required in production");
    }

    // Basic SSRF protection in production
    if (host_is_localname || ip_check_blocks) && is_production {
        return Err("Private/local hosts not allowed");
    }

    Ok(())
}

#[post("/api/webhooks/{id}/rotate")]
pub async fn rotate_secret(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    path: web::Path<i64>,
) -> impl Responder {
    match session.get::<Did>("did").unwrap_or(None) {
        Some(did) => {
            let id = path.into_inner();
            match db::rotate_webhook_secret(&db_pool, did.as_str(), id).await {
                Ok(new_secret) => {
                    HttpResponse::Ok().json(serde_json::json!({ "secret": new_secret }))
                }
                Err(e) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": e.to_string() })),
            }
        }
        None => {
            HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Not authenticated" }))
        }
    }
}

#[delete("/api/webhooks/{id}")]
pub async fn delete_webhook(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    path: web::Path<i64>,
) -> impl Responder {
    match session.get::<Did>("did").unwrap_or(None) {
        Some(did) => {
            let id = path.into_inner();
            match db::delete_webhook(&db_pool, did.as_str(), id).await {
                Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
                Err(e) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": e.to_string() })),
            }
        }
        None => {
            HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Not authenticated" }))
        }
    }
}
