use axum::{
    extract,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ValidateChirpPayload {
    body: String,
}

#[derive(Serialize)]
pub struct ValidateChirpValid {
    valid: bool,
}

#[derive(Serialize)]
pub struct ValidateChirpError {
    error: String,
}

pub async fn validate_chirp(
    extract::Json(chirp): extract::Json<ValidateChirpPayload>,
) -> impl IntoResponse {
    if chirp.body.len() > 140 {
        (
            StatusCode::BAD_REQUEST,
            Json(ValidateChirpError {
                error: "Chirp is too long".to_string(),
            }),
        )
            .into_response()
    } else {
        (StatusCode::OK, Json(ValidateChirpValid { valid: true })).into_response()
    }
}
