use crate::{db, error_handler::AppError};
use actix_session::Session;
use actix_web::{HttpResponse, Responder, Result, delete, get, post, put, web};
use async_sqlite::Pool;
use atrium_api::types::string::Did;
use serde::Deserialize;
use std::sync::Arc;

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
    payload: web::Json<CreateWebhookRequest>,
) -> Result<impl Responder> {
    let did = session.get::<Did>("did")?;
    if let Some(did) = did {
        // Basic URL validation
        if !(payload.url.starts_with("https://") || payload.url.starts_with("http://")) {
            return Ok(web::Json(serde_json::json!({
                "error": "URL must start with http:// or https://"
            })));
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
) -> impl Responder {
    match session.get::<Did>("did").unwrap_or(None) {
        Some(did) => {
            let id = path.into_inner();
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
