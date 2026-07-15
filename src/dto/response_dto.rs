use serde::{Deserialize, Serialize};
use axum::{
    response::{IntoResponse, Response},
    Json,
};

use super::pagination_dto::PaginationMeta;

/// Generic API response wrapper that ensures consistent response structure
/// across all endpoints.
///
/// This struct provides:
/// - Compile-time type safety for response payloads
/// - Consistent JSON schema for frontend consumption
/// - Automatic OpenAPI/Swagger documentation generation
///
/// # Examples
///
/// Simple success response:
/// ```ignore
/// ApiResponse {
///     success: true,
///     message: None,
///     data: Some(user),
///     pagination: None,
/// }
/// ```
///
/// Success with message:
/// ```ignore
/// ApiResponse {
///     success: true,
///     message: Some("User created successfully".to_string()),
///     data: Some(user),
///     pagination: None,
/// }
/// ```
///
/// Paginated response:
/// ```ignore
/// ApiResponse {
///     success: true,
///     message: None,
///     data: Some(users),
///     pagination: Some(pagination_meta),
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Indicates whether the operation was successful
    pub success: bool,

    /// Optional human-readable message (e.g., "User created successfully")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// The actual response data payload (generic type T)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    /// Optional pagination metadata for list endpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationMeta>,
}

impl<T> ApiResponse<T> {
    /// Creates a simple success response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
            pagination: None,
        }
    }

    /// Creates a success response with a custom message
    pub fn success_with_message(message: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            message: Some(message.into()),
            data: Some(data),
            pagination: None,
        }
    }

    /// Creates a success response with pagination metadata
    pub fn success_paginated(data: T, pagination: PaginationMeta) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
            pagination: Some(pagination),
        }
    }

    /// Creates a success response with message and pagination
    pub fn success_paginated_with_message(
        message: impl Into<String>,
        data: T,
        pagination: PaginationMeta,
    ) -> Self {
        Self {
            success: true,
            message: Some(message.into()),
            data: Some(data),
            pagination: Some(pagination),
        }
    }

    /// Creates a success response with only a message (no data payload)
    pub fn message_only(message: impl Into<String>) -> Self
    where
        T: Default,
    {
        Self {
            success: true,
            message: Some(message.into()),
            data: None,
            pagination: None,
        }
    }
}

/// Generic API error response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub success: bool,
    pub message: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

impl ApiErrorResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            retry_after: None,
        }
    }

    pub fn with_retry_after(message: impl Into<String>, retry_after: u64) -> Self {
        Self {
            success: false,
            message: message.into(),
            retry_after: Some(retry_after),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestUser {
        id: i32,
        name: String,
    }

    #[test]
    fn test_success_response() {
        let user = TestUser {
            id: 1,
            name: "Test User".to_string(),
        };
        let response = ApiResponse::success(user);
        
        assert!(response.success);
        assert!(response.message.is_none());
        assert!(response.data.is_some());
        assert!(response.pagination.is_none());
    }

    #[test]
    fn test_success_with_message() {
        let user = TestUser {
            id: 1,
            name: "Test User".to_string(),
        };
        let response = ApiResponse::success_with_message("User created", user);
        
        assert!(response.success);
        assert_eq!(response.message, Some("User created".to_string()));
        assert!(response.data.is_some());
    }

    #[test]
    fn test_message_only() {
        let response = ApiResponse::<()>::message_only("Operation completed");
        
        assert!(response.success);
        assert_eq!(response.message, Some("Operation completed".to_string()));
        assert!(response.data.is_none());
    }
}
