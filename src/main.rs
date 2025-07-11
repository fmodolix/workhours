use actix_web::{web, App, HttpServer, middleware::Logger};
use log::info;
use std::sync::Mutex;
use env_logger;

// Import from the library
use workhours::{
    db,
    add_holiday, get_work_hours, list_holidays,
    AppState
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let database = db::Database::new("workhours.db").expect("Failed to initialize database");
    let app_state = web::Data::new(AppState {
        db: Mutex::new(database),
    });
    info!("Starting server...");
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Logger::default())
            .service(add_holiday)
            .service(get_work_hours)
            .service(list_holidays)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
