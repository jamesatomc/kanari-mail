use serde::{Serialize, Deserialize};

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct Subscriber {
    pub email: String,
}

#[derive(Serialize)]
pub struct EmailList {
    pub emails: Vec<String>,
}