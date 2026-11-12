use axum::{routing::get, Json, Router};
use shared::Health;
use tower_http::services::{ServeDir, ServeFile};

pub fn app(dist: &str) -> Router {
    // Unknown paths fall back to index.html so client-side routes survive a refresh.
    let spa = ServeDir::new(dist).fallback(ServeFile::new(format!("{dist}/index.html")));
    Router::new()
        .nest("/api", api())
        .fallback_service(spa)
}

fn api() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
    })
}
