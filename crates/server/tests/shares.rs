mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;
use sqlx::PgPool;

async fn create_doc(app: &axum::Router, cookie: &str) -> String {
    let res = send(app, Method::POST, "/api/documents", Some(cookie), Some(json!({}))).await;
    assert_eq!(res.status, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_owned()
}

async fn share(app: &axum::Router, cookie: &str, id: &str, email: &str) -> Resp {
    send(
        app,
        Method::POST,
        &format!("/api/documents/{id}/shares"),
        Some(cookie),
        Some(json!({ "email": email })),
    )
    .await
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_shares_a_document_with_another_user(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;

    let res = share(&app, &alice, &id, "bob@ajaia.test").await;

    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["email"], "bob@ajaia.test");
    assert_eq!(res.body["name"], "Bob");
    assert!(res.body["id"].is_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_recipient_can_open_and_edit_a_shared_document(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    share(&app, &alice, &id, "bob@ajaia.test").await;
    let delta = json!({ "ops": [{ "insert": "from bob\n" }] });

    let opened = send(&app, Method::GET, &url, Some(&bob), None).await;
    let edited = send(&app, Method::PATCH, &url, Some(&bob), Some(json!({ "content": delta }))).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(opened.status, StatusCode::OK);
    assert_eq!(opened.body["is_owner"], false);
    assert_eq!(opened.body["owner"]["email"], "alice@ajaia.test");
    assert_eq!(edited.status, StatusCode::OK);
    assert_eq!(edited.body["is_owner"], false);
    assert_eq!(reopened.body["content"], json!({ "ops": [{ "insert": "from bob\n" }] }));
    assert_eq!(reopened.body["is_owner"], true);
}

#[sqlx::test(migrations = "../../migrations")]
async fn sharing_with_one_user_gives_a_third_user_no_access(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let carol = login(&app, CAROL).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    share(&app, &alice, &id, "bob@ajaia.test").await;

    let opened = send(&app, Method::GET, &url, Some(&carol), None).await;
    let edited = send(&app, Method::PATCH, &url, Some(&carol), Some(json!({ "title": "hijacked" }))).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(opened.status, StatusCode::NOT_FOUND);
    assert_eq!(opened.body["error"], "Document not found");
    assert_eq!(edited.status, StatusCode::NOT_FOUND);
    assert_eq!(edited.body["error"], "Document not found");
    assert_eq!(reopened.body["title"], "Untitled document");
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
async fn the_list_shows_shared_documents_next_to_my_own(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let carol = login(&app, CAROL).await;
    let shared_doc = create_doc(&app, &alice).await;
    let bobs_own = create_doc(&app, &bob).await;
    share(&app, &alice, &shared_doc, "bob@ajaia.test").await;

    let res = send(&app, Method::GET, "/api/documents", Some(&bob), None).await;

    assert_eq!(res.status, StatusCode::OK);
    let list = res.body.as_array().unwrap();
    // Most recently edited first: bob's document was created after alice's.
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["id"], bobs_own);
    assert_eq!(list[0]["is_owner"], true);
    assert_eq!(list[0]["owner"]["email"], "bob@ajaia.test");
    assert_eq!(list[1]["id"], shared_doc);
    assert_eq!(list[1]["is_owner"], false);
    assert_eq!(list[1]["owner"]["email"], "alice@ajaia.test");
    assert_eq!(list_ids(&app, &alice).await, [shared_doc.clone()]);
    assert_eq!(list_ids(&app, &carol).await, Vec::<String>::new());
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_share_list_names_the_shared_users_in_name_order(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let carol = login(&app, CAROL).await;
    let for_both = create_doc(&app, &alice).await;
    let for_bob_only = create_doc(&app, &alice).await;
    share(&app, &alice, &for_both, "carol@ajaia.test").await;
    share(&app, &alice, &for_both, "bob@ajaia.test").await;
    share(&app, &alice, &for_bob_only, "bob@ajaia.test").await;

    let as_owner = send(&app, Method::GET, &format!("/api/documents/{for_both}/shares"), Some(&alice), None).await;
    let as_shared = send(&app, Method::GET, &format!("/api/documents/{for_both}/shares"), Some(&bob), None).await;
    let as_stranger = send(&app, Method::GET, &format!("/api/documents/{for_bob_only}/shares"), Some(&carol), None).await;

    assert_eq!(as_owner.status, StatusCode::OK);
    let users = as_owner.body.as_array().unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0]["email"], "bob@ajaia.test");
    assert_eq!(users[0]["name"], "Bob");
    assert_eq!(users[1]["email"], "carol@ajaia.test");
    assert_eq!(users[1]["name"], "Carol");
    assert_eq!(as_shared.status, StatusCode::OK);
    assert_eq!(as_shared.body, as_owner.body);
    assert_eq!(as_stranger.status, StatusCode::NOT_FOUND);
    assert_eq!(as_stranger.body["error"], "Document not found");
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_shared_user_cannot_share_onward(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let carol = login(&app, CAROL).await;
    let id = create_doc(&app, &alice).await;
    share(&app, &alice, &id, "bob@ajaia.test").await;

    let res = share(&app, &bob, &id, "carol@ajaia.test").await;
    let carol_opens = send(&app, Method::GET, &format!("/api/documents/{id}"), Some(&carol), None).await;

    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.body["error"], "Only the owner can share this document");
    assert_eq!(carol_opens.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_user_without_access_gets_404_not_403_when_sharing(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let carol = login(&app, CAROL).await;
    let id = create_doc(&app, &alice).await;

    let res = share(&app, &carol, &id, "carol@ajaia.test").await;
    let carol_opens = send(&app, Method::GET, &format!("/api/documents/{id}"), Some(&carol), None).await;

    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.body["error"], "Document not found");
    assert_eq!(carol_opens.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn sharing_needs_an_email_address(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;

    let res = share(&app, &alice, &id, "   ").await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body["error"], "Enter an email address");
}

#[sqlx::test(migrations = "../../migrations")]
async fn sharing_with_an_unknown_email_is_rejected(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;

    let res = share(&app, &alice, &id, "nobody@ajaia.test").await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body["error"], "No user with that email address");
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_owner_cannot_share_with_themselves(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;

    let res = share(&app, &alice, &id, "alice@ajaia.test").await;
    let shares = send(&app, Method::GET, &format!("/api/documents/{id}/shares"), Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body["error"], "You already own this document");
    assert_eq!(shares.body, json!([]));
}

#[sqlx::test(migrations = "../../migrations")]
async fn sharing_twice_with_the_same_user_is_a_conflict(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;
    share(&app, &alice, &id, "bob@ajaia.test").await;

    let res = share(&app, &alice, &id, "bob@ajaia.test").await;
    let shares = send(&app, Method::GET, &format!("/api/documents/{id}/shares"), Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::CONFLICT);
    assert_eq!(res.body["error"], "This document is already shared with that user");
    assert_eq!(shares.body.as_array().unwrap().len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_email_is_matched_after_trimming_and_in_any_letter_case(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let id = create_doc(&app, &alice).await;

    let res = share(&app, &alice, &id, " Bob@Ajaia.Test ").await;
    let bob_opens = send(&app, Method::GET, &format!("/api/documents/{id}"), Some(&bob), None).await;

    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["email"], "bob@ajaia.test");
    assert_eq!(bob_opens.status, StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn share_routes_require_sign_in(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}/shares");

    let listed = send(&app, Method::GET, &url, None, None).await;
    let posted = send(&app, Method::POST, &url, None, Some(json!({ "email": "bob@ajaia.test" }))).await;
    let shares = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(listed.status, StatusCode::UNAUTHORIZED);
    assert_eq!(listed.body["error"], "Not signed in");
    assert_eq!(posted.status, StatusCode::UNAUTHORIZED);
    assert_eq!(posted.body["error"], "Not signed in");
    assert_eq!(shares.body, json!([]));
}
