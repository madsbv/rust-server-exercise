use axum::http::StatusCode;
use axum::{extract::State, response::Html, Extension};
use sqlx::PgPool;

use crate::queries::delete_all_users;
use crate::state::{AppState, Platform};

pub async fn metrics(State(state): State<AppState>) -> Html<String> {
    let hits = { state.data.lock().unwrap().fileserver_hits };

    format!(
        "<html>
  <body>
    <h1>Welcome, Chirpy Admin</h1>
    <p>Chirpy has been visited {hits} times!</p>
  </body>
</html>"
    )
    .into()
}

pub async fn reset(Extension(db): Extension<PgPool>, state: State<AppState>) -> StatusCode {
    if state.config.platform != Platform::Dev {
        return StatusCode::FORBIDDEN;
    }

    state.data.lock().unwrap().fileserver_hits = 0;

    match delete_all_users(db, state.config.platform).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
