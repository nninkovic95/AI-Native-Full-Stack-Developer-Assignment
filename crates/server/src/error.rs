use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use shared::ApiError;

use crate::import::ImportError;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    /// Same message for unknown email and wrong password, so accounts cannot be enumerated.
    BadCredentials,
    Unauthorized,
    Forbidden(String),
    /// Also used when the caller has no access, so document IDs cannot be probed.
    NotFound,
    Conflict(String),
    PayloadTooLarge(String),
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            AppError::BadCredentials => {
                (StatusCode::UNAUTHORIZED, "Wrong email or password".into())
            }
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Not signed in".into()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Document not found".into()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m),
            AppError::PayloadTooLarge(m) => (StatusCode::PAYLOAD_TOO_LARGE, m),
            AppError::Internal(m) => {
                tracing::error!("internal error: {m}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong".into())
            }
        };
        (status, Json(ApiError { error: msg })).into_response()
    }
}

/// `axum::Json` for request bodies, rejecting in the API's error shape instead of axum's plain text.
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, AppError> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(ApiJson(value)),
            Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                Err(AppError::PayloadTooLarge("The document is too large".into()))
            }
            Err(_) => Err(AppError::BadRequest("Invalid request body".into())),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<ImportError> for AppError {
    fn from(e: ImportError) -> Self {
        match e {
            ImportError::UnsupportedType => {
                AppError::BadRequest("Only .txt and .md files can be imported".into())
            }
            ImportError::TooLarge => AppError::PayloadTooLarge("The file is larger than 1 MB".into()),
            ImportError::NotUtf8 => AppError::BadRequest("The file is not valid UTF-8 text".into()),
            ImportError::Empty => AppError::BadRequest("The file is empty".into()),
        }
    }
}
