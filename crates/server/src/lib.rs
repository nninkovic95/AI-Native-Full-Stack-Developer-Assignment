pub mod auth;
pub mod docs;
pub mod error;
pub mod import;
pub mod seed;
pub mod shares;

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
pub use error::AppError;
use shared::{ApiError, Health, MAX_IMPORT_BYTES};
use sqlx::PgPool;
use tower_http::services::{ServeDir, ServeFile};

/// Largest JSON body the document routes accept, set here so it does not move with the framework default.
const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// Adds `Secure` to the session cookie; on behind Render's HTTPS, off for local http.
    pub cookie_secure: bool,
}

pub fn app(state: AppState, dist: &str) -> Router {
    // Unknown paths fall back to index.html so client-side routes survive a refresh.
    let spa = ServeDir::new(dist).fallback(ServeFile::new(format!("{dist}/index.html")));
    Router::new()
        .nest("/api", api(state))
        .fallback_service(spa)
}

fn api(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/me", get(auth::me))
        .route(
            "/documents",
            get(docs::list)
                .post(docs::create)
                .layer(DefaultBodyLimit::max(MAX_DOCUMENT_BYTES)),
        )
        .route(
            "/documents/import",
            // Room for the multipart framing around a file of the largest accepted size.
            post(import::upload).layer(DefaultBodyLimit::max(MAX_IMPORT_BYTES + 64 * 1024)),
        )
        .route(
            "/documents/{id}",
            get(docs::get_one)
                .patch(docs::update)
                .layer(DefaultBodyLimit::max(MAX_DOCUMENT_BYTES)),
        )
        .route("/documents/{id}/shares", get(shares::list).post(shares::create))
        // Without its own fallback, an unknown API path would be answered with the web page.
        .fallback(api_not_found)
        .method_not_allowed_fallback(api_method_not_allowed)
        .with_state(state)
}

async fn api_not_found() -> (StatusCode, Json<ApiError>) {
    (StatusCode::NOT_FOUND, Json(ApiError { error: "Not found".into() }))
}

async fn api_method_not_allowed() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(ApiError { error: "Method not allowed".into() }),
    )
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
    })
}
