use axum::{
    extract::{FromRequestParts, Path, State},
    http::{request::Parts, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use shared::{CreateDocument, Document, DocumentSummary, UpdateDocument, User, DEFAULT_TITLE, MAX_TITLE_LEN};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::CurrentUser,
    error::{ApiJson, AppResult},
    AppError, AppState,
};

#[derive(sqlx::FromRow)]
struct DocRow {
    id: Uuid,
    title: String,
    content: Value,
    updated_at: DateTime<Utc>,
    owner_id: Uuid,
    owner_email: String,
    owner_name: String,
}

impl DocRow {
    fn into_document(self, viewer: &User) -> Document {
        Document {
            id: self.id,
            title: self.title,
            content: self.content,
            is_owner: self.owner_id == viewer.id,
            owner: User {
                id: self.owner_id,
                email: self.owner_email,
                name: self.owner_name,
            },
            updated_at: self.updated_at,
        }
    }
}

/// The only place that decides who may open a document: its owner, or a user it was shared with.
/// Anyone else gets `NotFound`, the same as for an ID that does not exist.
pub(crate) async fn load_accessible(pool: &PgPool, user: &User, id: Uuid) -> AppResult<Document> {
    let row: Option<DocRow> = sqlx::query_as(
        "SELECT d.id, d.title, d.content, d.updated_at,
                u.id AS owner_id, u.email AS owner_email, u.name AS owner_name
         FROM documents d JOIN users u ON u.id = d.owner_id
         WHERE d.id = $1
           AND (d.owner_id = $2
                OR EXISTS (SELECT 1 FROM document_shares s WHERE s.document_id = d.id AND s.user_id = $2))",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(pool)
    .await?;
    row.map(|r| r.into_document(user)).ok_or(AppError::NotFound)
}

/// The `{id}` path segment. One that is not a UUID names no document, so it is `NotFound` like any unknown ID.
pub struct DocId(pub Uuid);

impl<S: Send + Sync> FromRequestParts<S> for DocId {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AppError> {
        match Path::<Uuid>::from_request_parts(parts, state).await {
            Ok(Path(id)) => Ok(DocId(id)),
            Err(_) => Err(AppError::NotFound),
        }
    }
}

/// The form a title is stored in: surrounding whitespace removed, never blank.
fn clean_title(title: &str) -> AppResult<&str> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::BadRequest("Title cannot be empty".into()));
    }
    if title.chars().count() > MAX_TITLE_LEN {
        return Err(AppError::BadRequest(format!(
            "Title is too long ({MAX_TITLE_LEN} characters at most)"
        )));
    }
    Ok(title)
}

/// Whether `content` has the shape of a Quill Delta, the only form the editor can reopen.
fn is_delta(content: &Value) -> bool {
    content
        .get("ops")
        .and_then(Value::as_array)
        .is_some_and(|ops| ops.iter().all(|op| op.get("insert").is_some()))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    ApiJson(req): ApiJson<CreateDocument>,
) -> AppResult<(StatusCode, Json<Document>)> {
    let title = req.title.unwrap_or_else(|| DEFAULT_TITLE.to_owned());
    let title = clean_title(&title)?;
    let doc = insert(&state.pool, &user, title, json!({ "ops": [] })).await?;
    Ok((StatusCode::CREATED, Json(doc)))
}

/// Stores a new document owned by `owner` and returns it as the owner sees it.
pub(crate) async fn insert(pool: &PgPool, owner: &User, title: &str, content: Value) -> AppResult<Document> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO documents (id, owner_id, title, content) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(owner.id)
        .bind(title)
        .bind(content)
        .execute(pool)
        .await?;
    load_accessible(pool, owner, id).await
}

pub async fn get_one(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    DocId(id): DocId,
) -> AppResult<Json<Document>> {
    Ok(Json(load_accessible(&state.pool, &user, id).await?))
}

pub async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    DocId(id): DocId,
    ApiJson(req): ApiJson<UpdateDocument>,
) -> AppResult<Json<Document>> {
    load_accessible(&state.pool, &user, id).await?;
    let title = req.title.as_deref().map(clean_title).transpose()?;
    if req.content.as_ref().is_some_and(|c| !is_delta(c)) {
        return Err(AppError::BadRequest("Content is not a valid document".into()));
    }
    sqlx::query(
        "UPDATE documents
         SET title = COALESCE($2, title), content = COALESCE($3, content), updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(title)
    .bind(req.content)
    .execute(&state.pool)
    .await?;
    Ok(Json(load_accessible(&state.pool, &user, id).await?))
}

/// Removes only the document row: its shares follow through the foreign key's `ON DELETE CASCADE`.
pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    DocId(id): DocId,
) -> AppResult<StatusCode> {
    if !load_accessible(&state.pool, &user, id).await?.is_owner {
        return Err(AppError::Forbidden("Only the owner can delete this document".into()));
    }
    sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<DocumentSummary>>> {
    let rows: Vec<SummaryRow> = sqlx::query_as(
        "SELECT d.id, d.title, d.updated_at,
                u.id AS owner_id, u.email AS owner_email, u.name AS owner_name
         FROM documents d JOIN users u ON u.id = d.owner_id
         WHERE d.owner_id = $1
            OR EXISTS (SELECT 1 FROM document_shares s WHERE s.document_id = d.id AND s.user_id = $1)
         ORDER BY d.updated_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows.into_iter().map(|r| r.into_summary(&user)).collect()))
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    id: Uuid,
    title: String,
    updated_at: DateTime<Utc>,
    owner_id: Uuid,
    owner_email: String,
    owner_name: String,
}

impl SummaryRow {
    fn into_summary(self, viewer: &User) -> DocumentSummary {
        DocumentSummary {
            id: self.id,
            title: self.title,
            is_owner: self.owner_id == viewer.id,
            owner: User {
                id: self.owner_id,
                email: self.owner_email,
                name: self.owner_name,
            },
            updated_at: self.updated_at,
        }
    }
}
