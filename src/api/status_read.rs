use crate::config::Config;
use crate::db;
use crate::resolver::HickoryDnsTxtResolver;
use crate::{
    api::auth::OAuthClientType,
    db::StatusFromDb,
    templates::{ErrorTemplate, FeedTemplate, StatusShareTemplate, StatusTemplate},
};
use actix_session::Session;
use actix_web::{HttpRequest, HttpResponse, Responder, Result, get, web};
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
            let mut current_status = StatusFromDb::my_status(&db_pool, &did)
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
            let mut history = StatusFromDb::load_user_statuses(&db_pool, &did, 10)
                .await
                .unwrap_or_else(|err| {
                    log::error!("Error loading status history: {err}");
                    vec![]
                });
            if let Some(ref mut status) = current_status {
                status.handle = Some(handle.clone());
            }
            for status in &mut history {
                status.handle = Some(handle.clone());
            }
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
            let mut current_status = if let Some(ref did) = owner_did {
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
            let mut history = if let Some(ref did) = owner_did {
                StatusFromDb::load_user_statuses(&db_pool, did, 10)
                    .await
                    .unwrap_or_else(|err| {
                        log::error!("Error loading status history: {err}");
                        vec![]
                    })
            } else {
                vec![]
            };
            if let Some(ref mut status) = current_status {
                status.handle = Some(OWNER_HANDLE.to_string());
            }
            for status in &mut history {
                status.handle = Some(OWNER_HANDLE.to_string());
            }
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
    let mut current_status = StatusFromDb::my_status(&db_pool, &did)
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
    let mut history = StatusFromDb::load_user_statuses(&db_pool, &did, 10)
        .await
        .unwrap_or_else(|err| {
            log::error!("Error loading status history: {err}");
            vec![]
        });
    if let Some(ref mut status) = current_status {
        status.handle = Some(handle.clone());
    }
    for status in &mut history {
        status.handle = Some(handle.clone());
    }
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

/// Public share page for a specific status
#[get("/s/{did}/{rkey}")]
pub async fn status_share_page(
    req: HttpRequest,
    params: web::Path<(String, String)>,
    db_pool: web::Data<Arc<Pool>>,
    handle_resolver: web::Data<HandleResolver>,
) -> Result<impl Responder> {
    let (did, rkey) = params.into_inner();
    let uri = format!("at://{}/io.zzstoatzz.status.record/{}", did, rkey);

    let mut status = match StatusFromDb::load_by_uri(&db_pool, &uri).await {
        Ok(Some(status)) => status,
        Ok(None) => {
            let html = ErrorTemplate {
                title: "Status not found",
                error: "We couldn't find that status any more.",
            }
            .render()
            .expect("template should be valid");
            return Ok(HttpResponse::NotFound()
                .content_type("text/html; charset=utf-8")
                .body(html));
        }
        Err(err) => {
            log::error!("Database error loading status {}: {}", uri, err);
            let html = ErrorTemplate {
                title: "Something went wrong",
                error: "We couldn't load that status right now.",
            }
            .render()
            .expect("template should be valid");
            return Ok(HttpResponse::InternalServerError()
                .content_type("text/html; charset=utf-8")
                .body(html));
        }
    };

    let handle = match Did::new(status.author_did.clone()) {
        Ok(did) => match handle_resolver.resolve(&did).await {
            Ok(doc) => doc
                .also_known_as
                .and_then(|aka| aka.first().cloned())
                .map(|h| h.replace("at://", "")),
            Err(err) => {
                log::debug!(
                    "Failed to resolve handle for {}: {}",
                    status.author_did,
                    err
                );
                None
            }
        },
        Err(err) => {
            log::warn!("Invalid DID on status {}: {}", status.uri, err);
            None
        }
    };
    status.handle = handle.clone();

    let display_handle = status.author_display_name();
    let meta_title = status.share_title();
    let meta_description = status.share_description();
    let share_text = status.share_text();
    let profile_href = handle
        .clone()
        .map(|h| format!("/@{}", h))
        .unwrap_or_else(|| format!("https://bsky.app/profile/{}", status.author_did));

    let info = req.connection_info();
    let canonical_url = format!("{}://{}/s/{}/{}", info.scheme(), info.host(), did, rkey);
    let share_path = format!("/s/{}/{}", did, rkey);

    let html = StatusShareTemplate {
        title: "status",
        status,
        canonical_url,
        display_handle,
        meta_title,
        meta_description,
        share_text,
        profile_href,
        share_path,
    }
    .render()
    .expect("template should be valid");

    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
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
    handle_resolver: web::Data<HandleResolver>,
    app_config: web::Data<Config>,
) -> Result<impl Responder> {
    let did_opt = session.get::<String>("did").unwrap_or(None);
    let is_admin_flag = did_opt.as_deref().map(is_admin).unwrap_or(false);

    let mut profile: Option<crate::templates::Profile> = None;
    if let Some(did_str) = did_opt.clone() {
        let mut handle_opt: Option<String> = None;
        if let Ok(doc) = handle_resolver
            .resolve(&atrium_api::types::string::Did::new(did_str.clone()).expect("did"))
            .await
        {
            if let Some(h) = doc.also_known_as.and_then(|aka| aka.first().cloned()) {
                handle_opt = Some(h.replace("at://", ""));
            }
        }
        profile = Some(crate::templates::Profile {
            did: did_str,
            display_name: None,
            handle: handle_opt,
        });
    }

    let html = FeedTemplate {
        title: "feed",
        profile,
        statuses: vec![],
        is_admin: is_admin_flag,
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
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<impl Responder> {
    // Paginated feed
    let offset = query
        .get("offset")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(20)
        .clamp(5, 50);

    let statuses = StatusFromDb::load_statuses_paginated(&db_pool, offset, limit)
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
    let has_more = (enriched.len() as i32) == limit;
    Ok(web::Json(
        json!({ "statuses": enriched, "has_more": has_more, "next_offset": offset + (enriched.len() as i32) }),
    ))
}

#[get("/api/frequent-emojis")]
pub async fn get_frequent_emojis(db_pool: web::Data<Arc<Pool>>) -> Result<impl Responder> {
    let emojis = db::get_frequent_emojis(&db_pool, 20)
        .await
        .unwrap_or_default();
    // Legacy response shape: raw array, not wrapped
    Ok(web::Json(emojis))
}

#[get("/api/custom-emojis")]
pub async fn get_custom_emojis(app_config: web::Data<Config>) -> Result<impl Responder> {
    // Response shape expected by UI:
    // [ { "name": "sparkle", "filename": "sparkle.png" }, ... ]
    let dir = app_config.emoji_dir.clone();
    let fs_dir = std::path::Path::new(&dir);
    let fallback = std::path::Path::new("static/emojis");

    let mut map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    let read_dirs = [fs_dir, fallback];
    for d in read_dirs.iter() {
        if let Ok(entries) = std::fs::read_dir(d) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let (Some(stem), Some(ext)) = (p.file_stem(), p.extension()) {
                    let name = stem.to_string_lossy().to_string();
                    let ext = ext.to_string_lossy().to_ascii_lowercase();
                    if ext == "png" || ext == "gif" {
                        // prefer png over gif if duplicates
                        let filename = format!("{}.{ext}", name);
                        map.entry(name)
                            .and_modify(|v| {
                                if v.ends_with(".gif") && ext == "png" {
                                    *v = filename.clone();
                                }
                            })
                            .or_insert(filename);
                    }
                }
            }
        }
    }

    let custom: Vec<serde_json::Value> = map
        .into_iter()
        .map(|(name, filename)| json!({ "name": name, "filename": filename }))
        .collect();
    Ok(web::Json(custom))
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
