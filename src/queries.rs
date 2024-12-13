use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::state::Platform;

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: Option<PrimitiveDateTime>,
    pub updated_at: Option<PrimitiveDateTime>,
    pub email: String,
}

pub async fn create_user_query(db: PgPool, email: &str) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users(id, created_at, updated_at, email)
        VALUES (
        gen_random_uuid(),
        NOW(),
        NOW(),
        $1
        )
        RETURNING *
        "#,
        email
    )
    .fetch_one(&db)
    .await
}

// Take `platform` as input to safeguard against accidental deletion.
// WARNING: The caller should never call this function with anything other than Platform::Dev, but because of how dangerous this endpoint is, we add an additional safeguard here.
// Returns the number of deleted rows as result if successful.
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
