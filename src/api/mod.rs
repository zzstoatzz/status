pub mod auth;
pub mod preferences;
pub mod status;

pub use auth::OAuthClientType;
pub use status::HandleResolver;

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
        // Status page routes
        .service(status::home)
        .service(status::user_status_page)
        .service(status::feed)
        // Status JSON API routes
        .service(status::owner_status_json)
        .service(status::user_status_json)
        .service(status::status_json)
        .service(status::api_feed)
        // Emoji API routes
        .service(status::get_frequent_emojis)
        .service(status::get_custom_emojis)
        .service(status::upload_emoji)
        .service(status::get_following)
        // Status management routes
        .service(status::status)
        .service(status::clear_status)
        .service(status::delete_status)
        .service(status::hide_status)
        // Preferences routes
        .service(preferences::get_preferences)
        .service(preferences::save_preferences);
}
