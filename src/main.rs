#![allow(clippy::collapsible_if)]

use crate::resolver::HickoryDnsTxtResolver;
use crate::{
    api::{HandleResolver, OAuthClientType},
    db::create_tables_in_database,
    ingester::start_ingester,
    rate_limiter::RateLimiter,
    storage::{SqliteSessionStore, SqliteStateStore},
};
use actix_files::Files;
use actix_session::{SessionMiddleware, config::PersistentSession, storage::CookieSessionStore};
use actix_web::{
    App, HttpServer,
    cookie::{self, Key},
    middleware, web,
};
use async_sqlite::PoolBuilder;
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig},
};
use atrium_oauth::{
    AtprotoClientMetadata, AtprotoLocalhostClientMetadata, AuthMethod, DefaultHttpClient,
    GrantType, KnownScope, OAuthClient, OAuthClientConfig, OAuthResolverConfig, Scope,
};
use dotenv::dotenv;
use std::{io::Error, sync::Arc, time::Duration};

mod api;
mod config;
mod db;
mod dev_utils;
mod emoji;
mod error_handler;
mod ingester;
#[allow(dead_code)]
mod lexicons;
mod rate_limiter;
mod resolver;
mod storage;
mod templates;
mod webhooks;

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
    let handle_resolver: HandleResolver = Arc::new(handle_resolver);

    // Create a new OAuth client
    let http_client = Arc::new(DefaultHttpClient::default());

    // Check if we're running in production (non-localhost) or locally
    let is_production = !config.oauth_redirect_base.starts_with("http://localhost")
        && !config.oauth_redirect_base.starts_with("http://127.0.0.1");

    let client: OAuthClientType = if is_production {
        // Production configuration with AtprotoClientMetadata
        log::debug!(
            "Configuring OAuth for production with URL: {}",
            config.oauth_redirect_base
        );

        let oauth_config = OAuthClientConfig {
            client_metadata: AtprotoClientMetadata {
                client_id: format!("{}/oauth-client-metadata.json", config.oauth_redirect_base),
                client_uri: Some(config.oauth_redirect_base.clone()),
                redirect_uris: vec![format!("{}/oauth/callback", config.oauth_redirect_base)],
                token_endpoint_auth_method: AuthMethod::None,
                grant_types: vec![GrantType::AuthorizationCode, GrantType::RefreshToken],
                scopes: vec![
                    Scope::Known(KnownScope::Atproto),
                    // Using granular scope for status records only
                    // This replaces TransitionGeneric with specific permissions
                    Scope::Unknown("repo:io.zzstoatzz.status.record".to_string()),
                    // Need to read profiles for the feed page
                    Scope::Unknown("rpc:app.bsky.actor.getProfile".to_string()),
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
        log::debug!(
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
                    // Using granular scope for status records only
                    // This replaces TransitionGeneric with specific permissions
                    Scope::Unknown("repo:io.zzstoatzz.status.record".to_string()),
                    // Need to read profiles for the feed page
                    Scope::Unknown(
                        "rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview"
                            .to_string(),
                    ),
                    // Need to read following list for following feed
                    Scope::Unknown(
                        "rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app".to_string(),
                    ),
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
        log::debug!("Starting Jetstream firehose ingester");
        //Spawns the ingester that listens for other's Statusphere updates
        tokio::spawn(async move {
            start_ingester(arc_pool).await;
        });
    } else {
        log::debug!("Jetstream firehose disabled (set ENABLE_FIREHOSE=true to enable)");
    }
    let arc_pool = Arc::new(pool.clone());

    // Create rate limiter - 30 requests per minute per IP
    let rate_limiter = web::Data::new(RateLimiter::new(30, Duration::from_secs(60)));

    // Initialize runtime emoji directory (kept out of main for clarity)
    emoji::init_runtime_dir(&config);

    log::debug!("starting HTTP server at http://{host}:{port}");
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
            .service(
                Files::new("/emojis", app_config.emoji_dir.clone())
                    .use_last_modified(true)
                    .use_etag(true)
                    .show_files_listing(),
            )
            .configure(api::configure_routes)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::status_read::{api_feed, feed, get_custom_emojis};
    use actix_web::{App, test};

    #[actix_web::test]
    async fn test_health_check() {
        // Simple test to verify our test infrastructure works
        assert_eq!(2 + 2, 4);
    }

    #[actix_web::test]
    async fn test_custom_emojis_endpoint() {
        // Test that the custom emojis endpoint returns JSON
        let cfg = crate::config::Config::from_env().expect("load config");
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(cfg))
                .service(get_custom_emojis),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/custom-emojis")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_feed_html_has_status_list_container() {
        use async_sqlite::PoolBuilder;
        use atrium_identity::did::{
            CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL,
        };
        use atrium_oauth::DefaultHttpClient;

        let cfg = crate::config::Config::from_env().expect("load config");
        let pool = PoolBuilder::new()
            .path(":memory:")
            .open()
            .await
            .expect("pool");
        let arc_pool = std::sync::Arc::new(pool);

        let resolver = CommonDidResolver::new(CommonDidResolverConfig {
            plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
            http_client: std::sync::Arc::new(DefaultHttpClient::default()),
        });
        let handle_resolver = std::sync::Arc::new(resolver);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(cfg))
                .app_data(web::Data::new(arc_pool))
                .app_data(web::Data::new(handle_resolver))
                .service(feed),
        )
        .await;

        let req = test::TestRequest::get().uri("/feed").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let html = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(
            html.contains("class=\"status-list\""),
            "feed HTML must include an empty .status-list container for client-side population"
        );
    }

    #[actix_web::test]
    async fn test_api_feed_shape() {
        use async_sqlite::PoolBuilder;
        use atrium_identity::did::{
            CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL,
        };
        use atrium_oauth::DefaultHttpClient;
        use serde_json::Value;

        let pool = PoolBuilder::new()
            .path(":memory:")
            .open()
            .await
            .expect("pool");
        let arc_pool = std::sync::Arc::new(pool);
        let resolver = CommonDidResolver::new(CommonDidResolverConfig {
            plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
            http_client: std::sync::Arc::new(DefaultHttpClient::default()),
        });
        let handle_resolver = std::sync::Arc::new(resolver);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(arc_pool))
                .app_data(web::Data::new(handle_resolver))
                .service(api_feed),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/feed?offset=0&limit=20")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let v: Value = serde_json::from_slice(&body).expect("json");
        assert!(
            v.get("statuses").map(|s| s.is_array()).unwrap_or(false),
            "statuses must be an array"
        );
        assert!(v.get("has_more").is_some(), "has_more present");
        assert!(v.get("next_offset").is_some(), "next_offset present");
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
