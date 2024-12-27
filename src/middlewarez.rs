use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use super::AppState;

pub async fn fileserver_hits_middleware(
    State(app_state): State<AppState>,
    // you can add more extractors here but the last
    // extractor must implement `FromRequest` which
    // `Request` does
    request: Request,
    next: Next,
) -> Response {
    let resp = next.run(request).await;
    // tower_http::ServeDir redirects paths to directories without trailing slash to the version with trailing slash with a 307 temporary redirect. This causes double counting of hits in those situations.
    if resp.status() != StatusCode::TEMPORARY_REDIRECT {
        let mut data_mux = app_state.data.lock().unwrap();
        data_mux.fileserver_hits += 1;
    }
    resp
}
