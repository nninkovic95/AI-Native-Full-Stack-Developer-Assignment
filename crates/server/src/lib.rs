pub mod auth;
pub mod docs;
pub mod error;
pub mod import;
pub mod seed;

use axum::{
    routing::{get, post},
    Json, Router,
};
pub use error::AppError;
use shared::Health;
use sqlx::PgPool;
use tower_http::services::{ServeDir, ServeFile};

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
        .route("/documents", get(docs::list).post(docs::create))
        .route("/documents/{id}", get(docs::get_one).patch(docs::update))
        .with_state(state)
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
    })
}
