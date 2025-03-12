#![allow(dead_code)]

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use env_logger::Env;
use log::info;

mod api;
mod apierror;
mod renderer;

use renderer::Renderer;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let renderer = web::Data::new(
        Renderer::new("blog", "template.html")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?,
    );

    info!("Starting server on http://localhost:8080");
    HttpServer::new(move || {
        // Configure CORS
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors) // Add CORS middleware
            .wrap(actix_web::middleware::Logger::default()) // Add logger middleware
            .app_data(renderer.clone())
            .service(api::routes::blog_routes())
    })
    .bind("localhost:8080")?
    .run()
    .await
}
