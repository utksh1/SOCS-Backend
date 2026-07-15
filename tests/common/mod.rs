use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use tower::ServiceExt;

/// Helper to make authenticated HTTP requests to the app
pub async fn make_request(
    app: Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> axum::response::Response {
    let method = match method {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "PATCH" => Method::PATCH,
        "DELETE" => Method::DELETE,
        _ => panic!("Unsupported HTTP method: {}", method),
    };
    
    let mut request = Request::builder()
        .method(method)
        .uri(path);
    
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }
    
    let request = if let Some(body) = body {
        request
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap()
    } else {
        request.body(Body::empty()).unwrap()
    };
    
    app.oneshot(request).await.unwrap()
}

/// Helper to extract JSON body from response
pub async fn response_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// Assert response status code
pub fn assert_status(response: &axum::response::Response, expected: StatusCode) {
    assert_eq!(
        response.status(),
        expected,
        "Expected status {}, got {}",
        expected,
        response.status()
    );
}
