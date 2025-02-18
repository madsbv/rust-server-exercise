use std::{collections::HashMap, ops::Deref};

use axum::{
    extract::{Path, Query},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use color_eyre::eyre::{ensure, OptionExt, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Database, Decode, PgPool};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    auth::{JwtKey, PolkaAPIKey},
    queries::{
        self, delete_chirp_if_author, get_all_chirps_by_author_sorted_by_creation,
        get_all_chirps_sorted_by_creation, get_refresh_token_entry, get_user_by_email,
        insert_chirp, insert_user, make_user_red, new_refresh_token, revoke_refresh_token,
        update_user_credentials, RefreshTokenEntry, SortOrder, User,
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
    let Ok(user_id) = extract_user_id_from_bearer(&headers, &key) else {
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

fn extract_user_id_from_bearer(headers: &HeaderMap, key: &JwtKey) -> Result<Uuid> {
    let token = extract_bearer_token(headers)?;

    key.decode_user(token)
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

pub fn extract_api_key(headers: &HeaderMap) -> Result<&str> {
    let auth_str = headers
        .get(AUTHORIZATION)
        .ok_or_eyre("Headers missing valid AUTHORIZATION header")?
        .to_str()?;

    auth_str
        .strip_prefix("ApiKey ")
        .ok_or_eyre("AUTHORIZATION header is malformed")
}

pub async fn get_all_chirps(
    Extension(db): Extension<PgPool>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let sort_order = match params.get("sort").unwrap_or(&"asc".to_string()).as_str() {
        "asc" => SortOrder::Asc,
        "desc" => SortOrder::Desc,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let chirps = match params.get("author_id").map(|s| Uuid::try_parse(s)) {
        Some(Ok(author_id)) => {
            get_all_chirps_by_author_sorted_by_creation(&db, author_id, sort_order).await
        }
        None => get_all_chirps_sorted_by_creation(&db, sort_order).await,
        Some(Err(_)) => return StatusCode::NOT_FOUND.into_response(),
    };

    match chirps {
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

pub async fn delete_chirp(
    Extension(db): Extension<PgPool>,
    Path(chirp_id): Path<Uuid>,
    headers: HeaderMap,
    Extension(key): Extension<JwtKey>,
) -> impl IntoResponse {
    let Ok(user_id) = extract_user_id_from_bearer(&headers, &key) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    match delete_chirp_if_author(&db, &chirp_id, &user_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::FORBIDDEN.into_response(),
    }
}

#[derive(Serialize)]
pub struct ChirpValidationError {
    error: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::Type, sqlx::FromRow)]
pub struct Chirp {
    #[serde(rename = "id")]
    pub chirp_id: Uuid,
    pub user_id: Uuid,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
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
    let res = insert_user(&db, &payload.email, &payload.password).await;
    match res {
        Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

#[derive(Deserialize)]
pub struct LoginPayload {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    #[serde(flatten)]
    user: User,
    #[serde(rename = "token")]
    jwt_token: String,
    refresh_token: String,
}

pub async fn login(
    Extension(db): Extension<PgPool>,
    Extension(key): Extension<JwtKey>,
    Json(payload): Json<LoginPayload>,
) -> impl IntoResponse {
    let user = get_user_by_email(&db, &payload.email).await;
    let expires_in = Duration::hours(1);

    let error_response = (StatusCode::UNAUTHORIZED, "Incorrect email or password").into_response();

    if let Ok(user) = user
        && user.verify(&payload.password).is_ok()
    {
        let (Ok(refresh_token_entry), Ok(jwt_token)) = (
            new_refresh_token(&db, &user).await,
            key.encode_user(&user.id, expires_in),
        ) else {
            return error_response;
        };

        assert_eq!(user.id, refresh_token_entry.user_id);

        return (
            StatusCode::OK,
            Json(LoginResponse {
                user,
                jwt_token,
                refresh_token: refresh_token_entry.token,
            }),
        )
            .into_response();
    }

    error_response
}

#[derive(Serialize)]
pub struct RefreshResponse {
    #[serde(rename = "token")]
    pub jwt_token: String,
}

pub async fn refresh(
    Extension(db): Extension<PgPool>,
    Extension(key): Extension<JwtKey>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Ok(token) = authorize_user_refresh_token(&db, &headers).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Ok(jwt_token) = key.encode_user(&token.user_id, Duration::hours(1)) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    Json(RefreshResponse { jwt_token }).into_response()
}

async fn authorize_user_refresh_token(
    db: &PgPool,
    headers: &HeaderMap,
) -> Result<RefreshTokenEntry> {
    let token = extract_bearer_token(headers)?;

    let token_entry = get_refresh_token_entry(db, token).await?;

    let current_time = OffsetDateTime::now_utc();

    ensure!(token_entry.expires_at < current_time, "token has expired");

    if let Some(revoked_at) = token_entry.revoked_at {
        ensure!(revoked_at < current_time, "token was revoked");
    }

    Ok(token_entry)
}

async fn extract_jwt_token_user_id(headers: &HeaderMap, key: &JwtKey) -> Result<Uuid> {
    let token = extract_bearer_token(headers)?;
    key.decode_user(token)
}

pub async fn revoke(Extension(db): Extension<PgPool>, headers: HeaderMap) -> impl IntoResponse {
    let Ok(token) = extract_bearer_token(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    if revoke_refresh_token(&db, token).await.is_err() {
        return StatusCode::NOT_FOUND.into_response();
    };

    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
pub struct PutUserReq {
    email: String,
    password: String,
}

pub async fn update_user(
    Extension(db): Extension<PgPool>,
    headers: HeaderMap,
    Extension(key): Extension<JwtKey>,
    Json(req_body): Json<PutUserReq>,
) -> impl IntoResponse {
    // FIXME: This should be a jwt token instead of refresh token
    let Ok(user_id) = extract_jwt_token_user_id(&headers, &key).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    match update_user_credentials(&db, user_id, &req_body.email, &req_body.password).await {
        Ok(user) => (StatusCode::OK, Json(user)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(Deserialize)]
pub struct PolkaData {
    pub user_id: Uuid,
}

#[derive(Deserialize)]
pub struct PolkaReq {
    pub event: String,
    pub data: PolkaData,
}

pub async fn polka_webhook(
    Extension(db): Extension<PgPool>,
    Extension(polka_api_key): Extension<PolkaAPIKey>,
    headers: HeaderMap,
    Json(req): Json<PolkaReq>,
) -> impl IntoResponse {
    if !polka_api_key.request_authorized(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    if req.event != "user.upgraded" {
        return StatusCode::NO_CONTENT.into_response();
    }

    match make_user_red(&db, req.data.user_id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::NOT_FOUND,
    }
    .into_response()
}
