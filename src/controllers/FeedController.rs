use super::BaseTemplate;
use actix_web::{App, HttpServer, Responder, Result, Scope, get, middleware, web};
use askama::Template;
use atrium_api::client::AtpServiceClient;
use atrium_api::types::LimitedU32;
use atrium_xrpc_client::reqwest::ReqwestClient;
use std::{collections::HashMap, ops::Deref};

#[derive(Template)]
#[template(path = "user.html")]
struct UserTemplate<'a> {
    name: &'a str,
    text: &'a str,
}

#[derive(Template)]
#[template(path = "feed.html")]
struct FeedTemplate<'a> {
    _parent: &'a BaseTemplate<'a>,
}

impl<'a> Deref for FeedTemplate<'a> {
    type Target = BaseTemplate<'a>;

    fn deref(&self) -> &Self::Target {
        self._parent
    }
}

#[get("")]
async fn index(query: web::Query<HashMap<String, String>>) -> Result<impl Responder> {
    let client = AtpServiceClient::new(ReqwestClient::new("https://public.api.bsky.app"));
    let feed =
        "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot".to_string();

    let feed_posts = client
        .service
        .app
        .bsky
        .feed
        .get_feed(
            atrium_api::app::bsky::feed::get_feed::ParametersData {
                cursor: None,
                feed,
                limit: None,
            }
            .into(),
        )
        .await;
    //Its working write out a nice thing to parse themS

    let html = if let Some(name) = query.get("name") {
        UserTemplate {
            name,
            text: "Welcome!",
        }
        .render()
        .expect("template should be valid")
    } else {
        FeedTemplate {
            _parent: &BaseTemplate {
                title: "Oh god not another bluesky client",
            },
        }
        .render()
        .expect("template should be valid")
    };

    Ok(web::Html::new(html))
}

pub fn feed_controller() -> Scope {
    web::scope("/feed").service(index)
}
