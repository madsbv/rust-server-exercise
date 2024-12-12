use tokio::fs;

use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse},
};

use tokio_stream::{wrappers::ReadDirStream, StreamExt};

pub async fn static_fallback() -> impl IntoResponse {
    (StatusCode::OK, "No such file".to_string())
}

pub async fn servedir_fallback(Path(path): Path<String>) -> impl IntoResponse {
    let path = format!("app/{path}");
    let metadata = fs::metadata(&path).await;
    let Ok(metadata) = metadata else {
        return static_fallback().await.into_response();
    };

    if metadata.is_dir()
        && let Ok(listing) = list_dir(&path).await
    {
        (StatusCode::OK, Html::from(listing)).into_response()
    } else {
        static_fallback().await.into_response()
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
