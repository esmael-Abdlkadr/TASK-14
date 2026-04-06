use actix_web::{web, App, HttpServer, middleware::Logger};
use dotenv::dotenv;
use log::info;

mod admin;
mod auth;
mod audit;
mod dedup;
mod db;
mod encryption;
mod errors;
mod images;
mod messaging;
mod middleware;
mod models;
mod risk;
mod review;
mod routes;
mod scheduling;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let bind_addr =
        std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    info!("Connecting to database...");
    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create database pool");

    info!("Running migrations...");
    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    // Initialize encryption keys
    if std::env::var("CIVICSORT_MASTER_KEY").is_ok() {
        if let Err(e) = encryption::key_management::init_default_key(&pool).await {
            log::warn!("Could not initialize encryption key: {}", e);
        }
    }

    info!("Starting CivicSort server on {}", bind_addr);

    let pool_data = web::Data::new(pool);

    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .wrap(Logger::default())
            .configure(routes::auth_routes::auth_config)
            .configure(routes::user_routes::user_config)
            .configure(routes::audit_routes::audit_config)
            .configure(routes::device_routes::device_config)
            .configure(routes::kb_routes::kb_config)
            .configure(routes::inspection_routes::inspection_config)
            .configure(routes::review_routes::review_config)
            .configure(routes::admin_routes::admin_config)
            .configure(routes::messaging_routes::messaging_config)
            .configure(routes::bulk_data_routes::bulk_data_config)
            .configure(routes::dispute_routes::dispute_config)
            .route(
                "/api/health",
                web::get().to(|| async {
                    actix_web::HttpResponse::Ok().json(serde_json::json!({
                        "status": "healthy",
                        "service": "civicsort"
                    }))
                }),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}
