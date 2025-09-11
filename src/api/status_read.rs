use crate::config::Config;
use crate::db;
use crate::resolver::HickoryDnsTxtResolver;
use crate::{
    api::auth::OAuthClientType,
    db::StatusFromDb,
    templates::{ErrorTemplate, FeedTemplate, StatusTemplate},
};
use actix_session::Session;
use actix_web::{Responder, Result, get, web};
use askama::Template;
use async_sqlite::Pool;
use atrium_api::types::string::Did;
use atrium_common::resolver::Resolver;
use atrium_identity::handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig};
use atrium_oauth::DefaultHttpClient;
use serde_json::json;
use std::sync::Arc;

use crate::api::status_util::{HandleResolver, is_admin};

/// Homepage - shows logged-in user's status, or owner's status if not logged in
#[get("/")]
pub async fn home(
    session: Session,
    _oauth_client: web::Data<OAuthClientType>,
    db_pool: web::Data<Arc<Pool>>,
    handle_resolver: web::Data<HandleResolver>,
) -> Result<impl Responder> {
    // Default owner of the domain
    const OWNER_HANDLE: &str = "zzstoatzz.io";

    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            let did = Did::new(did_string.clone()).expect("failed to parse did");
            let handle = match handle_resolver.resolve(&did).await {
                Ok(did_doc) => did_doc
                    .also_known_as
                    .and_then(|aka| aka.first().map(|h| h.replace("at://", "")))
                    .unwrap_or_else(|| did_string.clone()),
                Err(_) => did_string.clone(),
            };
            let current_status = StatusFromDb::my_status(&db_pool, &did)
                .await
                .unwrap_or(None)
                .and_then(|s| {
                    if let Some(expires_at) = s.expires_at {
                        if chrono::Utc::now() > expires_at {
                            return None;
                        }
                    }
                    Some(s)
                });
            let history = StatusFromDb::load_user_statuses(&db_pool, &did, 10)
                .await
                .unwrap_or_else(|err| {
                    log::error!("Error loading status history: {err}");
                    vec![]
                });
            let is_admin_flag = is_admin(did.as_str());
            let html = StatusTemplate {
                title: "your status",
                handle,
                current_status,
                history,
                is_owner: true,
                is_admin: is_admin_flag,
            }
            .render()
            .expect("template should be valid");
            Ok(web::Html::new(html))
        }
        None => {
            let atproto_handle_resolver = AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
                dns_txt_resolver: HickoryDnsTxtResolver::default(),
                http_client: Arc::new(DefaultHttpClient::default()),
            });
            let owner_handle =
                atrium_api::types::string::Handle::new(OWNER_HANDLE.to_string()).ok();
            let owner_did = if let Some(handle) = owner_handle {
                atproto_handle_resolver.resolve(&handle).await.ok()
            } else {
                None
            };
            let current_status = if let Some(ref did) = owner_did {
                StatusFromDb::my_status(&db_pool, did)
                    .await
                    .unwrap_or(None)
                    .and_then(|s| {
                        if let Some(expires_at) = s.expires_at {
                            if chrono::Utc::now() > expires_at {
                                return None;
                            }
                        }
                        Some(s)
                    })
            } else {
                None
            };
            let history = if let Some(ref did) = owner_did {
                StatusFromDb::load_user_statuses(&db_pool, did, 10)
                    .await
                    .unwrap_or_else(|err| {
                        log::error!("Error loading status history: {err}");
                        vec![]
                    })
            } else {
                vec![]
            };
            let html = StatusTemplate {
                title: "nate's status",
                handle: OWNER_HANDLE.to_string(),
                current_status,
                history,
                is_owner: false,
                is_admin: false,
            }
            .render()
            .expect("template should be valid");
            Ok(web::Html::new(html))
        }
    }
}

/// View a specific user's status page by handle
#[get("/@{handle}")]
pub async fn user_status_page(
    handle: web::Path<String>,
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    _handle_resolver: web::Data<HandleResolver>,
) -> Result<impl Responder> {
    let handle = handle.into_inner();
    let atproto_handle_resolver = AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
        dns_txt_resolver: HickoryDnsTxtResolver::default(),
        http_client: Arc::new(DefaultHttpClient::default()),
    });
    let handle_obj = atrium_api::types::string::Handle::new(handle.clone()).ok();
    let did = match handle_obj {
        Some(h) => match atproto_handle_resolver.resolve(&h).await {
            Ok(did) => did,
            Err(_) => {
                let html = ErrorTemplate {
                    title: "User not found",
                    error: &format!("Could not find user @{}.", handle),
                }
                .render()
                .expect("template should be valid");
                return Ok(web::Html::new(html));
            }
        },
        None => {
            let html = ErrorTemplate {
                title: "Invalid handle",
                error: &format!("'{}' is not a valid handle format.", handle),
            }
            .render()
            .expect("template should be valid");
            return Ok(web::Html::new(html));
        }
    };
    let is_owner = match session.get::<String>("did").unwrap_or(None) {
        Some(session_did) => session_did == did.to_string(),
        None => false,
    };
    let current_status = StatusFromDb::my_status(&db_pool, &did)
        .await
        .unwrap_or(None)
        .and_then(|s| {
            if let Some(expires_at) = s.expires_at {
                if chrono::Utc::now() > expires_at {
                    return None;
                }
            }
            Some(s)
        });
    let history = StatusFromDb::load_user_statuses(&db_pool, &did, 10)
        .await
        .unwrap_or_else(|err| {
            log::error!("Error loading status history: {err}");
            vec![]
        });
    let html = StatusTemplate {
        title: &format!("@{} status", handle),
        handle,
        current_status,
        history,
        is_owner,
        is_admin: false,
    }
    .render()
    .expect("template should be valid");
    Ok(web::Html::new(html))
}

#[get("/json")]
pub async fn owner_status_json(
    _session: Session,
    db_pool: web::Data<Arc<Pool>>,
    _handle_resolver: web::Data<HandleResolver>,
) -> Result<impl Responder> {
    // Resolve owner handle to DID (zzstoatzz.io)
    let owner_handle = atrium_api::types::string::Handle::new("zzstoatzz.io".to_string()).ok();
    let atproto_handle_resolver = AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
        dns_txt_resolver: HickoryDnsTxtResolver::default(),
        http_client: Arc::new(DefaultHttpClient::default()),
    });
    let did = if let Some(handle) = owner_handle {
        atproto_handle_resolver.resolve(&handle).await.ok()
    } else {
        None
    };
    let current_status = if let Some(did) = did {
        StatusFromDb::my_status(&db_pool, &did)
            .await
            .unwrap_or(None)
            .and_then(|s| {
                if let Some(expires_at) = s.expires_at {
                    if chrono::Utc::now() > expires_at {
                        return None;
                    }
                }
                Some(s)
            })
    } else {
        None
    };
    let response = if let Some(status_data) = current_status {
        json!({ "status": "known", "emoji": status_data.status, "text": status_data.text, "since": status_data.started_at.to_rfc3339(), "expires": status_data.expires_at.map(|e| e.to_rfc3339()) })
    } else {
        json!({ "status": "unknown", "message": "No current status is known" })
    };
    Ok(web::Json(response))
}

#[get("/@{handle}/json")]
pub async fn user_status_json(
    handle: web::Path<String>,
    _session: Session,
    db_pool: web::Data<Arc<Pool>>,
) -> Result<impl Responder> {
    let handle = handle.into_inner();
    let atproto_handle_resolver = AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
        dns_txt_resolver: HickoryDnsTxtResolver::default(),
        http_client: Arc::new(DefaultHttpClient::default()),
    });
    let handle_obj = atrium_api::types::string::Handle::new(handle.clone()).ok();
    let did = if let Some(h) = handle_obj {
        atproto_handle_resolver.resolve(&h).await.ok()
    } else {
        None
    };
    if let Some(did) = did {
        let current_status = StatusFromDb::my_status(&db_pool, &did)
            .await
            .unwrap_or(None)
            .and_then(|s| {
                if let Some(expires_at) = s.expires_at {
                    if chrono::Utc::now() > expires_at {
                        return None;
                    }
                }
                Some(s)
            });
        let response = if let Some(status_data) = current_status {
            json!({ "status": "known", "emoji": status_data.status, "text": status_data.text, "since": status_data.started_at.to_rfc3339(), "expires": status_data.expires_at.map(|e| e.to_rfc3339()) })
        } else {
            json!({ "status": "unknown", "message": format!("No current status is known for @{}", handle) })
        };
        Ok(web::Json(response))
    } else {
        Ok(web::Json(
            json!({ "status": "unknown", "message": format!("Unknown user @{}", handle) }),
        ))
    }
}

#[get("/api/status")]
pub async fn status_json(db_pool: web::Data<Arc<Pool>>) -> Result<impl Responder> {
    // Owner: zzstoatzz.io
    let atproto_handle_resolver = AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
        dns_txt_resolver: HickoryDnsTxtResolver::default(),
        http_client: Arc::new(DefaultHttpClient::default()),
    });
    let owner_handle = atrium_api::types::string::Handle::new("zzstoatzz.io".to_string()).ok();
    let did = if let Some(h) = owner_handle {
        atproto_handle_resolver.resolve(&h).await.ok()
    } else {
        None
    };
    let current_status = if let Some(ref did) = did {
        StatusFromDb::my_status(&db_pool, did)
            .await
            .unwrap_or(None)
            .and_then(|s| {
                if let Some(expires_at) = s.expires_at {
                    if chrono::Utc::now() > expires_at {
                        return None;
                    }
                }
                Some(s)
            })
    } else {
        None
    };
    let response = if let Some(status_data) = current_status {
        json!({ "status": "known", "emoji": status_data.status, "text": status_data.text, "since": status_data.started_at.to_rfc3339(), "expires": status_data.expires_at.map(|e| e.to_rfc3339()) })
    } else {
        json!({ "status": "unknown", "message": "No current status is known" })
    };
    Ok(web::Json(response))
}

#[get("/feed")]
pub async fn feed(
    session: Session,
    _db_pool: web::Data<Arc<Pool>>,
    _handle_resolver: web::Data<HandleResolver>,
    app_config: web::Data<Config>,
) -> Result<impl Responder> {
    let did = session.get::<String>("did").unwrap_or(None);
    let is_admin = did.as_deref().map(is_admin).unwrap_or(false);
    let html = FeedTemplate {
        title: "feed",
        profile: None,
        statuses: vec![],
        is_admin,
        dev_mode: app_config.dev_mode,
    }
    .render()
    .expect("template should be valid");
    Ok(web::Html::new(html))
}

#[get("/api/feed")]
pub async fn api_feed(
    db_pool: web::Data<Arc<Pool>>,
    handle_resolver: web::Data<HandleResolver>,
) -> Result<impl Responder> {
    // Simple paginated feed (offset/limit defaulted)
    let statuses = StatusFromDb::load_latest_statuses(&db_pool)
        .await
        .unwrap_or_default();
    let mut enriched = Vec::with_capacity(statuses.len());
    for mut s in statuses {
        // Resolve handle lazily
        let did = Did::new(s.author_did.clone()).expect("did");
        if let Ok(doc) = handle_resolver.resolve(&did).await {
            if let Some(h) = doc.also_known_as.and_then(|aka| aka.first().cloned()) {
                s.handle = Some(h.replace("at://", ""));
            }
        }
        enriched.push(s);
    }
    Ok(web::Json(json!({ "statuses": enriched })))
}

#[get("/api/frequent-emojis")]
pub async fn get_frequent_emojis(db_pool: web::Data<Arc<Pool>>) -> Result<impl Responder> {
    let emojis = db::get_frequent_emojis(&db_pool, 20)
        .await
        .unwrap_or_default();
    Ok(web::Json(json!({ "emojis": emojis })))
}

#[get("/api/custom-emojis")]
pub async fn get_custom_emojis(app_config: web::Data<Config>) -> Result<impl Responder> {
    // Serve emojis from configured directory
    let dir = app_config.emoji_dir.clone();
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|_| std::fs::read_dir("static/emojis").unwrap());
    let mut names = vec![];
    for entry in entries.flatten() {
        if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(web::Json(json!({ "custom": names })))
}

#[get("/api/following")]
pub async fn get_following(
    _session: Session,
    _oauth_client: web::Data<OAuthClientType>,
    _db_pool: web::Data<Arc<Pool>>,
) -> Result<impl Responder> {
    // Placeholder: follow list disabled here to keep module slim
    Ok(web::Json(json!({ "follows": [] })))
}
