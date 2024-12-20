use password_auth::{generate_hash, verify_password};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{
    api::{Chirp, ChirpBody},
    state::Platform,
};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: Option<PrimitiveDateTime>,
    pub updated_at: Option<PrimitiveDateTime>,
    pub email: String,
    #[serde(skip_serializing)]
    hashed_password: String,
}

impl User {
    pub fn verify(&self, password: &str) -> Result<(), password_auth::VerifyError> {
        verify_password(password, &self.hashed_password)
    }
}

pub async fn insert_user(db: PgPool, email: &str, password: &str) -> Result<User, sqlx::Error> {
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
    .fetch_one(&db)
    .await
}

pub async fn get_user_by_email(db: PgPool, email: &str) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT * FROM users WHERE email = $1
"#,
        email
    )
    .fetch_one(&db)
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
