use axum::{
    routing::{delete, get, post},
    Router,
    http::Method,
    response::IntoResponse,
    http::StatusCode,
};
use dotenv::dotenv;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
    limit::RequestBodyLimitLayer,
};
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};


#[macro_use]
mod models;
mod routes;
use routes::subscribers::{get_subscribers, subscribe, unsubscribe};


#[derive(Clone)]
pub struct AppState {
    db: PgPool,
}

async fn create_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS subscribers (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) NOT NULL UNIQUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );"#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

 
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenv().ok();

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(["http://localhost:3000".parse().unwrap()])
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(tower_http::cors::Any)
        .max_age(Duration::from_secs(3600));

    // Initialize DB
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    create_tables(&db_pool)
        .await
        .expect("Failed to create tables");

    let app_state = AppState { db: db_pool };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/subscribers", get(get_subscribers))
        .route("/subscribe", post(subscribe))
        .route("/unsubscribe", delete(unsubscribe))
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(1024 * 1024)) // 1MB limit
        .layer(cors)
        .with_state(app_state);

    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    tracing::info!("Shutting down server...");
}