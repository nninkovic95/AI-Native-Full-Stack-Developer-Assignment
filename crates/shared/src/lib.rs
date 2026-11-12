//! Request and response types shared by the server and the web client,
//! so both sides of the API are checked by the same compiler.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const MAX_TITLE_LEN: usize = 200;
pub const DEFAULT_TITLE: &str = "Untitled document";
/// File types `POST /api/documents/import` accepts, matched without regard to letter case.
pub const IMPORT_EXTENSIONS: [&str; 2] = ["txt", "md"];
/// Largest file `POST /api/documents/import` accepts.
pub const MAX_IMPORT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Health {
    pub status: String,
}

/// Body of every non-2xx API response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

/// `POST /api/login`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// `POST /api/documents/{id}/shares`: the email of the user to give access to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShareRequest {
    pub email: String,
}

/// Returned by `POST /api/login` and `GET /api/me`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

/// One row of `GET /api/documents`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSummary {
    pub id: Uuid,
    pub title: String,
    pub owner: User,
    /// False when the document was shared with the current user.
    pub is_owner: bool,
    pub updated_at: DateTime<Utc>,
}

/// `GET /api/documents/{id}`, and the response of create and update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    /// A Quill Delta: `{"ops": [...]}`.
    pub content: Value,
    pub owner: User,
    pub is_owner: bool,
    pub updated_at: DateTime<Utc>,
}

/// `POST /api/documents`. A missing title becomes [`DEFAULT_TITLE`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateDocument {
    #[serde(default)]
    pub title: Option<String>,
}

/// `PATCH /api/documents/{id}`. Absent fields are left unchanged.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateDocument {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
}
