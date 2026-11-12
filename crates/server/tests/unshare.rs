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

/// Shares the document and returns the id of the user it is now shared with.
async fn share(app: &axum::Router, cookie: &str, id: &str, email: &str) -> String {
    let res = send(
        app,
        Method::POST,
        &format!("/api/documents/{id}/shares"),
        Some(cookie),
        Some(json!({ "email": email })),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_owned()
}

async fn shared_emails(app: &axum::Router, cookie: &str, id: &str) -> Vec<String> {
    let res = send(app, Method::GET, &format!("/api/documents/{id}/shares"), Some(cookie), None).await;
    assert_eq!(res.status, StatusCode::OK);
    res.body
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["email"].as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_owner_removes_a_shared_user(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    let bob_id = share(&app, &alice, &id, "bob@ajaia.test").await;

    let removed = send(&app, Method::DELETE, &format!("{url}/shares/{bob_id}"), Some(&alice), None).await;
    let opened = send(&app, Method::GET, &url, Some(&bob), None).await;

    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    assert_eq!(removed.body, Value::Null);
    assert_eq!(opened.status, StatusCode::NOT_FOUND);
    assert_eq!(opened.body["error"], "Document not found");
    assert_eq!(shared_emails(&app, &alice, &id).await, Vec::<String>::new());
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
async fn removing_one_person_leaves_the_others_with_access(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let carol = login(&app, CAROL).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    let bob_id = share(&app, &alice, &id, "bob@ajaia.test").await;
    share(&app, &alice, &id, "carol@ajaia.test").await;

    let removed = send(&app, Method::DELETE, &format!("{url}/shares/{bob_id}"), Some(&alice), None).await;
    let opened_by_carol = send(&app, Method::GET, &url, Some(&carol), None).await;
    let edited_by_carol = send(&app, Method::PATCH, &url, Some(&carol), Some(json!({ "title": "still mine to edit" }))).await;
    let edited_by_bob = send(&app, Method::PATCH, &url, Some(&bob), Some(json!({ "title": "let me back in" }))).await;

    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    assert_eq!(opened_by_carol.status, StatusCode::OK);
    assert_eq!(edited_by_carol.status, StatusCode::OK);
    assert_eq!(edited_by_carol.body["title"], "still mine to edit");
    assert_eq!(edited_by_bob.status, StatusCode::NOT_FOUND);
    assert_eq!(edited_by_bob.body["error"], "Document not found");
    assert_eq!(shared_emails(&app, &alice, &id).await, ["carol@ajaia.test"]);
    assert_eq!(list_ids(&app, &carol).await, [id.clone()]);
    assert_eq!(list_ids(&app, &bob).await, Vec::<String>::new());
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_shared_user_cannot_remove_anyone_not_even_themself(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    let bob_id = share(&app, &alice, &id, "bob@ajaia.test").await;
    let carol_id = share(&app, &alice, &id, "carol@ajaia.test").await;

    let removed_other = send(&app, Method::DELETE, &format!("{url}/shares/{carol_id}"), Some(&bob), None).await;
    let removed_self = send(&app, Method::DELETE, &format!("{url}/shares/{bob_id}"), Some(&bob), None).await;
    let opened_by_bob = send(&app, Method::GET, &url, Some(&bob), None).await;

    assert_eq!(removed_other.status, StatusCode::FORBIDDEN);
    assert_eq!(removed_other.body, json!({ "error": "Only the owner can change who has access" }));
    assert_eq!(removed_self.status, StatusCode::FORBIDDEN);
    assert_eq!(removed_self.body, json!({ "error": "Only the owner can change who has access" }));
    assert_eq!(opened_by_bob.status, StatusCode::OK);
    assert_eq!(shared_emails(&app, &alice, &id).await, ["bob@ajaia.test", "carol@ajaia.test"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_user_without_access_cannot_remove_anyone_and_gets_404(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let carol = login(&app, CAROL).await;
    let id = create_doc(&app, &alice).await;
    let bob_id = share(&app, &alice, &id, "bob@ajaia.test").await;

    let removed = send(&app, Method::DELETE, &format!("/api/documents/{id}/shares/{bob_id}"), Some(&carol), None).await;
    let unknown = send(
        &app,
        Method::DELETE,
        &format!("/api/documents/00000000-0000-4000-8000-000000000000/shares/{bob_id}"),
        Some(&carol),
        None,
    )
    .await;
    let malformed = send(&app, Method::DELETE, &format!("/api/documents/not-a-uuid/shares/{bob_id}"), Some(&carol), None).await;

    assert_eq!(removed.status, StatusCode::NOT_FOUND);
    assert_eq!(removed.body, json!({ "error": "Document not found" }));
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(unknown.body, json!({ "error": "Document not found" }));
    assert_eq!(malformed.status, StatusCode::NOT_FOUND);
    assert_eq!(malformed.body, json!({ "error": "Document not found" }));
    assert_eq!(shared_emails(&app, &alice, &id).await, ["bob@ajaia.test"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn removing_someone_the_document_is_not_shared_with_is_404(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let carol = login(&app, CAROL).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    share(&app, &alice, &id, "bob@ajaia.test").await;
    let me = send(&app, Method::GET, "/api/me", Some(&carol), None).await;
    let carol_id = me.body["id"].as_str().unwrap();

    let not_shared = send(&app, Method::DELETE, &format!("{url}/shares/{carol_id}"), Some(&alice), None).await;
    let malformed = send(&app, Method::DELETE, &format!("{url}/shares/not-a-uuid"), Some(&alice), None).await;

    assert_eq!(not_shared.status, StatusCode::NOT_FOUND);
    assert_eq!(not_shared.body, json!({ "error": "That person does not have access" }));
    assert_eq!(malformed.status, StatusCode::NOT_FOUND);
    assert_eq!(malformed.body, json!({ "error": "That person does not have access" }));
    assert_eq!(shared_emails(&app, &alice, &id).await, ["bob@ajaia.test"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn removing_access_requires_sign_in(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;
    let bob_id = share(&app, &alice, &id, "bob@ajaia.test").await;

    let removed = send(&app, Method::DELETE, &format!("/api/documents/{id}/shares/{bob_id}"), None, None).await;

    assert_eq!(removed.status, StatusCode::UNAUTHORIZED);
    assert_eq!(removed.body, json!({ "error": "Not signed in" }));
    assert_eq!(shared_emails(&app, &alice, &id).await, ["bob@ajaia.test"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_owner_can_share_again_with_a_removed_user(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    let bob_id = share(&app, &alice, &id, "bob@ajaia.test").await;
    let removed = send(&app, Method::DELETE, &format!("{url}/shares/{bob_id}"), Some(&alice), None).await;

    let shared_again = send(&app, Method::POST, &format!("{url}/shares"), Some(&alice), Some(json!({ "email": "bob@ajaia.test" }))).await;
    let opened = send(&app, Method::GET, &url, Some(&bob), None).await;

    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    assert_eq!(shared_again.status, StatusCode::CREATED);
    assert_eq!(shared_again.body["email"], "bob@ajaia.test");
    assert_eq!(opened.status, StatusCode::OK);
    assert_eq!(shared_emails(&app, &alice, &id).await, ["bob@ajaia.test"]);
}
