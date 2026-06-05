use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::app_state::AppState;

pub async fn static_fallback(
    State(state): State<AppState>,
    uri: Uri,
) -> Response {
    let path = uri.path();
    let serve = ServeDir::new(&state.static_dir).append_index_html_on_directories(true);

    // Try appending .html first for Next.js clean URLs
    // (avoids ServeDir's trailing-slash redirect when a same-name directory exists)
    let clean = path.trim_end_matches('/');
    if !clean.is_empty() {
        if let Ok(html_uri) = format!("{}.html", clean).parse::<Uri>() {
            if let Ok(req) = Request::builder().uri(html_uri).body(Body::empty()) {
                let resp = serve.clone().oneshot(req).await.expect("ServeDir is infallible");
                if resp.status() != StatusCode::NOT_FOUND {
                    return resp.into_response();
                }
            }
        }
    }

    // Fall through to exact path (handles / -> index.html, _next/static/...)
    if let Ok(req) = Request::builder().uri(uri).body(Body::empty()) {
        let resp = serve.oneshot(req).await.expect("ServeDir is infallible");
        return resp.into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}
