//! Servicio de ficheros estáticos de la WebUI embebidos en el binario con `rust-embed`.

use axum::{
    body::Body,
    http::{header, Response, StatusCode},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

/// Activos estáticos de la WebUI embebidos en tiempo de compilación desde `../webui/dist`.
#[derive(RustEmbed)]
#[folder = "../webui/dist"]
pub struct Assets;

/// Sirve un fichero embebido. Si no existe devuelve `index.html` (SPA fallback).
/// Si `index.html` tampoco existe (webui no compilada), devuelve 404.
pub async fn serve(path: axum::extract::Path<String>) -> impl IntoResponse {
    serve_path(&path.0).await
}

/// Sirve `index.html` directamente en la ruta raíz `/`.
pub async fn serve_root() -> impl IntoResponse {
    serve_path("index.html").await
}

/// Busca y devuelve el fichero embebido en `path`, con fallback a `index.html` para SPA routing.
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
