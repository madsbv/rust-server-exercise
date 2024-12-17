#![feature(let_chains)]

use api::{get_all_chirps, post_chirp};
use axum::{
    handler::HandlerWithoutStateExt,
    middleware::{self},
    routing::{get, post},
    Extension, Router,
};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;

mod admin;
mod api;
mod list_dir;
mod middlewarez;
mod queries;
mod state;

use self::{
    admin::{metrics, reset},
    api::create_user,
    list_dir::{servedir_fallback, static_fallback},
    middlewarez::fileserver_hits_middleware,
    state::{AppState, Platform},
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Environment variables must be set in .env");
    let db_url = dotenvy::var("DATABASE_URL").expect("Database url must be set");

    let db = PgPoolOptions::new()
        .connect(&db_url)
        .await
        .expect("Database must be available");

    let platform: Platform = Platform::from(
        dotenvy::var("PLATFORM")
            .unwrap_or("prod".to_string())
            .as_str(),
    );

    let mut app_state = AppState::new();
    app_state.config.platform = platform;

    let file_server = ServeDir::new("").fallback(servedir_fallback.into_service());

    let app_router = Router::new()
        .route_service("/app/*path", file_server.clone())
        .route_service("/app", file_server.clone())
        .route_service("/app/", file_server.clone())
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            app_state.clone(),
            fileserver_hits_middleware,
        )));

    let admin_router = Router::new()
        .route("/metrics", get(metrics))
        .route("/reset", post(reset));

    let api_router = Router::new()
        .route("/healthz", get(healthz))
        .route("/chirps", post(post_chirp))
        .route("/chirps", get(get_all_chirps))
        .route("/users", post(create_user));

    let main_router = Router::new()
        .merge(app_router)
        .nest("/api", api_router)
        .nest("/admin", admin_router)
        .fallback(static_fallback)
        .with_state(app_state)
        .layer(Extension(db));

    // run our app with hyper, listening globally on port 8080
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, main_router).await.unwrap();
}

// `String` implements `IntoResponse`; the response will have statuscode 200 and `text/plain; charset=utf-8` content-type.
async fn healthz() -> String {
    "OK".to_string()
}
