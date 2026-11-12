use std::net::SocketAddr;

use server::{seed, AppState};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let dist = std::env::var("DIST_DIR").unwrap_or_else(|_| "dist".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let cookie_secure = std::env::var("COOKIE_SECURE").is_ok_and(|v| v == "true");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect to database");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    seed::seed_users(&pool).await.expect("seed users");

    let app = server::app(AppState { pool, cookie_secure }, &dist);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}, serving frontend from {dist}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind port");
    axum::serve(listener, app).await.expect("server error");
}
