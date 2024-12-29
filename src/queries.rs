use password_auth::{generate_hash, verify_password};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    api::{Chirp, ChirpBody},
    auth::make_refresh_token,
    state::Platform,
};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: Option<OffsetDateTime>,
    pub updated_at: Option<OffsetDateTime>,
    pub email: String,
    #[serde(skip_serializing)]
    hashed_password: String,
    pub is_chirpy_red: bool,
}

impl User {
    pub fn verify(&self, password: &str) -> Result<(), password_auth::VerifyError> {
        verify_password(password, &self.hashed_password)
    }
}

pub async fn insert_user(db: &PgPool, email: &str, password: &str) -> Result<User, sqlx::Error> {
    let hashed_password = generate_hash(password);
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users(id, created_at, updated_at, email, hashed_password)
        VALUES (
        gen_random_uuid(),
        NOW(),
        NOW(),
        $1,
        $2
        )
        RETURNING *
        "#,
        email,
        hashed_password
    )
    .fetch_one(db)
    .await
}

pub async fn get_user_by_email(db: &PgPool, email: &str) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT * FROM users WHERE email = $1
"#,
        email
    )
    .fetch_one(db)
    .await
}

pub async fn update_user_credentials(
    db: &PgPool,
    user_id: Uuid,
    email: &str,
    password: &str,
) -> Result<User, sqlx::Error> {
    let hashed_password = generate_hash(password);
    sqlx::query_as!(
        User,
        r#"
UPDATE users
SET email = $1, hashed_password = $2
WHERE id = $3
RETURNING *
"#,
        email,
        hashed_password,
        user_id
    )
    .fetch_one(db)
    .await
}

pub async fn make_user_red(db: &PgPool, user_id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
UPDATE users
SET is_chirpy_red = true
WHERE id = $1
RETURNING *
"#,
        user_id
    )
    .fetch_one(db)
    .await
}

pub async fn insert_chirp(
    db: PgPool,
    body: ChirpBody,
    user_id: Uuid,
) -> Result<Chirp, sqlx::Error> {
    sqlx::query_as!(
        Chirp,
        r#"
        INSERT INTO chirps(chirp_id, user_id, created_at, updated_at, body)
        VALUES (
        gen_random_uuid(),
        $1,
        NOW(),
        NOW(),
        $2
        )
        RETURNING chirp_id, user_id, created_at, updated_at, body as "body: _"
        "#,
        user_id,
        &body
    )
    .fetch_one(&db)
    .await
}

pub async fn get_all_chirps_ascending_by_creation(db: PgPool) -> Result<Vec<Chirp>, sqlx::Error> {
    sqlx::query_as!(
        Chirp,
        r#"
SELECT chirp_id, user_id, created_at, updated_at, body as "body: _" FROM chirps
ORDER BY created_at ASC
"#
    )
    .fetch_all(&db)
    .await
}

pub async fn get_chirp(db: PgPool, chirp_id: Uuid) -> Result<Chirp, sqlx::Error> {
    sqlx::query_as!(
        Chirp,
        r#"
SELECT chirp_id, user_id, created_at, updated_at, body as "body: _" FROM chirps
WHERE chirp_id = $1
"#,
        chirp_id
    )
    .fetch_one(&db)
    .await
}
pub async fn delete_chirp_if_author(
    db: &PgPool,
    chirp_id: &Uuid,
    user_id: &Uuid,
) -> Result<Chirp, sqlx::Error> {
    sqlx::query_as!(
        Chirp,
        r#"
DELETE FROM chirps
WHERE chirp_id = $1 AND user_id = $2
RETURNING chirp_id, user_id, created_at, updated_at, body as "body: _"
"#,
        chirp_id,
        user_id
    )
    .fetch_one(db)
    .await
}
/// Take `platform` as input to safeguard against accidental deletion.
/// WARNING: The caller should never call this function with anything other than Platform::Dev, but because of how dangerous this endpoint is, we add an additional safeguard here.
/// Returns the number of deleted rows as result if successful.
pub async fn delete_all_users(db: PgPool, platform: Platform) -> Result<u64, sqlx::Error> {
    assert_eq!(platform, Platform::Dev);
    sqlx::query!(
        r#"
        DELETE FROM users
        "#
    )
    .execute(&db)
    .await
    .map(|ok| ok.rows_affected())
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct RefreshTokenEntry {
    pub token: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub user_id: Uuid,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

pub async fn new_refresh_token(db: &PgPool, user: &User) -> Result<RefreshTokenEntry, sqlx::Error> {
    let refresh_token = make_refresh_token().await;
    sqlx::query_as!(
        RefreshTokenEntry,
        r#"
INSERT INTO refresh_tokens(token, created_at, updated_at, user_id, expires_at, revoked_at)
VALUES (
$1,
NOW(),
NOW(),
$2,
NOW() + INTERVAL '60 days',
NULL
) RETURNING *
"#,
        refresh_token,
        user.id
    )
    .fetch_one(db)
    .await
}

pub async fn get_refresh_token_entry(
    db: &PgPool,
    token: &str,
) -> Result<RefreshTokenEntry, sqlx::Error> {
    sqlx::query_as!(
        RefreshTokenEntry,
        r#"
SELECT * FROM refresh_tokens WHERE token = $1
"#,
        token
    )
    .fetch_one(db)
    .await
}

pub async fn revoke_refresh_token(
    db: &PgPool,
    token: &str,
) -> Result<RefreshTokenEntry, sqlx::Error> {
    sqlx::query_as!(
        RefreshTokenEntry,
        r#"UPDATE refresh_tokens
SET updated_at = NOW(), revoked_at = NOW()
WHERE token = $1
RETURNING *"#,
        token
    )
    .fetch_one(db)
    .await
}
