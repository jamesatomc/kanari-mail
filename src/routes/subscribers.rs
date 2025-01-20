use hyper::StatusCode;

use crate::{models::{email::{EmailList, Subscriber}, response::ApiResponse}, AppState};
use axum::{extract::State, response::IntoResponse, Json};

pub async fn get_subscribers(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_as::<_, (String,)>("SELECT email FROM subscribers")
        .fetch_all(&state.db)
        .await
    {
        Ok(rows) => {
            let emails = rows.into_iter().map(|r| r.0).collect();
            let response = ApiResponse {
                success: true,
                message: "Subscribers retrieved successfully".to_string(),
                data: Some(EmailList { emails }),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            eprintln!("Error fetching subscribers: {:?}", e);
            let response = ApiResponse {
                success: false,
                message: "Failed to fetch subscribers".to_string(),
                data: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

pub async fn subscribe(
    State(state): State<AppState>,
    Json(subscriber): Json<Subscriber>,
) -> impl IntoResponse {
    match sqlx::query("INSERT INTO subscribers (email) VALUES ($1)")
        .bind(&subscriber.email)
        .execute(&state.db)
        .await
    {
        Ok(_) => {
            let response = ApiResponse {
                success: true,
                message: "Subscription successful".to_string(),
                data: Some(subscriber),
            };
            (StatusCode::CREATED, Json(response))
        }
        Err(e) => {
            eprintln!("Error inserting subscriber: {:?}", e);
            let response = ApiResponse {
                success: false,
                message: "Failed to subscribe".to_string(),
                data: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}


pub async fn unsubscribe(
    State(state): State<AppState>,
    Json(subscriber): Json<Subscriber>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM subscribers WHERE email = $1")
        .bind(&subscriber.email)
        .execute(&state.db)
        .await
    {
        Ok(result) => {
            if result.rows_affected() == 0 {
                let response = ApiResponse {
                    success: false,
                    message: "Email not found".to_string(),
                    data: None,
                };
                (StatusCode::NOT_FOUND, Json(response))
            } else {
                let response = ApiResponse {
                    success: true,
                    message: "Unsubscribed successfully".to_string(),
                    data: Some(subscriber),
                };
                (StatusCode::OK, Json(response))
            }
        }
        Err(e) => {
            eprintln!("Error removing subscriber: {:?}", e);
            let response = ApiResponse {
                success: false,
                message: "Failed to unsubscribe".to_string(),
                data: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}