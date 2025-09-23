use crate::resolver::HickoryDnsTxtResolver;
use crate::{
    config,
    storage::{SqliteSessionStore, SqliteStateStore},
    templates::{ErrorTemplate, LoginTemplate},
};
use actix_session::Session;
use actix_web::{
    HttpRequest, HttpResponse, Responder, Result, get, post,
    web::{self, Redirect},
};
use askama::Template;
use atrium_api::agent::Agent;
use atrium_identity::{did::CommonDidResolver, handle::AtprotoHandleResolver};
use atrium_oauth::{
    AuthorizeOptions, CallbackParams, DefaultHttpClient, KnownScope, OAuthClient, Scope,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct OAuthCallbackParams {
    pub state: Option<String>,
    pub iss: Option<String>,
    pub code: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

pub type OAuthClientType = Arc<
    OAuthClient<
        SqliteStateStore,
        SqliteSessionStore,
        CommonDidResolver<DefaultHttpClient>,
        AtprotoHandleResolver<HickoryDnsTxtResolver, DefaultHttpClient>,
    >,
>;

/// OAuth client metadata endpoint for production
#[get("/oauth-client-metadata.json")]
pub async fn client_metadata(config: web::Data<config::Config>) -> Result<HttpResponse> {
    let public_url = config.oauth_redirect_base.clone();

    let metadata = serde_json::json!({
        "client_id": format!("{}/oauth-client-metadata.json", public_url),
        "client_name": "Status Sphere",
        "client_uri": public_url.clone(),
        "redirect_uris": [format!("{}/oauth/callback", public_url)],
        "scope": "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app",
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
        "dpop_bound_access_tokens": true
    });

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(metadata.to_string()))
}

/// OAuth callback endpoint to complete session creation
#[get("/oauth/callback")]
pub async fn oauth_callback(
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

/// Takes you to the login page
#[get("/login")]
pub async fn login() -> Result<impl Responder> {
    let html = LoginTemplate {
        title: "Log in",
        error: None,
    };
    Ok(web::Html::new(
        html.render().expect("template should be valid"),
    ))
}

/// Logs you out by destroying your cookie on the server and web browser
#[get("/logout")]
pub async fn logout(request: HttpRequest, session: Session) -> HttpResponse {
    session.purge();
    Redirect::to("/")
        .see_other()
        .respond_to(&request)
        .map_into_boxed_body()
}

/// The post body for logging in
#[derive(Serialize, Deserialize, Clone)]
pub struct LoginForm {
    pub handle: String,
}

/// Login endpoint
#[post("/login")]
pub async fn login_post(
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
                            // Using granular scope for status records only
                            // This replaces TransitionGeneric with specific permissions
                            Scope::Unknown("repo:io.zzstoatzz.status.record".to_string()),
                            // Need to read profiles for the feed page
                            Scope::Unknown("rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview".to_string()),
                            // Need to read following list for following feed
                            Scope::Unknown("rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app".to_string()),
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
