use actix_web::{web, App, HttpServer, middleware::Logger, get, Responder, HttpResponse};
use log::info;
use std::sync::Mutex;
use env_logger;
use dotenv::dotenv;
use std::env;

// Import from the library
use workhours::{
    db,
    add_holiday, get_work_hours, list_holidays,
    AppState,
    openapi
};

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    env_logger::init();

    // Get database location from environment variable or use default
    let db_location = env::var("DATABASE_LOCATION").unwrap_or_else(|_| "workhours.db".to_string());

    // Get server host and port from environment variables or use defaults
    let server_host = "0.0.0.0".to_string();
    let server_port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let server_url = format!("{}:{}", server_host, server_port);

    let database = db::Database::new(&db_location).expect("Failed to initialize database");
    let app_state = web::Data::new(AppState {
        db: Mutex::new(database),
    });

    info!("Starting server on {}...", server_url);


    println!("Starting server at {}", server_url);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .service(health)
            .service(add_holiday)
            .service(get_work_hours)
            .service(list_holidays)
            .service(openapi::swagger_routes())
    })
    .bind(&server_url)?
    .run()
    .await
}
