use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use shared::{ShareRequest, User};
use uuid::Uuid;

use crate::{
    auth::CurrentUser,
    docs::{load_accessible, DocId},
    error::{ApiJson, AppResult},
    AppError, AppState,
};

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    DocId(id): DocId,
) -> AppResult<Json<Vec<User>>> {
    load_accessible(&state.pool, &user, id).await?;
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT u.id, u.email, u.name
         FROM document_shares s JOIN users u ON u.id = s.user_id
         WHERE s.document_id = $1
         ORDER BY u.name",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, email, name)| User { id, email, name })
            .collect(),
    ))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    DocId(id): DocId,
    ApiJson(req): ApiJson<ShareRequest>,
) -> AppResult<(StatusCode, Json<User>)> {
    let doc = load_accessible(&state.pool, &user, id).await?;
    if !doc.is_owner {
        return Err(AppError::Forbidden("Only the owner can share this document".into()));
    }
    // Accounts are stored in lower case, the same form the login lookup uses.
    let email = req.email.trim().to_lowercase();
    if email.is_empty() {
        return Err(AppError::BadRequest("Enter an email address".into()));
    }
    let target: Option<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, email, name FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&state.pool)
            .await?;
    let (user_id, email, name) =
        target.ok_or_else(|| AppError::BadRequest("No user with that email address".into()))?;
    if user_id == doc.owner.id {
        return Err(AppError::BadRequest("You already own this document".into()));
    }
    // The primary key decides whether the share is new, so two concurrent requests cannot both succeed.
    let inserted = sqlx::query(
        "INSERT INTO document_shares (document_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(user_id)
    .execute(&state.pool)
    .await?
    .rows_affected();
    if inserted == 0 {
        return Err(AppError::Conflict(
            "This document is already shared with that user".into(),
        ));
    }
    Ok((StatusCode::CREATED, Json(User { id: user_id, email, name })))
}
