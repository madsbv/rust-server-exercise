#![feature(let_chains)]

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Request, State},
    handler::HandlerWithoutStateExt,
    http::HeaderMap,
    middleware::{self, Next},
    response::Response,
    routing::any,
    Extension, Router,
};
use tower::ServiceBuilder;
use tower_http::services::ServeDir;

mod list_dir;
use self::list_dir::{servedir_fallback, static_fallback};
mod middlewarez;
use self::middlewarez::{fileserver_hits_middleware, initialize_request_state};

#[derive(Clone)]
struct AppState {
    data: Arc<Mutex<AppStateData>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(AppStateData::new())),
        }
    }
}

struct AppStateData {
    fileserver_hits: u64,
}

impl AppStateData {
    fn new() -> Self {
        Self { fileserver_hits: 0 }
    }
}

#[tokio::main]
async fn main() {
    let app_state = AppState::new();

    // let file_server = ServeDir::new("static").append_index_html_on_directories(false);
    let file_server = ServeDir::new("").fallback(servedir_fallback.into_service());

    // build our application with a separate router
    let app_router = Router::new()
        .route_service("/app/*path", file_server.clone())
        .route_service("/app", file_server.clone())
        .route_service("/app/", file_server.clone())
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(initialize_request_state))
                .layer(middleware::from_fn_with_state(
                    app_state.clone(),
                    fileserver_hits_middleware,
                )),
        );

    let main_router = Router::new()
        .merge(app_router)
        // .nest("/app/", app_router)
        .route("/healthz", any(healthz))
        .route("/metrics", any(fileserver_hits))
        .route("/reset", any(reset_fileserver_hits))
        .fallback(static_fallback)
        .with_state(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, main_router).await.unwrap();
}

async fn fileserver_hits(State(state): State<AppState>) -> String {
    let hits = { state.data.lock().unwrap().fileserver_hits };
    format!("Hits: {hits}")
}
async fn reset_fileserver_hits(State(state): State<AppState>) {
    state.data.lock().unwrap().fileserver_hits = 0;
}

// `String` implements `IntoResponse`; the response will have statuscode 200 and `text/plain; charset=utf-8` content-type.
async fn healthz() -> String {
    "OK".to_string()
}
