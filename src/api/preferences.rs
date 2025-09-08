use crate::{db, error_handler::AppError};
use actix_session::Session;
use actix_web::{Responder, Result, get, post, web};
use async_sqlite::Pool;
use atrium_api::types::string::Did;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct PreferencesUpdate {
    pub font_family: Option<String>,
    pub accent_color: Option<String>,
}

/// Get user preferences
#[get("/api/preferences")]
pub async fn get_preferences(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
) -> Result<impl Responder> {
    let did = session.get::<Did>("did")?;

    if let Some(did) = did {
        let prefs = db::get_user_preferences(&db_pool, did.as_str())
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(web::Json(serde_json::json!({
            "font_family": prefs.font_family,
            "accent_color": prefs.accent_color
        })))
    } else {
        Ok(web::Json(serde_json::json!({
            "error": "Not authenticated"
        })))
    }
}

/// Save user preferences
#[post("/api/preferences")]
pub async fn save_preferences(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    payload: web::Json<PreferencesUpdate>,
) -> Result<impl Responder> {
    let did = session.get::<Did>("did")?;

    if let Some(did) = did {
        let mut prefs = db::get_user_preferences(&db_pool, did.as_str())
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(font) = &payload.font_family {
            prefs.font_family = font.clone();
        }
        if let Some(color) = &payload.accent_color {
            prefs.accent_color = color.clone();
        }
        prefs.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        db::save_user_preferences(&db_pool, &prefs)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(web::Json(serde_json::json!({
            "success": true
        })))
    } else {
        Ok(web::Json(serde_json::json!({
            "error": "Not authenticated"
        })))
    }
}
