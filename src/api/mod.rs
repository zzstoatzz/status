pub mod auth;
pub mod preferences;
pub mod status_read;
pub mod status_util;
pub mod status_write;
pub mod webhooks;

pub use crate::api::status_util::HandleResolver;
pub use auth::OAuthClientType;

use actix_web::web;

/// Configure all API routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Auth routes
        .service(auth::client_metadata)
        .service(auth::oauth_callback)
        .service(auth::login)
        .service(auth::logout)
        .service(auth::login_post)
        // Status page routes (read)
        .service(status_read::home)
        .service(status_read::user_status_page)
        .service(status_read::feed)
        // Status JSON API routes (read)
        .service(status_read::owner_status_json)
        .service(status_read::user_status_json)
        .service(status_read::status_json)
        .service(status_read::api_feed)
        // Emoji + following routes
        .service(status_read::get_frequent_emojis)
        .service(status_read::get_custom_emojis)
        .service(status_write::upload_emoji)
        .service(status_read::get_following)
        // Status management routes (write)
        .service(status_write::status)
        .service(status_write::clear_status)
        .service(status_write::delete_status)
        .service(status_write::hide_status)
        // Preferences routes
        .service(preferences::get_preferences)
        .service(preferences::save_preferences)
        // Webhook routes
        .service(webhooks::list_webhooks)
        .service(webhooks::create_webhook)
        .service(webhooks::update_webhook)
        .service(webhooks::rotate_secret)
        .service(webhooks::delete_webhook);
}
