use askama::Template;

pub mod FeedController;

#[derive(Template)]
#[template(path = "base.html")]
struct BaseTemplate<'a> {
    title: &'a str,
}
