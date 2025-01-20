use axum::{extract::State, response::IntoResponse, routing::{get, post}, Json, Router};
use dotenv::dotenv;
use hyper::StatusCode;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower_http::trace::TraceLayer;
mod models;
use crate::models::email::{EmailList, Subscriber};



#[derive(Clone)]
struct AppState {
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

async fn get_subscribers(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, (String,)>("SELECT email FROM subscribers")
        .fetch_all(&state.db)
        .await
    {
        Ok(rows) => {
            let emails = rows.into_iter().map(|r| r.0).collect();
            (StatusCode::OK, Json(EmailList { emails }))
        }
        Err(e) => {
            eprintln!("Error fetching subscribers: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(EmailList { emails: vec![] }),
            )
        }
    }
}

async fn subscribe(
    State(state): State<AppState>,
    Json(subscriber): Json<Subscriber>,
) -> impl IntoResponse {
    match sqlx::query("INSERT INTO subscribers (email) VALUES ($1)")
        .bind(&subscriber.email)
        .execute(&state.db)
        .await
    {
        Ok(_) => (StatusCode::CREATED, Json(())),
        Err(e) => {
            eprintln!("Error inserting subscriber: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(()))
        }
    }
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
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
