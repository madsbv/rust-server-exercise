use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    Extension,
};

use super::AppState;

#[derive(Clone)]
pub struct RequestState {
    has_been_counted: bool,
}

impl RequestState {
    fn new() -> Self {
        Self {
            has_been_counted: false,
        }
    }
}

pub async fn initialize_request_state(mut request: Request, next: Next) -> Response {
    request.extensions_mut().insert(RequestState::new());

    println!("Initialize request state");
    println!("{request:?}");
    let resp = next.run(request).await;
    println!("{resp:?}");
    resp
}

pub async fn fileserver_hits_middleware(
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
