use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    #[serde(skip)]
    status: StatusCode,
    message: String,
    success: bool,
    data: T,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn with_status(status: StatusCode, data: T, message: &str) -> Self {
        Self {
            status,
            message: message.to_string(),
            success: true,
            data,
        }
    }

    pub fn ok(data: T, message: &str) -> Self {
        Self::with_status(StatusCode::OK, data, message)
    }

    pub fn created(data: T, message: &str) -> Self {
        Self::with_status(StatusCode::CREATED, data, message)
    }

    pub fn accepted(data: T, message: &str) -> Self {
        Self::with_status(StatusCode::ACCEPTED, data, message)
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(self)).into_response()
    }
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Unauthorized(String),

    #[error("{0}")]
    Forbidden(String),

    #[error("{0}")]
    Conflict(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized(message.into())
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn internal(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();

        if let ApiError::Internal(ref err) = self {
            tracing::error!(error = ?err, "internal server error");
        }

        let message = match &self {
            ApiError::Internal(_) => "Internal server error".to_string(),
            _ => self.to_string(),
        };

        (
            status,
            Json(ApiErrorResponse {
                success: false,
                error: ErrorBody { message },
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct ApiErrorResponse {
    success: bool,
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
}
