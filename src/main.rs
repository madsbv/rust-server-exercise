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

#[derive(Clone)]
struct RequestState {
    has_been_counted: bool,
}

impl RequestState {
    fn new() -> Self {
        Self {
            has_been_counted: false,
        }
    }
}

async fn initialize_request_state(mut request: Request, next: Next) -> Response {
    request.extensions_mut().insert(RequestState::new());

    println!("Initialize request state");
    println!("{request:?}");
    let resp = next.run(request).await;
    println!("{resp:?}");
    resp
}

async fn fileserver_hits_middleware(
    State(app_state): State<AppState>,
    Extension(mut request_state): Extension<RequestState>,
    // you can add more extractors here but the last
    // extractor must implement `FromRequest` which
    // `Request` does
    request: Request,
    next: Next,
) -> Response {
    if !request_state.has_been_counted && !request_is_self_redirect(&request) {
        let mut data_mux = app_state.data.lock().unwrap();
        data_mux.fileserver_hits += 1;
        request_state.has_been_counted = true;
    }
    println!("Fileserver hits middleware");
    println!("{request:?}");

    let resp = next.run(request).await;
    println!("{resp:?}");
    resp
}

fn request_is_self_redirect(request: &Request) -> bool {
    if let Some(referer) = request.headers().get("referer") {
        if let Ok(referer_str) = referer.to_str() {
            // TODO: This is a very specific hack, need another method in general--probably wrapping ServeDir more directly to avoid the redirects in the first place.
            // We can potentially also inspect responses and handle 307 redirects in the fileserver hits middleware
            return referer_str.contains("localhost");
        }
    }
    false
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
