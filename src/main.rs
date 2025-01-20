use axum::{routing::{get, post}, Router};
use dotenv::dotenv;

use sqlx::{PgPool, postgres::PgPoolOptions};

mod models;
mod routes;

use routes::subscribers::{get_subscribers, subscribe};



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



#[tokio::main]
async fn main() {
    dotenv().ok();

    // Initialize DB
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

        
    // Create tables
    create_tables(&db_pool)
        .await
        .expect("Failed to create tables");

    let app_state = AppState { db: db_pool };

    let app = Router::new()
        .route("/subscribers", get(get_subscribers))
        .route("/subscribe", post(subscribe))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
