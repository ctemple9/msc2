//! The optional, packaged browser UI.  Its files are byte-for-byte copies of
//! the Vite output Tauri loads; this module only maps those bytes to HTTP.

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use include_dir::{Dir, include_dir};
use msc_api::dto::ErrorDto;

#[cfg(feature = "web-ui")]
static BUNDLE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/web-ui");

/// Serves the client bundle as the final application fallback.  API routes
/// are deliberately excluded so a typo under `/v1` remains an API 404 instead
/// of silently becoming the application page.
pub async fn serve(request: axum::extract::Request) -> Response {
    let path = request.uri().path();
    if path == "/v1" || path.starts_with("/v1/") {
        return not_found();
    }

    #[cfg(feature = "web-ui")]
    {
        embedded(path)
    }

    #[cfg(not(feature = "web-ui"))]
    {
        unavailable()
    }
}

#[cfg(feature = "web-ui")]
fn embedded(path: &str) -> Response {
    let relative = path.trim_start_matches('/');
    if relative.split('/').any(|segment| segment == "..") {
        return not_found();
    }

    if let Some(file) = BUNDLE.get_file(relative) {
        return file_response(relative, file.contents());
    }

    // A filename-like missing resource is not a client route.  Keeping it a
    // 404 lets the browser report a broken asset instead of booting an app
    // which cannot load its JavaScript.
    if relative
        .rsplit('/')
        .next()
        .is_some_and(|name| name.contains('.'))
    {
        return not_found();
    }

    match BUNDLE.get_file("index.html") {
        Some(index) => file_response("index.html", index.contents()),
        None => unavailable(),
    }
}

#[cfg(feature = "web-ui")]
fn file_response(path: &str, contents: &'static [u8]) -> Response {
    let mut response = Response::new(Body::from(contents));
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, mime_type(path));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if path.starts_with("assets/") {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        }),
    );
    apply_ui_headers(headers);
    response
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(ErrorDto {
            code: "web_ui_unavailable".to_owned(),
            message: "This headless MSC package does not include the browser interface.".to_owned(),
            help_id: None,
            details: None,
        }),
    )
        .into_response()
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        axum::Json(ErrorDto {
            code: "not_found".to_owned(),
            message: "No matching web resource exists.".to_owned(),
            help_id: None,
            details: None,
        }),
    )
        .into_response()
}

#[cfg(feature = "web-ui")]
fn mime_type(path: &str) -> HeaderValue {
    let value = match path.rsplit('.').next() {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("mp4") => "video/mp4",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        _ => "text/html; charset=utf-8",
    };
    HeaderValue::from_static(value)
}

#[cfg(feature = "web-ui")]
fn apply_ui_headers(headers: &mut axum::http::HeaderMap) {
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static(
            "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: blob:; connect-src 'self'; worker-src 'self' blob:",
        ),
    );
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("Referrer-Policy", HeaderValue::from_static("no-referrer"));
}
