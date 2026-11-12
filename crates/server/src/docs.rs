use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use shared::{CreateDocument, Document, DocumentSummary, UpdateDocument, User, DEFAULT_TITLE};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{auth::CurrentUser, error::AppResult, AppError, AppState};

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
async fn load_accessible(pool: &PgPool, user: &User, id: Uuid) -> AppResult<Document> {
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

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<CreateDocument>,
) -> AppResult<(StatusCode, Json<Document>)> {
    let title = req.title.unwrap_or_else(|| DEFAULT_TITLE.to_owned());
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO documents (id, owner_id, title, content) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(user.id)
        .bind(&title)
        .bind(json!({ "ops": [] }))
        .execute(&state.pool)
        .await?;
    let doc = load_accessible(&state.pool, &user, id).await?;
    Ok((StatusCode::CREATED, Json(doc)))
}

pub async fn get_one(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Document>> {
    Ok(Json(load_accessible(&state.pool, &user, id).await?))
}

pub async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDocument>,
) -> AppResult<Json<Document>> {
    load_accessible(&state.pool, &user, id).await?;
    sqlx::query(
        "UPDATE documents
         SET title = COALESCE($2, title), content = COALESCE($3, content), updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(req.title)
    .bind(req.content)
    .execute(&state.pool)
    .await?;
    Ok(Json(load_accessible(&state.pool, &user, id).await?))
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
