use std::time::Duration;

use axum::{Json, Router, extract::State, routing::post};
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Clone)]
struct AppState {
    db: PgPool,
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

#[derive(sqlx::FromRow, Serialize)]
struct Subscriber {
    id: i32,
    email: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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
        );"#,
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
        "INSERT INTO subscribers (email) VALUES ($1) RETURNING id, email, created_at, updated_at",
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

    // Initialize email client
    let smtp_username = std::env::var("SMTP_USERNAME").expect("SMTP_USERNAME must be set");
    let smtp_password = std::env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD must be set");
    let smtp_host = std::env::var("SMTP_HOST").expect("SMTP_HOST must be set");
    let smtp_port = std::env::var("SMTP_PORT")
        .expect("SMTP_PORT must be set")
        .parse::<u16>()
        .expect("SMTP_PORT must be a valid port number");

    // Create credentials with proper authentication
    let creds = Credentials::new(smtp_username.to_string(), smtp_password.to_string());
    
    let mailer = match AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host) {
        Ok(transport) => transport
            .credentials(creds)
            .port(smtp_port)
            .timeout(Some(Duration::from_secs(10)))
            .tls(lettre::transport::smtp::client::Tls::Required(
                lettre::transport::smtp::client::TlsParameters::new(smtp_host.clone())
                    .expect("Failed to create TLS parameters"),
            ))
            .build(),
        Err(e) => {
            eprintln!("Failed to create SMTP transport: {}", e);
            std::process::exit(1);
        }
    };
    



    let app_state = AppState {
        db: db_pool,
        mailer,
    };

    let app = Router::new()
        .route("/subscribe", post(subscribe))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
