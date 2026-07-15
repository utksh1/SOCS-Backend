use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;

/// Middleware to log all incoming HTTP requests with timing information
pub async fn log_request(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let query = uri.query().map(|q| q.to_string());
    let start = Instant::now();
    
    // Extract client IP if available (done before moving request)
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
        })
        .map(|s| s.to_string());
    
    tracing::debug!(
        method = %method,
        path = %path,
        query = ?query,
        client_ip = ?client_ip,
        "Incoming request"
    );
    
    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status();
    
    // Log level based on status code
    match status.as_u16() {
        200..=299 => {
            tracing::info!(
                method = %method,
                path = %path,
                status = status.as_u16(),
                duration_ms = duration.as_millis(),
                client_ip = ?client_ip,
                "Request completed successfully"
            );
        }
        400..=499 => {
            tracing::warn!(
                method = %method,
                path = %path,
                status = status.as_u16(),
                duration_ms = duration.as_millis(),
                client_ip = ?client_ip,
                "Client error response"
            );
        }
        500..=599 => {
            tracing::error!(
                method = %method,
                path = %path,
                status = status.as_u16(),
                duration_ms = duration.as_millis(),
                client_ip = ?client_ip,
                "Server error response"
            );
        }
        _ => {
            tracing::debug!(
                method = %method,
                path = %path,
                status = status.as_u16(),
                duration_ms = duration.as_millis(),
                client_ip = ?client_ip,
                "Request completed"
            );
        }
    }
    
    response
}
