use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let dist = std::env::var("DIST_DIR").unwrap_or_else(|_| "dist".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let app = server::app(&dist);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}, serving frontend from {dist}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind port");
    axum::serve(listener, app).await.expect("server error");
}
