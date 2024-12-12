use axum::{extract, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ValidateChirpPayload {
    body: String,
}

#[derive(Serialize)]
pub struct CleanedValidChirp {
    cleaned_body: String,
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
        (
            StatusCode::OK,
            Json(CleanedValidChirp {
                cleaned_body: clean_chirp(&chirp.body),
            }),
        )
            .into_response()
    }
}

fn clean_chirp(chirp: &str) -> String {
    chirp
        .split_whitespace()
        .map(|w| if is_word_bad(w) { "****" } else { w })
        .collect::<Vec<&str>>()
        .join(" ")
}

fn is_word_bad(w: &str) -> bool {
    let bad_words = ["kerfuffle", "sharbert", "fornax"];

    bad_words.contains(&w.to_lowercase().as_str())
}
