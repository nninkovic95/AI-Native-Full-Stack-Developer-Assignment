mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use common::*;
use serde_json::json;
use sqlx::PgPool;

async fn create_doc(app: &axum::Router, cookie: &str) -> String {
    let res = send(app, Method::POST, "/api/documents", Some(cookie), Some(json!({}))).await;
    assert_eq!(res.status, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_owned()
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_new_document_title_is_trimmed_before_saving(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::POST, "/api/documents", Some(&alice), Some(json!({ "title": "  Plan  " }))).await;

    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["title"], "Plan");
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_renamed_document_title_is_trimmed_before_saving(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let url = format!("/api/documents/{}", create_doc(&app, &alice).await);

    let res = send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "title": "  Plan  " }))).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(reopened.body["title"], "Plan");
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_new_document_cannot_have_a_blank_title(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::POST, "/api/documents", Some(&alice), Some(json!({ "title": "   " }))).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Title cannot be empty" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_document_cannot_be_renamed_to_a_blank_title(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let url = format!("/api/documents/{}", create_doc(&app, &alice).await);
    send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "title": "Kept" }))).await;

    let res = send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "title": "   " }))).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Title cannot be empty" }));
    assert_eq!(reopened.body["title"], "Kept");
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_new_document_title_over_200_characters_is_rejected(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::POST, "/api/documents", Some(&alice), Some(json!({ "title": "a".repeat(201) }))).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Title is too long (200 characters at most)" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_title_of_exactly_200_characters_is_accepted_whatever_its_byte_length(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let url = format!("/api/documents/{}", create_doc(&app, &alice).await);
    let title = "é".repeat(200);

    let res = send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "title": title }))).await;
    let too_long = send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "title": "é".repeat(201) }))).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(too_long.status, StatusCode::BAD_REQUEST);
    assert_eq!(too_long.body, json!({ "error": "Title is too long (200 characters at most)" }));
    assert_eq!(reopened.body["title"], title);
}

/// Saves a known body, tries to overwrite it with `content`, and returns the rejection and the body found afterwards.
async fn patch_content_over_saved(pool: PgPool, content: serde_json::Value) -> (Resp, serde_json::Value) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let url = format!("/api/documents/{}", create_doc(&app, &alice).await);
    send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "content": { "ops": [{ "insert": "saved\n" }] } }))).await;

    let res = send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "content": content }))).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    (res, reopened.body["content"].clone())
}

#[sqlx::test(migrations = "../../migrations")]
async fn content_that_is_not_an_object_is_rejected_and_nothing_is_saved(pool: PgPool) {
    let (res, stored) = patch_content_over_saved(pool, json!("just a string")).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Content is not a valid document" }));
    assert_eq!(stored, json!({ "ops": [{ "insert": "saved\n" }] }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn content_whose_ops_is_not_a_list_is_rejected_and_nothing_is_saved(pool: PgPool) {
    let (res, stored) = patch_content_over_saved(pool, json!({ "ops": "x" })).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Content is not a valid document" }));
    assert_eq!(stored, json!({ "ops": [{ "insert": "saved\n" }] }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn content_with_an_op_that_inserts_nothing_is_rejected_and_nothing_is_saved(pool: PgPool) {
    let (res, stored) =
        patch_content_over_saved(pool, json!({ "ops": [{ "insert": "fine" }, { "retain": 3 }] })).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Content is not a valid document" }));
    assert_eq!(stored, json!({ "ops": [{ "insert": "saved\n" }] }));
}

/// Sends a body exactly as given, with no Content-Type unless one is passed.
async fn send_raw(
    app: &axum::Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    content_type: Option<&str>,
    body: impl Into<Body>,
) -> Resp {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    if let Some(t) = content_type {
        req = req.header(header::CONTENT_TYPE, t);
    }
    dispatch(app, req.body(body.into()).unwrap()).await
}

#[sqlx::test(migrations = "../../migrations")]
async fn login_with_a_body_that_is_not_json_is_a_readable_400(pool: PgPool) {
    let app = app(pool).await;

    let res = send_raw(&app, Method::POST, "/api/login", None, Some("application/json"), "{ not json").await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Invalid request body" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn creating_a_document_with_a_mistyped_field_is_a_readable_400(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::POST, "/api/documents", Some(&alice), Some(json!({ "title": 5 }))).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Invalid request body" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn saving_a_document_without_a_content_type_is_a_readable_400(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let url = format!("/api/documents/{}", create_doc(&app, &alice).await);

    let res = send_raw(&app, Method::PATCH, &url, Some(&alice), None, r#"{"title":"No header"}"#).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Invalid request body" }));
    assert_eq!(reopened.body["title"], "Untitled document");
}

#[sqlx::test(migrations = "../../migrations")]
async fn sharing_with_a_mistyped_email_field_is_a_readable_400(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let url = format!("/api/documents/{}/shares", create_doc(&app, &alice).await);

    let res = send(&app, Method::POST, &url, Some(&alice), Some(json!({ "email": 1 }))).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Invalid request body" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn opening_a_document_by_an_id_that_is_not_a_uuid_is_not_found(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::GET, "/api/documents/not-a-uuid", Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.body, json!({ "error": "Document not found" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn saving_a_document_by_an_id_that_is_not_a_uuid_is_not_found(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::PATCH, "/api/documents/not-a-uuid", Some(&alice), Some(json!({ "title": "x" }))).await;

    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.body, json!({ "error": "Document not found" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn listing_the_shares_of_an_id_that_is_not_a_uuid_is_not_found(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::GET, "/api/documents/not-a-uuid/shares", Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.body, json!({ "error": "Document not found" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn sharing_an_id_that_is_not_a_uuid_is_not_found(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::POST, "/api/documents/not-a-uuid/shares", Some(&alice), Some(json!({ "email": BOB }))).await;

    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.body, json!({ "error": "Document not found" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_document_body_over_2_mb_is_a_readable_413_and_nothing_is_saved(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let url = format!("/api/documents/{}", create_doc(&app, &alice).await);
    let huge = json!({ "content": { "ops": [{ "insert": "a".repeat(2 * 1024 * 1024 + 1) }] } });

    let res = send(&app, Method::PATCH, &url, Some(&alice), Some(huge)).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(res.body, json!({ "error": "The document is too large" }));
    assert_eq!(reopened.body["content"], json!({ "ops": [] }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_unknown_api_path_is_a_json_404_not_the_web_page(pool: PgPool) {
    let app = app(pool).await;

    let res = send(&app, Method::GET, "/api/nope", None, None).await;

    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.body, json!({ "error": "Not found" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_wrong_method_on_a_known_api_path_is_a_json_405(pool: PgPool) {
    let app = app(pool).await;

    let res = send(&app, Method::DELETE, "/api/login", None, None).await;

    assert_eq!(res.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(res.body, json!({ "error": "Method not allowed" }));
}
