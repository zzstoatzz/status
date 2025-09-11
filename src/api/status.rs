use crate::config::Config;
use crate::emoji::is_builtin_slug;
use crate::resolver::HickoryDnsTxtResolver;
use crate::{
    api::auth::OAuthClientType,
    config,
    db::{self, StatusFromDb},
    dev_utils,
    error_handler::AppError,
    lexicons::record::KnownRecord,
    rate_limiter::RateLimiter,
    templates::{ErrorTemplate, FeedTemplate, Profile, StatusTemplate},
};
use actix_multipart::Multipart;
use actix_session::Session;
use actix_web::{
    HttpRequest, HttpResponse, Responder, Result, get, post,
    web::{self, Redirect},
};
use askama::Template;
use async_sqlite::{Pool, rusqlite};
use atrium_api::{
    agent::Agent,
    types::string::{Datetime, Did},
};
use atrium_common::resolver::Resolver;
use atrium_identity::{
    did::CommonDidResolver,
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig},
};
use atrium_oauth::DefaultHttpClient;
use futures_util::TryStreamExt as _;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

/// HandleResolver to make it easier to access the OAuthClient in web requests
pub type HandleResolver = Arc<CommonDidResolver<DefaultHttpClient>>;

/// Admin DID for moderation
const ADMIN_DID: &str = "did:plc:xbtmt2zjwlrfegqvch7fboei"; // zzstoatzz.io

/// Check if a DID is the admin
fn is_admin(did: &str) -> bool {
    did == ADMIN_DID
}

use crate::api::status_util::{DeleteRequest, HideStatusRequest, StatusForm, parse_duration};

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

    // Check if user is logged in
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            // User is logged in - show their status page
            log::debug!("Home: User is logged in with DID: {}", did_string);
            let did = Did::new(did_string.clone()).expect("failed to parse did");

            // Get their handle
            let handle = match handle_resolver.resolve(&did).await {
                Ok(did_doc) => did_doc
                    .also_known_as
                    .and_then(|aka| aka.first().map(|h| h.replace("at://", "")))
                    .unwrap_or_else(|| did_string.clone()),
                Err(_) => did_string.clone(),
            };

            // Get user's status
            let current_status = StatusFromDb::my_status(&db_pool, &did)
                .await
                .unwrap_or(None)
                .and_then(|s| {
                    // Check if status is expired
                    if let Some(expires_at) = s.expires_at {
                        if chrono::Utc::now() > expires_at {
                            return None; // Status expired
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
                is_owner: true, // They're viewing their own status
                is_admin: is_admin_flag,
            }
            .render()
            .expect("template should be valid");

            Ok(web::Html::new(html))
        }
        None => {
            // Not logged in - show owner's status
            // Resolve owner handle to DID
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
                        // Check if status is expired
                        if let Some(expires_at) = s.expires_at {
                            if chrono::Utc::now() > expires_at {
                                return None; // Status expired
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
                is_owner: false, // Visitor viewing owner's status
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

    // Resolve handle to DID using ATProto handle resolution
    // First we need to create a handle resolver
    let atproto_handle_resolver = AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
        dns_txt_resolver: HickoryDnsTxtResolver::default(),
        http_client: Arc::new(DefaultHttpClient::default()),
    });

    let handle_obj = atrium_api::types::string::Handle::new(handle.clone()).ok();
    let did = match handle_obj {
        Some(h) => match atproto_handle_resolver.resolve(&h).await {
            Ok(did) => did,
            Err(_) => {
                // Could not resolve handle
                let html = ErrorTemplate {
                    title: "User not found",
                    error: &format!("Could not find user @{}. This handle may not exist or may not be using the ATProto network.", handle),
                }
                .render()
                .expect("template should be valid");
                return Ok(web::Html::new(html));
            }
        },
        None => {
            // Invalid handle format
            let html = ErrorTemplate {
                title: "Invalid handle",
                error: &format!(
                    "'{}' is not a valid handle format. Handles should be like 'alice.bsky.social'",
                    handle
                ),
            }
            .render()
            .expect("template should be valid");
            return Ok(web::Html::new(html));
        }
    };

    // Check if logged in user is viewing their own page
    let is_owner = match session.get::<String>("did").unwrap_or(None) {
        Some(session_did) => session_did == did.to_string(),
        None => false,
    };

    // Get user's status
    let current_status = StatusFromDb::my_status(&db_pool, &did)
        .await
        .unwrap_or(None)
        .and_then(|s| {
            // Check if status is expired
            if let Some(expires_at) = s.expires_at {
                if chrono::Utc::now() > expires_at {
                    return None; // Status expired
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

    let is_admin_flag = match session.get::<String>("did").unwrap_or(None) {
        Some(d) => is_admin(&d),
        None => false,
    };
    let html = StatusTemplate {
        title: &format!("@{} status", handle),
        handle: handle.clone(),
        current_status,
        history,
        is_owner,
        is_admin: is_admin_flag,
    }
    .render()
    .expect("template should be valid");

    Ok(web::Html::new(html))
}

/// JSON API for the owner's status (top-level endpoint)
#[get("/json")]
pub async fn owner_status_json(
    db_pool: web::Data<Arc<Pool>>,
    _handle_resolver: web::Data<HandleResolver>,
) -> Result<impl Responder> {
    // Default owner of the domain
    const OWNER_HANDLE: &str = "zzstoatzz.io";

    // Resolve handle to DID using ATProto handle resolution
    let atproto_handle_resolver = AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
        dns_txt_resolver: HickoryDnsTxtResolver::default(),
        http_client: Arc::new(DefaultHttpClient::default()),
    });

    let did = match atproto_handle_resolver
        .resolve(&OWNER_HANDLE.parse().expect("failed to parse handle"))
        .await
    {
        Ok(d) => Some(d.to_string()),
        Err(e) => {
            log::error!("Failed to resolve handle {}: {}", OWNER_HANDLE, e);
            None
        }
    };

    let current_status = if let Some(did) = did {
        let did = Did::new(did).expect("failed to parse did");
        StatusFromDb::my_status(&db_pool, &did)
            .await
            .unwrap_or(None)
            .and_then(|s| {
                // Check if status is expired
                if let Some(expires_at) = s.expires_at {
                    if chrono::Utc::now() > expires_at {
                        return None; // Status expired
                    }
                }
                Some(s)
            })
    } else {
        None
    };

    let response = if let Some(status_data) = current_status {
        serde_json::json!({
            "handle": OWNER_HANDLE,
            "status": "known",
            "emoji": status_data.status,
            "text": status_data.text,
            "since": status_data.started_at.to_rfc3339(),
            "expires": status_data.expires_at.map(|e| e.to_rfc3339()),
        })
    } else {
        serde_json::json!({
            "handle": OWNER_HANDLE,
            "status": "unknown",
            "message": "No current status is known"
        })
    };

    Ok(web::Json(response))
}

/// JSON API for a specific user's status
#[get("/@{handle}/json")]
pub async fn user_status_json(
    handle: web::Path<String>,
    db_pool: web::Data<Arc<Pool>>,
    _handle_resolver: web::Data<HandleResolver>,
) -> Result<impl Responder> {
    let handle = handle.into_inner();

    // Resolve handle to DID using ATProto handle resolution
    // First we need to create a handle resolver
    let atproto_handle_resolver = AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
        dns_txt_resolver: HickoryDnsTxtResolver::default(),
        http_client: Arc::new(DefaultHttpClient::default()),
    });

    let handle_obj = atrium_api::types::string::Handle::new(handle.clone()).ok();
    let did = match handle_obj {
        Some(h) => match atproto_handle_resolver.resolve(&h).await {
            Ok(did) => did,
            Err(_) => {
                return Ok(web::Json(serde_json::json!({
                    "status": "unknown",
                    "message": format!("Could not resolve handle @{}", handle)
                })));
            }
        },
        None => {
            return Ok(web::Json(serde_json::json!({
                "status": "unknown",
                "message": format!("Invalid handle format: @{}", handle)
            })));
        }
    };

    let current_status = StatusFromDb::my_status(&db_pool, &did)
        .await
        .unwrap_or(None)
        .and_then(|s| {
            // Check if status is expired
            if let Some(expires_at) = s.expires_at {
                if chrono::Utc::now() > expires_at {
                    return None; // Status expired
                }
            }
            Some(s)
        });

    let response = if let Some(status_data) = current_status {
        serde_json::json!({
            "status": "known",
            "emoji": status_data.status,
            "text": status_data.text,
            "since": status_data.started_at.to_rfc3339(),
            "expires": status_data.expires_at.map(|e| e.to_rfc3339()),
        })
    } else {
        serde_json::json!({
            "status": "unknown",
            "message": format!("No current status is known for @{}", handle)
        })
    };

    Ok(web::Json(response))
}

/// JSON API endpoint for status - returns current status or "unknown"
#[get("/api/status")]
pub async fn status_json(db_pool: web::Data<Arc<Pool>>) -> Result<impl Responder> {
    const OWNER_DID: &str = "did:plc:xbtmt2zjwlrfegqvch7fboei"; // zzstoatzz.io

    let owner_did = Did::new(OWNER_DID.to_string()).ok();
    let current_status = if let Some(ref did) = owner_did {
        StatusFromDb::my_status(&db_pool, did)
            .await
            .unwrap_or(None)
            .and_then(|s| {
                // Check if status is expired
                if let Some(expires_at) = s.expires_at {
                    if chrono::Utc::now() > expires_at {
                        return None; // Status expired
                    }
                }
                Some(s)
            })
    } else {
        None
    };

    let response = if let Some(status_data) = current_status {
        serde_json::json!({
            "status": "known",
            "emoji": status_data.status,
            "text": status_data.text,
            "since": status_data.started_at.to_rfc3339(),
            "expires": status_data.expires_at.map(|e| e.to_rfc3339()),
        })
    } else {
        serde_json::json!({
            "status": "unknown",
            "message": "No current status is known"
        })
    };

    Ok(web::Json(response))
}

/// Feed page - shows all users' statuses
#[get("/feed")]
pub async fn feed(
    request: HttpRequest,
    session: Session,
    oauth_client: web::Data<OAuthClientType>,
    db_pool: web::Data<Arc<Pool>>,
    handle_resolver: web::Data<HandleResolver>,
    config: web::Data<config::Config>,
) -> Result<impl Responder> {
    // This is essentially the old home function
    const TITLE: &str = "status feed";

    // Check if dev mode is active
    let query = request.query_string();
    let use_dev_mode = config.dev_mode && dev_utils::is_dev_mode_requested(query);

    let mut statuses = if use_dev_mode {
        // Mix dummy data with real data for testing
        let mut real_statuses = StatusFromDb::load_latest_statuses(&db_pool)
            .await
            .unwrap_or_else(|err| {
                log::error!("Error loading statuses: {err}");
                vec![]
            });
        let dummy_statuses = dev_utils::generate_dummy_statuses(15);
        real_statuses.extend(dummy_statuses);
        // Resort by started_at
        real_statuses.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        real_statuses
    } else {
        StatusFromDb::load_latest_statuses(&db_pool)
            .await
            .unwrap_or_else(|err| {
                log::error!("Error loading statuses: {err}");
                vec![]
            })
    };

    let mut quick_resolve_map: HashMap<Did, String> = HashMap::new();
    for db_status in &mut statuses {
        let authors_did = Did::new(db_status.author_did.clone()).expect("failed to parse did");
        match quick_resolve_map.get(&authors_did) {
            None => {}
            Some(found_handle) => {
                db_status.handle = Some(found_handle.clone());
                continue;
            }
        }
        db_status.handle = match handle_resolver.resolve(&authors_did).await {
            Ok(did_doc) => match did_doc.also_known_as {
                None => None,
                Some(also_known_as) => match also_known_as.is_empty() {
                    true => None,
                    false => {
                        let full_handle = also_known_as.first().unwrap();
                        let handle = full_handle.replace("at://", "");
                        quick_resolve_map.insert(authors_did, handle.clone());
                        Some(handle)
                    }
                },
            },
            Err(err) => {
                log::error!("Error resolving did: {err}");
                None
            }
        };
    }

    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            log::debug!("Feed: User has session with DID: {}", did_string);
            let did = Did::new(did_string.clone()).expect("failed to parse did");
            let _my_status = StatusFromDb::my_status(&db_pool, &did)
                .await
                .unwrap_or_else(|err| {
                    log::error!("Error loading my status: {err}");
                    None
                });

            log::debug!(
                "Feed: Attempting to restore OAuth session for DID: {}",
                did_string
            );
            match oauth_client.restore(&did).await {
                Ok(session) => {
                    log::debug!("Feed: Successfully restored OAuth session");
                    let agent = Agent::new(session);
                    let profile = agent
                        .api
                        .app
                        .bsky
                        .actor
                        .get_profile(
                            atrium_api::app::bsky::actor::get_profile::ParametersData {
                                actor: atrium_api::types::string::AtIdentifier::Did(did.clone()),
                            }
                            .into(),
                        )
                        .await;

                    let is_admin = is_admin(did.as_str());
                    let html = FeedTemplate {
                        title: TITLE,
                        profile: match profile {
                            Ok(profile) => {
                                let profile_data = Profile {
                                    did: profile.did.to_string(),
                                    display_name: profile.display_name.clone(),
                                    handle: Some(profile.handle.to_string()),
                                };
                                Some(profile_data)
                            }
                            Err(err) => {
                                log::error!("Error accessing profile: {err}");
                                None
                            }
                        },
                        statuses,
                        is_admin,
                        dev_mode: use_dev_mode,
                    }
                    .render()
                    .expect("template should be valid");

                    Ok(web::Html::new(html))
                }
                Err(err) => {
                    // Don't purge the session - OAuth tokens might be expired but user is still logged in
                    log::warn!("Could not restore OAuth session for feed: {:?}", err);

                    // Show feed without profile info instead of error page
                    let html = FeedTemplate {
                        title: TITLE,
                        profile: None,
                        statuses,
                        is_admin: is_admin(did.as_str()),
                        dev_mode: use_dev_mode,
                    }
                    .render()
                    .expect("template should be valid");

                    Ok(web::Html::new(html))
                }
            }
        }
        None => {
            let html = FeedTemplate {
                title: TITLE,
                profile: None,
                statuses,
                is_admin: false,
                dev_mode: use_dev_mode,
            }
            .render()
            .expect("template should be valid");

            Ok(web::Html::new(html))
        }
    }
}

/// Get paginated statuses for infinite scrolling
#[get("/api/feed")]
pub async fn api_feed(
    query: web::Query<HashMap<String, String>>,
    db_pool: web::Data<Arc<Pool>>,
    handle_resolver: web::Data<HandleResolver>,
    config: web::Data<config::Config>,
) -> Result<impl Responder> {
    let offset = query
        .get("offset")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let limit = query
        .get("limit")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(20)
        .min(50); // Cap at 50 items per request

    // Check if dev mode is requested
    let use_dev_mode = config.dev_mode && query.get("dev").is_some_and(|v| v == "true" || v == "1");

    let mut statuses = if use_dev_mode && offset == 0 {
        // For first page in dev mode, mix dummy data with real data
        let mut real_statuses = StatusFromDb::load_statuses_paginated(&db_pool, 0, limit / 2)
            .await
            .unwrap_or_else(|err| {
                log::error!("Error loading paginated statuses: {err}");
                vec![]
            });
        let dummy_statuses = dev_utils::generate_dummy_statuses((limit / 2) as usize);
        real_statuses.extend(dummy_statuses);
        real_statuses.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        real_statuses
    } else {
        StatusFromDb::load_statuses_paginated(&db_pool, offset, limit)
            .await
            .unwrap_or_else(|err| {
                log::error!("Error loading statuses: {err}");
                vec![]
            })
    };

    // Resolve handles for each status
    let mut quick_resolve_map: HashMap<Did, String> = HashMap::new();
    for db_status in &mut statuses {
        let authors_did = Did::new(db_status.author_did.clone()).expect("failed to parse did");
        match quick_resolve_map.get(&authors_did) {
            None => {}
            Some(found_handle) => {
                db_status.handle = Some(found_handle.clone());
                continue;
            }
        }
        db_status.handle = match handle_resolver.resolve(&authors_did).await {
            Ok(did_doc) => match did_doc.also_known_as {
                None => None,
                Some(also_known_as) => match also_known_as.is_empty() {
                    true => None,
                    false => {
                        let full_handle = also_known_as.first().unwrap();
                        let handle = full_handle.replace("at://", "");
                        quick_resolve_map.insert(authors_did, handle.clone());
                        Some(handle)
                    }
                },
            },
            Err(_) => None,
        };
    }

    Ok(HttpResponse::Ok().json(statuses))
}

/// Get the most frequently used emojis from all statuses
#[get("/api/frequent-emojis")]
pub async fn get_frequent_emojis(db_pool: web::Data<Arc<Pool>>) -> Result<impl Responder> {
    // Get top 20 most frequently used emojis
    let emojis = db::get_frequent_emojis(&db_pool, 20)
        .await
        .unwrap_or_else(|err| {
            log::error!("Failed to get frequent emojis: {}", err);
            Vec::new()
        });

    // If we have less than 12 emojis, add some defaults to fill it out
    let mut result = emojis;
    if result.is_empty() {
        log::info!("No emoji usage data found, using defaults");
        let defaults = vec![
            "😊", "👍", "❤️", "😂", "🎉", "🔥", "✨", "💯", "🚀", "💪", "🙏", "👏",
        ];
        result = defaults.into_iter().map(String::from).collect();
    } else if result.len() < 12 {
        log::info!("Found {} emojis, padding with defaults", result.len());
        let defaults = vec![
            "😊", "👍", "❤️", "😂", "🎉", "🔥", "✨", "💯", "🚀", "💪", "🙏", "👏",
        ];
        for emoji in defaults {
            if !result.contains(&emoji.to_string()) && result.len() < 20 {
                result.push(emoji.to_string());
            }
        }
    } else {
        log::info!("Found {} frequently used emojis", result.len());
    }

    Ok(web::Json(result))
}

/// Get all custom emojis available on the site
#[get("/api/custom-emojis")]
pub async fn get_custom_emojis(app_config: web::Data<Config>) -> Result<impl Responder> {
    use std::fs;

    #[derive(Serialize)]
    struct SimpleEmoji {
        name: String,
        filename: String,
    }

    let emojis_dir = app_config.emoji_dir.clone();
    let mut emojis = Vec::new();

    if let Ok(entries) = fs::read_dir(&emojis_dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                // Only include image files
                if filename.ends_with(".png")
                    || filename.ends_with(".gif")
                    || filename.ends_with(".jpg")
                    || filename.ends_with(".webp")
                {
                    // Remove file extension to get name
                    let name = filename
                        .rsplit_once('.')
                        .map(|(name, _)| name)
                        .unwrap_or(filename)
                        .to_string();
                    emojis.push(SimpleEmoji {
                        name: name.clone(),
                        filename: filename.to_string(),
                    });
                }
            }
        }
    }

    // Sort by name
    emojis.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(HttpResponse::Ok().json(emojis))
}

/// Admin-only upload of a custom emoji (PNG or GIF)
#[post("/admin/upload-emoji")]
pub async fn upload_emoji(
    session: Session,
    app_config: web::Data<Config>,
    mut payload: Multipart,
) -> Result<impl Responder> {
    // Require admin
    let did = match session.get::<String>("did").unwrap_or(None) {
        Some(d) => d,
        None => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Not authenticated"
            })));
        }
    };
    if !is_admin(&did) {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Admin access required"
        })));
    }

    // Parse multipart for optional name and the file
    let mut desired_name: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_ext: Option<&'static str> = None; // "png" | "gif"

    const MAX_SIZE: usize = 5 * 1024 * 1024; // 5MB cap

    loop {
        let mut field = match payload.try_next().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => {
                log::warn!("multipart error: {}", e);
                return Ok(HttpResponse::BadRequest()
                    .json(serde_json::json!({"error":"Invalid multipart data"})));
            }
        };
        let name = field.name().to_string();

        if name == "name" {
            // Collect small text field
            let mut buf = Vec::new();
            loop {
                match field.try_next().await {
                    Ok(Some(chunk)) => {
                        buf.extend_from_slice(&chunk);
                        if buf.len() > 1024 {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        log::warn!("multipart read error: {}", e);
                        return Ok(HttpResponse::BadRequest()
                            .json(serde_json::json!({"error":"Invalid multipart data"})));
                    }
                }
            }
            if let Ok(s) = String::from_utf8(buf) {
                desired_name = Some(s.trim().to_string());
            }
            continue;
        }

        if name == "file" {
            let ct = field.content_type().cloned();
            let mut ext_guess: Option<&'static str> = match ct.as_ref().map(|m| m.essence_str()) {
                Some("image/png") => Some("png"),
                Some("image/gif") => Some("gif"),
                _ => None,
            };

            // Read file bytes with size cap
            let mut data = Vec::new();
            loop {
                match field.try_next().await {
                    Ok(Some(chunk)) => {
                        data.extend_from_slice(&chunk);
                        if data.len() > MAX_SIZE {
                            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                                "error": "File too large (max 5MB)"
                            })));
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        log::warn!("file read error: {}", e);
                        return Ok(HttpResponse::BadRequest()
                            .json(serde_json::json!({"error":"Invalid file upload"})));
                    }
                }
            }

            // If content-type was ambiguous, try to infer from magic bytes
            if ext_guess.is_none() && data.len() >= 4 {
                if data.starts_with(&[0x89, b'P', b'N', b'G']) {
                    ext_guess = Some("png");
                } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
                    ext_guess = Some("gif");
                }
            }

            if ext_guess.is_none() {
                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Unsupported file type (only PNG or GIF)"
                })));
            }

            file_ext = ext_guess;
            file_bytes = Some(data);
        }
    }

    let data = match file_bytes {
        Some(d) => d,
        None => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Missing file field"
            })));
        }
    };
    let ext = file_ext.unwrap_or("png");

    // Sanitize/derive filename base
    let base = desired_name
        .as_ref()
        .cloned()
        .unwrap_or_else(|| format!("emoji_{}", chrono::Utc::now().timestamp()));
    let mut safe: String = base
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if safe.is_empty() {
        safe = "emoji".to_string();
    }
    let mut filename = format!("{}.{}", safe.to_lowercase(), ext);

    // Ensure directory exists and avoid overwrite
    let dir = std::path::Path::new(&app_config.emoji_dir);
    if let Err(e) = std::fs::create_dir_all(dir) {
        log::error!("Failed to create emoji dir {}: {}", app_config.emoji_dir, e);
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Filesystem error"
        })));
    }

    // If user provided a name explicitly and it conflicts with a builtin emoji slug, reject
    if desired_name.is_some() && is_builtin_slug(&safe.to_lowercase()).await {
        return Ok(HttpResponse::Conflict().json(serde_json::json!({
            "error": "Name is reserved by a standard emoji.",
            "code": "name_exists",
            "name": safe.to_lowercase(),
        })));
    }

    // If user provided a name explicitly and that base already exists with any supported
    // extension, reject with a clear error so the UI can prompt to choose a different name.
    if desired_name.is_some() {
        let png_path = dir.join(format!("{}.png", safe.to_lowercase()));
        let gif_path = dir.join(format!("{}.gif", safe.to_lowercase()));
        if png_path.exists() || gif_path.exists() {
            return Ok(HttpResponse::Conflict().json(serde_json::json!({
                "error": "Name already exists. Choose a different name.",
                "code": "name_exists",
                "name": safe.to_lowercase(),
            })));
        }
    }

    let mut path = dir.join(&filename);
    if path.exists() {
        // Only auto-deconflict when name wasn't provided explicitly
        if desired_name.is_none() {
            for i in 1..1000 {
                filename = format!("{}-{}.{}", safe.to_lowercase(), i, ext);
                path = dir.join(&filename);
                if !path.exists() {
                    break;
                }
            }
        } else {
            return Ok(HttpResponse::Conflict().json(serde_json::json!({
                "error": "Name already exists. Choose a different name.",
                "code": "name_exists",
                "name": safe.to_lowercase(),
            })));
        }
    }

    if let Err(e) = std::fs::write(&path, &data) {
        log::error!("Failed to save emoji to {:?}: {}", path, e);
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Write failed"
        })));
    }

    let url = format!("/emojis/{}", filename);
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "filename": filename,
        "url": url
    })))
}

/// Get the DIDs of accounts the logged-in user follows
#[get("/api/following")]
pub async fn get_following(
    session: Session,
    _oauth_client: web::Data<OAuthClientType>,
) -> Result<impl Responder> {
    // Check if user is logged in
    let did = match session.get::<Did>("did").ok().flatten() {
        Some(did) => did,
        None => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Not logged in"
            })));
        }
    };

    // WORKAROUND: Call public API directly for getFollows since OAuth scope isn't working
    // Both getProfile and getFollows are public endpoints that don't require auth
    // but when called through OAuth, getFollows requires a scope that doesn't exist yet

    let mut all_follows = Vec::new();
    let mut cursor: Option<String> = None;

    // Use reqwest to call the public API directly
    let client = reqwest::Client::new();

    loop {
        let mut url = format!(
            "https://public.api.bsky.app/xrpc/app.bsky.graph.getFollows?actor={}",
            did.as_str()
        );

        if let Some(c) = &cursor {
            url.push_str(&format!("&cursor={}", c));
        }

        match client.get(&url).send().await {
            Ok(response) => {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        // Extract follows
                        if let Some(follows) = json["follows"].as_array() {
                            for follow in follows {
                                if let Some(did_str) = follow["did"].as_str() {
                                    all_follows.push(did_str.to_string());
                                }
                            }
                        }

                        // Check for cursor
                        cursor = json["cursor"].as_str().map(|s| s.to_string());
                        if cursor.is_none() {
                            break;
                        }
                    }
                    Err(err) => {
                        log::error!("Failed to parse follows response: {}", err);
                        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": "Failed to parse follows"
                        })));
                    }
                }
            }
            Err(err) => {
                log::error!("Failed to fetch follows from public API: {}", err);
                return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to fetch follows"
                })));
            }
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "follows": all_follows
    })))
}

/// Clear the user's status by deleting the ATProto record
#[post("/status/clear")]
pub async fn clear_status(
    request: HttpRequest,
    session: Session,
    oauth_client: web::Data<OAuthClientType>,
    db_pool: web::Data<Arc<Pool>>,
) -> HttpResponse {
    // Check if the user is logged in
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            let did = Did::new(did_string.clone()).expect("failed to parse did");

            // Get the user's current status to find the record key
            match StatusFromDb::my_status(&db_pool, &did).await {
                Ok(Some(current_status)) => {
                    // Extract the record key from the URI
                    // URI format: at://did:plc:xxx/io.zzstoatzz.status.record/rkey
                    let parts: Vec<&str> = current_status.uri.split('/').collect();
                    if let Some(rkey) = parts.last() {
                        // Get OAuth session
                        match oauth_client.restore(&did).await {
                            Ok(session) => {
                                let agent = Agent::new(session);

                                // Delete the record from ATProto using com.atproto.repo.deleteRecord
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
                                        // Also remove from local database
                                        let _ = StatusFromDb::delete_by_uri(
                                            &db_pool,
                                            current_status.uri.clone(),
                                        )
                                        .await;

                                        // Fire webhooks asynchronously for cleared status
                                        {
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
                                        }

                                        Redirect::to("/")
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
                Ok(None) => {
                    // No status to clear
                    Redirect::to("/")
                        .see_other()
                        .respond_to(&request)
                        .map_into_boxed_body()
                }
                Err(e) => {
                    log::error!("Database error: {e}");
                    HttpResponse::InternalServerError().body("Database error")
                }
            }
        }
        None => {
            // Not logged in
            Redirect::to("/login")
                .see_other()
                .respond_to(&request)
                .map_into_boxed_body()
        }
    }
}

/// Delete a specific status by URI (JSON endpoint)
#[post("/status/delete")]
pub async fn delete_status(
    session: Session,
    oauth_client: web::Data<OAuthClientType>,
    db_pool: web::Data<Arc<Pool>>,
    req: web::Json<DeleteRequest>,
) -> HttpResponse {
    // Check if the user is logged in
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            let did = Did::new(did_string.clone()).expect("failed to parse did");

            // Parse the URI to verify it belongs to this user
            // URI format: at://did:plc:xxx/io.zzstoatzz.status.record/rkey
            let uri_parts: Vec<&str> = req.uri.split('/').collect();
            if uri_parts.len() < 5 {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Invalid status URI format"
                }));
            }

            // Extract DID from URI (at://did:plc:xxx/...)
            let uri_did_part = uri_parts[2];
            if uri_did_part != did_string {
                return HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "You can only delete your own statuses"
                }));
            }

            // Extract record key
            if let Some(rkey) = uri_parts.last() {
                // Get OAuth session
                match oauth_client.restore(&did).await {
                    Ok(session) => {
                        let agent = Agent::new(session);

                        // Delete the record from ATProto
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
                                // Also remove from local database
                                let _ =
                                    StatusFromDb::delete_by_uri(&db_pool, req.uri.clone()).await;

                                // Fire webhooks asynchronously for deleted status
                                {
                                    let pool = db_pool.get_ref().clone();
                                    let did_for_event = did_string.clone();
                                    let uri = req.uri.clone();
                                    tokio::spawn(async move {
                                        crate::webhooks::emit_deleted(pool, &did_for_event, &uri)
                                            .await;
                                    });
                                }

                                HttpResponse::Ok().json(serde_json::json!({
                                    "success": true
                                }))
                            }
                            Err(e) => {
                                log::error!("Failed to delete status from ATProto: {e}");
                                HttpResponse::InternalServerError().json(serde_json::json!({
                                    "error": "Failed to delete status"
                                }))
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to restore OAuth session: {e}");
                        HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": "Session error"
                        }))
                    }
                }
            } else {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "Invalid status URI"
                }))
            }
        }
        None => {
            // Not logged in
            HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Not authenticated"
            }))
        }
    }
}

/// Hide/unhide a status (admin only)
#[post("/admin/hide-status")]
pub async fn hide_status(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    req: web::Json<HideStatusRequest>,
) -> HttpResponse {
    // Check if the user is logged in and is admin
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            if !is_admin(&did_string) {
                return HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "Admin access required"
                }));
            }

            // Update the hidden status in the database
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
                Ok(rows_affected) if rows_affected > 0 => {
                    HttpResponse::Ok().json(serde_json::json!({
                        "success": true,
                        "message": if hidden { "Status hidden" } else { "Status unhidden" }
                    }))
                }
                Ok(_) => HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Status not found"
                })),
                Err(err) => {
                    log::error!("Error updating hidden status: {}", err);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Database error"
                    }))
                }
            }
        }
        None => HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Not authenticated"
        })),
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
    // Apply rate limiting
    let client_key = RateLimiter::get_client_key(&request);
    if !rate_limiter.check_rate_limit(&client_key) {
        return Err(AppError::RateLimitExceeded);
    }
    // Check if the user is logged in
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            let did = Did::new(did_string.clone()).expect("failed to parse did");
            // gets the user's session from the session store to resume
            match oauth_client.restore(&did).await {
                Ok(session) => {
                    let agent = Agent::new(session);

                    // Calculate expiration time if provided
                    let expires = form
                        .expires_in
                        .as_ref()
                        .and_then(|exp| parse_duration(exp))
                        .and_then(|duration| {
                            let expiry_time = chrono::Utc::now() + duration;
                            // Convert to ATProto Datetime format (RFC3339)
                            Some(Datetime::new(expiry_time.to_rfc3339().parse().ok()?))
                        });

                    //Creates a strongly typed ATProto record
                    let status: KnownRecord =
                        crate::lexicons::io::zzstoatzz::status::record::RecordData {
                            created_at: Datetime::now(),
                            emoji: form.status.clone(),
                            text: form.text.clone(),
                            expires,
                        }
                        .into();

                    // TODO no data validation yet from esquema
                    // Maybe you'd like to add it? https://github.com/fatfingers23/esquema/issues/3

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

                            // Set the text field if provided
                            status.text = form.text.clone();

                            // Set the expiration time if provided
                            if let Some(exp_str) = &form.expires_in {
                                if let Some(duration) = parse_duration(exp_str) {
                                    status.expires_at = Some(chrono::Utc::now() + duration);
                                }
                            }

                            let _ = status.save(db_pool.clone()).await;

                            // Fire webhooks asynchronously (helper keeps this file lean)
                            {
                                let pool = db_pool.get_ref().clone();
                                let s = status.clone();
                                tokio::spawn(async move {
                                    crate::webhooks::emit_created(pool, &s).await;
                                });
                            }
                            Ok(Redirect::to("/")
                                .see_other()
                                .respond_to(&request)
                                .map_into_boxed_body())
                        }
                        Err(err) => {
                            log::error!("Error creating status: {err}");
                            let error_html = ErrorTemplate {
                                title: "Error",
                                error: "Was an error creating the status, please check the logs.",
                            }
                            .render()
                            .expect("template should be valid");
                            Ok(HttpResponse::Ok().body(error_html))
                        }
                    }
                }
                Err(err) => {
                    // Destroys the system or you're in a loop
                    session.purge();
                    log::error!(
                        "Error restoring session, we are removing the session from the cookie: {err}"
                    );
                    Err(AppError::AuthenticationError("Session error".to_string()))
                }
            }
        }
        None => Err(AppError::AuthenticationError(
            "You must be logged in to create a status.".to_string(),
        )),
    }
}
