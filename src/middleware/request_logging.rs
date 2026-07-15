use axum::{
    extract::{connect_info::ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use std::{net::SocketAddr, time::Instant};

/// Middleware to log all incoming HTTP requests with timing information
pub async fn log_request(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    // Deliberately log only the path. Verification/reset tokens live in query
    // parameters and must never be copied into application logs.
    let path = request.uri().path().to_string();
    let start = Instant::now();
    
    // Extract client IP if available (done before moving request)
    let client_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(address)| address.ip().to_string());
    
    tracing::debug!(
        method = %method,
        path = %path,
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
