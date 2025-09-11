use atrium_identity::did::CommonDidResolver;
use atrium_oauth::DefaultHttpClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// HandleResolver to make it easier to access the OAuthClient in web requests
pub type HandleResolver = Arc<CommonDidResolver<DefaultHttpClient>>;

/// Admin DID for moderation
pub const ADMIN_DID: &str = "did:plc:xbtmt2zjwlrfegqvch7fboei"; // zzstoatzz.io

/// Check if a DID is the admin
pub fn is_admin(did: &str) -> bool {
    did == ADMIN_DID
}

/// The post body for changing your status
#[derive(Serialize, Deserialize, Clone)]
pub struct StatusForm {
    pub status: String,
    pub text: Option<String>,
    pub expires_in: Option<String>, // e.g., "1h", "30m", "1d", etc.
}

/// The post body for deleting a specific status
#[derive(Serialize, Deserialize)]
pub struct DeleteRequest {
    pub uri: String,
}

/// Hide/unhide a status (admin only)
#[derive(Deserialize)]
pub struct HideStatusRequest {
    pub uri: String,
    pub hidden: bool,
}

/// Parse duration string like "1h", "30m", "1d" into chrono::Duration
pub fn parse_duration(duration_str: &str) -> Option<chrono::Duration> {
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
