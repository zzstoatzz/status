use crate::config::Config;
use crate::{
    api::auth::OAuthClientType, db::StatusFromDb, error_handler::AppError,
    lexicons::record::KnownRecord, rate_limiter::RateLimiter,
};
use actix_multipart::Multipart;
use actix_session::Session;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use async_sqlite::{rusqlite, Pool};
use atrium_api::{
    agent::Agent,
    types::string::{Datetime, Did},
};
use futures_util::TryStreamExt as _;
use std::sync::Arc;

use crate::api::status_util::{parse_duration, HideStatusRequest, StatusForm};

#[post("/admin/upload-emoji")]
pub async fn upload_emoji(
    session: Session,
    mut payload: Multipart,
    app_config: web::Data<Config>,
) -> Result<impl Responder, AppError> {
    if session.get::<String>("did").unwrap_or(None).is_none() {
        return Ok(HttpResponse::Unauthorized().body("Not authenticated"));
    }
    let mut name: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;
    while let Some(item) = payload
        .try_next()
        .await
        .map_err(|e| AppError::ValidationError(e.to_string()))?
    {
        let mut field = item;
        let disp = field.content_disposition().clone();
        let field_name = disp.get_name().unwrap_or("");
        if field_name == "name" {
            let mut buf = Vec::new();
            while let Some(chunk) = field
                .try_next()
                .await
                .map_err(|e| AppError::ValidationError(e.to_string()))?
            {
                buf.extend_from_slice(&chunk);
            }
            name = Some(String::from_utf8_lossy(&buf).trim().to_string());
        } else if field_name == "file" {
            // Capture content type if available
            if let Some(ct) = field.content_type() {
                content_type = Some(ct.to_string());
            }
            let mut buf = Vec::new();
            while let Some(chunk) = field
                .try_next()
                .await
                .map_err(|e| AppError::ValidationError(e.to_string()))?
            {
                buf.extend_from_slice(&chunk);
            }
            file_bytes = Some(buf);
        }
    }
    let file_bytes = file_bytes.ok_or_else(|| AppError::ValidationError("No file".into()))?;
    
    // Determine file extension based on content type or file signature
    let extension = if let Some(ct) = content_type.as_ref() {
        match ct.as_str() {
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => {
                // Fallback to detecting by file signature
                if file_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                    "png"
                } else if file_bytes.starts_with(&[0x47, 0x49, 0x46]) {
                    "gif"
                } else if file_bytes.starts_with(b"RIFF")
                    && file_bytes.len() > 12
                    && &file_bytes[8..12] == b"WEBP"
                {
                    "webp"
                } else {
                    return Err(AppError::ValidationError(
                        "Unsupported image format. Only PNG, GIF, and WebP are allowed.".into(),
                    ));
                }
            }
        }
    } else {
        // Detect by file signature if no content type
        if file_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            "png"
        } else if file_bytes.starts_with(&[0x47, 0x49, 0x46]) {
            "gif"
        } else if file_bytes.starts_with(b"RIFF")
            && file_bytes.len() > 12
            && &file_bytes[8..12] == b"WEBP"
        {
            "webp"
        } else {
            return Err(AppError::ValidationError(
                "Unsupported image format. Only PNG, GIF, and WebP are allowed.".into(),
            ));
        }
    };
    
    let emoji_dir = app_config.emoji_dir.clone();
    let filename = name
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("emoji_{}", chrono::Utc::now().timestamp()));
    let file_path = format!("{}/{}.{}", emoji_dir, filename, extension);
    std::fs::write(&file_path, &file_bytes)
        .map_err(|e| AppError::ValidationError(e.to_string()))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"ok": true, "name": format!("{}.{}", filename, extension)})))
}

/// Clear the user's status by deleting the ATProto record
#[post("/status/clear")]
pub async fn clear_status(
    request: HttpRequest,
    session: Session,
    oauth_client: web::Data<OAuthClientType>,
    db_pool: web::Data<Arc<Pool>>,
) -> HttpResponse {
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            let did = Did::new(did_string.clone()).expect("failed to parse did");
            match StatusFromDb::my_status(&db_pool, &did).await {
                Ok(Some(current_status)) => {
                    let parts: Vec<&str> = current_status.uri.split('/').collect();
                    if let Some(rkey) = parts.last() {
                        match oauth_client.restore(&did).await {
                            Ok(session) => {
                                let agent = Agent::new(session);
                                let delete_request =
                                    atrium_api::com::atproto::repo::delete_record::InputData {
                                        collection: atrium_api::types::string::Nsid::new(
                                            "io.zzstoatzz.status.record".to_string(),
                                        )
                                        .expect("valid nsid"),
                                        repo: did.clone().into(),
                                        rkey: atrium_api::types::string::RecordKey::new(
                                            rkey.to_string(),
                                        )
                                        .expect("valid rkey"),
                                        swap_commit: None,
                                        swap_record: None,
                                    };
                                match agent
                                    .api
                                    .com
                                    .atproto
                                    .repo
                                    .delete_record(delete_request.into())
                                    .await
                                {
                                    Ok(_) => {
                                        let _ = StatusFromDb::delete_by_uri(
                                            &db_pool,
                                            current_status.uri.clone(),
                                        )
                                        .await;
                                        let pool = db_pool.get_ref().clone();
                                        let did_for_event = did_string.clone();
                                        let uri = current_status.uri.clone();
                                        tokio::spawn(async move {
                                            crate::webhooks::emit_deleted(
                                                pool,
                                                &did_for_event,
                                                &uri,
                                            )
                                            .await;
                                        });
                                        web::Redirect::to("/")
                                            .see_other()
                                            .respond_to(&request)
                                            .map_into_boxed_body()
                                    }
                                    Err(e) => {
                                        log::error!("Failed to delete status from ATProto: {e}");
                                        HttpResponse::InternalServerError()
                                            .body("Failed to clear status")
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to restore OAuth session: {e}");
                                HttpResponse::InternalServerError().body("Session error")
                            }
                        }
                    } else {
                        HttpResponse::BadRequest().body("Invalid status URI")
                    }
                }
                Ok(None) => web::Redirect::to("/")
                    .see_other()
                    .respond_to(&request)
                    .map_into_boxed_body(),
                Err(e) => {
                    log::error!("Database error: {e}");
                    HttpResponse::InternalServerError().body("Database error")
                }
            }
        }
        None => web::Redirect::to("/login")
            .see_other()
            .respond_to(&request)
            .map_into_boxed_body(),
    }
}

/// Delete a specific status by URI (JSON endpoint)
#[post("/status/delete")]
pub async fn delete_status(
    session: Session,
    oauth_client: web::Data<OAuthClientType>,
    db_pool: web::Data<Arc<Pool>>,
    req: web::Json<crate::api::status_util::DeleteRequest>,
) -> HttpResponse {
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            let did = Did::new(did_string.clone()).expect("failed to parse did");
            let uri_parts: Vec<&str> = req.uri.split('/').collect();
            if uri_parts.len() < 5 {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({"error":"Invalid status URI format"}));
            }
            let uri_did_part = uri_parts[2];
            if uri_did_part != did_string {
                return HttpResponse::Forbidden()
                    .json(serde_json::json!({"error":"You can only delete your own statuses"}));
            }
            if let Some(rkey) = uri_parts.last() {
                match oauth_client.restore(&did).await {
                    Ok(session) => {
                        let agent = Agent::new(session);
                        let delete_request =
                            atrium_api::com::atproto::repo::delete_record::InputData {
                                collection: atrium_api::types::string::Nsid::new(
                                    "io.zzstoatzz.status.record".to_string(),
                                )
                                .expect("valid nsid"),
                                repo: did.clone().into(),
                                rkey: atrium_api::types::string::RecordKey::new(rkey.to_string())
                                    .expect("valid rkey"),
                                swap_commit: None,
                                swap_record: None,
                            };
                        match agent
                            .api
                            .com
                            .atproto
                            .repo
                            .delete_record(delete_request.into())
                            .await
                        {
                            Ok(_) => {
                                let _ =
                                    StatusFromDb::delete_by_uri(&db_pool, req.uri.clone()).await;
                                let pool = db_pool.get_ref().clone();
                                let did_for_event = did_string.clone();
                                let uri = req.uri.clone();
                                tokio::spawn(async move {
                                    crate::webhooks::emit_deleted(pool, &did_for_event, &uri)
                                        .await;
                                });
                                HttpResponse::Ok().json(serde_json::json!({"success":true}))
                            }
                            Err(e) => {
                                log::error!("Failed to delete status from ATProto: {e}");
                                HttpResponse::InternalServerError()
                                    .json(serde_json::json!({"error":"Failed to delete status"}))
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to restore OAuth session: {e}");
                        HttpResponse::InternalServerError()
                            .json(serde_json::json!({"error":"Session error"}))
                    }
                }
            } else {
                HttpResponse::BadRequest()
                    .json(serde_json::json!({"error":"Invalid status URI"}))
            }
        }
        None => HttpResponse::Unauthorized()
            .json(serde_json::json!({"error":"Not authenticated"})),
    }
}

/// Hide/unhide a status (admin only)
#[post("/admin/hide-status")]
pub async fn hide_status(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    req: web::Json<HideStatusRequest>,
) -> HttpResponse {
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            if did_string != crate::api::status_util::ADMIN_DID {
                return HttpResponse::Forbidden()
                    .json(serde_json::json!({"error":"Admin access required"}));
            }
            let uri = req.uri.clone();
            let hidden = req.hidden;
            let result = db_pool
                .conn(move |conn| {
                    conn.execute(
                        "UPDATE status SET hidden = ?1 WHERE uri = ?2",
                        rusqlite::params![hidden, uri],
                    )
                })
                .await;
            match result {
                Ok(rows_affected) if rows_affected > 0 => HttpResponse::Ok().json(
                    serde_json::json!({
                        "success": true,
                        "message": if hidden { "Status hidden" } else { "Status unhidden" }
                    }),
                ),
                Ok(_) => HttpResponse::NotFound()
                    .json(serde_json::json!({"error":"Status not found"})),
                Err(err) => {
                    log::error!("Error updating hidden status: {}", err);
                    HttpResponse::InternalServerError()
                        .json(serde_json::json!({"error":"Database error"}))
                }
            }
        }
        None => HttpResponse::Unauthorized()
            .json(serde_json::json!({"error":"Not authenticated"})),
    }
}

/// Creates a new status
#[post("/status")]
pub async fn status(
    request: HttpRequest,
    session: Session,
    oauth_client: web::Data<OAuthClientType>,
    db_pool: web::Data<Arc<Pool>>,
    form: web::Form<StatusForm>,
    rate_limiter: web::Data<RateLimiter>,
) -> Result<HttpResponse, AppError> {
    let client_key = RateLimiter::get_client_key(&request);
    if !rate_limiter.check_rate_limit(&client_key) {
        return Err(AppError::RateLimitExceeded);
    }
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            let did = Did::new(did_string.clone()).expect("failed to parse did");
            match oauth_client.restore(&did).await {
                Ok(session) => {
                    let agent = Agent::new(session);
                    let expires = form
                        .expires_in
                        .as_ref()
                        .and_then(|exp| parse_duration(exp))
                        .and_then(|duration| {
                            let expiry_time = chrono::Utc::now() + duration;
                            Some(Datetime::new(expiry_time.to_rfc3339().parse().ok()?))
                        });
                    let status: KnownRecord =
                        crate::lexicons::io::zzstoatzz::status::record::RecordData {
                            created_at: Datetime::now(),
                            emoji: form.status.clone(),
                            text: form.text.clone(),
                            expires,
                        }
                        .into();
                    let create_result = agent
                        .api
                        .com
                        .atproto
                        .repo
                        .create_record(
                            atrium_api::com::atproto::repo::create_record::InputData {
                                collection: "io.zzstoatzz.status.record".parse().unwrap(),
                                repo: did.into(),
                                rkey: None,
                                record: status.into(),
                                swap_commit: None,
                                validate: None,
                            }
                            .into(),
                        )
                        .await;
                    match create_result {
                        Ok(record) => {
                            let mut status = StatusFromDb::new(
                                record.uri.clone(),
                                did_string,
                                form.status.clone(),
                            );
                            status.text = form.text.clone();
                            if let Some(exp_str) = &form.expires_in {
                                if let Some(duration) = parse_duration(exp_str) {
                                    status.expires_at = Some(chrono::Utc::now() + duration);
                                }
                            }
                            let _ = status.save(db_pool.clone()).await;
                            {
                                let pool = db_pool.get_ref().clone();
                                let s = status.clone();
                                tokio::spawn(async move {
                                    crate::webhooks::emit_created(pool, &s).await;
                                });
                            }
                            Ok(web::Redirect::to("/")
                                .see_other()
                                .respond_to(&request)
                                .map_into_boxed_body())
                        }
                        Err(err) => {
                            log::error!("Error creating status: {err}");
                            Ok(HttpResponse::Ok()
                                .body("Was an error creating the status, please check the logs."))
                        }
                    }
                }
                Err(err) => {
                    session.purge();
                    log::error!("Error restoring session: {err}");
                    Err(AppError::AuthenticationError("Session error".to_string()))
                }
            }
        }
        None => Err(AppError::AuthenticationError(
            "You must be logged in to create a status.".to_string(),
        )),
    }
}
