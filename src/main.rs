use axum::{
    routing::post,
    Router,
    Json,
    extract::State,
};
use serde::{Deserialize, Serialize};
use lettre::{
    Message,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport,
    AsyncTransport,
    Tokio1Executor,
};

#[derive(Deserialize)]
struct EmailRequest {
    to: String,
    subject: String,
    body: String,
}

#[derive(Serialize)]
struct EmailResponse {
    success: bool,
    message: String,
}

#[derive(Clone)]
struct AppState {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

async fn send_email(
    State(state): State<AppState>,
    Json(request): Json<EmailRequest>,
) -> Json<EmailResponse> {
    let email = Message::builder()
        .from("your-email@example.com".parse().unwrap())
        .to(request.to.parse().unwrap())
        .subject(request.subject)
        .body(request.body)
        .unwrap();

    match state.mailer.send(email).await {
        Ok(_) => Json(EmailResponse {
            success: true,
            message: "Email sent successfully".to_string(),
        }),
        Err(e) => Json(EmailResponse {
            success: false,
            message: format!("Failed to send email: {}", e),
        }),
    }
}


#[derive(Deserialize)]
struct SubscribeRequest {
    email: String,
    name: String,
}

#[derive(Serialize)]
struct SubscribeResponse {
    success: bool,
    message: String,
}

async fn subscribe(
    State(state): State<AppState>,
    Json(request): Json<SubscribeRequest>,
) -> Json<SubscribeResponse> {
    let confirmation_email = Message::builder()
        .from("your-email@example.com".parse().unwrap())
        .to(request.email.parse().unwrap())
        .subject("Welcome to Our Newsletter!")
        .body(format!("Hello {}, thank you for subscribing!", request.name))
        .unwrap();

    match state.mailer.send(confirmation_email).await {
        Ok(_) => Json(SubscribeResponse {
            success: true,
            message: "Successfully subscribed to newsletter".to_string(),
        }),
        Err(e) => Json(SubscribeResponse {
            success: false,
            message: format!("Failed to subscribe: {}", e),
        }),
    }
}

#[tokio::main]
async fn main() {
    let creds = Credentials::new(
        "your-email@example.com".to_string(),
        "your-password".to_string(),
    );

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    let app_state = AppState { mailer };

    let app = Router::new()
        .route("/send-email", post(send_email))
        .route("/subscribe", post(subscribe))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}