mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;

async fn create_doc(app: &axum::Router, cookie: &str) -> String {
    let res = send(app, Method::POST, "/api/documents", Some(cookie), Some(json!({}))).await;
    assert_eq!(res.status, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_owned()
}

async fn list_ids(app: &axum::Router, cookie: &str) -> Vec<String> {
    let res = send(app, Method::GET, "/api/documents", Some(cookie), None).await;
    assert_eq!(res.status, StatusCode::OK);
    res.body
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_owner_deletes_their_document(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let kept = create_doc(&app, &alice).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");

    let deleted = send(&app, Method::DELETE, &url, Some(&alice), None).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(deleted.body, Value::Null);
    assert_eq!(reopened.status, StatusCode::NOT_FOUND);
    assert_eq!(reopened.body["error"], "Document not found");
    assert_eq!(list_ids(&app, &alice).await, [kept]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_user_without_access_cannot_delete_and_gets_404(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let carol = login(&app, CAROL).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");

    let deleted = send(&app, Method::DELETE, &url, Some(&carol), None).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(deleted.status, StatusCode::NOT_FOUND);
    assert_eq!(deleted.body["error"], "Document not found");
    assert_eq!(reopened.status, StatusCode::OK);
    assert_eq!(reopened.body["title"], "Untitled document");
}

async fn share(app: &axum::Router, cookie: &str, id: &str, email: &str) {
    let res = send(
        app,
        Method::POST,
        &format!("/api/documents/{id}/shares"),
        Some(cookie),
        Some(json!({ "email": email })),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_shared_user_cannot_delete_and_gets_403(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    share(&app, &alice, &id, "bob@ajaia.test").await;

    let deleted = send(&app, Method::DELETE, &url, Some(&bob), None).await;
    let reopened_by_owner = send(&app, Method::GET, &url, Some(&alice), None).await;
    let reopened_by_bob = send(&app, Method::GET, &url, Some(&bob), None).await;

    assert_eq!(deleted.status, StatusCode::FORBIDDEN);
    assert_eq!(deleted.body["error"], "Only the owner can delete this document");
    assert_eq!(reopened_by_owner.status, StatusCode::OK);
    assert_eq!(reopened_by_bob.status, StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_deleted_document_is_gone_for_the_users_it_was_shared_with(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let kept = create_doc(&app, &alice).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    share(&app, &alice, &kept, "bob@ajaia.test").await;
    share(&app, &alice, &id, "bob@ajaia.test").await;

    let deleted = send(&app, Method::DELETE, &url, Some(&alice), None).await;
    let opened = send(&app, Method::GET, &url, Some(&bob), None).await;
    let edited = send(&app, Method::PATCH, &url, Some(&bob), Some(json!({ "title": "still here" }))).await;
    let shares = send(&app, Method::GET, &format!("{url}/shares"), Some(&alice), None).await;

    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(opened.status, StatusCode::NOT_FOUND);
    assert_eq!(opened.body["error"], "Document not found");
    assert_eq!(edited.status, StatusCode::NOT_FOUND);
    assert_eq!(edited.body["error"], "Document not found");
    assert_eq!(shares.status, StatusCode::NOT_FOUND);
    assert_eq!(list_ids(&app, &bob).await, [kept.clone()]);
    assert_eq!(list_ids(&app, &alice).await, [kept]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleting_requires_sign_in(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");

    let deleted = send(&app, Method::DELETE, &url, None, None).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(deleted.status, StatusCode::UNAUTHORIZED);
    assert_eq!(deleted.body, json!({ "error": "Not signed in" }));
    assert_eq!(reopened.status, StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleting_an_id_that_is_not_a_uuid_is_404(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let malformed = send(&app, Method::DELETE, "/api/documents/not-a-uuid", Some(&alice), None).await;
    let unknown = send(
        &app,
        Method::DELETE,
        "/api/documents/00000000-0000-4000-8000-000000000000",
        Some(&alice),
        None,
    )
    .await;

    assert_eq!(malformed.status, StatusCode::NOT_FOUND);
    assert_eq!(malformed.body, json!({ "error": "Document not found" }));
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(unknown.body, json!({ "error": "Document not found" }));
}
