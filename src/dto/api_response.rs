use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a success response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
            meta: None,
        }
    }

    /// Create a success response with only a message
    pub fn message(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(msg.into()),
            data: None,
            meta: None,
        }
    }

    /// Create a success response with both data and a message
    pub fn success_with_message(data: T, msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(msg.into()),
            data: Some(data),
            meta: None,
        }
    }

    /// Create a success response with paginated data
    pub fn paginated(data: T, total: i64, page: i64, limit: i64) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
            meta: Some(PaginationMeta { total, page, limit }),
        }
    }
}

// For cases where we want to return just a message with no data, 
// we can use ApiResponse<()>
impl ApiResponse<()> {
    pub fn empty() -> Self {
        Self {
            success: true,
            message: None,
            data: None,
            meta: None,
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
