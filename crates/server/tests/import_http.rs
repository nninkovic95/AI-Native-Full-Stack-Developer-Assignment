//! File import through the real router: POST /api/documents/import.

mod common;

use axum::http::{Method, StatusCode};
use common::*;
use serde_json::json;
use sqlx::PgPool;

const IMPORT: &str = "/api/documents/import";

#[sqlx::test(migrations = "../../migrations")]
async fn an_uploaded_markdown_file_becomes_a_document_the_caller_can_reopen(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let file = file_part("file", "Q3 plan.md", b"# Goals\n\nShip **import**.\n");

    let res = send_multipart(&app, IMPORT, Some(&alice), &[file]).await;

    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.body);
    let id = res.body["id"].as_str().unwrap();
    let reopened = send(&app, Method::GET, &format!("/api/documents/{id}"), Some(&alice), None).await;
    assert_eq!(reopened.status, StatusCode::OK);
    assert_eq!(reopened.body["title"], "Q3 plan");
    assert_eq!(
        reopened.body["content"],
        json!({ "ops": [
            { "insert": "Goals" },
            { "insert": "\n", "attributes": { "header": 1 } },
            { "insert": "Ship " },
            { "insert": "import", "attributes": { "bold": true } },
            { "insert": ".\n" }
        ]})
    );
    assert_eq!(reopened.body["owner"]["email"], ALICE);
    assert_eq!(reopened.body["is_owner"], true);
}

#[sqlx::test(migrations = "../../migrations")]
async fn importing_requires_sign_in(pool: PgPool) {
    let app = app(pool).await;
    let file = file_part("file", "notes.txt", b"Some words");

    let res = send_multipart(&app, IMPORT, None, &[file]).await;

    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.body["error"], "Not signed in");
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_unsupported_file_type_is_refused_with_a_clear_message(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let file = file_part("file", "report.docx", b"Some words");

    let res = send_multipart(&app, IMPORT, Some(&alice), &[file]).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "Only .txt and .md files can be imported" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_file_just_over_one_megabyte_is_refused_as_too_large(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let file = file_part("file", "big.txt", &vec![b'a'; 1024 * 1024 + 1]);

    let res = send_multipart(&app, IMPORT, Some(&alice), &[file]).await;

    assert_eq!(res.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(res.body, json!({ "error": "The file is larger than 1 MB" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_far_too_large_upload_gets_the_same_json_answer(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let file = file_part("file", "huge.txt", &vec![b'a'; 3 * 1024 * 1024]);

    let res = send_multipart(&app, IMPORT, Some(&alice), &[file]).await;

    assert_eq!(res.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(res.body, json!({ "error": "The file is larger than 1 MB" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_upload_without_a_file_field_is_refused(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let misnamed = file_part("attachment", "notes.txt", b"Some words");

    let res = send_multipart(&app, IMPORT, Some(&alice), &[misnamed]).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "No file was uploaded" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_file_that_is_not_utf8_text_is_refused_with_a_clear_message(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let file = file_part("file", "latin1.txt", b"caf\xe9");

    let res = send_multipart(&app, IMPORT, Some(&alice), &[file]).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "The file is not valid UTF-8 text" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_empty_file_is_refused_with_a_clear_message(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;
    let file = file_part("file", "blank.md", b"\n  \n");

    let res = send_multipart(&app, IMPORT, Some(&alice), &[file]).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "The file is empty" }));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_request_that_is_not_a_form_upload_is_refused_as_json(pool: PgPool) {
    let app = app(pool).await;
    let alice = login(&app, ALICE).await;

    let res = send(&app, Method::POST, IMPORT, Some(&alice), Some(json!({ "file": "notes.txt" }))).await;

    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.body, json!({ "error": "No file was uploaded" }));
}
