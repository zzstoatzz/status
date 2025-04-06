use crate::{
    db::{StatusFromDb, create_tables_in_database},
    ingester::start_ingester,
    lexicons::record::KnownRecord,
    lexicons::xyz::statusphere::Status,
    storage::{SqliteSessionStore, SqliteStateStore},
    templates::{HomeTemplate, LoginTemplate},
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
use async_sqlite::{Pool, PoolBuilder};
use atrium_api::{
    agent::Agent,
    types::Collection,
    types::string::{Datetime, Did},
};
use atrium_common::resolver::Resolver;
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig},
};
use atrium_oauth::{
    AtprotoLocalhostClientMetadata, AuthorizeOptions, CallbackParams, DefaultHttpClient,
    KnownScope, OAuthClient, OAuthClientConfig, OAuthResolverConfig, Scope,
};
use dotenv::dotenv;
use resolver::HickoryDnsTxtResolver;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    sync::Arc,
};
use templates::{ErrorTemplate, Profile};

extern crate dotenv;

mod db;
mod ingester;
mod lexicons;
mod resolver;
mod storage;
mod templates;

/// OAuthClientType to make it easier to access the OAuthClient in web requests
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
    params: web::Query<CallbackParams>,
    oauth_client: web::Data<OAuthClientType>,
    session: Session,
) -> HttpResponse {
    //Processes the call back and parses out a session if found and valid
    match oauth_client.callback(params.into_inner()).await {
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

/// TS version https://github.com/bluesky-social/statusphere-example-app/blob/e4721616df50cd317c198f4c00a4818d5626d4ce/src/routes.ts#L146
/// Home
#[get("/")]
async fn home(
    session: Session,
    oauth_client: web::Data<OAuthClientType>,
    db_pool: web::Data<Arc<Pool>>,
    handle_resolver: web::Data<HandleResolver>,
) -> Result<impl Responder> {
    const TITLE: &str = "Home";
    //Loads the last 10 statuses saved in the DB
    let mut statuses = StatusFromDb::load_latest_statuses(&db_pool)
        .await
        .unwrap_or_else(|err| {
            log::error!("Error loading statuses: {err}");
            vec![]
        });

    //Simple way to cut down on resolve calls if we already know the handle for the did
    let mut quick_resolve_map: HashMap<Did, String> = HashMap::new();
    // We resolve the handles to the DID. This is a bit messy atm,
    // and there are hopes to find a cleaner way
    // to handle resolving the DIDs and formating the handles,
    // But it gets the job done for the purpose of this tutorial.
    // PRs are welcomed!
    for db_status in &mut statuses {
        let authors_did = Did::new(db_status.author_did.clone()).expect("failed to parse did");
        //Check to see if we already resolved it to cut down on resolve requests
        match quick_resolve_map.get(&authors_did) {
            None => {}
            Some(found_handle) => {
                db_status.handle = Some(found_handle.clone());
                continue;
            }
        }
        //Attempts to resolve the DID to a handle
        db_status.handle = match handle_resolver.resolve(&authors_did).await {
            Ok(did_doc) => {
                match did_doc.also_known_as {
                    None => None,
                    Some(also_known_as) => {
                        match also_known_as.is_empty() {
                            true => None,
                            false => {
                                //also_known as a list starts the array with the highest priority handle
                                let formatted_handle =
                                    format!("@{}", also_known_as[0]).replace("at://", "");
                                quick_resolve_map.insert(authors_did, formatted_handle.clone());
                                Some(formatted_handle)
                            }
                        }
                    }
                }
            }
            Err(err) => {
                log::error!("Error resolving did: {err}");
                None
            }
        };
    }

    // If the user is signed in, get an agent which communicates with their server
    match session.get::<String>("did").unwrap_or(None) {
        Some(did) => {
            let did = Did::new(did).expect("failed to parse did");
            //Grabs the users last status to highlight it in the ui
            let my_status = StatusFromDb::my_status(&db_pool, &did)
                .await
                .unwrap_or_else(|err| {
                    log::error!("Error loading my status: {err}");
                    None
                });

            // gets the user's session from the session store to resume
            match oauth_client.restore(&did).await {
                Ok(session) => {
                    //Creates an agent to make authenticated requests
                    let agent = Agent::new(session);

                    // Fetch additional information about the logged-in user
                    let profile = agent
                        .api
                        .app
                        .bsky
                        .actor
                        .get_profile(
                            atrium_api::app::bsky::actor::get_profile::ParametersData {
                                actor: atrium_api::types::string::AtIdentifier::Did(did),
                            }
                            .into(),
                        )
                        .await;

                    let html = HomeTemplate {
                        title: TITLE,
                        status_options: &STATUS_OPTIONS,
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
                        my_status: my_status.as_ref().map(|s| s.status.clone()),
                    }
                    .render()
                    .expect("template should be valid");

                    Ok(web::Html::new(html))
                }
                Err(err) => {
                    // Destroys the system or you're in a loop
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
            let html = HomeTemplate {
                title: TITLE,
                status_options: &STATUS_OPTIONS,
                profile: None,
                statuses,
                my_status: None,
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
) -> HttpResponse {
    // Check if the user is logged in
    match session.get::<String>("did").unwrap_or(None) {
        Some(did_string) => {
            let did = Did::new(did_string.clone()).expect("failed to parse did");
            // gets the user's session from the session store to resume
            match oauth_client.restore(&did).await {
                Ok(session) => {
                    let agent = Agent::new(session);
                    //Creates a strongly typed ATProto record
                    let status: KnownRecord = lexicons::xyz::statusphere::status::RecordData {
                        created_at: Datetime::now(),
                        status: form.status.clone(),
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
                                collection: Status::NSID.parse().unwrap(),
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
                            let status = StatusFromDb::new(
                                record.uri.clone(),
                                did_string,
                                form.status.clone(),
                            );

                            let _ = status.save(db_pool).await;
                            Redirect::to("/")
                                .see_other()
                                .respond_to(&request)
                                .map_into_boxed_body()
                        }
                        Err(err) => {
                            log::error!("Error creating status: {err}");
                            let error_html = ErrorTemplate {
                                title: "Error",
                                error: "Was an error creating the status, please check the logs.",
                            }
                            .render()
                            .expect("template should be valid");
                            HttpResponse::Ok().body(error_html)
                        }
                    }
                }
                Err(err) => {
                    // Destroys the system or you're in a loop
                    session.purge();
                    log::error!(
                        "Error restoring session, we are removing the session from the cookie: {err}"
                    );
                    let error_html = ErrorTemplate {
                        title: "Error",
                        error: "Was an error resuming the session, please check the logs.",
                    }
                    .render()
                    .expect("template should be valid");
                    HttpResponse::Ok().body(error_html)
                }
            }
        }
        None => {
            let error_template = ErrorTemplate {
                title: "Error",
                error: "You must be logged in to create a status.",
            }
            .render()
            .expect("template should be valid");
            HttpResponse::Ok().body(error_template)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    //Uses a default sqlite db path or use the one from env
    let db_connection_string =
        std::env::var("DB_PATH").unwrap_or_else(|_| String::from("./statusphere.sqlite3"));

    //Crates a db pool to share resources to the db
    let pool = match PoolBuilder::new().path(db_connection_string).open().await {
        Ok(pool) => pool,
        Err(err) => {
            log::error!("Error creating the sqlite pool: {}", err);
            return Err(Error::new(
                ErrorKind::Other,
                "sqlite pool could not be created.",
            ));
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
    let config = OAuthClientConfig {
        client_metadata: AtprotoLocalhostClientMetadata {
            redirect_uris: Some(vec![String::from(format!(
                //This must match the endpoint you use the callback function
                "http://{host}:{port}/oauth/callback"
            ))]),
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
    let client = Arc::new(OAuthClient::new(config).expect("failed to create OAuth client"));
    let arc_pool = Arc::new(pool.clone());
    //Spawns the ingester that listens for other's Statusphere updates
    tokio::spawn(async move {
        start_ingester(arc_pool).await;
    });
    let arc_pool = Arc::new(pool.clone());
    log::info!("starting HTTP server at http://{host}:{port}");
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(client.clone()))
            .app_data(web::Data::new(arc_pool.clone()))
            .app_data(web::Data::new(handle_resolver.clone()))
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
            .service(Files::new("/css", "public/css").show_files_listing())
            .service(oauth_callback)
            .service(login)
            .service(login_post)
            .service(logout)
            .service(home)
            .service(status)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
