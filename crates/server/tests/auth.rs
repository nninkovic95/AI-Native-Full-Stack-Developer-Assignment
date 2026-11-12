mod common;

use axum::http::{header, Method, StatusCode};
use common::*;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn login_sets_a_secure_http_only_session_cookie(pool: PgPool) {
    let app = app(pool).await;

    let res = login_raw(&app, ALICE, PASSWORD).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["email"], ALICE);
    let cookie = res.headers[header::SET_COOKIE].to_str().unwrap();
    assert!(cookie.starts_with("session="), "{cookie}");
    for flag in ["HttpOnly", "SameSite=Lax", "Path=/", "Secure"] {
        assert!(cookie.contains(flag), "missing {flag} in {cookie}");
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn wrong_password_is_rejected_without_a_cookie(pool: PgPool) {
    let app = app(pool).await;

    let res = login_raw(&app, ALICE, "not-the-password").await;

    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.body["error"], "Wrong email or password");
    assert!(res.headers.get(header::SET_COOKIE).is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn me_returns_the_signed_in_user(pool: PgPool) {
    let app = app(pool).await;
    let cookie = login(&app, BOB).await;

    let res = send(&app, Method::GET, "/api/me", Some(&cookie), None).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["email"], BOB);
    assert_eq!(res.body["name"], "Bob");
}

#[sqlx::test(migrations = "../../migrations")]
async fn me_without_a_valid_session_is_401(pool: PgPool) {
    let app = app(pool).await;

    let none = send(&app, Method::GET, "/api/me", None, None).await;
    let forged = send(&app, Method::GET, "/api/me", Some("session=forged"), None).await;

    assert_eq!(none.status, StatusCode::UNAUTHORIZED);
    assert_eq!(forged.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn logout_ends_the_session(pool: PgPool) {
    let app = app(pool).await;
    let cookie = login(&app, ALICE).await;

    let res = send(&app, Method::POST, "/api/logout", Some(&cookie), None).await;
    let after = send(&app, Method::GET, "/api/me", Some(&cookie), None).await;

    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
}
