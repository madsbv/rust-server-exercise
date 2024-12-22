use std::ops::Deref;

use axum::{
    extract::Path,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use color_eyre::eyre::{OptionExt, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Database, Decode, PgPool};
use time::{Duration, PrimitiveDateTime};
use uuid::Uuid;

use crate::{
    jwt::JwtKey,
    queries::{
        self, get_all_chirps_ascending_by_creation, get_user_by_email, insert_chirp, insert_user,
        User,
    },
};

#[derive(Deserialize)]
pub struct PostChirpPayload {
    body: String,
}

pub async fn post_chirp(
    Extension(db): Extension<PgPool>,
    Extension(key): Extension<JwtKey>,
    headers: HeaderMap,
    Json(chirp_payload): Json<PostChirpPayload>,
) -> impl IntoResponse {
    let Ok(token) = extract_bearer_token(&headers) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let Ok(user_id) = key.decode_user(token) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Ok(body) = ChirpBody::try_from(chirp_payload.body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ChirpValidationError {
                error: "Chirp is too long".to_string(),
            }),
        )
            .into_response();
    };

    match insert_chirp(db, body, user_id).await {
        Ok(chirp) => (StatusCode::CREATED, Json(chirp)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str> {
    let bearer = headers
        .get(AUTHORIZATION)
        .ok_or_eyre("Headers missing valid AUTHORIZATION header")?
        .to_str()?;

    bearer
        .strip_prefix("Bearer ")
        .ok_or_eyre("AUTHORIZATION header is malformed")
}

pub async fn get_all_chirps(Extension(db): Extension<PgPool>) -> impl IntoResponse {
    match get_all_chirps_ascending_by_creation(db).await {
        Ok(chirps) => Json(chirps).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn get_chirp(
    Extension(db): Extension<PgPool>,
    Path(chirp_id): Path<Uuid>,
) -> impl IntoResponse {
    match queries::get_chirp(db, chirp_id).await {
        Ok(chirp) => Json(chirp).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Serialize)]
pub struct ChirpValidationError {
    error: String,
}

// TODO: Figure out how to organize data structures. `Chirp` should probably go in the same place as `User`, but should `ChirpBody` then also go there? There's some strange cross-dependencies going on here.
// Maybe "fundamental" types (i.e. those that are determined by the business requirements) and all that they depend on should go in a separate module.
#[derive(Serialize, Deserialize, Debug, Clone, sqlx::Type)]
pub struct Chirp {
    #[serde(rename = "id")]
    pub chirp_id: Uuid,
    pub user_id: Uuid,
    pub created_at: Option<PrimitiveDateTime>,
    pub updated_at: Option<PrimitiveDateTime>,
    pub body: ChirpBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Encode)]
#[serde(try_from = "String", into = "String")]
pub struct ChirpBody(String);

impl sqlx::Type<sqlx::Postgres> for ChirpBody {
    fn type_info() -> <sqlx::Postgres as Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

// DB is the database driver
// `'r` is the lifetime of the `Row` being decoded
impl<'r, DB: Database> sqlx::Decode<'r, DB> for ChirpBody
where
    // we want to delegate some of the work to string decoding so let's make sure strings
    // are supported by the database
    &'r str: Decode<'r, DB>,
{
    fn decode(
        value: <DB as Database>::ValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let value = <&str as Decode<DB>>::decode(value)?;

        Ok(Self::try_from(value.to_owned())?)
    }
}

impl From<ChirpBody> for String {
    fn from(s: ChirpBody) -> String {
        s.0
    }
}

impl Deref for ChirpBody {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<String> for ChirpBody {
    type Error = String;

    fn try_from(body: String) -> Result<Self, Self::Error> {
        if body.len() > 140 {
            Err("Body exceeds the maximum length of a chirp".to_owned())
        } else {
            Ok(ChirpBody(clean_chirp(body)))
        }
    }
}

fn clean_chirp(chirp: String) -> String {
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

#[derive(Deserialize)]
pub struct CreateUserPayload {
    email: String,
    password: String,
}

pub async fn create_user(
    Extension(db): Extension<PgPool>,
    Json(payload): Json<CreateUserPayload>,
) -> impl IntoResponse {
    let res = insert_user(db, &payload.email, &payload.password).await;
    match res {
        Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

#[derive(Deserialize)]
pub struct LoginPayload {
    email: String,
    password: String,
    expires_in_seconds: Option<i64>,
}

#[derive(Serialize)]
pub struct UserWithToken {
    #[serde(flatten)]
    user: User,
    token: String,
}

pub async fn login(
    Extension(db): Extension<PgPool>,
    Extension(key): Extension<JwtKey>,
    Json(payload): Json<LoginPayload>,
) -> impl IntoResponse {
    let user = get_user_by_email(db, &payload.email).await;

    let error_response = (StatusCode::UNAUTHORIZED, "Incorrect email or password").into_response();

    if let Ok(user) = user
        && user.verify(&payload.password).is_ok()
    {
        let expires_in = match payload.expires_in_seconds {
            Some(n) => Duration::seconds(n).max(Duration::hours(1)),
            None => Duration::hours(1),
        };

        let Ok(token) = key.encode_user(&user, expires_in) else {
            return error_response;
        };
        return (StatusCode::OK, Json(UserWithToken { user, token })).into_response();
    }

    error_response
}
