#![allow(clippy::collapsible_if)]

use crate::{
    db::{StatusFromDb, WebhookConfig, create_tables_in_database},
    error_handler::AppError,
    ingester::start_ingester,
    lexicons::record::KnownRecord,
    rate_limiter::RateLimiter,
    storage::{SqliteSessionStore, SqliteStateStore},
    templates::{FeedTemplate, LoginTemplate, SettingsTemplate, StatusTemplate},
};
use actix_files::Files;
use actix_session::{
    Session, SessionMiddleware, config::PersistentSession, storage::CookieSessionStore,
};
use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer, Responder, Result,
    cookie::{self, Key},
    get, middleware, post,
    web::{self, Redirect},
};
use askama::Template;
use async_sqlite::rusqlite;
use async_sqlite::{Pool, PoolBuilder};
use atrium_api::{
    agent::Agent,
    types::string::{Datetime, Did},
};
use atrium_common::resolver::Resolver;
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig},
};
use atrium_oauth::{
    AtprotoClientMetadata, AtprotoLocalhostClientMetadata, AuthMethod, AuthorizeOptions,
    CallbackParams, DefaultHttpClient, GrantType, KnownScope, OAuthClient, OAuthClientConfig,
    OAuthResolverConfig, Scope,
};
use dotenv::dotenv;
use resolver::HickoryDnsTxtResolver;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Error, sync::Arc, time::Duration};
use templates::{ErrorTemplate, Profile};

mod config;
mod db;
mod dev_utils;
mod error_handler;
mod ingester;
#[allow(dead_code)]
mod lexicons;
mod rate_limiter;
mod resolver;
mod storage;
mod templates;
mod webhooks;

/// OAuthClientType to make it easier to access the OAuthClient in web requests
/// Custom OAuth callback parameters that can handle both success and error cases
#[derive(Debug, Deserialize)]
struct OAuthCallbackParams {
    state: Option<String>,
    iss: Option<String>,
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

type OAuthClientType = Arc<
    OAuthClient<
        SqliteStateStore,
        SqliteSessionStore,
        CommonDidResolver<DefaultHttpClient>,
        AtprotoHandleResolver<HickoryDnsTxtResolver, DefaultHttpClient>,
    >,
>;

/// HandleResolver to make it easier to access the OAuthClient in web requests
type HandleResolver = Arc<CommonDidResolver<DefaultHttpClient>>;

/// Admin DID for moderation
const ADMIN_DID: &str = "did:plc:xbtmt2zjwlrfegqvch7fboei"; // zzstoatzz.io

/// Check if a DID is the admin
fn is_admin(did: &str) -> bool {
    did == ADMIN_DID
}

/// OAuth client metadata endpoint for production
#[get("/client-metadata.json")]
async fn client_metadata(config: web::Data<config::Config>) -> Result<HttpResponse> {
    let public_url = config.oauth_redirect_base.clone();

    let metadata = serde_json::json!({
        "client_id": format!("{}/client-metadata.json", public_url),
        "client_name": "Status Sphere",
        "client_uri": public_url.clone(),
        "redirect_uris": [format!("{}/oauth/callback", public_url)],
        "scope": "atproto transition:generic",
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
        "dpop_bound_access_tokens": true
    });

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(metadata.to_string()))
}

/// All the available emoji status options
const STATUS_OPTIONS: [&str; 29] = [
    "👍",
    "👎",
    "💙",
    "🥹",
    "😧",
    "😤",
    "🙃",
    "😉",
    "😎",
    "🤓",
    "🤨",
    "🥳",
    "😭",
    "😤",
    "🤯",
    "🫡",
    "💀",
    "✊",
    "🤘",
    "👀",
    "🧠",
    "👩‍💻",
    "🧑‍💻",
    "🥷",
    "🧌",
    "🦋",
    "🚀",
    "🥔",
    "🦀",
];

/// TS version https://github.com/bluesky-social/statusphere-example-app/blob/e4721616df50cd317c198f4c00a4818d5626d4ce/src/routes.ts#L71
/// OAuth callback endpoint to complete session creation
#[get("/oauth/callback")]
async fn oauth_callback(
    request: HttpRequest,
    params: web::Query<OAuthCallbackParams>,
    oauth_client: web::Data<OAuthClientType>,
    session: Session,
) -> HttpResponse {
    // Check if there's an OAuth error from BlueSky
    if let Some(error) = &params.error {
        let error_msg = params
            .error_description
            .as_deref()
            .unwrap_or("An error occurred during authentication");
        log::error!("OAuth error from BlueSky: {} - {}", error, error_msg);

        let html = ErrorTemplate {
            title: "Authentication Error",
            error: error_msg,
        };
        return HttpResponse::BadRequest().body(html.render().expect("template should be valid"));
    }

    // Check if we have the required code field for a successful callback
    let code = match &params.code {
        Some(code) => code.clone(),
        None => {
            log::error!("OAuth callback missing required code parameter");
            let html = ErrorTemplate {
                title: "Error",
                error: "Missing required OAuth code. Please try logging in again.",
            };
            return HttpResponse::BadRequest()
                .body(html.render().expect("template should be valid"));
        }
    };

    // Create CallbackParams for the OAuth client
    let callback_params = CallbackParams {
        code,
        state: params.state.clone(),
        iss: params.iss.clone(),
    };

    //Processes the call back and parses out a session if found and valid
    match oauth_client.callback(callback_params).await {
        Ok((bsky_session, _)) => {
            let agent = Agent::new(bsky_session);
            match agent.did().await {
                Some(did) => {
                    session.insert("did", did).unwrap();
                    Redirect::to("/")
                        .see_other()
                        .respond_to(&request)
                        .map_into_boxed_body()
                }
                None => {
                    let html = ErrorTemplate {
                        title: "Error",
                        error: "The OAuth agent did not return a DID. May try re-logging in.",
                    };
                    HttpResponse::Ok().body(html.render().expect("template should be valid"))
                }
            }
        }
        Err(err) => {
            log::error!("Error: {err}");
            let html = ErrorTemplate {
                title: "Error",
                error: "OAuth error, check the logs",
            };
            HttpResponse::Ok().body(html.render().expect("template should be valid"))
        }
    }
}

/// TS version https://github.com/bluesky-social/statusphere-example-app/blob/e4721616df50cd317c198f4c00a4818d5626d4ce/src/routes.ts#L93
/// Takes you to the login page
#[get("/login")]
async fn login() -> Result<impl Responder> {
    let html = LoginTemplate {
        title: "Log in",
        error: None,
    };
    Ok(web::Html::new(
        html.render().expect("template should be valid"),
    ))
}

/// Settings page (owner only) for configuring outbound webhook
#[get("/settings")]
async fn settings_page(session: Session, db_pool: web::Data<Arc<Pool>>) -> Result<impl Responder> {
    if let Some(did) = session.get::<String>("did").unwrap_or(None) {
        let cfg = WebhookConfig::get_by_did(&db_pool, did)
            .await
            .unwrap_or(None);
        let html = SettingsTemplate {
            title: "Settings",
            webhook_url: cfg.as_ref().map(|c| c.url.clone()),
            webhook_enabled: cfg.as_ref().map(|c| c.enabled).unwrap_or(false),
        }
        .render()
        .expect("template should be valid");
        Ok(web::Html::new(html))
    } else {
        Ok(web::Html::new(
            ErrorTemplate {
                title: "Unauthorized",
                error: "Please log in",
            }
            .render()
            .unwrap(),
        ))
    }
}

#[derive(Deserialize)]
struct WebhookForm {
    url: String,
    secret: String,
    enabled: Option<String>,
}

/// Save webhook settings
#[post("/settings/webhook")]
async fn save_webhook(
    session: Session,
    db_pool: web::Data<Arc<Pool>>,
    form: web::Form<WebhookForm>,
) -> Result<impl Responder> {
    if let Some(did) = session.get::<String>("did").unwrap_or(None) {
        let cfg = WebhookConfig {
            user_did: did,
            url: form.url.clone(),
            secret: form.secret.clone(),
            enabled: form.enabled.as_deref() == Some("true"),
        };
        let _ = cfg.upsert(&db_pool).await;
        Ok(Redirect::to("/settings").see_other())
    } else {
        Ok(Redirect::to("/login").see_other())
    }
}

/// Send a test webhook event to verify receiver
#[post("/settings/webhook/test")]
async fn test_webhook(session: Session, db_pool: web::Data<Arc<Pool>>) -> Result<impl Responder> {
    if let Some(did) = session.get::<String>("did").unwrap_or(None) {
        // Construct and send a minimal test event
        webhooks::send_status_event(&db_pool, &did, "status.test", None).await;
        Ok(Redirect::to("/settings").see_other())
    } else {
        Ok(Redirect::to("/login").see_other())
    }
}

/// TS version https://github.com/bluesky-social/statusphere-example-app/blob/e4721616df50cd317c198f4c00a4818d5626d4ce/src/routes.ts#L93
/// Logs you out by destroying your cookie on the server and web browser
#[get("/logout")]
async fn logout(request: HttpRequest, session: Session) -> HttpResponse {
    session.purge();
    Redirect::to("/")
        .see_other()
        .respond_to(&request)
        .map_into_boxed_body()
}

/// The post body for logging in
#[derive(Serialize, Deserialize, Clone)]
struct LoginForm {
    handle: String,
}

/// TS version https://github.com/bluesky-social/statusphere-example-app/blob/e4721616df50cd317c198f4c00a4818d5626d4ce/src/routes.ts#L101
/// Login endpoint
#[post("/login")]
async fn login_post(
    request: HttpRequest,
    params: web::Form<LoginForm>,
    oauth_client: web::Data<OAuthClientType>,
) -> HttpResponse {
    // This will act the same as the js method isValidHandle to make sure it is valid
    match atrium_api::types::string::Handle::new(params.handle.clone()) {
        Ok(handle) => {
            //Creates the oauth url to redirect to for the user to log in with their credentials
            let oauth_url = oauth_client
                .authorize(
                    &handle,
                    AuthorizeOptions {
                        scopes: vec![
                            Scope::Known(KnownScope::Atproto),
                            Scope::Known(KnownScope::TransitionGeneric),
                        ],
                        ..Default::default()
                    },
                )
                .await;
            match oauth_url {
                Ok(url) => Redirect::to(url)
                    .see_other()
                    .respond_to(&request)
                    .map_into_boxed_body(),
                Err(err) => {
                    log::error!("Error: {err}");
                    let html = LoginTemplate {
                        title: "Log in",
                        error: Some("OAuth error"),
                    };
                    HttpResponse::Ok().body(html.render().expect("template should be valid"))
                }
            }
        }
        Err(err) => {
            let html: LoginTemplate<'_> = LoginTemplate {
                title: "Log in",
                error: Some(err),
            };
            HttpResponse::Ok().body(html.render().expect("template should be valid"))
        }
    }
}

/// Homepage - shows logged-in user's status, or owner's status if not logged in
#[get("/")]
async fn home(
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

            let html = StatusTemplate {
                title: "your status",
                handle,
                status_options: &STATUS_OPTIONS,
                current_status,
                history,
                is_owner: true, // They're viewing their own status
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
                status_options: &STATUS_OPTIONS,
                current_status,
                history,
                is_owner: false, // Visitor viewing owner's status
            }
            .render()
            .expect("template should be valid");

            Ok(web::Html::new(html))
        }
    }
}

/// View a specific user's status page by handle
#[get("/@{handle}")]
async fn user_status_page(
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

    let html = StatusTemplate {
        title: &format!("@{} status", handle),
        handle: handle.clone(),
        status_options: &STATUS_OPTIONS,
        current_status,
        history,
        is_owner,
    }
    .render()
    .expect("template should be valid");

    Ok(web::Html::new(html))
}

/// JSON API for the owner's status (top-level endpoint)
#[get("/json")]
async fn owner_status_json(
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

/// Get paginated statuses for infinite scrolling
#[get("/api/feed")]
async fn api_feed(
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

/// Get all custom emojis available on the site
#[get("/api/custom-emojis")]
async fn get_custom_emojis() -> Result<impl Responder> {
    use std::fs;

    #[derive(Serialize)]
    struct SimpleEmoji {
        name: String,
        filename: String,
    }

    let emojis_dir = "static/emojis";
    let mut emojis = Vec::new();

    if let Ok(entries) = fs::read_dir(emojis_dir) {
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

/// Get the DIDs of accounts the logged-in user follows
#[get("/api/following")]
async fn get_following(
    session: Session,
    oauth_client: web::Data<OAuthClientType>,
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

    // Restore OAuth session
    let bsky_session = match oauth_client.restore(&did).await {
        Ok(session) => session,
        Err(err) => {
            log::error!("Failed to restore session: {}", err);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to restore session"
            })));
        }
    };

    let agent = Agent::new(bsky_session);

    // Fetch follows from Bluesky
    let mut all_follows = Vec::new();
    let mut cursor = None;

    loop {
        let params = atrium_api::app::bsky::graph::get_follows::ParametersData {
            actor: atrium_api::types::string::AtIdentifier::Did(did.clone()),
            limit: None, // Use default limit
            cursor: cursor.clone(),
        };

        match agent.api.app.bsky.graph.get_follows(params.into()).await {
            Ok(response) => {
                // Extract DIDs from the follows
                for follow in &response.follows {
                    all_follows.push(follow.did.to_string());
                }

                // Check if there are more pages
                cursor = response.cursor.clone();
                if cursor.is_none() {
                    break;
                }
            }
            Err(err) => {
                log::error!("Failed to fetch follows: {}", err);
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

/// JSON API for a specific user's status
#[get("/@{handle}/json")]
async fn user_status_json(
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
async fn status_json(db_pool: web::Data<Arc<Pool>>) -> Result<impl Responder> {
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
async fn feed(
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
        Some(did) => {
            let did = Did::new(did).expect("failed to parse did");
            let _my_status = StatusFromDb::my_status(&db_pool, &did)
                .await
                .unwrap_or_else(|err| {
                    log::error!("Error loading my status: {err}");
                    None
                });

            match oauth_client.restore(&did).await {
                Ok(session) => {
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
                    session.purge();
                    log::error!("Error restoring session: {err}");
                    let error_html = ErrorTemplate {
                        title: "Error",
                        error: "Was an error resuming the session, please check the logs.",
                    }
                    .render()
                    .expect("template should be valid");
                    Ok(web::Html::new(error_html))
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

/// The post body for changing your status
#[derive(Serialize, Deserialize, Clone)]
struct StatusForm {
    status: String,
    text: Option<String>,
    expires_in: Option<String>, // e.g., "1h", "30m", "1d", etc.
}

/// The post body for deleting a specific status
#[derive(Serialize, Deserialize)]
struct DeleteRequest {
    uri: String,
}

/// Parse duration string like "1h", "30m", "1d" into chrono::Duration
fn parse_duration(duration_str: &str) -> Option<chrono::Duration> {
    if duration_str.is_empty() {
        return None;
    }

    let (num_str, unit) = duration_str.split_at(duration_str.len() - 1);
    let num: i64 = num_str.parse().ok()?;

    match unit {
        "m" => Some(chrono::Duration::minutes(num)),
        "h" => Some(chrono::Duration::hours(num)),
        "d" => Some(chrono::Duration::days(num)),
        "w" => Some(chrono::Duration::weeks(num)),
        _ => None,
    }
}

/// Clear the user's status by deleting the ATProto record
#[post("/status/clear")]
async fn clear_status(
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
                                            current_status.uri,
                                        )
                                        .await;

                                        // Emit webhook: status.cleared
                                        let did_for_wh = did_string.clone();
                                        let pool_for_wh = db_pool.clone();
                                        tokio::spawn(async move {
                                            webhooks::send_status_event(
                                                &pool_for_wh,
                                                &did_for_wh,
                                                "status.cleared",
                                                None,
                                            )
                                            .await;
                                        });

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
async fn delete_status(
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

                                // Emit webhook: status.cleared
                                let did_for_wh = did_string.clone();
                                let pool_for_wh = db_pool.clone();
                                tokio::spawn(async move {
                                    webhooks::send_status_event(
                                        &pool_for_wh,
                                        &did_for_wh,
                                        "status.cleared",
                                        None,
                                    )
                                    .await;
                                });

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
#[derive(Deserialize)]
struct HideStatusRequest {
    uri: String,
    hidden: bool,
}

#[post("/admin/hide-status")]
async fn hide_status(
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

/// TS version https://github.com/bluesky-social/statusphere-example-app/blob/e4721616df50cd317c198f4c00a4818d5626d4ce/src/routes.ts#L208
/// Creates a new status
#[post("/status")]
async fn status(
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
                    let status: KnownRecord = lexicons::io::zzstoatzz::status::record::RecordData {
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
                                did_string.clone(),
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

                            // Emit webhook asynchronously: status.set
                            let did_for_wh = did_string.clone();
                            let pool_for_wh = db_pool.clone();
                            let status_for_wh = status.clone();
                            tokio::spawn(async move {
                                webhooks::send_status_event(
                                    &pool_for_wh,
                                    &did_for_wh,
                                    "status.set",
                                    Some(&status_for_wh),
                                )
                                .await;
                            });
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    // Load configuration
    let config = config::Config::from_env().expect("Failed to load configuration");
    let app_config = config.clone();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or(&config.log_level));
    let host = config.server_host.clone();
    let port = config.server_port;

    // Use database URL from config
    let db_connection_string = if config.database_url.starts_with("sqlite://") {
        config
            .database_url
            .strip_prefix("sqlite://")
            .unwrap_or(&config.database_url)
            .to_string()
    } else {
        config.database_url.clone()
    };

    //Crates a db pool to share resources to the db
    let pool = match PoolBuilder::new().path(db_connection_string).open().await {
        Ok(pool) => pool,
        Err(err) => {
            log::error!("Error creating the sqlite pool: {}", err);
            return Err(Error::other("sqlite pool could not be created."));
        }
    };

    //Creates the DB and tables
    create_tables_in_database(&pool)
        .await
        .expect("Could not create the database");

    //Create a new handle resolver for the home page
    let http_client = Arc::new(DefaultHttpClient::default());

    let handle_resolver = CommonDidResolver::new(CommonDidResolverConfig {
        plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
        http_client: http_client.clone(),
    });
    let handle_resolver = Arc::new(handle_resolver);

    // Create a new OAuth client
    let http_client = Arc::new(DefaultHttpClient::default());

    // Check if we're running in production (non-localhost) or locally
    let is_production = !config.oauth_redirect_base.starts_with("http://localhost")
        && !config.oauth_redirect_base.starts_with("http://127.0.0.1");

    let client: OAuthClientType = if is_production {
        // Production configuration with AtprotoClientMetadata
        log::info!(
            "Configuring OAuth for production with URL: {}",
            config.oauth_redirect_base
        );

        let oauth_config = OAuthClientConfig {
            client_metadata: AtprotoClientMetadata {
                client_id: format!("{}/client-metadata.json", config.oauth_redirect_base),
                client_uri: Some(config.oauth_redirect_base.clone()),
                redirect_uris: vec![format!("{}/oauth/callback", config.oauth_redirect_base)],
                token_endpoint_auth_method: AuthMethod::None,
                grant_types: vec![GrantType::AuthorizationCode, GrantType::RefreshToken],
                scopes: vec![
                    Scope::Known(KnownScope::Atproto),
                    Scope::Known(KnownScope::TransitionGeneric),
                ],
                jwks_uri: None,
                token_endpoint_auth_signing_alg: None,
            },
            keys: None,
            resolver: OAuthResolverConfig {
                did_resolver: CommonDidResolver::new(CommonDidResolverConfig {
                    plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
                    http_client: http_client.clone(),
                }),
                handle_resolver: AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
                    dns_txt_resolver: HickoryDnsTxtResolver::default(),
                    http_client: http_client.clone(),
                }),
                authorization_server_metadata: Default::default(),
                protected_resource_metadata: Default::default(),
            },
            state_store: SqliteStateStore::new(pool.clone()),
            session_store: SqliteSessionStore::new(pool.clone()),
        };
        Arc::new(OAuthClient::new(oauth_config).expect("failed to create OAuth client"))
    } else {
        // Local development configuration with AtprotoLocalhostClientMetadata
        log::info!(
            "Configuring OAuth for local development at {}:{}",
            host,
            port
        );

        let oauth_config = OAuthClientConfig {
            client_metadata: AtprotoLocalhostClientMetadata {
                redirect_uris: Some(vec![format!(
                    //This must match the endpoint you use the callback function
                    "http://{host}:{port}/oauth/callback"
                )]),
                scopes: Some(vec![
                    Scope::Known(KnownScope::Atproto),
                    Scope::Known(KnownScope::TransitionGeneric),
                ]),
            },
            keys: None,
            resolver: OAuthResolverConfig {
                did_resolver: CommonDidResolver::new(CommonDidResolverConfig {
                    plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
                    http_client: http_client.clone(),
                }),
                handle_resolver: AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
                    dns_txt_resolver: HickoryDnsTxtResolver::default(),
                    http_client: http_client.clone(),
                }),
                authorization_server_metadata: Default::default(),
                protected_resource_metadata: Default::default(),
            },
            state_store: SqliteStateStore::new(pool.clone()),
            session_store: SqliteSessionStore::new(pool.clone()),
        };
        Arc::new(OAuthClient::new(oauth_config).expect("failed to create OAuth client"))
    };
    // Only start the firehose ingester if enabled (from config)
    if app_config.enable_firehose {
        let arc_pool = Arc::new(pool.clone());
        log::info!("Starting Jetstream firehose ingester");
        //Spawns the ingester that listens for other's Statusphere updates
        tokio::spawn(async move {
            start_ingester(arc_pool).await;
        });
    } else {
        log::info!("Jetstream firehose disabled (set ENABLE_FIREHOSE=true to enable)");
    }
    let arc_pool = Arc::new(pool.clone());

    // Create rate limiter - 30 requests per minute per IP
    let rate_limiter = web::Data::new(RateLimiter::new(30, Duration::from_secs(60)));

    log::info!("starting HTTP server at http://{host}:{port}");
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(client.clone()))
            .app_data(web::Data::new(arc_pool.clone()))
            .app_data(web::Data::new(handle_resolver.clone()))
            .app_data(web::Data::new(app_config.clone()))
            .app_data(rate_limiter.clone())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                    //TODO will need to set to true in production
                    .cookie_secure(false)
                    // customize session and cookie expiration
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(cookie::time::Duration::days(14)),
                    )
                    .build(),
            )
            .service(Files::new("/static", "static").show_files_listing())
            .service(Files::new("/emojis", "static/emojis").show_files_listing())
            .service(client_metadata)
            .service(oauth_callback)
            .service(login)
            .service(login_post)
            .service(logout)
            .service(settings_page)
            .service(save_webhook)
            .service(test_webhook)
            .service(home)
            .service(feed)
            .service(status_json)
            .service(owner_status_json)
            .service(get_custom_emojis)
            .service(get_following)
            .service(api_feed)
            .service(user_status_page)
            .service(user_status_json)
            .service(status)
            .service(clear_status)
            .service(delete_status)
            .service(hide_status)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};

    #[actix_web::test]
    async fn test_health_check() {
        // Simple test to verify our test infrastructure works
        assert_eq!(2 + 2, 4);
    }

    #[actix_web::test]
    async fn test_custom_emojis_endpoint() {
        // Test that the custom emojis endpoint returns JSON
        let app = test::init_service(App::new().service(get_custom_emojis)).await;

        let req = test::TestRequest::get()
            .uri("/api/custom-emojis")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_rate_limiting() {
        // Simple test of the rate limiter directly
        let rate_limiter = RateLimiter::new(3, Duration::from_secs(60));

        // Should allow first 3 requests from same IP
        for i in 0..3 {
            assert!(
                rate_limiter.check_rate_limit("test_ip"),
                "Request {} should be allowed",
                i + 1
            );
        }

        // 4th request should be blocked
        assert!(
            !rate_limiter.check_rate_limit("test_ip"),
            "4th request should be blocked"
        );

        // Different IP should have its own limit
        assert!(
            rate_limiter.check_rate_limit("different_ip"),
            "Different IP should have its own rate limit"
        );
    }

    #[actix_web::test]
    async fn test_error_handling() {
        use crate::error_handler::AppError;
        use actix_web::{ResponseError, http::StatusCode};

        // Test that our error types return correct status codes
        let err = AppError::ValidationError("test".to_string());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);

        let err = AppError::RateLimitExceeded;
        assert_eq!(err.status_code(), StatusCode::TOO_MANY_REQUESTS);

        let err = AppError::AuthenticationError("test".to_string());
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }
}
