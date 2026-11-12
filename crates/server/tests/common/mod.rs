#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use server::{seed, AppState};
use sqlx::PgPool;
use tower::ServiceExt;

pub const PASSWORD: &str = seed::DEMO_PASSWORD;
pub const ALICE: &str = "alice@ajaia.test";
pub const BOB: &str = "bob@ajaia.test";
pub const CAROL: &str = "carol@ajaia.test";

pub struct Resp {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Value,
}

pub async fn app(pool: PgPool) -> Router {
    seed::seed_users(&pool).await.expect("seed users");
    server::app(
        AppState {
            pool,
            cookie_secure: true,
        },
        "dist-not-built",
    )
}

pub async fn send(app: &Router, method: Method, uri: &str, cookie: Option<&str>, body: Option<Value>) -> Resp {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()))
    };
    Resp { status, headers, body }
}

pub async fn login_raw(app: &Router, email: &str, password: &str) -> Resp {
    send(
        app,
        Method::POST,
        "/api/login",
        None,
        Some(json!({ "email": email, "password": password })),
    )
    .await
}

/// Logs in and returns the `name=value` pair to send back as a Cookie header.
pub async fn login(app: &Router, email: &str) -> String {
    let res = login_raw(app, email, PASSWORD).await;
    assert_eq!(res.status, StatusCode::OK, "login failed: {:?}", res.body);
    let set_cookie = res.headers[header::SET_COOKIE].to_str().unwrap();
    set_cookie.split(';').next().unwrap().to_string()
}
