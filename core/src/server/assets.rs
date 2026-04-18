use axum::{
    body::Body,
    http::{header, Response, StatusCode},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../webui/dist"]
pub struct Assets;

/// Sirve un fichero embebido. Si no existe devuelve `index.html` (SPA fallback).
/// Si `index.html` tampoco existe (webui no compilada), devuelve 404.
pub async fn serve(path: axum::extract::Path<String>) -> impl IntoResponse {
    serve_path(&path.0).await
}

pub async fn serve_root() -> impl IntoResponse {
    serve_path("index.html").await
}

async fn serve_path(path: &str) -> Response<Body> {
    let path = path.trim_start_matches('/');

    if let Some(content) = Assets::get(path) {
        let mime = content.metadata.mimetype();
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .body(Body::from(content.data.into_owned()))
            .unwrap()
    } else if let Some(index) = Assets::get("index.html") {
        let mime = index.metadata.mimetype();
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .body(Body::from(index.data.into_owned()))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()
    }
}
