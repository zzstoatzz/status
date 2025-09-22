///The askama template types for HTML
///
use crate::db::StatusFromDb;
use askama::Template;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    pub did: String,
    pub display_name: Option<String>,
    pub handle: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate<'a> {
    #[allow(dead_code)]
    pub title: &'a str,
    pub error: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate<'a> {
    #[allow(dead_code)]
    pub title: &'a str,
    pub error: &'a str,
}

#[derive(Template)]
#[template(path = "status.html")]
pub struct StatusTemplate<'a> {
    #[allow(dead_code)]
    pub title: &'a str,
    pub handle: String,
    pub current_status: Option<StatusFromDb>,
    pub history: Vec<StatusFromDb>,
    pub is_owner: bool,
    pub is_admin: bool,
}

#[derive(Template)]
#[template(path = "status_share.html")]
pub struct StatusShareTemplate<'a> {
    #[allow(dead_code)]
    pub title: &'a str,
    pub status: StatusFromDb,
    pub canonical_url: String,
    pub display_handle: String,
    pub meta_title: String,
    pub meta_description: String,
    pub share_text: String,
    pub profile_href: String,
}

#[derive(Template)]
#[template(path = "feed.html")]
pub struct FeedTemplate<'a> {
    #[allow(dead_code)]
    pub title: &'a str,
    pub profile: Option<Profile>,
    pub statuses: Vec<StatusFromDb>,
    pub is_admin: bool,
    pub dev_mode: bool,
}
