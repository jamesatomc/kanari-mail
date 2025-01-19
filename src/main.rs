use axum::{
    routing::post,
    Router,
    Json,
    extract::State,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use lettre::{
    Message,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport,
    AsyncTransport,
    Tokio1Executor,
};
use chrono::{DateTime, Utc};


#[derive(Clone)]
struct AppState {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    db: PgPool
}

#[derive(sqlx::FromRow, Serialize)]
struct Subscriber {
    id: i32,
    email: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>
}

#[derive(Deserialize)]
struct SubscribeRequest {
    email: String,
}

#[derive(Serialize)]
struct SubscribeResponse {
    success: bool,
    message: String,
}

async fn create_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS subscribers (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) NOT NULL UNIQUE,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        );"#
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn subscribe(
    State(state): State<AppState>,
    Json(request): Json<SubscribeRequest>,
) -> Json<SubscribeResponse> {
    let result = sqlx::query_as::<_, Subscriber>(
        "INSERT INTO subscribers (email) VALUES ($1) RETURNING id, email, created_at, updated_at"
    )
    .bind(&request.email)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(subscriber) => {
            let confirmation_email = Message::builder()
                .from("your-email@example.com".parse().unwrap())
                .to(subscriber.email.parse().unwrap())
                .subject("Welcome to Our Newsletter!")
                .body("Thank you for subscribing to our newsletter!".to_string())
                .unwrap();

            match state.mailer.send(confirmation_email).await {
                Ok(_) => Json(SubscribeResponse {
                    success: true,
                    message: "Successfully subscribed to newsletter".to_string(),
                }),
                Err(e) => Json(SubscribeResponse {
                    success: false,
                    message: format!("Failed to send welcome email: {}", e),
                }),
            }
        }
        Err(e) => Json(SubscribeResponse {
            success: false, 
            message: format!("Failed to subscribe: {}", e),
        }),
    }
}

#[tokio::main]
async fn main() {
    // Initialize DB
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    // Create tables
    create_tables(&db_pool).await.expect("Failed to create tables");

    // Initialize email client
    let creds = Credentials::new(
        "your-email@example.com".to_string(),
        "your-password".to_string(),
    );

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    let app_state = AppState { 
        mailer,
        db: db_pool,
    };

    let app = Router::new()
        .route("/subscribe", post(subscribe))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}