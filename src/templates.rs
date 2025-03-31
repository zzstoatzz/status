///The askama template types for HTML
///
use crate::db::StatusFromDb;
use askama::Template;
use serde::{Deserialize, Serialize};

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate<'a> {
    pub title: &'a str,
    pub status_options: &'a [&'a str],
    pub profile: Option<Profile>,
    pub statuses: Vec<StatusFromDb>,
    pub my_status: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    pub did: String,
    pub display_name: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate<'a> {
    pub title: &'a str,
    pub error: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate<'a> {
    pub title: &'a str,
    pub error: &'a str,
}
