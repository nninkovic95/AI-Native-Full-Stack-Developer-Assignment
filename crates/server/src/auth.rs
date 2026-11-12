use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use axum::{
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use shared::{LoginRequest, User};
use uuid::Uuid;

use crate::{error::AppResult, AppError, AppState};

pub const SESSION_COOKIE: &str = "session";
const SESSION_DAYS: i64 = 7;

/// The signed-in user, resolved from the session cookie; rejects with 401 otherwise.
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_owned())
            .ok_or(AppError::Unauthorized)?;
        let user: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT u.id, u.email, u.name FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token = $1 AND s.expires_at > now()",
        )
        .bind(token)
        .fetch_optional(&state.pool)
        .await?;
        let (id, email, name) = user.ok_or(AppError::Unauthorized)?;
        Ok(CurrentUser(User { id, email, name }))
    }
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, StatusCode)> {
    if let Some(c) = jar.get(SESSION_COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(c.value())
            .execute(&state.pool)
            .await?;
    }
    let jar = jar.remove(Cookie::build(SESSION_COOKIE).path("/"));
    Ok((jar, StatusCode::NO_CONTENT))
}

pub async fn me(CurrentUser(user): CurrentUser) -> Json<User> {
    Json(user)
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<User>)> {
    let row: Option<(Uuid, String, String, String)> =
        sqlx::query_as("SELECT id, email, name, password_hash FROM users WHERE email = $1")
            .bind(req.email.trim().to_lowercase())
            .fetch_optional(&state.pool)
            .await?;
    let Some((id, email, name, hash)) = row else {
        return Err(AppError::BadCredentials);
    };
    let parsed = PasswordHash::new(&hash).map_err(|e| AppError::Internal(e.to_string()))?;
    if Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed)
        .is_err()
    {
        return Err(AppError::BadCredentials);
    }

    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(&token)
        .bind(id)
        .bind(Utc::now() + Duration::days(SESSION_DAYS))
        .execute(&state.pool)
        .await?;

    let cookie = Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .secure(state.cookie_secure)
        .max_age(time::Duration::days(SESSION_DAYS))
        .build();
    Ok((jar.add(cookie), Json(User { id, email, name })))
}
