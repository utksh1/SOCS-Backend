use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::dto::response_dto::ApiErrorResponse;

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    InternalServerError,
    ValidationError(String),
    RateLimitExceeded { retry_after: u64 },
    TooManyRequests(String),
    Gone(String),
    ServiceUnavailable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::RateLimitExceeded { retry_after } => {
                let mut response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ApiErrorResponse::with_retry_after(
                        format!("Rate limit exceeded. Please try again in {} seconds.", retry_after),
                        retry_after
                    )),
                ).into_response();
                
                response.headers_mut().insert(
                    "Retry-After",
                    retry_after.to_string().parse().expect("u64 should always produce valid header value")
                );
                
                response
            }
            _ => {
                let (status, message) = match self {
                    ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
                    ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
                    ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
                    ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
                    ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
                    ApiError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
                    ApiError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
                    ApiError::Gone(msg) => (StatusCode::GONE, msg),
                    ApiError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
                    ApiError::InternalServerError => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
                    }
                    ApiError::RateLimitExceeded { .. } => unreachable!(),
                };
                
                (
                    status,
                    Json(ApiErrorResponse::new(message)),
                )
                    .into_response()
            }
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        if matches!(err, sqlx::Error::RowNotFound) {
            return ApiError::NotFound("Record not found".to_string());
        }

        if err
            .as_database_error()
            .and_then(|database_error| database_error.code())
            .is_some_and(|code| code == "23505")
        {
            tracing::warn!("Database unique-constraint violation: {:?}", err);
            return ApiError::Conflict("A record with that value already exists".to_string());
        }

        if err
            .as_database_error()
            .and_then(|database_error| database_error.code())
            .is_some_and(|code| code == "23503")
        {
            tracing::warn!("Database foreign-key violation: {:?}", err);
            return ApiError::Conflict("The record still has related data".to_string());
        }

        println!("SQLX DB ERROR: {:?}", err);
        tracing::error!("Database error: {:?}", err);
        ApiError::InternalServerError
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(errors: validator::ValidationErrors) -> Self {
        ApiError::ValidationError(errors.to_string())
    }
}

impl From<bcrypt::BcryptError> for ApiError {
    fn from(err: bcrypt::BcryptError) -> Self {
        tracing::error!("Bcrypt error: {:?}", err);
        ApiError::InternalServerError
    }
}
