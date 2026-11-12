//! Fetch wrappers. Same-origin, so the session cookie is sent automatically.

use gloo_net::http::{Request, Response};
use serde::{de::DeserializeOwned, Serialize};
use shared::{CreateDocument, Document, DocumentSummary, LoginRequest, ShareRequest, UpdateDocument, User};
use uuid::Uuid;

pub use crate::logic::api_shapes::{ApiFailure, FailureKind};
use crate::logic::api_shapes::failure_from_response;

pub type ApiResult<T> = Result<T, ApiFailure>;

fn network_error() -> ApiFailure {
    failure_from_response(0, "")
}

async fn check(resp: Response) -> ApiResult<Response> {
    if resp.ok() {
        return Ok(resp);
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    Err(failure_from_response(status, &body))
}

async fn send(req: Result<Request, gloo_net::Error>) -> ApiResult<Response> {
    let req = req.map_err(|_| network_error())?;
    let resp = req.send().await.map_err(|_| network_error())?;
    check(resp).await
}

async fn json<T: DeserializeOwned>(resp: Response) -> ApiResult<T> {
    let status = resp.status();
    resp.json::<T>().await.map_err(|e| ApiFailure {
        kind: FailureKind::Other,
        status,
        message: format!("Unexpected response from the server: {e}"),
    })
}

fn json_post<B: Serialize>(method: &str, url: &str, body: &B) -> Result<Request, gloo_net::Error> {
    let builder = match method {
        "PATCH" => Request::patch(url),
        _ => Request::post(url),
    };
    builder.json(body)
}

pub async fn login(email: &str, password: &str) -> ApiResult<User> {
    let body = LoginRequest { email: email.to_string(), password: password.to_string() };
    json(send(json_post("POST", "/api/login", &body)).await?).await
}

pub async fn logout() -> ApiResult<()> {
    send(Request::post("/api/logout").build()).await.map(|_| ())
}

pub async fn me() -> ApiResult<User> {
    json(send(Request::get("/api/me").build()).await?).await
}

pub async fn list_documents() -> ApiResult<Vec<DocumentSummary>> {
    json(send(Request::get("/api/documents").build()).await?).await
}

pub async fn create_document() -> ApiResult<Document> {
    json(send(json_post("POST", "/api/documents", &CreateDocument::default())).await?).await
}

pub async fn get_document(id: Uuid) -> ApiResult<Document> {
    json(send(Request::get(&format!("/api/documents/{id}")).build()).await?).await
}

pub async fn update_document(id: Uuid, body: &UpdateDocument) -> ApiResult<Document> {
    json(send(json_post("PATCH", &format!("/api/documents/{id}"), body)).await?).await
}

pub async fn list_shares(id: Uuid) -> ApiResult<Vec<User>> {
    json(send(Request::get(&format!("/api/documents/{id}/shares")).build()).await?).await
}

pub async fn share_document(id: Uuid, email: &str) -> ApiResult<User> {
    let body = ShareRequest { email: email.to_string() };
    json(send(json_post("POST", &format!("/api/documents/{id}/shares"), &body)).await?).await
}

/// The success answer is 204 with no body, so nothing is parsed.
pub async fn delete_document(id: Uuid) -> ApiResult<()> {
    send(Request::delete(&format!("/api/documents/{id}")).build()).await.map(|_| ())
}

/// Uploads a `.txt` or `.md` file as multipart form data and returns the new document.
/// No Content-Type is set so the browser adds the multipart boundary itself.
pub async fn import_document(file: &web_sys::File) -> ApiResult<Document> {
    let unreadable = || ApiFailure {
        kind: FailureKind::Other,
        status: 0,
        message: "Could not read the selected file".to_string(),
    };
    let form = web_sys::FormData::new().map_err(|_| unreadable())?;
    form.append_with_blob_and_filename("file", file, &file.name()).map_err(|_| unreadable())?;
    json(send(Request::post("/api/documents/import").body(form)).await?).await
}
