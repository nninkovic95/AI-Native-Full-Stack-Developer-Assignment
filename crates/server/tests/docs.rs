mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn create_gives_an_empty_untitled_document_owned_by_the_caller(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::POST, "/api/documents", Some(&alice), Some(json!({}))).await;

    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["title"], "Untitled document");
    assert_eq!(res.body["content"], json!({ "ops": [] }));
    assert_eq!(res.body["owner"]["email"], ALICE);
    assert_eq!(res.body["is_owner"], true);
}

#[sqlx::test(migrations = "../../migrations")]
async fn documents_require_sign_in(pool: PgPool) {
    let app = app(pool).await;

    let res = send(&app, Method::POST, "/api/documents", None, Some(json!({}))).await;

    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

async fn create_doc(app: &axum::Router, cookie: &str) -> String {
    let res = send(app, Method::POST, "/api/documents", Some(cookie), Some(json!({}))).await;
    assert_eq!(res.status, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_owned()
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_can_open_their_document(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;

    let res = send(&app, Method::GET, &format!("/api/documents/{id}"), Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["id"], id);
    assert_eq!(res.body["is_owner"], true);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_user_without_access_gets_404_not_403(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let carol = login(&app, CAROL).await;
    let id = create_doc(&app, &alice).await;

    let res = send(&app, Method::GET, &format!("/api/documents/{id}"), Some(&carol), None).await;

    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.body["error"], "Document not found");
}

#[sqlx::test(migrations = "../../migrations")]
async fn patch_saves_title_and_formatted_content(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    let delta = json!({ "ops": [
        { "insert": "Plan" }, { "insert": "\n", "attributes": { "header": 1 } },
        { "insert": "bold", "attributes": { "bold": true } }, { "insert": "\n" }
    ]});

    let res = send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "title": "Q3 plan", "content": delta }))).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(reopened.body["title"], "Q3 plan");
    assert_eq!(reopened.body["content"], delta);
}

#[sqlx::test(migrations = "../../migrations")]
async fn patch_with_only_a_title_keeps_the_content(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");
    let delta = json!({ "ops": [{ "insert": "keep me\n" }] });
    send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "content": delta }))).await;

    send(&app, Method::PATCH, &url, Some(&alice), Some(json!({ "title": "Renamed" }))).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(reopened.body["title"], "Renamed");
    assert_eq!(reopened.body["content"], delta);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_user_without_access_cannot_patch(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let carol = login(&app, CAROL).await;
    let id = create_doc(&app, &alice).await;
    let url = format!("/api/documents/{id}");

    let res = send(&app, Method::PATCH, &url, Some(&carol), Some(json!({ "title": "hijacked" }))).await;
    let reopened = send(&app, Method::GET, &url, Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(reopened.body["title"], "Untitled document");
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_shows_only_my_documents_most_recently_edited_first(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let bob = login(&app, BOB).await;
    let first = create_doc(&app, &alice).await;
    let second = create_doc(&app, &alice).await;
    create_doc(&app, &bob).await;
    send(&app, Method::PATCH, &format!("/api/documents/{first}"), Some(&alice), Some(json!({ "title": "Edited last" }))).await;

    let res = send(&app, Method::GET, "/api/documents", Some(&alice), None).await;

    assert_eq!(res.status, StatusCode::OK);
    let list = res.body.as_array().unwrap();
    let ids: Vec<&str> = list.iter().map(|d| d["id"].as_str().unwrap()).collect();
    assert_eq!(ids, [first.as_str(), second.as_str()]);
    assert_eq!(list[0]["title"], "Edited last");
    assert_eq!(list[0]["is_owner"], true);
    assert_eq!(list[0]["owner"]["email"], ALICE);
    assert!(list[0].get("content").is_none(), "list rows should not carry document bodies");
}
