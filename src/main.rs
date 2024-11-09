#![feature(let_chains)]

use std::{
    convert::Infallible,
    task::{Context, Poll},
};

use tokio::fs;

use axum::{
    extract::Path,
    handler::{Handler, HandlerWithoutStateExt},
    http::{
        header::{self, HeaderMap, HeaderName},
        StatusCode, Uri,
    },
    response::{Html, IntoResponse},
    routing::{any, get},
    Json, Router,
};
use tokio_stream::{wrappers::ReadDirStream, StreamExt};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    // let file_server = ServeDir::new("static").append_index_html_on_directories(false);
    let file_server = ServeDir::new("").fallback(servedir_fallback.into_service());

    // build our application with a separate router
    let app_router = Router::new()
        .route_service("/app/*path", file_server.clone())
        .route_service("/app/", file_server.clone())
        .route_service("/app", file_server);

    let main_router = Router::new()
        .merge(app_router)
        // .nest("/app/", app_router)
        .route("/healthz", any(healthz))
        .fallback(static_fallback);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, main_router).await.unwrap();
}

// `String` implements `IntoResponse`; the response will have statuscode 200 and `text/plain; charset=utf-8` content-type.
async fn healthz() -> String {
    "OK".to_string()
}

async fn static_fallback(uri: Uri) -> impl IntoResponse {
    println!("{uri:?}");
    (StatusCode::OK, "No such file".to_string())
}

async fn servedir_fallback(Path(path): Path<String>, uri: Uri) -> impl IntoResponse {
    println!("path: {path}, uri: {uri:?}");
    let path = format!("app/{path}");
    println!("path: {path}, uri: {uri:?}");
    let metadata = fs::metadata(&path).await;
    println!("{metadata:?}");
    let Ok(metadata) = metadata else {
        return static_fallback(uri).await.into_response();
    };

    if metadata.is_dir()
        && let Ok(listing) = list_dir(&path).await
    {
        (StatusCode::OK, Html::from(listing)).into_response()
    } else {
        static_fallback(uri).await.into_response()
    }
}

async fn list_dir(path: &str) -> std::io::Result<String> {
    let dir_entries = ReadDirStream::new(fs::read_dir(path).await?);
    let file_links: Vec<String> = dir_entries
        .filter_map(|rf| rf.ok().map(|f| f.file_name()))
        .filter_map(|f| f.into_string().ok())
        .map(|f| format!("<a href=\"{f}\">{f}</a>",))
        .collect()
        .await;
    let file_links = file_links.join("\n");
    Ok(format!("<pre>\n{file_links}\n</pre>"))
}
