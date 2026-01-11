//! Embedded static assets for the web client.
//!
//! All files from the `dist/` directory are embedded at compile time,
//! except for `valve.zip` which must be provided separately at runtime.

use axum::body::Body;
use axum::http::{header, Response, StatusCode};
use rust_embed::RustEmbed;

/// Embedded assets from the dist folder (excludes valve.zip)
#[derive(RustEmbed)]
#[folder = "dist/"]
#[exclude = "valve.zip"]
pub struct Assets;

/// Serve an embedded asset by path
pub fn serve_embedded(path: &str) -> Response<Body> {
    // Normalize path - remove leading slash
    let path = path.trim_start_matches('/');

    // Handle root path
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .body(Body::from(content.data.into_owned()))
                .unwrap_or_else(|_| internal_error())
        }
        None => not_found(),
    }
}

/// Return a 404 response
fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap_or_else(|_| internal_error())
}

/// Return a 500 response
fn internal_error() -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("Internal Server Error"))
        .expect("building error response should not fail")
}
