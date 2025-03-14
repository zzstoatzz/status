use actix_files::Files;
use actix_web::{App, HttpServer, Responder, Result, middleware, web};
use controllers::FeedController::feed_controller;
use std::collections::HashMap;

pub mod controllers;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .service(Files::new("/css", "public/css").show_files_listing())
            .service(feed_controller())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
